//! Office management — create, switch, track.
//! Follows Decision D1 + D21/C: global registry with per-office overrides.

use anyhow::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Office {
    pub id: String,
    /// Human-readable name (shown in sidebar).
    pub name: String,
    /// Path to the office directory (.bureau is under here).
    pub home: PathBuf,
    /// Path to external worksite (actual project files being worked on).
    pub worksite: PathBuf,
    /// Local office config loaded from .bureau/config.yaml.
    #[serde(skip)]
    // Note: serde(skip) will need a manual Deserialize impl if we ever load from file
    pub config: crate::config::OfficeConfig,
    /// Which bureau mode is currently active for this office.
    pub status: OfficeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OfficeStatus {
    #[default]
    Idle,
    Interviewing,
    Planning,
    Executing,
    Inspecting,
    Archiving,
    // These map to the lifecycle phases. The sidebar shows these as status badges.
}

impl OfficeStatus {
    /// Display name for this status (used in work-log auto-append).
    pub fn display(&self) -> &'static str {
        match self {
            OfficeStatus::Idle => "idle",
            OfficeStatus::Interviewing => "interviewing",
            OfficeStatus::Planning => "planning",
            OfficeStatus::Executing => "executing",
            OfficeStatus::Inspecting => "inspecting",
            OfficeStatus::Archiving => "archiving",
        }
    }
}

impl Office {
    /// Create a new office from an existing directory.
    pub fn new(id: impl Into<String>, name: impl Into<String>, home: PathBuf) -> Result<Self> {
        let config = crate::config::load_office_config(&home)?;
        
        if !home.exists() {
            anyhow::bail!("Office directory does not exist: {}", home.display());
        }

        Ok(Self {
            id: id.into(),
            name: name.into(),
            home,
            worksite: config.worksite.clone(),
            config,
            status: OfficeStatus::default(),
        })
    }

    /// Path to the interview tickets dir (relative to office.home).
    pub fn interviews_dir(&self) -> PathBuf {
        let layout = self.config.layout.interviews_dir.as_ref()
            .unwrap_or(&PathBuf::from(".bureau/tickets"));
        self.home.join(layout)
    }

    /// Path to the plans directory.
    pub fn plans_dir(&self) -> PathBuf {
        let layout = self.config.layout.plans_dir.as_ref()
            .unwrap_or(&PathBuf::from(".bureau/plans"));
        self.home.join(layout)
    }

    /// Path to the executions directory.
    pub fn executions_dir(&self) -> PathBuf {
        let layout = self.config.layout.executions_dir.as_ref()
            .unwrap_or(&PathBuf::from(".bureau/executions"));
        self.home.join(layout)
    }

    /// Path to the inspections directory (where inspection annotations live).
    pub fn inspections_dir(&self) -> PathBuf {
        let layout = self.config.layout.inspections_dir.as_ref()
            .unwrap_or(&PathBuf::from(".bureau/inspections"));
        self.home.join(layout)
    }

    /// Full path to the archive directory (always under home).
    pub fn archive_dirs(&self) -> PathBuf {
        let layout = self.config.layout.archive_dir.as_ref()
            .unwrap_or(&PathBuf::from(".bureau/archive"));
        self.home.join(layout)
    }

    /// External worksite path — where the actual project lives.
    pub fn worksite_path(&self) -> &std::path::Path {
        &self.worksite
    }

    // ----------------------------------------------------------------------
    // P3: Auto-append gate integration
    // ----------------------------------------------------------------------

    /// Transition this office to a new status, auto-logging the work-log entry.
    /// 
    /// This is the phase-transition gate that ensures every mode change
    /// produces an auto-appended work-log system entry for traceability.
    /// The append is best-effort (non-fatal) so it never blocks valid
    /// transitions even if the work-log file cannot be written.
    pub fn transition_ticket(
        &mut self,
        old_status: OfficeStatus,
        new_status: OfficeStatus,
        ticket_num: u64,
    ) -> Result<()> {
        // Record the phase transition in the work-log before updating state.
        let msg = format!(
            "status change: {} → {}",
            old_status.display(),
            new_status.display()
        );
        
        // Best-effort append — log but don't block the transition.
        let wl_path = self.executions_dir()
            .join(format!("{:03}-worklog.md", ticket_num));
        let _ = crate::worklog::append_system_event(
            &wl_path,
            new_status.display().to_string(),
            msg,
        );
        
        // Update office status after logging.
        self.status = new_status;

        Ok(())
    }
}
