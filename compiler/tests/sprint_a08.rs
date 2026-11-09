// ============================================================================
// sprint_a08.rs — independent ABI evidence for symbol spelling, internal
// linkage, duplicate labels, and fail-closed assembly rejection.
//
// Baseline: 5f3c753 (`t01-c01-c06/55`). The implementation under test is the
// existing host-only M1 emitter in `src/bin/candidate/target_m1.rs`, included
// directly as the other target tests do. Fixtures are synthetic committed IR
// modules (the A01–A08 boundary), so symbol bytes and linkage are selected
// independently of the C frontend and no external compiler processes the
// candidate output.
//
// Oracle: GNU AArch64 accepts C-like `[A-Za-z_][A-Za-z0-9_]*` labels but
// reserves register spellings in instruction operands. Because this emitter
// writes raw function names into labels and `bl` operands without escaping,
// every unsafe spelling must be rejected before assembly is returned.
//
// Known RED at this baseline: the reserved-register guard ends at x30/w30, so
// a callable function named `x31` or `w31` is accepted and produces `bl x31`
// or `bl w31`, which GNU as rejects. That isolated test documents the defect;
// the remaining assertions are expected GREEN.
// ============================================================================

#[path = "../src/bin/candidate/target_m1.rs"]
mod target_m1;

use cc_silicon_compiler::bus::{
    BlockRecord, CompilerBus, ConstRecord, ConstValue, FunctionRecord, InstructionRecord, IntRank,
    IntegerConst, IrOp, Linkage, ScopeKind, ScopeRecord, StorageDuration, SymbolKind, SymbolRecord,
    TypeKind, TypeRecord, ValueRecord,
};
use cc_silicon_compiler::ids::{
    BlockId, ConstId, NameId, NodeId, ScopeId, SymbolId, TypeId, ValueId,
};
use std::collections::BTreeSet;

/// Minimal canonical-int module builder for synthetic committed IR.
struct Module {
    bus: CompilerBus,
    int: TypeId,
    file: ScopeId,
    next_decl: u32,
}

#[derive(Clone)]
struct FunctionDecl {
    symbol: SymbolId,
    block: BlockId,
    params: Vec<SymbolId>,
}

impl Module {
    fn new() -> Self {
        let mut bus = CompilerBus::default();
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
        let file = bus
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
            file,
            next_decl: 0,
        }
    }

    fn next_node(&mut self) -> NodeId {
        let node = NodeId::from_index(self.next_decl);
        self.next_decl += 1;
        node
    }

    fn intern_bytes(&mut self, bytes: &[u8]) -> NameId {
        let limits = self.bus.limits();
        self.bus.intern.intern(bytes, &limits).unwrap()
    }

    fn intern(&mut self, name: &str) -> NameId {
        self.intern_bytes(name.as_bytes())
    }

    fn signature(&mut self, parameter_count: usize) -> TypeId {
        let limits = self.bus.limits();
        self.bus
            .arenas
            .types
            .alloc(
                TypeRecord {
                    kind: TypeKind::Function {
                        result: self.int,
                        params: vec![self.int; parameter_count],
                        prototype: true,
                        variadic: false,
                    },
                },
                &limits,
            )
            .unwrap()
    }

    fn declare(&mut self, name: &str, params: &[&str], linkage: Linkage) -> FunctionDecl {
        self.declare_raw(name.as_bytes(), params, linkage, linkage)
    }

    fn declare_raw(
        &mut self,
        name: &[u8],
        params: &[&str],
        symbol_linkage: Linkage,
        function_linkage: Linkage,
    ) -> FunctionDecl {
        let name = self.intern_bytes(name);
        let param_ids: Vec<NameId> = params.iter().map(|param| self.intern(param)).collect();
        let signature = self.signature(params.len());
        let limits = self.bus.limits();
        let scope_owner = self.next_node();
        let scope = self
            .bus
            .arenas
            .scopes
            .alloc(
                ScopeRecord {
                    kind: ScopeKind::Block,
                    parent: Some(self.file),
                    owner: Some(scope_owner),
                },
                &limits,
            )
            .unwrap();
        let mut parameters = Vec::new();
        for param_name in param_ids {
            let decl = self.next_node();
            let symbol = self
                .bus
                .arenas
                .symbols
                .alloc(
                    SymbolRecord {
                        name: param_name,
                        scope,
                        kind: SymbolKind::Object,
                        ty: Some(self.int),
                        linkage: Linkage::None,
                        storage: StorageDuration::Automatic,
                        decl,
                    },
                    &limits,
                )
                .unwrap();
            parameters.push(symbol);
        }
        let decl_node = self.next_node();
        let symbol = self
            .bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name,
                    scope: self.file,
                    kind: SymbolKind::Function,
                    ty: Some(signature),
                    linkage: symbol_linkage,
                    storage: StorageDuration::Static,
                    decl: decl_node,
                },
                &limits,
            )
            .unwrap();
        let block = BlockId::from_index(self.bus.arenas.blocks.allocated());
        let id = self
            .bus
            .arenas
            .functions
            .alloc(
                FunctionRecord {
                    symbol,
                    signature,
                    entry: block,
                    linkage: function_linkage,
                    parameters: parameters.clone(),
                },
                &limits,
            )
            .unwrap();
        self.bus
            .arenas
            .blocks
            .alloc(
                BlockRecord {
                    function: id,
                    ordinal: 0,
                },
                &limits,
            )
            .unwrap();
        FunctionDecl {
            symbol,
            block,
            params: parameters,
        }
    }

    fn body(&mut self, decl: &FunctionDecl) -> Body<'_> {
        Body {
            module: self,
            decl: decl.clone(),
        }
    }
}

struct Body<'a> {
    module: &'a mut Module,
    decl: FunctionDecl,
}

impl Body<'_> {
    fn value(&mut self) -> ValueId {
        let limits = self.module.bus.limits();
        self.module
            .bus
            .arenas
            .values
            .alloc(
                ValueRecord {
                    ty: self.module.int,
                },
                &limits,
            )
            .unwrap()
    }

    fn instruction(
        &mut self,
        op: IrOp,
        operands: Vec<ValueId>,
        immediate: Option<ConstId>,
        result: Option<ValueId>,
        local: Option<SymbolId>,
        callee: Option<SymbolId>,
    ) {
        let limits = self.module.bus.limits();
        self.module
            .bus
            .arenas
            .instructions
            .alloc(
                InstructionRecord {
                    op,
                    block: self.decl.block,
                    operands,
                    immediate,
                    result,
                    local,
                    successors: vec![],
                    callee,
                },
                &limits,
            )
            .unwrap();
    }

    fn constant(&mut self, value: i32) -> ValueId {
        let limits = self.module.bus.limits();
        let constant = self
            .module
            .bus
            .arenas
            .consts
            .alloc(
                ConstRecord {
                    ty: self.module.int,
                    value: ConstValue::Integer(IntegerConst {
                        magnitude: value.unsigned_abs().to_be_bytes().to_vec(),
                        negative: value < 0,
                    }),
                    origins: vec![],
                },
                &limits,
            )
            .unwrap();
        let result = self.value();
        self.instruction(
            IrOp::Constant,
            vec![],
            Some(constant),
            Some(result),
            None,
            None,
        );
        result
    }

    fn load_param(&mut self, index: usize) -> ValueId {
        let symbol = self.decl.params[index];
        let result = self.value();
        self.instruction(
            IrOp::LocalLoad,
            vec![],
            None,
            Some(result),
            Some(symbol),
            None,
        );
        result
    }

    fn binary(&mut self, op: IrOp, lhs: ValueId, rhs: ValueId) -> ValueId {
        let result = self.value();
        self.instruction(op, vec![lhs, rhs], None, Some(result), None, None);
        result
    }

    fn call(&mut self, callee: SymbolId, arguments: Vec<ValueId>) -> ValueId {
        let result = self.value();
        self.instruction(
            IrOp::Call,
            arguments,
            None,
            Some(result),
            None,
            Some(callee),
        );
        result
    }

    fn ret(mut self, value: ValueId) {
        self.instruction(IrOp::Return, vec![value], None, None, None, None);
    }
}

fn emit(module: &Module) -> Result<String, String> {
    target_m1::emit_m1_assembly(&module.bus)
        .map(|bytes| String::from_utf8(bytes).expect("assembly is UTF-8"))
}

fn globl_symbols(assembly: &str) -> Vec<&str> {
    assembly
        .lines()
        .filter_map(|line| line.strip_prefix(".globl "))
        .collect()
}

fn function_labels(assembly: &str) -> Vec<&str> {
    assembly
        .lines()
        .filter_map(|line| {
            let label = line.strip_suffix(':')?;
            if label.is_empty() || label.starts_with('.') || label.starts_with('#') {
                None
            } else {
                Some(label)
            }
        })
        .collect()
}

fn assert_unique_labels(assembly: &str) {
    let labels = function_labels(assembly);
    let unique: BTreeSet<&str> = labels.iter().copied().collect();
    assert_eq!(
        unique.len(),
        labels.len(),
        "duplicate function label emitted:\n{assembly}"
    );
}

fn duplicate_error(name: &str, first: Linkage, second: Linkage) -> String {
    let mut module = Module::new();
    let _main = module.declare("main", &[], Linkage::External);
    let _first = module.declare(name, &[], first);
    let _second = module.declare(name, &[], second);
    emit(&module).unwrap_err()
}

#[test]
fn legal_symbols_are_emitted_verbatim_without_escaping() {
    let names = ["_helper", "Helper", "helper2", "add", "ret", "_"];
    let mut module = Module::new();
    let mut declarations = Vec::new();
    for (index, name) in names.iter().enumerate() {
        let declaration = module.declare(name, &[], Linkage::External);
        {
            let mut body = module.body(&declaration);
            let value = body.constant(index as i32 + 1);
            body.ret(value);
        }
        declarations.push(declaration);
    }
    let helper = declarations[0].clone();
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let result = body.call(helper.symbol, vec![]);
        body.ret(result);
    }

    let assembly = emit(&module).unwrap();
    let mut expected = names.to_vec();
    expected.push("main");
    assert_eq!(function_labels(&assembly), expected, "{assembly}");
    assert_unique_labels(&assembly);

    for name in &names {
        assert!(
            assembly.contains(&format!(".globl {name}\n")),
            "missing global for {name}:\n{assembly}"
        );
        assert!(
            assembly.contains(&format!(".type {name}, %function\n")),
            "missing type for {name}:\n{assembly}"
        );
        assert!(
            assembly.contains(&format!(".size {name}, .-{name}\n")),
            "missing size for {name}:\n{assembly}"
        );
    }
    assert!(assembly.contains("\n    bl _helper\n"), "{assembly}");
    assert!(!assembly.contains("__helper"), "{assembly}");
    assert!(!assembly.contains("_main"), "{assembly}");
}

#[test]
fn internal_linkage_omits_globl_but_keeps_label_call_and_size() {
    let mut module = Module::new();
    let helper = module.declare("helper", &["x"], Linkage::Internal);
    {
        let mut body = module.body(&helper);
        let argument = body.load_param(0);
        let one = body.constant(1);
        let result = body.binary(IrOp::Add, argument, one);
        body.ret(result);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let four = body.constant(4);
        let result = body.call(helper.symbol, vec![four]);
        body.ret(result);
    }

    let assembly = emit(&module).unwrap();
    assert_eq!(globl_symbols(&assembly), vec!["main"], "{assembly}");
    assert!(!assembly.contains(".globl helper\n"), "{assembly}");
    assert!(assembly.contains(".type helper, %function\n"), "{assembly}");
    assert!(assembly.contains("\nhelper:\n"), "{assembly}");
    assert!(assembly.contains("    bl helper\n"), "{assembly}");
    assert!(assembly.contains(".size helper, .-helper\n"), "{assembly}");
    assert_unique_labels(&assembly);
}

#[test]
fn internal_main_and_function_without_linkage_are_rejected() {
    let mut module = Module::new();
    let main = module.declare("main", &[], Linkage::Internal);
    {
        let mut body = module.body(&main);
        let zero = body.constant(0);
        body.ret(zero);
    }
    let error = emit(&module).unwrap_err();
    assert!(error.contains("main must be int(void)"), "{error}");

    let mut module = Module::new();
    let dead = module.declare("dead", &[], Linkage::None);
    {
        let mut body = module.body(&dead);
        let one = body.constant(1);
        body.ret(one);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let zero = body.constant(0);
        body.ret(zero);
    }
    let error = emit(&module).unwrap_err();
    assert!(error.contains("has no linkage"), "{error}");
}

#[test]
fn symbol_and_function_linkage_must_match() {
    let mut module = Module::new();
    let _mismatch = module.declare_raw(b"mismatch", &[], Linkage::External, Linkage::Internal);
    let _main = module.declare("main", &[], Linkage::External);
    let error = emit(&module).unwrap_err();
    assert!(error.contains("linkage mismatch"), "{error}");
}

#[test]
fn duplicate_function_labels_are_rejected() {
    for (first, second, case) in [
        (Linkage::External, Linkage::External, "external/external"),
        (Linkage::Internal, Linkage::Internal, "internal/internal"),
        (Linkage::Internal, Linkage::External, "internal/external"),
        (Linkage::External, Linkage::Internal, "external/internal"),
    ] {
        let error = duplicate_error("dup", first, second);
        assert!(
            error.contains("duplicate function symbol dup"),
            "{case}: {error}"
        );
    }

    let error = duplicate_error("main", Linkage::External, Linkage::External);
    assert!(error.contains("duplicate function symbol main"), "{error}");
}

#[test]
fn case_distinct_symbols_are_distinct_labels() {
    let mut module = Module::new();
    for name in ["dup", "Dup"] {
        let declaration = module.declare(name, &[], Linkage::External);
        let mut body = module.body(&declaration);
        let one = body.constant(1);
        body.ret(one);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let zero = body.constant(0);
        body.ret(zero);
    }

    let assembly = emit(&module).unwrap();
    assert_eq!(
        function_labels(&assembly),
        vec!["dup", "Dup", "main"],
        "{assembly}"
    );
    assert_eq!(globl_symbols(&assembly), vec!["dup", "Dup", "main"]);
    assert_unique_labels(&assembly);
}

const UNSAFE_NAMES: &[&str] = &[
    "x0",
    "x7",
    "x30",
    "w0",
    "w30",
    "sp",
    "wsp",
    "xzr",
    "wzr",
    "lr",
    "fp",
    "ip0",
    "ip1",
    "pc",
    "pstate",
    "nzcv",
    "b0",
    "b31",
    "h0",
    "h31",
    "s0",
    "s31",
    "d0",
    "d31",
    "q0",
    "q31",
    "v0",
    "v31",
    "z0",
    "z31",
    "eq",
    "ne",
    "cs",
    "hs",
    "cc",
    "lo",
    "mi",
    "pl",
    "vs",
    "vc",
    "hi",
    "ls",
    "ge",
    "lt",
    "gt",
    "le",
    "al",
    "nv",
    "9lives",
    "",
    "has space",
    "a-b",
    "a.b",
    "$name",
    "café",
    "a/b",
];

#[test]
fn assembler_unsafe_spellings_are_rejected_before_emission() {
    for name in UNSAFE_NAMES {
        let mut module = Module::new();
        let _main = module.declare("main", &[], Linkage::External);
        let _unsafe = module.declare(name, &[], Linkage::External);
        let error = emit(&module).unwrap_err();
        assert!(error.contains("unsafe name"), "{name:?}: {error}");
    }

    let mut module = Module::new();
    let _main = module.declare("main", &[], Linkage::External);
    let _invalid = module.declare_raw(b"caf\xff", &[], Linkage::External, Linkage::External);
    let error = emit(&module).unwrap_err();
    assert!(error.contains("unsafe name (invalid UTF-8)"), "{error}");
}

#[test]
fn callable_x31_and_w31_spellings_are_rejected() {
    // GNU as rejects `bl x31`/`bl w31` even when a matching local label exists.
    // The emitter currently accepts both spellings because its guard stops at
    // x30/w30; this is the isolated RED assertion recorded in the header.
    for name in ["x31", "w31"] {
        let mut module = Module::new();
        let callee = module.declare(name, &[], Linkage::External);
        {
            let mut body = module.body(&callee);
            let one = body.constant(1);
            body.ret(one);
        }
        let main = module.declare("main", &[], Linkage::External);
        {
            let mut body = module.body(&main);
            let result = body.call(callee.symbol, vec![]);
            body.ret(result);
        }

        match emit(&module) {
            Ok(assembly) => panic!(
                "callable symbol {name:?} produces assembly rejected by GNU as \
                 (`bl {name}`):\n{assembly}"
            ),
            Err(error) => assert!(error.contains("unsafe name"), "{name:?}: {error}"),
        }
    }
}
