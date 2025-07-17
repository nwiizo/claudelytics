# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claudelytics is a Rust CLI tool for analyzing Claude Code usage patterns and costs. It parses JSONL files from the `~/.claude/projects/` directory structure and generates comprehensive reports on token usage, costs, and session analytics.

**Current Version**: 0.5.1 - Enhanced with live dashboard, responsive tables, real-time analytics, and quick install

## Similarity Scanning Strategies

- Use `similarity-rs` for checking code similarity in Rust files
  - Check Rust files in `./src` directory
  - Skip test functions with `test_` prefix or `#[test]` annotation
  - Set minimum tokens to 50 for more meaningful similarity detection
  - Example commands:
    ```bash
    # Check Rust files
    similarity-rs ./src

    # Skip test functions (test_ prefix or #[test])
    similarity-rs . --skip-test

    # Set minimum tokens (default: 30)
    similarity-rs . --min-tokens 50 
    ```

## Development Workflow

### Code Quality Checks

After implementing new features or making changes, run the following checks:

1. **Format and lint the code:**
   ```bash
   cargo fmt
   cargo clippy -- -D warnings
   ```

2. **Run tests:**
   ```bash
   cargo test
   ```

3. **Check for code similarity:**
   ```bash
   # Run similarity check after implementation
   similarity-rs ./src --min-tokens 50
   ```

### Pre-Commit Checklist

Before committing changes, ensure:

1. All tests pass
2. Code is properly formatted
3. No clippy warnings
4. **Run similarity check to identify duplicate code:**
   ```bash
   similarity-rs ./src --min-tokens 50
   ```

### Post-Commit Actions

After committing:

1. **Review similarity report to plan refactoring:**
   ```bash
   similarity-rs ./src --min-tokens 50
   ```
2. Create issues for any significant duplication found
3. Consider extracting common patterns into shared modules

## Architecture

The project is organized into several modules:

- `main.rs` - CLI entry point and command handling
- `models.rs` - Data structures for sessions, conversations, and model pricing
- `parser.rs` - JSONL parsing logic
- `reports.rs` - Report generation and formatting
- `pricing_cache.rs` - Offline pricing data management
- `tui.rs` - Terminal UI implementation with conversation viewing
- `session_analytics.rs` - Session analysis and statistics
- `conversation_parser.rs` - Full conversation content parsing (NEW)
- `conversation_display.rs` - Conversation formatting and display (NEW)
- `live_dashboard.rs` - Real-time token burn rate monitoring (NEW)

## Key Features

1. **5-Hour Billing Blocks**: Sessions are automatically grouped into 5-hour blocks for cost calculation
2. **Offline Pricing**: Pricing data is cached locally to avoid API dependencies
3. **Interactive TUI**: Rich terminal interface for exploring usage data
4. **Flexible Reporting**: Multiple report formats and filtering options
5. **Conversation Viewing**: Full conversation content display with thinking blocks and tool usage (NEW)
   - View complete conversation threads with parent/child relationships
   - Export conversations in markdown, JSON, or text format
   - Search within conversation content
   - Filter by thinking blocks and tool usage
   - Enhanced TUI with dedicated Conversations tab
6. **Live Dashboard**: Real-time token usage monitoring with burn rate calculations (NEW)
   - Real-time token burn rate (tokens/minute, tokens/hour)
   - Active session progress tracking
   - Cost projections based on current usage rate
   - Time to reach daily/monthly limits
   - Configurable alerts for high usage
   - Auto-refresh display with customizable interval

## Testing

Run the test suite with:
```bash
cargo test
```

For integration tests with sample data:
```bash
cargo test -- --ignored
```