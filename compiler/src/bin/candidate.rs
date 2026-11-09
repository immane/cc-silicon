// ============================================================================
// bin/candidate.rs — H04 candidate command-line driver (Part A).
//
// Host-side orchestration only: it reads source bytes, drives the frozen
// M1 worker pipeline from real tasks (PP01→PP02→PP03→PP04→LX→PA→TY→
// SE literal→SE binary→SE return→SE29 function fact→VF06/VF05→IR
// (consuming the function fact)→VF12→VF01), and prints Part A evidence
// (snapshot hash, trace hash, modeled constant). It performs no compilation
// itself, never shells out to another C compiler, and refuses target
// emission (`-S`, `-c`) while the target is unverified (fail-closed, Part B).
// Part B adds
// host-only `-E` preprocessed emission through the frozen PP28 worker
// (no contract change, no version bump), and H04 `-D`/`-U` seeds the frozen
// macro table in flag order before preprocessing (host-only, no contract
// change, no version bump). Remaining flag spellings are illustrative
// until the H04 driver contract freezes; `-I` and multi-source
// compilations are explicit deferred errors, never silent fallbacks.
//
// Exit codes: 0 = evidence produced; 1 = candidate diagnostic (the input
// is rejected with a structured message, no evidence); 2 = driver error
// (usage, unsupported flag, refused emission, or a non-UTF-8 argument).
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, NodeKind, TokenKind};
use cc_silicon_compiler::chips::{
    handler_for, FoldChip, IrFunctionChip, LxClassifyChip, LxDecodeLiteralChip, LxInternChip,
    PaTuChip, PpCommentChip, PpConditionalChip, PpDefineChip, PpDiagnosticChip, PpDirectiveChip,
    PpEmitChip, PpIncludeEnterChip, PpIncludeResolveChip, PpInvokeChip, PpNormalizeChip,
    PpRedefineChip, PpScanChip, PpSpliceChip, PpSubstituteChip, PpUndefChip, SeAssignmentChip,
    SeBinChip, SeFuncChip, SeIdentifierChip, SeLitChip, SeRetChip, SeUnaryChip, TyConvChip,
    TyScopeChip, TySymbolChip, TyTypeChip, Vf01Chip, Vf05Chip, Vf06Chip, Vf12Chip, Worker,
    WorkerRegistry,
};
use cc_silicon_compiler::codec::hex32;
use cc_silicon_compiler::contract::CONTRACT_VERSION;
use cc_silicon_compiler::ids::{ChipId, NodeId, RecordRef, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    StoreSchema, G1_FOLD_CHIP, IR_FUNCTION_CHIP, LX_CLASSIFY_CHIP, LX_DECODE_CHIP, LX_INTERN_CHIP,
    PA_TU_CHIP, PP01_CHIP, PP05_CHIP, PP06_CHIP, PP07_CHIP, PP08_CHIP, PP09_CHIP, PP12_CHIP,
    PP17_CHIP, PP18_CHIP, PP19_CHIP, PP26_CHIP, PP28_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP,
    PP_SPLICE_CHIP, SE_ASSIGN_CHIP, SE_BIN_CHIP, SE_FUNC_CHIP, SE_ID_CHIP, SE_LIT_CHIP,
    SE_RET_CHIP, SE_UNARY_CHIP, TY_CONV_CHIP, TY_SCOPE_CHIP, TY_SYMBOL_CHIP, TY_TYPE_CHIP,
    VF01_CHIP, VF05_CHIP, VF06_CHIP, VF12_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{Snapshot, Trace};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, ResultValue, TaskDraft, TaskKind, TaskKindRegistry, TaskState,
};

/// H04 Part B `-E` terminal host step (pure orchestration; chip does the
/// work, host owns file/stdout persistence).
#[path = "candidate/emit_helper.rs"]
mod emit_helper;

/// H04 `-D`/`-U` flag parsing helper (pure host-side argv parsing plus the
/// frozen `MacroRecord` shapes; value scanning and record commit stay here
/// with the integrator so chips own every semantic decision).
#[path = "candidate/macro_flags.rs"]
mod macro_flags;

/// B3-03a host-only PP line/directive grouping helper.
#[path = "candidate/pp_driver.rs"]
mod pp_driver;

/// B3-03b bounded quote-include host policy.
#[path = "candidate/pp_include.rs"]
mod pp_include;

/// M1 AArch64 assembly prototype (host-only T11 bootstrap).
#[path = "candidate/target_m1.rs"]
mod target_m1;

/// Maximum drain ticks per pipeline step (M1 needs a handful for joins).
const DRAIN_BUDGET: u32 = 16;

/// Bounded quote-include limit per translation unit.
const MAX_INCLUDES: usize = 64;

/// Parsed Part A invocation.
struct Invocation {
    /// Single input source path.
    input: String,
    /// Optional evidence report path (`-o`).
    output: Option<String>,
    /// Evidence sections (empty = all).
    snapshot: bool,
    trace: bool,
    interpret: bool,
    /// Preprocessed emission (`-E`, Part B).
    emit: bool,
    /// `-D`/`-U` macro seeds in flag order (last-wins by commit order).
    macro_flags: Vec<macro_flags::MacroFlagSeed>,
    /// Optional M1 AArch64 assembly output path (`--emit-asm`).
    asm_output: Option<String>,
}

fn usage() -> &'static str {
    "usage: candidate [-std=c11] [-O0] [-D NAME[=value]] [-U NAME] [-o <path>] [--emit-ir-snapshot] [--emit-trace] [--interpret-ir] file.c\n\
     usage: candidate -E [-D NAME[=value]] [-U NAME] [-o <path>] file.c\n\
     usage: candidate --emit-asm <path> [-D NAME[=value]] [-U NAME] file.c\n\
     Part A evidence driver (M1): snapshot/trace emission and IR interpretation; --emit-asm is the host-only M1 AArch64 prototype.\n\
     -D NAME defines NAME as 1; -D NAME=value defines NAME with that replacement text (empty value defines it as empty).\n\
     -U NAME undefines NAME; repeated -D/-U for one name resolve last-wins in flag order.\n\
     -E emits the preprocessed source (PP28) to stdout or -o <path> (Part B, host-only).\n\
     -S and -c are refused while the target is unverified (fail-closed, Part B).\n\
     -I and multi-source compilations are deferred (explicit error)."
}

fn parse_args(args: &[String]) -> Result<Invocation, String> {
    let mut invocation = Invocation {
        input: String::new(),
        output: None,
        snapshot: false,
        trace: false,
        interpret: false,
        emit: false,
        macro_flags: Vec::new(),
        asm_output: None,
    };
    let mut inputs = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "-h" || arg == "--help" {
            return Err("help".to_string());
        } else if arg == "-S" || arg == "-c" {
            return Err(format!(
                "{arg} refused: target emission is fail-closed while the target is unverified (Part B, probe-gated)"
            ));
        } else if arg == "-E" {
            invocation.emit = true;
        } else if arg == "--emit-asm" {
            index += 1;
            let path = args
                .get(index)
                .ok_or_else(|| "--emit-asm requires a path".to_string())?;
            invocation.asm_output = Some(path.clone());
        } else if arg == "-o" {
            index += 1;
            let path = args
                .get(index)
                .ok_or_else(|| "-o requires a path".to_string())?;
            invocation.output = Some(path.clone());
        } else if let Some(std) = arg.strip_prefix("-std=") {
            if std != "c11" {
                return Err(format!("-std={std} unsupported in Part A (only c11)"));
            }
        } else if arg == "-std" {
            index += 1;
            let std = args
                .get(index)
                .ok_or_else(|| "-std requires a value".to_string())?;
            if std != "c11" {
                return Err(format!("-std {std} unsupported in Part A (only c11)"));
            }
        } else if arg.starts_with("-O") && arg.len() == 3 {
            if arg != "-O0" {
                return Err(format!("{arg} unsupported in Part A (only -O0)"));
            }
        } else if arg == "-O" {
            index += 1;
            let level = args
                .get(index)
                .ok_or_else(|| "-O requires a level".to_string())?;
            if level != "0" {
                return Err(format!("-O {level} unsupported in Part A (only -O0)"));
            }
        } else if arg == "-I" || arg.starts_with("-I") {
            return Err(format!("{arg} deferred: include support is not in Part A"));
        } else if let Some((kind, body)) = macro_flags::split_flag(arg) {
            let body: &str = if body.is_empty() {
                index += 1;
                match args.get(index) {
                    Some(next) => next.as_str(),
                    None => {
                        return Err(if kind == 'D' {
                            "-D requires a macro name".to_string()
                        } else {
                            "-U requires a macro name".to_string()
                        });
                    }
                }
            } else {
                body
            };
            if kind == 'D' {
                let seed = macro_flags::parse_define_body(body)?;
                // Fail fast on values the frozen scan rejects so a bad `-D`
                // value stays a driver error (exit 2), not a pipeline
                // diagnostic. The commit path below re-scans the same bytes.
                if let Err(draft) = cc_silicon_compiler::chips::scan(seed.value.as_slice()) {
                    return Err(format!(
                        "-D {} has an invalid value: {}",
                        String::from_utf8_lossy(&seed.name),
                        draft.message
                    ));
                }
                invocation
                    .macro_flags
                    .push(macro_flags::MacroFlagSeed::Define(seed));
            } else {
                let seed = macro_flags::parse_undef_body(body)?;
                invocation
                    .macro_flags
                    .push(macro_flags::MacroFlagSeed::Undef(seed));
            }
        } else if arg == "--emit-ir-snapshot" {
            invocation.snapshot = true;
        } else if arg == "--emit-trace" {
            invocation.trace = true;
        } else if arg == "--interpret-ir" {
            invocation.interpret = true;
        } else if arg.starts_with('-') {
            return Err(format!("unknown flag {arg}"));
        } else {
            inputs.push(arg.clone());
        }
        index += 1;
    }
    if inputs.len() != 1 {
        return Err(format!(
            "Part A compiles exactly one source (got {})",
            inputs.len()
        ));
    }
    invocation.input = inputs.pop().expect("one input checked above");
    if invocation.emit {
        if invocation.snapshot || invocation.trace || invocation.interpret {
            return Err(
                "-E clashes with evidence selection (--emit-ir-snapshot/--emit-trace/--interpret-ir)"
                    .to_string(),
            );
        }
    } else if !invocation.snapshot && !invocation.trace && !invocation.interpret {
        invocation.snapshot = true;
        invocation.trace = true;
        invocation.interpret = true;
    }
    Ok(invocation)
}

fn install(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::se_function_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    bus.registrations
        .register(FoldChip.manifest(), &bus.schema, &bus.kinds)
        .expect("fold manifest registers");
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
        &PpDefineChip,
        &PpRedefineChip,
        &PpUndefChip,
        &PpConditionalChip,
        &PpInvokeChip,
        &PpSubstituteChip,
        &PpDirectiveChip,
        &PpDiagnosticChip,
        &PpIncludeResolveChip,
        &PpIncludeEnterChip,
        &PpEmitChip,
        &LxInternChip,
        &LxClassifyChip,
        &LxDecodeLiteralChip,
        &PaTuChip,
        &TyTypeChip,
        &TyScopeChip,
        &TySymbolChip,
        &TyConvChip,
        &SeLitChip,
        &SeBinChip,
        &SeRetChip,
        &SeFuncChip,
        &SeIdentifierChip,
        &SeUnaryChip,
        &SeAssignmentChip,
        &Vf06Chip,
        &Vf05Chip,
        &IrFunctionChip,
        &Vf12Chip,
        &Vf01Chip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .expect("worker manifest registers");
    }
    let routes: &[(TaskKind, ChipId, u16)] = &[
        (TaskKind::PREPROCESS_NORMALIZE, PP01_CHIP, 1),
        (TaskKind::PREPROCESS_SPLICE, PP_SPLICE_CHIP, 1),
        (TaskKind::PREPROCESS_COMMENT, PP_COMMENT_CHIP, 1),
        (TaskKind::PREPROCESS_SCAN, PP_SCAN_CHIP, 1),
        (TaskKind::PREPROCESS_MACRO_DEFINE, PP06_CHIP, 1),
        (TaskKind::PREPROCESS_MACRO_REDEFINE, PP07_CHIP, 1),
        (TaskKind::PREPROCESS_MACRO_UNDEF, PP08_CHIP, 1),
        (TaskKind::PREPROCESS_CONDITIONAL, PP19_CHIP, 1),
        (TaskKind::PREPROCESS_MACRO_INVOKE, PP09_CHIP, 1),
        (TaskKind::PREPROCESS_MACRO_SUBSTITUTE, PP12_CHIP, 1),
        (TaskKind::PREPROCESS_DIRECTIVE, PP05_CHIP, 1),
        (TaskKind::PREPROCESS_DIAGNOSTIC, PP26_CHIP, 1),
        (TaskKind::PREPROCESS_INCLUDE_RESOLVE, PP17_CHIP, 1),
        (TaskKind::PREPROCESS_INCLUDE_ENTER, PP18_CHIP, 1),
        (TaskKind::PREPROCESS_EMIT, PP28_CHIP, 1),
        (TaskKind::LEX_INTERN, LX_INTERN_CHIP, 2),
        (TaskKind::LEX_CLASSIFY, LX_CLASSIFY_CHIP, 2),
        (TaskKind::LEX_DECODE_LITERAL, LX_DECODE_CHIP, 2),
        (TaskKind::PARSE_TU, PA_TU_CHIP, 2),
        (TaskKind::SYMBOL_INT_TYPE, TY_TYPE_CHIP, 3),
        (TaskKind::SYMBOL_FUNC_TYPE, TY_TYPE_CHIP, 3),
        (TaskKind::SYMBOL_SCOPE_ENTER, TY_SCOPE_CHIP, 3),
        (TaskKind::SYMBOL_SCOPE_EXIT, TY_SCOPE_CHIP, 3),
        (TaskKind::SYMBOL_DECLARE, TY_SYMBOL_CHIP, 3),
        (TaskKind::SYMBOL_LOOKUP, TY_SYMBOL_CHIP, 3),
        (TaskKind::SYMBOL_PROMOTE, TY_CONV_CHIP, 3),
        (TaskKind::SYMBOL_COMMON_TYPE, TY_CONV_CHIP, 3),
        (TaskKind::SYMBOL_RETURN_CONVERT, TY_CONV_CHIP, 3),
        (TaskKind::SEMANTIC_LITERAL_EXPR, SE_LIT_CHIP, 4),
        (TaskKind::SEMANTIC_BINARY_EXPR, SE_BIN_CHIP, 4),
        (TaskKind::SEMANTIC_RETURN_STMT, SE_RET_CHIP, 4),
        (TaskKind::SEMANTIC_FUNCTION_DEF, SE_FUNC_CHIP, 4),
        (TaskKind::SEMANTIC_IDENTIFIER_EXPR, SE_ID_CHIP, 4),
        (TaskKind::SEMANTIC_UNARY_EXPR, SE_UNARY_CHIP, 4),
        (TaskKind::SEMANTIC_ASSIGNMENT_EXPR, SE_ASSIGN_CHIP, 4),
        (TaskKind::VERIFICATION_TYPED_INVARIANT, VF06_CHIP, 4),
        (TaskKind::VERIFICATION_TOKEN_AST_INVARIANT, VF05_CHIP, 2),
        (TaskKind::IR_FUNCTION, IR_FUNCTION_CHIP, 5),
        (TaskKind::VERIFICATION_IR_INTERPRET, VF12_CHIP, 6),
        (TaskKind::VERIFICATION_STORE_INVARIANT, VF01_CHIP, 6),
        (
            TaskKind::CONSTANT_CONST_FOLD,
            cc_silicon_compiler::manifest::G1_FOLD_CHIP,
            2,
        ),
    ];
    for (kind, chip, layer) in routes {
        bus.routing
            .register(*kind, *chip, *layer)
            .expect("route registers");
    }
}

/// One pipeline step: bootstrap, drain to quiescence, and collect the
/// completed value (or the structured failure).
fn quiescent(bus: &CompilerBus) -> bool {
    bus.arenas
        .tasks
        .iter()
        .all(|(_, task)| matches!(task.state, TaskState::Completed(_) | TaskState::Failed(_)))
}

fn step(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
    trace: &mut Trace,
    kind: TaskKind,
    owner: ChipId,
    payload: Payload,
) -> Result<ResultValue, String> {
    let task = bus
        .bootstrap_task(TaskDraft {
            kind,
            owner,
            parent: None,
            payload,
            continuation: None,
        })
        .map_err(|error| format!("bootstrap {kind:?} failed: {error}"))?;
    for _ in 0..DRAIN_BUDGET {
        if quiescent(bus) {
            break;
        }
        let pins = CompilerPins::default();
        let report = RoutingShell
            .clock_tick_with(&pins, bus, handler_for(workers))
            .map_err(|error| format!("tick failed: {error}"))?;
        trace.record(bus, &report, &pins);
        if matches!(report.outcome, TickOutcome::BudgetExhausted) {
            return Err("tick budget exhausted".to_string());
        }
    }
    if !quiescent(bus) {
        return Err("drain budget exhausted with tasks still live".to_string());
    }
    match &bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| "step task vanished".to_string())?
        .state
    {
        TaskState::Completed(result) => {
            let value = bus
                .arenas
                .results
                .get(*result)
                .map_err(|_| "step result vanished".to_string())?
                .value
                .clone();
            Ok(value)
        }
        TaskState::Failed(diagnostic) => {
            let message = bus
                .arenas
                .diagnostics
                .get(*diagnostic)
                .map(|record| record.message.clone())
                .unwrap_or_else(|_| "unresolved diagnostic".to_string());
            Err(format!("candidate diagnostic: {message}"))
        }
        _ => Err("step task is not terminal after drain".to_string()),
    }
}

/// Post-order traversal of one expression subtree (children before parent).
fn expression_postorder(
    bus: &CompilerBus,
    root: NodeId,
    out: &mut Vec<NodeId>,
) -> Result<(), String> {
    let body = bus
        .arenas
        .nodes
        .get(root)
        .map_err(|_| format!("expression node {} vanished", root.index()))?;
    for child in &body.children {
        expression_postorder(bus, *child, out)?;
    }
    out.push(root);
    Ok(())
}

fn run_literal(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
    trace: &mut Trace,
    node: NodeId,
    fold: bool,
) -> Result<(), String> {
    step(
        bus,
        workers,
        trace,
        TaskKind::SEMANTIC_LITERAL_EXPR,
        SE_LIT_CHIP,
        Payload::from_refs(vec![RecordRef::Node(node)]),
    )?;
    if !fold {
        return Ok(());
    }
    let literal = bus
        .arenas
        .nodes
        .get(node)
        .map_err(|_| "literal node vanished".to_string())?
        .literal
        .ok_or_else(|| "literal node carries no literal".to_string())?;
    step(
        bus,
        workers,
        trace,
        TaskKind::CONSTANT_CONST_FOLD,
        G1_FOLD_CHIP,
        Payload::from_refs(vec![RecordRef::Literal(literal)]),
    )?;
    Ok(())
}

fn run_expression_node(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
    trace: &mut Trace,
    node: NodeId,
) -> Result<(), String> {
    let kind = bus
        .arenas
        .nodes
        .get(node)
        .map_err(|_| "expression node vanished".to_string())?
        .kind;
    match kind {
        NodeKind::IntLiteral => run_literal(bus, workers, trace, node, true),
        NodeKind::Identifier => {
            step(
                bus,
                workers,
                trace,
                TaskKind::SEMANTIC_IDENTIFIER_EXPR,
                SE_ID_CHIP,
                Payload::from_refs(vec![RecordRef::Node(node)]),
            )?;
            Ok(())
        }
        NodeKind::BinaryAdd
        | NodeKind::BinarySub
        | NodeKind::BinaryMul
        | NodeKind::BinaryDiv
        | NodeKind::BinaryRem => {
            step(
                bus,
                workers,
                trace,
                TaskKind::SEMANTIC_BINARY_EXPR,
                SE_BIN_CHIP,
                Payload::from_refs(vec![RecordRef::Node(node)]),
            )?;
            Ok(())
        }
        NodeKind::UnaryPlus | NodeKind::UnaryMinus => {
            step(
                bus,
                workers,
                trace,
                TaskKind::SEMANTIC_UNARY_EXPR,
                SE_UNARY_CHIP,
                Payload::from_refs(vec![RecordRef::Node(node)]),
            )?;
            Ok(())
        }
        _ => Err(format!(
            "unsupported expression node kind {kind:?} in locals shape"
        )),
    }
}

/// Find nodes of one kind in ascending-ID order.
fn nodes_of(bus: &CompilerBus, kind: NodeKind) -> Vec<NodeId> {
    let mut ids: Vec<NodeId> = bus
        .arenas
        .nodes
        .iter()
        .filter(|(_, body)| body.kind == kind)
        .map(|(id, _)| id)
        .collect();
    ids.sort_by_key(|id| id.index());
    ids
}

fn build_workers() -> Result<WorkerRegistry, String> {
    let mut workers = WorkerRegistry::new();
    workers
        .register(PpNormalizeChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpSpliceChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpCommentChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpScanChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpDefineChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpRedefineChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpUndefChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpConditionalChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpInvokeChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpSubstituteChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpDirectiveChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpDiagnosticChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpIncludeResolveChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpIncludeEnterChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PpEmitChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(LxInternChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(LxClassifyChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(LxDecodeLiteralChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(PaTuChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(TyTypeChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(TyScopeChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(TySymbolChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(TyConvChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(SeLitChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(SeBinChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(SeRetChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(SeFuncChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(SeIdentifierChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(SeUnaryChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(SeAssignmentChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(Vf06Chip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(Vf05Chip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(IrFunctionChip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(Vf12Chip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(Vf01Chip)
        .map_err(|error| format!("worker registration failed: {error}"))?;
    workers
        .register(FoldChip)
        .map_err(|error| format!("fold worker registration failed: {error}"))?;
    Ok(workers)
}

/// Commit `-D`/`-U` seeds in flag order before the PP pipeline.
///
/// Host-side initial input (like source bytes): names are validated by
/// `macro_flags`, values are tokenized by the frozen `scan` path, and
/// records use the frozen `define_record`/`undef_record` shapes. Each
/// non-empty `-D` value gets its own `<command-line>` source so
/// replacement spans are raw offsets into committed bytes (an empty value
/// commits no tokens: defined-as-empty, distinct from `-U`). Lookup takes
/// the greatest `MacroId` with equal spelling, so flag order is last-wins
/// with no dedup here. Values were trial-scanned in `parse_args`, so a
/// scan failure here is unreachable in practice and surfaces as a driver
/// error string.
fn commit_macro_flags(
    bus: &mut CompilerBus,
    seeds: &[macro_flags::MacroFlagSeed],
) -> Result<(), String> {
    use cc_silicon_compiler::bus::{PpTokenRecord, SpanRecord};
    let limits = bus.limits();
    let command_line = bus
        .intern_name(b"<command-line>")
        .map_err(|error| format!("name table exhausted: {error}"))?;
    for seed in seeds {
        match seed {
            macro_flags::MacroFlagSeed::Define(define) => {
                let scanned =
                    cc_silicon_compiler::chips::scan(define.value.as_slice()).map_err(|draft| {
                        format!(
                            "-D {} has an invalid value: {}",
                            String::from_utf8_lossy(&define.name),
                            draft.message
                        )
                    })?;
                let source = if define.value.is_empty() {
                    None
                } else {
                    Some(
                        bus.alloc_source(command_line, define.value.clone())
                            .map_err(|error| format!("command-line source rejected: {error}"))?,
                    )
                };
                let mut replacement = Vec::with_capacity(scanned.len());
                for token in &scanned {
                    let owner = source.ok_or_else(|| {
                        "non-empty -D value has no command-line source".to_string()
                    })?;
                    bus.ensure_total_records(1)
                        .map_err(|error| format!("span commit rejected: {error}"))?;
                    let span = bus
                        .arenas
                        .spans
                        .alloc(
                            SpanRecord {
                                source: owner,
                                start: token.start as u64,
                                end: token.end as u64,
                                expansion: None,
                            },
                            &limits,
                        )
                        .map_err(|error| format!("span commit rejected: {error}"))?;
                    bus.ensure_total_records(1)
                        .map_err(|error| format!("pp-token commit rejected: {error}"))?;
                    let id = bus
                        .arenas
                        .pp_tokens
                        .alloc(
                            PpTokenRecord {
                                kind: token.kind,
                                span,
                                spelling: define.value[token.start..token.end].to_vec(),
                            },
                            &limits,
                        )
                        .map_err(|error| format!("pp-token commit rejected: {error}"))?;
                    replacement.push(id);
                }
                let record = macro_flags::define_record(define.name.clone(), replacement);
                bus.ensure_total_records(1)
                    .map_err(|error| format!("macro commit rejected: {error}"))?;
                bus.arenas
                    .macros
                    .alloc(record, &limits)
                    .map_err(|error| format!("macro commit rejected: {error}"))?;
            }
            macro_flags::MacroFlagSeed::Undef(undef) => {
                let record = macro_flags::undef_record(undef.name.clone());
                bus.ensure_total_records(1)
                    .map_err(|error| format!("macro commit rejected: {error}"))?;
                bus.arenas
                    .macros
                    .alloc(record, &limits)
                    .map_err(|error| format!("macro commit rejected: {error}"))?;
            }
        }
    }
    Ok(())
}

/// Run one PP step and require a `Records` result.
fn step_records(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
    trace: &mut Trace,
    kind: TaskKind,
    owner: ChipId,
    refs: Vec<RecordRef>,
) -> Result<Vec<RecordRef>, String> {
    match step(bus, workers, trace, kind, owner, Payload::from_refs(refs))? {
        ResultValue::Records(refs) => Ok(refs),
        other => Err(format!("expected pp-token refs, got {other:?}")),
    }
}

/// Run PP09 over one contiguous text run and return the expanded refs.
fn expand_text_run(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
    trace: &mut Trace,
    refs: Vec<RecordRef>,
) -> Result<Vec<RecordRef>, String> {
    if refs.is_empty() {
        return Ok(Vec::new());
    }
    step_records(
        bus,
        workers,
        trace,
        TaskKind::PREPROCESS_MACRO_INVOKE,
        PP09_CHIP,
        refs,
    )
}

/// A loaded header source plus its single PP01→PP04 scan.
#[derive(Clone)]
struct HeaderUnit {
    source: cc_silicon_compiler::ids::SourceId,
}

/// Per-TU bounded include cache and occurrence counter.
struct IncludeState {
    by_path: std::collections::BTreeMap<std::path::PathBuf, (Vec<u8>, HeaderUnit)>,
    occurrences: usize,
}

impl IncludeState {
    fn new() -> Self {
        Self {
            by_path: std::collections::BTreeMap::new(),
            occurrences: 0,
        }
    }
}

/// Process a header token stream under the bounded B3-03b policy.
///
/// Leading `#define`/`#undef` lines are applied with PP06/PP08; text runs are
/// expanded with PP09. Nested includes, conditionals/guards, `#line`,
/// `#pragma`, `#error`, and unknown directives are rejected.
fn process_header_tokens(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
    trace: &mut Trace,
    refs: Vec<RecordRef>,
) -> Result<Vec<RecordRef>, String> {
    let stream = pp_driver::group_lines(bus, &refs)?;
    let mut expanded_refs: Vec<RecordRef> = Vec::new();
    let mut index = 0usize;
    while index < stream.lines.len() {
        let line = &stream.lines[index];
        if !pp_driver::line_is_directive(bus, line) {
            let start = index;
            while index < stream.lines.len()
                && !pp_driver::line_is_directive(bus, &stream.lines[index])
            {
                index += 1;
            }
            let mut run = Vec::new();
            for text_line in &stream.lines[start..index] {
                run.extend(text_line.refs());
            }
            let expanded = expand_text_run(bus, workers, trace, run)?;
            expanded_refs.extend(expanded);
            continue;
        }
        if pp_driver::line_has_continuation(bus, line) {
            return Err(format!(
                "unsupported include: header directive on raw line {} spans multiple physical lines",
                line.line + 1
            ));
        }
        match pp_driver::directive_name(bus, line).as_deref() {
            Some(name) if pp_driver::is_define(name) => {
                step(
                    bus,
                    workers,
                    trace,
                    TaskKind::PREPROCESS_MACRO_DEFINE,
                    PP06_CHIP,
                    Payload::from_refs(line.directive_body_refs()),
                )?;
            }
            Some(name) if pp_driver::is_undef(name) => {
                step(
                    bus,
                    workers,
                    trace,
                    TaskKind::PREPROCESS_MACRO_UNDEF,
                    PP08_CHIP,
                    Payload::from_refs(line.directive_body_refs()),
                )?;
            }
            Some(name) if pp_driver::is_include(name) || pp_driver::is_include_next(name) => {
                return Err(
                    "unsupported include: nested includes are outside the B3-03b single-level policy"
                        .to_string(),
                );
            }
            Some(name) if pp_driver::is_conditional_name(name) => {
                return Err(
                    "unsupported include: header conditionals/include guards are outside the B3-03b policy"
                        .to_string(),
                );
            }
            Some(name)
                if name == b"line"
                    || name == b"pragma"
                    || name == b"error"
                    || name == b"warning" =>
            {
                return Err(format!(
                    "unsupported include: header directive `{}` is outside the B3-03b policy",
                    String::from_utf8_lossy(name)
                ));
            }
            Some(name) => {
                return Err(format!(
                    "unsupported include: unknown header directive `{}`",
                    String::from_utf8_lossy(name)
                ));
            }
            None if line.tokens.len() == 1 => {
                // Null directive: drop it.
            }
            None => {
                return Err("unsupported include: malformed header directive line".to_string());
            }
        }
        index += 1;
    }
    Ok(expanded_refs)
}

/// Load, resolve, and splice one quoted `#include "..."` line.
fn expand_include_line(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
    trace: &mut Trace,
    input: &str,
    line: &pp_driver::PpLine,
    eof: cc_silicon_compiler::ids::PpTokenId,
    state: &mut IncludeState,
) -> Result<Vec<RecordRef>, String> {
    state.occurrences += 1;
    if state.occurrences > MAX_INCLUDES {
        return Err(format!(
            "unsupported include: more than {MAX_INCLUDES} includes in one translation unit"
        ));
    }
    let request = pp_include::parse_include_line(bus, &line.tokens)?;
    let path =
        pp_include::resolve_header_path(input, &request.name, bus.limits().max_source_bytes)?;
    let unit = if let Some((known_name, unit)) = state.by_path.get(&path) {
        if known_name != &request.name {
            return Err(format!(
                "unsupported include: `{}` was already loaded under a different spelling",
                path.display()
            ));
        }
        unit.clone()
    } else {
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("cannot read include `{}`: {error}", path.display()))?;
        if bytes.len() as u64 > bus.limits().max_source_bytes {
            return Err(format!(
                "include `{}` exceeds the configured source-byte limit",
                path.display()
            ));
        }
        let name = bus
            .intern_name(&request.name)
            .map_err(|error| format!("name table exhausted: {error}"))?;
        let source = bus
            .alloc_source(name, bytes)
            .map_err(|error| format!("include source rejected: {error}"))?;
        let _ = scan_source(bus, workers, trace, source)?;
        let unit = HeaderUnit { source };
        state
            .by_path
            .insert(path.clone(), (request.name.clone(), unit.clone()));
        unit
    };
    // PP17 must resolve the main-line header token to the loaded source.
    match step(
        bus,
        workers,
        trace,
        TaskKind::PREPROCESS_INCLUDE_RESOLVE,
        PP17_CHIP,
        Payload::from_refs(vec![RecordRef::PpToken(request.header)]),
    )? {
        ResultValue::Record(RecordRef::Source(id)) if id == unit.source => {}
        other => {
            return Err(format!(
                "include resolution did not name the loaded header: {other:?}"
            ));
        }
    }
    // Per-line PP18 splice: include-line refs plus the outer EOF and the
    // resolved header source. The header's own EOF is excluded by PP18.
    let mut payload = line.refs();
    payload.push(RecordRef::PpToken(eof));
    payload.push(RecordRef::Source(unit.source));
    let mut stitched = step_records(
        bus,
        workers,
        trace,
        TaskKind::PREPROCESS_INCLUDE_ENTER,
        PP18_CHIP,
        payload,
    )?;
    match stitched.last() {
        Some(RecordRef::PpToken(id))
            if bus
                .arenas
                .pp_tokens
                .get(*id)
                .is_ok_and(|body| body.kind == cc_silicon_compiler::bus::PpTokenKind::Eof) =>
        {
            stitched.pop();
        }
        _ => {
            return Err("include splice did not preserve the outer EOF".to_string());
        }
    }
    process_header_tokens(bus, workers, trace, stitched)
}

/// Run the frozen PP01→PP04 scan for one committed source.
fn scan_source(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
    trace: &mut Trace,
    source: cc_silicon_compiler::ids::SourceId,
) -> Result<Vec<RecordRef>, String> {
    let artifact = |value: ResultValue| match value {
        ResultValue::Record(RecordRef::Artifact(id)) => Ok(id),
        other => Err(format!("expected artifact ref, got {other:?}")),
    };
    let normalized = artifact(step(
        bus,
        workers,
        trace,
        TaskKind::PREPROCESS_NORMALIZE,
        PP01_CHIP,
        Payload::from_refs(vec![RecordRef::Source(source)]),
    )?)?;
    let spliced = artifact(step(
        bus,
        workers,
        trace,
        TaskKind::PREPROCESS_SPLICE,
        PP_SPLICE_CHIP,
        Payload::from_refs(vec![RecordRef::Artifact(normalized)]),
    )?)?;
    let comment_free = artifact(step(
        bus,
        workers,
        trace,
        TaskKind::PREPROCESS_COMMENT,
        PP_COMMENT_CHIP,
        Payload::from_refs(vec![RecordRef::Artifact(spliced)]),
    )?)?;
    step_records(
        bus,
        workers,
        trace,
        TaskKind::PREPROCESS_SCAN,
        PP_SCAN_CHIP,
        vec![RecordRef::Artifact(comment_free)],
    )
}

/// Shared preprocessing prefix: source bytes through PP01→PP04, then the
/// bounded B3-03a line walk.
///
/// Leading source `#define`/`#undef` lines are applied with PP06/PP08 before
/// conditional evaluation; contiguous text runs are expanded with PP09 at
/// their stream position. At the first conditional line, PP19 runs once over
/// the remaining stream; active non-conditional directives are then handled
/// by PP05 or rejected (active source macro mutation after the first
/// conditional). Every consumed directive line is removed and exactly one
/// EOF is preserved. `-D`/`-U` seeds commit in flag order before the first
/// PP step, so conditionals and expansion observe them.
fn pp_prefix(
    input: &str,
    seeds: &[macro_flags::MacroFlagSeed],
) -> Result<(CompilerBus, WorkerRegistry, Trace, Vec<RecordRef>), String> {
    let bytes = std::fs::read(input).map_err(|error| format!("cannot read {input}: {error}"))?;
    let mut bus = CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ));
    install(&mut bus);
    let workers = build_workers()?;
    let mut trace = Trace::new();
    // Source import (host file access; the name is the file stem).
    let stem = std::path::Path::new(input)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("main.c");
    let name = bus
        .intern_name(stem.as_bytes())
        .map_err(|error| format!("name table exhausted: {error}"))?;
    let source = bus
        .alloc_source(name, bytes)
        .map_err(|error| format!("source rejected: {error}"))?;
    // H04 `-D`/`-U` seeds commit before the first PP step (flag order,
    // last-wins by MacroId).
    commit_macro_flags(&mut bus, seeds)?;
    // Preprocess from real source bytes (no seeded fixtures).
    let pp_tokens = scan_source(&mut bus, &workers, &mut trace, source)?;
    let stream = pp_driver::group_lines(&bus, &pp_tokens)?;
    let mut final_refs: Vec<RecordRef> = Vec::new();
    let mut include_state = IncludeState::new();
    let mut index = 0usize;
    let mut conditional_at: Option<usize> = None;
    while index < stream.lines.len() {
        let line = &stream.lines[index];
        if !pp_driver::line_is_directive(&bus, line) {
            let start = index;
            while index < stream.lines.len()
                && !pp_driver::line_is_directive(&bus, &stream.lines[index])
            {
                index += 1;
            }
            let mut refs = Vec::new();
            for text_line in &stream.lines[start..index] {
                refs.extend(text_line.refs());
            }
            let expanded = expand_text_run(&mut bus, &workers, &mut trace, refs)?;
            final_refs.extend(expanded);
            continue;
        }
        if pp_driver::line_has_continuation(&bus, line) {
            return Err(format!(
                "unsupported preprocessing: directive on raw line {} spans multiple physical lines; logical-line grouping is not frozen",
                line.line + 1
            ));
        }
        let name = pp_driver::directive_name(&bus, line);
        match name.as_deref() {
            Some(name) if pp_driver::is_conditional_name(name) => {
                conditional_at = Some(index);
                break;
            }
            Some(name) if pp_driver::is_define(name) => {
                step(
                    &mut bus,
                    &workers,
                    &mut trace,
                    TaskKind::PREPROCESS_MACRO_DEFINE,
                    PP06_CHIP,
                    Payload::from_refs(line.directive_body_refs()),
                )?;
            }
            Some(name) if pp_driver::is_undef(name) => {
                step(
                    &mut bus,
                    &workers,
                    &mut trace,
                    TaskKind::PREPROCESS_MACRO_UNDEF,
                    PP08_CHIP,
                    Payload::from_refs(line.directive_body_refs()),
                )?;
            }
            Some(name) if pp_driver::is_include(name) => {
                let eof = stream
                    .eof
                    .ok_or_else(|| "preprocessing stream has no EOF token".to_string())?;
                let refs = expand_include_line(
                    &mut bus,
                    &workers,
                    &mut trace,
                    input,
                    line,
                    eof,
                    &mut include_state,
                )?;
                final_refs.extend(refs);
            }
            Some(name) if pp_driver::is_include_next(name) => {
                return Err(
                    "unsupported include: `include_next` is outside the B3-03b policy".to_string(),
                );
            }
            Some(_) => {
                step(
                    &mut bus,
                    &workers,
                    &mut trace,
                    TaskKind::PREPROCESS_DIRECTIVE,
                    PP05_CHIP,
                    Payload::from_refs(line.refs()),
                )?;
            }
            None if line.tokens.len() == 1 => {
                // Null directive: drop it.
            }
            None => {
                step(
                    &mut bus,
                    &workers,
                    &mut trace,
                    TaskKind::PREPROCESS_DIRECTIVE,
                    PP05_CHIP,
                    Payload::from_refs(line.refs()),
                )?;
            }
        }
        index += 1;
    }
    if let Some(start) = conditional_at {
        let mut remaining = Vec::new();
        for line in &stream.lines[start..] {
            remaining.extend(line.refs());
        }
        let eof = stream
            .eof
            .ok_or_else(|| "preprocessing stream has no EOF token".to_string())?;
        remaining.push(RecordRef::PpToken(eof));
        let active = step_records(
            &mut bus,
            &workers,
            &mut trace,
            TaskKind::PREPROCESS_CONDITIONAL,
            PP19_CHIP,
            remaining,
        )?;
        let active_stream = pp_driver::group_lines(&bus, &active)?;
        let mut active_index = 0usize;
        while active_index < active_stream.lines.len() {
            let line = &active_stream.lines[active_index];
            if !pp_driver::line_is_directive(&bus, line) {
                let start = active_index;
                while active_index < active_stream.lines.len()
                    && !pp_driver::line_is_directive(&bus, &active_stream.lines[active_index])
                {
                    active_index += 1;
                }
                let mut refs = Vec::new();
                for text_line in &active_stream.lines[start..active_index] {
                    refs.extend(text_line.refs());
                }
                let expanded = expand_text_run(&mut bus, &workers, &mut trace, refs)?;
                final_refs.extend(expanded);
                continue;
            }
            if pp_driver::line_has_continuation(&bus, line) {
                return Err(format!(
                    "unsupported preprocessing: active directive on raw line {} spans multiple physical lines; logical-line grouping is not frozen",
                    line.line + 1
                ));
            }
            match pp_driver::directive_name(&bus, line).as_deref() {
                Some(name) if pp_driver::is_define(name) || pp_driver::is_undef(name) => {
                    return Err(format!(
                        "unsupported preprocessing: source #define/#undef after the first conditional (raw line {}) is not supported; PP19 has already fixed the active stream",
                        line.line + 1
                    ));
                }
                Some(name) if pp_driver::is_conditional_name(name) => {
                    return Err(
                        "internal preprocessing error: conditional directive survived PP19"
                            .to_string(),
                    );
                }
                Some(name) if pp_driver::is_include(name) || pp_driver::is_include_next(name) => {
                    return Err(format!(
                        "unsupported include: `#include` after the first conditional (raw line {}) is outside the B3-03b policy",
                        line.line + 1
                    ));
                }
                Some(_) => {
                    step(
                        &mut bus,
                        &workers,
                        &mut trace,
                        TaskKind::PREPROCESS_DIRECTIVE,
                        PP05_CHIP,
                        Payload::from_refs(line.refs()),
                    )?;
                }
                None if line.tokens.len() == 1 => {
                    // Null directive: drop it.
                }
                None => {
                    step(
                        &mut bus,
                        &workers,
                        &mut trace,
                        TaskKind::PREPROCESS_DIRECTIVE,
                        PP05_CHIP,
                        Payload::from_refs(line.refs()),
                    )?;
                }
            }
            active_index += 1;
        }
        let active_eof = active_stream
            .eof
            .ok_or_else(|| "PP19 returned a stream with no EOF token".to_string())?;
        final_refs.push(RecordRef::PpToken(active_eof));
    } else {
        let eof = stream
            .eof
            .ok_or_else(|| "preprocessing stream has no EOF token".to_string())?;
        final_refs.push(RecordRef::PpToken(eof));
    }
    Ok((bus, workers, trace, final_refs))
}

/// Part B `-E` terminal: the shared prefix, then the frozen PP28 worker
/// over the final pp-token stream. Returns the emitted preprocessed bytes;
/// persistence stays with the caller (host owns artifact persistence).
fn run_emit(input: &str, seeds: &[macro_flags::MacroFlagSeed]) -> Result<Vec<u8>, String> {
    let (mut bus, workers, mut trace, final_tokens) = pp_prefix(input, seeds)?;
    emit_helper::emit_preprocessed_bytes(&mut bus, &workers, &mut trace, final_tokens)
}

fn run(
    input: &str,
    seeds: &[macro_flags::MacroFlagSeed],
    asm_output: Option<&str>,
) -> Result<Evidence, String> {
    let (mut bus, workers, mut trace, expanded_tokens) = pp_prefix(input, seeds)?;
    // Lex over every expanded pp-token (no fixed positions).
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::LEX_INTERN,
        LX_INTERN_CHIP,
        Payload::from_refs(expanded_tokens.clone()),
    )?;
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::LEX_CLASSIFY,
        LX_CLASSIFY_CHIP,
        Payload::from_refs(expanded_tokens),
    )?;
    let mut tokens: Vec<TokenId> = bus.arenas.tokens.iter().map(|(id, _)| id).collect();
    tokens.sort_by_key(|id| id.index());
    // Decode every integer token (no fixed positions).
    let mut integer_tokens = Vec::new();
    for (id, body) in bus.arenas.tokens.iter() {
        if body.kind == TokenKind::Integer {
            integer_tokens.push((id, body.pp_token));
        }
    }
    integer_tokens.sort_by_key(|(id, _)| id.index());
    for (id, pp_token) in integer_tokens {
        step(
            &mut bus,
            &workers,
            &mut trace,
            TaskKind::LEX_DECODE_LITERAL,
            LX_DECODE_CHIP,
            Payload::from_refs(vec![RecordRef::Token(id), RecordRef::PpToken(pp_token)]),
        )?;
    }
    // Parse over every committed token.
    let token_refs: Vec<RecordRef> = tokens.iter().map(|id| RecordRef::Token(*id)).collect();
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(token_refs),
    )?;
    let tu = nodes_of(&bus, NodeKind::TranslationUnit);
    if tu.len() != 1 {
        return Err("expected exactly one translation unit".to_string());
    }
    // Token-AST check right after parse (stage 2, like the slice chain).
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::VERIFICATION_TOKEN_AST_INVARIANT,
        VF05_CHIP,
        Payload::from_refs(vec![RecordRef::Node(tu[0])]),
    )?;
    // Types and symbols by tree walk (no fixed node IDs). The `/49` locals
    // shape has one function declarator plus one block-scope declarator.
    let declarations = nodes_of(&bus, NodeKind::Declaration);
    let locals = declarations.len() == 1;
    let declarators = nodes_of(&bus, NodeKind::Declarator);
    let expected_declarators = if locals { 2 } else { 1 };
    if declarators.len() != expected_declarators {
        return Err(format!(
            "expected {expected_declarators} declarator(s), got {}",
            declarators.len()
        ));
    }
    let int = match step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::SYMBOL_INT_TYPE,
        TY_TYPE_CHIP,
        Payload::empty(),
    )? {
        ResultValue::Record(RecordRef::Type(id)) => id,
        other => return Err(format!("expected type ref, got {other:?}")),
    };
    let func = match step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::SYMBOL_FUNC_TYPE,
        TY_TYPE_CHIP,
        Payload::from_refs(vec![RecordRef::Type(int)]),
    )? {
        ResultValue::Record(RecordRef::Type(id)) => id,
        other => return Err(format!("expected type ref, got {other:?}")),
    };
    let file_scope = match step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::SYMBOL_SCOPE_ENTER,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Node(tu[0])]),
    )? {
        ResultValue::Record(RecordRef::Scope(id)) => id,
        other => return Err(format!("expected scope ref, got {other:?}")),
    };
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        Payload::from_refs(vec![
            RecordRef::Node(declarators[0]),
            RecordRef::Type(func),
            RecordRef::Scope(file_scope),
        ]),
    )?;
    let _block_scope = if locals {
        let compounds = nodes_of(&bus, NodeKind::Compound);
        if compounds.len() != 1 {
            return Err("expected exactly one function body".to_string());
        }
        let block_scope = match step(
            &mut bus,
            &workers,
            &mut trace,
            TaskKind::SYMBOL_SCOPE_ENTER,
            TY_SCOPE_CHIP,
            Payload::from_refs(vec![RecordRef::Node(compounds[0])]),
        )? {
            ResultValue::Record(RecordRef::Scope(id)) => id,
            other => return Err(format!("expected block scope ref, got {other:?}")),
        };
        let declaration = bus
            .arenas
            .nodes
            .get(declarations[0])
            .map_err(|_| "local declaration vanished".to_string())?;
        let local_declarator = *declaration
            .children
            .get(1)
            .ok_or_else(|| "local declaration has no declarator".to_string())?;
        step(
            &mut bus,
            &workers,
            &mut trace,
            TaskKind::SYMBOL_DECLARE,
            TY_SYMBOL_CHIP,
            Payload::from_refs(vec![
                RecordRef::Node(local_declarator),
                RecordRef::Type(int),
                RecordRef::Scope(block_scope),
            ]),
        )?;
        Some(block_scope)
    } else {
        None
    };
    if locals {
        // Initializer literal fact and its folded constant.
        let declaration = bus
            .arenas
            .nodes
            .get(declarations[0])
            .map_err(|_| "local declaration vanished".to_string())?;
        let initializer = *declaration
            .children
            .get(2)
            .ok_or_else(|| "local declaration has no initializer".to_string())?;
        run_literal(&mut bus, &workers, &mut trace, initializer, true)?;

        // The return expression is checked in post-order so every child fact
        // exists before its parent binary/unary task runs.
        let returns = nodes_of(&bus, NodeKind::Return);
        if returns.len() != 1 {
            return Err("expected exactly one return statement".to_string());
        }
        let ret = returns[0];
        let root = bus
            .arenas
            .nodes
            .get(ret)
            .map_err(|_| "return node vanished".to_string())?
            .children
            .first()
            .copied()
            .ok_or_else(|| "return statement has no expression".to_string())?;
        let compounds = nodes_of(&bus, NodeKind::Compound);
        if compounds.len() != 1 {
            return Err("expected exactly one function body".to_string());
        }
        let assignments: Vec<NodeId> = bus
            .arenas
            .nodes
            .get(compounds[0])
            .map_err(|_| "function body vanished".to_string())?
            .children[1..]
            .split_last()
            .map(|(_, rest)| rest.to_vec())
            .unwrap_or_default();
        for statement in assignments {
            let statement_body = bus
                .arenas
                .nodes
                .get(statement)
                .map_err(|_| "assignment statement vanished".to_string())?;
            if statement_body.kind != NodeKind::Assignment || statement_body.children.len() != 2 {
                return Err("unsupported statement in locals shape".to_string());
            }
            let mut order = Vec::new();
            expression_postorder(&bus, statement_body.children[1], &mut order)?;
            for node in order {
                run_expression_node(&mut bus, &workers, &mut trace, node)?;
            }
            step(
                &mut bus,
                &workers,
                &mut trace,
                TaskKind::SEMANTIC_ASSIGNMENT_EXPR,
                SE_ASSIGN_CHIP,
                Payload::from_refs(vec![RecordRef::Node(statement)]),
            )?;
        }
        let mut order = Vec::new();
        expression_postorder(&bus, root, &mut order)?;
        for node in order {
            run_expression_node(&mut bus, &workers, &mut trace, node)?;
        }
        step(
            &mut bus,
            &workers,
            &mut trace,
            TaskKind::SEMANTIC_RETURN_STMT,
            SE_RET_CHIP,
            Payload::from_refs(vec![RecordRef::Node(ret)]),
        )?;
    } else {
        // Canonical shaped path: literal facts, then the additive fold, then
        // the return fact.
        for node in nodes_of(&bus, NodeKind::IntLiteral) {
            run_literal(&mut bus, &workers, &mut trace, node, false)?;
        }
        for node in nodes_of(&bus, NodeKind::BinaryAdd) {
            step(
                &mut bus,
                &workers,
                &mut trace,
                TaskKind::SEMANTIC_BINARY_EXPR,
                SE_BIN_CHIP,
                Payload::from_refs(vec![RecordRef::Node(node)]),
            )?;
        }
        for node in nodes_of(&bus, NodeKind::Return) {
            step(
                &mut bus,
                &workers,
                &mut trace,
                TaskKind::SEMANTIC_RETURN_STMT,
                SE_RET_CHIP,
                Payload::from_refs(vec![RecordRef::Node(node)]),
            )?;
        }
    }
    // SE29 function-definition fact: exactly one M1 definition. The
    // committed SemRecord is the signature fact IR lowering consumes below;
    // the node is found once and threaded onward.
    let funcdefs = nodes_of(&bus, NodeKind::FunctionDefinition);
    if funcdefs.len() != 1 {
        return Err("expected exactly one function definition".to_string());
    }
    let funcdef = funcdefs[0];
    match step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::SEMANTIC_FUNCTION_DEF,
        SE_FUNC_CHIP,
        Payload::from_refs(vec![RecordRef::Node(funcdef)]),
    )? {
        ResultValue::Record(RecordRef::Sem(_)) => {}
        other => return Err(format!("expected sem ref, got {other:?}")),
    }
    // Typed-invariant gate, lowering (consuming the SE29 fact), symbolic
    // model, store check.
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::VERIFICATION_TYPED_INVARIANT,
        VF06_CHIP,
        Payload::from_refs(vec![RecordRef::Node(tu[0])]),
    )?;
    let function = match step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(funcdef)]),
    )? {
        ResultValue::Record(RecordRef::Function(id)) => id,
        other => return Err(format!("expected function ref, got {other:?}")),
    };
    let (modeled_value, modeled_negative) = match step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::VERIFICATION_IR_INTERPRET,
        VF12_CHIP,
        Payload::from_refs(vec![RecordRef::Function(function)]),
    )? {
        ResultValue::Record(RecordRef::Const(id)) => {
            let body = bus
                .arenas
                .consts
                .get(id)
                .map_err(|_| "modeled const vanished".to_string())?;
            let integer = body
                .value
                .as_integer()
                .ok_or_else(|| "modeled const is not an integer".to_string())?;
            (integer.magnitude.clone(), integer.negative)
        }
        ResultValue::Integer(integer) => (integer.magnitude, integer.negative),
        other => return Err(format!("expected modeled integer, got {other:?}")),
    };
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::VERIFICATION_STORE_INVARIANT,
        VF01_CHIP,
        Payload::empty(),
    )?;
    if let Some(path) = asm_output {
        let assembly = target_m1::emit_m1_assembly(&bus)?;
        std::fs::write(path, assembly)
            .map_err(|error| format!("cannot write assembly {path}: {error}"))?;
    }
    let snapshot = Snapshot::capture(&bus);
    Ok(Evidence {
        snapshot,
        trace,
        value: modeled_value,
        negative: modeled_negative,
        input: input.to_string(),
    })
}

/// Evidence rendering (kept separate so flags select sections).
struct Evidence {
    snapshot: Snapshot,
    trace: Trace,
    value: Vec<u8>,
    negative: bool,
    input: String,
}

fn main() {
    // Collect argv explicitly: `std::env::args()` panics on a non-UTF-8
    // element. Reject the element by position so invalid bytes are never
    // copied into the diagnostic.
    let mut args = Vec::new();
    for (position, argument) in std::env::args_os().enumerate() {
        match argument.into_string() {
            Ok(argument) => {
                if position > 0 {
                    args.push(argument);
                }
            }
            Err(_) => {
                eprintln!(
                    "candidate: argv[{position}] is not valid UTF-8; arguments must be valid UTF-8"
                );
                std::process::exit(2);
            }
        }
    }
    let invocation = match parse_args(&args) {
        Ok(invocation) => invocation,
        Err(message) => {
            if message == "help" {
                println!("{}", usage());
                std::process::exit(0);
            }
            eprintln!("candidate: {message}\n{}", usage());
            std::process::exit(2);
        }
    };
    if invocation.emit {
        match run_emit(&invocation.input, &invocation.macro_flags) {
            Ok(bytes) => match &invocation.output {
                Some(path) => {
                    if let Err(error) = std::fs::write(path, &bytes) {
                        eprintln!("candidate: cannot write {path}: {error}");
                        std::process::exit(2);
                    }
                }
                None => {
                    use std::io::Write as _;
                    if let Err(error) = std::io::stdout().lock().write_all(&bytes) {
                        eprintln!("candidate: cannot write stdout: {error}");
                        std::process::exit(2);
                    }
                }
            },
            Err(message) => {
                eprintln!("candidate: {message}");
                std::process::exit(1);
            }
        }
        return;
    }
    match run(
        &invocation.input,
        &invocation.macro_flags,
        invocation.asm_output.as_deref(),
    ) {
        Ok(evidence) => {
            let mut report = format!("candidate evidence ({CONTRACT_VERSION})\n");
            report.push_str(&format!("input: {}\n", evidence.input));
            let show_all = !invocation.snapshot && !invocation.trace && !invocation.interpret;
            if invocation.snapshot || show_all {
                report.push_str(&format!("snapshot: {}\n", hex32(&evidence.snapshot.hash())));
            }
            if invocation.trace || show_all {
                report.push_str(&format!("trace: {}\n", hex32(&evidence.trace.hash())));
            }
            if invocation.interpret || show_all {
                let bytes: String = evidence
                    .value
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect();
                report.push_str(&format!(
                    "interpret: value={bytes} negative={}\n",
                    evidence.negative
                ));
            }
            match &invocation.output {
                Some(path) => {
                    if let Err(error) = std::fs::write(path, &report) {
                        eprintln!("candidate: cannot write {path}: {error}");
                        std::process::exit(2);
                    }
                }
                None => print!("{report}"),
            }
        }
        Err(message) => {
            eprintln!("candidate: {message}");
            std::process::exit(1);
        }
    }
}
