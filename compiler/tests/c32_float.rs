// ============================================================================
// c32_float.rs — Wave 3 (`/32`) LX float slice acceptance.
//
// Covers the frozen closure: the `lex.float_syntax` (local 19) and
// `lex.float_value` (local 20) kinds, kind-to-stage assignment (stage 2),
// the LX09/LX10 manifests (Ack-only, zero writes, unchanged allowlist),
// decimal and hex syntax acceptance, malformed-exponent `Invalid`
// failures, suffix-selected formats (binary32/binary64; binary128
// deferred), correctly-rounded halfway-to-even bits, overflow-to-infinity
// with flags, the subnormal grid, bus dispatch (`Ack`), and
// snapshot determinism. The `FloatParts` reconciliation (two local
// shapes, one frozen spelling interchange) is pinned by the
// syntax/value agreement case.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, PpTokenKind, PpTokenRecord, SpanRecord};
use cc_silicon_compiler::chips::{
    convert_float_parts, handler_for, parse_float_parts, spelling_to_value_parts,
    FloatConvertOutcome, FloatValueError, LxFloatSyntaxChip, LxFloatSyntaxInput, LxFloatValueChip,
    LxFloatValueInput, Worker, WorkerRegistry, LX09_TASK_KIND, LX10_TASK_KIND,
};
use cc_silicon_compiler::ids::{ChipId, RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    check_stage_layer_agreement, is_lx_float_slice_kind, stage_of, LX09_CHIP, LX10_CHIP,
    STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::Snapshot;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, FloatFormat, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, Proposal, ResultValue, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry, TaskState,
};

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn install_lx_float(bus: &mut CompilerBus) {
    use cc_silicon_compiler::manifest::StoreSchema;
    bus.kinds = TaskKindRegistry::lx_float_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    for chip in [&LxFloatSyntaxChip as &dyn Worker, &LxFloatValueChip] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(TaskKind::LEX_FLOAT_SYNTAX, LX09_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::LEX_FLOAT_VALUE, LX10_CHIP, 2)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(LxFloatSyntaxChip).unwrap();
    workers.register(LxFloatValueChip).unwrap();
    workers
}

fn bootstrap(bus: &mut CompilerBus, kind: TaskKind, owner: ChipId, payload: Payload) -> TaskId {
    bus.bootstrap_task(TaskDraft {
        kind,
        owner,
        parent: None,
        payload,
        continuation: None,
    })
    .unwrap()
}

fn tick(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
) -> cc_silicon_compiler::routing::TickReport {
    RoutingShell
        .clock_tick_with(&CompilerPins::default(), bus, handler_for(workers))
        .unwrap()
}

/// Seed one `PpNumber` token carrying `spelling`; returns its ref.
fn seed_pp_number(bus: &mut CompilerBus, spelling: &[u8]) -> RecordRef {
    let limits = bus.limits();
    let name = bus.intern_name(b"float.c").unwrap();
    let source = bus.alloc_source(name, spelling.to_vec()).unwrap();
    let span = bus
        .arenas
        .spans
        .alloc(
            SpanRecord {
                source,
                start: 0,
                end: spelling.len() as u64,
                expansion: None,
            },
            &limits,
        )
        .unwrap();
    let token = bus
        .arenas
        .pp_tokens
        .alloc(
            PpTokenRecord {
                kind: PpTokenKind::PpNumber,
                span,
                spelling: spelling.to_vec(),
            },
            &limits,
        )
        .unwrap();
    RecordRef::PpToken(token)
}

fn syntax_input(spelling: &[u8]) -> LxFloatSyntaxInput {
    LxFloatSyntaxInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        spelling: spelling.to_vec(),
    }
}

fn value_input(spelling: &[u8]) -> LxFloatValueInput {
    LxFloatValueInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        spelling: spelling.to_vec(),
    }
}

fn is_ack(proposals: &[Proposal]) -> bool {
    matches!(
        proposals,
        [Proposal::Complete {
            value: ResultValue::Ack,
            ..
        }]
    )
}

fn is_fail(proposals: &[Proposal]) -> bool {
    matches!(proposals, [Proposal::Fail { .. }])
}

#[test]
fn float_kinds_stage_registry_manifest_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(TaskKind::LEX_FLOAT_SYNTAX.group(), TaskGroup::LEX);
    assert_eq!(TaskKind::LEX_FLOAT_SYNTAX.local(), 19);
    assert_eq!(TaskKind::LEX_FLOAT_VALUE.group(), TaskGroup::LEX);
    assert_eq!(TaskKind::LEX_FLOAT_VALUE.local(), 20);
    assert_eq!(LX09_TASK_KIND, TaskKind::LEX_FLOAT_SYNTAX);
    assert_eq!(LX10_TASK_KIND, TaskKind::LEX_FLOAT_VALUE);
    assert!(is_lx_float_slice_kind(TaskKind::LEX_FLOAT_SYNTAX));
    assert!(is_lx_float_slice_kind(TaskKind::LEX_FLOAT_VALUE));
    assert!(!is_lx_float_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_lx_float_slice_kind(TaskKind::LEX_DECODE_LITERAL));
    assert_eq!(stage_of(TaskKind::LEX_FLOAT_SYNTAX), Some(2));
    assert_eq!(stage_of(TaskKind::LEX_FLOAT_VALUE), Some(2));
    // The float registry extends the `/31` head linearly; `LEX` owners
    // start new codes at local 21.
    let head = TaskKindRegistry::pp_emit_slice();
    assert_eq!(head.len(), 48);
    let registry = TaskKindRegistry::lx_float_slice();
    assert_eq!(registry.len(), 50);
    assert_eq!(
        registry.lookup(TaskKind::LEX_FLOAT_SYNTAX).unwrap().name,
        "lex.float_syntax"
    );
    assert_eq!(
        registry.lookup(TaskKind::LEX_FLOAT_VALUE).unwrap().name,
        "lex.float_value"
    );
    assert_eq!(stage_of(TaskKind::new(TaskGroup::LEX, 21).unwrap()), None);
    // Ack-only manifests: zero writes, so the allowlist is unchanged.
    assert_eq!(LX09_CHIP, ChipId(39));
    assert_eq!(LX10_CHIP, ChipId(40));
    assert_eq!(STORE_OWNER_ALLOWLIST.len(), 30);
    for manifest in [LxFloatSyntaxChip.manifest(), LxFloatValueChip.manifest()] {
        assert!(manifest.writes.is_empty());
        assert!(manifest.deterministic);
        assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
        assert!(manifest.declares_read(StoreId::Pp, "tokens"));
        assert!(check_stage_layer_agreement(&manifest, &{
            let mut bus = new_bus();
            install_lx_float(&mut bus);
            bus.routing
        })
        .is_ok());
    }
    assert_eq!(
        LxFloatSyntaxChip.manifest().tests,
        vec!["compiler/tests/c32_float.rs"]
    );
    assert_eq!(
        LxFloatValueChip.manifest().tests,
        vec!["compiler/tests/c32_float.rs"]
    );
}

#[test]
fn decimal_syntax_accepts() {
    for spelling in [
        b"1.5".as_slice(),
        b"1.",
        b".5",
        b"1e+10",
        b"1E-5",
        b".5e2",
        b"1.5f",
        b"1.5F",
        b"1.5l",
        b"1.5L",
    ] {
        assert!(
            parse_float_parts(spelling).is_ok(),
            "LX09 rejects `{}`",
            String::from_utf8_lossy(spelling)
        );
        assert!(
            is_ack(&LxFloatSyntaxChip.compute(&syntax_input(spelling))),
            "LX09 compute rejects `{}`",
            String::from_utf8_lossy(spelling)
        );
    }
}

#[test]
fn hex_syntax_accepts() {
    for spelling in [
        b"0x1.fp+2".as_slice(),
        b"0x1p-1",
        b"0XABCp+10",
        b"0x.8p0",
        b"0x1P+2F",
    ] {
        assert!(
            parse_float_parts(spelling).is_ok(),
            "LX09 rejects `{}`",
            String::from_utf8_lossy(spelling)
        );
        assert!(
            is_ack(&LxFloatSyntaxChip.compute(&syntax_input(spelling))),
            "LX09 compute rejects `{}`",
            String::from_utf8_lossy(spelling)
        );
    }
}

#[test]
fn bad_exponent_fails_invalid() {
    // Malformed exponents, hex without a binary exponent, and
    // integer-shaped spellings (LX05–LX08 territory) all fail loud.
    for spelling in [
        b"1e".as_slice(),
        b"1e+",
        b"1e+foo",
        b"1E-",
        b"0x1",
        b"0x1.f",
        b"0x.",
        b"0x1p",
        b"0x1p+",
        b"42",
        b"0",
        b"0x10",
        b"",
    ] {
        assert!(
            parse_float_parts(spelling).is_err(),
            "LX09 accepts `{}`",
            String::from_utf8_lossy(spelling)
        );
        assert!(
            is_fail(&LxFloatSyntaxChip.compute(&syntax_input(spelling))),
            "LX09 compute accepts `{}`",
            String::from_utf8_lossy(spelling)
        );
    }
}

#[test]
fn bad_suffix_fails() {
    for spelling in [b"1.5ll".as_slice(), b"1.0i", b"0x1p+2u", b"1.5d", b"1e10j"] {
        assert!(
            parse_float_parts(spelling).is_err(),
            "LX09 accepts `{}`",
            String::from_utf8_lossy(spelling)
        );
        assert!(
            is_fail(&LxFloatSyntaxChip.compute(&syntax_input(spelling))),
            "LX09 compute accepts `{}`",
            String::from_utf8_lossy(spelling)
        );
    }
}

#[test]
fn suffix_selects_format_with_long_double_deferred() {
    let (_, f32) = spelling_to_value_parts(b"2.5f").expect("f suffix parses");
    assert_eq!(f32, FloatFormat::IeeeBinary32);
    let (_, f32_upper) = spelling_to_value_parts(b"2.5F").expect("F suffix parses");
    assert_eq!(f32_upper, FloatFormat::IeeeBinary32);
    let (_, f64) = spelling_to_value_parts(b"2.5").expect("bare spelling parses");
    assert_eq!(f64, FloatFormat::IeeeBinary64);
    assert_eq!(
        spelling_to_value_parts(b"2.5l"),
        Err(FloatValueError::DeferredFormat)
    );
    // Bus level: `f` converts, `l` fails as explicit `Unsupported`.
    assert!(is_ack(&LxFloatValueChip.compute(&value_input(b"2.5f"))));
    assert!(is_ack(&LxFloatValueChip.compute(&value_input(b"2.5"))));
    assert!(is_fail(&LxFloatValueChip.compute(&value_input(b"2.5l"))));
    assert!(is_fail(&LxFloatValueChip.compute(&value_input(b"42"))));
    assert!(is_fail(&LxFloatValueChip.compute(&value_input(b"1e"))));
}

#[test]
fn halfway_rounds_to_even_with_known_bits() {
    // 16777217 = 2^24 + 1: halfway between 2^24 and 2^24+2 → even 2^24.
    assert_eq!(
        spelling_to_value_parts(b"16777217"),
        Err(FloatValueError::NotFloat)
    );
    let parts = spelling_to_value_parts(b"16777217.0")
        .expect("float-shaped")
        .0;
    let out = convert_float_parts(&parts, FloatFormat::IeeeBinary32).expect("converts");
    assert_eq!(out.bits, 0x4B80_0000);
    assert!(out.inexact);
    assert_eq!(
        out,
        FloatConvertOutcome {
            bits: 0x4B80_0000,
            overflow: false,
            underflow: false,
            inexact: true,
        }
    );
    // 16777219: halfway between 16777218 (odd) and 16777220 (even).
    let parts = spelling_to_value_parts(b"16777219.0")
        .expect("float-shaped")
        .0;
    let out = convert_float_parts(&parts, FloatFormat::IeeeBinary32).expect("converts");
    assert_eq!(out.bits, 0x4B80_0002);
    // One tenth is inexact with trusted-oracle bit patterns.
    let tenth = spelling_to_value_parts(b"0.1").expect("tenth parses").0;
    let f = convert_float_parts(&tenth, FloatFormat::IeeeBinary32).expect("converts");
    assert_eq!(f.bits, 0x3DCC_CCCD);
    assert!(f.inexact);
    let d = convert_float_parts(&tenth, FloatFormat::IeeeBinary64).expect("converts");
    assert_eq!(d.bits, 0x3FB9_9999_9999_999A);
    assert!(d.inexact);
}

#[test]
fn overflow_yields_infinity_and_still_acks() {
    let parts = spelling_to_value_parts(b"1e400").expect("parses").0;
    let out = convert_float_parts(&parts, FloatFormat::IeeeBinary64).expect("converts");
    assert_eq!(out.bits, 0x7FF0_0000_0000_0000);
    assert!(out.overflow && out.inexact && !out.underflow);
    // Range outcomes ride as flags, never as value-level failures.
    assert!(is_ack(&LxFloatValueChip.compute(&value_input(b"1e400"))));
    assert!(is_ack(&LxFloatSyntaxChip.compute(&syntax_input(b"1e400"))));
}

#[test]
fn subnormal_grid_rounds_with_underflow_flag() {
    // 0x0.0000000000001p-1022 = 2^-1074: smallest subnormal, exact.
    let parts = spelling_to_value_parts(b"0x0.0000000000001p-1022")
        .expect("parses")
        .0;
    let out = convert_float_parts(&parts, FloatFormat::IeeeBinary64).expect("converts");
    assert_eq!(out.bits, 1);
    assert!(!out.inexact && !out.underflow);
    // Half the least subnormal rounds to even (zero), inexact.
    let parts = spelling_to_value_parts(b"0x0.8p-1074").expect("parses").0;
    let out = convert_float_parts(&parts, FloatFormat::IeeeBinary64).expect("converts");
    assert_eq!(out.bits, 0);
    assert!(out.inexact && out.underflow && !out.overflow);
    // A tiny decimal underflows to zero with flags — still `Ack` on the bus.
    assert!(is_ack(&LxFloatValueChip.compute(&value_input(b"1.0e-400"))));
}

#[test]
fn syntax_value_agreement_over_corpus() {
    // The frozen interchange is the spelling: LX09 accept ⟺ LX10 parse
    // accept, with identical `Invalid` messages on malformed shapes.
    // (The two `FloatParts` shapes stay chip-local copies by precedent.)
    let valid = [
        "1.5", "1.", ".5", "1e+10", "0x1.fp+2", "0x1p-1", "2.5f", "3.25E-2",
    ];
    for spelling in valid {
        assert!(parse_float_parts(spelling.as_bytes()).is_ok());
        assert!(spelling_to_value_parts(spelling.as_bytes()).is_ok());
    }
    let invalid: &[(&[u8], &str)] = &[
        (b"1e", "malformed decimal exponent in number spelling"),
        (b"1e+foo", "malformed decimal exponent in number spelling"),
        (b"1.5ll", "malformed decimal exponent in number spelling"),
        (b"0x1", "hexadecimal float spelling needs a binary exponent"),
        (
            b"0x10",
            "hexadecimal float spelling needs a binary exponent",
        ),
        (
            b"0x1.f",
            "hexadecimal float spelling needs a binary exponent",
        ),
        (b"1.5d", "unsupported suffix in floating spelling"),
        (b"1.0i", "unsupported suffix in floating spelling"),
    ];
    for (spelling, message) in invalid {
        assert!(parse_float_parts(spelling).is_err());
        let error = spelling_to_value_parts(spelling).expect_err("must reject");
        assert_eq!(error.diagnostic().message, message.to_string());
        assert!(is_fail(&LxFloatSyntaxChip.compute(&syntax_input(spelling))));
        assert!(is_fail(&LxFloatValueChip.compute(&value_input(spelling))));
    }
    // Integer-shaped spellings are LX05–LX08 territory for both chips
    // (`0x10` is hex-shaped without an exponent, so both chips report
    // the missing binary exponent instead).
    for spelling in [b"42".as_slice(), b"0"] {
        assert!(parse_float_parts(spelling).is_err());
        assert_eq!(
            spelling_to_value_parts(spelling),
            Err(FloatValueError::NotFloat)
        );
    }
}

#[test]
fn bus_dispatch_completes_ack_and_replays_deterministically() {
    fn run_once(spelling: &[u8]) -> (Vec<u8>, bool, bool) {
        let mut bus = new_bus();
        install_lx_float(&mut bus);
        let workers = workers();
        let token = seed_pp_number(&mut bus, spelling);
        bootstrap(
            &mut bus,
            TaskKind::LEX_FLOAT_SYNTAX,
            LX09_CHIP,
            Payload::from_refs(vec![token]),
        );
        bootstrap(
            &mut bus,
            TaskKind::LEX_FLOAT_VALUE,
            LX10_CHIP,
            Payload::from_refs(vec![token]),
        );
        // One task drains per tick under the fixture quota; two ticks
        // settle both dispatches.
        let mut completed = 0;
        let mut failed = 0;
        for _ in 0..2 {
            let report = tick(&mut bus, &workers);
            match &report.outcome {
                TickOutcome::Executed { commit, .. } => {
                    completed += commit.completed.len();
                    failed += commit.failed.len();
                }
                other => panic!("expected executed, got {other:?}"),
            }
        }
        let bytes = Snapshot::capture(&bus).bytes().to_vec();
        (bytes, completed == 2, failed > 0)
    }
    // Valid spelling: both chips complete; replay is byte-identical.
    let (first, ok, _) = run_once(b"0x1.fp+2");
    assert!(ok);
    let (second, ok, _) = run_once(b"0x1.fp+2");
    assert!(ok);
    assert_eq!(first, second);
    // Malformed spelling: completions fail loud, never silent.
    let (_, _, failed) = run_once(b"1e+foo");
    assert!(failed);
    // Ack completions carry no records.
    let mut bus = new_bus();
    install_lx_float(&mut bus);
    let workers = workers();
    let literals_before = bus.arenas.literals.allocated();
    let token = seed_pp_number(&mut bus, b"1.5");
    bootstrap(
        &mut bus,
        TaskKind::LEX_FLOAT_SYNTAX,
        LX09_CHIP,
        Payload::from_refs(vec![token]),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                ResultValue::Ack => {}
                other => panic!("expected Ack, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    }
    assert_eq!(bus.arenas.literals.allocated(), literals_before);
}

#[test]
fn float_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    // A wrong layer is refused on the driver path for both new kinds.
    for (kind, chip, worker) in [
        (
            TaskKind::LEX_FLOAT_SYNTAX,
            LX09_CHIP,
            &LxFloatSyntaxChip as &dyn Worker,
        ),
        (
            TaskKind::LEX_FLOAT_VALUE,
            LX10_CHIP,
            &LxFloatValueChip as &dyn Worker,
        ),
    ] {
        let mut mismatched = new_bus();
        mismatched.kinds = TaskKindRegistry::lx_float_slice();
        mismatched.schema = StoreSchema::pp_macro_slice();
        mismatched
            .registrations
            .register(worker.manifest(), &mismatched.schema, &mismatched.kinds)
            .unwrap();
        mismatched.routing.register(kind, chip, 9).unwrap();
        let task = bootstrap(&mut mismatched, kind, chip, Payload::empty());
        let error =
            cc_silicon_compiler::chips::drive_task(&mismatched, task, &workers()).unwrap_err();
        assert!(matches!(
            error,
            cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
        ));
    }
    // The pre-`/32` registry knows neither kind, so both manifests are
    // rejected there (frozen kinds are never re-registered).
    let mut stale = ManifestRegistry::new();
    let kinds = TaskKindRegistry::pp_emit_slice();
    let schema = StoreSchema::pp_macro_slice();
    for worker in [&LxFloatSyntaxChip as &dyn Worker, &LxFloatValueChip] {
        assert!(stale.register(worker.manifest(), &schema, &kinds).is_err());
    }
}
