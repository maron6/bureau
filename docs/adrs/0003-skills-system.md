# ADR 3: Skills System Design

**Status**: Accepted  
**Date**: Post-grilling session Rounds 3–5  
**Context**: Decision D18 from original architecture (per-mode SKILL.md injection). Extended with grilling decisions Q8, Q9–Q15.

## Problem

- How should skills be discovered, loaded, and injected into agent system prompts?
- Should skills declare their own permissions or merely suggest intent that the permission model resolves?
- Where do skills live globally vs per-office, and how does precedence work?
- What format do SKILL.md files use (schema-enforced vs free-form markdown)?

## Decision

### Skill Discovery Paths (alphabetical within each segment)

```
1. ~/.bureau/skills/shared/*.md          → global shared base layer
2. ~/.bureau/skills/{mode}/*.md         → global mode-specific overrides
3. {office}//.bureau/skills/shared/*    → office-level shared (stacks after global)
4. {office}/.bureau/skills/{mode}/*     → office mode-specific (highest precedence)
```

### Merge Strategy: Stacked Precedence

Global loads first (base), then office stacks on top. Latter entries have higher precedence for overlapping field conflicts (`disabled_tools`, `scope_read`). Last-wins semantics within each segment. Skills with the same name across sources are treated as separate skills — no cross-source merging by name.

No deduplication or shadowing. Everything concatenates in order, appended to the system prompt.

### Skill Struct

```rust
pub struct Skill {
    pub name: String,                 // frontmatter 'name:' or directory default
    pub description: String,          // first sentence of body if available
    pub content: String,              // free-form markdown (no section enforcement)
    pub required_tools: Vec<ToolName>,// tools the skill expects available
    pub disabled_tools: Vec<ToolName>,// merged with sandbox deny-list
    pub scope_read: Vec<PathPattern>, // intent → resolved by P4 permission system
}
```

### Name Resolution

Directory name = default. `name:` frontmatter can override for cases where you want one SKILL.md to contribute to an existing skill pack (e.g., `.bureau/skills/shared/fix-lint/SKILL.md` with `name: "linting"` joins the existing linting skill).

### Frontmatter Schema (YAML between `---` delimiters)

| Field | Type | Required? | Default |
|---|---|---|---|
| `name` | String | No | Directory filename |
| `description` | String | No | First sentence of body |
| `scope_read` | `[string]` | No | deny-all (sandbox default) |
| `disabled_tools` | `[string]` | No | empty list |
| `enabled_tools` | `[string]` | No | all tools (not listed in disabled) |

Markdown body is free-form. No section enforcement on the content — designed for human writers, not machine-validated docs. Only YAML frontmatter is programmatic.

### Permission Architecture: Intent Declaration (Option A)

Skills declare intent (`scope_read`, `required_tools`). The permission model (P4) resolves conflicts between what agents want and what they've been granted. This separates concerns: skills express capability needs; the permission system enforces security boundaries.

### Priority Order for Prompt Injection

1. Base context block (mode, workspace path, office language metadata)
2. Global shared skills (alpha-sorted)
3. Office shared skills (alpha-sorted)
4. Mode section header (`# For {MODE} mode:`)
5. Global mode-specific skills (alpha-sorted)
6. Office mode-specific skills (alpha-sorted, highest precedence for tool restrictions)
7. Permission summary block (what agent can/can't read/write)

### Error Handling

| Failure | Response |
|---|---|
| Invalid YAML frontmatter | Skip this SKILL.md + log warning; continue loading others |
| Unknown tool name in `disabled_tools` | Warn at load time; ignore silently (no such tool exists in the agent's toolset) |
| Missing `SKILL.md` in a skills sub-directory | Error — directory must contain SKILL.md to be recognized as a skill dir |

### Caching: None for V1

Fresh load every time. No cache layer needed initially. Future optimization via mtime-based invalidation is trivial but unnecessary at current file counts (< 20 skill files per office).

## Consequences

### Positive
1. **Stacked override model**: Office admins can extend global skills without duplicating them — just add fields that override in frontmatter.
2. **Free-form markdown body**: Low friction for authors; no validation on content structure, only on YAML frontmatter.
3. **No language-gated loading**: Skills are placed explicitly by the user into `shared/` or mode dirs. The office's language field is metadata only.
4. **Deterministic loading**: Alphabetical sorting within each segment ensures predictable behavior across environments.

### Negative
1. **No deduplication**: Two skills with the same `name:` from different sources will duplicate content in the prompt if users accidentally name them identically — relies on discipline from skill authors.
2. **Last-wins ambiguity**: If both global and office declare the same `scope_read: ["src/", ".bureau/"]`, they're not merged — last segment's value wins entirely for that field.

## Alternatives Considered

| Alternative | Rejected Because |
|---|---|
| Schema-enforced markdown sections (like JSDoc) | Too much friction for human writers who just want to document how to use the skill; frontmatter YAML is convention already common in AI agent tooling (Claude, GitHub Copilot) |
| Language-gated skill loading | Adds complexity without value — users control which skills apply via placement into shared/ or mode dirs |
| Full merge of same-name skills across sources | Hard to do correctly when fields are lists vs scalars; stacking is simpler and predictable |
