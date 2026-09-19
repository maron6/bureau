# ADR 2: Workspace Layout — bureau-lib Library + tui Binary Crate

**Status**: Accepted  
**Date**: Post-grilling session Round 5  
**Context**: Decision Q5 from grilling session. Previously the project was a single binary with flat `src/` modules. The question was whether to keep that structure or split into crates for cleaner boundaries.

## Problem

- Should bureau be a single Rust binary, or split the domain logic (agent loop, config types, sandbox) from the user-facing TUI?
- If splitting, should it be one workspace with two directories (Option A), or modify `Cargo.toml` to add `[lib]` path alongside the existing binary (Option B)?

## Decision

**Option A**: Separate crates in a Cargo workspace.

```
workspace/
├── bureau-lib/          # library crate containing all domain logic
│   ├── Cargo.toml       # no ratatui dependency here
│   └── src/
│       ├── lib.rs       # mod declarations + re-exports
│       ├── config.rs    ← GlobalConfig, OfficeConfig
│       ├── ticket.rs    ← Ticket types + state machine
│       ├── sandbox.rs   ← Scope enforcement gatekeeper
│       ├── agent.rs     ← LLM loop + checkpoint protocol
│       ├── skills/      ← Skill loading, prompt assembly
│       ├── run_config/  ← Run profiles (.bureau/run_configs.yaml)
│       ├── dashboard/   ← Permit cards, ticket grid (types only)
│       ├── research/    ← Read-only explorer mode
│       └── piping/      ← Intent-setting + pipe-to-office flow
│
└── tui/                 # binary crate — user-facing application
    ├── Cargo.toml       # depends on bureau-lib, ratatui, crossterm
    └── src/
        ├── main.rs      ← entry point + CLI parsing
        ├── mod.rs       ← event loop, raw mode, terminal setup
        └── app.rs       ← App state, office switching, command palette
```

**Domain logic in bureau-lib** (no TUI dependencies on this crate):
- `config.rs`, `ticket.rs`, `sandbox.rs`, `agent.rs` — core types and state machines
- `skills/`, `run_config/`, `research/`, `piping/` — domain behavior modules

**User-facing code in tui binary**:
- Ratatui widgets, command palette, sidebar rendering
- Terminal event loop (keyboard input, mouse capture)
- CLI argument parsing + subcommand dispatch

## Consequences

### Positive
1. **Clean dependency boundary**: `use bureau_lib::config` works without pulling ratatui/crossterm into your build deps.
2. **Testability**: Unit tests against domain logic (sandbox, tickets config) don't need TUI dependencies.
3. **Extensibility**: A future CLI-only or API consumer can use bureau-lib without any terminal code.
4. **Encapsulation discipline**: Domain logic must not import from tui — this is enforced by Cargo at the crate boundary level.

### Negative
1. **More files to manage**: Three Cargo.toml files instead of one; workspace config for deps that both crates share.
2. **Path changes**: All existing `use crate::` references in TUI code become `use bureau_lib::`. This is a multi-file refactor (the P0 task).
3. **Build times slightly longer**: More crates = more compile units in release mode; negligible at this file count.

## Alternatives Considered

| Alternative | Rejected Because |
|---|---|
| Single binary with `[lib]` path added to existing Cargo.toml (Option B) | Domain logic mixes directly with TUI code in same `Cargo.toml`; no crate-level dep enforcement between lib and bin targets; future consumers can't use domain types without pulling in ratatui. |
