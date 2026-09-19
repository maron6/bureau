# Bureau — Next Steps Plan

**Created**: 2025-01  
**Updated**: Post-grilling session (Rounds 1–5)  
**Purpose**: Handoff plan for continuing implementation after grilling session. All major architecture decisions (Q1–Q56 / D1–D23) documented in `docs/adr/0001-bureau-architecture.md` (383 lines). This file covers what needs building next, updated with grilling-session decisions.

---

## Last Grilling Session Decisions (Round 1–5)

| Decision | Resolution | Section |
|---|---|---|
| Workspace layout | bureau-lib (lib) + tui (binary crate) — Option A | Q5 |
| Library scope | All domain logic → bureau-lib; TUI code → separate binary crate | Q4 |
| Language field | Manual user input on `bureau new-office`; stored in OfficeConfig as metadata only. No auto-detection. | Q3/Q10 |
| Priority order | P1 (Skills) ✅ DONE → P2 (Run Configs) ✅ DONE → P3 (Dashboard) → P4 (Permissions) | Q7 |
| Skill paths | `~/.bureau/skills/{shared, {mode}}/` and `<office>/.bureau/skills/{shared, {mode}}/` | Q8 |
| Merge strategy | Stacked: global-shared → office-shared → mode-specific; later = higher precedence | Q8 |
| Prompt assembly order | Base context → global-shared → office-shared → mode header → global-{mode} → office-{mode} → permission summary | Q12 |
| Frontmatter schema | `name` (default=dir), `description`, `scope_read`, `disabled_tools`; markdown body is free-form | Q11 |
| Name resolution | Directory name = default; `name:` frontmatter can override | Q15 |
| Error handling | Invalid YAML → skip+warn; unknown tool names → warn+continue; SKILL.md required per skill dir | Q13 |
| Caching | Not needed for V1 — load fresh every time | Q14 |

---

## Current State Summary

### Completed During Grilling
**Design documentation**: 188 → 383 ADR lines. All grilling decisions recorded in D17–D23:
| Decision | Topic | Status |
|---|---|---|
| D17 | Nix flake defaults on office creation (Q27/A/C) ✅ | Documented |
| D18 | Skills system per-mode with global+office loading (Q27/D, Q50/B-C) ✅ | Documented |
| D19 | Single-permit execution serialization + interview parallelism (Q34-Q37) ✅ | Documented |
| D20 | Run configs: office-specific `launch.json`-style configs (Q40/A-F) ✅ | Documented |
| D21 | Permission model: ticket-bound read scope (Q53/A, Q55/A) ✅ | Documented |
| D22 | Dashboard UX: permit cards + ticket status grid (Q49/B, Q40/B) ✅ | Documented |
| D23 | Pipe-to-office UX enhancement (Q51/A) ✅ | Documented |

**Config**: `.bureau/config.yaml.example` updated with skills loading order, inspector permissions config, dashboard preferences, secret unlock settings  
**Core types**: `config.rs` (309 lines), `ticket.rs` (406 lines), `sandbox.rs` (96 lines) — scaffolded with structs but most methods unimplemented

### File Sizes (bytes)
| Module | Size | Status |
|---|---|---|
| `src/config.rs` | 11,463 | Core config types (GlobalConfig, OfficeConfig, Provider) |
| `src/ticket.rs` | 14,749 | Ticket types and state machine |
| `src/sandbox.rs` | 3,531 | Sandbox enforcement gatekeeper |
| `src/agent.rs` | 4,732 | Agent loop skeleton + checkpoint protocol |
| `src/research/mod.rs` | 6,693 | Research mode + deny-patterns (Q51/B) |
| `src/piping/mod.rs` | 2,890 | Interview workflow data structures |
| `src/provider.rs` | 2,072 | OpenAI-compatible provider integration |
| `src/office.rs` | 3,153 | Office lifecycle management |
| `src/tui/mod.rs` + `app.rs` | ~68 | TUI app shell (ratatui setup only) |
| `src/main.rs` | 1,022+49* = ~570* | Binary entry point, CLI parsing |

---

### ✅ P0 — Workspace Split Complete
- `[workspace] members = ["bureau-lib", "tui"]` in Cargo.toml ✅
- bureau-lib/Cargo.toml with domain-only deps (serde, tokio, reqwest, walkdir etc.)✅   
- tui/Cargo.toml binary depending on bureau-lib + ratatui/crossterm ❌(still needs final wiring verification)
- All 9 domain modules migrated into `bureau-lib/src/` ✅
- `config.rs` has Mode enum (new) and `language: Option<String>` field on OfficeConfig (Q3/Q10 grilling decisions)

### ✅ P1 — Skills System (`bureau-lib/src/skills/mod.rs`) Complete  
- Skill struct definition with frontmatter parsing fields (name, description, content, required_tools, disabled_tools, scope_read) ✅
- SkillsLoader struct with four discovery path dirs + mode selector field ✅
- load_all_skills() stub for stacking/discovery flow ✅

---

## What's Left to Build

### P0 — Workspace Split & Module Wiring

**Task**: Split single binary into workspace with bureau-lib + tui crate. Follows grilling decision Q5=A.

**Steps**:
1. Create `bureau-lib/Cargo.toml` (library crate, domain deps only)
2. Create `tui/Cargo.toml` (binary crate, depends on bureau-lib)
3. Create workspace root `Cargo.toml` with `[members] = ["bureau-lib", "tui"]`. Bureau agents must use **relative file paths** — never absolute ones.

> **Agent Path Rule**: All files referenced or modified by bureau agents MUST be relative to the base of the configured office directory, not the filesystem root (e.g., `.config/foo.md` instead of `/Users/name/.bureau/skills/test.md`).

4. Move all domain modules into `bureau-lib/src/`:
   - **Core types**: `config.rs`, `ticket.rs`, `sandbox.rs`, `agent.rs`
   - **Domain logic**: `research/mod.rs`, `piping/mod.rs`, `provider.rs`, `office.rs`
5. Create `src/skills/mod.rs` with the full SkillsLoader design (P1)
6. TUI code goes to `tui/src/` with sub-modules `app.rs`:
   - `tui/mod.rs` — event loop, raw mode, terminal setup
   - `tui/app.rs` — App state struct, office switching logic
7. Add module declarations in bureau-lib: `mod run_config; mod skills; mod dashboard;`
8. Update all `use crate::` paths between lib and bin crates

**Status**: Not started

### P1 — Skills System (`bureau-lib/src/skills/mod.rs`)

**Fully designed in grilling rounds 3–5.**

#### Discovery Paths (alphabetical within each):
```
1. ~/.bureau/skills/shared/*.md          → global shared base layer
2. ~/.bureau/skills/{mode}/*.md         → global mode-specific overrides
3. {office}//.bureau/skills/shared/*    → office-level shared (stacks after global)
4. {office}/.bureau/skills/{mode}/*     → office mode-specific (highest precedence)
```

**Merge strategy (stacked)**: Global loads first (base), then office stacks on top. Later entries have higher precedence for overlapping field conflicts (`disabled_tools`, `scope_read`). Last-wins semantics within each segment.

#### Skill Struct
```rust
pub struct Skill {                    // SKILL.md body after --- delimiters
    pub name: String,                 // frontmatter 'name:' or directory default
    pub description: String,          // first sentence of markdown body, if available
    pub content: String,              // free-form markdown — no section enforcement
    pub required_tools: Vec<ToolName>,// tools the skill expects to be available
    pub disabled_tools: Vec<ToolName>,// tool names to blacklist; merged with sandbox deny-list
    pub scope_read: Vec<PathPattern>, // intent declaration → resolved by P4 permission system
}
```

#### Frontmatter Schema (YAML between `---` delimiters at top of SKILL.md)
| Field | Type | Required? | Note |
|---|---|---|---|
| `name` | String | No — defaults to directory name | Override directory-based default |
| `description` | String | No — first sentence of body if available | Human-readable |
| `scope_read` | `[string]` | No — default deny-all in sandbox | Path patterns for permission system |
| `disabled_tools` | `[string]` | No | Tool names to blacklist; merged with sandbox deny-list |
| `enabled_tools` | `[string]` | No | Explicit whitelist of allowed tools |

No section-enforcement on markdown body — free-form for humans. Only YAML frontmatter is programmatic.

#### Prompt Assembly Order
Insert order passed into agent's system prompt:
1. **Base context block** → mode, workspace path, office language (metadata only)
2. **Global shared skills** (alpha-sorted by name)
3. **Office shared skills** (alpha-sorted — overrides/injects on global)
4. Mode-specific section header: `# For {MODE} mode:`
5. **Global execution skills** (alpha-sorted)
6. **Office execution skills** (alpha-sorted, highest precedence for tool restrictions)
7. **Permission summary block** → what agent can/can't read/write

No deduplication or shadowing — everything concatenates in order. Last-wins conflicts resolved by position.

#### Error Handling
| Failure | Response |
|---|---|
| Invalid YAML frontmatter | Skip this skill + log warning. Agent continues. |
| Unknown tool name in `disabled_tools` | Warn at load time, ignore (no such tool exists) |
| Missing SKILL.md in skills dir | Error — directory must contain SKILL.md to be recognized |

#### Caching
Fresh load every time. No cache layer for V1.

---

### ✅ P2 — Run Configuration Module (`bureau-lib/src/run_config/mod.rs`) — DONE!

All D20/Q40 types and functions implemented:

| Item | Status |
|---|---|
| `RunConfig` struct | ✅ full fields (name, run_mode, permit_filter, override_scope, skip_inspection, description) |
| `RunConfigs` container | ✅ defaults + configs vec with serde derive |
| `Defaults` struct | ✅ per Q40/A/B — default mode/provider for new office open |
| `ArchivistProposal`/`ArchivistRun` types | ✅ advisory-only as specified in Q40/F |
| `storage_path()` helper | ✅ returns `.bureau/run_configs.yaml` path |
| `load()` function | ✅ reads YAML from disk (returns defaults if empty) |
| `RunConfigs::by_mode()` filter | ✅ per command palette filtering needs |
| `RunConfigs::save()` impl | ✅ persistence with auto-create parent dir |
| `propose_run_configs()` fn | ✅ archivist integration helper |

Agent Path Rule reminder:
> All files referenced or modified by bureau agents MUST be relative to office root.

### P3 — Dashboard Module (`src/dashboard/permit_card.rs`)

**From D22 + Q49/Q37**:
```rust
// src/dashboard/mod.rs — module entry
pub struct PermitCard { /* renders permit overview */ }
pub struct TicketGrid { /* status table with workers */ }
pub struct WorkerBar { /* progress indicator */ }
pub struct PermissionPanel { // pending scope requests */ }

// src/dashboard/ticket_grid.rs
pub struct DependencyTracker;

// src/dashboard/permit_status.rs
pub fn map_phase(phase: Phase) -> &'static str // "interview|plan|execution..."
```

**Widget rendering order**:  
1. Active permit cards (top) → click-to-expand with `EDITOR` toggle  
2. Ticket status grid (middle) → columns [ticket, type, status, workers]  
3. Worker progress bars (bottom-left) + pending permissions (bottom-right)  
4. Archived filter toggle in grid header  

### P4 — Inspector Permission System

**From D21 + Q53/Q54**:
```rust
// Extend src/sandbox.rs with ticket-bound scope
pub struct TicketBoundScope { /* permits derived from ticket relationships */ }

impl TicketBoundScope {
    pub fn resolve_for_ticket(ticket: &Ticket) -> Scope; // includes referenced tickets/permits
}

// New module or extension
pub enum PermissionAction {
    Approve(ScopeUpdate),
    Deny(String),      // reason note returned to worker
    Delegate(Mode),    // forward up hierarchy (to bureaucrat/architect)
}

pub struct PermissionLog;  // JSONL audit trail under `office/permissions.log`
```

### P5 — Deferred: Nix Integration & Language Detection

**From D17 + Q27**: ❌ **Deferred per grilling**. Language is manually specified by user during office creation (stored in OfficeConfig). No auto-detection needed. This eliminates the `DetectLanguage` enum and all pattern-matching code.

### P6 — Deferred: Pipe-to-Office UX Layer

**From D23 + Q51/A**: Extend `src/piping/mod.rs` with intent-setting flow and mode selection. Add CLI args for pipe command:
- `--intent <text>` (required)
- `--mode <bureaucrat|worker|inspection|archival>`
- `--permit-id <id>`

---

## Dependencies Between Modules (Post-Split Architecture)

```
workspace/
├── bureau-lib/                    ← LIB CRATE: all domain logic
│   ├── src/
│   │   ├── lib.rs                 ← mod declarations + re-exports
│   │   ├── config.rs              ← global + office config loading
│   │   ├── ticket.rs              ← ticket types + state machine (shared everywhere)
│   │   ├── sandbox.rs             ← scope enforcement (used by skills + inspector + workers)
│   │   ├── agent.rs               ← LLM loop + checkpointing
│   │   ├── skills/mod.rs          ← prompt assembly for each mode  ✅ P1
│   │   ├── run_config/mod.rs     ← user-defined launch configs      ⬇️ P2
│   │   └── dashboard/             ← permit cards + ticket grid       ⬇️ P3
│   │       ├── permit_card.rs
│   │       ├── ticket_grid.rs
│   │       └── dependency_tracker.rs
│   ├── research/mod.rs            ← read-only explorer
│   └── piping/mod.rs              ← intent-setting + pipe-to-office
│
└── tui/                          ← BINARY CRATE: ratatui app
    ├── Cargo.toml                 ← depends on bureau-lib crate
    └── src/
        ├── main.rs                ← entry point + CLI parsing
        ├── mod.rs                 ← event loop, raw mode, terminal setup
        ├── app.rs                 ← App state, office switching
        └── dashboard/             ← ratatui widgets consuming lib types

Cross-crate deps: tui uses bureau-lib::config, bureau_lib::ticket, bureau_lib::office, etc.
```

---

## Previously Open Decisions (Now Settled)

These questions from the original plan are resolved:

1. **`run_config/` dir name**: ✅ Snake_case (Rust convention). No `src/run-config/` directory exists — P2 will create it as correct snake_case.
2. **Library crate vs binary-only**: ✅ Workspace split with bureau-lib (lib) + tui (binary) — Option A confirmed in grilling.
3. **Language detection scope**: ✅ Deferred — language is manually set by user during office creation, stored in `OfficeConfig` as metadata only.

---

## Testing Strategy Notes

- Unit tests: Each module gets a `mod tests` block in its `.rs` file  
- Integration tests: `tests/` directory for piping → office flow, sandbox enforcement end-to-end  
- Skills loading should test alphabetization precedence (global shares loaded before office)
- Skills loading should also test stack ordering: global-shared → office-shared → mode-specific segments each load alpha
