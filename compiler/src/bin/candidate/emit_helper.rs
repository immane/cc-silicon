// ============================================================================
// bin/candidate/emit_helper.rs — H04 `-E` terminal host step (Part A).
//
// Host-side orchestration only: dispatches the frozen PP28 worker
// (`TaskKind::PREPROCESS_EMIT`, `PP28_CHIP`) over the final pp-token
// stream (post-conditional, post-expansion, post-directive — the
// `expanded_tokens` the driver already threads through PP05) and returns
// the emitted `Preprocessed` artifact bytes. The chip strips consumed
// directive lines and `Eof` itself, so the caller passes the stream
// through untouched, in payload order.
//
// This is NOT a chip: no manifest, no bus writes, no I/O. File/stdout
// persistence stays in `candidate.rs` (host owns artifact persistence).
// The file is inert until the integrator declares
// `#[path = "candidate/emit_helper.rs"] mod emit_helper;` in
// `candidate.rs` (a plain `mod` would resolve to `src/bin/`, where the
// file would become its own binary target) and performs the co-owned
// wiring (kinds registry, chip registration, route, `-E` flag, emit
// branch); see the H04 report.
// ============================================================================

use super::step;
use cc_silicon_compiler::bus::CompilerBus;
use cc_silicon_compiler::chips::WorkerRegistry;
use cc_silicon_compiler::ids::RecordRef;
use cc_silicon_compiler::manifest::PP28_CHIP;
use cc_silicon_compiler::snapshot::Trace;
use cc_silicon_compiler::task::{Payload, ResultValue, TaskKind};

/// Dispatch PP28 over the final pp-token stream and return the emitted
/// preprocessed bytes.
///
/// Mirrors the driver's `step`/`artifact` convention: bootstrap, drain to
/// quiescence, and read the single `Preprocessed` artifact. Chip failures
/// surface with the same `candidate diagnostic: ...` prefix the driver
/// maps to exit 1; the empty-stream guard is a host invariant (PP04
/// always appends the trailing `Eof`, so a live stream is never empty).
pub(super) fn emit_preprocessed_bytes(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
    trace: &mut Trace,
    final_tokens: Vec<RecordRef>,
) -> Result<Vec<u8>, String> {
    if final_tokens.is_empty() {
        return Err("emit requires at least one pp-token".to_string());
    }
    match step(
        bus,
        workers,
        trace,
        TaskKind::PREPROCESS_EMIT,
        PP28_CHIP,
        Payload::from_refs(final_tokens),
    )? {
        ResultValue::Record(RecordRef::Artifact(id)) => bus
            .arenas
            .artifacts
            .get(id)
            .map(|record| record.bytes.clone())
            .map_err(|_| "emit artifact vanished".to_string()),
        other => Err(format!("expected artifact ref, got {other:?}")),
    }
}
