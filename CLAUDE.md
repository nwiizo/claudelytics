# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claudelytics is a Rust CLI tool for analyzing Claude Code usage patterns and costs. It parses JSONL files from the `~/.claude/projects/` directory structure and generates comprehensive reports on token usage, costs, and session analytics.

## Build and Development Commands

```bash
# Build the project
cargo build

# Build for release (optimized)
cargo build --release

# Run the project with arguments
cargo run -- daily --help
cargo run -- session --json

# Run tests
cargo test

# Run with release optimizations
cargo run --release -- daily --since 20240101
```

## Code Architecture

### Module Structure
- **main.rs**: CLI entry point using clap for argument parsing
- **models.rs**: Core data structures for usage records, reports, and token aggregation
- **parser.rs**: JSONL file parsing with parallel processing using rayon
- **display.rs**: Output formatting (table and JSON) with colored terminal output
- **reports.rs**: Report generation logic for daily and session analytics

### Key Data Flow
1. **UsageParser** scans `~/.claude/projects/` recursively for `*.jsonl` files
2. **Parallel processing** parses multiple files concurrently using rayon
3. **TokenUsage aggregation** groups data by date and session paths
4. **Report generation** creates sorted daily/session reports
5. **Display formatting** outputs as colored tables or JSON

### Important Patterns
- Uses **HashMap<NaiveDate, TokenUsage>** for daily aggregation
- Uses **HashMap<String, (TokenUsage, DateTime<Utc>)>** for session tracking
- Session paths are extracted from file system structure relative to `projects/`
- Date filtering supports YYYYMMDD format for `--since` and `--until`
- Error handling uses anyhow for context-aware error messages

### Data Model
The tool expects Claude Code JSONL records with this structure:
```json
{
  "timestamp": "2024-01-01T12:34:56Z",
  "message": {
    "usage": {
      "input_tokens": 1000,
      "output_tokens": 2000,
      "cache_creation_input_tokens": 500,
      "cache_read_input_tokens": 300
    }
  },
  "costUSD": 0.15
}
```

### Performance Considerations
- Uses rayon for parallel file processing to handle large datasets efficiently
- BufReader for memory-efficient JSONL parsing
- Skips invalid JSON lines silently to handle corrupted data gracefully