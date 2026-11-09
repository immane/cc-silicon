// ============================================================================
// sprint_l02.rs — L02 zero-literal decode contract.
//
// The accepted zero fix canonicalizes unsuffixed zero magnitude to decimal
// radix while preserving the original spelling. `0` and `00` were previously
// classified as octal zero, and `0x0` as hexadecimal zero; all must now carry
// radix 10. Nonzero spellings keep their lexical radix, and malformed or
// suffixed forms remain explicit failures.
// ============================================================================

use cc_silicon_compiler::bus::{LiteralRecord, TokenKind};
use cc_silicon_compiler::chips::lex::{LxDecodeInput, LxDecodeLiteralChip};
use cc_silicon_compiler::ids::{TaskId, TokenId};
use cc_silicon_compiler::records::G1DraftBody;
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use cc_silicon_compiler::task::{Proposal, TaskState};

const TASK_INDEX: u32 = 7;
const TOKEN_INDEX: u32 = 11;
const LITERALS_ALLOCATED: u32 = 3;

fn input(spelling: &[u8]) -> LxDecodeInput {
    LxDecodeInput {
        task: TaskId::from_index(TASK_INDEX),
        state: TaskState::Running,
        token: TokenId::from_index(TOKEN_INDEX),
        token_kind: TokenKind::Integer,
        spelling: spelling.to_vec(),
        literals_allocated: LITERALS_ALLOCATED,
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
    assert_eq!(batch.records.len(), 1);
    assert_eq!(batch.bodies.len(), 1);
    let G1DraftBody::Literal(record) = &batch.bodies[0] else {
        panic!("expected Literal body for {spelling:?}: {:?}", batch.bodies);
    };
    record.clone()
}

fn assert_zero_semantics(spelling: &[u8]) {
    let record = decoded(spelling);
    assert_eq!(
        record.radix, 10,
        "zero spelling {spelling:?} must normalize to decimal radix"
    );
    assert_eq!(record.value, vec![0], "zero spelling {spelling:?}");
    assert_eq!(record.spelling, spelling, "spelling must be preserved");
    assert_eq!(record.kind, LiteralKind::Integer);
    assert_eq!(record.suffix, LiteralSuffix::None);
    assert_eq!(record.candidate_type, Lx08CandidateType::Int);
    assert!(!record.negative);
    assert_eq!(record.token, Some(TokenId::from_index(TOKEN_INDEX)));
}

#[test]
fn zero_spellings_normalize_to_decimal_radix() {
    for spelling in [
        b"0".as_slice(),
        b"00",
        b"0x0",
        b"0X0",
        b"000",
        b"0x00",
        b"0X00",
    ] {
        assert_zero_semantics(spelling);
    }
}

#[test]
fn zero_spellings_preserve_their_exact_bytes() {
    for spelling in [b"0".as_slice(), b"00", b"0x0"] {
        let record = decoded(spelling);
        assert_eq!(record.spelling, spelling);
        assert_eq!(record.radix, 10);
        assert_eq!(record.value, vec![0]);
    }
}

#[test]
fn nonzero_spellings_keep_their_lexical_radix() {
    for (spelling, radix, value) in [
        (b"10".as_slice(), 10_u8, vec![10_u8]),
        (b"010", 8, vec![8]),
        (b"0x10", 16, vec![16]),
    ] {
        let record = decoded(spelling);
        assert_eq!(record.radix, radix, "spelling {spelling:?}");
        assert_eq!(record.value, value, "spelling {spelling:?}");
        assert_eq!(record.spelling, spelling);
    }
}

#[test]
fn invalid_zero_like_forms_fail_explicitly() {
    for spelling in [
        b"08".as_slice(),
        b"09",
        b"019",
        b"0x",
        b"0X",
        b"0xg",
        b"0x0g",
        b"0b0",
        b"0B0",
        b"0u",
        b"00U",
        b"0x0u",
        b"0L",
        b"",
        b"+0",
        b"-0",
        b" 0",
        b"0 ",
        b"0.0",
        b"0e0",
        b"0x_0",
    ] {
        let proposals = LxDecodeLiteralChip.compute(&input(spelling));
        assert!(
            matches!(proposals.as_slice(), [Proposal::Fail { .. }]),
            "expected one Fail for {spelling:?}, got {proposals:?}"
        );
        assert!(
            !proposals
                .iter()
                .any(|proposal| matches!(proposal, Proposal::AppendRecords { .. })),
            "invalid spelling {spelling:?} must not append a literal"
        );
    }
}
