set shell := ["bash", "-cu"]

# Default: list available recipes
default:
    @just --list

# Build with default features (serial)
build:
    cargo build

# Build with all features
build-all:
    cargo build --all-features

# Build in release mode
release:
    cargo build --release

# Run all tests (default features)
test: build
    cargo test

# Run tests with all features
test-all: build-all
    cargo test --all-features

# Run clippy lints
clippy:
    cargo clippy --all-targets -- -D warnings

# Run clippy with all features
clippy-all:
    cargo clippy --all-targets --all-features -- -D warnings

# Format code
fmt:
    cargo fmt --all

# Check formatting without changing files
fmt-check:
    cargo fmt --all -- --check

# Full CI check: format, clippy, build, test
ci: fmt-check clippy build test

# Full CI with all features
ci-all: fmt-check clippy-all build-all test-all

# Clean build artifacts
clean:
    cargo clean

# Generate documentation
doc:
    cargo doc --no-deps --open

# Generate docs with all features
doc-all:
    cargo doc --no-deps --all-features --open

# Check the crate compiles without producing artifacts
check:
    cargo check

# Check with all features
check-all:
    cargo check --all-features
