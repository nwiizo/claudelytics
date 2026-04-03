# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claudelytics is a Rust CLI tool for analyzing Claude Code usage patterns and costs. It parses JSONL files from `~/.claude/projects/` and `~/.config/claude/projects/` directories and generates comprehensive reports on token usage, costs, and session analytics.

**Current Version**: 0.6.0

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
- `models.rs` - Data structures for sessions, conversations, and reports
- `parser.rs` - JSONL parsing with multi-directory support and cost mode routing
- `pricing.rs` - Model pricing with tiered pricing and fast mode multiplier
- `reports.rs` - Report generation (daily, weekly, monthly, session)
- `models_registry.rs` - Model registration with aliases and family matching
- `pricing_cache.rs` - Offline pricing data management
- `tui.rs` - Terminal UI implementation with conversation viewing
- `conversation_parser.rs` - Full conversation content parsing
- `conversation_display.rs` - Conversation formatting and display
- `live_dashboard.rs` - Real-time token burn rate monitoring

## Key Features

1. **5-Hour Billing Blocks**: Sessions are automatically grouped into 5-hour blocks for cost calculation
2. **Offline Pricing**: Pricing data with tiered pricing (200k threshold for 1M context models)
3. **Interactive TUI**: Rich terminal interface for exploring usage data
4. **Flexible Reporting**: Daily, weekly, monthly, session reports with multiple formats
5. **Fast Mode Support**: Detects `/fast` mode usage (6x pricing multiplier)
6. **Cost Modes**: `auto`/`calculate`/`display` for cost calculation control
7. **XDG Support**: Reads from both `~/.claude/` and `~/.config/claude/` directories
8. **Model Coverage**: Claude 3.x, 4, 4.5, 4.6 models with accurate pricing
9. **Conversation Viewing**: Full conversation content display with thinking blocks and tool usage
10. **Live Dashboard**: Real-time token usage monitoring with burn rate calculations

## Testing

Run the test suite with:
```bash
cargo test
```

For integration tests with sample data:
```bash
cargo test -- --ignored
```