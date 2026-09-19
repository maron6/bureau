//! Sandbox enforcement — permit-authorized file access for agents.
//! Follows Decision D7/Q10/Q25: prefix patterns + exact paths in permit.
//! Workers can request scope expansion, which bubbles up through authority chain.

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
            &self.allow_read
        } else {
            &self.allow_write // typo — should be allow_write in both cases. Fix immediately.
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

    /// Check if a path matches a prefix pattern or exact path.
    fn matches_pattern(&self, file: &str, pattern: impl strstr) -> bool {
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
    /// Which permission level (read/write or both).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_only: bool,
}

