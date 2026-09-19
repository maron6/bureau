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
    
    // ------------------------------------------------------------------
    // P2: Work-log overlay state
    // ------------------------------------------------------------------
    
    /// Index of the ticket row currently showing a work-log overlay (None = hidden).
    pub active_worklog_ticket: Option<u64>,
    /// Pre-parsed work-log entries for the active overlay.
    pub worklog_entries: Vec<bureau_lib::WorkLogEntry>,
}

impl App {
    pub fn new(global_config: config::GlobalConfig) -> Self {
        Self {
            current_office: None,  // user must init a office before switching
            offices: global_config.offices,
            command_palette_open: false,
            should_exit: false,
            active_worklog_ticket: None,
            worklog_entries: vec![],
        }
    }
    
    /// Switch the active office by index. Called from the command palette (Ctrl+P).
    pub fn switch_office(&mut self, index: usize) {
        if let Some(office_id) = self.offices.get(index).map(|o| o.id.clone()) {
            println!("Switching to office '{}'...", office_id);
            // In the full implementation: load office config, update TUI state.
            // This is scaffolding only.
        }
    }

    // ----------------------------------------------------------------------
    // P2: Work-log badge and overlay helpers
    // ----------------------------------------------------------------------

    /// Get the entry count (badge value) for a given ticket number.
    pub fn worklog_entry_count(&self, office_home: &std::path::Path, ticket_num: u64) -> anyhow::Result<usize> {
        let wl_path = bureau_lib::generate_worklog_path(
            &office_home.join("executions"),
            ticket_num,
        );
        bureau_lib::count_entries(&wl_path)
    }

    /// Load work-log entries for display in the overlay.
    pub fn load_worklog_entries(&mut self, office_home: &std::path::Path, ticket_num: u64) -> anyhow::Result<usize> {
        let wl_path = bureau_lib::generate_worklog_path(
            &office_home.join("executions"),
            ticket_num,
        );
        self.worklog_entries = bureau_lib::read_worklog(&wl_path)?;
        Ok(self.worklog_entries.len())
    }

    /// Show the work-log overlay for a ticket (toggles if already active).
    pub fn toggle_worklog_overlay(&mut self, office_home: &std::path::Path, ticket_num: u64) -> anyhow::Result<()> {
        if self.active_worklog_ticket == Some(ticket_num) {
            // Hide overlay.
            self.active_worklog_ticket = None;
            self.worklog_entries.clear();
        } else {
            // Show overlay — load last 5 entries for display.
            self.load_worklog_entries(office_home, ticket_num)?;
            let len = self.worklog_entries.len();
            if len > 0 {
                // Keep only the last 5.
                self.worklog_entries.truncate(len);
            }
            self.active_worklog_ticket = Some(ticket_num);
        }
        Ok(())
    }

    /// Check whether a work-log exists for this ticket (badge visibility).
    pub fn has_worklog(&self, office_home: &std::path::Path, ticket_num: u64) -> bool {
        let wl_path = bureau_lib::generate_worklog_path(
            &office_home.join("executions"),
            ticket_num,
        );
        // Use count_entries with default path — returns Err only on I/O.
        self.worklog_entry_count(office_home, ticket_num).unwrap_or(0) > 0
    }

    /// Format the last N work-log entries as plain text for TUI display.
    pub fn format_worklog_display(&self, max_entries: usize) -> String {
        let entries = &self.worklog_entries;
        // Show the last `max_entries` entries (oldest first).
        let show: &[bureau_lib::WorkLogEntry] = if entries.len() > max_entries {
            &entries[entries.len().saturating_sub(max_entries)..]
        } else {
            entries
        };

        show.iter().map(|e| {
            let icon = match e.event {
                bureau_lib::WorkLogEvent::System => "✓",
                bureau_lib::WorkLogEvent::Draft => "📄",
                bureau_lib::WorkLogEvent::Update => "📝",
                bureau_lib::WorkLogEvent::Note => "💡",
                bureau_lib::WorkLogEvent::Milestone => "⚑",
            };
            format!(
                " {} (turn {}) {}\n    {}\n",
                icon,
                e.turn,
                match &e.event {
                    bureau_lib::WorkLogEvent::System => "system",
                    bureau_lib::WorkLogEvent::Draft => "draft",
                    bureau_lib::WorkLogEvent::Update => "update",
                    bureau_lib::WorkLogEvent::Note => "note",
                    bureau_lib::WorkLogEvent::Milestone => "milestone",
                },
                e.body.trim()
            )
        }).collect::<Vec<_>>().join("")
    }
}

/// Helper for constructing an App without requiring a GlobalConfig.
#[derive(Default)]
pub struct AppBuilder {
    offices: Vec<config::OfficeEntry>,
}

impl AppBuilder {
    pub fn build(self) -> App {
        App {
            current_office: None,
            offices: self.offices,
            command_palette_open: false,
            should_exit: false,
            active_worklog_ticket: None,
            worklog_entries: vec![],
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
