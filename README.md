# Bureau — Multi-Agent Project Governance TUI

A Rust TUI application (ratatui + crossterm) that manages AI-assisted project work through a structured **bureau** of agent modes. Each "office" handles the full lifecycle: bureaucrant interviews → architect plans → workers execute → inspector validates → archivist summarizes and closes.

## Goals

- **Structured AI governance**: Every agent's scope is explicitly defined in YAML frontmatter permits, with a sandbox enforcing strict file access boundaries (prefix patterns + exact paths).
- **Multi-office management**: A sidebar + command-palette (`Ctrl+P`) for switching between offices/projects. Each office has its own `.bureau/` config and ticket history preserved in phase-specific folders.
- **Checkpoint/resume agents**: Agents save partial progress to JSONL logs so no work is lost on API failures or timeouts. Granular recovery (resume from the Nth turn).
- **Human-in-the-loop control**: Workers request scope expansion through official channels with justification — the user reviews and approves in the TUI (summary by default, diff view expandable via keystroke).

## Architecture decisions

All settled-from grilling are recorded in [ADR 0001](docs/adr/0001-bureau-architecture.md):

| Section | Key decision |
|---|---|
| **Office structure** | Long-lived app with per-project offices (`~/.bureau/config.yaml` global index + per-office `.bureau/config`) |
| **Ticket system** | YAML frontmatter + Markdown body, named `001-interview.md` convention |
| **Lifecycle phases** | Interview → Plan → Execution → Inspection → Archival (phase-specific folders preserved as history) |
| **State machine** | Type-specific transitional states (`needs_more_info`, `review_requested`, etc.) |
| **Authority model** | Strict hierarchy + inspector rollback to planning via direct ticket annotation (`@inspection_fail`) |
| **Agent execution** | LLM via HTTP API + checkpoint/resume with JSONL history per ticket |
| **Sandbox** | Per-permit allow-list (prefix + exact paths), scope expansion request bubbles up through chain → user approval with diff UI |
| **Concurrency** | Fan-out/fan-in with barriers within an office: workers concurrent, modes sequential gates |
| **UX** | Sidebar + command palette (`Ctrl+P`) for fast switching between offices and modes |

## Research Mode & Secrets Protection (Q26–Q33)

A sixth bureau mode available globally (not scoped to any office):

### Research Mode (D15/Q26/C)
- **Sandbox-unrestricted read-only agent**: full filesystem access, zero write tools. Explores any codebase, API docs, or system configuration freely.
- **Deny-pattern enforcement** (`~/.bureau/config.yaml`): application-level list blocks ALL agents (not just workers) from reading `.env*`, `/secrets/`, `/*.pem`, `*.key`, `*.crt`, etc. This is a hard security boundary — no agent may bypass it even with per-permit authorization.
- **Session persistence** under `~/.bureau/research/sessions/{adjective-noun}/` (Q28/C): auto-save every turn during exploration, plus user can explicitly mark sessions as "important" to prevent automatic TTL-based cleanup. Archived by the archivist mode later.
- **Configurable escape behavior** (Q30/config option): `q` = auto-save with prompted default name (random adjective+noun) vs `Q` = quit without saving.

### Pipe-to-Office Workflow (Q27/B+C + Q32/B Intent-Setting)
When user pipes research results to an office, they choose:
1. **Existing Office**: pipe notes into it, optionally triggering bureaucrant review of that content
2. **New Office**: create a new bureau pipeline from this research — triggers full bureaucratic governance (permit issuance → planning → execution)

Both modes FORCE focus-intent-setting (Q32/B): the user must specify *what the new or existing office should govern*. This prevents silent data dumping into bureaucracy — piping always has intentional direction for the bureacratic process.

### Secret Unlock Mechanism (Q33/A/C/D)
- **Default deny**: no agent can read any `.env`/secret files automatically
- **Per-permit unlock**: workers request access via `@secret_unlock: [path]` annotation scoped to their permit → user approves → only that permit's scope gets updated
- No global secret access changes. Sensitive file access is always tightly coupled to specific bureaucratic work items under user supervision.

## Project structure

```
bureau/
├── Cargo.toml                    # Dependencies: ratatui, crossterm, tokio, serde_yaml
├── src/
│   ├── main.rs                   # Entrypoint — init TUI + load global config  
│   ├── config.rs                 # Global `~/.bureau/config` + per-office config loading
│   │                               # + PipeTarget enum (Q27/B+C pipe-to-office destinations)
│   ├── ticket.rs                 # Ticket types, schema state machine, checkpoint protocol
│   ├── office.rs                 # Office management: create/switch/lifecycle tracking
│   ├── agent.rs                  # Agent loop + fan-out/fan-in barrier coordination
│   ├── provider.rs               # LLM chat abstraction (OpenAI-compatible API)
│   ├── sandbox.rs                # Permit-authorized file access + scope expansion handling
│   ├── research/                 # Q26: Sandbox-unrestricted read-only exploration mode
│   │   └── mod.rs                # ResearchSession, deny-pattern enforcement, persistence
│   ├── piping/                   # Q27/B+C: pipe-to-office workflow with intent-setting (Q32/B)
│   │   └── mod.rs                # PipeTarget enum, PipelineChoice UX data structures  
│   └── tui/
│       ├── mod.rs                # Terminal init + event loop entry-point for ratatui
│       └── app.rs               # App state: active office tracking + mode + command palette 
├── docs/
│   └── adr/
│       └── 0001-bureau-architecture.md    # All architecture decisions from grilling
├── .bureau/
│   ├── config.yaml.example                  # Template configs (per-office + global)
└── README.md                                # This file — complete architecture overview
```

## Development status

✅ **Architectural skeleton complete** — all 33 design decisions resolved &amp; recorded.  
🟡 Core types defined: ticket schemas, office config, sandbox API, checkpoint protocol, research session + pipe workflow UX data.  
🟡 Research mode scaffolding: session persistence under ~/.bureau/research/{session}/, global deny-pattern enforcement for secrets (Q33/A).

## Next steps (TODO)

- [ ] Implement full agent LLM loop — message sending, tool call interception, checkpointing + resume logic
- [ ] Complete sandbox enforcement — prefix/regex pattern matching, diff generation for scope expansion  
- [ ] Build ratatui application — sidebar component rendering, command palette input/filtering, tab layout option
- [ ] Implement fan-out/fan-in barrier coordination in Rust (JoinHandle tracking per office) 
- [ ] Inspector inline annotation format (`@inspection_fail: [reason]`) and archivist validation audit pass  
- [ ] Per-office multi-provider LLM support (planned after initial global single-provider mode — D22 Roadmap).
- [ ] Implement research agent read-only API with full filesystem access + deny-pattern enforcement

## Building

```bash
cargo build  # requires rustc ≥ 1.75, ratatui, crossterm, tokio...
```

