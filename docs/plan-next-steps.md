# Bureau — Next Steps Plan

**Created**: 2025-01  
**Last Updated**: 2025-09-18  
**Working Commit**: `9f72071` (P5–P9 complete: agent loop integration, TUI helpers, CLI + office scaffold, tests)  
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

### P5 — Agent Loop Integration ✅
Wired discovered skills into the agent loop prompt context:
- **AgentLoop**: added `mode: Mode` field + `new()` constructor; implemented `assemble_system_prompt()` that:
  - Creates a `SkillsLoader` for the current mode and office path
  - Calls `assembly_prompt()` and prepends results as system prompt prefix
  - Includes agent context (mode, ticket_id) in the output
- **Sandbox + InspectorGate integration**: added `check_with_gate()` method on `PermitSandbox`
  - Takes `ticket_id` + gate reference; blocks writes if pending scope requests exist
  - Returns `Err(pending_request_ids)` or `Ok(check)` depending on gate status
- **DashboardView builder**: added `from_offices()` and `empty()` constructors

### P6 — TUI Integration of Dashboard Types ✅
Added ratatui rendering helpers in `bureau-lib::dashboard`:
- `status_badge() → &str`: returns Unicode badge per PermitStatus variant (✓▶⧖◆ etc.)
- `pending_action_bar(&PermissionPanel) → String`: generates `[A]pprove/[D]eny` action bar text
- `ticket_grid_as_table(&TicketGrid) → (headers, rows)`: formats grid for ratatui Table widget

### P7 — Run Config CLI ✅
Implemented CLI entry point (`tui/src/main.rs`) with all commands:
- `bureau-tui run-config list [mode]` → lists configs by mode or all
- `bureau-tui run-config add --name X --mode execution [--description] [--skip-inspection]` → writes config to `.bureau/run_configs.yaml`
- `bureau-tui start <office>` → loads office config + defaults, prints available configs for the mode
- `bureau-tui tui` (no-args) → scaffold TUI launcher (reads global config, lists offices)

### P8 — Office Init (`bureau new-office`) ✅
Implemented scaffolding in `main.rs cmd_new_office()`:
- Creates `.bureau/` directory with subdirs: `tickets/`, `plans/`, `executions/`, `inspections/`, `archive/`
- Generates `.bureau/config.yaml` with office id, name, worksite
- Writes default `.bureau/run_configs.yaml`
- Registers office in `~/.bureau/config.yaml` global registry

### P9 — Testing ✅
Unit and integration tests:
- **skills**: name fallback to dir-name, frontmatter override, invalid YAML rejection
- **sandbox + inspector gate**: pattern matching, default-deny, write-gating workflow
- **run_config**: save/load roundtrip, by_mode filtering, defaults on missing file
- **dashboard**: ticket grid archiving filter, pending action bar, status badges, dependency tracker
- **integration tests** (`tests/integration_test.rs`): sandbox→inspector pipeline, run-config persistence, skill stacking precedence, dashboard assembly, inspector lifecycle

---

## Known Issues / Blockers

1. **No local toolchain**: cargo is not installed locally; requires Nix flake evaluation for dev shell.
2. **Provider trait incomplete**: provider.rs has ChatProvider scaffolding — full reqwest implementation TBD.
3. **Research session saving pending**: `save_session()` in research/mod.rs has TODO stub print statements rather than actual fs writes.
4. **Ticket numbering**: ticket ID generation uses `rand::random::<u32>()` instead of uuid crate (future improvement).

---

## Pending Work (Future Sprints)

### P0–P9 Complete ✅
All infrastructure, skills system, run configs, dashboard types, inspector gate, CLI subcommands, office scaffolding, and tests are implemented.

---

### Sprint 1 — LLM Provider Integration + Agent Loop
**Goal**: Wire up an actual provider so the agent can call an LLM.

| Task | Module(s) | Details |
|---|---|---|
| **Provider: OpenAI impl** | `provider.rs` → `providers/openai.rs` | Fill in ChatProvider trait; POST to `$base_url/chat/completions`, stream responses, handle auth via ApiKeySource variant |
| **Open other providers** | `providers/{ollama, anthropic}.rs` | Ollama (localhost:11434) first, then Claude API when needed |
| **AgentLoop::run_turn()** | `agent.rs` | Pull checkpoint history tail + assemble_system_prompt(); call provider; write JSONL event; update checkpoint |
| **ModeBarrier fan-out/fan-in** | `agent.rs` | Spawn worker tasks per ticket in a phase; wait for all to complete before moving next mode; trigger gate barriers |
| **Checkpoint recovery** | `checkpoint::load()` + `TicketContextSnapshot` | On resume, feed last N history events as system prompt continuation |

---

### Sprint 2 — TUI Widget Rendering
**Goal**: Connect the dashboard helpers (P6) to actual ratatui widgets.

| Task | Module(s) | Details |
|---|---|---|
| **Sidebar: PermitCard rendering** | `tui/src/app.rs` + `bureau_lib::dashboard` | Expandable cards with status_badge(); toggle $EDITOR on press; click-to-highlight active ticket |
| **Main area: TicketGrid as Table widget** | `tui/src/app.rs` | Use ticket_grid_as_table() to feed ratatui::widgets::Table; highlight selected row; filter archived toggle |
| **Bottom panel: WorkerBar → Gauge** | `tui/src/app.rs` + `bureau_lib::dashboard` | Map progress percentages to ratatui::widgets::Gauge/BarGauge per active ticket |
| **Action bar: PermissionPanel items** | `tui/src/app.rs` + pending_action_bar() | Render [A]pprove/[D]eny keybindings; call InspectorGate on confirm; show resolved toast |
| **TUI config loading** | `app.rs` → GlobalConfig | Load offices from ~/.bureau/config.yaml at startup; populate sidebar list |

---

### Sprint 3 — Pipe-to-Office + Research Persistence
**Goal**: Research session save/load and pipe workflow.

| Task | Module(s) | Details |
|---|---|---|
| **ResearchSession::save_session()** | `research/mod.rs` | Write metadata YAML + per-finding markdown files to ~/.bureau/research/sessions/{id}/ |
| **Housekeeping TTL cleanup** | `research/mod.rs` | Scan sessions dir; delete non-important sessions older than research_session_ttl_days |
| **Pipe UX state machine** | `piping/mod.rs` → tui/src/pipeline.rs | Show pipeline choice dialog (existing office vs new); force focus_intent gate per Q32/B |
| **Pipe target resolver** | `config.rs` → OfficeEntry lookup | Validate office_id exists in GlobalConfig; resolve worksite path for ExistingOffice pipe |

---

### Sprint 4 — Archivist Validation + Run Proposals
**Goal**: Complete the D9 archivist pass.

| Task | Module(s) | Details |
|---|---|---|
| **Cross-reference audit** | `agent.rs` or new `archivist/mod.rs` | Walk all tickets in an office; verify parent_plan references exist; flag orphaned tickets |
| **ArchivistProposal generation** | `run_config/mod.rs` + propose_run_configs() | Build summary artifact from completed tickets; propose reusable run configs based on patterns found |
| **Archivist state machine** | `ticket.rs::Status/Transition` | Transition: ArchivingStarted → AuditComplete → GeneratingSummary → Complete per D3 |

---

### Sprint 5 — Cleanup + Polish
**Goal**: Strengthen quality of the existing codebase.

| Task | Module(s) | Details |
|---|---|---|
| **Ticket uuid crate** | `ticket.rs` | Replace rand::random::<u32>() with uuid::Uuid::new_v4() (unblocks provider dep resolution) |
| **Provider trait implementation** | `provider.rs` | Flesh out ChatMessage, ChatResponse types for multi-provider dispatch |
| **Integration test suite** | `bureau-lib/tests/` | Add pipe→office flow test; sandbox enforcement scenarios across modes |
| **Error type unification** | All modules | Replace generic anyhow where context-specific Error types add value |

---

### Known Issues / Blockers (updated)

| # | Issue | Impact | Resolution Path |
|---|---|---|---|
| 1 | **No local toolchain**: cargo is not installed locally. P2: requires Nix flake evaluation for dev shell. | Cannot compile or run tests | Set up nix develop; add flake.nak to workspace
| 2 | **Provider trait incomplete**: provider.rs has ChatProvider scaffolding — full reqwest impl TBD.
 | S1 depends on this
| **Research saving pending**: `save_session()` in research/mod.rs has TODO stub print statements rather than actual fs writes.
 | P3 blocks pipe flow
| 4 | **Ticket numbering**: uses `rand::random::<u32>()` instead of uuid crate (future improvement).
 | Future; low urgency |
| 5 | **Dead import**: `AtomicUsize` unused in tui/src/app.rs. |
 Cosmetic/code quality fix. Remove import.


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
        ├── main.rs                  ← CLI entry point (P7 + P8 commands)
        ├── mod.rs                   ← TUI event loop scaffold
        └── app.rs                   ← App state, office switching logic + builder pattern

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
