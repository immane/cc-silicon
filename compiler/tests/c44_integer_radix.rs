// ============================================================================
// c44_integer_radix.rs — integer radix regression for LX05–LX08 (D-18).
//
// Pure decode tests with hand-derived C11 integer values. `010` is octal 8,
// `0x10` is hexadecimal 16, and malformed forms fail explicitly. The test
// does not calculate expected values through the decoder.
// ============================================================================

use cc_silicon_compiler::bus::TokenKind;
use cc_silicon_compiler::chips::lex::{LxDecodeInput, LxDecodeLiteralChip};
use cc_silicon_compiler::ids::{TaskId, TokenId};
use cc_silicon_compiler::records::G1DraftBody;
use cc_silicon_compiler::task::{Proposal, TaskState};

fn input(spelling: &[u8]) -> LxDecodeInput {
    LxDecodeInput {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        token: TokenId::from_index(0),
        token_kind: TokenKind::Integer,
        spelling: spelling.to_vec(),
        literals_allocated: 0,
    }
}

fn decoded(spelling: &[u8]) -> (u8, Vec<u8>) {
    let proposals = LxDecodeLiteralChip.compute(&input(spelling));
    let Proposal::AppendRecords { batch, .. } = &proposals[0] else {
        panic!("expected AppendRecords for {:?}: {proposals:?}", spelling);
    };
    let G1DraftBody::Literal(record) = &batch.bodies[0] else {
        panic!(
            "expected Literal body for {:?}: {:?}",
            spelling, batch.bodies
        );
    };
    assert_eq!(record.radix, expected_radix(spelling));
    assert_eq!(record.value, expected_value(spelling));
    (record.radix, record.value.clone())
}

fn expected_radix(spelling: &[u8]) -> u8 {
    match spelling {
        b"010" | b"077" => 8,
        b"0" => 10,
        b"0x10" | b"0X1f" | b"0x100" => 16,
        _ => 10,
    }
}

fn expected_value(spelling: &[u8]) -> Vec<u8> {
    match spelling {
        b"10" => vec![10],
        b"0" => vec![0],
        b"010" => vec![8],
        b"077" => vec![63],
        b"0x10" => vec![16],
        b"0X1f" => vec![31],
        b"0x100" => vec![1, 0],
        other => panic!("no hand-derived expected value for {other:?}"),
    }
}

#[test]
fn decodes_decimal_octal_and_hex_values() {
    for spelling in [
        b"10".as_slice(),
        b"0",
        b"010",
        b"077",
        b"0x10",
        b"0X1f",
        b"0x100",
    ] {
        let (radix, value) = decoded(spelling);
        assert_eq!(radix, expected_radix(spelling));
        assert_eq!(value, expected_value(spelling));
    }
}

#[test]
fn rejects_malformed_radix_forms_explicitly() {
    for spelling in [
        b"08".as_slice(),
        b"09",
        b"0x",
        b"0xg",
        b"2.5",
        b"10u",
        b"1e3",
    ] {
        let proposals = LxDecodeLiteralChip.compute(&input(spelling));
        assert!(
            matches!(proposals.as_slice(), [Proposal::Fail { .. }]),
            "expected Fail for {:?}, got {proposals:?}",
            spelling
        );
    }
}

#[test]
fn decode_is_deterministic() {
    for spelling in [b"010".as_slice(), b"0x100", b"10"] {
        assert_eq!(decoded(spelling), decoded(spelling));
    }
}
