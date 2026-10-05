# Wave 2 Slice 1: PP01 Source-Normalize Freeze (`/10`)

| Field | Value |
|---|---|
| Status | **FROZEN 2026-10-05** as `t01-c01-c06/10` (hash `9d2479e7…81a5e9`) |
| Scope | Compiler application (`compiler/`); root framework untouched |
| Authority | T01 integrator serial freeze (R1 auto-bump); `/9` preserved as history |
| Depends on | [T01](T01_COMPILER_CONTRACT.md) §7.1 (`/10` delta), [T03](T03_PREPROCESS_CHIPS.md) rev-44/45/53/54, [M1 vertical](M1_VERTICAL_SLICE_ACCEPTANCE.md) `M1-PP-01`/`01b`/`01c` |
| Non-goal | Splice/comment/macro/include/multi-source, token emission (PP04), spans |

## 1. What this slice is

The first Wave 2 executable slice: one chip (`PP01 SourceNormalizeChip`)
turning one committed source into one single-source `Normalized` artifact
with its raw-boundary map. Fixtures seed the source bytes as committed
records (Host import stays future work); the acceptance fixture is
**`P1-PP-01`** (below), explicitly **not** the full `M1-PP-01`–`M1-PP-08`
chain, which needs PP02–PP04 chips.

## 2. Freeze items

| # | Item | Acceptance |
|---|---|---|
| P1-1 | `ArtifactRecord { kind, source, bytes, raw_offsets }` (rev-44) + total 8 `ArtifactKind` + `requires_map` | struct + snapshot round-trip |
| P1-2 | Mandatory-map invariants (rev-45) + optional-kind empty-offset rule (rev-47) | commit rejects malformed maps before mutation |
| P1-3 | M1 boundary convention (rev-53, now frozen for the exercised scope): identity / inserted-LF zero-width at EOF / CRLF collapse | `M1-PP-01`/`01b`/`01c` exact arrays green |
| P1-4 | `preprocess.normalize` kind (`group 2`, local `16`, `Frozen`) + `pp01_slice()` registry | registry + kind/status tests |
| P1-5 | Stage row (`normalize → 1`) + allowlist row (`PP01_CHIP`, `Artifacts/fragments`, `normalize`) + `PP01_CHIP = ChipId(3)` | stage/allowlist/layer gates green |
| P1-6 | `AppendRecords` materialization for the `Artifact` family (1:1 bodies, map validation, `Normalized`-only, per-arena capacity, predicted refs for `Record`/`Records`) | materialization negatives + capacity + `UnpredictedRecord` |
| P1-7 | Snapshot bodies for the new shape + `ARTIFACT_KIND_NAMES` hash participation | encode round-trip + replay determinism |
| P1-8 | Worker template: `PpNormalizeChip` (`PpInput` projection, pure `compute`, ZST registry, driver stage/layer, lint) | `c10_pp01` 7 tests green |

## 3. Execution record

Implemented on the current branch, verified by `compiler/tests/c10_pp01.rs`
(7 tests: kind/stage/allowlist freeze, pure boundary unit cases, tick
integration for identity/CRLF/insert, bad-payload/source/map negatives,
stage/layer + manifest gates, snapshot replay determinism).

## 4. Explicitly deferred

- `Spliced`/`CommentFree`/`Preprocessed` production (schema-declared, unexercised).
- PP02 splice, PP03 comment removal, macro/include/multi-source.
- PP-token/span emission (PP04 slice), `Preprocessed` artifact production (PP28).
- `max_source_bytes` exact error-code freeze (worker fails with a `Config`
  diagnostic; the code inventory stays open).
- Lone-CR inputs: explicit `Unsupported`, never a silent reinterpretation.
