---
name: deploy-check
description: Pre-release verification workflow - version, security audit, performance, cross-platform builds, documentation.
disable-model-invocation: true
argument-hint: "[security|perf|docs|release]"
---

# Deploy/Release Check

Run pre-release verification. Focus area: $ARGUMENTS

## Step 1: Version & Changelog

```bash
grep version Cargo.toml
```

- Verify version is bumped appropriately
- Ensure CHANGELOG.md is updated with release notes

## Step 2: Security Audit

```bash
cargo audit
cargo outdated --exit-code 1
```

## Step 3: Performance Verification (parallel)

```bash
cargo build --release
ls -lh target/release/claudelytics
cargo bench
```

## Step 4: Cross-Platform Build Test (parallel)

```bash
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-musl
cargo build --release --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

## Step 5: Documentation

- README.md is up to date
- All new features are documented
- `cargo doc --no-deps` succeeds

## Step 6: Release Assets

```bash
shasum -a 256 target/release/claudelytics-*
```

## Final Checklist

- [ ] All tests pass in release mode (`cargo test --release`)
- [ ] No security vulnerabilities
- [ ] Documentation complete
- [ ] Version bumped
- [ ] CHANGELOG.md updated
- [ ] Release binaries build successfully
