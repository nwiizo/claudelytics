//! Model Context Protocol (MCP) server implementation for claudelytics
//!
//! Provides MCP resources and tools for accessing Claude usage analytics data
//! through a standardized protocol that other applications can consume.

use serde_json::{Value, json};
use std::path::PathBuf;

/// MCP server for claudelytics data access
pub struct McpServer;

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
    pub fn new(_claude_path: PathBuf) -> Self {
        Self
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
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_mcp_server_creation() {
        let server = McpServer::new(PathBuf::from("/tmp"));

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
