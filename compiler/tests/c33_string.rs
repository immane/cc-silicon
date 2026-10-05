// ============================================================================
// c33_string.rs — Wave 3 (`/33`) LX string-slice acceptance.
//
// Covers the frozen closure: the `lex.escape_decode` (local 21),
// `lex.char_decode` (local 22), and `lex.string_decode` (local 23) kinds,
// kind-to-stage assignment (stage 2), the LX11/LX12/LX13 manifests
// (LX11 Ack-only with zero writes and no allowlist row; LX12/LX13 each
// append one `LiteralRecord` with one allowlist row), the escape rule
// (simple/octal/greedy-hex/UCN), the frozen multicharacter truncation
// policy, the string length convention (units include the single
// terminator; embedded NULs preserved), wide encodings (wchar width,
// UTF-16 surrogate pairs, UTF-32 units), bus dispatch (records committed,
// replay determinism), and stage/layer gates. Escape-copy reconciliation
// (three chip-local copies kept by precedent, convergence deferred) is
// pinned by the shared-corpus agreement case.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, CompilerPins, PpTokenKind, PpTokenRecord, SpanRecord, TokenKind, TokenRecord,
};
use cc_silicon_compiler::chips::{
    decode_char_units, decode_escape_body, decode_string_body, encode_units_le, fold_char_value,
    handler_for, split_char_spelling, split_literal_body, split_string_prefix, CharPrefix,
    EscapeError, LxCharChip, LxCharInput, LxEscapeChip, LxEscapeInput, LxStringChip, LxStringInput,
    StringPrefix, Worker, WorkerRegistry, LX11_TASK_KIND, LX12_TASK_KIND, LX13_TASK_KIND,
};
use cc_silicon_compiler::ids::{ChipId, LiteralId, RecordRef, SpanId, TaskId, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    check_stage_layer_agreement, is_lx_string_slice_kind, stage_of, LX11_CHIP, LX12_CHIP,
    LX13_CHIP, STORE_OWNER_ALLOWLIST,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType, Snapshot};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
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

fn install_lx_string(bus: &mut CompilerBus) {
    use cc_silicon_compiler::manifest::StoreSchema;
    bus.kinds = TaskKindRegistry::lx_string_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    for chip in [&LxEscapeChip as &dyn Worker, &LxCharChip, &LxStringChip] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(TaskKind::LEX_ESCAPE_DECODE, LX11_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::LEX_CHAR_DECODE, LX12_CHIP, 2)
        .unwrap();
    bus.routing
        .register(TaskKind::LEX_STRING_DECODE, LX13_CHIP, 2)
        .unwrap();
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(LxEscapeChip).unwrap();
    workers.register(LxCharChip).unwrap();
    workers.register(LxStringChip).unwrap();
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

/// Seed one committed `Token` + `PpToken` pair carrying `spelling`; returns
/// the `[Token, PpToken]` payload refs in order.
fn seed_literal(
    bus: &mut CompilerBus,
    spelling: &[u8],
    kind: PpTokenKind,
) -> (RecordRef, RecordRef) {
    let limits = bus.limits();
    let name = bus.intern_name(b"string.c").unwrap();
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
    let pp_token = bus
        .arenas
        .pp_tokens
        .alloc(
            PpTokenRecord {
                kind,
                span,
                spelling: spelling.to_vec(),
            },
            &limits,
        )
        .unwrap();
    let token = bus
        .arenas
        .tokens
        .alloc(
            TokenRecord {
                kind: TokenKind::Punctuator,
                span,
                name: None,
                pp_token,
            },
            &limits,
        )
        .unwrap();
    (RecordRef::Token(token), RecordRef::PpToken(pp_token))
}

fn char_input(spelling: &[u8]) -> LxCharInput {
    LxCharInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        token: TokenId::from_index(0),
        spelling: spelling.to_vec(),
        literals_allocated: 0,
    }
}

fn string_input(spelling: &[u8]) -> LxStringInput {
    LxStringInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        token: TokenId::from_index(0),
        spelling: spelling.to_vec(),
        span: SpanId::from_index(0),
        literals_allocated: 0,
        wchar_bytes: 4,
        wchar_utf32: true,
    }
}

fn escape_input(spelling: &[u8]) -> LxEscapeInput {
    LxEscapeInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        spelling: spelling.to_vec(),
        span: SpanId::from_index(0),
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

fn char_record(proposals: &[Proposal]) -> cc_silicon_compiler::bus::LiteralRecord {
    use cc_silicon_compiler::records::G1DraftBody;
    assert_eq!(proposals.len(), 2);
    let Proposal::AppendRecords { batch, .. } = &proposals[0] else {
        panic!("first proposal must append the literal record");
    };
    let G1DraftBody::Literal(record) = &batch.bodies[0] else {
        panic!("appended body must be a literal");
    };
    record.clone()
}

#[test]
fn string_kinds_stage_registry_manifest_frozen() {
    use cc_silicon_compiler::task::StoreId;
    assert_eq!(TaskKind::LEX_ESCAPE_DECODE.group(), TaskGroup::LEX);
    assert_eq!(TaskKind::LEX_ESCAPE_DECODE.local(), 21);
    assert_eq!(TaskKind::LEX_CHAR_DECODE.group(), TaskGroup::LEX);
    assert_eq!(TaskKind::LEX_CHAR_DECODE.local(), 22);
    assert_eq!(TaskKind::LEX_STRING_DECODE.group(), TaskGroup::LEX);
    assert_eq!(TaskKind::LEX_STRING_DECODE.local(), 23);
    assert_eq!(LX11_TASK_KIND, TaskKind::LEX_ESCAPE_DECODE);
    assert_eq!(LX12_TASK_KIND, TaskKind::LEX_CHAR_DECODE);
    assert_eq!(LX13_TASK_KIND, TaskKind::LEX_STRING_DECODE);
    assert!(is_lx_string_slice_kind(TaskKind::LEX_ESCAPE_DECODE));
    assert!(is_lx_string_slice_kind(TaskKind::LEX_CHAR_DECODE));
    assert!(is_lx_string_slice_kind(TaskKind::LEX_STRING_DECODE));
    assert!(!is_lx_string_slice_kind(TaskKind::CONTROL_NOOP));
    assert!(!is_lx_string_slice_kind(TaskKind::LEX_DECODE_LITERAL));
    assert!(!is_lx_string_slice_kind(TaskKind::LEX_FLOAT_SYNTAX));
    assert_eq!(stage_of(TaskKind::LEX_ESCAPE_DECODE), Some(2));
    assert_eq!(stage_of(TaskKind::LEX_CHAR_DECODE), Some(2));
    assert_eq!(stage_of(TaskKind::LEX_STRING_DECODE), Some(2));
    // The string registry extends the `/32` head linearly; `LEX` owners
    // start new codes at local 24.
    let emit_head = TaskKindRegistry::pp_emit_slice();
    assert_eq!(emit_head.len(), 48);
    assert_eq!(TaskKindRegistry::lx_float_slice().len(), 50);
    let registry = TaskKindRegistry::lx_string_slice();
    assert_eq!(registry.len(), 53);
    assert_eq!(
        registry.lookup(TaskKind::LEX_ESCAPE_DECODE).unwrap().name,
        "lex.escape_decode"
    );
    assert_eq!(
        registry.lookup(TaskKind::LEX_CHAR_DECODE).unwrap().name,
        "lex.char_decode"
    );
    assert_eq!(
        registry.lookup(TaskKind::LEX_STRING_DECODE).unwrap().name,
        "lex.string_decode"
    );
    assert_eq!(stage_of(TaskKind::new(TaskGroup::LEX, 24).unwrap()), None);
    // LX11 is Ack-only (zero writes, no allowlist row); LX12/LX13 each
    // append `lex.literals` with one allowlist row; `/39` adds the SE29
    // `sem.records` row.
    assert_eq!(LX11_CHIP, ChipId(41));
    assert_eq!(LX12_CHIP, ChipId(42));
    assert_eq!(LX13_CHIP, ChipId(43));
    assert_eq!(STORE_OWNER_ALLOWLIST.len(), 33);
    assert!(
        STORE_OWNER_ALLOWLIST
            .iter()
            .any(|&(chip, store, field, kind)| {
                chip == LX12_CHIP
                    && store == StoreId::Lex
                    && field == "literals"
                    && kind == TaskKind::LEX_CHAR_DECODE
            }),
        "missing LX12 allowlist row"
    );
    assert!(
        STORE_OWNER_ALLOWLIST
            .iter()
            .any(|&(chip, store, field, kind)| {
                chip == LX13_CHIP
                    && store == StoreId::Lex
                    && field == "literals"
                    && kind == TaskKind::LEX_STRING_DECODE
            }),
        "missing LX13 allowlist row"
    );
    assert!(
        !STORE_OWNER_ALLOWLIST
            .iter()
            .any(|&(chip, _, _, _)| chip == LX11_CHIP),
        "Ack-only LX11 must hold no allowlist row"
    );
    assert!(LxEscapeChip.manifest().writes.is_empty());
    for manifest in [LxCharChip.manifest(), LxStringChip.manifest()] {
        assert_eq!(
            manifest.writes,
            vec![cc_silicon_compiler::manifest::FieldPath::new(
                StoreId::Lex,
                "literals"
            )]
        );
        assert!(manifest.deterministic);
        assert!(manifest.declares_read(StoreId::Tasks, "active.payload"));
        assert!(manifest.declares_read(StoreId::Pp, "tokens"));
        assert!(check_stage_layer_agreement(&manifest, &{
            let mut bus = new_bus();
            install_lx_string(&mut bus);
            bus.routing
        })
        .is_ok());
    }
    assert!(LxEscapeChip.manifest().deterministic);
    assert!(check_stage_layer_agreement(&LxEscapeChip.manifest(), &{
        let mut bus = new_bus();
        install_lx_string(&mut bus);
        bus.routing
    })
    .is_ok());
    for manifest in [
        LxEscapeChip.manifest(),
        LxCharChip.manifest(),
        LxStringChip.manifest(),
    ] {
        assert_eq!(manifest.tests, vec!["compiler/tests/c33_string.rs"]);
    }
}

#[test]
fn escape_simple_and_octal() {
    // Simple escapes map to ASCII controls (LX11 rule set).
    let units = decode_escape_body(b"\\a\\b\\f\\n\\r\\t\\v\\\\\\'\\\"\\?")
        .expect("simple escapes decode")
        .into_units();
    assert_eq!(
        units,
        vec![0x07, 0x08, 0x0C, 0x0A, 0x0D, 0x09, 0x0B, 0x5C, 0x27, 0x22, 0x3F]
    );
    // Octal takes at most three digits: `\1011` is 'A' followed by '1'.
    let units = decode_escape_body(b"\\1011")
        .expect("octal decodes")
        .into_units();
    assert_eq!(units, vec![0x41, 0x31]);
    assert_eq!(
        decode_escape_body(b"\\0")
            .expect("octal decodes")
            .into_units(),
        vec![0x00]
    );
    assert!(is_ack(&LxEscapeChip.compute(&escape_input(b"\"a\\nB\""))));
    assert!(is_ack(&LxEscapeChip.compute(&escape_input(b"'\\101'"))));
}

#[test]
fn escape_hex_greedy_and_ucn() {
    // Greedy hex: `\x41B` is one escape (0x41B); LX11 passes the value
    // through untruncated (consumer width policy lives in LX12/LX13).
    let units = decode_escape_body(b"\\x41B")
        .expect("greedy hex decodes")
        .into_units();
    assert_eq!(units, vec![0x41B]);
    assert_eq!(decode_escape_body(b"\\x"), Err(EscapeError::EmptyHex));
    // UCNs decode with scalar-range validation, including the C11 6.4.3
    // trio rule below U+00A0.
    let units = decode_escape_body(b"\\u00E9\\U0001F600")
        .expect("ucn decodes")
        .into_units();
    assert_eq!(units, vec![0xE9, 0x1F600]);
    assert_eq!(
        decode_escape_body(b"\\u0024\\u0040\\u0060")
            .expect("trio decodes")
            .into_units(),
        vec![0x24, 0x40, 0x60]
    );
    assert_eq!(
        decode_escape_body(b"\\uD800"),
        Err(EscapeError::SurrogateUniversal)
    );
    assert_eq!(
        decode_escape_body(b"\\u000A"),
        Err(EscapeError::ControlUniversal)
    );
    assert_eq!(
        decode_escape_body(b"\\q"),
        Err(EscapeError::UnknownEscape(b'q'))
    );
    assert!(is_fail(&LxEscapeChip.compute(&escape_input(b"\"\\q\""))));
    assert!(is_fail(&LxEscapeChip.compute(&escape_input(b"\"\\x\""))));
}

#[test]
fn escape_body_split_and_raw_bytes() {
    assert_eq!(split_literal_body(b"\"ab\""), Ok(b"ab".as_slice()));
    assert_eq!(split_literal_body(b"L\"ab\""), Ok(b"ab".as_slice()));
    assert_eq!(split_literal_body(b"\"\""), Ok(b"".as_slice()));
    assert_eq!(split_literal_body(b"\"ab"), Err(EscapeError::Unterminated));
    // Empty body decodes to no units (LX13 appends the terminator).
    assert_eq!(
        decode_escape_body(b"").expect("empty decodes").into_units(),
        Vec::<u32>::new()
    );
    assert!(is_fail(&LxEscapeChip.compute(&escape_input(b"\"ab"))));
}

#[test]
fn char_empty_and_malformed_fail() {
    // Empty literals fail; the empty-string terminator is LX13 policy.
    assert!(is_fail(&LxCharChip.compute(&char_input(b"''"))));
    assert!(is_fail(&LxCharChip.compute(&char_input(b"'a"))));
    // The `u8` character prefix is outside the none/L/u/U scope.
    assert!(is_fail(&LxCharChip.compute(&char_input(b"u8'a'"))));
    // Unknown escapes and raw newlines fail loud.
    assert!(is_fail(&LxCharChip.compute(&char_input(b"'\\q'"))));
    assert!(is_fail(&LxCharChip.compute(&char_input(b"'\\x'"))));
    let mut bad = char_input(b"'a'");
    bad.state = TaskState::Ready;
    assert!(is_fail(&LxCharChip.compute(&bad)));
    // Prefix/body split errors stay typed, never panics.
    assert!(split_char_spelling(b"u8'a'").is_err());
    assert!(split_char_spelling(b"'a").is_err());
    assert!(decode_char_units(b"").is_err());
}

#[test]
fn char_values_and_multichar_policy() {
    // Plain `'a'`: one Character record with the publish-time back-link.
    let record = char_record(&LxCharChip.compute(&char_input(b"'a'")));
    assert_eq!(record.kind, LiteralKind::Character);
    assert_eq!(record.value, vec![0x61]);
    assert_eq!(record.radix, 16);
    assert_eq!(record.suffix, LiteralSuffix::None);
    assert!(!record.negative);
    assert_eq!(record.spelling, b"'a'");
    assert_eq!(record.token, Some(TokenId::from_index(0)));
    assert_eq!(record.candidate_type, Lx08CandidateType::Int);
    // Multicharacter constants fold big-endian, low 32 bits kept.
    let record = char_record(&LxCharChip.compute(&char_input(b"'ab'")));
    assert_eq!(record.value, vec![0x61, 0x62]);
    let wide = char_record(&LxCharChip.compute(&char_input(b"L'ab'")));
    assert_eq!(wide.value, vec![0x61, 0x62]);
    // Greedy hex truncates to the prefix unit width (plain: low 8 bits).
    let record = char_record(&LxCharChip.compute(&char_input(b"'\\x41B'")));
    assert_eq!(record.value, vec![0x1b]);
    // Octal takes at most three digits here as well.
    let record = char_record(&LxCharChip.compute(&char_input(b"'\\1011'")));
    assert_eq!(record.value, vec![0x41, 0x31]);
    // Pure helpers agree with the worker.
    let (prefix, body) = split_char_spelling(b"'ab'").expect("splits");
    assert_eq!(prefix, CharPrefix::Plain);
    let units = decode_char_units(body).expect("decodes");
    let (value, multichar) = fold_char_value(prefix, &units);
    assert_eq!((value, multichar), (0x6162, true));
}

#[test]
fn char_prefix_widths_and_ucn() {
    // `u` prefix keeps 16-bit units; raw bytes compose no sequences.
    let record = char_record(&LxCharChip.compute(&char_input(b"u'\xce\xa9'")));
    assert_eq!(record.value, vec![0xce, 0xa9]);
    let record = char_record(&LxCharChip.compute(&char_input(b"U'A'")));
    assert_eq!(record.value, vec![0x41]);
    let record = char_record(&LxCharChip.compute(&char_input(b"L'a'")));
    assert_eq!(record.value, vec![0x61]);
    // UCNs validate surrogates and digit counts.
    let ok = char_record(&LxCharChip.compute(&char_input(b"'\\u00E9'")));
    assert_eq!(ok.value, vec![0xe9]);
    assert!(is_fail(&LxCharChip.compute(&char_input(b"'\\uD800'"))));
    assert!(is_fail(&LxCharChip.compute(&char_input(b"'\\u12'"))));
}

#[test]
fn string_empty_is_single_nul() {
    // The empty literal decodes to exactly one unit: the terminator.
    let proposals = LxStringChip.compute(&string_input(b"\"\""));
    let record = char_record(&proposals);
    assert_eq!(record.kind, LiteralKind::String);
    assert_eq!(record.value, vec![0x00]);
    assert_eq!(record.radix, 0);
    assert_eq!(record.suffix, LiteralSuffix::None);
    assert!(!record.negative);
    assert_eq!(record.spelling, b"\"\"");
    assert_eq!(record.token, Some(TokenId::from_index(0)));
    assert_eq!(record.candidate_type, Lx08CandidateType::Int);
    // Units length includes the terminator: `"ab"` -> [a, b, 0].
    let decoded = decode_string_body(StringPrefix::None, b"ab", 4).expect("decodes");
    assert_eq!(decoded.units, vec![0x61, 0x62, 0x00]);
    assert_eq!(encode_units_le(&decoded.units, 1), vec![0x61, 0x62, 0x00]);
}

#[test]
fn string_embedded_nul_preserved() {
    // `"a\0b"`: embedded NULs are ordinary mid-string zero units.
    let decoded = decode_string_body(StringPrefix::None, b"a\\0b", 4).expect("decodes");
    assert_eq!(decoded.units, vec![0x61, 0x00, 0x62, 0x00]);
    assert_eq!(decoded.units.len(), 4);
    let proposals = LxStringChip.compute(&string_input(b"\"a\\0b\""));
    let record = char_record(&proposals);
    assert_eq!(record.value, vec![0x61, 0x00, 0x62, 0x00]);
    // Simple and octal escapes share the LX11 rule shape.
    let decoded = decode_string_body(StringPrefix::None, b"\\n\\t\\101", 4).expect("decodes");
    assert_eq!(decoded.units, vec![0x0A, 0x09, 0x41, 0x00]);
}

#[test]
fn string_wide_encodings() {
    // `L` uses the frozen wchar width (4/utf32 on the frozen target).
    let decoded = decode_string_body(StringPrefix::Wide, b"A", 4).expect("decodes");
    assert_eq!(decoded.units, vec![0x41, 0x00]);
    assert_eq!(
        encode_units_le(&decoded.units, 4),
        vec![0x41, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
    );
    assert!(decode_string_body(StringPrefix::Wide, b"A", 3).is_err());
    // `u` encodes non-BMP code points as a UTF-16 surrogate pair.
    let decoded = decode_string_body(StringPrefix::Char16, b"\\U0001F600", 4).expect("decodes");
    assert_eq!(decoded.units, vec![0xD83D, 0xDE00, 0x00]);
    // `U` keeps full UTF-32 units; `u8`/narrow hold single bytes.
    let decoded = decode_string_body(StringPrefix::Char32, b"\\u00E9", 4).expect("decodes");
    assert_eq!(decoded.units, vec![0xE9, 0x00]);
    let proposals = LxStringChip.compute(&string_input(b"u8\"ab\""));
    assert_eq!(char_record(&proposals).value, b"ab\0");
    // Greedy hex consumes every digit; narrow units reject > 0xFF.
    assert!(decode_string_body(StringPrefix::None, b"\\x100", 4).is_err());
    assert!(decode_string_body(StringPrefix::None, b"\\x", 4).is_err());
    // Prefix split prefers `u8` over `u`.
    let (prefix, rest) = split_string_prefix(b"u8\"ab\"");
    assert_eq!(prefix, StringPrefix::U8);
    assert_eq!(rest, b"\"ab\"");
}

#[test]
fn string_invalid_fails_loud() {
    for spelling in [
        b"\"\\q\"".as_slice(),
        b"\"abc\\".as_slice(),
        b"\"ab".as_slice(),
        b"L\"\xc3\"".as_slice(),
    ] {
        assert!(
            is_fail(&LxStringChip.compute(&string_input(spelling))),
            "must fail: {:?}",
            spelling
        );
    }
    // Narrow UCNs must fit in one byte; wide rejects surrogates.
    assert!(decode_string_body(StringPrefix::None, b"\\u0100", 4).is_err());
    assert!(decode_string_body(StringPrefix::Char32, b"\\uD800", 4).is_err());
    assert!(decode_string_body(StringPrefix::Char32, b"\\U00111000", 4).is_err());
    // Non-utf32 wchar encoding and non-running tasks fail.
    let mut bad_encoding = string_input(b"\"a\"");
    bad_encoding.wchar_utf32 = false;
    assert!(is_fail(&LxStringChip.compute(&bad_encoding)));
    let mut bad_state = string_input(b"\"a\"");
    bad_state.state = TaskState::Ready;
    assert!(is_fail(&LxStringChip.compute(&bad_state)));
}

#[test]
fn escape_copy_agreement_over_corpus() {
    // The frozen interchange is the spelling plus the `lx.escape-decode`
    // rule: the three chip-local copies agree on the shared corpus. Cases
    // outside a copy's unit-width policy are documented deltas below, all
    // loud (never silent reinterpretation).
    let valid = ["a", "\\n", "\\101", "\\x41", "\\u00E9", "\\'\\\"\\?\\\\"];
    for body in valid {
        let bytes = body.as_bytes();
        assert!(decode_escape_body(bytes).is_ok(), "LX11 rejects `{body}`");
        assert!(decode_char_units(bytes).is_ok(), "LX12 rejects `{body}`");
        assert!(
            decode_string_body(StringPrefix::None, bytes, 4).is_ok(),
            "LX13 rejects `{body}`"
        );
    }
    // Wide LX13 agrees with LX11 on non-BMP code points.
    let units = decode_escape_body(b"\\U0001F600")
        .expect("LX11 decodes")
        .into_units();
    assert_eq!(units, vec![0x1F600]);
    let wide = decode_string_body(StringPrefix::Char32, b"\\U0001F600", 4).expect("LX13 decodes");
    assert_eq!(wide.units, vec![0x1F600, 0x00]);
    // Unknown escapes, truncated hex, and truncated UCNs fail everywhere.
    for body in ["\\q", "\\x", "\\u12", "abc\\"] {
        let bytes = body.as_bytes();
        assert!(decode_escape_body(bytes).is_err(), "LX11 accepts `{body}`");
        assert!(decode_char_units(bytes).is_err(), "LX12 accepts `{body}`");
        assert!(
            decode_string_body(StringPrefix::None, bytes, 4).is_err(),
            "LX13 accepts `{body}`"
        );
    }
    // Documented width deltas (convergence deferred; each side loud):
    // LX11 passes `\x41B` through untruncated while LX12 truncates to the
    // prefix width and LX13-narrow rejects above `0xFF`.
    assert_eq!(
        decode_escape_body(b"\\x41B")
            .expect("LX11 decodes")
            .into_units(),
        vec![0x41B]
    );
    let (value, _) = fold_char_value(CharPrefix::Plain, &[0x41B]);
    assert_eq!(value, 0x1B);
    assert!(decode_string_body(StringPrefix::None, b"\\x41B", 4).is_err());
}

#[test]
fn bus_dispatch_commits_records_and_replays_deterministically() {
    fn run_once() -> (Vec<u8>, usize, usize, usize) {
        let mut bus = new_bus();
        install_lx_string(&mut bus);
        let workers = workers();
        let literals_before = bus.arenas.literals.allocated();
        let (token, pp) = seed_literal(&mut bus, b"\"a\\n\"", PpTokenKind::StringLiteral);
        bootstrap(
            &mut bus,
            TaskKind::LEX_ESCAPE_DECODE,
            LX11_CHIP,
            Payload::from_refs(vec![token, pp]),
        );
        let (token, pp) = seed_literal(&mut bus, b"'a'", PpTokenKind::CharLiteral);
        bootstrap(
            &mut bus,
            TaskKind::LEX_CHAR_DECODE,
            LX12_CHIP,
            Payload::from_refs(vec![token, pp]),
        );
        let (token, pp) = seed_literal(&mut bus, b"\"a\"", PpTokenKind::StringLiteral);
        bootstrap(
            &mut bus,
            TaskKind::LEX_STRING_DECODE,
            LX13_CHIP,
            Payload::from_refs(vec![token, pp]),
        );
        // One task drains per tick under the fixture quota; three ticks
        // settle all dispatches.
        let mut completed = 0;
        let mut failed = 0;
        for _ in 0..3 {
            let report = tick(&mut bus, &workers);
            match &report.outcome {
                TickOutcome::Executed { commit, .. } => {
                    completed += commit.completed.len();
                    failed += commit.failed.len();
                }
                other => panic!("expected executed, got {other:?}"),
            }
        }
        let grown = (bus.arenas.literals.allocated() - literals_before) as usize;
        let bytes = Snapshot::capture(&bus).bytes().to_vec();
        (bytes, completed, failed, grown)
    }
    let (first, completed, failed, grown) = run_once();
    assert_eq!((completed, failed, grown), (3, 0, 2));
    let (second, completed, failed, _) = run_once();
    assert_eq!((completed, failed), (3, 0));
    assert_eq!(first, second);
    // Committed records carry the frozen shapes: Character radix 16,
    // String radix 0 with the terminator, both with token back-links.
    let mut bus = new_bus();
    install_lx_string(&mut bus);
    let bus_workers = workers();
    let base = bus.arenas.literals.allocated();
    let (token, pp) = seed_literal(&mut bus, b"'Z'", PpTokenKind::CharLiteral);
    bootstrap(
        &mut bus,
        TaskKind::LEX_CHAR_DECODE,
        LX12_CHIP,
        Payload::from_refs(vec![token, pp]),
    );
    let (token, pp) = seed_literal(&mut bus, b"\"Z\"", PpTokenKind::StringLiteral);
    bootstrap(
        &mut bus,
        TaskKind::LEX_STRING_DECODE,
        LX13_CHIP,
        Payload::from_refs(vec![token, pp]),
    );
    for _ in 0..2 {
        assert!(matches!(
            tick(&mut bus, &bus_workers).outcome,
            TickOutcome::Executed { .. }
        ));
    }
    let char_record = bus
        .arenas
        .literals
        .get(LiteralId::from_index(base))
        .expect("char literal committed");
    assert_eq!(char_record.kind, LiteralKind::Character);
    assert_eq!(char_record.value, vec![0x5A]);
    assert_eq!(char_record.radix, 16);
    assert!(char_record.token.is_some());
    let string_record = bus
        .arenas
        .literals
        .get(LiteralId::from_index(base + 1))
        .expect("string literal committed");
    assert_eq!(string_record.kind, LiteralKind::String);
    assert_eq!(string_record.value, vec![0x5A, 0x00]);
    assert_eq!(string_record.radix, 0);
    assert!(string_record.token.is_some());
    // The escape dispatch completes `Ack` and appends nothing.
    let mut bus = new_bus();
    install_lx_string(&mut bus);
    let escape_workers = workers();
    let literals_before = bus.arenas.literals.allocated();
    let (token, pp) = seed_literal(&mut bus, b"\"\\n\"", PpTokenKind::StringLiteral);
    bootstrap(
        &mut bus,
        TaskKind::LEX_ESCAPE_DECODE,
        LX11_CHIP,
        Payload::from_refs(vec![token, pp]),
    );
    match tick(&mut bus, &escape_workers).outcome {
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
fn string_stage_layer_manifest_gates() {
    use cc_silicon_compiler::manifest::{ManifestRegistry, StoreSchema};
    // A wrong layer is refused on the driver path for all three kinds.
    for (kind, chip, worker) in [
        (
            TaskKind::LEX_ESCAPE_DECODE,
            LX11_CHIP,
            &LxEscapeChip as &dyn Worker,
        ),
        (
            TaskKind::LEX_CHAR_DECODE,
            LX12_CHIP,
            &LxCharChip as &dyn Worker,
        ),
        (
            TaskKind::LEX_STRING_DECODE,
            LX13_CHIP,
            &LxStringChip as &dyn Worker,
        ),
    ] {
        let mut mismatched = new_bus();
        mismatched.kinds = TaskKindRegistry::lx_string_slice();
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
    // The pre-`/33` registry knows none of the three kinds, so all
    // manifests are rejected there (frozen kinds are never re-registered).
    let mut stale = ManifestRegistry::new();
    let kinds = TaskKindRegistry::lx_float_slice();
    let schema = StoreSchema::pp_macro_slice();
    for worker in [&LxEscapeChip as &dyn Worker, &LxCharChip, &LxStringChip] {
        assert!(stale.register(worker.manifest(), &schema, &kinds).is_err());
    }
}
