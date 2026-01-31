# Instruction Decoder
set dotenv-load

export SBF_OUT_DIR := "target/deploy"

default:
    @just --list

# === Build ===
build:
    cargo build --workspace

build-sbf:
    cd examples/counter && cargo build-sbf

# === Test ===
test: build-sbf
    cargo test --workspace

# === Lint & Format ===
lint:
    cargo +nightly fmt --all -- --check
    cargo clippy --workspace \
        --no-deps \
        -- -A unexpected-cfgs \
           -D warnings

format:
    cargo +nightly fmt --all

# === Clean ===
clean:
    cargo clean

# === Info ===
info:
    @echo "Solana: $(solana --version)"
    @echo "Rust: $(rustc --version)"
