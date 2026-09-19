# Plan: Work-log System Implementation

## Summary

The work-log is a per-ticket `{NNN}-worklog.md` artifact that records worker progress as
structured prose entries. It enables traceability for inspectors and resume capability for 
interrupted agents. This document tracks remaining integration steps.

## Completed ✅

### 1. Core module: `bureau-lib/src/worklog.rs`
- `WorkLogEvent` enum — system/draft/update/note/milestone
- `WorkLogEntry` struct with YAML frontmatter + prose body serialization/deserialization
- `read_worklog()`, `append_entry()`, `count_entries()` (for TUI badge)
- `generate_worklog_path(dir, ticket#)` — convention: `.bureau/executions/{NNN}-worklog.md`
- `bind_ticket(path, agent_id)` — operator-layer enforcement extracting expected/actual 
  ticket numbers from filenames; rejects on mismatch
- `handle_work_log_tool()` — maps event string → enum, creates entry, appends

### 2. Skill registration: `.bureau-executions/skills/worker/work-log/SKILL.md`
Declares the `work_log(event=..., body=...)` tool to worker agents with event type table,
examples per type, and format guidance.

### 3. Library wiring: `bureau-lib/src/lib.rs`
Added `pub mod worklog;` and re-exports `WorkLogEvent`, `WorkLogEntry`, `WorkLogBindingError`.

---

## Remaining Integration Steps (Priority Order)

### P1 — Wire tool dispatch in the agent loop
**File**: the module that currently handles tool intercept (likely `provider.rs` or a new 
`agent/executor.rs` being built).

This is the most critical piece. The agent loop needs to recognize `"work_log"` as an internal 
tool name alongside `write`, `read`, etc., and route it through this handler:

```rust
// In existing tool dispatch match arms:
"work_log" => {
    // Extract event_type + body from the LLM's JSON args.
    let event = serde_json::from_value::<String>(args["event"])?;
    let body = serde_json::from_value::<String>(args["body"])?;  
    let turn = agent_checkpoint.turn_count;
    
    // Build work-log path for this ticket.
    let path = generate_worklog_path(&executions_dir, ticket_number);
    
    worklog::handle_work_log_tool(&path, &agent.ticket_id, &event, turn, &body)
}
```

**Status**: Not yet in codebase — needs to be added where the tool intercept pipeline is wired.

---

### P2 — TUI badge + overlay for work-log entries
**File**: `tui/src/app.rs` (or wherever tick grid rows are rendered).

Two TUI changes needed:

1. **Badge on ticket row** — scan exec directory for `-worklog.md`, call `count_entries()`, display emoji count:
   ```
   │ 044-exec    execution   working     ● ○  [5] │
                              ↑── click to expand →
   ```

2. **Collapsible overlay on enter press** — show last 5 entries with icon prefix:
   ```
   ┌─── 044-execution Work Log ─────────────┐
   │ ⚑ milestone (turn 14)                   │
   │    Completed config parser             │
   │                                         │
   │ 📝 update (turn 5)                      │
   │    Wrote schema types in config.rs     │
   └──────────────────────────────────────────┘
   ```

**Status**: Requires TUI rendering changes — not yet started.

---

### P3 — Auto-append gate integration
**File**: wherever phase transitions are gated (likely `Office::transition_ticket()` or 
a dedicated `phase_gate` method).

Insert a call to `worklog::append_system_event(path, phase, message)` inside the state-change 
gate so every phase transition gets an auto-entry:

```rust
// Inside Office::transition_ticket(ticket, new_phase):
// ... existing validation + status update logic ...
let wl_path = generate_worklog_path(&executions_dir, ticket.number.unwrap_or(0));
worklog::append_system_event(
    &wl_path, 
    format!("{}", new_phase), 
    format!("Phase transition: {} → {}", old_phase, new_phase)
).ok();  // non-fatal — log but don't block
```

**Status**: Not started. Needs the gate method to exist first (it may be a TUI-only concern 
or may be in Office/agent.rs).

---

### P4 — Deploy skill to user directory for actual use
The SKILL.md currently lives at `.bureau-executions/skills/worker/work-log/SKILL.md` as a 
reference. For it to actually inject into worker prompts, copy to:

```
~/.bureau/skills/worker/work-log/SKILL.md
```

This is the path the `SkillsLoader` checks: `global_shared → office_shared → global_mode 
→ office_mode`, and `~/.bureau/skills/worker/` maps to `global_mode`.

---

## Files changed this commit

| File | Status | Purpose |
|------|--------|---------|
| `bureau-lib/src/worklog.rs` | **New** (516 lines) | Core work-log module with types, read/write, ticket binding, and tool handler |
| `bureau-lib/src/lib.rs` | **Modified** | Added `pub mod worklog;` + re-exports |
