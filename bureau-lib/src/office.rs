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
            worksite: config.workste.clone(),  // use office config's worksite path
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
}

