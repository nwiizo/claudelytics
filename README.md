# Claudelytics

[![Crates.io](https://img.shields.io/crates/v/claudelytics.svg)](https://crates.io/crates/claudelytics)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/nwiizo/claudelytics/workflows/CI/badge.svg)](https://github.com/nwiizo/claudelytics/actions)

A fast, parallel Rust CLI tool for analyzing Claude Code usage patterns, token consumption, and costs. Get comprehensive insights into your Claude Code usage with beautiful table outputs and JSON export capabilities.

## Features

- **Daily Reports**: Analyze usage patterns by day with token counts and costs
- **Session Reports**: Break down usage by individual Claude Code sessions
- **Flexible Filtering**: Filter data by date ranges
- **Multiple Output Formats**: Table view or JSON output
- **Fast Performance**: Parallel processing for large datasets
- **Error Handling**: Robust error handling with helpful messages

## Installation

### From Crates.io (Recommended)

```bash
cargo install claudelytics
```

### From Source

```bash
git clone https://github.com/nwiizo/claudelytics.git
cd claudelytics
cargo install --path .
```

### Download Binary

Download pre-built binaries from the [releases page](https://github.com/nwiizo/claudelytics/releases).

### Homebrew (macOS)

```bash
# Coming soon
brew install nwiizo/tap/claudelytics
```

## Usage

### Basic Commands

```bash
# Show daily usage report (default)
claudelytics daily

# Show session-based usage report
claudelytics session

# Get help
claudelytics --help
```

### Filtering by Date

```bash
# Show usage since January 1, 2024
claudelytics daily --since 20240101

# Show usage until January 31, 2024
claudelytics daily --until 20240131

# Show usage for a specific date range
claudelytics daily --since 20240101 --until 20240131
```

### Custom Claude Directory

```bash
# Use a custom Claude directory path
claudelytics daily --path /custom/path/to/.claude
```

### JSON Output

```bash
# Output in JSON format
claudelytics daily --json
claudelytics session --json
```

## Data Structure

Claudelytics expects Claude Code data to be stored in the following structure:

```
~/.claude/
└── projects/
    ├── project-name-1/
    │   ├── session-abc123/
    │   │   └── chat_2024_01_01.jsonl
    │   └── session-def456/
    │       └── chat_2024_01_02.jsonl
    └── project-name-2/
        └── nested/
            └── path/
                └── session-ghi789/
                    └── chat_2024_01_03.jsonl
```

### JSONL Record Format

Each line in the JSONL files should contain:

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

## Output Examples

### Daily Report (Table)

```
╭────────────┬──────────────┬───────────────┬─────────────────┬────────────┬──────────────┬─────────────╮
│ Date       │ Input Tokens │ Output Tokens │ Cache Creation  │ Cache Read │ Total Tokens │ Cost (USD)  │
├────────────┼──────────────┼───────────────┼─────────────────┼────────────┼──────────────┼─────────────┤
│ 2024-01-03 │ 5,000        │ 10,000        │ 1,000           │ 2,000      │ 18,000       │ $0.85       │
│ 2024-01-02 │ 3,000        │ 6,000         │ 500             │ 1,000      │ 10,500       │ $0.50       │
│ 2024-01-01 │ 2,000        │ 4,000         │ 300             │ 700        │ 7,000        │ $0.35       │
├────────────┼──────────────┼───────────────┼─────────────────┼────────────┼──────────────┼─────────────┤
│ Total      │ 10,000       │ 20,000        │ 1,800           │ 3,700      │ 35,500       │ $1.70       │
╰────────────┴──────────────┴───────────────┴─────────────────┴────────────┴──────────────┴─────────────╯
```

### Session Report (Table)

```
╭─────────────────────┬─────────────────────┬──────────────┬───────────────┬─────────────────┬────────────┬──────────────┬─────────────┬───────────────╮
│ Project Path        │ Session ID          │ Input Tokens │ Output Tokens │ Cache Creation  │ Cache Read │ Total Tokens │ Cost (USD)  │ Last Activity │
├─────────────────────┼─────────────────────┼──────────────┼───────────────┼─────────────────┼────────────┼──────────────┼─────────────┼───────────────┤
│ project-1/feature-a │ session-abc123      │ 5,000        │ 10,000        │ 1,000           │ 2,000      │ 18,000       │ $0.85       │ 2024-01-03    │
│ project-2/bug-fix   │ session-def456      │ 3,000        │ 6,000         │ 500             │ 1,000      │ 10,500       │ $0.50       │ 2024-01-02    │
├─────────────────────┼─────────────────────┼──────────────┼───────────────┼─────────────────┼────────────┼──────────────┼─────────────┼───────────────┤
│ Total               │                     │ 8,000        │ 16,000        │ 1,500           │ 3,000      │ 28,500       │ $1.35       │               │
╰─────────────────────┴─────────────────────┴──────────────┴───────────────┴─────────────────┴────────────┴──────────────┴─────────────┴───────────────╯
```

### JSON Output

```json
{
  "daily": [
    {
      "date": "2024-01-03",
      "inputTokens": 5000,
      "outputTokens": 10000,
      "cacheCreationTokens": 1000,
      "cacheReadTokens": 2000,
      "totalTokens": 18000,
      "totalCost": 0.85
    }
  ],
  "totals": {
    "inputTokens": 15000,
    "outputTokens": 30000,
    "cacheCreationTokens": 3000,
    "cacheReadTokens": 6000,
    "totalTokens": 54000,
    "totalCost": 2.55
  }
}
```

## Error Handling

### Common Errors

- **Claude directory not found**: Ensure Claude Code is installed and has been used at least once
- **No usage data found**: Check if the date range contains actual usage data
- **Invalid date format**: Use YYYYMMDD format (e.g., 20240101)
- **Permission errors**: Ensure read access to the Claude directory

### Exit Codes

- `0`: Success
- `1`: General error
- `2`: CLI usage error
- `3`: File system error

## Performance

- Handles 10,000+ JSONL files efficiently
- Parallel processing for optimal performance
- Memory usage optimized for large datasets
- Fast startup time

## Development

### Running Tests

```bash
cargo test
```

### Running with Cargo

```bash
# Development build
cargo run -- daily --help

# Release build
cargo run --release -- session --json
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## License

[Add your license information here]