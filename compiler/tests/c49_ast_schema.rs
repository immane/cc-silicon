// ============================================================================
// c49_ast_schema.rs — `/43` parser/AST common-base schema acceptance.
//
// Pins the append-only V1 NodeKind inventory, its canonical snapshot names,
// and wire round-trips. It does not claim a V1 parser producer or VF05 V1
// shape coverage; those belong to later parser/verification slices.
// ============================================================================

use cc_silicon_compiler::bus::{NodeKind, NodeRecord};
use cc_silicon_compiler::contract::NODE_KIND_NAMES;
use cc_silicon_compiler::ids::{NameId, NodeId, TokenId};
use cc_silicon_compiler::snapshot::{decode_node, encode_node, node_kind_name};

fn fixture(kind: NodeKind) -> NodeRecord {
    NodeRecord {
        kind,
        parent: None,
        children: vec![NodeId::from_index(1)],
        first_token: TokenId::from_index(0),
        last_token: TokenId::from_index(1),
        name: if kind == NodeKind::Identifier {
            Some(NameId::from_index(0))
        } else {
            None
        },
        literal: None,
    }
}

#[test]
fn v1_node_kind_inventory_appends_after_m1() {
    // `/39` order is preserved exactly; `/43` only appends.
    assert_eq!(
        &NODE_KIND_NAMES[..8],
        &[
            "translation_unit",
            "function_definition",
            "specifiers",
            "declarator",
            "compound",
            "return",
            "binary_add",
            "int_literal",
        ]
    );
    assert_eq!(
        &NODE_KIND_NAMES[8..],
        &[
            "declaration",
            "identifier",
            "expression_statement",
            "assignment",
            "binary_sub",
            "binary_mul",
            "binary_div",
            "binary_rem",
            "unary_plus",
            "unary_minus",
        ]
    );
}

#[test]
fn every_v1_node_kind_round_trips_the_canonical_codec() {
    for kind in [
        NodeKind::Declaration,
        NodeKind::Identifier,
        NodeKind::ExpressionStatement,
        NodeKind::Assignment,
        NodeKind::BinarySub,
        NodeKind::BinaryMul,
        NodeKind::BinaryDiv,
        NodeKind::BinaryRem,
        NodeKind::UnaryPlus,
        NodeKind::UnaryMinus,
    ] {
        let record = fixture(kind);
        assert_eq!(node_kind_name(kind), NODE_KIND_NAMES[kind as usize]);
        assert_eq!(decode_node(&encode_node(&record)).unwrap(), record);
    }
}
