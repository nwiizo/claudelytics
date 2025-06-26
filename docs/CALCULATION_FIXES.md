# Claudelytics Calculation Formula Fixes

## Summary

This document describes the fixes applied to calculation formulas in claudelytics to ensure accuracy and consistency.

## Issues Fixed

### 1. Monthly Token Projection (Fixed in `src/display.rs` line 421)

**Problem**: The monthly token projection was using a hardcoded calculation that didn't match actual projected data:
```rust
// Before (incorrect)
format_number(metrics_7d.projected_monthly_cost as u64 * 1000000 / 150)
```

**Solution**: Added `projected_monthly_tokens` field to `BurnRateMetrics` struct and properly calculated it:
```rust
// After (correct)
format_number(metrics_7d.projected_monthly_tokens)
```

### 2. Cache Efficiency Calculation Standardization

**Problem**: Cache efficiency calculation was inconsistent across the codebase, using different formulas:
- Some places: `cache_read_tokens / (input_tokens + cache_creation_tokens)`
- Other places: `cache_read_tokens / total_input`

**Solution**: Standardized to use the formula that represents the percentage of input that came from cache:
```rust
// Standardized formula
cache_efficiency = cache_read_tokens / (cache_read_tokens + input_tokens) * 100.0
```

This formula correctly represents what percentage of the total input (cache reads + new input) came from cached data.

### 3. Burn Rate Calculation Improvement

**Problem**: Burn rate was dividing daily usage uniformly by 24 hours, which doesn't reflect actual usage patterns.

**Solution**: Changed to distribute usage across active hours (9 hours, 9 AM to 6 PM) for more realistic burn rate:
```rust
const ACTIVE_HOURS_PER_DAY: u64 = 9; // Average active hours
// Distribute usage across typical working hours (9 AM to 6 PM)
```

## Technical Changes

### Files Modified

1. **`src/burn_rate.rs`**:
   - Added `projected_monthly_tokens: u64` field to `BurnRateMetrics` struct
   - Updated all places that construct `BurnRateMetrics` to include the new field
   - Changed hourly usage distribution from 24 hours to 9 active hours (9 AM - 6 PM)

2. **`src/display.rs`**:
   - Fixed line 421 to use `metrics_7d.projected_monthly_tokens` instead of hardcoded calculation
   - Standardized all cache efficiency calculations to use the consistent formula
   - Updated 7 occurrences of cache efficiency calculation

3. **`src/models.rs`**:
   - Updated `cache_efficiency()` method to return percentage (multiply by 100.0)
   - Changed formula to match the standardized approach

## Verification

All changes have been verified with:
- `cargo test` - All 42 tests pass
- `cargo fmt` - Code formatted according to Rust standards
- `cargo clippy -- -D warnings` - No warnings or errors

## Impact

These fixes ensure:
1. Accurate monthly token projections based on actual burn rate calculations
2. Consistent cache efficiency metrics across all displays
3. More realistic burn rate calculations that reflect typical usage patterns