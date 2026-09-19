// P9: Integration tests — pipe-to-office flow, sandbox enforcement, skills stacking precedence

use bureau_lib::config::Mode;
use bureau_lib::dashboard::{PermissionEntry, PermissionPanel, PermitStatus};
use bureau_lib::inspector::{InspectorGate, PermissionAction, ScopeRequest};
use bureau_lib::run_config::{RunConfig, RunConfigs};
use bureau_lib::sandbox::PermitSandbox;
use bureau_lib::ticket::{AgentType, Scope};
use chrono::Utc;
use std::io::Write;

// ============================================================================
// Integration: sandbox + inspector gate pipeline (P5)
// ============================================================================

#[test]
fn test_full_sandbox_inspepector_pipeline() {
    // 1. Create a sandbox with limited write scope.
    let scope = Scope {
        allow_read: vec!["/data/**".into()],
        allow_write: vec!["/output/**".into()],
        deny_default: true,
        requested_paths: vec![],
    };
    let mut sandbox = PermitSandbox::new("permit-001", &scope);
    
    // 2. Worker requests scope expansion for a new path.
    let mut gate = InspectorGate::default();
    let req = ScopeRequest {
        from_ticket: "t-exec-001".into(),
        agent_type: AgentType::Worker,
        requested_writes: vec!["/new-dir/**".into()],
        justification: "Need to write new analytics output".into(),
        created_at: Utc::now(),
    };
    gate.submit(req);

    // 3. Check that write is blocked while pending.
    let result = sandbox.check_with_gate(
        std::path::Path::new("/new-dir/analysis.txt"),
        true, "t-exec-001", &gate
    );
    assert!(result.is_err());

    // 4. Inspector approves the scope request.
    gate.approve("t-exec-001", vec![], vec!["/new-dir/**".into()]);

    // Now approve should clear from pending.
    gate.approve(
        "t-exec-001", 
        vec!["/new-dir/**".into()], 
        vec![].into_iter().collect()
    );

    let has_pending = gate.has_pending_for("t-exec-001");
    assert!(!has_pending);
}

// ============================================================================
// Integration: run config persistence roundtrip (P7)
// ============================================================================

#[test]
fn test_run_config_save_and_load() {
    let dir = tempfile::tempdir().unwrap();
    let mut configs = RunConfigs::default();
    
    // Add multiple configs for different modes.
    let execution_c1 = RunConfig {
        name: "run-analysis".into(),
        run_mode: Mode::Execution,
        description: Some("Run analytics pipeline".into()),
        ..Default::default()
    };
    let inspection_c1 = RunConfig {
        name: "inspect-results".into(),
        run_mode: Mode::Inspection,
        skip_inspection: true,
        ..Default::default()
    };
    configs.configs.extend([execution_c1, inspection_c1]);

    // Save and load.
    configs.save(dir.path()).unwrap();
    let loaded = bureau_lib::run_config::load(dir.path()).unwrap();

    assert_eq!(loaded.configs.len(), 2);
    
    let by_exec = loaded.by_mode(Mode::Execution);
    assert_eq!(by_exec.len(), 1);
    assert_eq!(by_exec[0].name, "run-analysis");
}

// ============================================================================
// Integration: skills loading precedence (P1/P9 per Q8 stacking)
// ============================================================================

#[test]
fn test_skills_loader_stack_ordering() {
    let temp = tempfile::tempdir().unwrap();
    
    // Create a fake office path.
    let office_path = temp.path().to_path_buf();
    
    // Set up global_shared directory with an "analyzer" skill.
    let g_shared = temp.path().join("global_shared");
    std::fs::create_dir_all(g_shared.join("analyzer")).unwrap();
    let mut f1 = std::fs::File::create(g_shared.join("analyzer").join("SKILL.md")).unwrap();
    writeln!(f1, "---\nname: analyzer\ndescription: global analyzer\n---\n# Global Analyzer Skill").unwrap();
    
    // Set up office_shared with an "analyzer" skill (higher precedence).
    let o_shared = temp.path().join("office_shared");
    std::fs::create_dir_all(o_shared.join("analyzer")).unwrap();
    let mut f2 = std::fs::File::create(o_shared.join("analyzer").join("SKILL.md")).unwrap();
    writeln!(f2, "---\nname: analyzer\ndescription: office-specific analyzer\n---\n# Office Analyzer Skill").unwrap();

    // Create a SkillsLoader.
    let loader = bureau_lib::skills::SkillsLoader {
        global_shared: g_shared,
        office_shared: o_shared,
        global_mode: temp.path().join("missing"),
        office_mode: temp.path().join("missing2"),
        mode: "execution".to_string(),
    };

    let skills = loader.load_all_skills().unwrap();
    
    // Both should be loaded; only one named "analyzer" wins (last-wins from office_shared).
    let analyzer = skills.iter().find(|s| s.name == "analyzer");
    assert!(analyzer.is_some());
    assert_eq!(analyzer.unwrap().description, "office-specific analyzer");
}

// ============================================================================
// Integration: dashboard component assembly (P5/P6)
// ============================================================================

#[test]
fn test_dashboard_view_assembly() {
    let permits = vec![
        bureau_lib::dashboard::PermitCard::new("001", "Data Pipeline Audit"),
    ];
    let grid = bureau_lib::dashboard::TicketGrid::default();
    let bars = vec![bureau_lib::dashboard::WorkerBar {
        ticket_id: "001".into(),
        progress: 75,
        label: "analytics-worker".into(),
    }];
    let permissions = PermissionPanel {
        approved: vec![],
        pending: vec![PermissionEntry {
            ticket_id: "w-001".into(),
            path_pattern: "/tmp/**".into(),
            access_type: bureau_lib::dashboard::PermissionAccessType::Write,
            justification: "Need /tmp for temp files".into(),
            requested_at: Utc::now(),
        }],
    };
    let deps = bureau_lib::dashboard::DependencyTracker::default();

    let view = bureau_lib::dashboard::DashboardView::from_offices(
        permits.clone(), grid, bars.clone(), permissions.clone(), deps.clone(),
    );
    
    assert_eq!(view.permits.len(), 1);
    assert_eq!(view.worker_bars.len(), 1);
    assert_eq!(view.permissions.pending.len(), 1);

    // Test rendering helpers.
    let bar_text = bureau_lib::dashboard::pending_action_bar(&permissions);
    assert!(bar_text.contains("1 pending"));

    let (headers, rows) = bureau_lib::dashboard::ticket_grid_as_table(&grid);
    assert_eq!(headers.len(), 4);
    assert_eq!(rows.len(), 0); // no rows in empty grid
}

// ============================================================================
// Integration: inspector gate workflow
// ============================================================================

#[test]
fn test_inspector_gate_lifecycle() {
    let mut gate = InspectorGate::default();
    
    // Submit a request.
    let req = ScopeRequest {
        from_ticket: "worker-1".into(),
        agent_type: AgentType::Worker,
        requested_writes: vec!["/src/main.rs".into()],
        justification: "Need to write source code".into(),
        created_at: Utc::now(),
    };
    assert!(!gate.has_pending_for("worker-1")); // not yet pending
    gate.submit(req);
    assert!(gate.has_pending_for("worker-1"));
    
    // Check pending_sorted returns the request.
    let pending = gate.pending_sorted();
    assert_eq!(pending.len(), 1);

    // Deny a non-existent request (returns false).
    let denied = gate.deny("nonexistent", "Does not exist".into());
    assert!(!denied);
    
    // Approve the real request.
    let approved = gate.approve("worker-1", vec![], vec!["/src/**/*.rs".into()]);
    assert!(approved);
    assert!(!gate.has_pending_for("worker-1"));

    // Verify audit trail exists in resolved map.
    assert_eq!(gate.resolved.len(), 1);
}
