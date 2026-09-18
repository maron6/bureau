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
