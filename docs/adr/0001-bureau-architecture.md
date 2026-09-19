# ADR 1: Bureau Architecture — High-Level Design

**Status**: Accepted  
**Date**: June 26, 2025  
**Context**: Design session grilling completed (Q1–Q25)

## Problem

We are building a Rust TUI application called **bureau** — a governance framework for AI-assisted project work. The app manages multiple project "offices," each with documentation, planning, execution, inspection, and archival agents running in structured lifecycles. Every file the agent touches is sandboxed against an explicit allow-list.

The core tension: **agents need freedom to explore** (especially when interviewing or drafting), but **the system must enforce hard boundaries** (sandboxing, scope expansion, cross-ticket authority).

## Decisions

### D1 - Bureau vs Office: Two-Level Model
- **Bureau** is the long-lived application — it persists independently of any project. The global registry (`~/.bureau/config.yaml`) tracks all offices and their statuses.
- **Office** is per-project, living inside `.bureau/` in each project root. Each office has its own config (overriding what it needs from the global), its own tickets/plans/execs directories, and its own agent state.
- An office can live *with* the project or separately; the worksite path is explicitly configured in the office config.

**Consequence**: Multi-office management is first-class. Users maintain a central index of their offices and switch between them with high velocity (sidebar + command palette).

### D2 - Ticket System: YAML Frontmatter + Markdown Body
Each ticket file has:
```yaml
---
id: "001"              # alphanumeric prefix (see D5)
type: interview|plan|execution|inspection|archival
status: draft|in_review|approved|needs_more_info|... (type-specific)
created_by: user|bureaucrat|architect|worker|inspector|archivist
approved_by: str | null
scope: {}              # structured scope definition
tags: []               # categorization
priority: 1-5
---
# Markdown body with free-form documentation...
```

**Consequence**: Parsed programmatically by the Rust TUI for sidebar rendering, status queries, and cross-reference auditing. Editable by humans during user review sessions. Supports structured search without fragile regex.

### D3 - Lifecycle: Phased Ticket Model with Folders by State
The bureau enforces a **ticket lifecycle** ordered across five phases:

1. **Interview** — Bureaucrat interviews user, iterates "relentlessly" until permit is complete
2. **Plan** — Architect takes the permit and crafts implementation plan; defines worker scope boundaries
3. **Execution** — One or more workers execute steps from the plan
4. **Inspection** — Inspector verifies implementation against requirements; can roll back to planning
5. **Archival** — Archivist validates cross-references, generates summaries, archives office

Within each phase: `draft → in_review → approved` with type-specific transitional states (e.g., interview: `needs_more_info`).

Each phase has its own folder under the office home: `tickets/`, `plans/`, `executions/`, `inspections/`, `archive/`. Historical preservation by design — past tickets are *never* deleted.

**Consequence**: Folder structure is stable and machine-readable. Any office can be inspected for completeness by scanning these directories. History is preserved without version control overhead.

### D4 - Authority Model: Strict Hierarchy + Inspector Rollback
- Bureaucrat → Architect → Workers → Inspector → Archivist is a **strict hierarchy**. No lower-tier can invalidate higher-tier work without user approval.
- **Exception**: Inspector can annotate execution tickets directly with `@inspection_fail: [reason]` to send them back to planning. This creates a feedback loop critical for quality but does not bypass the bureaucrat-permit authority chain.
- User is the ultimate arbiter of scope expansion requests.

**Consequence**: The Rust TUI must enforce this hierarchy — modes are gateable based on which tickets in the pipeline are in what state. Inspector's rollback power creates a real loop where plans can be invalidated without touching permits.

### D5 - Concurrency: Fan-Out/Fan-In with Mode Barriers
- **Per office**: Workers (execution phase) run concurrently, each under their own agent loop and sandbox context.
- The Rust TUI tracks all worker `JoinHandles` per office. An "architect barrier" or "inspector barrier" blocks the next mode's agent until all workers in the previous phase complete.
- Architect interviews, bureaucrat interviews: serial (one at a time).

**Consequence**: Memory model is bounded fan-out within an office. Barrier semaphores coordinate mode transitions. This is NOT fully async — modes are sequential gates with concurrent workers inside them.

### D6 - Agent Execution: LLM via HTTP + Checkpoint/Resume
Each agent is a Rust loop that:
1. Loads the ticket being worked on and its parent permit (if any)
2. Loads its checkpoint and history log from `office/checkpoints/<agent_id>.json` + `.history.jsonl`
3. Calls LLM provider via HTTP (OpenAI-compatible API)
4. Intercepts tool calls through an **approval proxy** — Rust controls every write before it reaches disk
5. On failure: saves checkpoint to a JSONL history, restarts the agent loop with context appended

Checkpoint format per turn:
```json
{
  "ticket_id": "003-execution",
  "agent_type": "worker",
  "current_tool_calls": ["write"],
  "partial_content_markdown": "...draft so far...",
  "context_snapshot": {"scope": "...", "requirement_ref": "#section-2"},
  "turn_count": 14,
  "next_action_hint": "continue drafting section 3 of the plan"
}
```

History JSONL stores full turn-by-turn log for complete recovery. No agent loses work on crash/timeout/API error.

**Consequence**: Rust loop has checkpointing overhead per turn (~5ms serial write). But it supports granular recovery (resume from Nth turn, not just "restart"). Checkpoint dir is part of the office config layout.

### D7 - Sandbox: Permit-Authorized File Access
The permit (interview ticket) authorizes all operations downstream:

- **Allow list format**: Each ticket has `<prefix_pattern>` + exact path entries in the permit's `scope` field.
  ```yaml
  scope:
    allow_read: [".bureau/plans/*", "src/engine/"]
    allow_write: [".bureau/tickets/output/*.md"]
    deny_default: true    # no writes outside explicit allow list
  ```
- **Exact path**: `src/engine/lib.rs` — single file access.
- **Prefix pattern**: `.bureau/plans/*` — glob-style prefix matching.

Workers request scope expansion by adding `@scope_request` annotation to their ticket, which bubbles up through the authority chain (per-ticket, Q13) until it reaches user approval via ratatui with diff view (summary by default, expandable to show "was → now").

**Consequence**: Every agent write passes through a validation gate. The agent can *ask* for more scope but can never grant itself access. The permit is immutable once approved — workers request changes through the official channel. Rust sidecar is minimal because the LLM calls are proxied (not spawned as sub-processes).

### D8 - Inspector: Direct Annotation of Execution Tickets
Inspector's sole role is to review and approve implementation quality, not produce separate artifacts. When inspection fails:
- Inspector directly edits the execution ticket file with `@inspection_fail: [reason]` annotations.
- The annotated ticket can then be re-submitted as a new plan iteration by the architect (who reads these annotations as required fixes).

**Consequence** no separate report folder is needed. Inspector's allow scope must include write access to execution tickets. Quality data stays collocated with the artifact being reviewed, making cross-reference simpler.

### D9 - Archivist: Full Validation + Summary
The archivist performs three concrete duties:
1. **Cross-reference audit**: Every plan links back to its permit; every execution links to its parent plan; no orphaned tickets exist.
2. **Summary generation**: Compressed one-pager of the office's history — key decisions, permits issued, plans adopted, execution count, inspection pass/fail rates.
3. **Orphan check**: Any tickets not properly referenced by at least one downstream artifact.

The archivist ticket gets comprehensive read access to the entire office. Validation gate before "office complete" status transition.

**Consequence**: Archival is not purely cosmetic — it's a meaningful quality gate. An office stays in `archiving` status until the archivist passes its audit. This ensures future readers (including humans) can reconstruct what happened without hunting through ticket history.

### D10 - UX: Sidebar + Command Palette
- **Sidebar** (`Ctrl+S`): Lists all offices with status badges, workspace type indicator. Spatial orientation similar to VS Code Explorer.
- **Command palette** (`Ctrl+P` or `F1`): Type to filter across offices and modes — "switch office", "view plan tickets", "approve scope expansion X". Fast jump capability without mouse.
- **Tab/swipe paradigm**: Available as a configuration option for TUI layout preference (stored in global config).

**Consequence**: TUI has two primary navigation patterns — spatial (sidebar) and search-driven (command palette). This handles both casual browsing and high-velocity power user workflows.

---

## Unsettled / Future Work
- **Per-offer selection model mapping** (`office.providers` list per office, with tickets selecting from it) — currently at global single-provider (D11a), planning for multi-provider per-office support (D11c).
- **Specific LLM API protocol details** — which OpenAI-compatible endpoint, streaming vs non-streaming handling.
- **Checkpoint history retention policy** — how many turns to keep before pruning the JSONL.
- **Error display in ratatui** — how to show agent failures and recovery status clearly to the user.

## Alternatives Considered
1. **Single-ticket office (Q6)**: Rejected. Lifecycle fidelity requires multiple documents through each phase.
2. **Default-deny with `@allow` comments in files (Q10)**: Rejected as insufficient — permit-authorization is the source of truth, not inline file comments.
3. **Separate inspector report instead of direct annotation (Q17a)**: Rejected because it adds folder complexity without benefit; collocated annotations are easier to cross-reference and maintain.
4. **Fully async concurrency (Q18c)**: Rejected for initial version — fan-out/fan-in with barriers is sufficient for the use case and simplifies memory model significantly.
5. **Archivist as file mover only (Q20a/b)**: Rejected. An archivist that doesn't validate completeness is just a script doing work the user could do manually.

## Addendum: Research Mode & Secrets Protection (Q26–Q33)

A sixth bureau mode, available globally (not scoped to any office):

### D12 - Research Mode
Research is a sandbox-unrestricted **read-only** agent with full filesystem access. It has **no write tools** — it can only read files and return their contents to the user. This lets the user freely explore any codebase, API docs, or system configuration without accidentally modifying anything.

The TUI shows research as:
- A sidebar section visible alongside office listings (Decision Q29/C)
- A quick-entry overlay via `Ctrl+R` from any active mode — no context-switching to full mode needed

Research sessions auto-save under `~/.bureau/research/sessions/` for recovery (Q28/B+C). The user can also explicitly mark a session as important so it isn't cleaned up by automated housekeeping.

### D13 - Research → Office Pipe
When the user pipes research results to an office, they choose one of:
- **Trigger bureaucrat on new topic**: Research summary + context are fed to the bureaucrant in a *new* office that gets created for this purpose
- **Save to existing office**: Research notes go into an existing office's workspace as input material

Both options require intent-setting (Q32/B): the user must name what the bureaucrat should focus on. This isn't just "save files somewhere" — it forces intention about *what bureaucracy the research will govern*.

### D14 - Escape & Session Management
User config (`~/.bureau/config.yaml`) controls escape behavior for research sessions:
- **Q** (uppercase): Quit without saving (abandon research)
- **q** (lowercase): Auto-save with prompted name — default generated as random adverb + noun combination

This fits different workflow styles. The TUI shows both options clearly during the escape prompt in the command palette panel.

### D15 - Research Read Scope vs Secrets Globally Denied
Research mode has full filesystem read access except for deny-patterns defined in `~/.bureau/config.yaml`:
```yaml
deny_patterns: [".env*", "/secrets/", "/.credentials/{**/,*/", "/*.pem", "*.key", "*.crt"]
```

These rules are application-level, not per-permit. No agent—research or worker—can ever read files matching these patterns without explicit user override at the global config level. This is a hard security boundary.

### D16 - Secret Unlock Mechanism for Workers
Workers can request access to secrets via a special `@secret_unlock: [path]` tool call annotation scoped to their permit. The user must **explicitly** approve this per-permit, which updates that specific permit's allow-list (not the global config). No secret unlock in any agent is possible without explicit user approval tied to a specific bureaucratic work item.

Consequence: Secrets are readable only by workers who have been explicitly authorized by the user within the context of an approved permit. Research cannot read secrets, and it doesn't need to—its role is exploration and summarization, not configuration access.

---

## Addendum: Grilling Session Q27–Q56 (Nix Integration, Run Configs, Skills System, Dashboard UX, Inspection Permissions)

### D17 - Nix Flake Defaults on Office Creation

**Q27/A**: When an office is created in a project that already has `flake.nix`, Bureau detects it and auto-configures to use the project's dev shell as its working directory. It queries `nix flake show` (via `nix flake metadata`) to extract available outputs.

When a project does *not* have `flake.nix`, Bureau offers an optional bootstrap wizard that:

1. Detects the primary language from the interview ticket's metadata (`language: rust|typescript|python|...`)
2. Offers standard dev-shell templates per detected language (e.g., `nixpkgs.rustc + cargo`, `nixpkgs.nodejs`, `nixpkgs.python39`)
3. **Does not** auto-generate outputs in the project's `flake.nix` — that would be a write to an arbitrary repo and outside Bureau's scope.
4. Instead, it offers to create `.bureau/flake.nix` as an *office-isolated* flake overlay if the user opts in.

The generated `devFlakes.nix` lives under `.bureau/` and contains dev-shell derivations for each language the office supports. This file is never committed — it is local to the developer's machine just like their existing `flake.nix`.

**Q27/C consequence**: The interview phase must capture `language: [lang]` metadata as a structured field, not just free text. This drives default tooling and dev-shell selection during office creation.

**Consequence**: Bureau respects the principle of "never write to arbitrary project files." Dev-shell setup is either detected (if flake exists) or confined to `.bureau/`. The user makes all git-impacting decisions explicitly.

### D18 - Skills System: Per-Mode SKILL.md Injection

**Q27/D**: Bureau supports a **skills** system where each mode (bureaucrat, architect, worker, inspector, archivist) can have one or more `SKILL.md` files that inject domain-specific behavior into the agent's system prompt at startup.

**Loading hierarchy** (global → office, user-defined order):
1. `~/.bureau/skills/{mode}/SKILL.md` — global skills, shared across all offices
2. `.bureau/skills/{mode}/SKILL.md` — office-specific skills (project-local)
3. Multiple skills per directory are loaded *alphabetically by filename* for predictable precedence

**Injection behavior** (`Q50/B`):
- Skills files are injected as **read-only context** into the agent's system prompt at loop start, alongside the permit + ticket
- The injection order is: `[global skills → office skills] then [permit scope] then [ticket content]`
- This means global skills provide base behavior, office skills override/refine it per-project, and the permit/ticket provide the active work context
- Skills can declare `required_tools: [...]` or `disabled_tools: [...]` — these are **enforced** by the sandbox during agent execution, not just advisory

**File format** (`Q50/C`):
```yaml
# SKILL.md frontmatter (parsed as YAML)
nmame: "code-review"
description: "Perform structured code review following X standards"
scope_read: ["src/", ".bureau/plans/*"]
disabled_tools: []
```

Skills can also declare **pre-injection hooks** — shell commands that run before any agent loop starts in this mode.

**Consequence**: Bureau becomes extensible without code changes. Users with domain-specific knowledge (e.g., company coding standards, project conventions) can write SKILL.md files and deploy them globally or per-office. Skills compose — multiple skills stack additively into the prompt context.

### D19 - Single-Permit Execution Rules

**Q34**: At most **one permit** may be in the `execution` phase per office at any time. This is a hard constraint enforced by the TUI and the office manager:

- An execution-mode agent loop cannot start if another permit's execution agent is already running
- The inspector barrier between planning and execution prevents double-starting
- If an execution agent crashes mid-run, its permit status reverts to `execution` with `crashed: true`; the user must explicitly acknowledge (UI button) before restarting

**Q35**: **Interviewing new permits is allowed concurrently** with an active execution. The bureaucrat can conduct multiple interviews in parallel — only *execution* is serialized.

**Q36**: Interview state is tracked independently per interview ticket and persists across Bureau restarts. Each interview gets `draft` → `in_review` → `approved`/`needs_more_info`. An approved interview becomes a permit that can enter the execution queue.

**Q37-C consequence**: The user dashboard must show at-a-glance which permits are in which states, especially highlighting the single-execution constraint. Active execution gets a prominent badge; upcoming permits get a queue indicator.

**Consequence**: Execution is intentionally serial within an office to prevent sandbox collisions and scope conflicts between workers. Interview parallelism allows rapid requirement gathering for future work without blocking current development.

### D20 - Run Configurations: Office-Specific Launch Settings

**Q40/A**: Each office can define **run configurations** independently of permits and tickets — this is a user-managed construct, not something the archivist produces.

Run configs are stored in `.bureau/run_configs.yaml` (similar to VS Code's `launch.json`):
```yaml
# .bureau/run_configs.yaml
defaults:
  mode: execution          # default mode when opening office
  provider: openai-gpt4   # preferred LLM provider
run_configs:
  - name: "full pipeline"
    mode: execution
    permit_filter: null    # all permits
    skip_inspection: false
    description: "Run full interview→plan→execution→inspection cycle"
  - name: "inspect only"
    mode: inspection
    permit_filter: "056"  # specific permit ID
    description: "Review latest execution for quality gate"
```

**Properties per configuration** (`Q40/D/E/F`):
- `name`: Display name (used in command palette and dashboard)
- `mode`: Target mode to launch (execution, inspection, interview, archival)
- `permit_filter`: Which permits this config applies to (null = all, string = specific ID)
- `override_scope`: Optional per-permit scope overrides for testing or audit purposes
- `skip_inspection`: UI toggle shown in command palette when launching; when checked, skips the inspection barrier immediately after execution
- `description`: Human-readable explanation shown in command palette hover

The **archivist** can *propose* run configurations tied to specific permits (Q40/F), but the archivist does **not** auto-create or auto-commit them. This is a user-decision workflow — archivist output is advisory only.

Run configs are managed via:
- Command palette: `> bureau: new run config`
- TUI panel within office dashboard view
- Direct file edit (`.bureau/run_configs.yaml`)

**Consequence**: Run configs give users a way to create reusable "workflows" without manually configuring permits and modes each time. They are decoupled from the permit system — no lifecycle tie, just convenience shortcuts.

### D21 - Permission Model: Ticket-Bound Read Scope

**Q53/A**: A worker's read scope is **bound to the ticket in which it operates**, not only to its permit. If a worker holds execution ticket `008-exec` and the inspection passes, that worker gets read scope extending to:
- All tickets that `008-exec` references (direct links like `#003-plan`)
- Permits referenced by `008-exec`

This creates a **ticket-bound permissions model** where each agent's read capability is derived from the specific ticket it is working on, not just the broad permit that initialized the office.

**Q54/A**: Inspector scope follows the same pattern: inspector gets read access to all tickets within the inspection permit's lifecycle (plan + execution tickets). Scope expansion requests for inspectors bypass the architect gatekeeper — they go **directly to user approval** via the TUI permission panel.

Permission toggle behavior in the TUI:
- When a worker requests scope expansion (`@scope_request`), it appears as a named item in the dashboard's right sidebar under "Pending Permissions"
- User can: **Approve** (adds to permit allow-list), **Deny** (rejects, returns request to worker with reason note), or **Delegate** (forwards to bureaucrat/architect per hierarchy)
- Approved permissions are logged in `office/permissions.log` as a JSONL audit trail

**Consequence**: Agents gain read access dynamically based on their ticket's relationships. This means an agent working on execution ticket `008-exec` can read whatever that ticket references (plans, permits), but nothing else — including files the permit technically authorized, unless explicitly linked from its ticket.

### D22 - Dashboard UX: Permit Cards & Ticket Status Grid

**Q49/B**: The dashboard view uses a **card-based layout** for the active permit at the top, followed by a **ticket status grid** below:
```
┌───────────── ACTIVE PERMITS ────────────────┐
│ [permit 045: Implement authentication]       │
│   State: execution     Workers: 3 active      │
│   Plan: 042-plan ✓    Inspection: waiting     │
│   Escalations: 2     Permissions: 1 pending   │
│                                                │
├───────────── TICKET STATUS GRID ─────────────┤
│ Ticket  Type       Status      Workers        │
│ 043-plan plan        approved    -            │
│ 044-exec execution in_progress  ●●○           │
│ 045-exec execution pending     ○             │
│ 046-inspection inspection waiting   -         │
│                                                │
├───────────── WORKERS ─────────────────────────┤
│ W3: writing src/auth.rs ... (47/120)         │
│ W4: reading config.md ... (done)              │
│ W5: inspecting plan-042 ✓                     │
├───────────── PENDING PERMISSIONS ────────────┤
│ [!] scope_expand@044-exec → src/certs/       │
│     → Deny / Approve / Delegate               │
└────────────────────────────────────────────────┘
```

**Active permit cards** at the top:
- Permit name + number from 045-interview (or whichever phase title is chosen)
- Current status badge (`execution`, `planning`, etc.)
- Worker count indicator (number of active worker loops)
- Plan status (✓ approved, ○ pending, ! needs updates)
- Pending escalation/permission count
- **Click to expand** → shows raw YAML permit content with `$EDITOR` toggle option

**Ticket status grid:**
- Columns: ticket number (prefix), type label (plan/execution/etc.), status badge, worker count
- Click any ticket row to open it in a new tab/view
- Dependency tracking via `depends_on: [042-plan]` in the YAML — displayed as thin dependency lines in grid rendering if ratatui supports them
- Archived tickets are **hidden by default** but accessible via toggle in grid header (`Show archived: ON/OFF`)

**Worker progress bars:**
- Rendered as `[=======○...] (47/120)` using bar characters from tick module or custom rendering per ratatui's unicode support
- Shows file-modifying operations when the agent is writing files
- Progress is approximate — based on estimated tokens per phase, not actual token count

**Quick actions in dashboard:**
- `p` — Pause/resume current execution (or permit-wide pause)
- `r` — Restart crashed worker
- `a` — Approve all pending permissions at once
- `/` — Start new interview (bureaucrat)
- Esc/`q` — Return to office list

**Dependency tracking across tickets** (`Q40/B`):
- When a permit references tickets by ID, Bureau displays them as connected nodes in the dependency graph
- A simple ASCII line graph or ratatui's unicode box-drawing characters show:
  `interview → plan → execution → inspection → archival`
- Circular dependencies (ticket A depends on B, B depends on A) are flagged.

**Consequence**: Dashboard gives a complete operational overview: what work is happening, which permits exist, how the pipeline flows, pending decisions needing user action. Agent progress visibility replaces terminal-only workflows with visual feedback.

### D23 - Pipe-to-Office UX Enhancement

**Q51/A**: When piping results to bureau (e.g., from an external editor or CLI), the system must collect:
- **Bureau name**: Which office to pipe to (or create)
- **Intent description**: What the user wants in plain language (required by D3 — bureaucrat interviews until permit is complete)
- **Optional permit ID**: Attach to existing permit vs. create new
- **Mode**: Which mode to launch (bureaucrat, worker, inspection, archival)
- **Target file path** in target office where the piped content lands (`$EDITOR` or write command)`

Example: `echo "review auth implementation" | bureau pipe --intent="Review authentication for security issues" --mode=inspection`

**Consequence**: Pipes become a way to invoke Bureau's workflow from external tools (CLI, editors, CI pipelines) without opening the TUI first.
