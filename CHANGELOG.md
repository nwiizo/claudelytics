# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[0.3.0]: https://github.com/nwiizo/claudelytics/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/nwiizo/claudelytics/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/nwiizo/claudelytics/releases/tag/v0.1.0