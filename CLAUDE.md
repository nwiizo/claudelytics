# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claudelytics is a Rust CLI tool for analyzing Claude Code usage patterns and costs. It parses JSONL files from the `~/.claude/projects/` directory structure and generates comprehensive reports on token usage, costs, and session analytics.

**Current Version**: 0.2.0 - Major feature release with comprehensive analytics data structures

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

# Run with release optimizations
cargo run --release -- daily --since 20240101

# Interactive mode (peco-style session selector)
cargo run -- interactive

# Watch mode for real-time monitoring
cargo run -- watch

# Terminal User Interface (TUI) mode
cargo run -- tui
cargo run -- --tui                  # Alternative flag format

# Advanced Terminal User Interface (Advanced TUI) mode
cargo run -- advanced-tui
cargo run -- --advanced-tui         # Alternative flag format

# Analytics Studio TUI mode (data science features) - PLANNED
# cargo run -- analytics-tui         # Comprehensive analytics studio
# cargo run -- --analytics-tui       # Alternative flag format

# Export data to CSV
cargo run -- export --daily --sessions --summary -o report

# Configuration management
cargo run -- config --show
cargo run -- config --set-path /path/to/claude

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
```

## Pre-commit Requirements

**IMPORTANT**: Before committing any changes, you MUST run:

1. `cargo test` - Ensure all tests pass
2. `cargo fmt` - Format the code according to Rust standards

These checks are enforced in CI/CD and will cause the build to fail if not followed.

## Installation and Shell Integration

### Building and Installing

```bash
# Build release version
cargo build --release

# Install to system (requires ~/.cargo/bin in PATH)
cargo install --path .

# Or copy binary to a directory in your PATH
sudo cp target/release/claudelytics /usr/local/bin/
```

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
alias catui='claudelytics advanced-tui'
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
alias catui='claudelytics advanced-tui'
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
alias catui='claudelytics advanced-tui'
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

# Enhanced TUI interface
ctui

# Advanced TUI with professional features
catui

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
- **parser.rs**: JSONL file parsing with parallel processing using rayon
- **display.rs**: Output formatting (table and JSON) with colored terminal output
- **reports.rs**: Report generation logic for daily and session analytics
- **interactive.rs**: peco-style interactive session selector with fuzzy search
- **tui.rs**: Enhanced terminal user interface with tabs, search, and visual elements
- **advanced_tui.rs**: Professional-grade advanced TUI with drill-down, comparison, and live monitoring
- **watcher.rs**: Real-time file monitoring for live usage updates
- **export.rs**: CSV export functionality for daily, session, and summary reports
- **config.rs**: Configuration management with YAML-based settings

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
- **Enhanced TUI**: Full-featured terminal interface with tabs, navigation, and visual charts
- **Advanced TUI**: Professional-grade analytics with drill-down, comparison, and live monitoring
- **Export Functions**: CSV export for daily, session, and summary reports
- **Configuration**: YAML-based config file support for persistent settings
- **Cost Display**: Quick cost summary for today, specific dates, or total usage
- **Today Filter**: `--today` flag to show only current day's usage
- **Enhanced Display**: Beautiful card-based layout with visual summaries (default)
- **Classic Format**: Traditional table format available with `--classic` flag
- **Enhanced UX**: Colored output, progress indicators, and better error messages

### Enhanced TUI Features (claudelytics tui)
- **Multi-tab Interface**: Overview, Daily, Sessions, Charts, and Help tabs
- **Keyboard Navigation**: vim-style (j/k) and arrow key navigation
- **Visual Elements**: Cost gauges, colored tables, ASCII charts, and formatted cards
- **Real-time Updates**: Live data display with scroll support
- **Interactive Tables**: Navigate through daily and session data
- **Search & Filter**: Real-time search and filtering capabilities
- **Multiple Sort Options**: Sort by date, cost, tokens, or project
- **Help System**: Built-in help with keyboard shortcuts

### Advanced TUI Features (claudelytics advanced-tui)
- **9 Comprehensive Tabs**: Overview, Daily, Sessions, Drill-Down, Compare, Benchmark, Live, Charts, Help
- **Command Palette**: Quick action access with Ctrl+P and fuzzy search
- **Session Drill-Down**: Message-level analysis with efficiency metrics and timeline
- **Session Comparison**: Side-by-side comparison of multiple sessions
- **Benchmarking**: Performance rankings and optimization recommendations
- **Live Monitoring**: Real-time metrics with sparklines and activity tracking
- **Advanced Visualizations**: Heatmaps, trend analysis, and usage patterns
- **Bookmark System**: Save and organize important sessions
- **Professional Analytics**: Efficiency scoring, cost optimization tips, and predictions
- **Enhanced Navigation**: Mouse support, multiple input modes, and sophisticated UI state management

## Advanced Analytics Data Structures (v0.2.0+)

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