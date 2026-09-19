//! Ticket types, schema, state machine definitions.
//! Follows Decision D2: YAML frontmatter + Markdown body.
//! Follows Decision D3: Lifecycle phases with type-specific states.
//! Follows decision A: named 001-execution.md convention with UUID in frontmatter.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// --------------------------------------------------------------------------
// Ticket Type Enum
// Follows D3: interview -> plan -> execution -> inspection -> archival
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TicketType {
    #[default]
    /// Bureaucrat's interview ticket - interviews user until permit is complete.
    Interview,
    /// Architect's plan document derived from a bureau work item (permit).
    Plan,
    /// Individual worker execution tickets under a plan.
    Execution,
    /// Inspector quality review annotations on execution tickets.
    Inspection,
    /// Archivist summary artifact and the office's own archiving pass.
    Archival,
}

impl std::fmt::Display for TicketType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Interview => write!(f, "interview"),
            Self::Plan => write!(f, "plan"),
            Self::Execution => write!(f, "execution"),
            Self::Inspection => write!(f, "inspection"),
            Self::Archival => write!(f, "archival"),
        }
    }

}

// --------------------------------------------------------------------------
// Agent Type (who created/approved)
// Follows D4: strict authority hierarchy
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    /// The human user interacting with the TUI.
    User,
    /// Bureaucrat interview agent - interviews user until permit is complete.
    Bureaucrat,
    /// Architect planning agent - crafts implementation plan from permits.
    Architect,
    /// Worker execution agent (multiple workers can run concurrently).
    Worker,
    /// Inspector quality review agent - validates against requirements/permits.
    Inspector,
    /// Archivist documentation agent - cross-reference audit + summary gen.
    Archivist,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Bureaucrat => write!(f, "bureaucrat"),
            Self::Architect => write!(f, "architect"),
            Self::Worker => write!(f, "worker"),
            Self::Inspector => write!(f, "inspector"),
            Self::Archivist => write!(f, "archivist"),
        }
    }

}

// --------------------------------------------------------------------------
// Ticket Status + Transitional States
// Follows Q11 (type-specific granularity) + D3/Lifecycle phases
// --------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TicketState {
    /// The current status of this ticket.
    pub status: Status,
    /// Type-specific transitional state (optional, for mode-specific behavior).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<Transition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Status {
    /// Initial creation - not yet shared for review.
    Draft,
    /// Submitted for user/agent review.
    InReview,
    /// Approved and binding on downstream work.
    Approved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InterviewTransition {
    /// Bureaucrat needs more information from the user.
    NeedsMoreInfo,
    /// Interviews complete, permit ready for architect review.
    PermitReady,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanTransition {
    PlanningStarted,
    ReviewRequested,
    ApprovedForWork,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionTransition {
    /// Worker has picked up the ticket.
    Assigned,
    /// Agent is actively working on it (can be interrupted by checkpoint).
    Working,
    /// Ready for inspector review.
    ReviewRequested,
    /// Passed inspection by the inspector agent.
    Inspected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InspectionTransition {
    /// Inspector flagged the execution ticket as non-compliant.
    Failed {
        /// Annotation-style reason string (see decision D8).
        reason: String,
        pub permit_section: Option<String>,
    },
    /// Inspector approved this work.
    Passed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]  
pub enum ArchivalTransition {
    /// Archivist has begun the archiving process.
    ArchivingStarted,
    /// Cross-reference audit phase complete.
    AuditComplete,
    /// Summary artifact generation in progress.
    GeneratingSummary,
    /// Office fully archived.
    Complete,
}

// --------------------------------------------------------------------------
// The Ticket Struct — YAML frontmatter schema (all fields from Q7)
// Follows Decision D2/YAML+Markdown schema
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    #[serde(default = "new_default_id")]
    pub id: String,
    
    /// Display ID in filename convention (e.g. "003" for 003-execution.md).
    /// Assigned by the office tracker to ensure monotonic ordering.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "display_id")]
    pub number: Option<u64>,

    #[serde(rename = "type")]
    pub ticket_type: TicketType,

    // Q7: status + type-specific transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TicketState>,

    /// Who created this ticket.
    #[serde(default)]
    pub created_by: AgentType,

    /// Who approved it (null until approved).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<AgentType>,

    /// Structured scope/allow-list definition for sandbox enforcement.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,

    /// Categorization tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Priority (1-5, 5 being highest).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u8>,

    /// Human-readable title/summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Which parent plan this execution.ticket belongs to (authority chain).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_plan: Option<String>,

    /// Office ID this ticket belongs to.
    #[serde(default)]
    pub office_id: String,

    /// Timestamp of last modification (for cross-reference audit).
    #[serde(with = "chrono::serde::ts_seconds", skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<DateTime<Utc>>,
}

// --------------------------------------------------------------------------
// Scope definition — the permit/allow-list for sandbox enforcement
// Follows Decision Q10/Q25: prefix patterns + exact paths in permit
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scope {
    /// Prefix patterns or exact paths this ticket can READ.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_read: Vec<String>,
    
    /// Prefix patterns or exact paths this ticket can WRITE to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow_write: Vec<String>,

    /// Whether to deny by default if not in allow lists.
    #[serde(default = "default_deny")]
    pub deny_default: bool,

    /// Paths this ticket NEEDS write access to but isn't yet authorized for.
    /// Used for scope expansion requests — bubbles up through authority chain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "@scope_request")]    // prefix with @ so agent recognizes it as metadata
    pub requested_paths: Vec<String>,
}

fn default_deny() -> bool {
    true
}

// --------------------------------------------------------------------------
// Agent Checkpoint Protocol (for D6/Launch/Resume)
// Follows Decision Q16/B + Decision A: JSONL history alongside checkpoint.
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCheckpoint {
    /// Which ticket this agent is working on.
    pub ticket_id: String,
    
    /// What type of agent this is (worker, inspector, etc.).
    pub agent_type: AgentType,
    
    /// Last tool calls the agent attempted (for partial work recovery).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_tool_calls: Option<Vec<ToolAction>>,

    /// Partial content saved so far - agent's draft state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub partial_content: Vec<String>,

    /// Snapshot of relevant context for resuming (permits/requiring).
    #[serde(skip_serialization_if = "Option::is_none")]
    pub context_snapshot: Option<TicketContextSnapshot>,

    /// Which turn this checkpoint is at.
    pub turn_count: u64,

    /// Hint for which action to try next (if agent failed mid-task).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_action_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAction {
    /// The tool name (e.g. "write", "read", "list").
    pub name: String,
    /// Arguments passed to the tool.
    pub args: serde_json::Value,
    /// Result returned from the tool call (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TicketContextSnapshot {
    /// Reference to the permit that governs this ticket.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permit_ref: Option<String>,   // UUID of parent permit
    
    pub scope_definitions: Vec<String>,

    /// Key section references the agent should pay attention to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub critical_sections: Vec<String>,
}

// --------------------------------------------------------------------------
// Ticket helpers for file naming (Decision Q23/A)
// --------------------------------------------------------------------------

impl Ticket {
    /// Create a new ticket with current timestamp and agent author.
    pub fn new(ticket_type: TicketType, created_by: AgentType, office_id: impl Into<String>) -> Self {
        let transition = Transition::for_ticket(ticket_type);

        // TODO: number should be assigned by the office tracker to ensure uniqueness/ordering
        let status = TicketState {
            status: Status::Draft,
            transition,
        };

        Self {
            id: new_default_id(),
            number: None,       // assigned at save time
            ticket_type,
            status: Some(status),
            created_by,
            approved_by: None,
            scope: None,
            tags: vec![],
            priority: None,
            title: None,
            parent_plan: None,
            office_id: office_id.into(),
            modified_at: Some(Utc::now()),
        }
    }

    /// Generate the filename for this ticket (e.g. "003-execution.md").
    pub fn filename(&self) -> String {
        let number = match self.number {
            Some(n) => n,
            None => {
                // Fallback if not yet assigned a number - use last 4 chars of ID
                format!("{:07}", self.id.len()) + "-untitled-" + &self.ticket_type.to_string()
            }
        };
        let type_label = self.transition_slug();
        format!("{:03}-{}.md", number, type_label)
    }

    /// The transition slug for this ticket (used in filenames during interview/plan phases).
    fn transition_slug(&self) -> String {
        match &self.status.as_ref().and_then(|s| s.transition.as_ref()) {
            Some(Transition::Interview(t)) => match t {
                InterviewTransition::NeedsMoreInfo => "interview-waiting".to_string(),
                InterviewTransition::PermitReady => "interview-ready".to_string(),
            },
            Some(Transition::Plan(t)) => match t {
                PlanTransition::PlanningStarted => "plan-draft".to_string(), 
                PlanTransition::ReviewRequested => "plan-review".to_string(),
                PlanTransition::ApprovedForWork => "plan-approved".to_string(),
            },
            _ => self.ticket_type.to_string(),
        }
    }

    /// Generate the checkpoint filename for this ticket.
    pub fn checkpoint_filename(&self) -> String {
        format!("{:03}.checkpoint.json", self.number.unwrap_or(0))
    }

    /// Generate the history log filename for this ticket.
    pub fn history_filename(&self) -> String {
        format!("{:03}.history.jsonl", self.number.unwrap_or(0))
    }
}

/// The unified transition enum - bridges typed transitions from the status field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum Transition {
    Interview(InterviewTransition),
    Plan(PlanTransition),
}

impl Transition {
    pub fn for_ticket(ticket_type: TicketType) -> Option<Self> {
        match ticket_type {
            TicketType::Interview => Some(Transition::Interview(InterviewTransition::NeedsMoreInfo)),
            TicketType::Plan => Some(Transition::Plan(PlanTransition::PlanningStarted)), 
            _ => None, // Execution/Inspection use their own types or are inline in the execution phase
        }
    }

    /// Get the human-readable slug for this transition (used in filenames).
    pub fn slug(&self) -> String {
        match self {
            Self::Interview(_) => "interview".into(),
            Self::Plan(_) => "plan".into(),  
        }
    }

}

/// Generate a new default ID string.
fn new_default_id() -> String {
    // TODO: replace with uuid crate when available (e.g., `uuid::Uuid::new_v4().to_string()`)
    format!("{:08x}", rand::random::<u32>())
}

