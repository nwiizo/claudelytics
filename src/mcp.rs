//! Model Context Protocol (MCP) server implementation for claudelytics
//!
//! Provides MCP resources and tools for accessing Claude usage analytics data
//! through a standardized protocol that other applications can consume.

use crate::parser::UsageParser;
use crate::reports::{
    generate_daily_report_sorted, generate_monthly_report_sorted, generate_session_report_sorted,
};
use anyhow::Result;
use serde_json::{Value, json};
use std::path::PathBuf;

/// MCP server for claudelytics data access
pub struct McpServer {
    claude_path: PathBuf,
}

/// MCP Resource definition
#[derive(Debug, Clone)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

/// MCP Tool definition
#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

impl McpServer {
    /// Create a new MCP server instance
    pub fn new(claude_path: PathBuf) -> Self {
        Self { claude_path }
    }

    /// Get list of available MCP resources
    pub fn list_resources(&self) -> Vec<McpResource> {
        vec![
            McpResource {
                uri: "claudelytics://daily-usage".to_string(),
                name: "daily-usage".to_string(),
                description: "Daily Claude usage aggregated by date".to_string(),
                mime_type: "application/json".to_string(),
            },
            McpResource {
                uri: "claudelytics://session-usage".to_string(),
                name: "session-usage".to_string(),
                description: "Claude usage aggregated by sessions".to_string(),
                mime_type: "application/json".to_string(),
            },
            McpResource {
                uri: "claudelytics://monthly-usage".to_string(),
                name: "monthly-usage".to_string(),
                description: "Claude usage aggregated by month".to_string(),
                mime_type: "application/json".to_string(),
            },
            McpResource {
                uri: "claudelytics://cost-summary".to_string(),
                name: "cost-summary".to_string(),
                description: "Total cost summary and statistics".to_string(),
                mime_type: "application/json".to_string(),
            },
        ]
    }

    /// Get list of available MCP tools
    pub fn list_tools(&self) -> Vec<McpTool> {
        vec![
            McpTool {
                name: "get_usage_data".to_string(),
                description: "Get Claude usage data with optional filtering and sorting"
                    .to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "report_type": {
                            "type": "string",
                            "enum": ["daily", "session", "monthly"],
                            "description": "Type of report to generate"
                        },
                        "since": {
                            "type": "string",
                            "description": "Start date in YYYYMMDD format",
                            "pattern": "^\\d{8}$"
                        },
                        "until": {
                            "type": "string",
                            "description": "End date in YYYYMMDD format",
                            "pattern": "^\\d{8}$"
                        },
                        "sort_field": {
                            "type": "string",
                            "enum": ["date", "cost", "tokens", "efficiency", "project"],
                            "description": "Field to sort by"
                        },
                        "sort_order": {
                            "type": "string",
                            "enum": ["asc", "desc"],
                            "description": "Sort order"
                        }
                    },
                    "required": ["report_type"]
                }),
            },
            McpTool {
                name: "get_cost_summary".to_string(),
                description: "Get cost summary for a specific date or total".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "date": {
                            "type": "string",
                            "description": "Specific date in YYYYMMDD format, or 'today' for today",
                            "pattern": "^(\\d{8}|today)$"
                        }
                    }
                }),
            },
            McpTool {
                name: "find_sessions".to_string(),
                description: "Find sessions matching specific criteria".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "project_filter": {
                            "type": "string",
                            "description": "Filter sessions by project path (supports regex)"
                        },
                        "min_cost": {
                            "type": "number",
                            "description": "Minimum cost threshold"
                        },
                        "max_cost": {
                            "type": "number",
                            "description": "Maximum cost threshold"
                        },
                        "min_tokens": {
                            "type": "integer",
                            "description": "Minimum token count"
                        },
                        "date_range": {
                            "type": "object",
                            "properties": {
                                "start": {"type": "string", "pattern": "^\\d{8}$"},
                                "end": {"type": "string", "pattern": "^\\d{8}$"}
                            }
                        }
                    }
                }),
            },
        ]
    }

    /// Read a specific MCP resource
    pub async fn read_resource(&self, uri: &str) -> Result<Value> {
        let parser = UsageParser::new(self.claude_path.clone(), None, None)?;
        let (daily_map, session_map) = parser.parse_all()?;

        match uri {
            "claudelytics://daily-usage" => {
                let report = generate_daily_report_sorted(daily_map, None, None);
                Ok(serde_json::to_value(report)?)
            }
            "claudelytics://session-usage" => {
                let report = generate_session_report_sorted(session_map, None, None);
                Ok(serde_json::to_value(report)?)
            }
            "claudelytics://monthly-usage" => {
                let report = generate_monthly_report_sorted(daily_map, None, None);
                Ok(serde_json::to_value(report)?)
            }
            "claudelytics://cost-summary" => {
                let daily_report = generate_daily_report_sorted(daily_map, None, None);
                Ok(json!({
                    "total_cost": daily_report.totals.total_cost,
                    "total_tokens": daily_report.totals.total_tokens,
                    "input_tokens": daily_report.totals.input_tokens,
                    "output_tokens": daily_report.totals.output_tokens,
                    "cache_creation_tokens": daily_report.totals.cache_creation_tokens,
                    "cache_read_tokens": daily_report.totals.cache_read_tokens,
                    "total_days": daily_report.daily.len(),
                    "avg_daily_cost": if !daily_report.daily.is_empty() {
                        daily_report.totals.total_cost / daily_report.daily.len() as f64
                    } else {
                        0.0
                    }
                }))
            }
            _ => anyhow::bail!("Unknown resource URI: {}", uri),
        }
    }

    /// Call an MCP tool
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        match name {
            "get_usage_data" => self.handle_get_usage_data(arguments).await,
            "get_cost_summary" => self.handle_get_cost_summary(arguments).await,
            "find_sessions" => self.handle_find_sessions(arguments).await,
            _ => anyhow::bail!("Unknown tool: {}", name),
        }
    }

    async fn handle_get_usage_data(&self, args: Value) -> Result<Value> {
        let report_type = args["report_type"].as_str().unwrap_or("daily");
        let since = args["since"].as_str();
        let until = args["until"].as_str();

        // Create parser with date filters if provided
        let parser = if since.is_some() || until.is_some() {
            UsageParser::new(
                self.claude_path.clone(),
                since.map(String::from),
                until.map(String::from),
            )?
        } else {
            UsageParser::new(self.claude_path.clone(), None, None)?
        };

        let (daily_map, session_map) = parser.parse_all()?;

        let sort_field = self.parse_sort_field(args["sort_field"].as_str());
        let sort_order = self.parse_sort_order(args["sort_order"].as_str());

        match report_type {
            "daily" => {
                let report = generate_daily_report_sorted(daily_map, sort_field, sort_order);
                Ok(serde_json::to_value(report)?)
            }
            "session" => {
                let report = generate_session_report_sorted(session_map, sort_field, sort_order);
                Ok(serde_json::to_value(report)?)
            }
            "monthly" => {
                let report = generate_monthly_report_sorted(daily_map, sort_field, sort_order);
                Ok(serde_json::to_value(report)?)
            }
            _ => anyhow::bail!("Invalid report type: {}", report_type),
        }
    }

    async fn handle_get_cost_summary(&self, args: Value) -> Result<Value> {
        let date_filter = args["date"].as_str();

        let parser = if let Some(date) = date_filter {
            if date == "today" {
                let today = chrono::Local::now().format("%Y%m%d").to_string();
                UsageParser::new(self.claude_path.clone(), Some(today.clone()), Some(today))?
            } else {
                UsageParser::new(
                    self.claude_path.clone(),
                    Some(date.to_string()),
                    Some(date.to_string()),
                )?
            }
        } else {
            UsageParser::new(self.claude_path.clone(), None, None)?
        };

        let (daily_map, _) = parser.parse_all()?;
        let report = generate_daily_report_sorted(daily_map, None, None);

        Ok(json!({
            "total_cost": report.totals.total_cost,
            "total_tokens": report.totals.total_tokens,
            "days_count": report.daily.len(),
            "date_range": {
                "start": report.daily.last().map(|d| &d.date),
                "end": report.daily.first().map(|d| &d.date)
            }
        }))
    }

    async fn handle_find_sessions(&self, args: Value) -> Result<Value> {
        let parser = UsageParser::new(self.claude_path.clone(), None, None)?;
        let (_, session_map) = parser.parse_all()?;
        let mut report = generate_session_report_sorted(session_map, None, None);

        // Apply filters
        if let Some(project_filter) = args["project_filter"].as_str() {
            let regex = regex::Regex::new(project_filter)
                .map_err(|e| anyhow::anyhow!("Invalid regex: {}", e))?;
            report.sessions.retain(|s| regex.is_match(&s.project_path));
        }

        if let Some(min_cost) = args["min_cost"].as_f64() {
            report.sessions.retain(|s| s.total_cost >= min_cost);
        }

        if let Some(max_cost) = args["max_cost"].as_f64() {
            report.sessions.retain(|s| s.total_cost <= max_cost);
        }

        if let Some(min_tokens) = args["min_tokens"].as_u64() {
            report.sessions.retain(|s| s.total_tokens >= min_tokens);
        }

        Ok(serde_json::to_value(report)?)
    }

    fn parse_sort_field(&self, field: Option<&str>) -> Option<crate::reports::SortField> {
        use crate::reports::SortField;
        match field {
            Some("date") => Some(SortField::Date),
            Some("cost") => Some(SortField::Cost),
            Some("tokens") => Some(SortField::Tokens),
            Some("efficiency") => Some(SortField::Efficiency),
            Some("project") => Some(SortField::Project),
            _ => None,
        }
    }

    fn parse_sort_order(&self, order: Option<&str>) -> Option<crate::reports::SortOrder> {
        use crate::reports::SortOrder;
        match order {
            Some("asc") => Some(SortOrder::Asc),
            Some("desc") => Some(SortOrder::Desc),
            _ => None,
        }
    }

    /// Start MCP server in stdio mode
    pub async fn run_stdio(&self) -> Result<()> {
        println!(
            "{{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"params\":{{\"protocolVersion\":\"1.0.0\",\"capabilities\":{{\"tools\":{{}},\"resources\":{{}}}}}}}}"
        );

        // Simple stdio server loop
        // In a real implementation, this would use a proper MCP library
        loop {
            // Read from stdin and respond with MCP protocol messages
            // This is a simplified implementation for demonstration
            break;
        }

        Ok(())
    }

    /// Start MCP server in HTTP mode
    pub async fn run_http(&self, port: u16) -> Result<()> {
        println!("Starting MCP server on HTTP port {}", port);
        // HTTP server implementation would go here
        // Using Server-Sent Events for bidirectional communication
        Ok(())
    }
}

/// MCP server capability advertisement
pub fn get_server_info() -> Value {
    json!({
        "name": "claudelytics",
        "version": "0.3.0",
        "description": "Claude Code usage analytics via Model Context Protocol",
        "author": "nwiizo",
        "homepage": "https://github.com/nwiizo/claudelytics",
        "capabilities": {
            "resources": true,
            "tools": true,
            "prompts": false
        },
        "protocolVersion": "1.0.0"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_mcp_server_creation() {
        let temp_dir = TempDir::new().unwrap();
        let server = McpServer::new(temp_dir.path().to_path_buf());

        assert_eq!(server.list_resources().len(), 4);
        assert_eq!(server.list_tools().len(), 3);
    }

    #[test]
    fn test_server_info() {
        let info = get_server_info();
        assert_eq!(info["name"], "claudelytics");
        assert_eq!(info["protocolVersion"], "1.0.0");
    }
}
