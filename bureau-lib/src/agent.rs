//! Agent loop management — checkpoints, resume, mode gating.
//! Follows Decision D6: LLM via HTTP + checkpoint/resume with JSONL history.
//! Follows Q16/B: Save partial content and context between turns.
P5: Skills assembly, dashboard context injection, inspector gate wiring.

// --------------------------------------------------------------------------
// Agent State (what the orchestrator knows about each running agent)
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct AgentLoop {
    pub ticket_id: String,
    pub agent_type: crate::ticket::AgentType,
    /// Which bureau mode this loop is operating in (execution, inspection, etc.).
    pub mode: crate::config::Mode,
    /// JoinHandle tracker for concurrency management (fan-out/fan-in).
    /// The Rust TUI tracks these to know when workers finish.
    // Note: tokio::task::JoinHandle is in the handle module since it can't be stored directly.
}

impl AgentLoop {
    /// Create a new agent loop for the given mode and ticket.
    pub fn new(ticket_id: impl Into<String>, agent_type: crate::ticket::AgentType, mode: crate::config::Mode) -> Self {
        Self {
            ticket_id: ticket_id.into(),
            agent_type,
            mode,
        }
    }
}

/// Barrier that blocks the next mode from starting until all current workers are done.
/// Follows Decision Q18/B: fan-out/fan-in with mode barriers.
pub struct ModeBarrier {
    counter: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    tx: tokio::sync::mpsc::Sender<()>,
}

impl ModeBarrier {
    /// Create a new barrier with `expected` workers that must complete before unblocking.
    pub fn new(expected: usize) -> Self {
        let (tx, _rx) = tokio::sync::broadcast::channel::<()>(1);
        Self {
            counter: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            tx,
        }
    }

    /// Register a worker that must complete before unblocking.
    pub fn add_worker(&self) {
        self.counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

// --------------------------------------------------------------------------
// P5: System prompt assembly (skills + dashboard state)
// --------------------------------------------------------------------------

impl AgentLoop {
    /// Build the full system prompt prefix for this agent loop.
    /// Composes: skills assembly → permit/dashboard context → authority info.
    pub fn assemble_system_prompt(&self, office_home: &std::path::Path) -> anyhow::Result<String> {
        let mut parts = Vec::new();

        // Skills assembly (ADR3 / Q12 ordering).
        let loader = crate::skills::SkillsLoader::new(&self.mode.to_string(), office_home);
        if let Ok(skills_prompt) = loader.assembly_prompt() {
            if !skills_prompt.is_empty() {
                parts.push(skills_prompt);
            }
        }

        // Dashboard context: provide a snapshot of current ticket/state state.
        parts.push(format!(
            "## AGENT CONTEXT\nMode: {}\nTicket: {}\n",
            self.mode, self.ticket_id
        ));

        // Note: InspectorGate is passed at runtime in the full agent loop;
        // here we note the pattern that will be wired.
        parts.push("## SCOPE STATUS\nAll writes require approved scope expansion.\n".to_string());

        Ok(parts.concat())
    }
}

// --------------------------------------------------------------------------
// Checkpoint I/O (follows Decision Q24/A: JSONL history + JSON metadata)
// --------------------------------------------------------------------------

pub mod checkpoint {
    use super::*;

    /// Save an agent's checkpoint to disk (the checkpoint file).
    pub fn save(office_home: &std::path::Path, ticket_num: u64, checkpoint: &AgentCheckpointData) -> anyhow::Result<()> {
        let ckpt_path = office_home.join(format!("{:03}.checkpoint.json", ticket_num));
        let content = serde_json::to_string_pretty(checkpoint)?;
        std::fs::write(&ckpt_path, content)?;
        Ok(())
    }

    /// Load an agent's checkpoint from disk (for resume).
    pub fn load(office_home: &std::path::Path, ticket_num: u64) -> anyhow::Result<Option<AgentCheckpointData>> {
        let ckpt_path = office_home.join(format!("{:03}.checkpoint.json", ticket_num));
        if !ckpt_path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&ckpt_path)?;
        Ok(Some(serde_json::from_str(&content)?))
    }

    /// Append a tool call to the agent's JSONL history log.
    pub fn append_history(office_home: &std::path::Path, ticket_num: u64, action: &super::agent_turn_event) -> anyhow::Result<()> {
        let log_path = office_home.join(format!("{:03}.history.jsonl", ticket_num));
        let line = serde_json::to_string(action)?;
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?
            .write_all(line.as_bytes())?;
        Ok(())
    }

    /// Load the last N events from history for context recovery.
    pub fn load_history_tail(office_home: &std::path::Path, ticket_num: u64, last_n: usize) -> anyhow::Result<Vec<super::agent_turn_event>> {
        let log_path = office_home.join(format!("{:03}.history.jsonl", ticket_num));
        if !log_path.exists() {
            return Ok(vec![]);
        }
        let content = std::fs::read_to_string(&log_path)?;
        let events: Vec<super::agent_turn_event> = content.lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(events.into_iter().rev().take(last_n).rev().collect())
    }

    /// The data stored in the checkpoint file.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AgentCheckpointData {
        pub ticket_id: String,
        pub agent_type: crate::ticket::AgentType,
        pub current_tool_calls: Option<Vec<crate::agent::ToolAction>>,
        pub previous_tool_calls: Vec<crate::ticket::ToolAction>,
        /// Partial markdown content the agent has drafted so far.
        pub partial_content: Vec<String>,
    }

    /// Event logged to JSONL history per turn.
    #[derive(Debug, Clone, Serialize)]
    pub struct AgentTurnEvent {
        pub turn_number: u64,
        pub timestamp: String,
        pub phase: String,     // which "phase" of the ticket lifecycle
        pub content_type: String, // "summary", "draft", "requirement", etc.
    }
}
