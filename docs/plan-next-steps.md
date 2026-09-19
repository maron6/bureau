# Bureau — Next Steps Plan

**Created**: 2025-01  
**Last Updated**: Post-execution session (P0/P1/P2)  
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

### P1 — Skills System ✅ (stub)
- `Skill` struct: name, description, content, required_tools, disabled_tools, scope_read
- `SkillsLoader`: 4 stacked discovery path dirs + mode selector field
- `load_all_skills()` scaffolding for stacking/discovery flow

### P2 — Run Configuration Module ✅ **DONE THIS SESSION**
Per D20/Q40: office-specific launch profiles (like VS Code launch.json)

| Type | Lines | Purpose |
|---|---|---|
| `ArchivistProposal` | 16 | Advisory-only per D20/Q40/F |
| `ArchivistRun` | 24 | Single suggestion entry |
| `RunConfig` | 45 | User-defined launch profile (serde) |
| `RunConfigs` | 89 | Container: defaults + configs vec |
| `Defaults` | 106 | Initial office-open mode/provider values |

Key functions implemented:
- `propose_run_configs(permit)` → Advisory archivist helper, returns ArchivistProposal
- `storage_path()` → `.bureau/run_configs.yaml` location (per office root)
- `load(office_home)` → deserializes YAML; returns defaults if file missing
- `RunConfigs::by_mode()` → filter configs by Mode variant
- `RunConfigs::save()` → persistence with auto-create parent directory

---

## Known Issues / Blockers

1. **Rust syntax in run_config/mod.rs**: Some escaped-quote sequences may still be present from multi-round writing — verify with compiler before proceeding. See line ~160 map_err closures.
2. **No local toolchain**: cargo is not installed locally; requires Nix flake evaluation for dev shell.
3. **TUI imports need verification**: `crate::` → `bureau_lib::` migration should compile cleanly when toolchain available.
4. **P1 SkillsLoader needs full impl**: Currently a stub — the actual frontmatter YAML parsing, stacking logic, and prompt assembly must be wired next.
5. **serde yaml import in run_config**: Check import of `serde_yaml` is present in Cargo.toml dependencies (added from grilling decisions).

---

## Pending Work

### ✅ P3 — Dashboard Module (`bureau-lib/src/dashboard/`)
**From D22 + Q49/Q37**

Types to implement:
```rust
pub struct PermitCard { id, title, status }       // renders permit overview
pub struct TicketGrid {}                             // status table with workers
pub struct WorkerBar {}                              // progress indicator  
pub struct PermissionPanel {}                        // pending scope requests
pub struct DependencyTracker                         // cross-ticket deps
pub fn map_phase(phase: Phase) -> &'static str     // "interview|plan|execution..."
```

Rendering order (top → bottom):
1. Active permit cards (click-to-expand with EDITOR toggle)
2. Ticket status grid (columns: ticket, type, status, workers)
3. Worker progress bars + pending permissions split view
4. Archived filter toggle in grid header

### ✅ P4 — Inspector Permission System
**From D21 + Q53/Q54**

Types to implement:
```rust
pub struct TicketBoundScope { /* derives from ticket relationships */ }
pub enum PermissionAction { Approve(ScopeUpdate), Deny(String), Delegate(Mode) }
```

### P5 — Deferred Nix Integration & Language Detection
❌ **Deferred per grilling** — language is manually set by user on office creation. No auto-detection needed. Eliminates DetectLanguage enum entirely.

### P6 — Deferred: Pipe-to-Office UX Layer
Add CLI args for pipe command: `--intent` (required), `--mode`, `--permit-id`.

---

## Workspace Architecture (current state)

```
bureau/                              ← WORKSPACE ROOT
├── Cargo.toml                       ← [workspace] members = ["bureau-lib", "tui"]
├── bureau-lib/                      ← LIB CRATE: all domain logic
│   ├── Cargo.toml                   ← domain deps only (serde, tokio, reqwest...)
│   └── src/
│       ├── lib.rs                   ← mod declarations + pub use re-exports
│       ├── config.rs                ← GlobalConfig, OfficeConfig, Mode enum
│       ├── ticket.rs                ← Ticket types + state machine
│       ├── sandbox.rs               ← Scope enforcement
│       ├── agent.rs                 ← LLM loop + checkpointing
│       ├── office.rs                ← Office lifecycle
│       ├── provider.rs              ← OpenAI-compatible client
│       ├── skills/mod.rs            ← Skill + SkillsLoader (stub) ✅ P1
│       ├── run_config/mod.rs        ← RunConfig types & storage ✅ P2
│       ├── dashboard/mod.rs         ← PermitCard stub ⬇️ P3
│       ├── research/mod.rs          ← Read-only explorer mode
│       └── piping/mod.rs            ← Intent-setting data structures
└── tui/                             ← BINARY CRATE
    ├── Cargo.toml                   ← depends on bureau-lib + ratatui/crossterm
    └── src/
        ├── main.rs                  ← CLI entry point
        ├── mod.rs                   ← Event loop, raw mode, terminal setup
        └── app.rs                   ← App state, office switching logic

Cross-crate boundary: tui uses bureau_lib::{config, ticket, sandbox, skills, run_config}
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
