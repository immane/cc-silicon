// ============================================================================
// sprint_a04.rs — independent ABI evidence for SP alignment and frame bounds.
//
// The fixtures are synthetic compiler-bus modules for the host-only M1
// emitter. Expected frame sizes below are hand calculations from the slot
// layout rules, not calls to the emitter's private `frame_size` helper:
//
// * a slot is 4 bytes;
// * a non-leaf function also reserves 8 bytes for LR;
// * the result is rounded up to a 16-byte boundary and capped at 4080,
//   the largest 16-byte-aligned `sub sp, sp, #imm` value.
//
// Therefore the accepted boundary is 4080 bytes and the first invalid
// 16-byte-aligned frame is 4096 bytes.
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

/// Builds a canonical int-only module with one file scope.
struct Module {
    bus: CompilerBus,
    int: TypeId,
    file: ScopeId,
    next_decl: u32,
}

#[derive(Clone)]
struct FnDecl {
    symbol: SymbolId,
    block: BlockId,
    scope: ScopeId,
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

    fn intern(&mut self, name: &str) -> NameId {
        let limits = self.bus.limits();
        self.bus.intern.intern(name.as_bytes(), &limits).unwrap()
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

    fn declare(&mut self, name: &str, params: &[&str], linkage: Linkage) -> FnDecl {
        let name = self.intern(name);
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
        for param_name in &param_ids {
            let decl = self.next_node();
            let symbol = self
                .bus
                .arenas
                .symbols
                .alloc(
                    SymbolRecord {
                        name: *param_name,
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
                    linkage,
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
                    linkage,
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
        FnDecl {
            symbol,
            block,
            scope,
            params: parameters,
        }
    }

    fn body(&mut self, decl: &FnDecl) -> Body<'_> {
        Body {
            module: self,
            decl: decl.clone(),
        }
    }
}

struct Body<'a> {
    module: &'a mut Module,
    decl: FnDecl,
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
        let magnitude = value.unsigned_abs().to_be_bytes().to_vec();
        let constant = self
            .module
            .bus
            .arenas
            .consts
            .alloc(
                ConstRecord {
                    ty: self.module.int,
                    value: ConstValue::Integer(IntegerConst {
                        magnitude,
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

    fn store_symbol(&mut self, symbol: SymbolId, value: ValueId) {
        self.instruction(
            IrOp::LocalStore,
            vec![value],
            None,
            None,
            Some(symbol),
            None,
        );
    }

    fn local(&mut self, name: &str) -> SymbolId {
        let name = self.module.intern(name);
        let limits = self.module.bus.limits();
        let decl = self.module.next_node();
        self.module
            .bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name,
                    scope: self.decl.scope,
                    kind: SymbolKind::Object,
                    ty: Some(self.module.int),
                    linkage: Linkage::None,
                    storage: StorageDuration::Automatic,
                    decl,
                },
                &limits,
            )
            .unwrap()
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

    fn ret(self, value: ValueId) {
        let mut this = self;
        this.instruction(IrOp::Return, vec![value], None, None, None, None);
    }
}

fn emit(module: &Module) -> Result<String, String> {
    target_m1::emit_m1_assembly(&module.bus)
        .map(|bytes| String::from_utf8(bytes).expect("assembly is UTF-8"))
}

fn function_text<'a>(assembly: &'a str, name: &str) -> &'a str {
    let label = format!("\n{name}:\n");
    let start = assembly
        .find(&label)
        .map(|index| index + 1)
        .unwrap_or_else(|| panic!("missing label {name}"));
    let rest = &assembly[start..];
    let end = rest
        .find(&format!(".size {name}, .-{name}"))
        .unwrap_or_else(|| panic!("missing size for {name}"));
    &rest[..end]
}

/// Checks SP bookkeeping in emitted assembly and returns every call site as
/// `(function label, SP displacement from function entry)`.
///
/// At a call boundary SP must be 16-byte aligned. The only adjustments the
/// emitter is allowed to make are one prologue subtraction and one epilogue
/// addition, each a multiple of 16 no larger than 4080.
fn call_offsets(assembly: &str) -> Vec<(String, i64)> {
    let mut sp = 0i64;
    let mut function = String::new();
    let mut calls = Vec::new();
    for raw in assembly.lines() {
        let line = raw.trim();
        if let Some(label) = line.strip_suffix(':') {
            if !label.is_empty() && !label.starts_with('.') {
                assert_eq!(sp, 0, "SP is not restored before entering {label}");
                function = label.to_string();
                continue;
            }
        }
        if let Some(immediate) = line.strip_prefix("sub sp, sp, #") {
            let bytes: i64 = immediate.parse().expect("frame immediate");
            assert!(bytes > 0, "empty prologue frame in {function}");
            assert_eq!(bytes % 16, 0, "unaligned frame in {function}: {bytes}");
            assert!(
                bytes <= 4080,
                "frame exceeds the largest aligned immediate in {function}: {bytes}"
            );
            sp -= bytes;
        } else if let Some(immediate) = line.strip_prefix("add sp, sp, #") {
            let bytes: i64 = immediate.parse().expect("frame immediate");
            assert_eq!(bytes % 16, 0, "unaligned epilogue in {function}: {bytes}");
            sp += bytes;
            assert_eq!(sp, 0, "epilogue does not restore SP in {function}");
        } else if line.starts_with("bl ") {
            assert_eq!(
                sp % 16,
                0,
                "SP is not 16-byte aligned at a call in {function}: {sp}"
            );
            calls.push((function.clone(), sp));
        } else if line == "ret" {
            assert_eq!(sp, 0, "return with a non-entry SP in {function}");
        }
    }
    calls
}

/// Eight argument registers, eight parameter loads and seven additions:
/// `8 + 8 + 7 = 23` slots, so the leaf frame is `ceil16(23 * 4) = 96`.
/// `main` has nine result slots and is non-leaf: `ceil16(9 * 4 + 8) = 48`.
fn alignment_module() -> Module {
    let mut module = Module::new();
    let weighted = module.declare(
        "weighted",
        &["a", "b", "c", "d", "e", "f", "g", "h"],
        Linkage::External,
    );
    {
        let mut body = module.body(&weighted);
        let mut value = body.load_param(0);
        for (index, op) in [
            IrOp::Sub,
            IrOp::Add,
            IrOp::Sub,
            IrOp::Add,
            IrOp::Sub,
            IrOp::Add,
            IrOp::Sub,
        ]
        .into_iter()
        .enumerate()
        {
            let next = body.load_param(index + 1);
            value = body.binary(op, value, next);
        }
        body.ret(value);
    }

    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let mut arguments = Vec::new();
        for value in 1..=8 {
            arguments.push(body.constant(value));
        }
        let result = body.call(weighted.symbol, arguments);
        body.ret(result);
    }
    module
}

/// `locals` locals, one parameter, one constant value and one call result:
/// `locals + 3` slots. A non-leaf frame is `ceil16(4 * slots + 8)`.
///
/// * 1015 locals -> 1018 slots -> 4080 bytes (accepted boundary);
/// * 1016 locals -> 1019 slots -> 4096 bytes (first invalid aligned size).
fn nonleaf_frame_module(locals: usize) -> Module {
    let mut module = Module::new();
    let helper = module.declare("zero", &[], Linkage::External);
    {
        let mut body = module.body(&helper);
        let zero = body.constant(0);
        body.ret(zero);
    }

    let big = module.declare("big", &["seed"], Linkage::External);
    {
        let mut body = module.body(&big);
        let seed = body.constant(1);
        for index in 0..locals {
            let local = body.local(&format!("b{index}"));
            body.store_symbol(local, seed);
        }
        let result = body.call(helper.symbol, vec![]);
        body.ret(result);
    }

    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let argument = body.constant(0);
        let result = body.call(big.symbol, vec![argument]);
        body.ret(result);
    }
    module
}

/// `locals` locals and one constant value: `locals + 1` slots. A leaf frame
/// is `ceil16(4 * slots)`.
///
/// * 1019 locals -> 1020 slots -> 4080 bytes (accepted boundary);
/// * 1020 locals -> 1021 slots -> 4096 bytes (first invalid aligned size).
fn leaf_frame_module(locals: usize) -> Module {
    let mut module = Module::new();
    let big = module.declare("lbig", &[], Linkage::External);
    {
        let mut body = module.body(&big);
        let seed = body.constant(1);
        for index in 0..locals {
            let local = body.local(&format!("l{index}"));
            body.store_symbol(local, seed);
        }
        body.ret(seed);
    }

    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let result = body.call(big.symbol, vec![]);
        body.ret(result);
    }
    module
}

const FRAME_OVERFLOW: &str = "target emit: frame exceeds the AArch64 immediate range (4096 bytes)";

#[test]
fn calls_are_made_with_sixteen_byte_aligned_sp() {
    let module = alignment_module();
    let assembly = emit(&module).unwrap();

    let calls = call_offsets(&assembly);
    assert_eq!(calls, vec![("main".to_string(), -48)], "{assembly}");

    let weighted = function_text(&assembly, "weighted");
    assert!(weighted.contains("    sub sp, sp, #96\n"), "{weighted}");
    assert!(weighted.contains("    add sp, sp, #96\n"), "{weighted}");
    assert!(!weighted.contains("bl "), "{weighted}");

    let main = function_text(&assembly, "main");
    assert!(main.contains("    sub sp, sp, #48\n"), "{main}");
    assert!(main.contains("    bl weighted\n"), "{main}");
    assert!(main.contains("    add sp, sp, #48\n"), "{main}");
}

#[test]
fn nonleaf_frame_4080_is_accepted_and_4096_is_rejected() {
    let accepted = nonleaf_frame_module(1015);
    let assembly = emit(&accepted).unwrap();

    let big = function_text(&assembly, "big");
    assert!(big.contains("    sub sp, sp, #4080\n"), "{big}");
    assert!(!big.contains("#4096"), "{big}");
    assert!(
        call_offsets(&assembly)
            .iter()
            .any(|(function, offset)| function == "big" && *offset == -4080),
        "{assembly}"
    );

    let rejected = nonleaf_frame_module(1016);
    assert_eq!(emit(&rejected).unwrap_err(), FRAME_OVERFLOW);
}

#[test]
fn leaf_frame_4080_is_accepted_and_4096_is_rejected() {
    let accepted = leaf_frame_module(1019);
    let assembly = emit(&accepted).unwrap();

    let big = function_text(&assembly, "lbig");
    assert!(big.contains("    sub sp, sp, #4080\n"), "{big}");
    assert!(!big.contains("#4096"), "{big}");
    assert!(!big.contains("bl "), "{big}");

    let rejected = leaf_frame_module(1020);
    assert_eq!(emit(&rejected).unwrap_err(), FRAME_OVERFLOW);
}
