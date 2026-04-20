set shell := ["bash", "-cu"]

# Default: list available recipes
default:
    @just --list

# Build the entire workspace
build:
    cargo build --workspace

# Build in release mode
release:
    cargo build --workspace --release

# Run all tests
test: build
    cargo test --workspace

# Run clippy lints
clippy:
    cargo clippy --workspace -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting without changing files
fmt-check:
    cargo fmt --all -- --check

# Full CI check: format, clippy, build, test
ci: fmt-check clippy build test

# Build with zig c++ (requires ZIG_CXX wrapper script)
build-zig:
    ZIG_CXX="{{justfile_directory()}}/scripts/zig-cxx.sh" cargo build --workspace

# Clean build artifacts
clean:
    cargo clean

# Generate documentation
doc:
    cargo doc --workspace --no-deps --open

# Check the workspace compiles without producing artifacts
check:
    cargo check --workspace
