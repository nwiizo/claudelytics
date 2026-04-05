---
name: pre-release-check
description: Run full quality gate before release — fmt, clippy, test, mutants, similarity, coupling.
---

# Pre-Release Check

Full quality gate to run before tagging a release.

## Steps (sequential)

1. **Basic quality**:
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   ```

2. **Mutation testing** (core modules):
   ```bash
   cargo mutants --timeout 60 --jobs 4 \
     --file src/parser.rs --file src/reports.rs \
     --file src/models/ --file src/pricing.rs \
     --file src/billing_blocks.rs
   ```
   Review missed mutants. Add tests for any critical logic that survived mutation.

3. **Code similarity**:
   ```bash
   similarity-rs ./src --min-tokens 50
   ```
   Flag any 95%+ duplicates that weren't addressed.

4. **Coupling analysis**:
   ```bash
   cargo coupling --exclude-tests --check --min-grade C
   ```
   Ensure no regression from previous grade.

5. **Dry-run publish**:
   ```bash
   cargo publish --dry-run
   ```

6. **Report**: Summarize pass/fail for each gate. Block release if mutants expose critical untested logic.
