// ============================================================================
// c20_ppscan.rs — Wave 2 (`/20`) PP full-token scan slice acceptance.
//
// Covers the frozen closure: unchanged kinds/registry/stages (no new task
// kinds), the full C11 punctuator table with maximal munch, string/char
// literal and header-name tokens, literal-aware comment replacement, the
// PP03→PP04 drift agreement, LX rejection of the new kinds, snapshot
// round-trip of the new token kinds, and the `/20` hash participation.
// The `/16` subset rules (six punctuators, literal `Unsupported`) stay
// superseded; M1 behavior is otherwise unchanged.
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, PpTokenKind, PpTokenRecord, SpanRecord};
use cc_silicon_compiler::chips::{
    handler_for, replace_comments, scan, LxClassifyChip, PpCommentChip, PpNormalizeChip,
    PpScanChip, PpSpliceChip, ScannedToken, Worker, WorkerRegistry,
};
use cc_silicon_compiler::ids::{RecordRef, TaskId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{stage_of, PP_COMMENT_CHIP, PP_SCAN_CHIP, PP_SPLICE_CHIP};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, TaskDraft, TaskKind, TaskKindRegistry};

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn install_pp(bus: &mut CompilerBus) {
    use cc_silicon_compiler::manifest::{StoreSchema, PP01_CHIP};
    bus.kinds = TaskKindRegistry::pp_slice();
    bus.schema = StoreSchema::pp_slice();
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(TaskKind::PREPROCESS_NORMALIZE, PP01_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_SPLICE, PP_SPLICE_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_COMMENT, PP_COMMENT_CHIP, 1)
        .unwrap();
    bus.routing
        .register(TaskKind::PREPROCESS_SCAN, PP_SCAN_CHIP, 1)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(PpNormalizeChip).unwrap();
    workers.register(PpSpliceChip).unwrap();
    workers.register(PpCommentChip).unwrap();
    workers.register(PpScanChip).unwrap();
    workers
}

fn bootstrap(
    bus: &mut CompilerBus,
    kind: TaskKind,
    owner: cc_silicon_compiler::ids::ChipId,
    payload: Payload,
) -> TaskId {
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

fn seed_source(
    bus: &mut CompilerBus,
    name: &[u8],
    raw: Vec<u8>,
) -> cc_silicon_compiler::ids::SourceId {
    let name = bus.intern_name(name).unwrap();
    bus.alloc_source(name, raw).unwrap()
}

fn kinds_of(tokens: &[ScannedToken]) -> Vec<PpTokenKind> {
    tokens.iter().map(|token| token.kind).collect()
}

#[test]
fn pp_kinds_registry_and_stages_are_unchanged() {
    use cc_silicon_compiler::contract::{NORMATIVE_RULES, PPTOKEN_KIND_NAMES};
    use cc_silicon_compiler::manifest::is_pp_slice_kind;
    // No new task kinds: the registry still carries the `/16` closure.
    let registry = TaskKindRegistry::pp_slice();
    assert_eq!(registry.len(), 29);
    assert!(is_pp_slice_kind(TaskKind::PREPROCESS_SCAN));
    assert_eq!(stage_of(TaskKind::PREPROCESS_SCAN), Some(1));
    assert_eq!(stage_of(TaskKind::PREPROCESS_COMMENT), Some(1));
    // The token-kind inventory grows by exactly the three `/20` kinds.
    assert_eq!(
        PPTOKEN_KIND_NAMES,
        &[
            "identifier",
            "pp_number",
            "punctuator",
            "string_literal",
            "char_literal",
            "header_name",
            "eof"
        ]
    );
    for rule in [
        "pp.comment-literal-aware",
        "pp.scan-full-punctuators",
        "pp.scan-literal-header-tokens",
    ] {
        assert!(NORMATIVE_RULES.contains(&rule), "missing rule `{rule}`");
    }
    // Same chips, same manifests (no read/write change).
    assert!(PpScanChip.manifest().reads.len() >= 7);
    assert_eq!(PpCommentChip.manifest().id, PP_COMMENT_CHIP);
    assert_eq!(PpScanChip.manifest().id, PP_SCAN_CHIP);
}

#[test]
fn pp_full_punctuator_table_maximal_munch() {
    use cc_silicon_compiler::bus::PpTokenKind as Kind;
    // Two-character operators stay split when spaced, join when adjacent.
    assert_eq!(
        kinds_of(&scan(b"a++b").unwrap()),
        vec![Kind::Identifier, Kind::Punctuator, Kind::Identifier]
    );
    assert_eq!(
        kinds_of(&scan(b"+ +").unwrap()),
        vec![Kind::Punctuator, Kind::Punctuator]
    );
    // Longest match first across the table.
    let tokens = scan(b"a...b").unwrap();
    assert_eq!(tokens.len(), 3);
    assert_eq!(&tokens[1].kind, &Kind::Punctuator);
    let tokens = scan(b"x <<= 1").unwrap();
    assert_eq!(tokens.len(), 3);
    let tokens = scan(b"a ## b").unwrap();
    assert_eq!(tokens.len(), 3);
    let tokens = scan(b"a->b").unwrap();
    assert_eq!(tokens.len(), 3);
    let tokens = scan(b"a--b").unwrap();
    assert_eq!(tokens.len(), 3);
    // Digraphs and alternative spellings scan as punctuators.
    let digraphs: &[&[u8]] = &[b"<:", b":>", b"<%", b"%>", b"%:", b"%:%:"];
    for spelling in digraphs {
        let tokens = scan(spelling).unwrap();
        assert_eq!(tokens.len(), 1, "{}", String::from_utf8_lossy(spelling));
        assert_eq!(tokens[0].kind, Kind::Punctuator);
    }
    // Full single set newly covered past M1.
    for spelling in [
        b"*", b"!", b"%", b"^", b"?", b":", b"=", b",", b"#", b"~", b"/",
    ] {
        let tokens = scan(spelling).unwrap();
        assert_eq!(tokens.len(), 1, "{}", String::from_utf8_lossy(spelling));
        assert_eq!(tokens[0].kind, Kind::Punctuator);
    }
    // Still explicitly unsupported, never mis-tokenized.
    assert!(scan(b"@").is_err());
    assert!(scan(b"$").is_err());
    assert!(scan(b"`x`").is_err());
    assert!(scan("\\".as_bytes()).is_err());
}

#[test]
fn pp_string_char_literals_scan_single_tokens() {
    // Escapes do not close the literal.
    let tokens = scan(b"\"a\\\"b\"").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, PpTokenKind::StringLiteral);
    let tokens = scan(b"'\\''").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, PpTokenKind::CharLiteral);
    let tokens = scan(b"'\\\\'").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, PpTokenKind::CharLiteral);
    // Comment openers inside literals are literal bytes.
    let tokens = scan(b"\"/*\"").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, PpTokenKind::StringLiteral);
    // Unterminated literals fail loudly.
    assert!(scan(b"\"abc").is_err());
    assert!(scan(b"'ab").is_err());
    // Dotted pp-numbers (the only `/20` pp-number extension).
    let tokens = scan(b".5").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, PpTokenKind::PpNumber);
    let tokens = scan(b".5e+3").unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, PpTokenKind::PpNumber);
    // A lone dot stays a punctuator.
    let tokens = scan(b"a.b").unwrap();
    assert_eq!(
        kinds_of(&tokens),
        vec![
            PpTokenKind::Identifier,
            PpTokenKind::Punctuator,
            PpTokenKind::Identifier
        ]
    );
}

#[test]
fn pp_header_names_scan_single_tokens() {
    // Angle and quoted headers after `#include`.
    let tokens = scan(b"#include <stdio.h>").unwrap();
    assert_eq!(tokens.last().unwrap().kind, PpTokenKind::HeaderName);
    let tokens = scan(b"#include \"my.h\"").unwrap();
    assert_eq!(tokens.last().unwrap().kind, PpTokenKind::HeaderName);
    // Block comment skipped between `include` and the header.
    let tokens = scan(b"#include /*c*/ <x.h>").unwrap();
    assert_eq!(tokens.last().unwrap().kind, PpTokenKind::HeaderName);
    // `//` aborts the header context: no header token forms.
    let tokens = scan(b"#include // <x.h>").unwrap();
    assert!(!tokens
        .iter()
        .any(|token| token.kind == PpTokenKind::HeaderName));
    // `include_next` is not a header context (deferred to T12).
    let tokens = scan(b"#include_next <x.h>").unwrap();
    assert!(!tokens
        .iter()
        .any(|token| token.kind == PpTokenKind::HeaderName));
    // A `#` off line-start never opens a header context.
    let tokens = scan(b"int # <x.h>").unwrap();
    assert!(!tokens
        .iter()
        .any(|token| token.kind == PpTokenKind::HeaderName));
    // `//` inside an angle header is header bytes, not a comment.
    let tokens = scan(b"#include <a//b>").unwrap();
    assert_eq!(tokens.last().unwrap().kind, PpTokenKind::HeaderName);
    // Unterminated headers fail loudly.
    assert!(scan(b"#include <abc").is_err());
    assert!(scan(b"#include \"abc").is_err());
}

#[test]
fn pp_comment_literal_header_protection() {
    // Comment markers inside literals survive byte-identical.
    let (bytes, _) = replace_comments(b"char *s = \"/*\"; // real\n").unwrap();
    assert_eq!(bytes, b"char *s = \"/*\";  \n");
    let (bytes, _) = replace_comments(b"char c = '/';").unwrap();
    assert_eq!(bytes, b"char c = '/';");
    // Escaped quotes do not end the literal early (the trailing ` */`
    // is not a comment opener, so the bytes pass through unchanged; a
    // real comment after the literal is still replaced).
    let (bytes, _) = replace_comments(b"\"a\\\"/*\" */ x").unwrap();
    assert_eq!(bytes, b"\"a\\\"/*\" */ x");
    let (bytes, _) = replace_comments(b"\"a\\\"b\" /*c*/ x").unwrap();
    assert_eq!(bytes, b"\"a\\\"b\"   x");
    // Header names are protected, including `//` inside angle headers.
    let (bytes, _) = replace_comments(b"#include <a//b>\n").unwrap();
    assert_eq!(bytes, b"#include <a//b>\n");
    let (bytes, _) = replace_comments(b"#include \"m//n.h\"\n").unwrap();
    assert_eq!(bytes, b"#include \"m//n.h\"\n");
    // Ordinary comment behavior is unchanged.
    let (bytes, _) = replace_comments(b"a/**/b").unwrap();
    assert_eq!(bytes, b"a b");
    assert!(replace_comments(b"a /* x").is_err());
    // Unterminated literals fail with typed diagnostics.
    assert!(replace_comments(b"\"abc").is_err());
    assert!(replace_comments(b"'ab").is_err());
}

#[test]
fn pp03_pp04_drift_end_to_end() {
    use cc_silicon_compiler::manifest::PP01_CHIP;
    let raw = b"const char *s = \"a//b\";\n#include <sys/types.h>\nint main(void){return 2+3;}\n"
        .to_vec();
    let mut bus = new_bus();
    install_pp(&mut bus);
    let source = seed_source(&mut bus, b"drift.c", raw.clone());
    let workers = workers();
    // normalize → splice → comment from real source bytes.
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_NORMALIZE,
        PP01_CHIP,
        Payload::from_refs(vec![RecordRef::Source(source)]),
    );
    let report = tick(&mut bus, &workers);
    let mut artifact = match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Artifact(id)) => *id,
                other => panic!("expected artifact ref, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    for (kind, owner) in [
        (TaskKind::PREPROCESS_SPLICE, PP_SPLICE_CHIP),
        (TaskKind::PREPROCESS_COMMENT, PP_COMMENT_CHIP),
    ] {
        bootstrap(
            &mut bus,
            kind,
            owner,
            Payload::from_refs(vec![RecordRef::Artifact(artifact)]),
        );
        let report = tick(&mut bus, &workers);
        artifact = match &report.outcome {
            TickOutcome::Executed { commit, .. } => {
                match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                    cc_silicon_compiler::task::ResultValue::Record(RecordRef::Artifact(id)) => *id,
                    other => panic!("expected artifact ref, got {other:?}"),
                }
            }
            other => panic!("expected executed, got {other:?}"),
        };
    }
    let comment_free = bus.arenas.artifacts.get(artifact).unwrap().bytes.clone();
    // scan closes over the comment-free stream.
    bootstrap(
        &mut bus,
        TaskKind::PREPROCESS_SCAN,
        PP_SCAN_CHIP,
        Payload::from_refs(vec![RecordRef::Artifact(artifact)]),
    );
    let report = tick(&mut bus, &workers);
    let refs = match &report.outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.completed.len(), 1);
            match &bus.arenas.results.get(commit.completed[0].1).unwrap().value {
                cc_silicon_compiler::task::ResultValue::Records(refs) => refs.clone(),
                other => panic!("expected records, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    // Drift agreement: every literal/header token spelling is byte-identical
    // in the comment-free stream AND matches the raw source at its span.
    let source_bytes = bus.arenas.sources.get(source).unwrap().bytes.clone();
    let mut saw_string = false;
    let mut saw_header = false;
    for reference in &refs {
        let token = match reference {
            RecordRef::PpToken(id) => bus.arenas.pp_tokens.get(*id).unwrap().clone(),
            other => panic!("expected pp-token ref, got {other:?}"),
        };
        if token.kind == PpTokenKind::StringLiteral {
            saw_string = true;
            assert_eq!(token.spelling, b"\"a//b\"");
        }
        if token.kind == PpTokenKind::HeaderName {
            saw_header = true;
            assert_eq!(token.spelling, b"<sys/types.h>");
        }
        if matches!(
            token.kind,
            PpTokenKind::StringLiteral | PpTokenKind::CharLiteral | PpTokenKind::HeaderName
        ) {
            let span = bus.arenas.spans.get(token.span).unwrap();
            assert_eq!(
                &source_bytes[span.start as usize..span.end as usize],
                token.spelling.as_slice()
            );
            assert!(comment_free
                .windows(token.spelling.len())
                .any(|window| window == token.spelling.as_slice()));
        }
    }
    assert!(saw_string && saw_header);
}

#[test]
fn pp_lx_rejects_new_kinds() {
    use cc_silicon_compiler::manifest::{StoreSchema, LX_CLASSIFY_CHIP};
    use cc_silicon_compiler::task::TaskKindRegistry;
    let mut bus = new_bus();
    bus.kinds = TaskKindRegistry::lx_slice();
    bus.schema = StoreSchema::lx_slice();
    bus.registrations
        .register(LxClassifyChip.manifest(), &bus.schema, &bus.kinds)
        .unwrap();
    bus.routing
        .register(TaskKind::LEX_CLASSIFY, LX_CLASSIFY_CHIP, 2)
        .unwrap();
    let mut workers = WorkerRegistry::new();
    workers.register(LxClassifyChip).unwrap();
    let source = seed_source(&mut bus, b"s.c", b"\"ab\"\n".to_vec());
    let limits = bus.limits();
    let span = bus
        .arenas
        .spans
        .alloc(
            SpanRecord {
                source,
                start: 0,
                end: 4,
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
                kind: PpTokenKind::StringLiteral,
                span,
                spelling: b"\"ab\"".to_vec(),
            },
            &limits,
        )
        .unwrap();
    bootstrap(
        &mut bus,
        TaskKind::LEX_CLASSIFY,
        LX_CLASSIFY_CHIP,
        Payload::from_refs(vec![RecordRef::PpToken(token)]),
    );
    match tick(&mut bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
}

#[test]
fn pp_snapshot_round_trip_new_kinds_and_hash() {
    use cc_silicon_compiler::contract::FrozenSchema;
    use cc_silicon_compiler::contract::{NORMATIVE_RULES, PPTOKEN_KIND_NAMES};
    use cc_silicon_compiler::ids::SpanId;
    use cc_silicon_compiler::snapshot::{decode_pp_token, encode_pp_token};
    for (kind, name) in [
        (PpTokenKind::StringLiteral, "string_literal"),
        (PpTokenKind::CharLiteral, "char_literal"),
        (PpTokenKind::HeaderName, "header_name"),
    ] {
        assert!(PPTOKEN_KIND_NAMES.contains(&name));
        let record = PpTokenRecord {
            kind,
            span: SpanId::from_index(3),
            spelling: b"x".to_vec(),
        };
        assert_eq!(decode_pp_token(&encode_pp_token(&record)).unwrap(), record);
    }
    // The frozen bytes carry the extended inventory.
    let bytes = FrozenSchema::current().encode();
    for marker in [
        "header_name",
        "string_literal",
        "char_literal",
        "t01-c01-c06/37",
    ] {
        assert!(
            bytes
                .windows(marker.len())
                .any(|window| window == marker.as_bytes()),
            "frozen bytes miss `{marker}`"
        );
    }
    assert!(NORMATIVE_RULES.contains(&"pp.scan-literal-header-tokens"));
}
