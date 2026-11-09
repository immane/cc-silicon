// ============================================================================
// c50_ir_schema.rs — `/44` IR/CFG schema acceptance.
//
// Pins the append-only IR op inventory, the `local`/`successors` instruction
// fields, and their canonical wire round-trips. No new lowering producer or
// CFG verifier is claimed by this schema slice.
// ============================================================================

use cc_silicon_compiler::bus::{InstructionRecord, IrOp};
use cc_silicon_compiler::contract::{INSTRUCTION_RECORD_FIELDS, IR_OP_NAMES};
use cc_silicon_compiler::ids::{BlockId, SymbolId, ValueId};
use cc_silicon_compiler::snapshot::{decode_instruction, encode_instruction};

fn instruction(op: IrOp) -> InstructionRecord {
    InstructionRecord {
        op,
        block: BlockId::from_index(0),
        operands: vec![ValueId::from_index(0)],
        immediate: None,
        result: Some(ValueId::from_index(1)),
        local: Some(SymbolId::from_index(2)),
        successors: vec![BlockId::from_index(3), BlockId::from_index(4)],
    }
}

#[test]
fn ir_op_inventory_is_append_only() {
    assert_eq!(&IR_OP_NAMES[..2], &["constant", "return"]);
    assert_eq!(
        &IR_OP_NAMES[2..],
        &[
            "local_load",
            "local_store",
            "add",
            "sub",
            "mul",
            "div",
            "rem",
            "neg",
            "branch",
            "cond_branch",
        ]
    );
}

#[test]
fn instruction_fields_and_wire_round_trip() {
    assert_eq!(
        INSTRUCTION_RECORD_FIELDS,
        &[
            "op",
            "block",
            "operands",
            "immediate",
            "result",
            "local",
            "successors",
        ]
    );
    for op in [
        IrOp::LocalLoad,
        IrOp::LocalStore,
        IrOp::Add,
        IrOp::Sub,
        IrOp::Mul,
        IrOp::Div,
        IrOp::Rem,
        IrOp::Neg,
        IrOp::Branch,
        IrOp::CondBranch,
    ] {
        let record = instruction(op);
        assert_eq!(
            decode_instruction(&encode_instruction(&record)).unwrap(),
            record
        );
    }
}
