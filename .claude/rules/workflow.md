---
paths: ["src/**", "Cargo.toml"]
description: Pre-commit quality checks and development workflow
---

# Development Workflow

## Quality check order (run before commit)
1. `cargo fmt`
2. `cargo clippy -- -D warnings`
3. `cargo test`
4. `similarity-rs ./src --min-tokens 50`

## Post-commit
- Review similarity report to plan refactoring
- Consider extracting common patterns into shared modules
