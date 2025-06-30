//! Conversation Display Module
//!
//! Provides formatting and display utilities for Claude conversations in the terminal.
//! Supports syntax highlighting for code blocks, distinct visual styles for different
//! content types, and both compact and detailed view modes.

use crate::conversation_parser::{
    Conversation, ConversationMessage, MessageContentBlock, MessageThread,
};
use crate::models::{ClaudeMessage, ClaudeSession, ContentPart};
use colored::*;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

/// Display mode for conversation rendering
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DisplayMode {
    /// Compact view with minimal details
    Compact,
    /// Detailed view with all information
    Detailed,
}

/// Conversation display configuration
#[derive(Clone, Debug)]
pub struct ConversationDisplay {
    /// Terminal width for wrapping
    terminal_width: usize,
    /// Indentation level
    indent_level: usize,
    /// Display mode
    mode: DisplayMode,
}

impl Default for ConversationDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationDisplay {
    /// Create a new conversation display handler
    pub fn new() -> Self {
        Self {
            terminal_width: 80,
            indent_level: 2,
            mode: DisplayMode::Detailed,
        }
    }

    /// Set the terminal width for text wrapping
    pub fn with_terminal_width(mut self, width: usize) -> Self {
        self.terminal_width = width;
        self
    }

    /// Set the display mode
    pub fn with_mode(mut self, mode: DisplayMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set display mode (mutable reference version)
    pub fn set_mode(&mut self, mode: DisplayMode) {
        self.mode = mode;
    }

    /// Format a conversation for terminal display
    pub fn format_conversation(&self, conversation: &Conversation) -> String {
        let mut output = String::new();

        // Display conversation header
        output.push_str(&self.format_conversation_header(conversation));
        output.push('\n');

        // Display messages in threaded structure
        let threads = conversation.get_thread_structure();
        for thread in threads {
            output.push_str(&self.format_thread(&thread, 0));
        }

        // Display conversation summary
        if self.mode == DisplayMode::Detailed {
            output.push_str(&self.format_conversation_summary(conversation));
        }

        output
    }

    /// Format a Claude session for terminal display
    #[allow(dead_code)]
    pub fn format_claude_session(&self, session: &ClaudeSession) -> String {
        let mut output = String::new();

        // Session header
        output.push_str(&format!(
            "{}\n",
            "═".repeat(self.terminal_width).bright_black()
        ));
        output.push_str(&format!(
            "{} {}\n",
            "💬 Claude Session".bright_blue().bold(),
            session.session_id.dimmed()
        ));
        output.push_str(&format!("Summary: {}\n", session.summary));
        output.push_str(&format!(
            "Messages: {} | Tokens: {} | Cost: ${:.4}\n",
            session.message_count,
            session.usage.total_tokens(),
            session.usage.total_cost
        ));
        output.push_str(&format!(
            "{}\n\n",
            "═".repeat(self.terminal_width).bright_black()
        ));

        output
    }

    /// Format conversation header
    fn format_conversation_header(&self, conversation: &Conversation) -> String {
        let mut header = String::new();

        header.push_str(&format!(
            "{}\n",
            "═".repeat(self.terminal_width).bright_black()
        ));

        if let Some(ref summary) = conversation.summary {
            header.push_str(&format!(
                "{} {}\n",
                "📄".bright_blue(),
                summary.summary.bright_white().bold()
            ));
        } else {
            header.push_str(&format!("{}\n", "💬 Conversation".bright_blue().bold()));
        }

        if let (Some(start), Some(end)) = (conversation.started_at, conversation.ended_at) {
            let duration = end - start;
            header.push_str(&format!(
                "{} {} → {} ({})\n",
                "⏱️".dimmed(),
                start.format("%Y-%m-%d %H:%M:%S").to_string().dimmed(),
                end.format("%H:%M:%S").to_string().dimmed(),
                format_duration(&duration).dimmed()
            ));
        }

        header.push_str(&format!(
            "{} Total: {} tokens (${:.4})\n",
            "💰".dimmed(),
            conversation.total_usage.total_tokens().to_string().yellow(),
            conversation.total_usage.total_cost.to_string().green()
        ));

        header.push_str(&format!(
            "{}\n",
            "═".repeat(self.terminal_width).bright_black()
        ));

        header
    }

    /// Format a message thread recursively
    fn format_thread(&self, thread: &MessageThread, depth: usize) -> String {
        let mut output = String::new();

        // Format the message
        output.push_str(&self.format_conversation_message(&thread.message, depth));

        // Format children with increased depth
        for child in &thread.children {
            output.push_str(&self.format_thread(child, depth + 1));
        }

        output
    }

    /// Format a single conversation message
    fn format_conversation_message(&self, message: &ConversationMessage, depth: usize) -> String {
        let mut output = String::new();
        let indent = " ".repeat(depth * self.indent_level);

        // Message header
        let role_icon = match message.role.as_str() {
            "user" => "👤",
            "assistant" => "🤖",
            _ => "📝",
        };

        let role_color = match message.role.as_str() {
            "user" => "cyan",
            "assistant" => "green",
            _ => "yellow",
        };

        output.push_str(&format!(
            "{}{} {} ",
            indent,
            role_icon,
            message.role.color(role_color).bold()
        ));

        if self.mode == DisplayMode::Detailed {
            output.push_str(&format!(
                "({}) ",
                message.timestamp.format("%H:%M:%S").to_string().dimmed()
            ));

            if let Some(ref model) = message.model {
                output.push_str(&format!("[{}] ", model.dimmed()));
            }
        }

        output.push('\n');

        // Message content
        for content_block in &message.content {
            output.push_str(&self.format_content_block(content_block, depth + 1));
        }

        // Token usage (if available and in detailed mode)
        if self.mode == DisplayMode::Detailed {
            if let Some(ref usage) = message.usage {
                output.push_str(&format!(
                    "{}  {} tokens: {} in, {} out\n",
                    indent,
                    "📊".dimmed(),
                    usage.input_tokens.to_string().dimmed(),
                    usage.output_tokens.to_string().dimmed()
                ));
            }
        }

        output.push('\n');
        output
    }

    /// Format a content block with appropriate styling
    fn format_content_block(&self, block: &MessageContentBlock, depth: usize) -> String {
        let indent = " ".repeat(depth * self.indent_level);

        match block {
            MessageContentBlock::Text { content_type, text } => match content_type.as_str() {
                "thinking" => self.format_thinking_block(text, &indent),
                _ => self.format_text_block(text, &indent),
            },
            MessageContentBlock::ToolUse {
                content_type: _,
                id,
                name,
                input,
            } => self.format_tool_use_block(id, name, input, &indent),
            MessageContentBlock::ToolResult {
                content_type: _,
                tool_use_id,
                content,
            } => self.format_tool_result_block(tool_use_id, content, &indent),
        }
    }

    /// Format a thinking block with special styling
    fn format_thinking_block(&self, text: &str, indent: &str) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "{}{}\n",
            indent,
            "💭 Thinking...".italic().bright_magenta()
        ));

        let max_width = self.terminal_width.saturating_sub(indent.len() + 2);
        let wrapped = self.wrap_text(text, max_width.max(20));
        for line in wrapped.lines() {
            output.push_str(&format!("{}  {}\n", indent, line.italic().bright_black()));
        }

        output
    }

    /// Format a text block with code detection and syntax highlighting
    fn format_text_block(&self, text: &str, indent: &str) -> String {
        let mut output = String::new();
        let mut in_code_block = false;
        let mut code_language = String::new();
        let mut code_buffer = String::new();

        for line in text.lines() {
            if line.starts_with("```") {
                if in_code_block {
                    // End of code block - highlight and output
                    output.push_str(&self.format_code_block(&code_buffer, &code_language, indent));
                    code_buffer.clear();
                    in_code_block = false;
                } else {
                    // Start of code block
                    code_language = line.trim_start_matches("```").trim().to_string();
                    in_code_block = true;
                }
            } else if in_code_block {
                code_buffer.push_str(line);
                code_buffer.push('\n');
            } else {
                // Regular text - wrap and output
                let max_width = self.terminal_width.saturating_sub(indent.len());
                let wrapped = self.wrap_text(line, max_width.max(20));
                for wrapped_line in wrapped.lines() {
                    output.push_str(&format!("{}{}\n", indent, wrapped_line));
                }
            }
        }

        // Handle unclosed code block
        if in_code_block && !code_buffer.is_empty() {
            output.push_str(&self.format_code_block(&code_buffer, &code_language, indent));
        }

        output
    }

    /// Format a code block with simple highlighting
    fn format_code_block(&self, code: &str, language: &str, indent: &str) -> String {
        let mut output = String::new();

        // Code block header
        output.push_str(&format!(
            "{}{} {}\n",
            indent,
            "📄".bright_yellow(),
            if language.is_empty() {
                "Code".bright_yellow()
            } else {
                language.bright_yellow()
            }
        ));

        // Apply simple color based on language
        let color = match language {
            "rust" | "rs" => "bright_yellow",
            "python" | "py" => "bright_blue",
            "javascript" | "js" | "typescript" | "ts" => "bright_cyan",
            "bash" | "sh" => "bright_green",
            _ => "bright_white",
        };

        // Format code with simple coloring
        for line in code.lines() {
            let colored_line = match color {
                "bright_yellow" => line.bright_yellow(),
                "bright_blue" => line.bright_blue(),
                "bright_cyan" => line.bright_cyan(),
                "bright_green" => line.bright_green(),
                _ => line.bright_white(),
            };
            output.push_str(&format!("{}  {}\n", indent, colored_line));
        }

        output
    }

    /// Format a tool use block
    fn format_tool_use_block(
        &self,
        id: &str,
        name: &str,
        input: &serde_json::Value,
        indent: &str,
    ) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "{}{} {} {}\n",
            indent,
            "🔧".bright_cyan(),
            "Tool:".bright_cyan().bold(),
            name.bright_white()
        ));

        if self.mode == DisplayMode::Detailed {
            output.push_str(&format!("{}  {} {}\n", indent, "ID:".dimmed(), id.dimmed()));

            // Format tool input as pretty JSON
            if let Ok(pretty_json) = serde_json::to_string_pretty(input) {
                for line in pretty_json.lines() {
                    output.push_str(&format!("{}  {}\n", indent, line.bright_black()));
                }
            }
        }

        output
    }

    /// Format a tool result block
    fn format_tool_result_block(&self, tool_use_id: &str, content: &str, indent: &str) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "{}{} {} {}\n",
            indent,
            "✅".bright_green(),
            "Result:".bright_green().bold(),
            format!("({})", tool_use_id).dimmed()
        ));

        // Wrap and format the result content
        let max_width = self.terminal_width.saturating_sub(indent.len() + 2);
        let wrapped = self.wrap_text(content, max_width.max(20));
        for line in wrapped.lines() {
            output.push_str(&format!("{}  {}\n", indent, line.bright_black()));
        }

        output
    }

    /// Format a Claude message
    #[allow(dead_code)]
    fn format_claude_message(&self, message: &ClaudeMessage) -> String {
        let mut output = String::new();

        let role = &message.message.role;
        let role_icon = match role.as_str() {
            "user" => "👤",
            "assistant" => "🤖",
            _ => "📝",
        };

        let role_color = match role.as_str() {
            "user" => "cyan",
            "assistant" => "green",
            _ => "yellow",
        };

        // Message header
        output.push_str(&format!("{} {} ", role_icon, role.color(role_color).bold()));

        output.push_str(&format!(
            "({}) ",
            message.timestamp.format("%H:%M:%S").to_string().dimmed()
        ));

        output.push('\n');

        // Message content
        for part in &message.message.content {
            output.push_str(&self.format_content_part(part));
        }

        output
    }

    /// Format a content part
    #[allow(dead_code)]
    fn format_content_part(&self, part: &ContentPart) -> String {
        let mut output = String::new();

        if let Some(text) = &part.text {
            if part.content_type == "tool_use" {
                // Try to parse as JSON for tool use
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(text) {
                    if let (Some(name), Some(input)) =
                        (json_value.get("name"), json_value.get("input"))
                    {
                        output.push_str(&self.format_tool_use_block(
                            "tool",
                            name.to_string().trim_matches('"'),
                            input,
                            "  ",
                        ));
                        return output;
                    }
                }
            }

            // Regular text content
            let max_width = self.terminal_width.saturating_sub(2);
            let wrapped = self.wrap_text(text, max_width.max(20));
            for line in wrapped.lines() {
                output.push_str(&format!("  {}\n", line));
            }
        }

        output
    }

    /// Format conversation summary
    fn format_conversation_summary(&self, conversation: &Conversation) -> String {
        let mut summary = String::new();

        summary.push_str(&format!(
            "\n{}\n",
            "─".repeat(self.terminal_width).bright_black()
        ));

        summary.push_str(&format!(
            "{}\n",
            "📊 Conversation Summary".bright_blue().bold()
        ));

        summary.push_str(&format!(
            "  Messages: {}\n",
            conversation.messages.len().to_string().yellow()
        ));

        // Count by role
        let user_count = conversation
            .messages
            .iter()
            .filter(|m| m.role == "user")
            .count();
        let assistant_count = conversation
            .messages
            .iter()
            .filter(|m| m.role == "assistant")
            .count();

        summary.push_str(&format!(
            "  User: {} | Assistant: {}\n",
            user_count.to_string().cyan(),
            assistant_count.to_string().green()
        ));

        // Token breakdown
        let usage = &conversation.total_usage;
        summary.push_str(&format!(
            "  Tokens: {} (Input: {}, Output: {})\n",
            usage.total_tokens().to_string().yellow(),
            usage.input_tokens.to_string().dimmed(),
            usage.output_tokens.to_string().dimmed()
        ));

        if usage.cache_read_tokens > 0 || usage.cache_creation_tokens > 0 {
            summary.push_str(&format!(
                "  Cache: {} read, {} created\n",
                usage.cache_read_tokens.to_string().green(),
                usage.cache_creation_tokens.to_string().blue()
            ));
        }

        summary.push_str(&format!(
            "  Cost: ${:.4}\n",
            usage.total_cost.to_string().green()
        ));

        summary
    }

    /// Wrap text to fit terminal width
    fn wrap_text(&self, text: &str, max_width: usize) -> String {
        let mut result = String::new();

        for line in text.lines() {
            if line.len() <= max_width {
                result.push_str(line);
                result.push('\n');
            } else {
                // Simple word wrapping
                let mut current_line = String::new();
                for word in line.split_whitespace() {
                    if current_line.len() + word.len() + 1 > max_width && !current_line.is_empty() {
                        result.push_str(&current_line);
                        result.push('\n');
                        current_line.clear();
                    }
                    if !current_line.is_empty() {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                }
                if !current_line.is_empty() {
                    result.push_str(&current_line);
                    result.push('\n');
                }
            }
        }

        // Remove trailing newline
        if result.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Create ratatui Text widget for conversation display
    #[allow(dead_code)]
    pub fn create_conversation_widget(&self, conversation: &Conversation) -> Text<'static> {
        let mut lines = Vec::new();

        // Header
        if let Some(ref summary) = conversation.summary {
            lines.push(Line::from(vec![
                Span::raw("📄 "),
                Span::styled(
                    summary.summary.clone(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        // Add separator
        lines.push(Line::from("─".repeat(50)));

        // Format messages
        let threads = conversation.get_thread_structure();
        for thread in threads {
            self.add_thread_lines(&thread, &mut lines, 0);
        }

        Text::from(lines)
    }

    /// Add thread lines for ratatui display
    #[allow(dead_code, clippy::only_used_in_recursion)]
    fn add_thread_lines(
        &self,
        thread: &MessageThread,
        lines: &mut Vec<Line<'static>>,
        depth: usize,
    ) {
        let indent = " ".repeat(depth * 2);
        let message = &thread.message;

        // Role line
        let (icon, color) = match message.role.as_str() {
            "user" => ("👤", Color::Cyan),
            "assistant" => ("🤖", Color::Green),
            _ => ("📝", Color::Yellow),
        };

        lines.push(Line::from(vec![
            Span::raw(format!("{}{} ", indent, icon)),
            Span::styled(
                message.role.clone(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({})", message.timestamp.format("%H:%M:%S")),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        // Content lines
        for block in &message.content {
            match block {
                MessageContentBlock::Text { content_type, text } => {
                    if content_type == "thinking" {
                        lines.push(Line::from(vec![
                            Span::raw(format!("{}  ", indent)),
                            Span::styled("💭 ", Style::default().fg(Color::Magenta)),
                            Span::styled(
                                text.clone(),
                                Style::default()
                                    .fg(Color::DarkGray)
                                    .add_modifier(Modifier::ITALIC),
                            ),
                        ]));
                    } else {
                        // Split text into lines for proper display
                        for text_line in text.lines() {
                            lines.push(Line::from(vec![
                                Span::raw(format!("{}  ", indent)),
                                Span::raw(text_line.to_string()),
                            ]));
                        }
                    }
                }
                MessageContentBlock::ToolUse { name, .. } => {
                    lines.push(Line::from(vec![
                        Span::raw(format!("{}  ", indent)),
                        Span::styled("🔧 Tool: ", Style::default().fg(Color::Blue)),
                        Span::styled(name.clone(), Style::default().fg(Color::White)),
                    ]));
                }
                MessageContentBlock::ToolResult { .. } => {
                    lines.push(Line::from(vec![
                        Span::raw(format!("{}  ", indent)),
                        Span::styled("✅ Result", Style::default().fg(Color::Green)),
                    ]));
                }
            }
        }

        // Add empty line between messages
        lines.push(Line::from(""));

        // Process children
        for child in &thread.children {
            self.add_thread_lines(child, lines, depth + 1);
        }
    }
}

/// Format duration for display
fn format_duration(duration: &chrono::Duration) -> String {
    let total_seconds = duration.num_seconds();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

impl ConversationDisplay {
    /// Format a single conversation message for TUI display
    pub fn format_conversation_message_for_tui(
        &self,
        message: &ConversationMessage,
        show_thinking: bool,
        show_tools: bool,
    ) -> Text<'static> {
        let mut lines = Vec::new();

        // Add role and timestamp
        lines.push(Line::from(vec![
            Span::styled("Role: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                message.role.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled("Time: ", Style::default().fg(Color::Yellow)),
            Span::styled(
                message.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                Style::default().fg(Color::Gray),
            ),
        ]));

        if let Some(ref model) = message.model {
            lines.push(Line::from(vec![
                Span::styled("Model: ", Style::default().fg(Color::Yellow)),
                Span::styled(model.clone(), Style::default().fg(Color::Gray)),
            ]));
        }

        lines.push(Line::from(""));

        // Format content blocks
        for block in &message.content {
            match block {
                MessageContentBlock::Text { content_type, text } => {
                    match content_type.as_str() {
                        "thinking" => {
                            if show_thinking {
                                lines.push(Line::from(vec![Span::styled(
                                    "🤔 [THINKING] ",
                                    Style::default()
                                        .fg(Color::Magenta)
                                        .add_modifier(Modifier::ITALIC),
                                )]));
                                for line in text.lines() {
                                    lines.push(Line::from(vec![
                                        Span::raw("  "),
                                        Span::styled(
                                            line.to_string(),
                                            Style::default()
                                                .fg(Color::DarkGray)
                                                .add_modifier(Modifier::ITALIC),
                                        ),
                                    ]));
                                }
                            }
                        }
                        _ => {
                            // Regular text - check for code blocks
                            let mut in_code_block = false;

                            for line in text.lines() {
                                if let Some(stripped) = line.strip_prefix("```") {
                                    if !in_code_block {
                                        let code_lang = stripped.trim();
                                        in_code_block = true;
                                        lines.push(Line::from(vec![Span::styled(
                                            format!("```{}", code_lang),
                                            Style::default().fg(Color::DarkGray),
                                        )]));
                                    } else {
                                        in_code_block = false;
                                        lines.push(Line::from(vec![Span::styled(
                                            "```",
                                            Style::default().fg(Color::DarkGray),
                                        )]));
                                    }
                                } else if in_code_block {
                                    // Syntax highlighting would go here based on code_lang
                                    lines.push(Line::from(vec![Span::styled(
                                        line.to_string(),
                                        Style::default().fg(Color::Green),
                                    )]));
                                } else {
                                    lines.push(Line::from(line.to_string()));
                                }
                            }
                        }
                    }
                }
                MessageContentBlock::ToolUse { name, input, .. } => {
                    if show_tools {
                        lines.push(Line::from(vec![Span::styled(
                            format!("🔧 [TOOL USE: {}] ", name),
                            Style::default().fg(Color::Cyan),
                        )]));
                        if let Ok(formatted) = serde_json::to_string_pretty(input) {
                            for line in formatted.lines() {
                                lines.push(Line::from(vec![
                                    Span::raw("  "),
                                    Span::styled(
                                        line.to_string(),
                                        Style::default().fg(Color::DarkGray),
                                    ),
                                ]));
                            }
                        }
                    }
                }
                MessageContentBlock::ToolResult { content, .. } => {
                    if show_tools {
                        lines.push(Line::from(vec![Span::styled(
                            "✅ [TOOL RESULT] ",
                            Style::default().fg(Color::Green),
                        )]));
                        for line in content.lines().take(10) {
                            // Limit output lines
                            lines.push(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(
                                    line.to_string(),
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ]));
                        }
                        if content.lines().count() > 10 {
                            lines.push(Line::from(vec![
                                Span::raw("  "),
                                Span::styled(
                                    "... (truncated)",
                                    Style::default()
                                        .fg(Color::DarkGray)
                                        .add_modifier(Modifier::ITALIC),
                                ),
                            ]));
                        }
                    }
                }
            }
            lines.push(Line::from(""));
        }

        // Add token usage if available
        if self.mode == DisplayMode::Detailed {
            if let Some(ref usage) = message.usage {
                lines.push(Line::from(vec![
                    Span::styled("📊 Tokens: ", Style::default().fg(Color::Yellow)),
                    Span::styled(
                        format!("{} in, {} out", usage.input_tokens, usage.output_tokens),
                        Style::default().fg(Color::Gray),
                    ),
                ]));
            }
        }

        Text::from(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation_parser::ConversationSummary;
    use chrono::Utc;

    #[test]
    fn test_conversation_display_creation() {
        let display = ConversationDisplay::new()
            .with_terminal_width(100)
            .with_mode(DisplayMode::Compact);

        assert_eq!(display.terminal_width, 100);
        assert_eq!(display.mode, DisplayMode::Compact);
    }

    #[test]
    fn test_format_empty_conversation() {
        let display = ConversationDisplay::new();
        let conversation = Conversation {
            file_path: std::path::PathBuf::from("test.jsonl"),
            summary: None,
            messages: Vec::new(),
            message_index: std::collections::HashMap::new(),
            total_usage: Default::default(),
            started_at: None,
            ended_at: None,
        };

        let output = display.format_conversation(&conversation);
        assert!(!output.is_empty());
        assert!(output.contains("Conversation"));
    }

    #[test]
    fn test_format_conversation_with_summary() {
        let display = ConversationDisplay::new();
        let conversation = Conversation {
            file_path: std::path::PathBuf::from("test.jsonl"),
            summary: Some(ConversationSummary {
                record_type: "summary".to_string(),
                summary: "Test Session".to_string(),
                leaf_uuid: "test-uuid".to_string(),
            }),
            messages: Vec::new(),
            message_index: std::collections::HashMap::new(),
            total_usage: Default::default(),
            started_at: Some(Utc::now()),
            ended_at: Some(Utc::now()),
        };

        let output = display.format_conversation(&conversation);
        assert!(output.contains("Test Session"));
    }

    #[test]
    fn test_wrap_text() {
        let display = ConversationDisplay::new().with_terminal_width(20);
        let text = "This is a very long line that should be wrapped";
        let wrapped = display.wrap_text(text, 20);

        assert!(wrapped.lines().count() > 1);
        assert!(wrapped.lines().all(|line| line.len() <= 20));
    }

    #[test]
    fn test_format_thinking_block() {
        let display = ConversationDisplay::new();
        let output = display.format_thinking_block("Let me think about this...", "  ");

        assert!(output.contains("💭"));
        assert!(output.contains("Thinking..."));
        assert!(output.contains("Let me think about this..."));
    }
}
