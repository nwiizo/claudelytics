use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::types::{TokenUsage, Usage};

/// Claude session metadata from first line of JSONL file
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ClaudeSessionSummary {
    #[serde(rename = "type")]
    pub record_type: String,
    pub summary: String,
    #[serde(rename = "leafUuid")]
    pub leaf_uuid: String,
}

/// Complete Claude session with metadata and messages
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ClaudeSession {
    pub file_path: PathBuf,
    pub project_path: String,
    pub session_id: String,
    pub summary: String,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub message_count: usize,
    pub usage: TokenUsage,
}

/// Claude conversation message
#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct ClaudeMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub timestamp: DateTime<Utc>,
    pub message: MessageContent,
    pub uuid: String,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

/// Message content within Claude message
#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct MessageContent {
    pub role: String,
    pub content: Vec<ContentPart>,
    pub usage: Option<Usage>,
}

/// Content part of message
#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}
