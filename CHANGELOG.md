# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.3] - 2025-06-26

### 🎉 Major TUI Enhancements and Critical Bug Fixes

### ✨ Added
- **Enhanced Vim Navigation**: Complete vim-style keyboard support
  - `gg`/`G` for jump to top/bottom
  - `Ctrl+d`/`Ctrl+u` for half-page scrolling
  - `v` for visual mode with multi-select
  - `w`/`b` for word navigation in search
  - `0`/`$` for line start/end navigation
- **Resume Tab Completion**: Fully functional message sending to sessions
  - Interactive input mode with character counter
  - Real-time cursor visualization
  - Session message history updates
- **Export Functionality**: Export data from any tab
  - `e`/`Ctrl+E` to open export dialog
  - CSV and JSON format support
  - Direct clipboard integration
- **Visual Enhancements**: Professional UI improvements
  - Loading animations with Unicode braille patterns
  - Toast notifications for user feedback
  - Smooth progress bars for cost/token visualization
  - Enhanced status bar with real-time information

### 🐛 Fixed
- **Critical Pricing Bug**: Fixed calculation error that made costs 1000x lower than actual
  - Corrected pricing for all Claude models (Opus 4, Sonnet 4, Haiku 3.5, etc.)
  - Updated from incorrect division by 1000 to correct division by 1,000,000
- **Cache Efficiency Formula**: Standardized calculation across codebase
  - Now correctly shows percentage of input from cache
- **Monthly Token Projection**: Fixed hardcoded calculation
  - Now uses actual burn rate data instead of fixed pricing assumption
- **Burn Rate Calculation**: Improved accuracy
  - Changed from 24-hour uniform distribution to 9 active hours

### 🔧 Improved
- **TUI Architecture**: Added new visual effects module (`tui_visuals.rs`)
- **Code Quality**: Fixed all clippy warnings and formatting issues
- **User Experience**: Comprehensive help system and visual feedback

### 📊 Impact
- Cost calculations now show accurate values (e.g., $70 instead of $70,000 for daily usage)
- All pricing matches official Anthropic API rates
- More accurate projections and analytics

## [0.4.2] - 2025-06-25

### ✨ Added
- **Complete Daily Breakdown**: Now displays all columns (Date, Cost, Tokens, Input, Output, O/I Ratio, Efficiency, Cache Hit) in the daily breakdown when there are more than 3 days of data

### 🔧 Improved
- **Code Quality**: Fixed all clippy warnings and errors for cleaner codebase
- **Performance**: Refactored code to use more efficient patterns (e.g., `.values()` iterator, `or_default()`)
- **Code Organization**: Restructured handle_blocks_command to use options struct for better maintainability

### 🐛 Fixed
- Removed redundant imports and unused code
- Fixed iterator patterns to be more idiomatic
- Fixed unnecessary cloning and borrowing operations
- Fixed manual range contains to use built-in methods

## [0.4.1] - 2025-06-25

### ✨ Added

#### Advanced Session Analytics
- **New `analytics` Command**: Comprehensive session analysis inspired by ccusage
  - Time of day usage patterns with peak/off-peak hours
  - Day of week analysis with weekend vs weekday ratios
  - Session duration tracking and distribution
  - Usage frequency and streak analysis
  - Cost efficiency metrics with threshold alerts
- **Flexible Analysis Options**: Use flags to show specific analyses
  - `--time-of-day`: Hourly usage patterns
  - `--day-of-week`: Weekly patterns
  - `--duration`: Session length analysis
  - `--frequency`: Usage streaks and frequency
  - `--efficiency`: Cost per token analysis

#### Burn Rate Calculation
- **Real-time Burn Rate**: Track token consumption rates
  - 24-hour and 7-day burn rate calculations
  - Hourly, daily, and monthly projections
  - Trend analysis with percentage changes
- **Integrated Display**: Burn rate section in daily reports
  - Visual trend indicators (up/down arrows)
  - Color-coded projections based on trends

#### Terminal-Aware Display
- **Responsive Output**: Automatic terminal width detection
  - Smart column hiding for narrow terminals
  - Compact mode with `--compact` flag
  - Display modes: Compact/Normal/Wide
- **Better Formatting**: Improved table layouts and truncation

### 🔧 Improved

- **Daily Report**: Limited to last 30 days for cleaner output
- **Date Ordering**: Newest dates now appear at bottom of tables
- **Performance**: Optimized analytics calculations for large datasets

### 🐛 Fixed

- **Test Stability**: Fixed projection test that was failing due to date ordering
- **Import Organization**: Cleaned up unused imports across modules

## [0.4.0] - 2025-06-19

### 🚀 Major Feature Release: Billing Blocks & Offline Support

This release introduces Claude's 5-hour billing block tracking and offline pricing cache, bringing key features from ccusage to claudelytics with enhanced visualization in the TUI.

### ✨ Added

#### 5-Hour Billing Blocks
- **Accurate Billing Alignment**: Track usage in Claude's actual billing periods (UTC):
  - 00:00-05:00, 05:00-10:00, 10:00-15:00, 15:00-20:00, 20:00-00:00
- **New `billing-blocks` Command**: Comprehensive billing block analysis
  - Peak usage identification and time pattern analysis
  - Average cost per block and session count tracking
  - JSON output support for scripting and automation
- **Thread-Safe Implementation**: Parallel processing with `Arc<Mutex<>>` for accurate aggregation
- **Smart Aggregation**: Automatic usage grouping by billing periods across multiple days

#### Offline Pricing Cache
- **7-Day Cache**: Store pricing data locally for offline operation
- **New `pricing-cache` Command**: Manage pricing cache
  - `--show`: Display cache status and validity
  - `--clear`: Remove cached pricing data
  - `--update`: Placeholder for future API integration
- **Smart Fallback**: Automatic fallback to built-in pricing when cache is unavailable
- **Version Awareness**: Cache invalidates on app version change for compatibility

#### Enhanced TUI
- **New Billing Tab**: Dedicated visualization for billing blocks (6th tab)
  - Real-time current block cost tracking
  - Peak block identification with timestamps
  - Average cost per block display
  - Percentage breakdown by billing block
  - Pricing cache status indicator
- **Interactive Features**: 
  - Press 's' in Billing tab to toggle summary view
  - Color coding based on cost thresholds (green < $2.5, yellow < $5, red > $5)
  - Scrollbar support for long billing block lists
- **Navigation Update**: Tab count increased from 5 to 6 tabs

### 🔧 Improved

#### Performance
- **Parallel Parsing**: Enhanced thread-safe data collection during file parsing
- **Memory Efficiency**: Optimized billing block storage with date-based HashMap
- **Fast Lookups**: O(1) current block retrieval with efficient data structures

#### Code Quality
- **Module Organization**: Clean separation of billing and pricing concerns
- **Type Safety**: Comprehensive use of Rust's type system for reliability
- **Error Handling**: Robust error handling with context-aware messages
- **Documentation**: Extensive inline documentation for new modules

### 🔄 Changed

#### Parser Integration
- **Dual Collection**: Parser now collects both daily/session data and billing blocks
- **Backward Compatible**: Existing functionality unchanged, billing blocks are additive
- **API Updates**: `parse_all()` now returns `(DailyUsageMap, SessionUsageMap, BillingBlockManager)`

#### Display Enhancements
- **Billing Block Format**: New display format for billing block reports
- **Time Labels**: Human-readable time ranges (e.g., "00:00-05:00 UTC")
- **Cost Formatting**: Consistent 4-decimal precision across all displays

### 🐛 Fixed

- **Clippy Warnings**: Resolved all clippy lints and warnings
- **Dead Code**: Properly attributed unused code for future API expansion
- **Import Organization**: Cleaned up and optimized module imports
- **Type Mismatches**: Fixed all type compatibility issues in new modules

### 📊 Technical Details

#### Data Structures
```rust
pub struct BillingBlock {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub usage: TokenUsage,
    pub session_count: usize,
}

pub struct PricingCache {
    pub pricing_data: HashMap<String, ModelPricing>,
    pub last_updated: DateTime<Utc>,
    pub version: String,
}
```

#### Key Algorithms
- **Block Normalization**: Timestamps normalized to 5-hour boundaries
- **Usage Aggregation**: Thread-safe accumulation across parallel processing
- **Cache Location**: Platform-appropriate using `dirs` crate (~/.cache/claudelytics/)

### 💡 Usage Examples

```bash
# View billing blocks with summary
claudelytics billing-blocks

# Get billing data as JSON
claudelytics billing-blocks --json

# Filter billing blocks by date
claudelytics billing-blocks --since 20240601

# Check pricing cache status
claudelytics pricing-cache

# Clear pricing cache
claudelytics pricing-cache --clear

# Navigate to Billing tab in TUI
claudelytics tui  # Then press '5' or Tab to reach Billing tab
```

### 🚀 Migration Notes

This release is fully backward compatible. No changes required for existing workflows:
- All existing commands work unchanged
- New features are additive and optional
- Configuration files remain compatible

---

## [0.3.1] - 2025-06-12

### 🔧 Cost Calculation Alignment with ccusage

This patch release aligns claudelytics' cost calculation methodology with ccusage, ensuring consistent and accurate cost reporting across different Claude usage analysis tools.

### ✨ Added

#### Cost Calculation Improvements
- **ccusage Compatibility**: Aligned cost calculation to match ccusage's methodology
- **Enhanced Calculation Mode**: Always recalculate costs for better accuracy
- **Fallback Logic**: Smart fallback to costUSD when calculation fails

### 🔧 Improved

#### Display Enhancements
- **Professional Headers**: Beautiful section headers with emoji indicators
- **Enhanced Summary Card**: Comprehensive cost and usage metrics at a glance
- **Efficiency Metrics**: Added tokens per dollar, O/I ratio, and cache hit rate
- **Visual Improvements**: Better spacing, color usage, and separator lines
- **3-Digit Cost Support**: Fixed display formatting for costs over $100

#### Code Quality
- **Dead Code Removal**: Cleaned up unused functions and added proper attributes
- **Build Warnings**: Resolved all compilation warnings
- **Code Organization**: Better separation of display logic

### 🔄 Changed

#### Pricing Updates
- **Model Pricing**: Updated to latest Claude 4 pricing structure
- **Official Rates**: Claude Sonnet 4 ($3/$15) and Claude Opus 4 ($15/$75) per million tokens
- **Cache Pricing**: Accurate cache creation (25% markup) and cache read (90% discount) rates

### 🐛 Fixed

- **Cost Display**: Fixed formatting issues for 3-digit costs
- **Build Errors**: Resolved all cargo build warnings and errors
- **Model Mapping**: Improved model name detection and pricing lookup

### 📊 Technical Details

- **Calculation Method**: Switched from "auto" mode to always recalculate for consistency
- **Price Precision**: Using precise per-token rates (e.g., $3.0/1,000,000 tokens)
- **Efficiency Calculation**: Added comprehensive efficiency metrics to TokenUsage struct

---

## [0.3.0] - 2025-06-11

### 🚀 Major Feature Release

This release introduces powerful new analytics capabilities and advanced data integration features, significantly expanding claudelytics' analytical capabilities.

### ✨ Added

#### Monthly Analytics
- **Monthly Reports**: New `claudelytics monthly` command for long-term usage analysis
- **Calendar Aggregation**: Data grouped by calendar months with activity day counts
- **Advanced Metrics**: Average daily cost calculation and monthly summaries
- **Multi-Format Support**: Monthly reports available in enhanced, classic table, and JSON formats

#### Advanced Sorting System
- **5 Sort Fields**: Date, cost, tokens, efficiency (tokens per dollar), and project name
- **Flexible Ordering**: Ascending and descending sort options for all report types
- **Smart Defaults**: Intelligent default sorting (date desc for daily/monthly, cost desc for sessions)
- **Universal Application**: Sorting available across daily, session, and monthly reports

#### Model Context Protocol (MCP) Integration
- **Standardized Integration**: Full MCP v1.0.0 protocol support for seamless tool integration
- **4 Data Resources**: 
  - `claudelytics://daily-usage` - Daily aggregated usage data
  - `claudelytics://session-usage` - Session-based analytics
  - `claudelytics://monthly-usage` - Monthly summaries
  - `claudelytics://cost-summary` - Cost analysis and statistics
- **3 Powerful Tools**:
  - `get_usage_data` - Flexible data retrieval with filtering and sorting
  - `get_cost_summary` - Cost analysis for specific dates or totals
  - `find_sessions` - Advanced session filtering with regex and thresholds
- **JSON Schema Validation**: Comprehensive input validation for all MCP tools
- **Server Infrastructure**: Ready for stdio and HTTP transport methods

#### Enhanced CLI Experience
- **Granular Filtering**: More precise data filtering options across all commands
- **Command Discovery**: `--list-tools` and `--list-resources` for MCP exploration
- **Improved Help**: Detailed help text with examples for all new features

### 🔧 Improved

#### Data Analysis
- **Efficiency Metrics**: New efficiency calculation (tokens per dollar) for cost optimization insights
- **Temporal Analysis**: Better month-over-month comparison with activity day normalization
- **Report Consistency**: Unified sorting and filtering across all report types
- **Data Presentation**: Enhanced table formatting with improved readability

#### User Experience
- **Command Discoverability**: Better help text and examples for all commands
- **Flexible Output**: Users can choose between enhanced, classic, and JSON formats
- **Smart Defaults**: Intelligent default sorting based on report type and common use cases
- **Error Handling**: Improved error messages for MCP operations

#### Integration Capabilities
- **Protocol Compliance**: Full adherence to MCP v1.0.0 specification
- **Schema Validation**: Robust input validation with detailed JSON schemas
- **Future-Ready**: Architecture prepared for HTTP transport and advanced MCP features

### 🔄 Changed

#### CLI Interface
- **New Commands**: Added `monthly` subcommand for calendar-based analysis
- **Enhanced Flags**: New `--sort-by` and `--sort-order` options for all report commands
- **MCP Commands**: Added `mcp-server` command with tool and resource discovery options

#### Data Processing
- **Monthly Aggregation**: New aggregation logic for calendar month grouping
- **Sorting Engine**: Comprehensive sorting system supporting multiple criteria
- **Report Generation**: Refactored report generation for better modularity and performance

### 📊 Feature Highlights

#### Command Examples
```bash
# New monthly analytics
claudelytics monthly --sort-by cost --sort-order asc

# Advanced session sorting by efficiency
claudelytics session --sort-by efficiency --sort-order desc

# MCP integration discovery
claudelytics mcp-server --list-tools
claudelytics mcp-server --list-resources
```

#### Data Insights
- **Monthly Trends**: Identify usage patterns across calendar months
- **Efficiency Analysis**: Find most cost-effective sessions and projects
- **Long-term Planning**: Monthly aggregation enables better budget forecasting

### 🧪 Quality Assurance

- **Test Coverage**: 28 comprehensive tests including new functionality
- **Real Data Validation**: Tested with production data (900M+ tokens, $400+ costs)
- **Performance**: Efficient processing of large datasets with parallel execution
- **Compatibility**: Maintains backward compatibility with existing workflows

### 📦 Dependencies

- **Added**: `regex` v1.10 for advanced session filtering capabilities
- **Dev Dependencies**: `tempfile` v3.8 for enhanced testing infrastructure
- **Updated**: All existing dependencies to latest stable versions

### 🚀 Migration Guide

This release maintains full backward compatibility. All existing commands work unchanged:

1. **Existing Workflows**: No changes required for current claudelytics usage
2. **New Features**: Optional enhancements available immediately
3. **Configuration**: All existing configurations remain valid

### 💡 Usage Examples

#### Monthly Analysis
```bash
# Enhanced monthly report with visual formatting
claudelytics monthly

# Classic table format for scripting
claudelytics monthly --classic

# JSON output for integrations
claudelytics --json monthly
```

#### Advanced Sorting
```bash
# Find most cost-effective sessions
claudelytics session --sort-by efficiency --sort-order desc

# Analyze daily costs from lowest to highest
claudelytics daily --sort-by cost --sort-order asc

# Monthly data by token volume
claudelytics monthly --sort-by tokens --sort-order desc
```

#### MCP Integration
```bash
# Discover available data resources
claudelytics mcp-server --list-resources

# Explore analytical tools
claudelytics mcp-server --list-tools

# Start MCP server for tool integration
claudelytics mcp-server
```

### 🔮 Future Enhancements

This release establishes the foundation for:
- **MCP HTTP Server**: Full bidirectional MCP communication
- **Advanced Analytics**: Machine learning insights and predictions
- **Custom Dashboards**: Configurable monitoring interfaces
- **Real-time Integration**: Live data streaming capabilities

---

## [0.2.0] - 2025-06-02

### Added
- Enhanced TUI with professional features
- Advanced analytics data structures
- Comprehensive reporting capabilities
- Configuration management system

### Changed
- Improved display formatting
- Enhanced error handling
- Better performance optimization

### Fixed
- Various UI/UX improvements
- Data processing optimizations

---

## [0.1.0] - 2025-05-29

### Added
- Initial release of claudelytics
- Basic Claude Code usage analysis
- Daily and session reporting
- Terminal user interface
- Cost calculation
- Export functionality

### Features
- Parse Claude Code JSONL files
- Generate daily usage reports
- Session-based analytics
- Interactive terminal interface
- CSV export capabilities
- Real-time monitoring with watch mode

[0.4.0]: https://github.com/nwiizo/claudelytics/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/nwiizo/claudelytics/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/nwiizo/claudelytics/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/nwiizo/claudelytics/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/nwiizo/claudelytics/releases/tag/v0.1.0