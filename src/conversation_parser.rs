use crate::models::TokenUsage;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Represents a complete conversation from a JSONL file
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct Conversation {
    /// Path to the JSONL file
    pub file_path: PathBuf,
    /// Session metadata from the first line (if type="summary")
    pub summary: Option<ConversationSummary>,
    /// All messages in the conversation with parent/child relationships preserved
    pub messages: Vec<ConversationMessage>,
    /// Map of UUID to message index for quick parent lookups
    pub message_index: HashMap<String, usize>,
    /// Total token usage for the entire conversation
    pub total_usage: TokenUsage,
    /// Timestamp of the first message
    pub started_at: Option<DateTime<Utc>>,
    /// Timestamp of the last message
    pub ended_at: Option<DateTime<Utc>>,
}

/// Session summary from the first line of JSONL (when type="summary")
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConversationSummary {
    #[serde(rename = "type")]
    pub record_type: String,
    pub summary: String,
    #[serde(rename = "leafUuid")]
    pub leaf_uuid: String,
}

/// Extended message structure that includes all conversation details
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ConversationMessage {
    /// Unique identifier for this message
    pub uuid: String,
    /// Parent message UUID (for threading)
    pub parent_uuid: Option<String>,
    /// Message type (user, assistant, tool_result)
    pub message_type: String,
    /// Timestamp of the message
    pub timestamp: DateTime<Utc>,
    /// Role (user, assistant)
    pub role: String,
    /// Message content with text and tool usage
    pub content: Vec<MessageContentBlock>,
    /// Token usage for this specific message
    pub usage: Option<TokenUsage>,
    /// Model used (for assistant messages)
    pub model: Option<String>,
    /// Session ID this message belongs to
    pub session_id: String,
    /// Working directory when message was sent
    pub cwd: Option<String>,
    /// Whether this is a sidechain message
    pub is_sidechain: bool,
}

/// Content block within a message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContentBlock {
    /// Text content (including thinking blocks)
    Text {
        #[serde(rename = "type")]
        content_type: String,
        text: String,
    },
    /// Tool use content
    ToolUse {
        #[serde(rename = "type")]
        content_type: String,
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result content
    ToolResult {
        #[serde(rename = "type")]
        content_type: String,
        tool_use_id: String,
        content: String,
    },
}

/// Parser for conversation JSONL files
#[allow(dead_code)]
pub struct ConversationParser {
    claude_dir: PathBuf,
}

#[allow(dead_code)]
impl ConversationParser {
    /// Create a new conversation parser
    pub fn new(claude_dir: PathBuf) -> Self {
        Self { claude_dir }
    }

    /// Parse a single conversation file
    pub fn parse_conversation(&self, file_path: &Path) -> Result<Conversation> {
        let file = File::open(file_path).with_context(|| {
            format!("Failed to open conversation file: {}", file_path.display())
        })?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        let mut conversation = Conversation {
            file_path: file_path.to_path_buf(),
            summary: None,
            messages: Vec::new(),
            message_index: HashMap::new(),
            total_usage: TokenUsage::default(),
            started_at: None,
            ended_at: None,
        };

        // Check if first line is a summary
        if let Some(first_line) = lines.next() {
            let first_line = first_line?;
            if let Ok(record) = serde_json::from_str::<serde_json::Value>(&first_line) {
                if record.get("type").and_then(|t| t.as_str()) == Some("summary") {
                    // Parse as summary
                    if let Ok(summary) = serde_json::from_value::<ConversationSummary>(record) {
                        conversation.summary = Some(summary);
                    }
                } else {
                    // It's a regular message, parse it
                    self.parse_message_line(&first_line, &mut conversation)?;
                }
            }
        }

        // Parse remaining lines as messages
        for line in lines {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            self.parse_message_line(&line, &mut conversation)?;
        }

        // Update start and end times
        if let Some(first_msg) = conversation.messages.first() {
            conversation.started_at = Some(first_msg.timestamp);
        }
        if let Some(last_msg) = conversation.messages.last() {
            conversation.ended_at = Some(last_msg.timestamp);
        }

        Ok(conversation)
    }

    /// Parse a single message line and add it to the conversation
    fn parse_message_line(&self, line: &str, conversation: &mut Conversation) -> Result<()> {
        let record: serde_json::Value =
            serde_json::from_str(line).with_context(|| "Failed to parse JSON line")?;

        // Skip non-message records (like summaries we've already handled)
        let record_type = record.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if record_type == "summary" {
            return Ok(());
        }

        // Extract message fields
        let uuid = record
            .get("uuid")
            .and_then(|u| u.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing uuid in message"))?
            .to_string();

        let parent_uuid = record
            .get("parentUuid")
            .and_then(|p| p.as_str())
            .map(|s| s.to_string());

        let timestamp = record
            .get("timestamp")
            .and_then(|t| t.as_str())
            .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
            .map(|t| t.with_timezone(&Utc))
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid timestamp"))?;

        let session_id = record
            .get("sessionId")
            .and_then(|s| s.as_str())
            .unwrap_or("")
            .to_string();

        let cwd = record
            .get("cwd")
            .and_then(|c| c.as_str())
            .map(|s| s.to_string());

        let is_sidechain = record
            .get("isSidechain")
            .and_then(|s| s.as_bool())
            .unwrap_or(false);

        // Parse message content
        if let Some(message) = record.get("message") {
            let role = message
                .get("role")
                .and_then(|r| r.as_str())
                .unwrap_or("unknown")
                .to_string();

            let model = message
                .get("model")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());

            // Parse content blocks
            let content = self.parse_content_blocks(message)?;

            // Parse usage if available
            let usage = if let Some(usage_data) = message.get("usage") {
                self.parse_usage(usage_data)?
            } else {
                None
            };

            // Add to total usage
            if let Some(ref msg_usage) = usage {
                conversation.total_usage.add(msg_usage);
            }

            let msg_index = conversation.messages.len();
            conversation.message_index.insert(uuid.clone(), msg_index);

            conversation.messages.push(ConversationMessage {
                uuid,
                parent_uuid,
                message_type: record_type.to_string(),
                timestamp,
                role,
                content,
                usage,
                model,
                session_id,
                cwd,
                is_sidechain,
            });
        }

        Ok(())
    }

    /// Parse content blocks from message
    fn parse_content_blocks(
        &self,
        message: &serde_json::Value,
    ) -> Result<Vec<MessageContentBlock>> {
        let mut blocks = Vec::new();

        // Handle string content (old format)
        if let Some(content_str) = message.get("content").and_then(|c| c.as_str()) {
            blocks.push(MessageContentBlock::Text {
                content_type: "text".to_string(),
                text: content_str.to_string(),
            });
            return Ok(blocks);
        }

        // Handle array content (new format)
        if let Some(content_array) = message.get("content").and_then(|c| c.as_array()) {
            for content_item in content_array {
                if let Ok(block) =
                    serde_json::from_value::<MessageContentBlock>(content_item.clone())
                {
                    blocks.push(block);
                }
            }
        }

        Ok(blocks)
    }

    /// Parse usage information
    fn parse_usage(&self, usage_data: &serde_json::Value) -> Result<Option<TokenUsage>> {
        let input_tokens = usage_data
            .get("input_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);

        let output_tokens = usage_data
            .get("output_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);

        let cache_creation_tokens = usage_data
            .get("cache_creation_input_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);

        let cache_read_tokens = usage_data
            .get("cache_read_input_tokens")
            .and_then(|t| t.as_u64())
            .unwrap_or(0);

        // Cost would be calculated separately using pricing strategies
        let usage = TokenUsage {
            input_tokens,
            output_tokens,
            cache_creation_tokens,
            cache_read_tokens,
            total_cost: 0.0,
        };

        Ok(Some(usage))
    }

    /// Find all conversation files in the Claude directory
    pub fn find_conversation_files(&self) -> Result<Vec<PathBuf>> {
        use walkdir::WalkDir;

        let projects_dir = self.claude_dir.join("projects");
        if !projects_dir.exists() {
            anyhow::bail!(
                "Claude projects directory not found at {}",
                projects_dir.display()
            );
        }

        let files: Vec<PathBuf> = WalkDir::new(projects_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "jsonl")
                    .unwrap_or(false)
            })
            .map(|entry| entry.path().to_path_buf())
            .collect();

        Ok(files)
    }
}

#[allow(dead_code)]
impl Conversation {
    /// Get messages in a threaded structure
    pub fn get_thread_structure(&self) -> Vec<MessageThread> {
        let mut threads = Vec::new();
        let mut processed = std::collections::HashSet::new();

        // Find root messages (no parent)
        for (idx, message) in self.messages.iter().enumerate() {
            if message.parent_uuid.is_none() && !processed.contains(&idx) {
                let thread = self.build_thread(idx, &mut processed);
                threads.push(thread);
            }
        }

        threads
    }

    /// Build a message thread starting from a root message
    fn build_thread(
        &self,
        root_idx: usize,
        processed: &mut std::collections::HashSet<usize>,
    ) -> MessageThread {
        processed.insert(root_idx);
        let root_message = &self.messages[root_idx];

        let mut thread = MessageThread {
            message: root_message.clone(),
            children: Vec::new(),
        };

        // Find all children
        for (idx, message) in self.messages.iter().enumerate() {
            if let Some(ref parent_uuid) = message.parent_uuid {
                if parent_uuid == &root_message.uuid && !processed.contains(&idx) {
                    let child_thread = self.build_thread(idx, processed);
                    thread.children.push(child_thread);
                }
            }
        }

        thread
    }

    /// Extract all thinking blocks from the conversation
    pub fn extract_thinking_blocks(&self) -> Vec<ThinkingBlock> {
        let mut thinking_blocks = Vec::new();

        for message in &self.messages {
            if message.role == "assistant" {
                for content in &message.content {
                    if let MessageContentBlock::Text { content_type, text } = content {
                        if content_type == "thinking" {
                            thinking_blocks.push(ThinkingBlock {
                                message_uuid: message.uuid.clone(),
                                timestamp: message.timestamp,
                                content: text.clone(),
                            });
                        }
                    }
                }
            }
        }

        thinking_blocks
    }

    /// Extract all tool usage from the conversation
    pub fn extract_tool_usage(&self) -> Vec<ToolUsageRecord> {
        let mut tool_records = Vec::new();

        for message in &self.messages {
            for content in &message.content {
                if let MessageContentBlock::ToolUse {
                    content_type: _,
                    id,
                    name,
                    input,
                } = content
                {
                    tool_records.push(ToolUsageRecord {
                        message_uuid: message.uuid.clone(),
                        timestamp: message.timestamp,
                        tool_id: id.clone(),
                        tool_name: name.clone(),
                        input: input.clone(),
                    });
                }
            }
        }

        tool_records
    }
}

/// Represents a message and its children in a threaded structure
#[derive(Debug, Clone, Serialize)]
#[allow(dead_code)]
pub struct MessageThread {
    pub message: ConversationMessage,
    pub children: Vec<MessageThread>,
}

/// Represents a thinking block extracted from assistant messages
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ThinkingBlock {
    pub message_uuid: String,
    pub timestamp: DateTime<Utc>,
    pub content: String,
}

/// Represents tool usage extracted from messages
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ToolUsageRecord {
    pub message_uuid: String,
    pub timestamp: DateTime<Utc>,
    pub tool_id: String,
    pub tool_name: String,
    pub input: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_parse_conversation_with_summary() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.jsonl");
        let mut file = File::create(&file_path).unwrap();

        // Write test data
        writeln!(
            file,
            r#"{{"type":"summary","summary":"Test Session","leafUuid":"test-uuid"}}"#
        )
        .unwrap();
        writeln!(file, r#"{{"uuid":"msg1","parentUuid":null,"type":"user","timestamp":"2024-01-01T12:00:00Z","sessionId":"session1","message":{{"role":"user","content":[{{"type":"text","text":"Hello"}}]}}}}"#).unwrap();
        writeln!(file, r#"{{"uuid":"msg2","parentUuid":"msg1","type":"assistant","timestamp":"2024-01-01T12:00:01Z","sessionId":"session1","message":{{"role":"assistant","model":"claude-3-opus","content":[{{"type":"text","text":"Hi there!"}}],"usage":{{"input_tokens":10,"output_tokens":5}}}}}}"#).unwrap();

        let parser = ConversationParser::new(dir.path().to_path_buf());
        let conversation = parser.parse_conversation(&file_path).unwrap();

        assert!(conversation.summary.is_some());
        assert_eq!(
            conversation.summary.as_ref().unwrap().summary,
            "Test Session"
        );
        assert_eq!(conversation.messages.len(), 2);
        assert_eq!(conversation.messages[0].role, "user");
        assert_eq!(conversation.messages[1].role, "assistant");
        assert_eq!(conversation.total_usage.input_tokens, 10);
        assert_eq!(conversation.total_usage.output_tokens, 5);
    }

    #[test]
    fn test_extract_thinking_blocks() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.jsonl");
        let mut file = File::create(&file_path).unwrap();

        writeln!(file, r#"{{"uuid":"msg1","parentUuid":null,"type":"assistant","timestamp":"2024-01-01T12:00:00Z","sessionId":"session1","message":{{"role":"assistant","content":[{{"type":"thinking","text":"Let me think about this..."}},{{"type":"text","text":"Here's my response"}}]}}}}"#).unwrap();

        let parser = ConversationParser::new(dir.path().to_path_buf());
        let conversation = parser.parse_conversation(&file_path).unwrap();
        let thinking_blocks = conversation.extract_thinking_blocks();

        assert_eq!(thinking_blocks.len(), 1);
        assert_eq!(thinking_blocks[0].content, "Let me think about this...");
    }

    #[test]
    fn test_extract_tool_usage() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.jsonl");
        let mut file = File::create(&file_path).unwrap();

        writeln!(file, r#"{{"uuid":"msg1","parentUuid":null,"type":"assistant","timestamp":"2024-01-01T12:00:00Z","sessionId":"session1","message":{{"role":"assistant","content":[{{"type":"tool_use","id":"tool1","name":"Read","input":{{"file_path":"/test.txt"}}}}]}}}}"#).unwrap();

        let parser = ConversationParser::new(dir.path().to_path_buf());
        let conversation = parser.parse_conversation(&file_path).unwrap();
        let tool_usage = conversation.extract_tool_usage();

        assert_eq!(tool_usage.len(), 1);
        assert_eq!(tool_usage[0].tool_name, "Read");
    }

    #[test]
    fn test_thread_structure() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.jsonl");
        let mut file = File::create(&file_path).unwrap();

        writeln!(file, r#"{{"uuid":"msg1","parentUuid":null,"type":"user","timestamp":"2024-01-01T12:00:00Z","sessionId":"session1","message":{{"role":"user","content":[{{"type":"text","text":"Question 1"}}]}}}}"#).unwrap();
        writeln!(file, r#"{{"uuid":"msg2","parentUuid":"msg1","type":"assistant","timestamp":"2024-01-01T12:00:01Z","sessionId":"session1","message":{{"role":"assistant","content":[{{"type":"text","text":"Answer 1"}}]}}}}"#).unwrap();
        writeln!(file, r#"{{"uuid":"msg3","parentUuid":"msg2","type":"user","timestamp":"2024-01-01T12:00:02Z","sessionId":"session1","message":{{"role":"user","content":[{{"type":"text","text":"Follow-up"}}]}}}}"#).unwrap();

        let parser = ConversationParser::new(dir.path().to_path_buf());
        let conversation = parser.parse_conversation(&file_path).unwrap();
        let threads = conversation.get_thread_structure();

        assert_eq!(threads.len(), 1);
        assert_eq!(threads[0].children.len(), 1);
        assert_eq!(threads[0].children[0].children.len(), 1);
    }
}
