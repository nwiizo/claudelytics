---
name: qa-pipeline
description: Comprehensive quality assurance - tests, linting, formatting, code analysis, and release build verification.
disable-model-invocation: true
argument-hint: "[analyze|test|lint|fix]"
---

# Quality Assurance Pipeline

QA focus: $ARGUMENTS

## Step 1: Project Analysis

- Analyze codebase structure and dependencies
- Check for outdated dependencies: `cargo outdated`
- Verify module organization

## Step 2: Quality Checks (parallel)

```bash
cargo test --verbose
cargo fmt --check
cargo clippy -- -D warnings
```

## Step 3: Issue Resolution (based on findings)

- Fix formatting issues: `cargo fmt`
- Fix clippy warnings: apply suggested fixes
- Debug and correct failing tests
- Update dependencies if needed: `cargo update`

## Step 4: Final Verification

```bash
cargo test --release
cargo build --release
```

## Step 5: Documentation Check

- Verify public APIs have documentation
- Check for missing examples in doc comments
- Update CHANGELOG.md if needed

## Checklist

- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code properly formatted
- [ ] Release build succeeds
- [ ] Documentation complete
