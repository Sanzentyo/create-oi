set shell := ["bash", "-cu"]

# Default: list available recipes
default:
    @just --list

# Build default workspace members (core, serial, tokio)
build:
    cargo build --workspace

# Build ALL crates including experimental (smol, dora)
build-all:
    cargo build --workspace --all-targets

# Build in release mode
release:
    cargo build --workspace --release

# Run all tests (default members)
test: build
    cargo test --workspace

# Run clippy lints on the entire workspace
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting without changing files
fmt-check:
    cargo fmt --all -- --check

# Full CI check: format, clippy, build, test
ci: fmt-check clippy build test

# Verify no_std builds (protocol bare + create-oi bare + embassy for embedded target)
check-nostd:
    cargo build -p create-oi-protocol --no-default-features
    cargo build -p create-oi --no-default-features
    cargo build -p create-oi-embassy --target thumbv7em-none-eabihf
    cargo build -p create-oi --no-default-features --target thumbv7em-none-eabihf

# Clean build artifacts
clean:
    cargo clean

# Generate documentation for all workspace crates
doc:
    cargo doc --workspace --no-deps --open

# Check the workspace compiles without producing artifacts
check:
    cargo check --workspace --all-targets
