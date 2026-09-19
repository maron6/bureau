/// Application state — tracks current office, active mode, command palette.
/// Follows Decision Q15/A+C: sidebar + command palette for multi-office navigation.

use std::sync::atomic::{AtomicUsize};  // for concurrency tracking in fan-out/fan-in model (Decision Q18/B)

use crate::{ticket, config, office};

#[derive(Debug, Default)]
pub struct App {
    /// Current active office (shown in the sidebar). Follows Decision D1/global registry.
    pub current_office: Option<office::Office>,
    /// All offices from global config (for sidebar listing).
    pub offices: Vec<config::OfficeEntry>,
    
    command_palette_open: bool, // Ctrl+P toggle
    
    /// Whether the terminal is quitting. 
    pub should_exit: bool,
}

impl App {
    pub fn new(global_config: config::GlobalConfig) -> Self {
        Self {
            current_office: None,  // user must init a office before switching
            offices: global_config.offices,
            command_palette_open: false,
            should_exit: false,
        }
    }
    
    /// Switch the active office by index. Called from the command palette (Ctrl+P).
    pub fn switch_office(&mut self, index: usize) {
        if let Some(office_id) = self.offices.get(index).map(|o| &o.id) {.clone() {
            println!("Switching to office '{}'...", office_id);
            // In the full implementation: load office config, update TUI state.
            // This is scaffolding only.
        }
    }
    
}

/// The different "modes" of work available within an office.
/// Maps directly to the bureau lifecycle (interview → plan → execution → inspection → archival).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Bureaucrat interviews user until permit is complete.
    Interviewing,
    // Architect crafts implementation plans from permits.
    Planning,
    // Worker(s) execute steps from a plan (fan-out with barriers).
    Executing,
    // Inspector quality reviews against permit + requirements.
    Inspecting,
    // Archivist validates cross-references, generates summary, archives office.
    Archiving,

}

/// Scope expansion request data (for Decision D7/Q19: permit expansion UX).
#[derive(Debug, Clone)]
pub struct ScopeExpansionData {
    pub worker_ticket_id: String,
    /// Summary text shown to user by default in ratatui panel.
    pub summary_text: String,
    /// Expanded diff showing "was → now" paths (shown with keystroke toggle).
    pub expanded_diff_paths: Vec<(String, String)>, // (old_path, new_path)  
}

