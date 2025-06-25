# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claudelytics is a Rust CLI tool for analyzing Claude Code usage patterns and costs. It parses JSONL files from the `~/.claude/projects/` directory structure and generates comprehensive reports on token usage, costs, and session analytics.

**Current Version**: 0.4.0 - Major release with 5-hour billing blocks, offline pricing cache, and enhanced TUI

## Build and Development Commands

```bash
# Build the project
cargo build

# Build for release (optimized)
cargo build --release

# Run the project with arguments
cargo run -- daily --help
cargo run -- session --json

# Run tests (REQUIRED before commits)
cargo test

# Format code (REQUIRED before commits)
cargo fmt

# Check formatting (for CI)
cargo fmt --check

# Run clippy linter (REQUIRED before commits)
cargo clippy -- -D warnings

# Run with release optimizations
cargo run --release -- daily --since 20240101

# Interactive mode (peco-style session selector)
cargo run -- interactive

# Watch mode for real-time monitoring
cargo run -- watch

# Enhanced Terminal User Interface (TUI) mode
cargo run -- tui
cargo run -- --tui                  # Alternative flag format

# Resume last TUI session
cargo run -- --resume               # Resume last TUI session with saved state

# 5-Hour Billing Blocks (matches Claude's actual billing periods)
cargo run -- billing-blocks         # Show billing blocks analysis
cargo run -- billing-blocks --json  # JSON output
cargo run -- billing-blocks --since 20240101  # Filter by date

# Pricing Cache Management
cargo run -- pricing-cache          # Show pricing cache status
cargo run -- pricing-cache --update # Update cache (placeholder for future API)
cargo run -- pricing-cache --clear  # Clear cached pricing data

# Analytics Studio TUI mode (data science features) - PLANNED
# cargo run -- analytics-tui         # Comprehensive analytics studio
# cargo run -- --analytics-tui       # Alternative flag format

# Export data to CSV
cargo run -- export --daily --sessions --summary -o report

# Configuration management
cargo run -- config --show
cargo run -- config --set-path /path/to/claude

# Monthly analytics
cargo run -- monthly                # Show monthly usage summary
cargo run -- monthly --json         # Monthly data in JSON format

# Cost display options
cargo run -- cost                    # Show total cost summary
cargo run -- cost --today           # Show today's cost only
cargo run -- cost --date 20240101   # Show cost for specific date
cargo run -- --today                # Show today's usage data
cargo run -- daily --today          # Show today's daily report


# Output format options
cargo run -- daily                  # Enhanced format (default)
cargo run -- daily --classic        # Classic table format
cargo run -- daily --json           # JSON output
cargo run -- session                # Enhanced session view
cargo run -- session --classic      # Classic session table

# Model filtering options
cargo run -- --model-filter opus     # Show only Opus model usage
cargo run -- --model-filter sonnet   # Show only Sonnet model usage
cargo run -- --model-filter opus4    # Filter by model alias
cargo run -- --list-models           # List all registered models
cargo run -- --model-filter opus --today daily  # Combine filters

# Model breakdown display
cargo run -- --by-model              # Show usage breakdown by model family
CLAUDELYTICS_TABLE_FORMAT=true cargo run -- --by-model  # Use table format for better alignment
CLAUDELYTICS_DISPLAY_FORMAT=table cargo run -- --by-model  # Alternative table format

# MCP server mode
cargo run -- mcp-server              # Start MCP server for IDE integration

# Advanced analytics
cargo run -- analytics               # Show all session analytics
cargo run -- analytics --time-of-day # Time of day usage patterns
cargo run -- analytics --day-of-week # Day of week patterns
cargo run -- analytics --duration    # Session duration analysis
cargo run -- analytics --frequency   # Session frequency and streaks
cargo run -- analytics --efficiency  # Cost efficiency analysis
```

## Pre-commit Requirements

**IMPORTANT**: Before committing any changes, you MUST run:

1. `cargo test` - Ensure all tests pass
2. `cargo fmt` - Format the code according to Rust standards

These checks are enforced in CI/CD and will cause the build to fail if not followed.


### Shell Integration

#### Bash (~/.bashrc or ~/.bash_profile)

```bash
# Add claudelytics aliases
alias ctoday='claudelytics cost --today'
alias csum='claudelytics cost'
alias cwt='claudelytics --today'
alias cwatch='claudelytics watch'
alias cint='claudelytics interactive'
alias ctui='claudelytics tui'
alias cresume='claudelytics --resume'
alias canalytics='claudelytics analytics'
# alias canatui='claudelytics analytics-tui'  # Analytics Studio (planned)

# Function to quickly check cost for a specific date
cdate() {
    if [ $# -eq 0 ]; then
        echo "Usage: cdate YYYYMMDD"
        return 1
    fi
    claudelytics cost --date "$1"
}

# Function to show a quick daily summary
cdaily() {
    echo "📊 Claude Usage Summary"
    claudelytics cost --today
    echo ""
    claudelytics --today -j | jq -r '.daily[0] | "Tokens: \(.totalTokens) | Input: \(.inputTokens) | Output: \(.outputTokens)"' 2>/dev/null || claudelytics --today
}

# Add to PATH if using local build
export PATH="$HOME/path/to/claudelytics/target/release:$PATH"
```

#### Fish (~/.config/fish/config.fish)

```fish
# Add claudelytics aliases
alias ctoday='claudelytics cost --today'
alias csum='claudelytics cost'
alias cwt='claudelytics --today'
alias cwatch='claudelytics watch'
alias cint='claudelytics interactive'
alias ctui='claudelytics tui'
alias cresume='claudelytics --resume'
alias canalytics='claudelytics analytics'
# alias canatui='claudelytics analytics-tui'  # Analytics Studio (planned)

# Function to quickly check cost for a specific date
function cdate
    if test (count $argv) -eq 0
        echo "Usage: cdate YYYYMMDD"
        return 1
    end
    claudelytics cost --date $argv[1]
end

# Function to show a quick daily summary
function cdaily
    echo "📊 Claude Usage Summary"
    claudelytics cost --today
    echo ""
    claudelytics --today
end

# Add to PATH if using local build
set -gx PATH $HOME/path/to/claudelytics/target/release $PATH
```

#### Zsh (~/.zshrc)

```zsh
# Add claudelytics aliases
alias ctoday='claudelytics cost --today'
alias csum='claudelytics cost'
alias cwt='claudelytics --today'
alias cwatch='claudelytics watch'
alias cint='claudelytics interactive'
alias ctui='claudelytics tui'
alias cresume='claudelytics --resume'
alias canalytics='claudelytics analytics'
# alias canatui='claudelytics analytics-tui'  # Analytics Studio (planned)

# Function to quickly check cost for a specific date
cdate() {
    if [[ $# -eq 0 ]]; then
        echo "Usage: cdate YYYYMMDD"
        return 1
    fi
    claudelytics cost --date "$1"
}

# Function to show a quick daily summary
cdaily() {
    echo "📊 Claude Usage Summary"
    claudelytics cost --today
    echo ""
    claudelytics --today -j | jq -r '.daily[0] | "Tokens: \(.totalTokens) | Input: \(.inputTokens) | Output: \(.outputTokens)"' 2>/dev/null || claudelytics --today
}

# Add to PATH if using local build
export PATH="$HOME/path/to/claudelytics/target/release:$PATH"
```

### Quick Commands After Setup

After adding to your shell config and reloading (`source ~/.bashrc`, `source ~/.config/fish/config.fish`, or `source ~/.zshrc`):

```bash
# Quick today's cost
ctoday

# Total cost summary
csum

# Check specific date
cdate 20241201

# Today's full report
cwt

# Interactive session browser
cint

# Enhanced TUI interface with advanced features
ctui

# Resume last TUI session  
cresume

# Real-time monitoring
cwatch

# Quick daily summary with tokens
cdaily
```

### Advanced Shell Integration

#### Prompt Integration (Show today's cost in prompt)

**Bash prompt with today's cost:**
```bash
# Add to ~/.bashrc
claude_cost_prompt() {
    local cost=$(claudelytics cost --today 2>/dev/null | grep "Cost:" | awk '{print $2}' | tr -d '$')
    if [[ -n "$cost" && "$cost" != "0.0000" ]]; then
        echo " 💰\$${cost}"
    fi
}

# Modify your PS1 to include Claude cost
export PS1='\u@\h:\w$(claude_cost_prompt)\$ '
```

**Fish prompt with today's cost:**
```fish
# Add to ~/.config/fish/functions/fish_prompt.fish
function claude_cost_prompt
    set cost (claudelytics cost --today 2>/dev/null | grep "Cost:" | awk '{print $2}' | tr -d '$')
    if test -n "$cost" -a "$cost" != "0.0000"
        echo " 💰\$$cost"
    end
end

function fish_prompt
    echo (pwd)(claude_cost_prompt)' $ '
end
```

#### Automatic Daily Summary

**Show summary when opening new terminal:**
```bash
# Add to ~/.bashrc (bash) or ~/.zshrc (zsh)
if command -v claudelytics >/dev/null 2>&1; then
    echo "📊 Today's Claude Usage:"
    claudelytics cost --today 2>/dev/null || echo "No usage data for today"
fi
```

```fish
# Add to ~/.config/fish/config.fish
if command -v claudelytics >/dev/null
    echo "📊 Today's Claude Usage:"
    claudelytics cost --today 2>/dev/null; or echo "No usage data for today"
end
```

#### Weekly Summary Cron Job

```bash
# Add to crontab (crontab -e)
# Send weekly summary every Monday at 9 AM
0 9 * * 1 /usr/local/bin/claudelytics cost | mail -s "Weekly Claude Usage Summary" your-email@example.com

# Or save to file
0 9 * * 1 /usr/local/bin/claudelytics cost >> ~/claude-weekly-summary.txt
```

## Code Architecture

### Module Structure
- **main.rs**: CLI entry point using clap for argument parsing
- **models.rs**: Core data structures for usage records, reports, token aggregation, and comprehensive analytics
- **models_registry.rs**: Model information registry for flexible model filtering and future model support
- **parser.rs**: JSONL file parsing with parallel processing using rayon and model filtering support
- **display.rs**: Output formatting (table and JSON) with colored terminal output
- **reports.rs**: Report generation logic for daily and session analytics
- **interactive.rs**: peco-style interactive session selector with fuzzy search
- **tui.rs**: Enhanced terminal user interface with advanced features, command palette, bookmarks, session comparison, and billing blocks view
- **watcher.rs**: Real-time file monitoring for live usage updates
- **export.rs**: CSV export functionality for daily, session, and summary reports
- **config.rs / config_v2.rs**: Configuration management with YAML-based settings
- **pricing.rs / pricing_strategies.rs**: Cost calculation and pricing models
- **pricing_cache.rs**: Offline pricing cache with 7-day validity
- **billing_blocks.rs**: 5-hour billing block tracking aligned with Claude's billing periods
- **mcp.rs**: MCP (Model Context Protocol) server integration
- **state.rs**: TUI session state management and persistence
- **error.rs**: Custom error types and error handling
- **domain.rs**: Core business domain logic
- **processing.rs**: Data processing utilities
- **performance.rs**: Performance monitoring and optimization

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
- Real-time file watching with debounced updates to minimize CPU usage
- Fuzzy search with efficient string matching for interactive mode

### New Features
- **Interactive Mode**: peco-style fuzzy searchable session selector
- **Watch Mode**: Real-time monitoring of usage data with automatic updates
- **Enhanced TUI**: Full-featured terminal interface with advanced analytics, command palette, bookmarks, and session comparison
- **Export Functions**: CSV export for daily, session, and summary reports
- **Configuration**: YAML-based config file support for persistent settings
- **Cost Display**: Quick cost summary for today, specific dates, or total usage
- **Today Filter**: `--today` flag to show only current day's usage
- **Enhanced Display**: Beautiful card-based layout with visual summaries (default)
- **Classic Format**: Traditional table format available with `--classic` flag
- **Enhanced UX**: Colored output, progress indicators, and better error messages
- **Model Filtering**: Filter usage by specific Claude models (opus, sonnet, haiku) with `--model-filter`
- **Model Registry**: Flexible model management system with aliases and future model support
- **List Models**: `--list-models` flag to display all registered models with their families and aliases
- **Model Breakdown Display**: `--by-model` flag with multiple format options (default, table, minimal, json)
- **5-Hour Billing Blocks**: Track usage in Claude's actual billing periods (00:00-05:00, 05:00-10:00, etc. UTC)
- **Offline Pricing Cache**: 7-day cache for pricing data to work without internet connection
- **TUI Billing Blocks View**: Dedicated tab in TUI showing billing block analysis with summary statistics

### Enhanced TUI Features (claudelytics tui)
- **Multi-tab Interface**: Overview, Daily, Sessions, Charts, Billing, Resume, and Help tabs
- **Command Palette**: Quick action access with Ctrl+P and fuzzy search
- **Bookmark System**: Save and organize important sessions with 'b' key
- **Session Comparison**: Mark sessions for comparison with 'x' key  
- **Efficiency Sorting**: Sort sessions by efficiency (tokens per dollar)
- **Resume Tab**: View recent sessions, bookmarks, and session history
  - **Interactive Message Input**: Press 'i' to enter input mode and send messages to sessions
  - **Input Buffer**: Full text editing with cursor movement (Left/Right/Home/End)
  - **Message Sending**: Type your message and press Enter to send (Esc to cancel)
- **Billing Tab**: 5-hour billing blocks with summary, peak usage, and pricing cache status
- **Keyboard Navigation**: vim-style (j/k) and arrow key navigation
- **Visual Elements**: Cost gauges, colored tables, ASCII charts, and formatted cards
- **Real-time Updates**: Live data display with scroll support
- **Interactive Tables**: Navigate through daily, session, and billing block data
- **Search & Filter**: Real-time search and filtering capabilities
- **Multiple Sort Options**: Sort by date, cost, tokens, efficiency, or project
- **Help System**: Built-in help with keyboard shortcuts


## Advanced Analytics Data Structures

The models.rs module now includes comprehensive data science-grade analytics structures supporting advanced pattern analysis, machine learning insights, and predictive analytics:

### Pattern Analysis & Data Mining
- **UsagePattern**: Detects and analyzes recurring usage patterns with frequency analysis, time preferences, and efficiency scoring
- **PatternAnalysis**: Comprehensive pattern detection with anomaly identification and predictability metrics
- **UsageAnomaly**: Advanced anomaly detection with severity assessment, impact analysis, and causal factor identification

### Productivity Analytics
- **DeepWorkSession**: Analyzes deep work patterns with focus quality metrics, interruption tracking, and flow state indicators
- **ContextSwitch**: Tracks project context switches with productivity loss estimation and recovery time analysis
- **FocusPeriod**: Measures focus intensity with consistency metrics and quality indicators
- **BreakPattern**: Analyzes break patterns and their impact on productivity with optimal timing recommendations
- **ProductivityTrend**: Daily, weekly, and seasonal productivity pattern analysis with peak performance identification

### Predictive Analytics & Forecasting
- **CostForecast**: Multi-timeframe cost prediction (week/month/quarter) with confidence intervals and seasonal adjustments
- **UsagePrediction**: Peak usage forecasting with saturation point analysis and growth rate tracking
- **TrendAnalysis**: Micro and macro trend identification with cyclical pattern detection and turning point analysis
- **BudgetTracker**: Advanced budget management with burn rate tracking, alert systems, and optimization suggestions
- **RiskAssessment**: Comprehensive risk analysis with probability assessment and mitigation strategies

### Machine Learning Insights
- **UsageClustering**: Advanced clustering analysis with silhouette scoring and stability metrics
- **PredictiveModel**: Model performance tracking with accuracy, precision, recall, and F1 scoring
- **AutomatedInsight**: AI-generated insights with evidence-based recommendations and impact assessments
- **ModelPerformance**: ML model drift detection and performance trend monitoring

### Interactive Analysis & Data Exploration
- **TimelineData**: Event-based timeline analysis with trend visualization and seasonal pattern detection
- **CorrelationMatrix**: Statistical correlation analysis with significance testing and practical interpretation
- **DrillDownPath**: Multi-dimensional data exploration with breadcrumb navigation and dynamic filtering
- **AdvancedFilter**: Sophisticated filtering system with regex support, smart suggestions, and saved filter management

### Workflow Integration
- **GitCorrelation**: Development workflow analysis with commit pattern correlation and code complexity metrics
- **ProjectMilestone**: Project milestone tracking with efficiency analysis and progress indicators
- **DevelopmentCycleAnalysis**: Development phase optimization with bottleneck identification and improvement suggestions

### Key Analytics Features
1. **Pattern Recognition**: Automatic detection of usage patterns, anomalies, and trends
2. **Predictive Modeling**: Cost forecasting, usage prediction, and risk assessment
3. **Efficiency Analysis**: Deep work identification, context switch optimization, and productivity enhancement
4. **Machine Learning**: Clustering analysis, automated insights, and model performance tracking
5. **Interactive Exploration**: Drill-down analysis, correlation discovery, and dynamic filtering
6. **Workflow Optimization**: Git integration, milestone tracking, and development cycle analysis

### Analytics Data Model Extensions
All analytics structures support:
- **Temporal Analysis**: Time-based pattern recognition and trend analysis
- **Statistical Significance**: Confidence intervals, p-values, and correlation strength metrics
- **Performance Metrics**: Efficiency scoring, optimization opportunities, and improvement tracking
- **Automated Reasoning**: AI-generated insights with evidence-based recommendations
- **Risk Management**: Probability assessment, impact analysis, and mitigation strategies

## Testing

The project has comprehensive test coverage across all major modules:
- **Unit Tests**: Core functionality testing in models, parser, display, and report modules
- **Integration Tests**: End-to-end testing of CLI commands and data processing
- **Error Handling Tests**: Comprehensive error scenario coverage
- **Performance Tests**: Benchmarks for critical paths using criterion

Run tests with:
```bash
cargo test                  # Run all tests
cargo test -- --nocapture  # Show test output
cargo test <module_name>   # Run tests for specific module
cargo test --release       # Run tests in release mode
```

## Release Process

### Creating a New Release

1. **Update version in Cargo.toml**
   ```bash
   # Edit Cargo.toml and update version field
   cargo build  # Verify it builds
   ```

2. **Run pre-release checks**
   ```bash
   cargo test
   cargo fmt
   cargo clippy -- -D warnings
   cargo build --release
   ```

3. **Create and push tag**
   ```bash
   git tag v0.3.3
   git push origin v0.3.3
   ```

4. **GitHub Actions will automatically**:
   - Build binaries for all platforms (Linux, Windows, macOS)
   - Create GitHub release with artifacts
   - Publish to crates.io
   - Generate SHA256 checksums

5. **Update Homebrew formula** (Formula/claudelytics.rb):
   ```bash
   # Get SHA256 for each platform
   curl -fsSL https://github.com/nwiizo/claudelytics/releases/download/v0.3.3/claudelytics-x86_64-apple-darwin.tar.gz | shasum -a 256
   curl -fsSL https://github.com/nwiizo/claudelytics/releases/download/v0.3.3/claudelytics-aarch64-apple-darwin.tar.gz | shasum -a 256
   # Update Formula/claudelytics.rb with new SHAs
   ```

### Cross-Platform Building

```bash
# Add rust targets if needed
rustup target add x86_64-unknown-linux-gnu
rustup target add x86_64-unknown-linux-musl
rustup target add x86_64-pc-windows-msvc
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# Build for specific targets
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-musl
cargo build --release --target x86_64-pc-windows-msvc
cargo build --release --target x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
```

## CI/CD Pipeline

### GitHub Actions Workflows

1. **CI Pipeline** (.github/workflows/ci.yml):
   - Triggers on: push to main, pull requests
   - Runs on: Ubuntu, Windows, macOS
   - Checks:
     ```bash
     cargo fmt --all -- --check
     cargo clippy -- -D warnings
     cargo test --verbose
     cargo build --verbose
     ```

2. **Release Pipeline** (.github/workflows/release.yml):
   - Triggers on: version tags (v*)
   - Builds release binaries for all platforms
   - Creates GitHub release with artifacts
   - Publishes to crates.io automatically

### Local CI Checks (run before pushing)

```bash
# Run the same checks as CI
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo test --verbose
cargo build --verbose --release
```

## Installation Methods

### Quick Install Script
```bash
curl -fsSL https://raw.githubusercontent.com/nwiizo/claudelytics/main/install.sh | bash
```

### Manual Binary Installation
```bash
# Download latest release
curl -LO https://github.com/nwiizo/claudelytics/releases/latest/download/claudelytics-$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]').tar.gz
tar -xzf claudelytics-*.tar.gz
sudo mv claudelytics /usr/local/bin/
```

### Package Managers
```bash
# Cargo (Rust developers)
cargo install claudelytics

# Homebrew (macOS/Linux)
brew tap nwiizo/claudelytics
brew install claudelytics
```