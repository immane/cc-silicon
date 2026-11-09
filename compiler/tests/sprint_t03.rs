// ============================================================================
// sprint_t03.rs — `/55` sibling parameter spellings and binding isolation.
//
// T03 focus: two sibling functions may spell a parameter the same way, but
// each body must bind its own automatic object. The synthetic semantic fixture
// builds both functions with the single interned name `x` and distinct block
// scopes; a third body that does not declare `x` must fail rather than borrow a
// sibling binding. The candidate cases re-check the same boundary end to end.
//
// Expected values are hand-derived from signed `int` arithmetic:
//   first(8) - 5 = 3, second(4) * 2 = 8, 3 + 8 = 11 (0x0b)
//   one(3) + 2 = 5, two(9, 4) = 9 - 4 = 5, 5 + 5 = 10 (0x0a)
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use cc_silicon_compiler::bus::{
    CompilerBus, EffectMask, IntRank, Linkage, NodeKind, NodeRecord, ScopeKind, ScopeRecord,
    SemRecord, StorageDuration, SymbolKind, SymbolRecord, TypeKind, TypeRecord, ValueCategory,
};
use cc_silicon_compiler::chips::{project_se_identifier_input, SeIdentifierChip};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{
    NameId, NodeId, RecordRef, ScopeId, SymbolId, TaskId, TokenId, TypeId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::SE_ID_CHIP;
use cc_silicon_compiler::records::G1DraftBody;
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, Proposal, TaskDraft, TaskKind, TaskState};

// ---------------------------------------------------------------------------
// Synthetic semantic fixture: same spelling, independent sibling bindings.
// ---------------------------------------------------------------------------

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn alloc_int(bus: &mut CompilerBus) -> TypeId {
    let limits = bus.limits();
    bus.arenas
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
        .unwrap()
}

fn alloc_node(bus: &mut CompilerBus, kind: NodeKind, name: Option<NameId>) -> NodeId {
    let limits = bus.limits();
    bus.arenas
        .nodes
        .alloc(
            NodeRecord {
                kind,
                parent: None,
                children: Vec::new(),
                first_token: TokenId::from_index(0),
                last_token: TokenId::from_index(0),
                name,
                literal: None,
            },
            &limits,
        )
        .unwrap()
}

fn attach(bus: &mut CompilerBus, parent: NodeId, children: Vec<NodeId>) {
    for child in &children {
        bus.arenas.nodes.get_mut(*child).unwrap().parent = Some(parent);
    }
    bus.arenas.nodes.get_mut(parent).unwrap().children = children;
}

fn alloc_scope(
    bus: &mut CompilerBus,
    kind: ScopeKind,
    parent: Option<ScopeId>,
    owner: Option<NodeId>,
) -> ScopeId {
    let limits = bus.limits();
    bus.arenas
        .scopes
        .alloc(
            ScopeRecord {
                kind,
                parent,
                owner,
            },
            &limits,
        )
        .unwrap()
}

fn alloc_parameter_symbol(
    bus: &mut CompilerBus,
    name: NameId,
    scope: ScopeId,
    ty: TypeId,
    decl: NodeId,
) -> SymbolId {
    let limits = bus.limits();
    bus.arenas
        .symbols
        .alloc(
            SymbolRecord {
                name,
                scope,
                kind: SymbolKind::Object,
                ty: Some(ty),
                linkage: Linkage::None,
                storage: StorageDuration::Automatic,
                decl,
            },
            &limits,
        )
        .unwrap()
}

struct Sibling {
    compound: NodeId,
    parameter: NodeId,
    identifier: NodeId,
    block: ScopeId,
    parameter_symbol: SymbolId,
}

/// `int <function_name>(int <parameter_name>) { return <parameter_name>; }`
fn build_sibling(
    bus: &mut CompilerBus,
    int: TypeId,
    file: ScopeId,
    function_name: &[u8],
    parameter_name: NameId,
) -> Sibling {
    let function_name = bus.intern_name(function_name).unwrap();
    let funcdef = alloc_node(bus, NodeKind::FunctionDefinition, None);
    let specifiers = alloc_node(bus, NodeKind::Specifiers, None);
    let declarator = alloc_node(bus, NodeKind::Declarator, Some(function_name));
    let parameter_list = alloc_node(bus, NodeKind::ParameterList, None);
    let parameter = alloc_node(bus, NodeKind::Parameter, Some(parameter_name));
    let compound = alloc_node(bus, NodeKind::Compound, None);
    let ret = alloc_node(bus, NodeKind::Return, None);
    let identifier = alloc_node(bus, NodeKind::Identifier, Some(parameter_name));

    attach(bus, funcdef, vec![specifiers, declarator, compound]);
    attach(bus, declarator, vec![parameter_list]);
    attach(bus, parameter_list, vec![parameter]);
    attach(bus, compound, vec![ret]);
    attach(bus, ret, vec![identifier]);

    let block = alloc_scope(bus, ScopeKind::Block, Some(file), Some(compound));
    let parameter_symbol = alloc_parameter_symbol(bus, parameter_name, block, int, parameter);
    Sibling {
        compound,
        parameter,
        identifier,
        block,
        parameter_symbol,
    }
}

/// `int missing(void) { return x; }` with no object named `x`.
fn build_undeclared_reference(bus: &mut CompilerBus, file: ScopeId, name: NameId) -> NodeId {
    let function_name = bus.intern_name(b"missing").unwrap();
    let funcdef = alloc_node(bus, NodeKind::FunctionDefinition, None);
    let specifiers = alloc_node(bus, NodeKind::Specifiers, None);
    let declarator = alloc_node(bus, NodeKind::Declarator, Some(function_name));
    let compound = alloc_node(bus, NodeKind::Compound, None);
    let ret = alloc_node(bus, NodeKind::Return, None);
    let identifier = alloc_node(bus, NodeKind::Identifier, Some(name));

    attach(bus, funcdef, vec![specifiers, declarator, compound]);
    attach(bus, compound, vec![ret]);
    attach(bus, ret, vec![identifier]);
    alloc_scope(bus, ScopeKind::Block, Some(file), Some(compound));
    identifier
}

fn running_identifier_task(bus: &mut CompilerBus, node: NodeId) -> TaskId {
    let task = bus
        .bootstrap_task(TaskDraft {
            kind: TaskKind::SEMANTIC_IDENTIFIER_EXPR,
            owner: SE_ID_CHIP,
            parent: None,
            payload: Payload::from_refs(vec![RecordRef::Node(node)]),
            continuation: None,
        })
        .unwrap();
    bus.arenas.tasks.get_mut(task).unwrap().state = TaskState::Running;
    task
}

fn proposed_sem(proposals: &[Proposal]) -> SemRecord {
    assert_eq!(
        proposals.len(),
        2,
        "identifier compute must append then complete"
    );
    match &proposals[0] {
        Proposal::AppendRecords { batch, .. } => match &batch.bodies[0] {
            G1DraftBody::Sem(record) => record.clone(),
            other => panic!("expected a Sem body, got {other:?}"),
        },
        other => panic!("expected AppendRecords, got {other:?}"),
    }
}

#[test]
fn sibling_parameters_resolve_to_their_own_symbols_without_leakage() {
    let mut bus = new_bus();
    let int = alloc_int(&mut bus);
    let file = alloc_scope(&mut bus, ScopeKind::File, None, None);
    let x_name = bus.intern_name(b"x").unwrap();

    let left = build_sibling(&mut bus, int, file, b"left", x_name);
    let right = build_sibling(&mut bus, int, file, b"right", x_name);
    let undeclared = build_undeclared_reference(&mut bus, file, x_name);

    // One spelling, two automatic objects in two function block scopes.
    assert_eq!(
        bus.arenas.symbols.get(left.parameter_symbol).unwrap().name,
        x_name
    );
    assert_eq!(
        bus.arenas.symbols.get(right.parameter_symbol).unwrap().name,
        x_name
    );
    assert_ne!(left.parameter_symbol, right.parameter_symbol);
    assert_ne!(left.block, right.block);
    for (sibling, expected_owner, expected_decl) in [
        (&left, left.compound, left.parameter),
        (&right, right.compound, right.parameter),
    ] {
        let scope = bus.arenas.scopes.get(sibling.block).unwrap();
        assert_eq!(scope.kind, ScopeKind::Block);
        assert_eq!(scope.parent, Some(file));
        assert_eq!(scope.owner, Some(expected_owner));
        let symbol = bus.arenas.symbols.get(sibling.parameter_symbol).unwrap();
        assert_eq!(symbol.name, x_name);
        assert_eq!(symbol.kind, SymbolKind::Object);
        assert_eq!(symbol.storage, StorageDuration::Automatic);
        assert_eq!(symbol.linkage, Linkage::None);
        assert_eq!(symbol.ty, Some(int));
        assert_eq!(symbol.decl, expected_decl);
        assert_eq!(symbol.scope, sibling.block);
    }

    // Each identifier resolves to its own sibling binding, never the other's.
    for (sibling, own, other) in [
        (&left, left.parameter_symbol, right.parameter_symbol),
        (&right, right.parameter_symbol, left.parameter_symbol),
    ] {
        let task = running_identifier_task(&mut bus, sibling.identifier);
        let input = project_se_identifier_input(&bus, task).unwrap();
        assert_eq!(input.symbol, own);
        assert_ne!(input.symbol, other);
        let sem = proposed_sem(&SeIdentifierChip.compute(&input));
        assert_eq!(sem.node, sibling.identifier);
        assert_eq!(sem.ty, int);
        assert_eq!(sem.category, ValueCategory::NonLvalue);
        assert_eq!(sem.effects, EffectMask(0));
    }

    // A body without its own `x` must not borrow either sibling parameter.
    let task = running_identifier_task(&mut bus, undeclared);
    let error = project_se_identifier_input(&bus, task).unwrap_err();
    assert_eq!(error.code.group, DiagGroup::Semantic);
    assert!(
        error.message.contains("undeclared name"),
        "{}",
        error.message
    );
}

// ---------------------------------------------------------------------------
// Candidate end-to-end cases.
// ---------------------------------------------------------------------------

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(label: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock moves forward")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "sprint-t03-{}-{}-{}",
        std::process::id(),
        label,
        nanos
    ));
    fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run_candidate(source: &str, asm: &Path) -> Output {
    let dir = asm.parent().expect("assembly parent");
    fs::create_dir_all(dir).expect("source dir");
    let source_path = dir.join("main.c");
    fs::write(&source_path, source.as_bytes()).expect("source file");
    Command::new(binary())
        .arg("--emit-asm")
        .arg(asm)
        .arg("--interpret-ir")
        .arg(&source_path)
        .output()
        .expect("candidate runs")
}

fn assert_modeled(output: &Output, expected: &str, context: &str) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{context}\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(expected),
        "{context}: expected `{expected}`\nstdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_label(assembly: &str, name: &str, context: &str) {
    let label = format!("{name}:");
    assert!(
        assembly.lines().any(|line| line.trim() == label),
        "{context}: missing function label `{label}` in:\n{assembly}"
    );
}

fn assert_direct_call(assembly: &str, callee: &str, context: &str) {
    assert!(
        assembly.lines().any(|line| {
            let mut tokens = line.split_whitespace();
            tokens.next() == Some("bl") && tokens.next() == Some(callee)
        }),
        "{context}: missing direct call `bl {callee}` in:\n{assembly}"
    );
}

#[test]
fn candidate_models_sibling_parameter_reuse_independently() {
    // `first(8) - 5 = 3`; `second(4) * 2 = 8`; returned sum is 11.
    let dir = workdir("two-unary-siblings");
    let asm = dir.join("main.s");
    let output = run_candidate(
        "int first(int x) { return x - 5; }\n\
         int second(int x) { return x * 2; }\n\
         int main(void) { return first(8) + second(4); }\n",
        &asm,
    );
    assert_modeled(
        &output,
        "interpret: value=0b negative=false",
        "two unary siblings",
    );
    let assembly = fs::read_to_string(&asm).expect("assembly file");
    for name in ["first", "second", "main"] {
        assert_label(&assembly, name, "two unary siblings");
    }
    assert_direct_call(&assembly, "first", "two unary siblings");
    assert_direct_call(&assembly, "second", "two unary siblings");

    // `one(3) + 2 = 5`; `two(9, 4) = 9 - 4 = 5`; returned sum is 10.
    let dir = workdir("mixed-arity-siblings");
    let asm = dir.join("main.s");
    let output = run_candidate(
        "int one(int x) { return x + 2; }\n\
         int two(int x, int y) { return x - y; }\n\
         int main(void) { return one(3) + two(9, 4); }\n",
        &asm,
    );
    assert_modeled(
        &output,
        "interpret: value=0a negative=false",
        "mixed arity siblings",
    );
    let assembly = fs::read_to_string(&asm).expect("assembly file");
    for name in ["one", "two", "main"] {
        assert_label(&assembly, name, "mixed arity siblings");
    }
    assert_direct_call(&assembly, "one", "mixed arity siblings");
    assert_direct_call(&assembly, "two", "mixed arity siblings");
}

#[test]
fn candidate_rejects_a_sibling_parameter_name_used_outside_its_function() {
    let dir = workdir("undeclared-sibling");
    let asm = dir.join("main.s");
    let output = run_candidate(
        "int has(int x) { return x + 1; }\n\
         int missing(void) { return x; }\n\
         int main(void) { return missing(); }\n",
        &asm,
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "source unexpectedly accepted\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("candidate diagnostic"),
        "rejection must use the candidate stderr channel: {stderr}"
    );
    assert!(
        stderr.contains("undeclared name"),
        "the sibling parameter must not resolve in another body: {stderr}"
    );
    assert!(
        !asm.exists(),
        "rejected source left an assembly artifact at {}",
        asm.display()
    );
}
