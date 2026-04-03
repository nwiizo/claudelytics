---
paths: ["src/**"]
description: Claudelytics-specific Rust safety constraints (supplements global rust.md)
---

# Rust Safety (project-specific)

## Integer overflow
- Use `checked_*` or `saturating_*` for arithmetic on token counts and costs
- Cast with `TryFrom` / `TryInto` instead of `as` for narrowing conversions

## Unsafe
- Avoid `unsafe` entirely in this codebase (no justification exists)
- If ever added, require `// SAFETY:` comment explaining invariants

## Dependency auditing
- Run `cargo audit` before releases
- Run `cargo deny check` if deny.toml exists
