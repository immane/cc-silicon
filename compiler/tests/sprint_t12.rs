// ============================================================================
// sprint_t12.rs — T12 `/55` adversarial regression: mixed canonical binary-fold
// and literal-fold origins; ambiguous handoffs are rejected.
//
// The constant arena is built by hand. `ConstRecord.origins` is either
// `[literal]` (a literal fold) or `[lhs, rhs]` (a canonical binary fold).
// Canonical `return 2 + 3;` lowering must select the `[lhs, rhs]` record even
// when literal folds for both operands are present, and must reject duplicate,
// absent, or reversed binary-origin pairs instead of guessing by value or by
// record order. The general literal map must ignore binary-fold records while
// still rejecting two constants that claim the same literal origin.
//
// Hand-derived: 2 + 3 = 5. Nothing below is read back from the implementation
// under test.
// ============================================================================

use cc_silicon_compiler::bus::{
    CompilerBus, ConstRecord, ConstValue, EffectMask, InstructionRecord, IntRank, IntegerConst,
    IrOp, Linkage, LiteralRecord, NodeKind, NodeRecord, ScopeKind, ScopeRecord, SemRecord,
    StorageDuration, SymbolKind, SymbolRecord, TypeKind, TypeRecord, ValueCategory,
};
use cc_silicon_compiler::chips::{project_ir_function_input, IrFunctionChip, IrFunctionInput};
use cc_silicon_compiler::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use cc_silicon_compiler::ids::{
    ConstId, LiteralId, NameId, NodeId, RecordRef, ScopeId, SymbolId, TokenId, TypeId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::IR_FUNCTION_CHIP;
use cc_silicon_compiler::records::G1DraftBody;
use cc_silicon_compiler::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, Proposal, TaskDraft, TaskKind, TaskState};

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

struct Fixture {
    bus: CompilerBus,
    int: TypeId,
    file_scope: ScopeId,
}

impl Fixture {
    fn new() -> Self {
        let mut bus = new_bus();
        let limits = bus.limits();
        let int = bus
            .arenas
            .types
            .alloc(
                TypeRecord {
                    kind: TypeKind::Int {
                        rank: IntRank::Int,
                        signed: true,
                    },
                },
                &limits,
            )
            .unwrap();
        let file_scope = bus
            .arenas
            .scopes
            .alloc(
                ScopeRecord {
                    kind: ScopeKind::File,
                    parent: None,
                    owner: None,
                },
                &limits,
            )
            .unwrap();
        Self {
            bus,
            int,
            file_scope,
        }
    }

    fn name(&mut self, text: &[u8]) -> NameId {
        self.bus.intern_name(text).unwrap()
    }

    fn node_with_literal(
        &mut self,
        kind: NodeKind,
        children: Vec<NodeId>,
        name: Option<NameId>,
        literal: Option<LiteralId>,
    ) -> NodeId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .nodes
            .alloc(
                NodeRecord {
                    kind,
                    parent: None,
                    children,
                    first_token: TokenId::from_index(0),
                    last_token: TokenId::from_index(0),
                    name,
                    literal,
                },
                &limits,
            )
            .unwrap()
    }

    fn node(&mut self, kind: NodeKind, children: Vec<NodeId>, name: Option<NameId>) -> NodeId {
        self.node_with_literal(kind, children, name, None)
    }

    /// Commit one decimal `Int` literal and its `IntLiteral` leaf.
    fn literal(&mut self, value: u8) -> (NodeId, LiteralId) {
        let limits = self.bus.limits();
        let literal = self
            .bus
            .arenas
            .literals
            .alloc(
                LiteralRecord {
                    token: None,
                    kind: LiteralKind::Integer,
                    radix: 10,
                    suffix: LiteralSuffix::None,
                    value: vec![value],
                    negative: false,
                    spelling: value.to_string().into_bytes(),
                    candidate_type: Lx08CandidateType::Int,
                },
                &limits,
            )
            .unwrap();
        let node = self.node_with_literal(NodeKind::IntLiteral, vec![], None, Some(literal));
        (node, literal)
    }

    fn sem_with(&mut self, node: NodeId, ty: TypeId) {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .sem
            .alloc(
                SemRecord {
                    node,
                    ty,
                    category: ValueCategory::NonLvalue,
                    effects: EffectMask(0),
                },
                &limits,
            )
            .unwrap();
    }

    fn int_sem(&mut self, node: NodeId) {
        self.sem_with(node, self.int);
    }

    fn const_record(&mut self, value: u8, origins: Vec<LiteralId>) -> ConstId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .consts
            .alloc(
                ConstRecord {
                    ty: self.int,
                    value: ConstValue::Integer(IntegerConst::new(vec![value], false)),
                    origins,
                },
                &limits,
            )
            .unwrap()
    }

    fn function_type(&mut self, parameters: usize) -> TypeId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .types
            .alloc(
                TypeRecord {
                    kind: TypeKind::Function {
                        result: self.int,
                        params: vec![self.int; parameters],
                        prototype: true,
                        variadic: false,
                    },
                },
                &limits,
            )
            .unwrap()
    }

    fn function_symbol(&mut self, name: NameId, ty: TypeId, decl: NodeId) -> SymbolId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name,
                    scope: self.file_scope,
                    kind: SymbolKind::Function,
                    ty: Some(ty),
                    linkage: Linkage::External,
                    storage: StorageDuration::Static,
                    decl,
                },
                &limits,
            )
            .unwrap()
    }

    fn block_scope(&mut self, owner: NodeId) -> ScopeId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .scopes
            .alloc(
                ScopeRecord {
                    kind: ScopeKind::Block,
                    parent: Some(self.file_scope),
                    owner: Some(owner),
                },
                &limits,
            )
            .unwrap()
    }

    fn object_symbol(&mut self, name: NameId, scope: ScopeId, decl: NodeId) -> SymbolId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name,
                    scope,
                    kind: SymbolKind::Object,
                    ty: Some(self.int),
                    linkage: Linkage::None,
                    storage: StorageDuration::Automatic,
                    decl,
                },
                &limits,
            )
            .unwrap()
    }

    /// The canonical `int main(void) { return <lhs> + <rhs>; }` shape.
    fn canonical_add_function(&mut self, lhs: NodeId, rhs: NodeId) -> NodeId {
        let main = self.name(b"main");
        let sum = self.node(NodeKind::BinaryAdd, vec![lhs, rhs], None);
        self.int_sem(sum);
        let ret = self.node(NodeKind::Return, vec![sum], None);
        self.int_sem(ret);
        let specifiers = self.node(NodeKind::Specifiers, vec![], None);
        let declarator = self.node(NodeKind::Declarator, vec![], Some(main));
        let compound = self.node(NodeKind::Compound, vec![ret], None);
        let funcdef = self.node(
            NodeKind::FunctionDefinition,
            vec![specifiers, declarator, compound],
            None,
        );
        let signature = self.function_type(0);
        self.function_symbol(main, signature, declarator);
        self.sem_with(funcdef, signature);
        funcdef
    }

    /// `int main(void) { int x = 2; return x; }` — takes the general path so
    /// the literal-origin map, not the canonical selector, chooses constants.
    fn local_function(&mut self) -> (NodeId, LiteralId) {
        let main = self.name(b"main");
        let x = self.name(b"x");

        let declaration_specifiers = self.node(NodeKind::Specifiers, vec![], None);
        let declarator_x = self.node(NodeKind::Declarator, vec![], Some(x));
        let (initializer_node, initializer_literal) = self.literal(2);
        let declaration = self.node(
            NodeKind::Declaration,
            vec![declaration_specifiers, declarator_x, initializer_node],
            None,
        );

        let use_x = self.node(NodeKind::Identifier, vec![], Some(x));
        self.int_sem(use_x);
        let ret = self.node(NodeKind::Return, vec![use_x], None);
        self.int_sem(ret);
        let compound = self.node(NodeKind::Compound, vec![declaration, ret], None);

        let block = self.block_scope(compound);
        self.object_symbol(x, block, declarator_x);

        let specifiers = self.node(NodeKind::Specifiers, vec![], None);
        let declarator = self.node(NodeKind::Declarator, vec![], Some(main));
        let funcdef = self.node(
            NodeKind::FunctionDefinition,
            vec![specifiers, declarator, compound],
            None,
        );
        let signature = self.function_type(0);
        self.function_symbol(main, signature, declarator);
        self.sem_with(funcdef, signature);
        (funcdef, initializer_literal)
    }

    fn project(&mut self, funcdef: NodeId) -> Result<IrFunctionInput, DiagnosticDraft> {
        let task = self
            .bus
            .bootstrap_task(TaskDraft {
                kind: TaskKind::IR_FUNCTION,
                owner: IR_FUNCTION_CHIP,
                parent: None,
                payload: Payload::from_refs(vec![RecordRef::Node(funcdef)]),
                continuation: None,
            })
            .unwrap();
        self.bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
        project_ir_function_input(&self.bus, task)
    }

    fn project_error(&mut self, funcdef: NodeId) -> DiagnosticDraft {
        match self.project(funcdef) {
            Ok(_) => panic!("expected the mixed-origin handoff to be rejected"),
            Err(error) => error,
        }
    }
}

fn instructions(input: &IrFunctionInput) -> Vec<InstructionRecord> {
    let proposals = IrFunctionChip.compute(input);
    assert_eq!(proposals.len(), 2, "expected append + complete");
    let batch = match &proposals[0] {
        Proposal::AppendRecords { task, batch } => {
            assert_eq!(*task, input.task);
            batch
        }
        other => panic!("expected an append batch, got {other:?}"),
    };
    assert!(matches!(
        &proposals[1],
        Proposal::Complete { task, .. } if *task == input.task
    ));
    batch
        .bodies
        .iter()
        .filter_map(|body| match body {
            G1DraftBody::Instruction(instruction) => Some(instruction.clone()),
            _ => None,
        })
        .collect()
}

fn assert_rejects(error: &DiagnosticDraft, expected: &str) {
    assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Task, 4));
    assert!(
        error.message.contains(expected),
        "expected {expected:?} in {:?}",
        error.message
    );
}

fn assert_no_ir(fixture: &Fixture) {
    assert_eq!(fixture.bus.arenas.functions.allocated(), 0);
    assert_eq!(fixture.bus.arenas.blocks.allocated(), 0);
    assert_eq!(fixture.bus.arenas.values.allocated(), 0);
    assert_eq!(fixture.bus.arenas.instructions.allocated(), 0);
}

#[test]
fn canonical_binary_fold_wins_over_literal_folds() {
    let mut fixture = Fixture::new();
    let (lhs_node, lhs_literal) = fixture.literal(2);
    let (rhs_node, rhs_literal) = fixture.literal(3);
    let lhs_fold = fixture.const_record(2, vec![lhs_literal]);
    let rhs_fold = fixture.const_record(3, vec![rhs_literal]);
    let binary_fold = fixture.const_record(5, vec![lhs_literal, rhs_literal]);
    let funcdef = fixture.canonical_add_function(lhs_node, rhs_node);

    let input = fixture
        .project(funcdef)
        .expect("a unique [lhs, rhs] fold amid literal folds is unambiguous");
    assert_eq!(input.constant, binary_fold);
    assert_ne!(input.constant, lhs_fold);
    assert_ne!(input.constant, rhs_fold);

    let instructions = instructions(&input);
    assert_eq!(
        instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect::<Vec<_>>(),
        vec![IrOp::Constant, IrOp::Return]
    );
    assert_eq!(instructions[0].immediate, Some(binary_fold));
    assert_eq!(
        instructions[1].operands,
        vec![instructions[0].result.unwrap()]
    );

    let record = fixture.bus.arenas.consts.get(binary_fold).unwrap();
    assert_eq!(record.ty, fixture.int);
    assert_eq!(record.origins, vec![lhs_literal, rhs_literal]);
    let ConstValue::Integer(value) = &record.value else {
        panic!("the folded record must be an integer");
    };
    // 2 + 3 = 5, hand-derived.
    assert_eq!(value.magnitude, vec![5]);
    assert!(!value.negative);
}

#[test]
fn duplicate_binary_origin_pairs_are_rejected() {
    let mut fixture = Fixture::new();
    let (lhs_node, lhs_literal) = fixture.literal(2);
    let (rhs_node, rhs_literal) = fixture.literal(3);
    fixture.const_record(2, vec![lhs_literal]);
    fixture.const_record(3, vec![rhs_literal]);
    fixture.const_record(5, vec![lhs_literal, rhs_literal]);
    // Same origins but a conflicting recorded value: value never disambiguates.
    fixture.const_record(6, vec![lhs_literal, rhs_literal]);
    let funcdef = fixture.canonical_add_function(lhs_node, rhs_node);

    let error = fixture.project_error(funcdef);
    assert_rejects(&error, "ambiguous folded constants for lowering");
    assert_no_ir(&fixture);
}

#[test]
fn literal_folds_are_not_a_canonical_binary_fold() {
    let mut fixture = Fixture::new();
    let (lhs_node, lhs_literal) = fixture.literal(2);
    let (rhs_node, rhs_literal) = fixture.literal(3);
    fixture.const_record(2, vec![lhs_literal]);
    fixture.const_record(3, vec![rhs_literal]);
    let funcdef = fixture.canonical_add_function(lhs_node, rhs_node);

    // `[2]` and `[3]` do not prove `2 + 3`; selecting either by value would be
    // a guess, so the missing `[2, 3]` record is an ambiguous handoff.
    let error = fixture.project_error(funcdef);
    assert_rejects(&error, "ambiguous folded constants for lowering");
    assert_no_ir(&fixture);
}

#[test]
fn reversed_binary_origin_pair_is_rejected() {
    let mut fixture = Fixture::new();
    let (lhs_node, lhs_literal) = fixture.literal(2);
    let (rhs_node, rhs_literal) = fixture.literal(3);
    fixture.const_record(2, vec![lhs_literal]);
    fixture.const_record(3, vec![rhs_literal]);
    // The pair matches by membership but not by operand order.
    fixture.const_record(5, vec![rhs_literal, lhs_literal]);
    let funcdef = fixture.canonical_add_function(lhs_node, rhs_node);

    let error = fixture.project_error(funcdef);
    assert_rejects(&error, "ambiguous folded constants for lowering");
    assert_no_ir(&fixture);
}

#[test]
fn general_literal_map_ignores_binary_folds() {
    let mut fixture = Fixture::new();
    let (funcdef, initializer_literal) = fixture.local_function();
    let literal_fold = fixture.const_record(2, vec![initializer_literal]);
    let (_, unused_literal) = fixture.literal(3);
    let binary_fold = fixture.const_record(5, vec![initializer_literal, unused_literal]);

    let input = fixture
        .project(funcdef)
        .expect("literal origins stay unique when a binary fold coexists");
    assert_eq!(input.constant, literal_fold);
    assert_ne!(input.constant, binary_fold);

    // `int x = 2; return x;` lowers to materialize 2, store it, reload it,
    // and return it.
    let instructions = instructions(&input);
    assert_eq!(
        instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect::<Vec<_>>(),
        vec![
            IrOp::Constant,
            IrOp::LocalStore,
            IrOp::LocalLoad,
            IrOp::Return
        ]
    );
    assert_eq!(instructions[0].immediate, Some(literal_fold));
    assert_eq!(instructions[1].local, input.local);
    assert_eq!(instructions[2].local, input.local);
    assert_eq!(
        instructions[3].operands,
        vec![instructions[2].result.unwrap()]
    );
}

#[test]
fn duplicate_literal_origins_are_rejected_amid_binary_folds() {
    let mut fixture = Fixture::new();
    let (funcdef, initializer_literal) = fixture.local_function();
    fixture.const_record(2, vec![initializer_literal]);
    fixture.const_record(2, vec![initializer_literal]);
    let (_, other_literal) = fixture.literal(3);
    fixture.const_record(5, vec![initializer_literal, other_literal]);

    let error = fixture.project_error(funcdef);
    assert_rejects(&error, "ambiguous folded constants for literal");
    assert_no_ir(&fixture);
}
