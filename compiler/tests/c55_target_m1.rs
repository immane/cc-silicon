// ============================================================================
// c55_target_m1.rs — M1 AArch64 host-only target-emission prototype.
//
// The candidate emits GNU AArch64 assembly for the committed single-function
// Constant+Return shape from source. The multi-function direct-call prototype
// is exercised here with synthetic compiler-bus fixtures because the source
// driver is still single-function. On a native AArch64 Linux host selected
// fixtures are assembled, linked, and executed; other hosts pin the assembly
// text only. This is not a general T11 code generator.
// ============================================================================

#[path = "../src/bin/candidate/target_m1.rs"]
mod target_m1;

use cc_silicon_compiler::bus::{
    BlockRecord, CompilerBus, ConstRecord, ConstValue, FunctionRecord, InstructionRecord, IntRank,
    IntegerConst, IrOp, Linkage, ScopeKind, ScopeRecord, StorageDuration, SymbolKind, SymbolRecord,
    TypeKind, TypeRecord, ValueRecord,
};
use cc_silicon_compiler::ids::{BlockId, NameId, NodeId, ScopeId, SymbolId, TypeId, ValueId};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_candidate")
}

fn workdir(test: &str) -> PathBuf {
    static NEXT_WORKDIR: AtomicUsize = AtomicUsize::new(0);
    let serial = NEXT_WORKDIR.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "m1-target-{}-{}-{}-{}",
        std::process::id(),
        test,
        serial,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock moves forward")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

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

    fn prototype(&mut self, name: &str, parameter_count: usize) -> SymbolId {
        let name = self.intern(name);
        let signature = self.signature(parameter_count);
        let limits = self.bus.limits();
        let decl = self.next_node();
        self.bus
            .arenas
            .symbols
            .alloc(
                SymbolRecord {
                    name,
                    scope: self.file,
                    kind: SymbolKind::Function,
                    ty: Some(signature),
                    linkage: Linkage::External,
                    storage: StorageDuration::Static,
                    decl,
                },
                &limits,
            )
            .unwrap()
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
        immediate: Option<cc_silicon_compiler::ids::ConstId>,
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

    fn load_symbol(&mut self, symbol: SymbolId) -> ValueId {
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

    fn load_param(&mut self, index: usize) -> ValueId {
        let symbol = self.decl.params[index];
        self.load_symbol(symbol)
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

    fn negate(&mut self, value: ValueId) -> ValueId {
        let result = self.value();
        self.instruction(IrOp::Neg, vec![value], None, Some(result), None, None);
        result
    }

    fn call(&mut self, callee: SymbolId, arguments: Vec<ValueId>) -> ValueId {
        self.call_optional(Some(callee), arguments)
    }

    fn call_optional(&mut self, callee: Option<SymbolId>, arguments: Vec<ValueId>) -> ValueId {
        let result = self.value();
        self.instruction(IrOp::Call, arguments, None, Some(result), None, callee);
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

fn assert_no_forbidden_registers(assembly: &str) {
    for index in 16..=29 {
        for prefix in ["x", "w"] {
            assert!(
                !assembly.contains(&format!(" {prefix}{index}")),
                "callee-saved or scratch register {prefix}{index} used:\n{assembly}"
            );
        }
    }
    for index in 8..=15 {
        for prefix in ["d", "v"] {
            assert!(
                !assembly.contains(&format!(" {prefix}{index}")),
                "callee-saved FP register {prefix}{index} used:\n{assembly}"
            );
        }
    }
}

fn run_native(assembly: &str, expected_exit: i32) {
    #[cfg(target_os = "linux")]
    {
        let machine = Command::new("gcc")
            .arg("-dumpmachine")
            .output()
            .expect("gcc available");
        if !String::from_utf8_lossy(&machine.stdout).contains("aarch64") {
            return;
        }
        let dir = workdir("native");
        let source = dir.join("fixture.s");
        let executable = dir.join("fixture");
        std::fs::write(&source, assembly).expect("assembly file");
        let build = Command::new("gcc")
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
        let run = Command::new(&executable).output().expect("program runs");
        assert_eq!(
            run.status.code(),
            Some(expected_exit),
            "unexpected native exit for:\n{assembly}"
        );
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (assembly, expected_exit);
    }
}

const CANONICAL_MAIN: &str = "\
# cc-silicon AArch64 bootstrap
.text
.align 2
.globl main
.type main, %function
main:
    movz w0, #5
    ret
.size main, .-main
.section .note.GNU-stack,\"\",%progbits
";

#[test]
fn canonical_main_only_output_is_byte_identical() {
    let mut module = Module::new();
    let main = module.declare("main", &[], Linkage::External);
    let mut body = module.body(&main);
    let five = body.constant(5);
    body.ret(five);

    let assembly = emit(&module).unwrap();
    assert_eq!(assembly, CANONICAL_MAIN);
    run_native(&assembly, 5);
}

#[test]
fn candidate_emits_m1_aarch64_assembly() {
    let dir = workdir("emit");
    let source = dir.join("main.c");
    let asm = dir.join("main.s");
    std::fs::write(&source, b"int main(void){return 2+3;}\n").expect("source");
    let output = Command::new(binary())
        .arg("--emit-asm")
        .arg(&asm)
        .arg("--interpret-ir")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(std::fs::read(&asm).expect("assembly file")).unwrap();
    assert!(text.contains(".globl main"));
    assert!(text.contains("main:"));
    assert!(text.contains("movz w0, #5"));
    assert!(text.contains("    ret"));
    assert!(text.contains(".size main, .-main"));
}

#[cfg(target_os = "linux")]
#[test]
fn emitted_assembly_assembles_links_and_returns_five() {
    let machine = Command::new("gcc")
        .arg("-dumpmachine")
        .output()
        .expect("gcc available");
    let machine = String::from_utf8_lossy(&machine.stdout);
    if !machine.contains("aarch64") {
        return;
    }
    let dir = workdir("run");
    let source = dir.join("main.c");
    let asm = dir.join("main.s");
    let exe = dir.join("main");
    std::fs::write(&source, b"int main(void){return 2+3;}\n").expect("source");
    let output = Command::new(binary())
        .arg("--emit-asm")
        .arg(&asm)
        .arg("--interpret-ir")
        .arg(&source)
        .output()
        .expect("candidate runs");
    assert_eq!(output.status.code(), Some(0));
    let build = Command::new("gcc")
        .arg("-o")
        .arg(&exe)
        .arg(&asm)
        .output()
        .expect("gcc runs");
    assert!(
        build.status.success(),
        "gcc failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = Command::new(&exe).output().expect("program runs");
    assert_eq!(run.status.code(), Some(5));
}

#[test]
fn multi_function_add_returns_five() {
    let mut module = Module::new();
    let add = module.declare("add", &["a", "b"], Linkage::External);
    {
        let mut body = module.body(&add);
        let lhs = body.load_param(0);
        let rhs = body.load_param(1);
        let sum = body.binary(IrOp::Add, lhs, rhs);
        body.ret(sum);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let two = body.constant(2);
        let three = body.constant(3);
        let result = body.call(add.symbol, vec![two, three]);
        body.ret(result);
    }

    let assembly = emit(&module).unwrap();
    assert!(assembly.contains(".globl add"), "{assembly}");
    assert!(assembly.contains(".globl main"), "{assembly}");
    assert!(assembly.contains("bl add"), "{assembly}");
    assert!(assembly.contains("str w0, [sp, #0]"), "{assembly}");
    assert!(assembly.contains("str w1, [sp, #4]"), "{assembly}");
    assert_no_forbidden_registers(&assembly);
    run_native(&assembly, 5);
}

#[test]
fn nested_calls_return_seven() {
    let mut module = Module::new();
    let inc = module.declare("inc", &["x"], Linkage::External);
    {
        let mut body = module.body(&inc);
        let x = body.load_param(0);
        let one = body.constant(1);
        let result = body.binary(IrOp::Add, x, one);
        body.ret(result);
    }
    let add = module.declare("add", &["a", "b"], Linkage::External);
    {
        let mut body = module.body(&add);
        let lhs = body.load_param(0);
        let rhs = body.load_param(1);
        let sum = body.binary(IrOp::Add, lhs, rhs);
        body.ret(sum);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let two = body.constant(2);
        let first = body.call(inc.symbol, vec![two]);
        let three = body.constant(3);
        let second = body.call(inc.symbol, vec![three]);
        let result = body.call(add.symbol, vec![first, second]);
        body.ret(result);
    }

    let assembly = emit(&module).unwrap();
    assert_eq!(
        assembly
            .lines()
            .filter(|line| line.trim() == "bl inc")
            .count(),
        2,
        "{assembly}"
    );
    assert_eq!(
        assembly
            .lines()
            .filter(|line| line.trim() == "bl add")
            .count(),
        1,
        "{assembly}"
    );
    assert_no_forbidden_registers(&assembly);
    run_native(&assembly, 7);
}

#[test]
fn eight_parameters_use_ascending_argument_registers() {
    let mut module = Module::new();
    let names = ["a", "b", "c", "d", "e", "f", "g", "h"];
    let weighted = module.declare("weighted", &names, Linkage::External);
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

    let assembly = emit(&module).unwrap();
    for index in 0..8 {
        assert!(
            assembly.contains(&format!("ldr w{index}, [sp,")),
            "{assembly}"
        );
    }
    assert_no_forbidden_registers(&assembly);
    run_native(&assembly, 252);
}

#[test]
fn negative_call_result_returns_negated_value() {
    let mut module = Module::new();
    let negate = module.declare("negate", &["x"], Linkage::External);
    {
        let mut body = module.body(&negate);
        let x = body.load_param(0);
        let result = body.negate(x);
        body.ret(result);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let three = body.constant(3);
        let result = body.call(negate.symbol, vec![three]);
        body.ret(result);
    }

    let assembly = emit(&module).unwrap();
    assert!(assembly.contains("neg w0, w0"), "{assembly}");
    run_native(&assembly, 253);
}

#[test]
fn two_calls_keep_live_results() {
    let mut module = Module::new();
    let sub = module.declare("sub", &["a", "b"], Linkage::External);
    {
        let mut body = module.body(&sub);
        let lhs = body.load_param(0);
        let rhs = body.load_param(1);
        let difference = body.binary(IrOp::Sub, lhs, rhs);
        body.ret(difference);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let seven = body.constant(7);
        let two = body.constant(2);
        let first = body.call(sub.symbol, vec![seven, two]);
        let three = body.constant(3);
        let second = body.call(sub.symbol, vec![first, three]);
        body.ret(second);
    }

    let assembly = emit(&module).unwrap();
    assert_eq!(
        assembly
            .lines()
            .filter(|line| line.trim() == "bl sub")
            .count(),
        2,
        "{assembly}"
    );
    run_native(&assembly, 2);
}

#[test]
fn parameter_slots_are_per_function() {
    let mut module = Module::new();
    let add = module.declare("add", &["a", "b"], Linkage::External);
    {
        let mut body = module.body(&add);
        let lhs = body.load_param(0);
        let rhs = body.load_param(1);
        let sum = body.binary(IrOp::Add, lhs, rhs);
        body.ret(sum);
    }
    let sub = module.declare("sub", &["a", "b"], Linkage::External);
    {
        let mut body = module.body(&sub);
        let lhs = body.load_param(0);
        let rhs = body.load_param(1);
        let difference = body.binary(IrOp::Sub, lhs, rhs);
        body.ret(difference);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let two = body.constant(2);
        let three = body.constant(3);
        let sum = body.call(add.symbol, vec![two, three]);
        let ten = body.constant(10);
        let twenty = body.constant(20);
        let difference = body.call(sub.symbol, vec![ten, twenty]);
        let result = body.binary(IrOp::Add, sum, difference);
        body.ret(result);
    }

    let assembly = emit(&module).unwrap();
    for name in ["add", "sub"] {
        let text = function_text(&assembly, name);
        assert!(text.contains("str w0, [sp, #0]"), "{name}: {assembly}");
        assert!(text.contains("str w1, [sp, #4]"), "{name}: {assembly}");
    }
    run_native(&assembly, 251);
}

#[test]
fn locals_and_parameters_share_the_function_frame() {
    let mut module = Module::new();
    let mix = module.declare("mix", &["a"], Linkage::External);
    {
        let mut body = module.body(&mix);
        let local = body.local("x");
        let three = body.constant(3);
        body.store_symbol(local, three);
        let loaded = body.load_symbol(local);
        let parameter = body.load_param(0);
        let sum = body.binary(IrOp::Add, loaded, parameter);
        body.store_symbol(local, sum);
        let result = body.load_symbol(local);
        body.ret(result);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let two = body.constant(2);
        let result = body.call(mix.symbol, vec![two]);
        body.ret(result);
    }

    let assembly = emit(&module).unwrap();
    let text = function_text(&assembly, "mix");
    assert!(text.contains("str w0, [sp, #0]"), "{assembly}");
    assert!(text.contains("str w0, [sp, #4]"), "{assembly}");
    run_native(&assembly, 5);
}

#[test]
fn leaf_functions_skip_lr_and_nonleaf_save_it() {
    let mut module = Module::new();
    let leaf = module.declare("leaf", &["x"], Linkage::External);
    {
        let mut body = module.body(&leaf);
        let x = body.load_param(0);
        body.ret(x);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let five = body.constant(5);
        let result = body.call(leaf.symbol, vec![five]);
        body.ret(result);
    }

    let assembly = emit(&module).unwrap();
    let leaf_text = function_text(&assembly, "leaf");
    assert!(!leaf_text.contains("x30"), "{assembly}");
    let main_text = function_text(&assembly, "main");
    assert!(main_text.contains("str x30, [sp,"), "{assembly}");
    assert!(main_text.contains("ldr x30, [sp,"), "{assembly}");
    assert_eq!(assembly.matches("str x30").count(), 1, "{assembly}");
    assert_no_forbidden_registers(&assembly);
    run_native(&assembly, 5);
}

#[test]
fn internal_linkage_omits_globl() {
    let mut module = Module::new();
    let helper = module.declare("helper", &["x"], Linkage::Internal);
    {
        let mut body = module.body(&helper);
        let x = body.load_param(0);
        let one = body.constant(1);
        let result = body.binary(IrOp::Add, x, one);
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
    assert!(!assembly.contains(".globl helper"), "{assembly}");
    assert!(assembly.contains(".type helper, %function"), "{assembly}");
    assert!(assembly.contains(".size helper, .-helper"), "{assembly}");
    assert!(assembly.contains(".globl main"), "{assembly}");
    assert_eq!(
        assembly.matches(".section .note.GNU-stack").count(),
        1,
        "{assembly}"
    );
    run_native(&assembly, 5);
}

#[test]
fn unsafe_names_and_duplicate_functions_fail() {
    for name in [
        "x0", "w30", "sp", "wsp", "xzr", "wzr", "lr", "fp", "ip0", "ip1", "d0", "s31", "q15", "v8",
        "eq", "al", "bad name", "9lives", "café",
    ] {
        let mut module = Module::new();
        let _main = module.declare("main", &[], Linkage::External);
        let _bad = module.declare(name, &[], Linkage::External);
        let error = emit(&module).unwrap_err();
        assert!(error.contains("unsafe name"), "{name}: {error}");
    }

    let mut module = Module::new();
    let _main = module.declare("main", &[], Linkage::External);
    let _first = module.declare("dup", &[], Linkage::External);
    let _second = module.declare("dup", &[], Linkage::External);
    let error = emit(&module).unwrap_err();
    assert!(error.contains("duplicate function symbol"), "{error}");

    let mut module = Module::new();
    let _main = module.declare("main", &[], Linkage::External);
    let _duplicate_params = module.declare("twice", &["a", "a"], Linkage::External);
    let error = emit(&module).unwrap_err();
    assert!(error.contains("duplicate parameter name"), "{error}");
}

#[test]
fn nine_arguments_fail() {
    let mut module = Module::new();
    let names = ["a", "b", "c", "d", "e", "f", "g", "h"];
    let callee = module.declare("eight", &names, Linkage::External);
    {
        let mut body = module.body(&callee);
        let value = body.load_param(0);
        body.ret(value);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let mut arguments = Vec::new();
        for value in 1..=9 {
            arguments.push(body.constant(value));
        }
        let result = body.call(callee.symbol, arguments);
        body.ret(result);
    }

    let error = emit(&module).unwrap_err();
    assert!(error.contains("more than eight arguments"), "{error}");

    let mut module = Module::new();
    let _main = module.declare("main", &[], Linkage::External);
    let _nine = module.declare(
        "nine",
        &["a", "b", "c", "d", "e", "f", "g", "h", "i"],
        Linkage::External,
    );
    let error = emit(&module).unwrap_err();
    assert!(error.contains("more than eight arguments"), "{error}");
}

#[test]
fn missing_and_duplicate_callees_fail() {
    let mut module = Module::new();
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let one = body.constant(1);
        let result = body.call_optional(None, vec![one]);
        body.ret(result);
    }
    let error = emit(&module).unwrap_err();
    assert!(error.contains("unresolved callee"), "{error}");

    let mut module = Module::new();
    let missing = module.prototype("missing", 1);
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let one = body.constant(1);
        let result = body.call(missing, vec![one]);
        body.ret(result);
    }
    let error = emit(&module).unwrap_err();
    assert!(error.contains("unresolved callee"), "{error}");

    let mut module = Module::new();
    let _main = module.declare("main", &[], Linkage::External);
    let _first = module.declare("dup", &[], Linkage::External);
    let _second = module.declare("dup", &[], Linkage::External);
    let error = emit(&module).unwrap_err();
    assert!(error.contains("duplicate function symbol"), "{error}");
}

#[test]
fn call_arity_mismatch_fails() {
    let mut module = Module::new();
    let add = module.declare("add", &["a", "b"], Linkage::External);
    {
        let mut body = module.body(&add);
        let lhs = body.load_param(0);
        let rhs = body.load_param(1);
        let result = body.binary(IrOp::Add, lhs, rhs);
        body.ret(result);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let one = body.constant(1);
        let result = body.call(add.symbol, vec![one]);
        body.ret(result);
    }
    let error = emit(&module).unwrap_err();
    assert!(error.contains("call arity mismatch"), "{error}");
}

#[test]
fn recursive_and_cyclic_calls_fail() {
    let mut module = Module::new();
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let result = body.call(main.symbol, vec![]);
        body.ret(result);
    }
    let error = emit(&module).unwrap_err();
    assert!(error.contains("recursive or cyclic call graph"), "{error}");

    let mut module = Module::new();
    let first = module.declare("first", &[], Linkage::External);
    let second = module.declare("second", &[], Linkage::External);
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&first);
        let result = body.call(second.symbol, vec![]);
        body.ret(result);
    }
    {
        let mut body = module.body(&second);
        let result = body.call(first.symbol, vec![]);
        body.ret(result);
    }
    {
        let mut body = module.body(&main);
        let seven = body.constant(7);
        body.ret(seven);
    }
    let error = emit(&module).unwrap_err();
    assert!(error.contains("recursive or cyclic call graph"), "{error}");
}

#[test]
fn cross_function_operand_fails() {
    let mut module = Module::new();
    let source = module.declare("source", &[], Linkage::External);
    let foreign = {
        let mut body = module.body(&source);
        let seven = body.constant(7);
        body.ret(seven);
        seven
    };
    let add = module.declare("add", &["a", "b"], Linkage::External);
    {
        let mut body = module.body(&add);
        let lhs = body.load_param(0);
        let rhs = body.load_param(1);
        let sum = body.binary(IrOp::Add, lhs, rhs);
        body.ret(sum);
    }
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        let two = body.constant(2);
        let result = body.call(add.symbol, vec![foreign, two]);
        body.ret(result);
    }

    let error = emit(&module).unwrap_err();
    assert!(error.contains("cross-function value"), "{error}");
}

#[test]
fn frame_over_the_immediate_range_fails() {
    let mut module = Module::new();
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        for value in 0..1019 {
            let _ = body.constant(value);
        }
        let last = body.constant(1);
        body.ret(last);
    }
    let assembly = emit(&module).unwrap();
    assert!(assembly.contains("sub sp, sp, #4080"), "{assembly}");

    let mut module = Module::new();
    let main = module.declare("main", &[], Linkage::External);
    {
        let mut body = module.body(&main);
        for value in 0..1021 {
            let _ = body.constant(value);
        }
        let last = body.constant(1);
        body.ret(last);
    }
    let error = emit(&module).unwrap_err();
    assert!(error.contains("frame exceeds"), "{error}");
}

#[test]
fn missing_main_and_wrong_main_signature_fail() {
    let module = Module::new();
    let error = emit(&module).unwrap_err();
    assert!(error.contains("missing main"), "{error}");

    let mut module = Module::new();
    let helper = module.declare("helper", &[], Linkage::External);
    {
        let mut body = module.body(&helper);
        let seven = body.constant(7);
        body.ret(seven);
    }
    let error = emit(&module).unwrap_err();
    assert!(error.contains("missing main"), "{error}");

    let mut module = Module::new();
    let main = module.declare("main", &["x"], Linkage::External);
    {
        let mut body = module.body(&main);
        let x = body.load_param(0);
        body.ret(x);
    }
    let error = emit(&module).unwrap_err();
    assert!(error.contains("main must be int(void)"), "{error}");

    let mut module = Module::new();
    let main = module.declare("main", &[], Linkage::Internal);
    {
        let mut body = module.body(&main);
        let seven = body.constant(7);
        body.ret(seven);
    }
    let error = emit(&module).unwrap_err();
    assert!(error.contains("main must be int(void)"), "{error}");
}
