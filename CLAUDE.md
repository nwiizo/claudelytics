# CLAUDE.md

## Project Overview

Claudelytics is a Rust CLI tool for analyzing Claude Code usage patterns and costs.
Parses JSONL from `~/.claude/projects/` and `~/.config/claude/projects/`.

**Current Version**: 0.6.2

## Commands

```bash
cargo fmt && cargo clippy -- -D warnings && cargo test
similarity-rs ./src --min-tokens 50   # post-implementation duplication check
cargo test -- --ignored               # integration tests with sample data
```

## Rules

- [workflow](.claude/rules/workflow.md) — similarity scanning, post-commit actions
- [rust-safety](.claude/rules/rust-safety.md) — integer overflow, unsafe, cargo audit

## Agents

- [cargo-error-resolver](.claude/agents/cargo-error-resolver.md) — build error diagnosis and fixing
- [refactor-cleaner](.claude/agents/refactor-cleaner.md) — dead code, unused deps, duplication cleanup
