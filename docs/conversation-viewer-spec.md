# Conversation Viewer Specification for Claudelytics

## Overview

This specification outlines the implementation of conversation viewing features in claudelytics, inspired by ccraw's functionality. The features will be available in both CLI and TUI interfaces.

**Note**: Cost calculation uses the `costUSD` field directly from logs without needing model-specific token pricing.

## 1. CLI Features

### 1.1 New Command: `conversation`

```bash
# View a specific conversation
claudelytics conversation <session-id>

# View conversations by project
claudelytics conversation --project <project-name>

# View recent conversations
claudelytics conversation --recent <n>

# Export conversation
claudelytics conversation <session-id> --export markdown
claudelytics conversation <session-id> --export html
claudelytics conversation <session-id> --export json

# Filter conversations
claudelytics conversation --since 20240101 --until 20240131
claudelytics conversation --search "keyword"
claudelytics conversation --role user  # Show only user messages
claudelytics conversation --role assistant  # Show only assistant messages

# Advanced viewing options
claudelytics conversation <session-id> --show-thinking  # Show thinking blocks
claudelytics conversation <session-id> --show-tools     # Show tool usage
claudelytics conversation <session-id> --compact        # Compact view
```

### 1.2 Enhanced `session` Command

```bash
# Add conversation preview to session list
claudelytics session --preview

# Show conversation summary for sessions
claudelytics session --summary
```

### 1.3 New Command: `analyze-content`

```bash
# Analyze conversation content patterns
claudelytics analyze-content --topic-modeling
claudelytics analyze-content --sentiment
claudelytics analyze-content --tool-usage
claudelytics analyze-content --message-length
```

## 2. TUI Features

### 2.1 New Tab: Conversations

- **Tab Name**: "Conversations" (shortcut: 'v')
- **Features**:
  - List view of conversations with metadata
  - Search and filter capabilities
  - Preview pane showing first few messages
  - Full conversation viewer on selection

### 2.2 Conversation Viewer Layout

```
┌─────────────────────────────────────────────────────────────┐
│ Session: project-name/2024-01-15 | Messages: 42 | Cost: $1.23│
├─────────────────────────────────────────────────────────────┤
│ [User] 2024-01-15 10:30:15                                  │
│ How can I implement a binary search tree in Rust?           │
├─────────────────────────────────────────────────────────────┤
│ [Assistant] 2024-01-15 10:30:18                            │
│ I'll help you implement a binary search tree in Rust...     │
│                                                              │
│ ```rust                                                      │
│ struct Node<T> {                                            │
│     value: T,                                               │
│     left: Option<Box<Node<T>>>,                            │
│     right: Option<Box<Node<T>>>,                           │
│ }                                                           │
│ ```                                                         │
├─────────────────────────────────────────────────────────────┤
│ [Tool Use] search_files                                     │
│ Pattern: "binary.*tree" | Files found: 3                   │
├─────────────────────────────────────────────────────────────┤
│ [j/k] Navigate | [/] Search | [t] Toggle view | [e] Export │
└─────────────────────────────────────────────────────────────┘
```

### 2.3 Enhanced Resume Tab

- Add conversation content preview
- Show recent messages from bookmarked sessions
- Quick navigation to full conversation view

### 2.4 Keyboard Shortcuts

- `v` - Switch to Conversations tab
- `c` - View full conversation (from Sessions tab)
- `t` - Toggle between compact/full view
- `T` - Toggle thinking blocks visibility
- `o` - Toggle tool usage visibility
- `e` - Export current conversation
- `/` - Search within conversation
- `n/N` - Next/previous search result
- `m` - Mark conversation for comparison
- `M` - View marked conversations side-by-side

### 2.5 Search and Filter Interface

```
┌─ Search & Filter ───────────────────────────────────────────┐
│ Search: [keyword________________]                           │
│ Role: [All ▼] | Date: [Last 7 days ▼] | Project: [All ▼]  │
│ □ Show thinking blocks  □ Show tool usage  □ Compact view  │
└─────────────────────────────────────────────────────────────┘
```

## 3. Data Models

### 3.1 Extended Conversation Structures

```rust
// Conversation viewing structures
pub struct Conversation {
    pub session_id: String,
    pub project: String,
    pub messages: Vec<ConversationMessage>,
    pub metadata: ConversationMetadata,
    pub summary: Option<String>,
}

pub struct ConversationMessage {
    pub uuid: String,
    pub parent_uuid: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub role: MessageRole,
    pub content: Vec<ContentBlock>,
    pub tool_use: Option<ToolUse>,
    pub thinking: Option<String>,
}

pub enum ContentBlock {
    Text(String),
    Code { language: String, content: String },
    Image { url: String, alt_text: Option<String> },
    ToolUse(ToolUse),
}

pub struct ConversationMetadata {
    pub total_messages: usize,
    pub duration: Duration,
    pub total_cost: f64,
    pub token_usage: TokenUsage,
    pub tools_used: Vec<String>,
    pub has_thinking_blocks: bool,
}

// Analysis structures
pub struct ContentAnalysis {
    pub topics: Vec<Topic>,
    pub sentiment: SentimentAnalysis,
    pub message_stats: MessageStatistics,
    pub tool_usage_patterns: ToolUsagePatterns,
}

pub struct Topic {
    pub name: String,
    pub confidence: f64,
    pub message_count: usize,
}

pub struct SentimentAnalysis {
    pub overall: f64,  // -1.0 to 1.0
    pub by_role: HashMap<MessageRole, f64>,
    pub trend: Vec<(DateTime<Utc>, f64)>,
}
```

### 3.2 Display Formatting

```rust
pub struct ConversationFormatter {
    pub show_thinking: bool,
    pub show_tools: bool,
    pub compact_mode: bool,
    pub syntax_highlighting: bool,
    pub max_line_length: usize,
}

impl ConversationFormatter {
    pub fn format_message(&self, msg: &ConversationMessage) -> String;
    pub fn format_code_block(&self, language: &str, code: &str) -> String;
    pub fn format_tool_use(&self, tool: &ToolUse) -> String;
    pub fn format_thinking(&self, thinking: &str) -> String;
}
```

## 4. Implementation Modules

### 4.1 New Modules

1. **`conversation_parser.rs`**
   - Parse full conversation content from JSONL
   - Handle message threading and relationships
   - Extract thinking blocks and tool usage

2. **`conversation_display.rs`**
   - Format conversations for terminal display
   - Syntax highlighting for code blocks
   - Markdown rendering for terminal

3. **`content_analysis.rs`**
   - Topic modeling using TF-IDF
   - Basic sentiment analysis
   - Message pattern analysis

4. **`conversation_export.rs`**
   - Export to Markdown
   - Export to HTML with syntax highlighting
   - Export to JSON

### 4.2 Enhanced Modules

1. **`tui.rs`**
   - Add Conversations tab
   - Implement conversation viewer widget
   - Add search and filter UI

2. **`models.rs`**
   - Add conversation-specific structures
   - Extend existing message types

3. **`main.rs`**
   - Add conversation subcommand
   - Add analyze-content subcommand

## 5. Performance Considerations

### 5.1 Lazy Loading
- Load conversation metadata first
- Load full content on demand
- Implement pagination for long conversations

### 5.2 Caching
- Cache parsed conversations in memory
- LRU cache for recently viewed conversations
- Optional disk cache for large datasets

### 5.3 Search Optimization
- Build search index for conversations
- Use parallel processing for content analysis
- Implement incremental search

## 6. Configuration

### 6.1 New Configuration Options

```yaml
# config.yaml additions
conversation:
  default_format: enhanced  # enhanced, compact, markdown
  show_thinking: true
  show_tools: true
  syntax_highlighting: true
  max_preview_lines: 10
  export_format: markdown  # markdown, html, json
  
analysis:
  enable_topic_modeling: true
  enable_sentiment: true
  topic_count: 10
  
cache:
  enable_conversation_cache: true
  max_cache_size_mb: 100
  cache_ttl_hours: 24
```

## 7. Error Handling

### 7.1 Graceful Degradation
- Handle missing or corrupted JSONL files
- Skip unparseable messages
- Provide clear error messages

### 7.2 User Feedback
- Progress indicators for long operations
- Clear status messages
- Helpful error suggestions

## 8. Testing Strategy

### 8.1 Unit Tests
- Test conversation parsing
- Test formatting functions
- Test search and filter logic

### 8.2 Integration Tests
- Test CLI commands with sample data
- Test TUI navigation
- Test export functionality

### 8.3 Performance Tests
- Benchmark large conversation loading
- Test search performance
- Memory usage profiling

## 9. Documentation

### 9.1 User Documentation
- Add conversation viewing guide
- Document keyboard shortcuts
- Provide usage examples

### 9.2 Developer Documentation
- Document new modules and APIs
- Add architectural decision records
- Provide contribution guidelines

## 10. Future Enhancements

### 10.1 Phase 2 Features
- AI-powered conversation summaries
- Advanced topic modeling with ML
- Conversation comparison tools
- Integration with Claude API for live viewing

### 10.2 Phase 3 Features
- Web UI for conversation viewing
- Collaborative annotation features
- Advanced analytics dashboard
- Plugin system for custom analyzers