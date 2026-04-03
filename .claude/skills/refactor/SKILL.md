---
name: refactor
description: Systematic code refactoring - analysis, pattern detection, incremental changes with verification. Use for code improvement and tech debt reduction.
argument-hint: "[module-name or area]"
---

# Code Refactoring Workflow

Refactor target: $ARGUMENTS

## Step 1: Code Analysis (parallel)

```bash
cargo clippy -- -W clippy::all -W clippy::pedantic
tokei src/
rg "TODO|FIXME|HACK" src/
```

## Step 2: Pattern Detection

Identify these code smells:
- **Duplicate code**: Similar logic in multiple places
- **Long functions**: Functions > 50 lines
- **Complex types**: Structs with > 10 fields
- **Deep nesting**: > 3 levels of indentation
- **Magic numbers**: Hardcoded values without constants
- **Inconsistent error handling**: Mixed error patterns

## Step 3: Refactoring Plan

Prioritize targets and plan changes:
1. Module organization (split large files)
2. Function decomposition (single responsibility)
3. Type safety improvements (enums over strings, newtypes)
4. Performance optimizations (iterators, reduce allocations)

## Step 4: Incremental Refactoring

For each change:
1. Create a focused commit
2. Run tests after each change: `cargo test`
3. Verify no behavior changes
4. Update documentation if needed

## Step 5: Verification

```bash
cargo test
cargo clippy -- -D warnings
git diff --stat
```

## Guidelines

- Never change behavior and refactor in the same commit
- Run tests after every change
- Keep commits small and focused
- If unsure about a refactoring, ask before proceeding
