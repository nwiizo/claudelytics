# Code Refactoring Workflow

Systematic code improvement process for claudelytics.

## Command: /project:refactor

### Usage
```bash
/project:refactor                  # Interactive refactoring assistant
/project:refactor analyze          # Code smell detection
/project:refactor suggest          # Improvement suggestions
/project:refactor module <name>    # Refactor specific module
/project:refactor perf             # Performance-focused refactoring
```

### Refactoring Process

#### Step 1: Code Analysis (3 parallel tasks)
```bash
# Identify improvement opportunities
cargo clippy -- -W clippy::all -W clippy::pedantic
tokei src/                         # Code statistics
rg "TODO|FIXME|HACK" src/         # Technical debt markers
```

#### Step 2: Pattern Detection
Common patterns to identify:
- **Duplicate code**: Similar logic in multiple places
- **Long functions**: Functions > 50 lines
- **Complex types**: Structs with > 10 fields
- **Deep nesting**: > 3 levels of indentation
- **Magic numbers**: Hardcoded values without constants
- **Error handling**: Inconsistent error patterns

#### Step 3: Refactoring Targets
1. **Module Organization**
   ```rust
   // Before: Everything in one file
   // After: Logical module separation
   - models/usage.rs
   - models/analytics.rs
   - models/patterns.rs
   ```

2. **Function Decomposition**
   ```rust
   // Extract complex logic into smaller functions
   // Apply single responsibility principle
   ```

3. **Type Safety Improvements**
   ```rust
   // Replace stringly-typed code with enums
   // Use NewType pattern for domain types
   ```

4. **Performance Optimizations**
   ```rust
   // Identify unnecessary allocations
   // Convert to iterators where possible
   // Use Cow for flexible ownership
   ```

#### Step 4: Incremental Refactoring
```bash
# For each refactoring:
1. Create focused commit
2. Run tests after each change
3. Verify no behavior changes
4. Update documentation
```

#### Step 5: Verification
```bash
# Ensure refactoring preserves behavior
cargo test
cargo bench -- --baseline before_refactor
git diff --stat  # Review scope of changes
```

### Common Refactorings for Claudelytics

#### 1. Extract Analytics Traits
```rust
// Before: Concrete implementations scattered
// After: Trait-based analytics system
trait AnalyticsProvider {
    fn calculate_metrics(&self) -> Result<Metrics>;
    fn generate_insights(&self) -> Result<Vec<Insight>>;
}
```

#### 2. Simplify Error Handling
```rust
// Before: Multiple error types
// After: Unified error type with thiserror
#[derive(Error, Debug)]
pub enum ClaudelyticsError {
    #[error("Parse error: {0}")]
    Parse(#[from] serde_json::Error),
    // ...
}
```

#### 3. Parallel Processing Enhancement
```rust
// Before: Sequential processing
// After: Rayon parallel iterators
records.par_iter()
    .map(|r| process_record(r))
    .collect()
```

### Success Metrics
- 📊 Code complexity reduced by X%
- 🚀 Performance improved by Y%
- 📝 Test coverage maintained/improved
- 🔍 Zero behavior regressions

### Time Savings
Manual refactoring analysis: ~2-3 hours
Automated assistance: ~30-45 minutes
**Time saved: 1.5+ hours per refactoring session**

### Example Session
```
🔧 Starting Refactoring Analysis...
✅ Step 1: Code analysis complete
   - 5 long functions identified
   - 3 duplicate code blocks found
   - 2 complex types could be simplified

📋 Suggested Refactorings:
1. Extract session parsing logic (priority: high)
2. Consolidate error handling (priority: medium)
3. Simplify TokenUsage aggregation (priority: medium)

Proceed with refactoring? (y/n)
```