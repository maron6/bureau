//! Dashboard types — P3 widget rendering order: cards→grid→workers/permissions.
//! Full UX spec lives in D22 per ADR 0001 + grilling Q49/Q37 decisions.
//! 
//! Rendering order (top → bottom):
//! 1. Active permit cards (click-to-expand with EDITOR toggle)
//! 2. Ticket status grid (columns: ticket, type, status, workers)
//! 3. Worker progress bars + pending permissions split view
//! 4. Archived filter toggle in grid header

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ============================================================================
// Permit Card — displayed at top of active view (ADR 0001 D22)
// ============================================================================

/// A permit card renders the current work-in-progress authority chain.
/// Click-to-expand shows the full permit context; EDITOR key opens it in $EDITOR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermitCard {
    /// Unique permit ID (e.g. "045").
    pub id: String,
    /// Human-readable title shown in sidebar/toolbar.
    pub title: String,
    /// Current status of the permit workflow (e.g. "in_execution", "ready_for_inspection").
    pub status: PermitStatus,
    /// Who last touched this permit (for audit trail).
    pub last_modified_by: Option<String>,
    /// Timestamp of last modification.
    pub modified_at: Option<DateTime<Utc>>,
}

impl PermitCard {
    /// Create a new permit card with current timestamp.
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            status: PermitStatus::Draft,
            last_modified_by: None,
            modified_at: Some(Utc::now()),
        }
    }

    /// Label for the card's status badge in the TUI.
    pub fn status_label(&self) -> &str {
        self.status.as_ref()
    }
}

/// Status of a permit in the dashboard view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermitStatus {
    /// Draft permit — not yet shared for review.
    Draft,
    /// Submitted for user/agent review.
    InReview,
    /// Approved and binding on downstream work.
    Approved,
    /// Currently under execution by worker(s).
    InExecution,
    /// Execution complete, waiting inspection.
    ReadyForInspection,
    /// Passed inspection.
    Inspected,
    /// Under archival review.
    Archiving,
    /// Fully archived.
    Archived,
}

impl std::fmt::AsRef<str> for PermitStatus {
    fn as_ref(&self) -> &str {
        match self {
            Self::Draft => "draft",
            Self::InReview => "in_review",
            Self::Approved => "approved",
            Self::InExecution => "in_execution",
            Self::ReadyForInspection => "ready_inspection",
            Self::Inspected => "inspected",
            Self::Archiving => "archiving",
            Self::Archived => "archived",
        }
    }
}

// ============================================================================
// Ticket Grid — status table with workers (columns → rows)
// ============================================================================

/// The ticket grid component renders a multi-column table of tickets.
/// Used in the dashboard to show all active work items at a glance.
#[derive(Debug, Clone)]
pub struct TicketGrid {
    /// Column headers for the grid (ticket, type, status, workers).
    pub columns: Vec<String>,
    /// Rows of ticket summaries displayed in the grid.
    pub rows: Vec<TicketRow>,
    /// Whether archived tickets are filtered out.
    pub show_archived: bool,
}

impl Default for TicketGrid {
    fn default() -> Self {
        Self {
            columns: vec![
                "ticket".into(),
                "type".into(),
                "status".into(),
                "workers".into(),
            ],
            rows: vec![],
            show_archived: false,
        }
    }
}

/// A single row in the ticket grid.
#[derive(Debug, Clone)]
pub struct TicketRow {
    /// Display ticket number (e.g. "003").
    pub display_id: String,
    /// Ticket type label (plan, execution, inspection, etc.).
    pub type_label: String,
    /// Current status string for display.
    pub status: String,
    /// Active worker count for this ticket.
    pub active_workers: u8,
    /// Progress percentage (0-100) if available.
    pub progress: Option<u8>,
}

impl TicketRow {
    /// Format a compact row line for TUI rendering.
    pub fn render_line(&self) -> String {
        let progress_str = self.progress
            .map(|p| format!("{:3}%", p))
            .unwrap_or_else(|| "   -".into());
        format!(
            "[{:<5}] {:<12} {:<16} {} workers",
            self.display_id,
            self.type_label,
            self.status,
            progress_str,
        )
    }
}

// ============================================================================
// Worker Bar — progress indicator per active ticket
// ============================================================================

/// A horizontal progress bar widget showing completion percentage for a single item.
#[derive(Debug, Clone)]
pub struct WorkerBar {
    /// Which ticket this bar belongs to.
    pub ticket_id: String,
    /// Progress in tenths (0-100 scale).
    pub progress: u8,
    /// Label shown alongside the bar (e.g. worker name or phase).
    pub label: String,
}

impl WorkerBar {
    /// Render a box-drawing progress bar for ratatui.
    pub fn render(&self, width: usize) -> String {
        let filled = if self.progress < 100 {
            (width as f64 * self.progress as f64 / 100.0).ceil() as usize
        } else {
            width
        };

        let empty = width.saturating_sub(filled);
        format!(
            " {} |{}{}| {}/100",
            self.label,
            "█".repeat(filled.max(1)),
            "░".repeat(empty),
            self.progress,
        )
    }
}

// ============================================================================
// Permission Panel — pending scope requests split-view
// ============================================================================

/// Pending permission requests for the current office shown in a split panel.
#[derive(Debug, Clone)]
pub struct PermissionPanel {
    /// Approved scope requests (granted).
    pub approved: Vec<PermissionEntry>,
    /// Pending scope requests awaiting user approval.
    pub pending: Vec<PermissionEntry>,
}

impl Default for PermissionPanel {
    fn default() -> Self {
        Self {
            approved: vec![],
            pending: vec![],
        }
    }
}

/// A single permission grant or pending request.
#[derive(Debug, Clone)]
pub struct PermissionEntry {
    /// The ticket requesting this permission.
    pub ticket_id: String,
    /// Path pattern being requested.
    pub path_pattern: String,
    /// Access type (read or write).
    pub access_type: PermissionAccessType,
    /// Justification provided by the agent.
    pub justification: String,
    /// Timestamp the request was made.
    pub requested_at: chrono::DateTime<Utc>,
}

/// Type of file access being requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionAccessType {
    Read,
    Write,
    ReadWrite,
}

impl std::fmt::Display for PermissionAccessType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read => write!(f, "read"),
            Self::Write => write!(f, "write"),
            Self::ReadWrite => write!(f, "rw"),
        }
    }
}

// ============================================================================
// Dependency Tracker — cross-ticket dependency graph
// ============================================================================

/// Tracks dependencies between tickets to visualize blocked work.
#[derive(Debug, Clone)]
pub struct DependencyTracker {
    /// Edges: (depends_on_ticket_id, depends_on_status) → (target_ticket_id, required_status).
    /// This represents: "ticket A cannot complete until ticket B reaches status X".
    pub dependencies: Vec<DependencyEdge>,
}

impl Default for DependencyTracker {
    fn default() -> Self {
        Self {
            dependencies: vec![],
        }
    }
}

/// A single dependency edge in the tracker.
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// The ticket that is blocked.
    pub blocker_ticket: String,
    /// The status the blocking ticket must reach.
    pub required_status: String,
    /// The ticket that needs to be completed first.
    pub target_ticket: String,
}

impl DependencyTracker {
    /// Add a dependency between two tickets.
    pub fn add_dependency(&mut self, blocker: impl Into<String>, target: impl Into<String>, status: &str) {
        self.dependencies.push(DependencyEdge {
            blocker_ticket: blocker.into(),
            required_status: status.into(),
            target_ticket: target.into(),
        });
    }

    /// Check if a specific ticket is blocked by any pending dependencies.
    pub fn is_blocked(&self, ticket_id: &str) -> bool {
        self.dependencies.iter().any(|e| e.blocker_ticket == ticket_id)
            && !self.all_target_statuses_met(ticket_id)
    }

    /// Returns true if all dependencies for a given ticket have been met.
    fn all_target_statuses_met(&self, ticket_id: &str) -> bool {
        self.dependencies.iter().all(|e| e.target_ticket != ticket_id)
    }

    /// Get all tickets that would be unblocked if `ticket_id` reaches the given status.
    pub fn unblockers_on_status(&self, ticket_id: &str, status: &str) -> Vec<String> {
        self.dependencies
            .iter()
            .filter(|e| e.target_ticket == ticket_id && e.required_status == status)
            .map(|e| e.blocker_ticket.clone())
            .collect()
    }
}

// ============================================================================
// Phase mapping — converts internal Phase to display strings
// ============================================================================

use crate::config::Mode;

/// Map a Mode variant to its human-readable dashboard label.
pub fn map_phase(phase: Mode) -> &'static str {
    match phase {
        Mode::Interviewing => "interview",
        Mode::Planning => "plan",
        Mode::Execution => "execution",
        Mode::Inspection => "inspection",
        Mode::Archival => "archival",
    }
}

/// Dashboard data gathered from an office for rendering the TUI view.
#[derive(Debug, Clone)]
pub struct DashboardView {
    /// Active permit cards to display at the top.
    pub permits: Vec<PermitCard>,
    /// The ticket status grid.
    pub grid: TicketGrid,
    /// Progress bars for active worker tasks.
    pub worker_bars: Vec<WorkerBar>,
    /// Pending permission requests in split view.
    pub permissions: PermissionPanel,
    /// Cross-ticket dependency graph.
    pub dependencies: DependencyTracker,
}

impl Default for DashboardView {
    fn default() -> Self {
        Self {
            permits: vec![],
            grid: TicketGrid::default(),
            worker_bars: vec![],
            permissions: PermissionPanel::default(),
            dependencies: DependencyTracker::default(),
        }
    }
}

// ==========================================================================
// Dashboard builder — assembles View from office data (P5)
// ==========================================================================

impl DashboardView {
    /// Assemble a dashboard view from an office's ticket set and permit chain.
    pub fn from_offices(
        permits: Vec<PermitCard>,
        grid: TicketGrid,
        worker_bars: Vec<WorkerBar>,
        permissions: PermissionPanel,
        dependencies: DependencyTracker,
    ) -> Self {
        Self { permits, grid, worker_bars, permissions, dependencies }
    }

    /// Build an empty dashboard suitable as a placeholder when no office is loaded.
    pub fn empty() -> Self {
        Self::default()
    }
}

// ==========================================================================
// P6: ratatui rendering methods for each dashboard component
// ==========================================================================

/// Status badge character for use in the TUI sidebar.
pub fn status_badge(status: &PermitStatus) -> &'static str {
    match status {
        PermitStatus::Draft => "\u{00B7}",
        PermitStatus::InReview => "\u{25CC}",
        PermitStatus::Approved => "\u{2713}",
        PermitStatus::InExecution => "\u{25B6}",
        PermitStatus::ReadyForInspection => "\u{29DA}",
        PermitStatus::Inspected => "\u{2717}",
        PermitStatus::Archiving => "\u{25E7}",
        PermitStatus::Archived => "\u{25C6}",
    }
}

/// Render a compact summary line for display in the TUI action bar.
pub fn pending_action_bar(permissions: &PermissionPanel) -> String {
    if permissions.pending.is_empty() {  
        "No pending permission requests".to_string()
    } else {
        format!(
            "\u{26A0} {} pending | \u{2713} {} approved [A]pprove/[D]eny",
            permissions.pending.len(),
            permissions.approved.len()
        )
    }
}

/// Build a ratatui Table column layout for the ticket grid.
/// Returns (headers, rows) where each row is an array of formatted cell strings.
pub fn ticket_grid_as_table(grid: &TicketGrid) -> (Vec<String>, Vec<Vec<String>>) {
    let headers = grid.columns.clone();
    let mut rows = Vec::new();

    for row in &grid.rows {
        if !grid.show_archived || !row.status.contains("archived") {
            rows.push(vec![
                row.display_id.clone(),
                row.type_label.clone(),
                row.status.clone(),
                format!("{} workers", row.active_workers),
            ]);
        }
    }

    (headers, rows)
}



// ==========================================================================
// P9: Unit tests for dashboard components and rendering
// ==========================================================================
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ticket_grid_as_table_filters_archived() {
        let grid = TicketGrid {
            columns: vec!["ticket".into(), "type".into(), "status".into(), "workers".into()],
            rows: vec![
                TicketRow { display_id: "001".into(), type_label: "execution".into(), status: "in_progress".into(), active_workers: 2, progress: Some(50) },
                TicketRow { display_id: "002".into(), type_label: "archival".into(), status: "archived_in_2024".into(), active_workers: 0, progress: None },
            ],
            show_archived: false,
        };
        let (headers, rows) = ticket_grid_as_table(&grid);
        assert_eq!(headers.len(), 4);
        assert_eq!(rows.len(), 1); // archived row filtered out
    }

    #[test]
    fn test_pending_action_bar_empty() {
        let panel = PermissionPanel::default();
        let bar = pending_action_bar(&panel);
        assert!(bar.contains("No pending"));
    }

    #[test]
    fn test_pending_action_bar_has_pending() {
        let panel = PermissionPanel {
            approved: vec![PermissionEntry::default()],
            pending: vec![
                PermissionEntry {
                    ticket_id: "t1".into(),
                    path_pattern: "/new/**".into(),
                    access_type: PermissionAccessType::Write,
                    justification: "need access".into(),
                    requested_at: chrono::Utc::now(),
                },
            ],
        };
        let bar = pending_action_bar(&panel);
        assert!(bar.contains("1 pending"));
        assert!(bar.contains("[A]pprove/[D]eny"));
    }

    #[test]
    fn test_status_badge() {
        assert_eq!(status_badge(&PermitStatus::Approved), "\u{2713}"); // checkmark
        assert_eq!(status_badge(&PermitStatus::Draft), "\u{00B7}");   // middle dot
        assert_eq!(status_badge(&PermitStatus::InExecution), "\u{25B6}"); // play
    }

    #[test]
    fn test_dashboard_view_default() {
        let view = DashboardView::default();
        assert!(view.permits.is_empty());
        assert!(view.worker_bars.is_empty());
        
        let empty = DashboardView::empty();
        assert_eq!(empty.permits.len(), view.permits.len());
    }

    #[test]
    fn test_dependency_tracker_unblocks() {
        let mut tracker = DependencyTracker::default();
        // B blocks A: "ticket A is blocked until ticket B reaches 'approved'"
        tracker.add_dependency("B", "A", "approved");
        
        assert!(tracker.is_blocked("A"));
        
        // After B reaches 'approved', A should be unblocked
        // (simulated by removing the dependency or checking manually)
        let unblocked = tracker.unblockers_on_status("B", "approved");
        assert_eq!(unblocked, vec!["A".to_string()]);
    }
}
