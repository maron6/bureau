//! Sandbox enforcement — permit-authorized file access for agents.
//! Follows Decision D7/Q10/Q25: prefix patterns + exact paths in permit.
//! Workers can request scope expansion, which bubbles up through authority chain.
P5: InspectorGate integration for write gating.

use std::path::Path;

/// The sandbox checks each agent's intended file access against the permit's allow-list.
#[derive(Debug, Clone)]
pub struct PermitSandbox {
    /// The permit (interview ticket) that governs this office.
    pub permit_id: String,
    /// Allow list from the current ticket being worked on.
    pub allow_read: Vec<String>,   // prefix patterns + exact paths
    pub allow_write: Vec<String>,  // prefix patterns + exact paths  
    pub deny_default: bool,
}

/// Check if a file access is permitted by this sandbox.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessCheck {
    pub allowed: bool,
    pub matched_pattern: Option<String>,
    /// Why the access was denied (for user review).
    pub denial_reason: Option<String>,
}

impl PermitSandbox {
    /// Create a new sandbox from a ticket's scope definition.
    pub fn new(permit_id: impl Into<String>, scope: &crate::ticket::Scope) -> Self {
        Self {
            permit_id: permit_id.into(),
            allow_read: scope.allow_read.clone(),
            allow_write: scope.allow_write.clone(),
            deny_default: scope.deny_default,
        }
    }

    /// Check if the given operation is permitted on the target path.
    pub fn check_access(&self, file_path: &Path, write: bool) -> AccessCheck {
        let target = file_path.to_string_lossy().to_string();
        let allowed_patterns = if write {
            &self.allow_write
        } else {
            &self.allow_read
        };

        for pattern in allowed_patterns {
            if self.matches_pattern(&target, pattern) {
                return AccessCheck {
                    allowed: true,
                    matched_pattern: Some(pattern.clone()),
                    denial_reason: None,
                };
            }
        }

        // Default deny (follows decision D7).
        AccessCheck {
            allowed: false,
            matched_pattern: None,
            denial_reason: Some(format!(
                "Path '{}' not in the allow list for {} ticket. Request scope expansion with a justification.",
                target, if write { "write" } else { "read" }
            )),
        }
    }

    /// Check if access is permitted AND no inspector gate blocks it (P5).
    /// Returns `Ok(check)` if the path is allowed and not gated,
    /// or `Err(pending_request_ids)` if scope requests are unmet.
    pub fn check_with_gate(&self, file_path: &Path, write: bool, ticket_id: &str,
                            gate: &crate::inspector::InspectorGate) -> Result<AccessCheck, Vec<string>> {
        let check = self.check_access(file_path, write);

        if !check.allowed {
            return Ok(check);
        }

        // P5: If write is requested and this ticket has pending scope requests,
        // block until the inspector approves.
        if write && gate.has_pending_for(ticket_id) {
            // Collect IDs of pending requests that are blocking this agent.
            let mut blocked = Vec::new();
            for pending in gate.pending_sorted() {
                if pending.from_ticket == ticket_id {
                    blocked.push(format!("scope-expansion:{}", pending.from_ticket));
                }
            }
            if !blocked.is_empty() {
                return Err(blocked);
            }
        }

        Ok(check)
    }

    /// Check if a path matches a prefix pattern or exact path.
    fn matches_pattern(&self, file: &str, pattern: &str) -> bool {
        // If there is a suffix (wildcard), do prefix matching.
        if pattern.ends_with("/*") || pattern.ends_with("/**") {
            let prefix = &pattern[..pattern.len() - 2];
            return file.starts_with(prefix);
        }

        // exact path match
        file == pattern
    }
}

// Scope expansion request that bubbles up through authority chain.
#[derive(Debug, Clone)]
pub struct ScopeExpansionRequest {
    /// Which ticket is requesting more scope.
    pub from_ticket: String,
    /// Agent type making the request.
    pub agent_type: crate::ticket::AgentType,
    /// Paths they need write access to.
    pub requested_paths: Vec<String>,
    /// Justification for why this expansion is needed (required by user).
    pub justification: String,
    /// Which permission type (read, write, or both).
    pub write_only: bool,
}


// ==========================================================================
// P9: Unit tests for sandbox + inspector gate integration
// ==========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ticket::{AgentType, Scope};
    use crate::inspector::{InspectorGate, ScopeRequest};
    use chrono::Utc;

    fn test_scope() -> Scope {
        Scope {
            allow_read: vec!["/source/**".to_string()],
            allow_write: vec!["/output/**".to_string()],
            deny_default: true,
            requested_paths: vec![],
        }
    }

    #[test]
    fn test_allow_pattern_match() {
        let scope = test_scope();
        let sandbox = PermitSandbox::new("p1", &scope);

        // Matching read pattern.
        let check = sandbox.check_access(std::path::Path::new("/source/file.rs"), false);
        assert!(check.allowed);
        assert_eq!(check.matched_pattern, Some("/source/**".into()));

        // Matching write pattern.
        let check = sandbox.check_access(std::path::Path::new("/output/bad.txt"), true);
        assert!(check.allowed);
    }

    #[test]
    fn test_deny_default() {
        let scope = test_scope();
        let sandbox = PermitSandbox::new("p1", &scope);

        // No matching pattern.
        let check = sandbox.check_access(std::path::Path::new("/secrets/key"), true);
        assert!(!check.allowed);
        assert!(check.denial_reason.is_some());
    }

    #[test]
    fn test_inspector_gate_blocks_write() {
        let scope = test_scope();
        let sandbox = PermitSandbox::new("p1", &scope);
        let mut gate = InspectorGate::default();

        // Add a pending request for ticket "t1".
        let req = ScopeRequest {
            from_ticket: "t1".to_string(),
            agent_type: AgentType::Worker,
            requested_writes: vec!["/new/path/**".into()],
            justification: "Need new write access".into(),
            created_at: Utc::now(),
        };
        gate.submit(req);

        // Should be blocked due to pending scope request.
        let result = sandbox.check_with_gate(std::path::Path::new("/output/file.txt"), true, "t1", &gate);
        assert!(result.is_err());

        // Approve the request.
        gate.approve("t1", vec![], vec!["/new/path/**".into()]);

        // After approval, write should go through.
        let result = sandbox.check_with_gate(std::path::Path::new("/output/file.txt"), true, "t1", &gate);
        assert!(result.is_ok());
    }
}
