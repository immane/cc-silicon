// ============================================================================
// sprint_t27.rs — T27: uninitialized locals, missing constants, stale IDs,
// and invalid origin links.
//
// Independent, synthetic checks for the frozen `/55` verifier inputs. The
// hand-derived controls are:
//
//   * `int x = 2; return x;` models 2;
//   * `return 7 + 1;` models 8;
//   * `return 4;` completes the committed `Const` for 4;
//   * the VF05 literal leaf's committed origin token is the leaf token.
//
// Each negative changes only the named link. A verifier that guesses,
// defaults, or accepts an uncommitted reference would return a value or an
// `Ack`, so the contract requires exactly one typed failure instead.
// ============================================================================

use cc_silicon_compiler::bus::{
    BlockRecord, ConstRecord, ConstValue, FunctionRecord, InstructionRecord, IntRank, IntegerConst,
    IrOp, Linkage, LiteralRecord, NodeKind, NodeRecord, StorageDuration, SymbolKind, SymbolRecord,
    TokenKind, TokenRecord, TypeKind, TypeRecord, ValueRecord,
};
use cc_silicon_compiler::chips::{Vf05Chip, Vf05Input, Vf12Chip, Vf12Input};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{
    BlockId, ConstId, FunctionId, InstructionId, LiteralId, NameId, NodeId, PpTokenId, RecordRef,
    ScopeId, SpanId, SymbolId, TaskId, TokenId, TypeId, ValueId,
};
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use cc_silicon_compiler::task::{Proposal, ResultValue, TaskState};

const INT: u32 = 0;
const SIGNATURE: u32 = 1;

fn int_type() -> TypeRecord {
    TypeRecord {
        kind: TypeKind::Int {
            rank: IntRank::Int,
            signed: true,
        },
    }
}

fn const_record(value: i32) -> ConstRecord {
    ConstRecord {
        ty: TypeId::from_index(INT),
        value: ConstValue::Integer(IntegerConst::new(
            value.unsigned_abs().to_be_bytes().to_vec(),
            value < 0,
        )),
        origins: Vec::new(),
    }
}

fn constant(index: u32, value: i32) -> (ConstId, ConstRecord) {
    (ConstId::from_index(index), const_record(value))
}

fn local_symbol() -> SymbolRecord {
    SymbolRecord {
        name: NameId::from_index(0),
        scope: ScopeId::from_index(0),
        kind: SymbolKind::Object,
        ty: Some(TypeId::from_index(INT)),
        linkage: Linkage::None,
        storage: StorageDuration::Automatic,
        decl: NodeId::from_index(0),
    }
}

fn instruction(
    op: IrOp,
    operands: Vec<ValueId>,
    immediate: Option<ConstId>,
    result: Option<ValueId>,
    local: Option<SymbolId>,
) -> InstructionRecord {
    InstructionRecord {
        op,
        block: BlockId::from_index(0),
        operands,
        immediate,
        result,
        local,
        successors: Vec::new(),
        callee: None,
    }
}

/// Build one committed straight-line function whose instruction IDs are the
/// vector order. `modeled` and `ret_value` are the legacy `/17` staging
/// markers; `None` uses the already-frozen sentinel shape.
fn vf12_program(
    instructions: Vec<InstructionRecord>,
    consts: Vec<(ConstId, ConstRecord)>,
    symbols: Vec<(SymbolId, SymbolRecord)>,
    modeled: Option<(ConstId, ConstRecord)>,
    ret_value: Option<(ValueId, ValueRecord)>,
) -> Vf12Input {
    let int = TypeId::from_index(INT);
    let signature = TypeId::from_index(SIGNATURE);
    let function = FunctionId::from_index(0);
    let block = BlockId::from_index(0);
    let function_body = FunctionRecord {
        symbol: SymbolId::from_index(1),
        signature,
        entry: block,
        linkage: Linkage::External,
        parameters: Vec::new(),
    };
    let block_body = BlockRecord {
        function,
        ordinal: 0,
    };
    let mut listed = Vec::with_capacity(instructions.len());
    let mut values = Vec::new();
    for (index, body) in instructions.into_iter().enumerate() {
        if let Some(result) = body.result {
            values.push((result, ValueRecord { ty: int }));
        }
        listed.push((InstructionId::from_index(index as u32), body));
    }
    Vf12Input {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        function,
        function_body: function_body.clone(),
        blocks: vec![(block, block_body.clone())],
        instructions: listed.clone(),
        ret_value: ret_value
            .unwrap_or_else(|| (ValueId::from_index(u32::MAX), ValueRecord { ty: int })),
        modeled: modeled.unwrap_or_else(|| (ConstId::from_index(u32::MAX), const_record(0))),
        int,
        consts,
        functions: vec![(function, function_body)],
        all_blocks: vec![(block, block_body)],
        all_instructions: listed,
        values,
        types: vec![
            (TypeId::from_index(INT), int_type()),
            (
                TypeId::from_index(SIGNATURE),
                TypeRecord {
                    kind: TypeKind::Function {
                        result: TypeId::from_index(INT),
                        params: Vec::new(),
                        prototype: true,
                        variadic: false,
                    },
                },
            ),
        ],
        symbols,
    }
}

fn integer_value(integer: &IntegerConst) -> i128 {
    let mut magnitude: u128 = 0;
    for byte in &integer.magnitude {
        magnitude = (magnitude << 8) | u128::from(*byte);
    }
    if integer.negative {
        -(magnitude as i128)
    } else {
        magnitude as i128
    }
}

fn expect_integer(proposals: &[Proposal], expected: i128) {
    match proposals {
        [Proposal::Complete {
            value: ResultValue::Integer(integer),
            ..
        }] => assert_eq!(integer_value(integer), expected),
        other => panic!("expected one Integer result, got {other:?}"),
    }
}

fn expect_const(proposals: &[Proposal], expected: ConstId) {
    match proposals {
        [Proposal::Complete {
            value: ResultValue::Record(RecordRef::Const(id)),
            ..
        }] => assert_eq!(*id, expected),
        other => panic!("expected one Const record result, got {other:?}"),
    }
}

fn expect_failure(proposals: &[Proposal], expected: &str) {
    match proposals {
        [Proposal::Fail { diagnostic, .. }] => {
            assert_eq!(
                diagnostic.code.group,
                DiagGroup::Task,
                "wrong diagnostic group: {}",
                diagnostic.message
            );
            if !expected.is_empty() {
                assert!(
                    diagnostic.message.contains(expected),
                    "expected {expected:?}, got {:?}",
                    diagnostic.message
                );
            }
        }
        other => panic!("expected one typed failure, got {other:?}"),
    }
}

#[test]
fn initialized_local_models_two_and_uninitialized_read_fails() {
    let value = ValueId::from_index(0);
    let loaded = ValueId::from_index(1);
    let local = SymbolId::from_index(0);
    let two = constant(0, 2);

    let initialized = vf12_program(
        vec![
            instruction(IrOp::Constant, Vec::new(), Some(two.0), Some(value), None),
            instruction(IrOp::LocalStore, vec![value], None, None, Some(local)),
            instruction(IrOp::LocalLoad, Vec::new(), None, Some(loaded), Some(local)),
            instruction(IrOp::Return, vec![loaded], None, None, None),
        ],
        vec![two.clone()],
        vec![(local, local_symbol())],
        None,
        None,
    );
    // Hand calculation: the stored initializer is 2.
    expect_integer(&Vf12Chip.compute(&initialized), 2);

    let uninitialized = vf12_program(
        vec![
            instruction(IrOp::Constant, Vec::new(), Some(two.0), Some(value), None),
            instruction(IrOp::LocalLoad, Vec::new(), None, Some(loaded), Some(local)),
            instruction(IrOp::Return, vec![loaded], None, None, None),
        ],
        vec![two],
        vec![(local, local_symbol())],
        None,
        None,
    );
    expect_failure(&Vf12Chip.compute(&uninitialized), "uninitialized local");
}

#[test]
fn missing_constant_operand_fails_instead_of_guessing() {
    let missing = ConstId::from_index(7);
    let one = constant(1, 1);
    let seven = constant(7, 7);
    let lhs = ValueId::from_index(0);
    let rhs = ValueId::from_index(1);
    let sum = ValueId::from_index(2);

    let complete = vf12_program(
        vec![
            instruction(IrOp::Constant, Vec::new(), Some(seven.0), Some(lhs), None),
            instruction(IrOp::Constant, Vec::new(), Some(one.0), Some(rhs), None),
            instruction(IrOp::Add, vec![lhs, rhs], None, Some(sum), None),
            instruction(IrOp::Return, vec![sum], None, None, None),
        ],
        vec![seven, one.clone()],
        Vec::new(),
        None,
        None,
    );
    // Hand calculation: 7 + 1 = 8.
    expect_integer(&Vf12Chip.compute(&complete), 8);

    let broken = vf12_program(
        vec![
            instruction(IrOp::Constant, Vec::new(), Some(missing), Some(lhs), None),
            instruction(IrOp::Constant, Vec::new(), Some(one.0), Some(rhs), None),
            instruction(IrOp::Add, vec![lhs, rhs], None, Some(sum), None),
            instruction(IrOp::Return, vec![sum], None, None, None),
        ],
        vec![one],
        Vec::new(),
        None,
        None,
    );
    expect_failure(&Vf12Chip.compute(&broken), "missing const");
}

#[test]
fn canonical_return_rejects_uncommitted_constant() {
    let four = ConstId::from_index(4);
    let value = ValueId::from_index(0);
    let instructions = vec![
        instruction(IrOp::Constant, Vec::new(), Some(four), Some(value), None),
        instruction(IrOp::Return, vec![value], None, None, None),
    ];
    let markers = Some((four, const_record(4)));
    let returned = Some((
        value,
        ValueRecord {
            ty: TypeId::from_index(INT),
        },
    ));

    let committed = vf12_program(
        instructions.clone(),
        vec![(four, const_record(4))],
        Vec::new(),
        markers.clone(),
        returned.clone(),
    );
    // Hand calculation: the committed literal constant is 4.
    expect_const(&Vf12Chip.compute(&committed), four);

    // The same canonical shape names no committed constant. Completing the
    // record would fabricate the value 4; the verifier must fail instead.
    let uncommitted = vf12_program(instructions, Vec::new(), Vec::new(), markers, returned);
    expect_failure(&Vf12Chip.compute(&uncommitted), "");
}

#[test]
fn local_operation_rejects_stale_symbol_id() {
    let value = ValueId::from_index(0);
    let loaded = ValueId::from_index(1);
    let local = SymbolId::from_index(0);
    let two = constant(0, 2);
    let instructions = vec![
        instruction(IrOp::Constant, Vec::new(), Some(two.0), Some(value), None),
        instruction(IrOp::LocalStore, vec![value], None, None, Some(local)),
        instruction(IrOp::LocalLoad, Vec::new(), None, Some(loaded), Some(local)),
        instruction(IrOp::Return, vec![loaded], None, None, None),
    ];
    let mut stale = vf12_program(
        instructions,
        vec![two],
        vec![(local, local_symbol())],
        None,
        None,
    );
    let committed = stale.clone();
    // Control: with the symbol committed, the stored value 2 is modeled.
    expect_integer(&Vf12Chip.compute(&committed), 2);
    // A stale projection no longer carries the committed local symbol. The
    // interpreter must not let the store create an undeclared local.
    stale.symbols.clear();
    expect_failure(&Vf12Chip.compute(&stale), "");
}

fn vf05_node(
    kind: NodeKind,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    first: u32,
    last: u32,
    name: Option<NameId>,
    literal: Option<LiteralId>,
) -> NodeRecord {
    NodeRecord {
        kind,
        parent,
        children,
        first_token: TokenId::from_index(first),
        last_token: TokenId::from_index(last),
        name,
        literal,
    }
}

/// Canonical `int main(void){return 2;}`: TU 0, function 1, specifiers 2,
/// declarator 3, compound 4, return 5, literal leaf 6. Tokens 0..6 are
/// identifiers, token 6 is the literal, and token 7 is the trailing EOF.
fn vf05_fixture(literal_token: Option<TokenId>, include_literal: bool) -> Vf05Input {
    let name = NameId::from_index(0);
    let tokens: Vec<(TokenId, TokenRecord)> = (0..=7u32)
        .map(|index| {
            let eof = index == 7;
            (
                TokenId::from_index(index),
                TokenRecord {
                    kind: if eof {
                        TokenKind::Eof
                    } else if index == 6 {
                        TokenKind::Integer
                    } else {
                        TokenKind::Identifier
                    },
                    span: SpanId::from_index(0),
                    name: if eof { None } else { Some(name) },
                    pp_token: PpTokenId::from_index(0),
                },
            )
        })
        .collect();
    let literal = (
        LiteralId::from_index(0),
        LiteralRecord {
            token: literal_token,
            kind: LiteralKind::Integer,
            radix: 10,
            suffix: LiteralSuffix::None,
            value: vec![2],
            negative: false,
            spelling: b"2".to_vec(),
            candidate_type: Lx08CandidateType::Int,
        },
    );
    let literals = if include_literal {
        vec![literal]
    } else {
        Vec::new()
    };
    let tree = vec![
        (
            NodeId::from_index(0),
            vf05_node(
                NodeKind::TranslationUnit,
                None,
                vec![NodeId::from_index(1)],
                0,
                6,
                None,
                None,
            ),
        ),
        (
            NodeId::from_index(1),
            vf05_node(
                NodeKind::FunctionDefinition,
                Some(NodeId::from_index(0)),
                vec![
                    NodeId::from_index(2),
                    NodeId::from_index(3),
                    NodeId::from_index(4),
                ],
                0,
                6,
                None,
                None,
            ),
        ),
        (
            NodeId::from_index(2),
            vf05_node(
                NodeKind::Specifiers,
                Some(NodeId::from_index(1)),
                Vec::new(),
                0,
                0,
                None,
                None,
            ),
        ),
        (
            NodeId::from_index(3),
            vf05_node(
                NodeKind::Declarator,
                Some(NodeId::from_index(1)),
                Vec::new(),
                1,
                4,
                Some(name),
                None,
            ),
        ),
        (
            NodeId::from_index(4),
            vf05_node(
                NodeKind::Compound,
                Some(NodeId::from_index(1)),
                vec![NodeId::from_index(5)],
                5,
                6,
                None,
                None,
            ),
        ),
        (
            NodeId::from_index(5),
            vf05_node(
                NodeKind::Return,
                Some(NodeId::from_index(4)),
                vec![NodeId::from_index(6)],
                5,
                6,
                None,
                None,
            ),
        ),
        (
            NodeId::from_index(6),
            vf05_node(
                NodeKind::IntLiteral,
                Some(NodeId::from_index(5)),
                Vec::new(),
                6,
                6,
                None,
                Some(LiteralId::from_index(0)),
            ),
        ),
    ];
    Vf05Input {
        task: TaskId::from_index(0),
        state: TaskState::Running,
        tu: NodeId::from_index(0),
        tree,
        tokens,
        literals,
    }
}

#[test]
fn vf05_rejects_invalid_literal_origin_links() {
    let good = vf05_fixture(Some(TokenId::from_index(6)), true);
    match Vf05Chip.compute(&good).as_slice() {
        [Proposal::Complete {
            value: ResultValue::Ack,
            ..
        }] => {}
        other => panic!("expected one Ack, got {other:?}"),
    }

    for origin in [
        None,
        Some(TokenId::from_index(5)),
        Some(TokenId::from_index(99)),
    ] {
        let broken = vf05_fixture(origin, true);
        expect_failure(&Vf05Chip.compute(&broken), "origin");
    }

    // The leaf names LiteralId 0, but the literal record is absent.
    let missing = vf05_fixture(Some(TokenId::from_index(6)), false);
    expect_failure(&Vf05Chip.compute(&missing), "literal is not committed");
}
