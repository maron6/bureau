# ADR 4: Office Language — Manual Input, No Auto-Detection

**Status**: Accepted  
**Date**: Post-grilling session Round 2 (Q3/Q10)  
**Context**: Original plan (D17 in ADR 0001) proposed auto-detecting programming language via pattern checking (Cargo.toml, tsconfig.json, pyproject.toml). Grilling round changed this.

## Problem

- How should the primary programming language of an office be determined?
- Should bureau detect it automatically from project files, or require manual specification?
- Does the language setting gate which skills load for that office?

## Decision

Language is **manually specified by the user** during office creation (`bureau new-office` prompt), stored in `OfficeConfig.language` as a free-form string (e.g., `"Rust"`, `"TypeScript"`, `"Python"`). No auto-detection. The language field is **metadata only** — it does not gate which skills load, nor does it trigger any tooling setup.

```rust
pub struct OfficeConfig {
    // ... existing fields from D17 ...
    pub language: Option<String>,  // manually set; no detection
}
```

The language value appears in the agent's system prompt (from `assemble_system_prompt()`) so agents know what to assume about the codebase when reasoning. It is not used for:
- Auto-generating `.bureau/flake.nix` (still manual or wizard-guided)
- Selecting which skills load
- Enforcing any build or toolchain behavior

### Agent Path Conventions

Bureau agents MUST prioritize **relative file paths** — all paths should be relative to the base of the configured office directory. For example:
- ✅ `.config/foo.md` (relative to office)
- ❌ `/Users/name/.bureau/skills/test.md` (absolute, filesystem-root path)
- ⚠️ `./src/main.rs` — acceptable but prefer without leading dot: `src/main.rs`

This convention keeps agent references portable and avoids leaking local filesystem paths across machines. It is documented in the system prompt assembled by the skills module.

## Consequences

### Positive
1. **No false positives**: Manual spec avoids misidentifying a Rust project as TypeScript just because both have package files.
2. **Simpler codebase**: No `detect_language()` function, no pattern-checking logic for Cargo.toml/tsconfig.pyproject.toml/etc.
3. **Explicit is better than implicit**: Users who create an office know what language they're using — this matches user intent.
4. **Metadata-only**: Language doesn't affect loading, permissions, or behavior beyond appearing in the agent's system prompt. No gates based on it.

### Negative
1. **Manual friction**: Every new office requires one extra input during creation. Mitigated by defaults from global config (e.g., a common language cached in `GlobalConfig`).
2. **No cross-office analysis**: Without auto-detection, bureau can't compare languages across offices for dashboard grouping (but this is an edge case solved manually if needed).

## Alternatives Considered

| Alternative | Rejected Because |
|---|---|
| Auto-detect via Cargo.toml, package.json, pyproject.toml checks | Becomes its own sub-project; monorepos break single-language assumptions; user might know better about their project's architecture than heuristics can infer. Deferred entirely per Q3 grilling resolution. |
| Language gates skill loading (Rust office only loads Rust skills) | Overly restrictive: cross-language skills are valid; user placement decisions (shared/ vs {mode}/ dirs) already handle this without code-level gating. |
