# Bug Fix Workflow

Systematic approach to identifying and fixing issues in claudelytics.

## Command: /project:bug-fix

### Usage
```bash
/project:bug-fix                   # Interactive bug fixing
/project:bug-fix issue <number>    # Fix specific GitHub issue
/project:bug-fix analyze           # Analyze potential bugs
/project:bug-fix test <module>     # Debug specific module tests
/project:bug-fix reproduce         # Help reproduce reported issue
```

### Bug Fix Process

#### Step 1: Issue Investigation (parallel tasks)
```bash
# Check for existing issues
gh issue list --label bug

# Search error patterns in code
rg "unwrap\(\)|expect\(" src/ --stats
rg "panic!|todo!|unimplemented!" src/

# Review recent commits for potential causes
git log --oneline -20
```

#### Step 2: Reproduction Strategy
1. **Parse Error Reports**
   - Extract error messages
   - Identify affected versions
   - Note environment details

2. **Create Minimal Reproduction**
   ```rust
   #[test]
   fn test_reproduce_issue_xyz() {
       // Minimal code to reproduce
   }
   ```

3. **Verify Issue Exists**
   ```bash
   cargo test test_reproduce_issue_xyz -- --nocapture
   ```

#### Step 3: Root Cause Analysis
Common bug categories in claudelytics:
- **Parsing Issues**: Malformed JSONL handling
- **Date/Time Bugs**: Timezone or format issues
- **Path Problems**: Cross-platform path handling
- **Memory Issues**: Large file processing
- **Display Bugs**: Terminal width/color issues

#### Step 4: Fix Implementation
```rust
// Before implementing fix:
// 1. Add failing test
// 2. Implement fix
// 3. Verify test passes
// 4. Check for regressions
```

#### Step 5: Comprehensive Testing
```bash
# Run targeted tests
cargo test <specific_test>

# Run all tests
cargo test

# Test with different features
cargo test --all-features

# Test in release mode
cargo test --release
```

#### Step 6: Regression Prevention
```rust
// Add comprehensive test coverage
#[cfg(test)]
mod regression_tests {
    #[test]
    fn issue_123_large_file_parsing() {
        // Ensure this specific issue doesn't reoccur
    }
}
```

### Common Bugs & Fixes

#### 1. JSONL Parsing Errors
```rust
// Problem: Fails on malformed JSON
// Fix: Skip invalid lines with error logging
match serde_json::from_str::<UsageRecord>(&line) {
    Ok(record) => records.push(record),
    Err(e) => {
        log::warn!("Skipping invalid line: {}", e);
        continue;
    }
}
```

#### 2. Cross-Platform Path Issues
```rust
// Problem: Hardcoded path separators
// Fix: Use PathBuf and proper joining
let session_path = base_path
    .join("projects")
    .join(&project_name)
    .join("sessions");
```

#### 3. Terminal Display Issues
```rust
// Problem: Assumes terminal width
// Fix: Dynamic width detection
let term_width = terminal_size()
    .map(|(w, _)| w.0 as usize)
    .unwrap_or(80);
```

#### 4. Memory Efficiency
```rust
// Problem: Loading entire file into memory
// Fix: Stream processing
use std::io::{BufRead, BufReader};
let reader = BufReader::new(file);
for line in reader.lines() {
    // Process line by line
}
```

### Bug Fix Checklist
- [ ] Issue reproduced locally
- [ ] Root cause identified
- [ ] Fix implemented with tests
- [ ] No regressions introduced
- [ ] Documentation updated if needed
- [ ] Performance impact assessed
- [ ] Cross-platform compatibility verified

### Success Metrics
- 🐛 Bug successfully fixed
- ✅ All tests passing
- 📈 No performance regression
- 🔒 No new security issues
- 📝 Issue properly documented

### Time Savings
Manual bug investigation: ~1-2 hours
Systematic approach: ~20-30 minutes
**Time saved: 40+ minutes per bug**

### Example Session
```
🔍 Starting Bug Fix Workflow...
✅ Step 1: Found issue #123 - "Panic on empty JSONL files"
✅ Step 2: Reproduction created
   - Created failing test case
   - Verified issue exists
✅ Step 3: Root cause identified
   - unwrap() on empty iterator
✅ Step 4: Fix implemented
   - Added proper error handling
✅ Step 5: Tests passing
   - All tests green
   - No regressions
✅ Step 6: Regression test added

🎉 Bug fixed successfully!
Ready to commit with message:
"fix: Handle empty JSONL files gracefully (#123)"
```