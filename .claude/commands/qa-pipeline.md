# Quality Assurance Pipeline

Comprehensive quality check workflow for claudelytics project.

## Command: /project:qa-pipeline

### Usage
```bash
/project:qa-pipeline              # Full QA check
/project:qa-pipeline analyze      # Code analysis only
/project:qa-pipeline test         # Test suite only
/project:qa-pipeline lint         # Linting only
/project:qa-pipeline fix          # Auto-fix issues
```

### Pipeline Steps

#### Step 1: Project Analysis (1 task)
- Analyze codebase structure and dependencies
- Check for outdated dependencies
- Verify module organization

#### Step 2: Quality Checks (3 parallel tasks)
```bash
# Parallel execution
cargo test --verbose | cargo fmt --check | cargo clippy -- -D warnings
```

#### Step 3: Issue Resolution (parallel tasks based on findings)
- Fix formatting issues: `cargo fmt`
- Fix clippy warnings: Apply suggested fixes
- Update failing tests: Debug and correct test cases
- Update dependencies: `cargo update` if needed

#### Step 4: Final Verification
```bash
# Sequential verification
cargo test --release
cargo build --release
git status
```

#### Step 5: Documentation Check
- Verify all public APIs have documentation
- Check for missing examples in doc comments
- Update CHANGELOG.md if needed

### Success Criteria
- ✅ All tests pass
- ✅ No clippy warnings
- ✅ Code properly formatted
- ✅ Release build succeeds
- ✅ Documentation complete

### Time Savings
Manual execution: ~25-30 minutes
Automated pipeline: ~5-7 minutes
**Time saved: 20+ minutes per check**

### Example Output
```
🔍 Starting QA Pipeline...
✅ Step 1: Project analysis complete
✅ Step 2: Running quality checks (parallel)
   - Tests: 42 passed, 0 failed
   - Format: All files formatted correctly
   - Clippy: No warnings found
✅ Step 3: No issues to fix
✅ Step 4: Final verification passed
✅ Step 5: Documentation complete

🎉 QA Pipeline completed successfully!
Total time: 5m 23s
```