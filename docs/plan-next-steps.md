# Bureau — Next Steps Plan

**Created**: 2025-01  
**Last Updated**: 2025-09-18  
**Git Commit**: [see commit hash when pushed]  
**ADR Reference**: `docs/adr/0001-bureau-architecture.md` (D1-D23 decisions)  
**Purpose**: Handoff for continuing implementation. All grilling decisions recorded in ADRs; this tracks what is built vs pending.

---

## Grilling Session Decisions (Rounds 1–5) — settled

| Decision | Resolution | Section |
|---|---|---|
| Workspace layout | bureau-lib (lib) + tui (binary crate), Option A | Q5 |
| Library scope | All domain logic → bureau-lib; TUI code → separate binary crate | Q4 |
| Language field | Manual user input on `bureau new-office`; metadata only. No auto-detection. | Q3/Q10 |
| Priority order | P1 (Skills) → P2 (Run Configs) → P3 (Dashboard) → P4 (Permissions) | Q7 |
| Skill paths | `~/.bureau/skills/{shared, {mode}}/` + `<office>/.bureau/skills/{shared, {mode}}/` | Q8 |
| Merge strategy | Stacked: global-shared → office-shared → mode-specific; last-wins within tiers | Q8 |
| Prompt assembly order | Base context → global-shared → office-shared → mode-header → global-mode → office-mode → permission summary | Q12 |
| Frontmatter schema | `name`, `description`, `scope_read`, `disabled_tools`; markdown body is free-form | Q11 |
| Name resolution | Directory name = default; `name:` frontmatter can override | Q15 |
| Error handling | Invalid YAML → skip+warn; unknown tools → warn+continue; SKILL.md required | Q13 |

---

## Completed (execution pass)

### P0 — Workspace Split ✅
- Root Cargo.toml: `[workspace] members = ["bureau-lib", "tui"]`
- bureau-lib/ Cargo.toml: domain-only deps (serde, tokio, reqwest, walkdir, anyhow, serde_yaml)
- tui/ Cargo.toml: binary crate depending on bureau-lib + ratatui/crossterm
- 8 domain modules migrated to `bureau-lib/src/` (config, ticket, sandbox, agent, office, provider, research, piping)
- lib.rs: mod declarations + pub use re-exports

### P1 — Skills System ✅
- `Skill` struct: name, description, content, required_tools, disabled_tools, scope_read
- `SkillsLoader`: 4 stacked discovery paths with full implementation:
  - `load_all_skills()` → walks all 4 paths in ADR3 order; last-wins dedup by name; returns alphabetically sorted skills
  - `load_skill(name)` → finds highest-precedence match for a named skill
  - `assembly_prompt()` → concatenates all discovered skills as `<skill>` blocks per Q12 ordering spec
  - `list_skill_names()` → alphabetical list for TUI command palette
- `parse_skill_file()` → extracts YAML frontmatter (name/description/scope_read/disabled_tools) from SKILL.md files, falls back to directory name as default (Q15)
- `parse_frontmatter()` → validates YAML syntax; per ADR3/Q13: invalid YAML → skip+warn

### P2 — Run Configuration Module ✅
Per D20/Q40: office-specific launch profiles (like VS Code launch.json)

| Type | Lines | Purpose |
|---|---|---|
| `ArchivistProposal` | 16 | Advisory-only per D20/Q40/F |
| `ArchivistRun` | 24 | Single suggestion entry |
| `RunConfig` | ~45 | User-defined launch profile (serde) |
| `RunConfigs` | ~89 | Container: defaults + configs vec |
| `Defaults` | ~106 | Initial office-open mode/provider values |

Key functions implemented:
- `propose_run_configs(permit)` → Advisory archivist helper, returns ArchivistProposal
- `storage_path()` → `.bureau/run_configs.yaml` location (per office root)
- `load(office_home)` → deserializes YAML; returns defaults if file missing
- `RunConfigs::by_mode()` → filter configs by Mode variant
- `RunConfigs::save()` → persistence with auto-create parent directory

### P3 — Dashboard Module ✅
Implemented types:

| Type | Purpose |
|---|---|
| `PermitCard` + `PermitStatus` | Permit overview with 8 lifecycle statuses; click-to-expand + $EDITOR toggle |
| `TicketGrid` + `TicketRow` | Multi-column table (ticket/type/status/workers) with rendered line output |
| `WorkerBar` | Box-drawing progress bar (`render(width) → string`) |
| `PermissionPanel` + `PermissionEntry` | Split view of approved vs pending scope requests |
| `DependencyTracker` + `DependencyEdge` | Cross-ticket DAG; add/unblock/is_blocked queries |
| `map_phase()` | `Mode → &'static str` display label mapping |
| `DashboardView` | Composite aggregating all sub-components for TUI rendering |

### P4 — Inspector Permission System ✅
Implemented types:

| Type | Purpose |
|---|---|
| `TicketBoundScope` | Derives from ticket permit chain; inherits read patterns + inspector write paths for annotations |
| `PermissionAction` | Enum: **Approve** (typed new_read/write_patterns), **Deny** (reason+suggestion), **Delegate** (mode+instructions) |
| `ScopeRequest` | Worker→inspector bridge: ticket, agent_type, requested_writes, justification, timestamp |
| `InspectorGate` | Approval workflow manager: `submit()`, `approve()`, `deny()`, `pending_sorted()`, `has_pending_for()` |
| `ResolvedRequest` | Audit trail record: from_ticket, action_label, resolved_at |

---

## Known Issues / Blockers

1. **No local toolchain**: cargo is not installed locally; requires Nix flake evaluation for dev shell.
2. **Provider trait incomplete**: provider.rs has ChatProvider scaffolding — full reqwest implementation TBD.
3. **Research session saving pending**: `save_session()` in research/mod.rs has TODO stub print statements rather than actual fs writes.
4. **Ticket numbering**: ticket ID generation uses `rand::random::<u32>()` instead of uuid crate (future improvement).

---

## Pending Work

### P5 — Agent Loop Integration (Next Priority)
Wire discovered skills into the agent loop prompt context:
- Call `SkillsLoader::assembly_prompt()` before each LLM call during execution/planning modes
- Pass assembly output as system prompt prefix alongside PermitCard/TicketGrid state snapshot
- Wire `InspectorGate.pending_sorted()` into sandbox decision routing (block worker writes until inspection clears scope requests)

### P6 — TUI Integration of Dashboard Types
Use `bureau_lib::dashboard` modules in the tui crate:
- Render `PermitCard`s as expandable sidebar items in `tui/src/app.rs` command palette
- Feed `TicketGrid.rows` into a ratatui Table widget
- Connect `WorkerBar.render()` to ratau's Gauge/BarGauge widgets for progress display
- Wire `PermissionPanel.pending` into the TUI action bar (Approve/Deny keybindings)

### P7 — Run Config CLI
- `bureau run-config list [mode]` → print configs from `.bureau/run_configs.yaml`
- `bureau run-config add --name X --mode execution ...` → write config entry
- `bureau start <office>` → resolve default config via `run_config::load()` then invoke agent loop

### P8 — Office Init (`bureau new-office`)
- Scaffold `.bureau/config.yaml`, directory layout (interviews_dir, plans_dir, etc.)
- Ask user for office name + language (manual per ADR3/Q10 grilling)
- Register in `GlobalConfig::offices` list

### P9 — Testing
- Unit tests in `mod tests` blocks in each module
- Integration tests: pipe→office flow, sandbox enforcement, skills stacking precedence
- Skills loading: test alphabetization precedence (global-shared loads before office-shared)

---

## Workspace Architecture (current state)

```
bureau/                              ← WORKSPACE ROOT
├── Cargo.toml                       ← [workspace] members = ["bureau-lib", "tui"]
├── bureau-lib/                      ← LIB CRATE: all domain logic
│   ├── Cargo.toml                   ← domain deps only (serde, tokio, reqwest...)
│   └── src/
│       ├── lib.rs                   ← mod declarations + pub use re-exports
│       ├── config.rs                ← GlobalConfig, OfficeConfig, Mode enum + ResearchConfig
│       ├── ticket.rs                ← Ticket types + state machine (Ticket, Scope, AgentCheckpoint)
│       ├── sandbox.rs               ← PermitSandbox + AccessCheck + ScopeExpansionRequest
│       ├── agent.rs                 ← AgentLoop + ModeBarrier + checkpoint I/O
│       ├── office.rs                ← Office lifecycle helpers per D1/D21
│       ├── provider.rs              ← ChatProvider trait + ChatMessage + ChatResponse (scaffolding)
│       ├── skills/mod.rs            ← Skill + SkillsLoader ✅ P1 completed
│       ├── run_config/mod.rs        ← RunConfig types & storage ✅ P2 completed
│       ├── dashboard/mod.rs         ← PermitCard, TicketGrid, WorkerBar, etc. ✅ P3 completed
│       ├── inspector/mod.rs         ← TicketBoundScope, PermissionAction, InspectorGate ✅ P4 completed
│       ├── research/mod.rs          ← Read-only explorer mode + session persistence
│       └── piping/mod.rs            ← Intent-setting data structures (PipeTarget, PipeUX)
└── tui/                             ← BINARY CRATE
    ├── Cargo.toml                   ← depends on bureau-lib + ratatui/crossterm
    └── src/
        ├── mod.rs                   ← CLI entry point
        └── app.rs                   ← App state, office switching logic

Cross-crate boundary: tui uses bureau_lib::{config, ticket, sandbox, skills, run_config, dashboard, inspector}
```

---

## Agent Path Rule (per Q5/Q7)

> All files referenced or modified by bureau agents MUST be relative to the base of the configured office directory, not the filesystem root.  
> Example: `.bureau/run_configs.yaml` — NOT `/home/user/projects/bureau/.bureau/run_configs.yaml`

---

## Testing Strategy Notes

- Unit tests: `mod tests` blocks in each `.rs` file
- Integration tests: `tests/` directory for pipe→office flow, sandbox enforcement
- Skills loading: test alphabetization precedence (global-shared loads before office-shared)
- Skills loading: test stack ordering — global-shared → office-shared → mode-specific tiers
