//! Work-log system — per-ticket progress tracking for worker agents.
//!
//! Each execution ticket has a companion `{NNN}-worklog.md` file that records:
//! - **System events** (phase transitions) auto-appended by the operator layer
//! - **Agent-initiated entries** via the `work_log` tool (milestones, decisions, failures)
//!
//! The work-log is ticket-bound: agent attempts to write to a work-log whose
//! ticket number doesn't match their assigned ticket are rejected at the handler.
//! This is enforced at the operator layer (not sandbox), since bureau-internal
//! paths should not require scope expansion.
//!
//! Entry format: YAML frontmatter + H2-separated markdown body per entry.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ===========================================================================
// Entry types & serialization
// ===========================================================================

/// Type of event recorded in a work-log entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkLogEvent {
    /// Phase transition detected by the operator layer.
    System,
    /// Ongoing work — agent reports what it's currently doing.
    Draft,
    /// Completed a notable step or sub-task.
    Update,
    /// Observation, decision rationale, or failure reason.
    Note,
    /// Significant completion milestone (sub-task done, blocker removed).
    Milestone,
}

/// A single entry in the work-log file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkLogEntry {
    /// Which agent turn this corresponds to (0 for system-auto entries).
    pub turn: u64,
    /// ISO timestamp of creation.
    #[serde(with = "chrono::serde::ts_seconds")]
    pub ts: chrono::DateTime<Utc>,
    /// Type of event being recorded.
    pub event: WorkLogEvent,
    /// Which phase the ticket is in (optional; system entries always include it).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    /// Which tool triggered this entry (only for agent-initiated events).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_used: Option<String>,
    /// Related ticket IDs for cross-ticket traceability.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refers_to: Vec<String>,
    /// The free-memo prose body (markdown).
    pub body: String,
}

impl WorkLogEntry {
    /// Create a system auto-entry for phase transitions.
    pub fn system(phase: String, message: String) -> Self {
        Self {
            turn: 0,
            ts: Utc::now(),
            event: WorkLogEvent::System,
            phase: Some(phase),
            tool_used: None,
            refers_to: vec![],
            body: message,
        }
    }

    /// Create an agent-initiated entry.
    pub fn agent(event: WorkLogEvent, turn: u64, body: String) -> Self {
        Self {
            turn,
            ts: Utc::now(),
            event,
            phase: None,
            tool_used: None,
            refers_to: vec![],
            body,
        }
    }

    /// Serialize this entry as a YAML frontmatter block + markdown text.
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        let mut map = serde_yaml::Map::new();
        map.insert("turn".into(), self.turn.into());
        map.insert("ts".into(), Self::format_ts(self.ts).into());
        map.insert("event".into(), serde_yaml::to_string(&self.event)?);
        if let Some(ref phase) = self.phase {
            map.insert("phase".into(), phase.clone().into());
        }
        if let Some(ref tool) = self.tool_used {
            map.insert("tool_used".into(), tool.clone().into());
        }
        if !self.refers_to.is_empty() {
            map.insert(
                "refers_to".into(),
                self.refers_to.iter().map(|s| s.clone().into()).collect::<Vec<_>>().into(),
            );
        }
        serde_yaml::to_string(&map)
    }

    fn format_ts(dt: chrono::DateTime<Utc>) -> String {
        dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    /// Format as an H2-annotated entry block (frontmatter + header + body).
    pub fn to_entry_block(&self) -> String {
        let event_str = match &self.event {
            WorkLogEvent::System => "✓ system".to_string(),
            WorkLogEvent::Draft => "📄 draft".to_string(),
            WorkLogEvent::Update => "📝 update".to_string(),
            WorkLogEvent::Note => "💡 note".to_string(),
            WorkLogEvent::Milestone => "⚑ milestone".to_string(),
        };

        // YAML frontmatter.
        let yaml = self.to_yaml().unwrap_or_else(|_| {
            format!("turn: {}\nts: \"{}\"\nevent: unknown", self.turn, Self::format_ts(self.ts))
        });

        // Build timestamp ref line for the header.
        let ts_ref = format!("{}", Self::format_ts(self.ts));
        let ref_line = if !self.refers_to.is_empty() {
            format!(" → ref: {}", self.refers_to.join(", "))
        } else {
            String::new()
        };

        format!(
            "{}\n## {} (turn {}, {}{})\n\n{}\n",
            yaml, event_str, self.turn, ts_ref, ref_line, self.body.trim()
        )
    }
}

// ===========================================================================
// Read / write helpers for the work-log file
// ===========================================================================

/// Read an existing work-log file and parse its entries.
pub fn read_worklog(worklog_path: &Path) -> Result<Vec<WorkLogEntry>, anyhow::Error> {
    if !worklog_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(worklog_path)?;
    let entries = parse_entries_from_file(&content);
    Ok(entries)
}

/// Count the number of work-log entries without loading them all.
pub fn count_entries(worklog_path: &Path) -> Result<usize, anyhow::Error> {
    if !worklog_path.exists() {
        return Ok(0);
    }
    let content = std::fs::read_to_string(worklog_path)?;
    Ok(content.matches("## ").count())
}

/// Append a single entry to the work-log file.
pub fn append_entry(worklog_path: &Path, entry: &WorkLogEntry) -> Result<(), anyhow::Error> {
    if let Some(parent) = worklog_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let block = entry.to_entry_block();

    if worklog_path.exists() && std::fs::metadata(worklog_path)?.len() > 0 {
        // Append to existing file.
        let mut file = std::fs::OpenOptions::new().create(true).append(true).open(worklog_path)?;
        writeln!(file, "{}", block)?;
    } else {
        // Create new file — prepend a comment header + first entry.
        let content = format!("# Work log for ticket — auto-created\n{}", block);
        std::fs::write(worklog_path, content)?;
    }

    Ok(())
}

/// Append a system event entry (auto-append from operator transition gate).
pub fn append_system_event(
    worklog_path: &Path,
    phase: String,
    message: impl Into<String>,
) -> Result<(), anyhow::Error> {
    let entry = WorkLogEntry::system(phase, message.into());
    append_entry(worklog_path, &entry)
}

// ===========================================================================
// File parsing — split worklog file into individual entry YAML blocks.
// ===========================================================================

fn parse_entries_from_file(content: &str) -> Vec<WorkLogEntry> {
    let mut entries = Vec::new();
    // Split by "## " (H2 header). Each chunk becomes one candidate entry.
    for chunk in content.split("## ") {
        let trimmed = chunk.trim();
        if trimmed.is_empty() || trimmed.starts_with("Work log") {
            // Skip the file header comment "# Work log...".
            if trimmed.starts_with("Work log") { continue; }
            continue;
        }

        // Try to parse as YAML frontmatter + body. The first thing in each block
        // should be YAML (between ---), followed by markdown text.
        if let Some(parse_result) = try_parse_entry_yaml(trimmed) {
            entries.push(parse_result);
        }
    }
    entries
}

fn try_parse_entry_yaml(block: &str) -> Option<WorkLogEntry> {
    // Split on the first "---" pair that delimits YAML frontmatter.
    let parts: Vec<&str> = block.splitn(3, "---").collect();
    if parts.len() < 3 || parts[0].trim().starts_with("##") {
        return None;
    }

    // The body is everything after the second --- (excluding the H2 header).
    let yaml_raw = parts[1].trim();
    let rest = parts[2];

    // Parse YAML into serde value.
    let val: serde_yaml::Value = match serde_yaml::from_str(yaml_raw) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let map = if let serde_yaml::Value::Mapping(m) = val { m } else { return None; };

    // Extract turn (always u64).
    let turn = map.get(&serde_yaml::Value::String("turn".into()))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    // Extract ts from ISO string.
    let ts = map.get(&serde_yaml::Value::String("ts".into()))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                strptime(s, "%Y-%m-%dT%H:%M:%SZ").ok()
            } else { None }
        })
        .unwrap_or_else(Utc::now);

    // Extract event type.
    let event = map.get(&serde_yaml::Value::String("event".into()))
        .and_then(|v| {
            if let Some(s) = v.as_str() {
                serde_yaml::from_value(serde_yaml::Value::String(s.to_string())).ok()
            } else { None }
        })
        .unwrap_or(WorkLogEvent::Note);

    // Extract optional fields.
    let phase = map.get(&serde_yaml::Value::String("phase".into()))
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let tool_used = map.get(&serde_yaml::Value::String("tool_used".into()))
        .and_then(|v| v.as_str().map(|s| s.to_string()));
    let refers_to = map.get(&serde_yaml::Value::String("refers_to".into()))
        .and_then(|v| {
            if let serde_yaml::Value::Sequence(seq) = v {
                Some(seq.into_iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect())
            } else { None }
        })
        .unwrap_or_default();

    // Body: everything after the "---" delimiters, after skipping any leading H2/header.
    let body_lines: Vec<&str> = rest.lines()
        .skip_while(|l| l.starts_with("##") || l.trim().is_empty())
        .collect();
    let body = body_lines.join("\n").trim().to_string();

    Some(WorkLogEntry { turn, ts, event, phase, tool_used, refers_to, body })
}

fn strptime(s: &str, fmt: &str) -> Option<chrono::DateTime<Utc>> {
    chrono::DateTime::parse_from_str(s, fmt).ok().map(|dt| dt.into())
}

// ===========================================================================
// Ticket binding enforcement
// ===========================================================================

/// Validate that the agent can write to this work-log path based on its ticket.
/// 
/// The operator layer calls this before allowing the `work_log` tool to
/// execute, or before auto-appending at a phase transition. It extracts the
/// ticket number from the work-log filename and compares against the agent's
/// assigned ticket.
/// 
/// Returns `Ok(worklog_path)` if binding is valid, or an error on failure.
pub fn bind_ticket(
    worklog_path: &Path,
    agent_ticket_id: &str,
) -> Result<PathBuf, WorkLogBindingError> {
    let filename = worklog_path.file_name()
        .ok_or_else(|| WorkLogBindingError::NoFilename)?
        .to_string_lossy()
        .to_string();

    let expected_num = extract_ticket_number(agent_ticket_id)
        .ok_or_else(|| WorkLogBindingError::InvalidTicketId(agent_ticket_id.to_string()))?;

    let actual_num = filename_to_ticket_number(&filename)
        .ok_or_else(|| WorkLogBindingError::UnparseableFilename(filename.clone()))?;

    if expected_num != actual_num {
        return Err(WorkLogBindingError::TicketMismatch {
            expected: expected_num,
            expected_id: agent_ticket_id.to_string(),
            got: actual_num,
            worklog_filename: filename,
        });
    }

    Ok(worklog_path.to_path_buf())
}

fn extract_ticket_number(ticket_id: &str) -> Option<u64> {
    ticket_id.split('-').next().and_then(|s| s.parse::<u64>().ok())
}

fn filename_to_ticket_number(filename: &str) -> Option<u64> {
    let stem = filename.strip_suffix(".md")
        .or_else(|| filename.strip_suffix(".txt"))
        .unwrap_or(filename);
    stem.split('-').next().and_then(|s| s.parse::<u64>().ok())
}

// ===========================================================================
// Error types for binding failures
// ===========================================================================

/// Error returned when work-log ticket binding validation fails.
#[derive(Debug, Clone)]
pub enum WorkLogBindingError {
    NoFilename,
    InvalidTicketId(String),
    UnparseableFilename(String),
    TicketMismatch { expected: u64, expected_id: String, got: u64, worklog_filename: String },
}

impl std::fmt::Display for WorkLogBindingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoFilename => write!(f, "work-log path has no filename"),
            Self::InvalidTicketId(id) => write!(f, "invalid ticket ID format: {}", id),
            Self::UnparseableFilename(name) => write!(f, "cannot extract ticket number from filename: {}", name),
            Self::TicketMismatch { expected, got, worklog_filename, .. } => {
                write!(f, "ticket binding mismatch: agent '{}' (#{}) attempted to write to '{}'",
                    expected, expected, worklog_filename)
            }
        }
    }
}

impl std::error::Error for WorkLogBindingError {}

// ===========================================================================
// Default work-log path generation for a given ticket number
// ===========================================================================

/// Generate the expected work-log path for an execution ticket.
pub fn generate_worklog_path(executions_dir: &Path, ticket_number: u64) -> PathBuf {
    executions_dir.join(format!("{:03}-worklog.md", ticket_number))
}

// ===========================================================================
// Agent-initiated work_log tool entry point
// ===========================================================================

/// Handles an agent-initiated `work_log` tool call.
pub fn handle_work_log_tool(
    worklog_path: &Path,
    agent_ticket_id: &str,
    event_type_str: &str,
    turn: u64,
    body: &str,
) -> std::result::Result<String, WorkLogBindingError> {
    // 1. Validate ticket binding first.
    bind_ticket(worklog_path, agent_ticket_id)?;

    let event: WorkLogEvent = match event_type_str.to_lowercase().as_str() {
        "draft" => WorkLogEvent::Draft,
        "update" => WorkLogEvent::Update,
        "note" => WorkLogEvent::Note,
        "milestone" => WorkLogEvent::Milestone,
        other => {
            eprintln!("warning: unknown work_log event type '{other}', defaulting to note");
            WorkLogEvent::Note
        }
    };

    let entry = WorkLogEntry::agent(event.clone(), turn, body.to_string());

    // Append to file (creates if necessary).
    append_entry(worklog_path, &entry).map_err(|e| {
        WorkLogBindingError::InvalidTicketId(format!("write failed: {}", e))
    })?;

    let status_msg = match event {
        WorkLogEvent::System => "system event recorded",
        WorkLogEvent::Draft => "draft recorded",
        WorkLogEvent::Update => "update recorded",
        WorkLogEvent::Note => "note recorded",
        WorkLogEvent::Milestone => "milestone recorded",
    };

    Ok(format!("work_log: {} [turn {}]", status_msg, turn))
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entry(event: WorkLogEvent, body: &str) -> WorkLogEntry {
        WorkLogEntry::agent(event, 42, body.to_string())
    }

    #[test]
    fn test_ticket_number_extraction() {
        assert_eq!(extract_ticket_number("044-execution"), Some(44u64));
        assert_eq!(extract_ticket_number("1-exec"), Some(1u64));
        assert_eq!(extract_ticket_number(""), None);
        assert_eq!(extract_ticket_number("no-num"), None);
    }

    #[test]
    fn test_filename_to_ticket_number() {
        assert_eq!(filename_to_ticket_number("044-worklog.md"), Some(44u64));
        assert_eq!(filename_to_ticket_number("1-worklog.txt"), Some(1u64));
        assert_eq!(filename_to_ticket_number("nobody.yaml"), None);
    }

    #[test]
    fn test_bind_ticket_valid() {
        let worklog_path = PathBuf::from("/tmp/044-worklog.md");
        let result = bind_ticket(&worklog_path, "044-execution").unwrap();
        assert_eq!(result, worklog_path);
    }

    #[test]
    fn test_bind_ticket_mismatch() {
        let worklog_path = PathBuf::from("/tmp/045-worklog.md");
        let result = bind_ticket(&worklog_path, "044-execution");
        assert!(result.is_err());
        match result.unwrap_err() {
            WorkLogBindingError::TicketMismatch { expected, got, .. } => {
                assert_eq!(expected, 44);
                assert_eq!(got, 45);
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[test]
    fn test_generate_worklog_path() {
        let dir = PathBuf::from(".bureau/executions");
        assert_eq!(generate_worklog_path(&dir, 44u64), dir.join("044-worklog.md"));
    }

    #[test]
    fn test_entry_roundtrip() {
        let entry = test_entry(WorkLogEvent::Milestone, "Completed the config parser module.");
        let block = entry.to_entry_block();

        // Parse back.
        let entries = parse_entries_from_file(&block);
        assert_eq!(entries.len(), 1);
        let parsed = &entries[0];
        assert_eq!(parsed.event, WorkLogEvent::Milestone);
        assert_eq!(parsed.turn, 42);
        assert!(parsed.body.contains("Completed the config parser module"));
    }

    #[test]
    fn test_entry_yaml_only() {
        let entry = test_entry(WorkLogEvent::Note, "Some note text.");
        let yaml = entry.to_yaml().unwrap();
        // Should contain our key fields.
        assert!(yaml.contains("turn"));
        assert!(yaml.contains("note"));
    }

    #[test]
    fn test_multi_entry_file_parsing() {
        // Simulate a two-entry file content.
        let e1 = WorkLogEntry::system("execution-draft".into(), "Phase: draft.".to_string());
        let e2 = WorkLogEntry::agent(WorkLogEvent::Milestone, 5, "Done.".to_string());
        let content = format!(
            "# work log\n{}\n{}",
            e1.to_entry_block(), e2.to_entry_block()
        );

        let entries = parse_entries_from_file(&content);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].event, WorkLogEvent::System);
        assert_eq!(entries[1].event, WorkLogEvent::Milestone);
    }
}
