---
name: dev-setup
description: Development environment setup - Rust toolchain, cross-compilation targets, dependencies, git hooks, IDE config.
disable-model-invocation: true
argument-hint: "[tools|deps|targets|hooks]"
---

# Development Environment Setup

Setup focus: $ARGUMENTS

## Step 1: Rust Toolchain

```bash
rustup update stable
rustup default stable
rustup component add rustfmt clippy
cargo install cargo-watch cargo-edit cargo-outdated cargo-audit cargo-tarpaulin
```

## Step 2: Cross-Compilation Targets

```bash
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-unknown-linux-musl
rustup target add x86_64-pc-windows-msvc
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

## Step 3: Dependencies

```bash
cargo update
cargo outdated
cargo audit
```

## Step 4: Git Hooks

Create `.git/hooks/pre-commit` with format check, clippy, and test gates. Make executable with `chmod +x`.

## Step 5: Development Tools

```bash
cargo install tokei ripgrep cargo-expand cargo-criterion
```

## Step 6: Environment Variables

```bash
export RUST_BACKTRACE=1
export RUST_LOG=claudelytics=debug
```

## Quick Commands

| Task | Command |
|------|---------|
| Watch tests | `cargo watch -x test` |
| Debug run | `RUST_LOG=debug cargo run -- daily` |
| Benchmark | `cargo bench --bench parser_bench` |
| Docs | `cargo doc --no-deps --open` |
| Macro expand | `cargo expand models` |

## Verification

```bash
cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo build --release
```
