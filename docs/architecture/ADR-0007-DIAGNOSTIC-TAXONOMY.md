# ADR-0007: Stable Diagnostic Taxonomy

| Field | Value |
|---|---|
| Status | **Accepted (2026-11-09)** |
| Decision owner | T01 integrator |
| Contract artifact | `t01-c01-c06/47` |
| Extends | `/13` diagnostic groups and legacy `Task:4`/`Unsupported:1` codes |
| Non-claims | No full legacy-call-site migration; no new panic path |

## 1. Context

The Stage A audit found ad hoc diagnostic groups and a `Task:4` catch-all used
across hundreds of sites. The existing `DiagnosticDraft` already carries
severity, structured code, message, optional span, and optional task. `/47`
freezes a stable group/code registry and the attribution contract; it does not
pretend every legacy site has been migrated.

## 2. Groups

The `/13` groups remain in order. Append:

```text
lex, parse, type, const, ir, codegen, host
```

Group names are hash-visible and append-only. Existing `semantic`,
`unsupported`, and `internal` remain valid.

## 3. Named code registry

`DIAGNOSTIC_CODES` is the frozen name/group/code table. It includes the legacy
`Task:4`/`Task:3`/`Unsupported:1` codes under explicit legacy names plus new
structured codes for lex, parse, type, semantic, constant, IR, codegen, and
Host faults. Names and `(group, code)` pairs must be unique.

New helper constructors (`missing_record`, `parse_unexpected`, `type_mismatch`,
`ir_invalid`, `host_io`) use named codes. Existing call sites may still use
legacy codes until their owner migrates them; migration is tracked separately
and must not change a test expectation silently.

## 4. Attribution

- `DiagnosticDraft` supports `with_span` and `with_task`.
- Producers should attach the closest committed span and owning task where
  available; absence is allowed only when the fault has no precise span/task.
- Messages must not leak credentials, unrelated files, environment secrets, or
  unrestricted stack traces.
- Capacity/internal faults remain typed records, never panics.

## 5. Rules

New hashed rules:

```text
diag.taxonomy-stable
diag.code-unique
diag.span-task-attribution
```

The registry is hashed as name + group name + numeric code. Numeric code reuse
or a group rename is a contract change.

## 6. Migration policy

1. Add a named code to the registry before migrating call sites.
2. Migrate by owner/slice with tests that assert the new code.
3. Do not mass-replace distinct faults with one code merely to reduce counts.
4. Legacy codes remain valid until the migration row is accepted; no silent
   reclassification.

## 7. Alternatives rejected

| Alternative | Reason |
|---|---|
| String-only diagnostics | Tooling would parse prose; no stable matching |
| One global code | Loses fault class/actionability |
| Renumber existing groups | Breaks frozen snapshots/tests |
| Remove `Task:4` immediately | Massive unreviewed migration with wrong classifications |

## 8. References

- [A18 diagnostic taxonomy audit](../reviews/dispatch/A18_DIAGNOSTIC_TAXONOMY.md)
- [T01 compiler contract](../tasks/T01_COMPILER_CONTRACT.md) §7.1 `/47`
- [T02 control chips](../tasks/T02_CONTROL_CHIPS.md)
