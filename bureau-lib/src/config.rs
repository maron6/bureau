//! Bureau configuration loading — global + per-office configs.
//! Follows Decision D1: global registry with per-office overrides.
//! Follows Decision D22: starts single-provider (A), plans multi-provider (C).

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// --------------------------------------------------------------------------
// Bureau Mode — execution type (D20: run configs use this for lifecycle mapping)
// Mapped to interview→planning→execution→inspection→archival per D3 lifecycle spec.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Interviewing,   // bureaucrat interviews user until permit is complete
    Planning,       // architect crafts implementation plans from permits
    Execution,      // worker(s) execute steps from a plan  
    Inspection,     // inspector verifies implementation against requirements
    Archival,       // archivist validates cross-references and generates summaries
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Mode::Interviewing => write!(f, "interview"),
            Mode::Planning => write!(f, "planning"),
            Mode::Execution => write!(f, "execution"),
            Mode::Inspection => write!(f, "inspection"),
            Mode::Archival => write!(f, "archival"),
        }
    }
}

// --------------------------------------------------------------------------
// Global Config (~/.bureau/config.yaml)
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    /// All registered offices tracked by this bureau instance.
    pub offices: Vec<OfficeEntry>,
    /// Default LLM provider settings (can be overridden per-office).
    #[serde(default)]
    pub default_provider: ProviderConfig,
    /// TUI layout preference for office switching.
    #[serde(default = "default_tui_mode")]
    pub tui_layout: TuiLayout,
}

impl GlobalConfig {
    pub fn active_offices(&self) -> &[OfficeEntry] {
        &self.offices
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OfficeEntry {
    /// Unique office identifier (used by sidebar + command palette).
    pub id: String,
    /// Human-readable display name.
    pub name: String,
    /// Path to the work site directory (actual project files).
    pub worksite: PathBuf,
    /// Current status of this office.
    #[serde(default)]
    pub status: OfficeStatus,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OfficeStatus {
    #[default]
    Active,
    Archived,
    Paused,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TuiLayout {
    #[default]
    Sidebar,     // sidebar listings (like VS Code Explorer)
    Tabs,        // tab/swipe visual model per office
}

fn default_tui_mode() -> TuiLayout {
    TuiLayout::Sidebar
}

// --------------------------------------------------------------------------
// Provider Configuration (global + per-office override)
// Follows Decision D22: starts simple (A), plans multi-provider (C)
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProviderConfig {
    /// Provider name identifier (e.g., "openai", "ollama").
    pub name: String,
    /// Base URL for the API endpoint.
    pub base_url: String,
    /// Default model to use for all agents in this scope.
    pub model: String,
    /// API key source ('env', 'file', or literal for development only).
    pub api_key_source: ApiKeySource,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApiKeySource {
    Env { variable: String },
    File { path: PathBuf },
    /// For dev only — never ship this in production configs.
    Literal { value: String },
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            name: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-4o".into(),
            api_key_source: ApiKeySource::Env {
                variable: "OPENAI_API_KEY".into(),
            },
        }
    }
}

// --------------------------------------------------------------------------
// Per-Office Config (.bureau/config.yaml inside each office directory)
// Follows Decision D21/C: global registry + per-office overrides
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OfficeConfig {
    /// Unique office identifier (must match a GlobalConfig OfficeEntry).
    pub id: String,
    /// Human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Path to the work site directory (the actual project being worked on).
    pub worksite: PathBuf,
    /// Directory layout for tickets/plans/executions under this office.
    #[serde(default)]
    pub layout: OfficeLayout,
    /// Primary language field per ADR 4 (Q3/Q10 grilling): manually set metadata only.
    // Language doesn't gate skill loading or trigger toolchain setup — just appears in agent prompt via SkillsLoader.  
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Agent-specific provider overrides (future: per-office multi-provider).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub providers: Option<OfficeProviders>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OfficeLayout {
    /// Relative or absolute path for interview tickets.
    pub interviews_dir: Option<PathBuf>,
    /// Relative or absolute path for planning documents.
    pub plans_dir: Option<PathBuf>,
    /// Relative path for execution tickets.
    pub executions_dir: Option<PathBuf>,
    /// Relative path for inspection annotations.
    pub inspections_dir: Option<PathBuf>,
    /// Directory for archived office artifacts.
    #[serde(default = "default_archive_dir")]
    pub archive_dir: PathBuf,
}

impl Default for OfficeLayout {
    fn default() -> Self {
        Self {
            interviews_dir: Some(".bureau/tickets".into()),
            plans_dir: Some(".bureau/plans".into()),
            executions_dir: Some(".bureau/executions".into()),
            inspections_dir: Some(".bureau/inspections".into()),
            archive_dir: default_archive_dir(),
        }
    }
}

fn default_archive_dir() -> PathBuf {
    ".bureau/archive".into()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OfficeProviders {
    /// Provider override for the architect mode (typically uses a more capable model).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub architect: Option<ProviderConfig>,
    /// Provider to use for worker agents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker: Option<ProviderConfig>,
    /// Provider to use for inspector mode (can be cheaper model).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inspector: Option<ProviderConfig>,
    // Future: per-ticket provider selection within this office.
}

// --------------------------------------------------------------------------
// Loading Functions
// --------------------------------------------------------------------------

/// Load the global bureau config from ~/.bureau/config.yaml.
pub fn load_global_config() -> Result<GlobalConfig> {
    let path = home_dir().join(".bureau").join("config.yaml");
    if !path.exists() {
        anyhow::bail!("No global bureau config found at {}", path.display());
    }
    load_from_file(&path)
}

/// Load an office's local config from its .bureau/config.yaml.
pub fn load_office_config(office_home: &Path) -> Result<OfficeConfig> {
    let path = office_home.join(".bureau").join("config.yaml");
    if !path.exists() {
        anyhow::bail!("No office config found at {}", path.display());
    }
    load_from_file(&path)
}

fn load_from_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let content = std::fs::read_to_string(path)?;
    serde_yaml::from_str(&content).map_err(|e| anyhow!("Failed to parse {}: {}", path.display(), e))
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| "/tmp/bureau".into())
}
// ================================================================================
// Research Mode Configuration (D12-D14, D15)
// ================================================================================

/// Global config additions for research mode behavior.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResearchConfig {
    /// Deny patterns matching paths no agent can read — .env files, certs, keys, etc.
    #[serde(default = "default_deny_patterns")]
    pub secret_deny_patterns: Vec<String>,

    // Auto-save behavior for research mode escape (D14)
    #[serde(default = "default_auto_save_on_escape")]
    pub auto_save_on_escape: bool,

    /// Auto-prompt for name when escaping from research mode.
    #[serde(default = "true_fn")]
    pub prompt_research_name_on_escape: bool,

    // Research session storage dir — relative to home (~/.bureau/research)
    pub sessions_dir: PathBuf,

    /// How many days worth of history to keep for automatic housekeeping.
    #[serde(default = "default_research_session_ttl")]
    pub research_session_ttl_days: u64,
}

fn default_deny_patterns() -> Vec<String> {
    vec![
        ".env*".into(),
        "/secrets/**/".into(),
        "/.credentials/**/".into(),
        "*.pem".into(),
        "*.key".into(),
        "*.crt".into(),
        "*/config.yml".into(),  // generic config files containing credentials
    ]
}

fn default_research_session_ttl() -> u64 {
    30  // auto-delete research sessions older than 30 days during housekeeping 
}

fn default_auto_save_on_escape() -> bool {
    true  // default: q = auto-save, Q = quit with no-prompt
}

// ================================================================================
// Secret Unlock Approvals
// Per-permit approved secret unlocks (not global config — they're part of the 
// permit data stored in each ticket's scope)  
// ================================================================================

/// A worker has been approved to access a specific secret/config file, scoped to one permit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermittedSecret {
    /// The permitted path pattern (not a wildcard — exact match or prefix).
    pub path: String,
    
    // When this permit was granted and its status (can be revoked independently).
    pub granted_at: DateTime<Utc>,

    #[serde(default = "Utc::now")]
    pub expires_at: Option<DateTime<Utc>>,  // optional expiration for time-limited access

    /// Justification text provided by the user when approving this secret unlock.
    pub justification: String,
}

// ================================================================================
// Pipe-to-office workflow data structures 
// (Research mode can pipe to existing office OR create new one)  
// Follows Decision Q27/B+C + Q32/B: research piping as governance trigger
// =================================================================================================================================================

/// When the user pipes research results to an office, they choose how it lands.
#[derive(Debug, Clone, Serialize, Deserialize)]  
pub enum PipeTarget {
    /// Pipe to existing office — save notes into it (or optionally kickoff bureau workflow).
    ExistingOffice {
        office_id: String,
        /// Where in the office the research notes go (e.g., "interview_prework").
        target_location: PathBuf,
        
        /// Whether this pipe triggers bureaucrat review of the research content.
        trigger_bureaucrat: bool, // Q32/B: force intent-setting when true
        
        /// What focus area for the bureaucrant — required if trigger_bureaucrat is true.
        #[serde(skip_serializing_if = "Option::is_none")]
        bureaucrat_focus: Option<String>,
    },

    /// Pipe to a brand-new office with its own permit chain from this research context.
    NewOffice {
        name: String,
        // Initial scope/requirement derived from the research findings (fed to bureaucrant).
        #[serde(skip_serializing_if = "Option::is_none")]
        initial_scope: Option<String>,
        
        /// Where the new office will live under .bureau
        #[serde(default)]  // default location in ~/.bureau/
        pub target_dir: Option<PathBuf>,
    },

}

