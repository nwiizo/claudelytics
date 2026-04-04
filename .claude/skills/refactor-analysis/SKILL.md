---
name: refactor-analysis
description: Run cargo-coupling and similarity-rs to identify refactoring targets, then create a prioritized plan.
---

# Refactor Analysis

Analyze code coupling and duplication, then create a prioritized refactoring plan.

## Steps

1. **Coupling analysis**:
   ```bash
   cargo coupling --exclude-tests --ai --hotspots=10
   ```
   Focus on: High Afferent (too many dependents), God Modules, circular deps.

2. **Similarity scan**:
   ```bash
   similarity-rs ./src --min-tokens 50
   ```
   Focus on: 95%+ similarity pairs, extract shared functions.

3. **Combine findings** into a prioritized plan:
   - High: modules with >20 dependents, circular deps
   - Medium: God modules (>30 functions or >500 lines), 100% duplicates
   - Low: 90%+ similarity, primitive obsession

4. **Create GitHub issues** for each refactoring target with `refactoring` label.

5. **Execute** high-priority items if requested, verifying with `cargo test` after each change.
