# T00 / H00 Implementation Status

Status: **H00 tooling scaffold only — H00 is not complete.**
Owner path: `tools/torture/**` (plus this note). This does not alter
[T00_GCC_TORTURE_GATE.md](T00_GCC_TORTURE_GATE.md), the compiler contract, the
framework, or CI.

## Implemented

A standalone, offline verifier crate `tools/torture` (library + CLI,
`cc-silicon-torture`) that checks an explicitly supplied local corpus lock against
an explicitly supplied local asset tree. It:

- parses and structurally validates the `cc-silicon.torture/lock/v1` lock,
  including exact GCC release/full commit SHA, source-archive digest, per-asset
  suite/test/aux/header/runtime/driver digests, original license and provenance,
  role, optional size, target identity, execution substrate, corpus policy, and
  reference-baseline policy;
- requires the source archive to be an explicit, cross-linked
  `asset role="archive"` record (path + digest must match
  `release.source-archive*`), so it cannot bypass license/provenance checks,
  duplicate-path detection, or double-counted verification;
- requires a frozen lock to declare all three T00 baseline suites
  (`compile`, `execute`, `ieee`) and to state dialect and option matrix
  explicitly rather than by assumption;
- rejects malformed identifiers and digests, unknown records/fields/roles,
  duplicate fields, and duplicate paths (no silent omission of unknown entries);
- rejects unsafe relative paths (absolute, `..`, backslash, `:`, control chars,
  `.`/empty components, trailing `/`) and symlinks that resolve outside the corpus
  root;
- verifies file presence, size, and SHA-256 from a single opened handle, and
  optionally reports unlisted tree files with **fail-closed** enumeration;
- escapes control characters in the rendered report and reads `args_os` so
  hostile names cannot forge diagnostic lines or panic the CLI;
- emits deterministic, sorted, timestamp-free diagnostics and documented exit
  codes (`0` verified frozen, `1` failed, `2` usage/operational, `3` draft/unfrozen).

Encoded policy expectations: corpus assets are **fetch-on-demand and hash-locked,
never vendored**; a **reference-only DejaGnu baseline** is authorized before
candidate implementation, labeled `non-candidate`, and must keep
`used-for-candidate-pass-rate=false`; the target identity is frozen as
**AArch64 GNU/Linux ELF LP64 little-endian AAPCS64**; the execution substrate is
**Linux CI/VM**.

Tests are synthetic and generated at runtime; no GCC corpus content, GPL
material, or vendored asset is present in this repository.

## Researched official-source facts (recorded, not locally verified)

The following are **recorded metadata from official sources**, supplied for H00
research. They are not locally fetched or verified by this scaffold, and **must
not be used to freeze a lock** or to claim H00 completion. No download, clone,
signature verification, or external command was performed.

| Fact | Value | Source |
|---|---|---|
| Tag | `releases/gcc-15.2.0` (signed annotated) | gcc.gnu.org gitweb tag page |
| Commit | `5115c7e447fc07457443df874bf57840e8316d5f` | gcc.gnu.org gitweb commit page |
| Release date | 2025-08-08 | official release page |
| `gcc-15.2.0.tar.xz` SHA-512 | `89047a2e07bd9da265b507b516ed3635adb17491c7f4f67cf090f0bd5b3fc7f2ee6e4cc4008beef7ca884b6b71dffe2bb652b21f01a702e17b468cca2d10b2de` | official `sha512.sum` |
| `gcc-15.2.0.tar.xz` SHA-256 | not published | official `sha512.sum` |
| Detached signature | exists; tag is signed | official release page |

Additional constraints recorded for later owners:

- **Per-file license varies.** Do not assume a single blanket license for the GCC
  testsuite; every asset must record its own license/provenance.
- **DejaGnu** official prerequisite minimum is 1.5.3; the exact Expect/Tcl
  versions are not pinned (H01 input).
- The lock format now represents a published `source-archive-sha512` alongside a
  locally fetched `source-archive-sha256`, plus an optional detached-signature
  asset. Signature verification is out of scope for this tool.

## Security-review hardening applied

Fail-closed tree enumeration; archive/license cross-linking; strict quoted-value
tokenizer separator; single root error on canonicalization failure; control-char
escaping in reports; non-UTF-8-safe CLI; narrow placeholder sentinels; and a
documented TOCTOU/trust assumption (the tree is assumed quiescent; hashing uses a
single opened handle). See the tool README for the full rule set.

Second-audit items are closed with conservative fail-closed behavior and tests:

- an empty `profile.configuration` list warns on a draft and errors on a frozen
  lock;
- bare known records with no fields are malformed (`lock.record.empty`), never
  silently ignored;
- a frozen lock must record the published `release.source-archive-sha512` and a
  matching archive-asset `sha512`;
- Unicode U+2028/U+2029 are escaped in reports so they cannot forge lines;
- `--check-unlisted` traversal is bounded (`--max-unlisted-entries`, default
  1,000,000) and exceeding the bound is an error;
- two asset paths resolving to the same canonical in-root file are rejected
  (`asset.canonical-duplicate`);
- supplied optional `sha512`/`size` markers are rejected on frozen locks.

The tool never claims that recorded official release metadata (tag, date,
published SHA-512, signature presence) was locally verified; those remain
recorded facts, and only local file bytes are hashed. Signature verification is
still out of scope.

## Intentionally not done / blocked

- **No authorized frozen lock exists.** H00 requires a real GCC corpus fetch and
  legal sign-off; neither has occurred. A real lock therefore stays `draft`, and
  the tool returns exit `3`, never `0`.
- **Legal truth is out of scope.** The tool checks that license/provenance records
  exist and that provenance embeds a revision-like token; it cannot confirm the
  recorded license is correct or that redistribution is lawful. Human legal
  review remains required before any freeze.
- **Runner/toolchain are unresolved (H01).** `substrate.runner`, `toolchain`,
  `sysroot`, `assembler`, and `linker` must be filled before any lock may freeze.
  The tool enforces this.
- **No download, fetch, or generation step.** This scaffold verifies a supplied
  lock; it does not produce one and never touches the network.
- **No GCC/Clang/DejaGnu execution and no pass-rate output.** Reference-baseline
  orchestration is future H02 work.
- **Not wired into CI.** `.github/workflows/ci.yml` is out of scope for this task
  and was not modified. The package is standalone like `tools/chip-lint`.

## Validation evidence

Run from the repository root:

```sh
cargo fmt --manifest-path tools/torture/Cargo.toml -- --check
cargo clippy --manifest-path tools/torture/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/torture/Cargo.toml
git diff --check
```

Results are recorded in the handoff message for this change.

## Handoff requirements

1. H00 owner: perform the authorized fetch, populate the asset tree, compute real
   digests, license/provenance records, and the license-distribution policy, then
   freeze the lock — or leave it draft.
2. H01 owner: fill `substrate.*` with the real Linux CI/VM runner, toolchain,
   sysroot, assembler, and linker.
3. Integration owner: decide whether to add the package to CI and whether the
   lock schema needs a shared contract entry. This scaffold does not claim that
   decision.
4. Legal review: confirm the recorded license/provenance per asset and that no
   GPL material is redistributed in the MIT tree. The tool cannot do this.
