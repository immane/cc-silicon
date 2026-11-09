// ============================================================================
// sprint_l06.rs — L06 integer-literal negative space and const-bit boundary.
//
// Scope: octal/hex forms outside the integer grammar, suffixed spellings,
// nonzero octal/hex staying outside the decimal-only T08 fold subset, and the
// inclusive constant-bit budget boundary. Zero spellings belong to L02 and are
// intentionally absent.
//
// The accepted zero fix (L01) normalizes only zero-valued octal/hex spellings
// to radix 10; nonzero spellings keep their lexical radix. These cases are
// therefore independent of that normalization and are expected to be green at
// both the `/55` baseline and the post-L01 tree.
//
// Every expected value is hand-derived from the decimal/octal/hex notation;
// the decoder and folder are never used to derive their own oracle.
// ============================================================================

use std::collections::{BTreeMap, BTreeSet};

use cc_silicon_compiler::bus::{
    ConstValue, IntRank, LiteralRecord, TokenKind, TypeKind, TypeRecord,
};
use cc_silicon_compiler::chips::{
    const_bits_required, FoldChip, FoldInput, LxDecodeInput, LxDecodeLiteralChip,
};
use cc_silicon_compiler::diagnostic::{DiagGroup, DiagnosticCode};
use cc_silicon_compiler::ids::{LiteralId, RecordRef, TaskId, TokenId, TypeId};
use cc_silicon_compiler::records::G1DraftBody;
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use cc_silicon_compiler::task::{Payload, Proposal, TaskKind, TaskState};

const TASK_INDEX: u32 = 3;
const TOKEN_INDEX: u32 = 5;

fn input(spelling: &[u8]) -> LxDecodeInput {
    LxDecodeInput {
        task: TaskId::from_index(TASK_INDEX),
        state: TaskState::Running,
        token: TokenId::from_index(TOKEN_INDEX),
        token_kind: TokenKind::Integer,
        spelling: spelling.to_vec(),
        literals_allocated: 0,
    }
}

fn decoded(spelling: &[u8]) -> LiteralRecord {
    let proposals = LxDecodeLiteralChip.compute(&input(spelling));
    assert_eq!(
        proposals.len(),
        2,
        "expected append+complete for {spelling:?}: {proposals:?}"
    );
    let Proposal::AppendRecords { task, batch } = &proposals[0] else {
        panic!("expected AppendRecords for {spelling:?}: {proposals:?}");
    };
    assert_eq!(*task, TaskId::from_index(TASK_INDEX));
    assert_eq!(batch.bodies.len(), 1, "one literal body for {spelling:?}");
    let G1DraftBody::Literal(record) = &batch.bodies[0] else {
        panic!("expected Literal body for {spelling:?}: {:?}", batch.bodies);
    };
    record.clone()
}

fn assert_decode_unsupported(spelling: &[u8]) {
    let proposals = LxDecodeLiteralChip.compute(&input(spelling));
    let [Proposal::Fail { task, diagnostic }] = proposals.as_slice() else {
        panic!("expected one decode Fail for {spelling:?}: {proposals:?}");
    };
    assert_eq!(*task, TaskId::from_index(TASK_INDEX));
    assert_eq!(
        diagnostic.code.group,
        DiagGroup::Unsupported,
        "decode of {spelling:?} must be an explicit unsupported failure"
    );
}

fn fold_input(record: &LiteralRecord, max_const_bits: u32) -> FoldInput {
    let literal = LiteralId::from_index(0);
    let mut literals = BTreeMap::new();
    literals.insert(literal, record.clone());
    FoldInput {
        task: TaskId::from_index(0),
        kind: TaskKind::CONSTANT_CONST_FOLD,
        state: TaskState::Running,
        payload: Payload::from_refs(vec![RecordRef::Literal(literal)]),
        literals,
        nodes_present: BTreeSet::new(),
        consts_allocated: 0,
        constant_type: Some((
            TypeId::from_index(0),
            TypeRecord {
                kind: TypeKind::Int {
                    rank: IntRank::Int,
                    signed: true,
                },
            },
        )),
        max_const_bits,
    }
}

fn assert_fold_unsupported(record: &LiteralRecord) {
    let proposals = FoldChip.compute(&fold_input(record, 128));
    let [Proposal::Fail { diagnostic, .. }] = proposals.as_slice() else {
        panic!(
            "expected fold Fail for spelling {:?}: {proposals:?}",
            record.spelling
        );
    };
    assert_eq!(
        diagnostic.code.group,
        DiagGroup::Unsupported,
        "spelling {:?} must stay outside the decimal-only fold subset",
        record.spelling
    );
}

fn assert_fold_accepts(record: &LiteralRecord, max_const_bits: u32) {
    let proposals = FoldChip.compute(&fold_input(record, max_const_bits));
    assert_eq!(
        proposals.len(),
        2,
        "expected append+complete for spelling {:?} at budget {max_const_bits}: {proposals:?}",
        record.spelling
    );
    let Proposal::AppendRecords { batch, .. } = &proposals[0] else {
        panic!(
            "expected AppendRecords for spelling {:?} at budget {max_const_bits}: {proposals:?}",
            record.spelling
        );
    };
    assert_eq!(batch.bodies.len(), 1);
    let G1DraftBody::Const(constant) = &batch.bodies[0] else {
        panic!(
            "expected Const body for spelling {:?}: {:?}",
            record.spelling, batch.bodies
        );
    };
    let ConstValue::Integer(integer) = &constant.value else {
        panic!(
            "expected integer constant for spelling {:?}: {:?}",
            record.spelling, constant.value
        );
    };
    assert_eq!(integer.magnitude, record.value);
    assert!(!integer.negative);
    assert_eq!(constant.origins, vec![LiteralId::from_index(0)]);
    assert!(
        matches!(proposals[1], Proposal::Complete { .. }),
        "expected Complete after the append for spelling {:?}: {proposals:?}",
        record.spelling
    );
}

fn assert_fold_overflow(record: &LiteralRecord, max_const_bits: u32) {
    let proposals = FoldChip.compute(&fold_input(record, max_const_bits));
    let [Proposal::Fail { diagnostic, .. }] = proposals.as_slice() else {
        panic!(
            "expected one overflow Fail for spelling {:?} at budget {max_const_bits}: {proposals:?}",
            record.spelling
        );
    };
    assert_eq!(
        diagnostic.code,
        DiagnosticCode::CONST_OVERFLOW,
        "spelling {:?} at budget {max_const_bits}",
        record.spelling
    );
}

#[test]
fn malformed_octal_spellings_are_rejected() {
    let cases: &[&[u8]] = &[b"078", b"0778", b"0_7", b"0o10", b"0O7", b"08a"];
    for &spelling in cases {
        assert_decode_unsupported(spelling);
    }
}

#[test]
fn hex_forms_outside_the_integer_subset_are_rejected() {
    let cases: &[&[u8]] = &[
        b"0x1g", b"0XG", b"0x_1", b"0x1_0", b"0xx1", b"0x1p3", b"0x.8", b"0x1z", b"0x+1",
    ];
    for &spelling in cases {
        assert_decode_unsupported(spelling);
    }
}

#[test]
fn suffixed_integer_spellings_are_rejected() {
    let cases: &[&[u8]] = &[
        b"10u", b"10U", b"10l", b"10L", b"10ul", b"10uL", b"10Ul", b"10UL", b"10lu", b"10LU",
        b"10ll", b"10LL", b"10ull", b"10ULL", b"10llu", b"10LLU", b"10uu", b"10lll", b"10lul",
        b"10u8", b"010u", b"010UL", b"0x10u", b"0x10ULL",
    ];
    for &spelling in cases {
        assert_decode_unsupported(spelling);
    }
}

#[test]
fn spellings_outside_the_integer_grammar_are_rejected() {
    let cases: &[&[u8]] = &[
        b"1e3", b"1E+3", b"2.5", b".5", b"1.", b"0b101", b"0B11", b"1'000", b"0x1'0", b"123abc",
        b"_1", b"abc", b"", b"+1", b"-1", b" 1", b"1 ",
    ];
    for &spelling in cases {
        assert_decode_unsupported(spelling);
    }
}

#[test]
fn nonzero_octal_and_hex_keep_lexical_provenance() {
    let cases: &[(&[u8], u8, &[u8])] = &[
        (b"07", 8, &[7]),
        (b"010", 8, &[8]),
        (b"077", 8, &[63]),
        (b"0x10", 16, &[16]),
        (b"0X1f", 16, &[31]),
        (b"0x100", 16, &[1, 0]),
        (b"0xdeadBEEF", 16, &[0xde, 0xad, 0xbe, 0xef]),
    ];
    for &(spelling, radix, value) in cases {
        let record = decoded(spelling);
        assert_eq!(record.kind, LiteralKind::Integer);
        assert_eq!(record.radix, radix, "spelling {spelling:?}");
        assert_eq!(record.suffix, LiteralSuffix::None);
        assert_eq!(record.value, value.to_vec(), "spelling {spelling:?}");
        assert_eq!(record.spelling, spelling.to_vec());
        assert_eq!(record.candidate_type, Lx08CandidateType::Int);
        assert_eq!(record.token, Some(TokenId::from_index(TOKEN_INDEX)));
        assert!(!record.negative);
    }
}

#[test]
fn nonzero_octal_and_hex_remain_outside_the_fold_subset() {
    let cases: &[&[u8]] = &[b"07", b"010", b"077", b"0x10", b"0X1f", b"0x100"];
    for &spelling in cases {
        let record = decoded(spelling);
        assert_fold_unsupported(&record);
    }
}

#[test]
fn const_bit_budget_accepts_exactly_the_limit_and_rejects_one_more_bit() {
    // 2^127 - 1 (127 bits) is accepted at budget 127; 2^127 (128 bits) is not.
    let max_127 = decoded(b"170141183460469231731687303715884105727");
    assert_eq!(const_bits_required(&max_127.value), 127);
    assert_fold_accepts(&max_127, 127);

    let at_128 = decoded(b"170141183460469231731687303715884105728");
    assert_eq!(const_bits_required(&at_128.value), 128);
    assert_fold_overflow(&at_128, 127);

    // The configured maximum is 128: 2^128 - 1 fits, 2^128 needs 129 bits.
    let max_128 = decoded(b"340282366920938463463374607431768211455");
    assert_eq!(const_bits_required(&max_128.value), 128);
    assert_fold_accepts(&max_128, 128);

    let over_128 = decoded(b"340282366920938463463374607431768211456");
    assert_eq!(const_bits_required(&over_128.value), 129);
    assert_fold_overflow(&over_128, 128);

    // The smallest valid budget: one bit admits 1 and rejects 2.
    let one = decoded(b"1");
    assert_eq!(const_bits_required(&one.value), 1);
    assert_fold_accepts(&one, 1);

    let two = decoded(b"2");
    assert_eq!(const_bits_required(&two.value), 2);
    assert_fold_overflow(&two, 1);
}
