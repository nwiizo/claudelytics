---
name: bug-fix
description: Systematic bug investigation and fix workflow. Use when fixing bugs, investigating issues, or debugging test failures.
argument-hint: "[issue-number or description]"
---

# Bug Fix Workflow

Fix $ARGUMENTS using this systematic process.

## Step 1: Investigation (parallel)

- Search for related issues: `gh issue list --label bug`
- Search error patterns: `rg "unwrap\(\)|expect\(" src/` and `rg "TODO|FIXME" src/`
- Review recent commits for potential causes: `git log --oneline -20`

## Step 2: Reproduction

1. Extract error messages and affected context
2. Create a minimal failing test:
   ```rust
   #[test]
   fn test_reproduce_issue() {
       // Minimal reproduction
   }
   ```
3. Verify the issue: `cargo test test_reproduce_issue -- --nocapture`

## Step 3: Root Cause Analysis

Common categories in this project:
- **Parsing**: Malformed JSONL handling
- **Date/Time**: Timezone or format issues
- **Paths**: Cross-platform path handling
- **Memory**: Large file processing
- **Display**: Terminal width/color issues

## Step 4: Fix Implementation

1. Add a failing test first
2. Implement the fix
3. Verify the test passes
4. Check for regressions

## Step 5: Verification

```bash
cargo test <specific_test>
cargo test
cargo test --all-features
```

## Step 6: Regression Prevention

Add a dedicated regression test to prevent recurrence:
```rust
#[cfg(test)]
mod regression_tests {
    #[test]
    fn issue_NNN_description() {
        // Ensure this specific issue doesn't reoccur
    }
}
```

## Checklist

- [ ] Issue reproduced locally
- [ ] Root cause identified
- [ ] Fix implemented with tests
- [ ] No regressions introduced
- [ ] Performance impact assessed
