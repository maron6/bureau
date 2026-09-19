//! Run Configuration Module (P2) — office-specific launch profiles.  
//! Follows D20: user-defined reusable workflows like VS Code launch.json convenience layer.
//! Archivist proposes configs but does NOT auto-commit them (advisory-only per Q40/F).

use crate::config::Mode;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ============================================================================  
// Archivist proposal types — D9 archivist validation output
// ============================================================================

/// When the archivist runs post-completion audit (D9), it can propose reusable run configs 
/// tied to specific permits. This is advisory only; the user decides whether to apply them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchivistProposal {
    pub permit_id: String,
   #[serde(default)]
    pub proposed_runs: Vec<ArchivistRun>,
}

/// A single run config suggestion from archivist proposal (advisory only).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchivistRun {    
    pub name: String,
    #[serde(rename = "mode")]  
    pub mode: Mode,
   #[serde(default)]
   pub description: Option<String>,
   #[serde(default)]
    pub skip_inspection: bool,     
}

// ============================================================================ 
// Run Config — user-defined launch profile per D20/Q40/F (like VS Code's launch.json)
// Properties defined in D20/Q40/D/E/F specifications.
// ============================================================================ 

/// A run configuration is a user-managed reusable workflow shortcut for starting permits.  
/// Stored in `.bureau/run_configs.yaml` under each office root.  
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunConfig {  
   /// Display name used in command palette and dashboard. From D20/Q40/D display requirement.
    pub name: String,
    
   /// Target mode: execution, inspection, interview, or archival (maps to D3 lifecycle phases).  
   #[serde(rename = "mode")]
    pub run_mode: Mode,
    
    // Which permit this config applies to (null = all permits; specific ID like "045" for targeted runs)  .
   #[serde(skip_serializing_if = "Option::is_none", default)]  
    pub permit_filter: Option<String>,
   
 // Optional per-permit scope overrides for testing or audit purposes (Q40/E feature).
 /// TODO: wire this to sandbox::Scope from D21 ticket-bound read scope when implementing that module.
 #[serde(skip_serializing_if = "Option::is_none", default)]
    pub override_scope: String,  

  // Skip-inspection UI toggle flag per command palette launch (Q40/A).  
   /// When true, skips the inspector barrier immediately after execution completes. Default is false. 
   #[serde(default = "default_false")]
  pub skip_inspection: bool,
   
   // Human-readable explanation shown in command palette hover text (Q40/C description field).
    #[serde(skip_serializing_if = "Option::is_none", default)]  
    pub description: Option<String>,
}

fn default_false() -> bool { false }

// ============================================================================ 
/// RunConfigs container that holds defaults + per-office config store for reuse patterns (D20 spec).  
/// Storage file is `.bureau/run_configs.yaml` at the office home root parallel to existing .bureau/config.yaml.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunConfigs {   
   /// Defaults when opening office without specific config selection applied yet (Q40/A default mode/provider).  
  #[serde(default)]
    defaults: Defaults,
    
  /// User-defined reusable workflow shortcuts in this office. May be empty list = no configs defined. 
   #[serde(default)]
    pub configs: Vec<RunConfig>,
}

// ============================================================================  
// RunDefault settings per D20/Q40/A/B — used as initial values for new configs user creates via palette.
// Similar concept to launch.json defaults VS Code offers out of the box per project workspace.
// ============================================================================  

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Defaults {   /// Primary mode applied first time when opening offices without explicit config chosen by hand. 
    #[serde(default = "default_execution_mode")]  
    pub default: Mode,
    
   /// Preferred LLM provider to use during agent loops (maps into OfficeConfig.providers keys later).
   #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,  // maps from office config providers list names 
}

fn default_execution_mode() -> Mode { Mode::Execution }

// ============================================================================  
// Helper fn for proposing new reusable patterns after archivist D9 audit completes (P5).  
/// Suggested configs are advisory-only — the user may review and optionally merge them in manually per D20 Q40/F spec.
pub fn propose_run_configs(permit: &str) -> anyhow::Result<ArchivistProposal> {  
   let proposal = ArchivistProposal {   
      permit_id: permit.to_string(),
       proposed_runs: vec![    
           ArchivistRun {
             name: format!("audit-complete-{}", permit),  // naming convention per D20/Q40/D pattern rules
              mode: Mode::Inspection,   // default to inspect since archivist audit comes after execution cycle completion  
              description: Some(format!(  
                 "Auto-generated based on completed work in {}\\nPermits processed during archival phase.", 
                 permit.to_string() )),
                skip_inspection: false,  // dont skip self-inspection; let archivists check own quality gate passes correctly
            },
        ],
    };
    
    Ok(proposal)  // return constructed proposal from archivist for user review and potential adoption into RunConfigs::configs vec
}

// ============================================================================
// Storage helpers (file path under office root per D20/Q40)  
// ============================================================================ 

/// Path where run config data gets persisted: `.bureau/run_configs.yaml` under each office home directory.
pub fn storage_path(office_home: &Path) -> PathBuf {
    office_home.join(".bureau").join("run_configs.yaml")
}

// ============================================================================
// RunConfigs load from / save to storage file (YAML serialization per D20 storage spec).  
/// Load returns defaults with empty configs list if no storage file exists yet in location returned by storage_path(office_home) fn.

pub fn load(office_home: &Path) -> anyhow::Result<RunConfigs> {  // uses anyhow error type consistent with other loader helpers like load_office_config() above in config module 
    let path = storage_path(office_home);
    
    if !path.exists() {  
        return Ok(RunConfigs::default());  // defaults apply; no configs file defined yet at expected location
        
    }
        
    let content = std::fs::read_to_string(&path)?;   
    serde_yaml::from_str(&content).map_err(|e| anyhow!("Failed to parse {}: {}", path.display(), e))
}

impl RunConfigs {
    /// Filter run configs by provided mode value. Returns only those matching the specified enum variant.
    pub fn by_mode(&self, mode: Mode) -> Vec<&RunConfig> {
        self.configs.iter().filter(|c| c.run_mode == mode).collect()
    }

    /// Insert user-created config into configs list; caller decides persistence strategy.
    pub fn save(&self, office_home: &Path) -> Result<(), anyhow::Error> {
        let path = storage_path(office_home);
        let dir = path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(dir)?;

        let content = serde_yaml::to_string(self)
            .map_err(|e| anyhow!("Unable to serialize run configs: {e}"))?;

        std::fs::write(&path, content)
            .map_err(|e| anyhow!("Failed to save {} (io): {e}", path.display()))?;
        Ok(())
    }
}

// ==========================================================================
// P9: Unit tests for run configuration module
// ==========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_run_config_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = RunConfigs::default();
        
        // Add a run config.
        let rc = RunConfig {
            name: "test-config".to_string(),
            run_mode: Mode::Execution,
            description: Some("A test config".into()),
            skip_inspection: true,
            permit_filter: Some("p001".into()),
            ..Default::default()
        };
        config.configs.push(rc);
        
        // Save and reload.
        config.save(dir.path()).unwrap();
        let loaded = load(dir.path()).unwrap();
        
        assert_eq!(loaded.configs.len(), 1);
        assert_eq!(loaded.configs[0].name, "test-config");
        assert_eq!(loaded.configs[0].run_mode, Mode::Execution);
        assert_eq!(loaded.configs[0].skip_inspection, true);
    }

    #[test]
    fn test_by_mode_filter() {
        let configs = RunConfigs {
            defaults: Defaults::default(),
            configs: vec![
                RunConfig { run_mode: Mode::Execution, ..Default::default() },
                RunConfig { run_mode: Mode::Inspection, ..Default::default() },
                RunConfig { run_mode: Mode::Execution, ..Default::default() },
            ],
        };
        
        let execution = configs.by_mode(Mode::Execution);
        assert_eq!(execution.len(), 2);
        
        let inspection = configs.by_mode(Mode::Inspection);
        assert_eq!(inspection.len(), 1);
    }

    #[test]
    fn test_defaults_applied_when_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        // No run_configs.yaml file exists.
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.configs.len(), 0);
        assert_eq!(loaded.defaults.default, Mode::Execution);
    }
}
