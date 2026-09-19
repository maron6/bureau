# Work-Log Skill

## Purpose

Record your progress on this ticket as structured prose entries. The work-log enables
traceability for inspectors and resume capability for interrupted agents. Each entry
becomes a permanent record in `{ticket}-worklog.md`.

## Tool

Call `work_log(event=..., body=...)` to log a work-log entry.

### Event Types

| event       | When to use                                         | Icon |
|-------------|-----------------------------------------------------|------|
| `"draft"`   | Reporting what you're *currently* working on        | 📄   |
| `"update"`  | Completed a notable step or sub-task                | 📝   |
| `"note"`    | Observation, decision rationale, or failure reason  | 💡   |
| `"milestone"` | Significant completion (sub-task done, blocker removed) | ⚑   |

### Examples

```python
work_log(event="draft", body="I'm starting to read config.rs to understand the schema types.")
```

```python
work_log(event="update", body="Wrote all config parser types in config/parser.rs (42 functions).")
```

```python
work_log(event="note", body="Noted that the existing code uses snake_case but the spec says camelCase — I'll keep snake_case for now and flag this for review.")
```

```python
work_log(event="milestone", body="Completed config parser. Ready to begin integration tests.")
```

## Format Guidance

- **Be specific**: Include file paths, function names, and line counts where relevant.
- **Keep body concise**: 1–5 sentences per entry is typical. Use bullet points for multi-item items.
- **Cross-reference**: Use `#NNN` notation for related tickets if applicable.
