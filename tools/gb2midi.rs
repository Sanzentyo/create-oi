#!/usr/bin/env -S cargo +nightly -Zscript
---
[package]
name = "gb2midi"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
---

//! Extract embedded MIDI data from Game Boy ROM files.
//!
//! Game Boy music drivers (e.g. GBT Player) embed standard MIDI files inside
//! ROM images.  This tool scans for the `MThd` header (0x4D546864) and the
//! `CHS*` end marker (0x43485300–0x43485400) to extract the raw MIDI bytes,
//! replicating the behaviour of <https://larkob.github.io/GB2MIDI/>.
//!
//! # Usage
//!
//! ```
//! # dry-run (shows what would be extracted):
//! cargo +nightly -Zscript tools/gb2midi.rs song.gb
//!
//! # write .mid file(s):
//! cargo +nightly -Zscript tools/gb2midi.rs --apply song.gb
//! cargo +nightly -Zscript tools/gb2midi.rs -a -o /tmp/midi *.gb
//! ```

use clap::Parser;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "gb2midi", version, about)]
struct Args {
    /// Input ROM file(s) (.gb, .gbc, .rom, …)
    #[arg(required = true)]
    input: Vec<PathBuf>,

    /// Output directory (default: same directory as each input file)
    #[arg(short = 'o', long)]
    output_dir: Option<PathBuf>,

    /// Write .mid files; without this flag only a dry-run summary is shown
    #[arg(short = 'a', long)]
    apply: bool,
}

/// Locate embedded MIDI bytes within `data`.
///
/// Replicates the original GB2MIDI JavaScript extraction logic:
/// - scan byte-by-byte (offset 4 … len-5) for `MThd` (MIDI start) and `CHS*`
///   (MIDI end) markers using big-endian 32-bit reads;
/// - return the slice `[startpos, endpos+3)`, clipped to the buffer end.
/// Returns `None` if no `MThd` marker is present.
fn find_midi(data: &[u8]) -> Option<&[u8]> {
    if data.len() < 8 {
        return None;
    }

    let mut startpos: Option<usize> = None;
    // JS initialises endpos = length so that if no CHS marker is found,
    // the slice extends to end-of-file (clipped automatically).
    let mut endpos = data.len();

    // JS: `while (offset < length - 4)` starting at offset = 4
    let limit = data.len().saturating_sub(4);
    for offset in 4..limit {
        let marker = u32::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);

        // 'MThd' — standard MIDI file header magic
        if marker == 0x4D546_864 {
            startpos = Some(offset);
        }
        // 'CHS\x00'–'CHS\xFF' — GB ROM end-of-MIDI sentinel
        if startpos.is_some() && (0x4348_5300..0x4348_5400).contains(&marker) {
            endpos = offset;
        }
    }

    let start = startpos?;
    // JS: `f.slice(startpos, endpos + 3)` — Blob.slice end is exclusive,
    // so the included range is [start, endpos+2].  Clip to buffer end.
    let end = (endpos + 3).min(data.len());
    (end > start).then(|| &data[start..end])
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.apply {
        eprintln!("Mode: write .mid files");
    } else {
        eprintln!("Mode: dry-run (pass --apply / -a to write files)");
    }

    let mut found_count = 0usize;
    let mut error_count = 0usize;

    for input_path in &args.input {
        let data = match fs::read(input_path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("Error reading {}: {e}", input_path.display());
                error_count += 1;
                continue;
            }
        };

        match find_midi(&data) {
            None => {
                eprintln!("{}: no MIDI data found", input_path.display());
                error_count += 1;
            }
            Some(midi) => {
                found_count += 1;
                let stem = input_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy();
                let out_name = format!("{stem}.mid");
                let out_path: PathBuf = match &args.output_dir {
                    Some(dir) => dir.join(&out_name),
                    None => input_path.with_file_name(&out_name),
                };

                println!(
                    "{}: {} bytes → {}",
                    input_path.display(),
                    midi.len(),
                    out_path.display()
                );

                if args.apply {
                    if let Some(parent) = out_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&out_path, midi)?;
                    println!("  ✓ written");
                }
            }
        }
    }

    if found_count == 0 {
        eprintln!("No MIDI data extracted.");
        std::process::exit(1);
    }

    if error_count > 0 {
        eprintln!("{error_count} file(s) had errors.");
    }

    Ok(())
}
