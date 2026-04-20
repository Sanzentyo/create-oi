use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_dir = manifest_dir.parent().unwrap();
    let libcreate_dir = workspace_dir.join("vendor").join("libcreate");
    let csrc_dir = manifest_dir.join("csrc");

    let boost_include = find_boost_include();
    let boost_lib = find_boost_lib();

    let mut build = cc::Build::new();

    // Use zig c++ if ZIG_CXX env var points to a zig c++ wrapper script,
    // otherwise fall back to system C++ compiler.
    // Direct `zig c++` doesn't work well with cc crate's flag passing.
    if let Ok(zig_cxx) = env::var("ZIG_CXX") {
        build.compiler(&zig_cxx);
    }

    build
        .cpp(true)
        .std("c++14")
        .warnings(false)
        .include(libcreate_dir.join("include"))
        .include(&csrc_dir)
        .include(&boost_include)
        // Force-include our Boost compatibility header before any source
        .flag(format!(
            "-include{}",
            csrc_dir.join("boost_compat.h").display()
        ));

    // libcreate source files
    let libcreate_sources = [
        "src/create.cpp",
        "src/serial.cpp",
        "src/serial_stream.cpp",
        "src/serial_query.cpp",
        "src/data.cpp",
        "src/packet.cpp",
        "src/types.cpp",
    ];
    for src in &libcreate_sources {
        build.file(libcreate_dir.join(src));
    }

    // Our C wrapper
    build.file(csrc_dir.join("wrapper.cpp"));

    build.compile("create_wrapper");

    // Link Boost libraries
    println!("cargo:rustc-link-search=native={}", boost_lib.display());
    // boost_system became header-only in Boost 1.69+; only link if the
    // library file actually exists.
    if boost_lib.join("libboost_system.dylib").exists()
        || boost_lib.join("libboost_system.so").exists()
        || boost_lib.join("libboost_system.a").exists()
    {
        println!("cargo:rustc-link-lib=dylib=boost_system");
    }
    println!("cargo:rustc-link-lib=dylib=boost_thread");

    // Link C++ standard library
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    // Rerun if sources change
    println!("cargo:rerun-if-changed=csrc/wrapper.h");
    println!("cargo:rerun-if-changed=csrc/wrapper.cpp");
    for src in &libcreate_sources {
        println!(
            "cargo:rerun-if-changed={}",
            libcreate_dir.join(src).display()
        );
    }
}

fn find_boost_include() -> PathBuf {
    if let Ok(path) = env::var("BOOST_INCLUDE_DIR") {
        return PathBuf::from(path);
    }

    // Homebrew on Apple Silicon
    let homebrew_arm = PathBuf::from("/opt/homebrew/include");
    if homebrew_arm.exists() {
        return homebrew_arm;
    }

    // Homebrew on Intel Mac
    let homebrew_intel = PathBuf::from("/usr/local/include");
    if homebrew_intel.exists() {
        return homebrew_intel;
    }

    // System default
    PathBuf::from("/usr/include")
}

fn find_boost_lib() -> PathBuf {
    if let Ok(path) = env::var("BOOST_LIB_DIR") {
        return PathBuf::from(path);
    }

    let homebrew_arm = PathBuf::from("/opt/homebrew/lib");
    if homebrew_arm.exists() {
        return homebrew_arm;
    }

    let homebrew_intel = PathBuf::from("/usr/local/lib");
    if homebrew_intel.exists() {
        return homebrew_intel;
    }

    PathBuf::from("/usr/lib")
}

/// Locate the `zig` binary on the system PATH.
#[allow(dead_code)]
fn which_zig() -> Option<PathBuf> {
    std::process::Command::new("which")
        .arg("zig")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if path.is_empty() {
                    None
                } else {
                    Some(PathBuf::from(path))
                }
            } else {
                None
            }
        })
}
