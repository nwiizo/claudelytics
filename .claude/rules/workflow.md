---
paths: ["src/**", "Cargo.toml"]
description: Claudelytics-specific workflow (supplements global rust.md quality commands)
---

# Development Workflow (project-specific)

## Similarity scanning (additional pre-commit step)
- After standard quality checks: `similarity-rs ./src --min-tokens 50`
- Add `--skip-test` to exclude test functions
- Create issues for significant duplication found

## Post-commit
- Review similarity report to plan refactoring
- Consider extracting common patterns into shared modules
