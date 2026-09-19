//! Inspector Permission System (P4) — per-D21/Q53/Q54.
//! 
//! Derived scope that flows from ticket relationships into sandbox enforcement
//! and inspector approval chain. An inspector agent needs authority to validate
//! a ticket's work against its permit, which requires access patterns beyond
//! what the worker was originally granted.

use crate::ticket::{AgentType, Scope, Ticket};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// TicketBoundScope — derives from ticket relationships and grant scope
// ============================================================================

/// A scope that extends worker permissions with inspector-specific access.
/// Derived from the ticket's permit chain: if an inspector is validating
/// ticket T under permit P, they get read access to everything in P's
/// allow_read plus their own write paths for annotations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketBoundScope {
    /// The ticket being inspected (this scope belongs to the inspection workflow).
    pub target_ticket: String,
    /// The permit that governs the original work (read-only reference).
    pub permit_id: String,
    /// Read paths inherited from the parent permit's allow list.
    pub permit_read_patterns: Vec<String>,
    /// Write paths for the inspector to annotate findings.
    #[serde(default)]
    pub inspector_write_paths: Vec<String>,
    /// Whether read is granted for inspecting source files (not just metadata).
    pub full_source_read: bool,

}

impl TicketBoundScope {
    /// Construct a ticket-bound scope from an existing ticket's permit chain.
    /// The inspector inherits the parent's read allow-list and gets annotation
    /// write access to the inspections directory.
    pub fn from_ticket(ticket: &Ticket) -> Self {
        let mut permit_read = vec![];
        let mut inspector_writes = vec![];

        if let Some(ref scope) = ticket.scope {
            // Inherit all read patterns from the parent permit/scope.
            permit_read.extend(scope.allow_read.iter().cloned());
            // Inspector write access to annotations directory (per D21/C).
            inspector_writes.push(".bureau/inspections".to_string());
        }

        Self {
            target_ticket: ticket.id.clone(),
            permit_id: ticket.office_id.clone(),
            permit_read_patterns: permit_read,
            inspector_write_paths: inspector_writes,
            full_source_read: false, // must be explicitly granted via approval
        }
    }

    /// Merge another scope's read patterns into this one (for multi-ticket reviews).
    pub fn merge_read(&mut self, other: &TicketBoundScope) {
        for pattern in &other.permit_read_patterns {
            if !self.permitted_read(pattern) {
                self.permit_read_patterns.push(pattern.clone());
            }
        }
    }

    /// Check if the inspector can read a given path under this scope.
    pub fn can_read(&self, path: &str) -> bool {
        self.permitted_read(path) || self.full_source_read
    }

    /// Check if the inspector can write to a given path under this scope.
    pub fn can_write(&self, path: &str) -> bool {
        self.inspector_write_paths.iter().any(|p| Self::path_matches(p, path))
    }

    fn permitted_read(&self, path: &str) -> bool {
        self.permitted_read_inner(path)
    }

    fn permitted_read_inner(&self, path: &str) -> bool {
        for pattern in &self.permitted_read_patterns {
            if Self::path_matches(pattern, path) {
                return true;
            }
        }
        false
    }

    /// Check if the inspector has been granted full source read access.
    pub fn has_full_source_read(&self) -> bool {
        self.full_source_read
    }

    fn path_matches(pattern: &str, path: &str) -> bool {
        if let Some(glob_pat) = pattern.strip_suffix("/**") {
            path.starts_with(glob_pat)
        } else if let Some(prefix) = pattern.strip_suffix("/*") {
            path.starts_with(prefix)
        } else {
            path == pattern
        }
    }
}

// ============================================================================
// PermissionAction — inspector decisions on scope requests
// ============================================================================

/// An inspector's decision on a worker's scope expansion request.
/// Per D21: inspectors gate scope changes; workers cannot self-expand.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionAction {
    /// Approve the scope update — grant the requested paths.
    Approve {
        /// The updated read patterns to add.
        new_read_patterns: Vec<String>,
            /// The updated write patterns to add.
        new_write_patterns: Vec<String>,
    },

    /// Deny with a reason shown to the requesting agent.
    Deny {
        /// Explanation for why this request was denied. Must be present per D21 audit trail.
        reason: String,
            /// Optional suggestion for what would make it approvable.
        suggestion: Option<String>,
    },

    /// Delegate to another role (e.g., inspector delegates architect review on a complex request).
    Delegate {
        /// The mode the other agent should operate in when reviewing.
        target_mode: crate::config::Mode,
            /// Instructions for the delegate.
        instructions: String,
    },
}

impl PermissionAction {
    /// Label shown in the TUI permission panel.
    pub fn label(&self) -> &str {
        match self {
            Self::Approve { .. } => "approve",  
            Self::Deny { .. } => "deny",
            Self::Delegate { .. } => "delegate",
        }
    }
}

// ============================================================================
// ScopeRequest — the request itself (bridges sandbox ↔ inspector)
// ============================================================================

/// A scope expansion request from a worker to an inspector for review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeRequest {
    /// The ticket requesting more scope.
    pub from_ticket: String,
    /// Agent type making the request (for audit trail).
    pub agent_type: AgentType,
    /// Paths needed for write access.
    #[serde(default)]
    pub requested_writes: Vec<String>,
    /// The justification required by the inspector's gate.
    pub justification: String,
    /// Timestamp when this request was created.
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// InspectorGate — manages the approval workflow for scope requests
// ============================================================================

/// Central registry of pending and resolved scope requests for an office.
#[derive(Debug, Clone)]
pub struct InspectorGate {
    /// Pending requests awaiting inspector review.
    pub pending: Vec<ScopeRequest>,
    /// Map from ticket_id → resolution timestamp for audit trail.
    pub resolved: HashMap<String, ResolvedRequest>,
}

impl Default for InspectorGate {
    fn default() -> Self {
        Self {
            pending: vec![],
            resolved: HashMap::new(),
        }
    }
}

/// Record of a resolved scope request.
#[derive(Debug, Clone)]
pub struct ResolvedRequest {
    pub from_ticket: String,
    pub action_label: String,
    pub resolved_at: DateTime<Utc>,
}

impl InspectorGate {
    /// Submit a scope expansion request to the review queue.
    pub fn submit(&mut self, req: ScopeRequest) {
        self.pending.push(req);
    }

    /// Approve a specific pending request. Returns true if found and processed.
    pub fn approve(
        &mut self,
        ticket_id: &str,
        read_patterns: Vec<String>,
        write_patterns: Vec<String>,
    ) -> bool {
        if let Some(pos) = self.pending.iter().position(|r| r.from_ticket == ticket_id) {
            let req = self.pending.remove(pos);
            self.resolved.insert(
                ticket_id.to_string(),
                ResolvedRequest {
                    from_ticket: ticket_id.to_string(),
                    action_label: "approve".into(),
                    resolved_at: Utc::now(),
                },
            );
            // Return the updated scope patterns the caller uses to update sandbox.
            return true;
        }
        false
    }

    /// Deny a specific pending request with a reason.
    pub fn deny(&mut self, ticket_id: &str, reason: String) -> bool {
        if let Some(pos) = self.pending.iter().position(|r| r.from_ticket == ticket_id) {
            let _req = self.pending.remove(pos);
            self.resolved.insert(
                ticket_id.to_string(),
                ResolvedRequest {
                    from_ticket: ticket_id.to_string(),
                    action_label: "deny".into(),
                    resolved_at: Utc::now(),
                },
            );
            return true;
        }
        false
    }

    /// Get all pending requests sorted by creation time (oldest first).
    pub fn pending_sorted(&self) -> Vec<&ScopeRequest> {
        let mut sorted = self.pending.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|r| r.created_at);
        sorted
    }

    /// Check if a specific ticket has any unresolved scope requests.
    pub fn has_pending_for(&self, ticket_id: &str) -> bool {
        self.pending.iter().any(|r| r.from_ticket == ticket_id)
    }
}
