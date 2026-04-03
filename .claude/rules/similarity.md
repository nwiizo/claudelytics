---
paths: ["src/**"]
description: Code similarity scanning conventions
---

# Similarity Scanning

- Use `similarity-rs ./src --min-tokens 50` to detect duplicate code
- Add `--skip-test` to exclude test functions
- Run after implementation and before commit
- Create issues for significant duplication found
