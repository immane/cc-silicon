// ============================================================================
// sprint_a02.rs — A02 independent ABI evidence.
//
// Obligation: function entry saves all incoming parameters before any
// scratch-register use. The fixture defines an eight-parameter function whose
// first body instruction writes w0 (a zero constant), then calls a helper
// (which clobbers the argument registers) and only afterwards reads parameters
// a and h. Parameters b..g are intentionally unused so that the prologue must
// save every incoming register, not only the referenced ones.
//
// Independent expected values:
//   - hand-derived prologue: 80-byte frame, w0..w7 stored at slots
//     0,4,...,28 before the first body scratch write;
//   - hand-computed result: a(1)*10 - h(8) + helper(0), where the helper
//     returns 1, so the process result is 3.
// The native check is limited to Linux hosts whose gcc targets AArch64.
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

struct Module {
    bus: CompilerBus,
    int: TypeId,
    file: ScopeId,
    next_decl: u32,
}

#[derive(Clone)]
#[allow(dead_code)]
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
        let param_names: Vec<String> = params.iter().map(|param| (*param).to_string()).collect();
        let param_ids: Vec<NameId> = param_names.iter().map(|param| self.intern(param)).collect();
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

fn function_lines<'a>(assembly: &'a str, name: &str) -> Vec<&'a str> {
    let label = format!("\n{name}:\n");
    let start = assembly
        .find(&label)
        .unwrap_or_else(|| panic!("missing label {name}"))
        + label.len();
    let rest = &assembly[start..];
    let end = rest
        .find(&format!(".size {name}, .-{name}"))
        .unwrap_or_else(|| panic!("missing size directive for {name}"));
    rest[..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect()
}

#[test]
fn entry_saves_all_incoming_parameters_before_first_scratch_write() {
    let mut module = Module::new();
    let helper = module.declare("helper", &["ignored"], Linkage::External);
    {
        let mut body = module.body(&helper);
        let one = body.constant(1);
        body.ret(one);
    }
    let target = module.declare(
        "target",
        &["a", "b", "c", "d", "e", "f", "g", "h"],
        Linkage::External,
    );
    {
        let mut body = module.body(&target);
        // The first body instruction writes w0, so any delayed parameter save
        // would observe a clobbered incoming register.
        let zero = body.constant(0);
        // The call clobbers the caller-saved argument registers; parameters
        // must already be on the stack at this point.
        let helper_result = body.call(helper.symbol, vec![zero]);
        let a = body.load_param(0);
        let ten = body.constant(10);
        let scaled = body.binary(IrOp::Mul, a, ten);
        let h = body.load_param(7);
        let difference = body.binary(IrOp::Sub, scaled, h);
        let total = body.binary(IrOp::Add, difference, helper_result);
        body.ret(total);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let mut arguments = Vec::new();
        for value in 1..=8 {
            arguments.push(body.constant(value));
        }
        let result = body.call(target.symbol, arguments);
        body.ret(result);
    }

    let assembly = emit(&module).expect("fixture emits");
    let body = function_lines(&assembly, "target");

    // Hand-derived from the covered ABI and layout contract: 8 parameter
    // slots (32 bytes) + 8 value slots (32 bytes) + 8-byte LR slot = 80.
    let expected_saves = [
        "str w0, [sp, #0]",
        "str w1, [sp, #4]",
        "str w2, [sp, #8]",
        "str w3, [sp, #12]",
        "str w4, [sp, #16]",
        "str w5, [sp, #20]",
        "str w6, [sp, #24]",
        "str w7, [sp, #28]",
    ];
    let first_body = body
        .iter()
        .position(|line| {
            !line.starts_with("sub sp, sp, #")
                && !line.starts_with("str x30, [sp,")
                && !line.starts_with("str w")
        })
        .unwrap_or_else(|| panic!("missing body after prologue:\n{assembly}"));

    // The frame must be reserved first; parameter saves and the LR save may
    // appear in either order, but no scratch write may precede the saves.
    assert_eq!(
        body.first().copied(),
        Some("sub sp, sp, #80"),
        "unexpected frame reservation:\n{assembly}"
    );
    let prologue_saves: Vec<&str> = body[..first_body]
        .iter()
        .copied()
        .filter(|line| line.starts_with("str w"))
        .collect();
    assert_eq!(
        prologue_saves.as_slice(),
        expected_saves.as_slice(),
        "all eight incoming parameters must be saved before body scratch use:\n{assembly}"
    );
    assert_eq!(
        body.get(first_body).copied(),
        Some("movz w0, #0"),
        "the first body instruction must come after the parameter saves:\n{assembly}"
    );

    run_native(&assembly, 3);
}

#[cfg(target_os = "linux")]
fn run_native(assembly: &str, expected_exit: i32) {
    let machine = std::process::Command::new("gcc")
        .arg("-dumpmachine")
        .output()
        .expect("gcc available");
    if !String::from_utf8_lossy(&machine.stdout).contains("aarch64") {
        return;
    }
    let dir = std::env::temp_dir().join(format!(
        "cc-silicon-sprint-a02-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let source = dir.join("fixture.s");
    let executable = dir.join("fixture");
    std::fs::write(&source, assembly).expect("assembly file");
    let build = std::process::Command::new("gcc")
        .arg("-o")
        .arg(&executable)
        .arg(&source)
        .output()
        .expect("gcc runs");
    assert!(
        build.status.success(),
        "gcc failed for:\n{assembly}\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = std::process::Command::new(&executable)
        .output()
        .expect("program runs");
    assert_eq!(
        run.status.code(),
        Some(expected_exit),
        "unexpected native exit for:\n{assembly}"
    );
}

#[cfg(not(target_os = "linux"))]
fn run_native(assembly: &str, expected_exit: i32) {
    let _ = (assembly, expected_exit);
}
