//! LLM provider abstraction — HTTP-based agent execution.
//! Follows Decision Q9/B + Q22/A: LLM via HTTP API, starting single-provider mode.
//! Future: per-office multi-provider support (Q22/C).

use anyhow::Result;

/// Generic chat interface for all LLM providers.
pub trait ChatProvider {
    /// Send a message with context and get back the agent's response.
    async fn chat(&self, messages: &[ChatMessage], max_tokens: u32) -> Result<ChatResponse>;
    
    /// Send multiple messages as a single request (streaming not yet implemented).
    async fn chat_stream(
        &self,
        messages: &[ChatMessage]
    ) -> impl Stream<Item = ChatChunk> + 'static;

    #[serde(skip)]
}

/// A message in the chat history. Follows OpenAI-compatible API shapes.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
}

// TODO: implement full trait + provider implementations.
// This module is scaffolding only — real implementation needs tokio::sync::Stream for streaming.

/// The different message roles in a chat history.
#[derive(Debug, Clone)]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

/// Response from the LLM's chat endpoint.
#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    /// How many tokens were used in this completion.
    pub usage: MessageUsage,
}

/// Token usage for cost tracking and rate limiting.
pub struct MessageUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone)]
pub enum ChatChunk {
    Text(String),
    /// Tool call the LLM wants to make (file read / file write / process exec).
        // Note: tokio::sync::Stream is not used here yet. This is scaffolding only — implement proper streaming later.

/// A tool call that the agent loop intercepts and validates before executing.
/// Follows decision D7 + sandbox enforcement: Rust controls every write.  
    pub name: String,
    /// The arguments (JSON) for this tool call.
    pub args: serde_json::Value,
}
