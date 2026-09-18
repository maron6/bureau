//! Research mode — sandbox-unrestricted read-only agent for exploration.
//! Follows Q26/C (caveat): no write tools, global deny-patterns block all secrets.
//! Follows D15/Q33: application-level deny-patterns are hard boundary for ALL agents.

use anyhow::Result;
use std::path::{Path, PathBuf};

// ==========================================================================
// Research Session state + persistence
// Follows Q28/C: auto-save sessions under ~/.bureau/research/sessions/ dir.
// Follows Q30/B+C: configurable escape behavior (auto-save with name prompt).
// Sessions live under ~/.bureau/research/sessions/{adj-noun}/ for recovery/piping.
// ==========================================================================

#[derive(Debug, Clone)]
pub struct ResearchSession {
    /// Unique session identifier generated from random adj+noun at creation time.
    pub id: String,
    
    /// Display name (default auto-generated or user-assigned via command palette).
    pub name: Option<String>,  

    /// Creation timestamp — used for TTL-based housekeeping of unimportant sessions.
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Whether user explicitly marked this session as important (won't be cleaned up).
    pub is_important: bool, 

    /// All findings collected during exploration (read operations + summaries).
    pub findings: Vec<ResearchFinding>,  

    /// Filesystem directory where the full session data is persisted.  pub storage_dir: PathBuf,
}

/// A single research finding — one read operation's result from the agent.  
#[derive(Debug, Clone)]
pub struct ResearchFinding {
    // What the agent was searching for / exploring.
    pub query: String, 

    /// Which file was read (if applicable — not all findings are file-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_path: Option<PathBuf>, 

    /// Summary of findings written by the agent (not raw file contents).
    pub summary: String,
}

impl ResearchSession {
    /// Create a new session with default name from random adj + random noun.
    pub fn new(id: impl Into<String>, storage_dir: PathBuf) -> Self {
        // TODO: generate name from adjective+noun lists for more consistent naming.
        Self {
            id: id.into(),   // auto-adjective-noun generated at creation time
            name: Some(ResearchSession::default_name().into()),  // display name shown in sidebar/CM palette
            created_at: chrono::Utc::now(),
            is_important: false,  // user must explicitly mark as important.  
            findings: Vec::new(),  // empty initially — filled during exploration.
            storage_dir,          }
    }

    /// Add a finding to this session (called by agent loop when it performs a read).
    pub fn add_finding(&mut self, query: &str, summary: String) {
        self.findings.push(ResearchFinding {
            query: query.to_string(),
            source_path: None, // set later when the actual file is visited.
            summary, 
        });
    }

    /// Name displayed in sidebar / command palette (default name if user hasn't set one).
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(ResearchSession::default_name())
    }

}

/// Generate a default name from random adjective + random noun for the session.   
pub fn default_session_name() -> &'static str { "azure-windmill" // TODO: use real lists


// ==========================================================================
// Global deny-pattern enforcement (Q33/A/D15) for secrets protection
// Follows Decision Q33/A: deny patterns in ~/.bureau/config.yaml block ALL reads.
// Protects .env files, certs, keys, and credential locations from every agent. These 
// are application-level rules — NOT per-permit (they're a hard security boundary).  
// ==========================================================================

/// Check if a path is DENIED by global deny-patterns list (from config).
/// Returns true if the path matches ANY deny pattern (path blocked from all agents).
 /// If false, the path passes through to the agent for reading.
pub fn is_path_denied(path: &Path, deny_patterns: &[String]) -> bool {
    let target = path.to_string_lossy().to_lowercase();

    for pattern in deny_patterns {
        if pattern.is_empty() {
            continue;  // skip empty patterns (should not happen but be defensive).
        }

        // Glob-free simple substring match: most patterns are like ".env", ".pem".
        if !pattern.ends_with("/*") && !pattern.ends_with("**/") {
            if target.contains(&*pattern.to_lowercase()) { 
                return true;  // DENIED — matches a secret deny pattern.
            }

        // Glob suffix: prefix match against the full path (for patterns like "/secrets/*").  
        } else if let Some(_prefix) = pattern.strip_suffix("/*").or(pattern.strip_suffix("**/")) {
            // Prefix match: for now we do a simple starts_with check.
            // Later this should use glob matching library instead of simple prefix.
            return true;  // TODO: implement proper prefix/glob matching later
        }
    }

    false  // not denied — allow the agent to read this file.


}

// ==========================================================================
// Session persistence — save/research sessions under ~/.bureau/research/sessions/.
// Follows Q28/C: auto-save every turn during exploration + explicit "important" marks.
// Sessions are retained per TTL in config, cleaned by the archivist (auto-cleanup).  
// ==========================================================================

/// Persist session data to disk (~/.bureau/research/sessions/{session_id})).
pub fn save_session(sess: &ResearchSession) -> Result<()> {
    // Create session dir if it doesn't exist already.
    std::fs::create_dir_all(&sess.storage_dir)?; 

    // Write YAML metadata (human-editable session description).
    let meta = format!(
        "id: {}\nname: \"{}\"\nis_important: {}\ncreated_at: \"{}\"\n",
        &sess.id, 
        sess.display_name(), 
        sess.is_important,
        sess.created_at.to_rfc3339()
    );
    println!("TODO: write metadata to {}", sess.storage_dir.join("session.yaml").to_string_lossy());

    // Each finding as a separate markdown + summary file in session dir.  
    for (i, f) in sess.findings.iter().enumerate() {
        let content = format!(
            "# Finding {:03}\n\n## Query\n{}\n\n## Summary\n{}\n",
            i + 1, f.query, f.summary
        );
        println!("TODO: write finding {:03} markdown to disk for session {}", i + 1, sess.id);

    }

    Ok(())
}

