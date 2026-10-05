// ============================================================================
// bin/candidate.rs — H04 candidate command-line driver (Part A).
//
// Host-side orchestration only: it reads source bytes, drives the frozen
// M1 worker pipeline from real tasks (PP01→PP02→PP03→PP04→LX→PA→TY→SE→
// VF06/VF05→IR→VF12→VF01), and prints Part A evidence (snapshot hash,
// trace hash, modeled constant). It performs no compilation itself, never
// shells out to another C compiler, and refuses target emission (`-S`,
// `-c`) while the target is unverified (fail-closed, Part B). Flag
// spellings are illustrative until the H04 driver contract freezes;
// `-E`, `-I`/`-D`/`-U`, and multi-source compilations are explicit
// deferred errors, never silent fallbacks.
//
// Exit codes: 0 = evidence produced; 1 = candidate diagnostic (the input
// is rejected with a structured message, no evidence); 2 = driver error
// (usage, unsupported flag, or refused emission).
// ============================================================================

use cc_silicon_compiler::bus::{CompilerBus, CompilerPins, NodeKind, TokenKind};
use cc_silicon_compiler::chips::{
    handler_for, FoldChip, IrFunctionChip, LxClassifyChip, LxDecodeLiteralChip, LxInternChip,
    PaTuChip, PpCommentChip, PpConditionalChip, PpDiagnosticChip, PpDirectiveChip, PpNormalizeChip,
    PpScanChip, PpSpliceChip, SeBinChip, SeLitChip, SeRetChip, TyConvChip, TyScopeChip,
    TySymbolChip, TyTypeChip, Vf01Chip, Vf05Chip, Vf06Chip, Vf12Chip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::codec::hex32;
use cc_silicon_compiler::contract::CONTRACT_VERSION;
use cc_silicon_compiler::ids::{ChipId, NodeId, RecordRef, TokenId};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    StoreSchema, IR_FUNCTION_CHIP, LX_CLASSIFY_CHIP, LX_DECODE_CHIP, LX_INTERN_CHIP, PA_TU_CHIP,
    PP01_CHIP, PP05_CHIP, PP19_CHIP, PP26_CHIP, PP_COMMENT_CHIP, PP_SCAN_CHIP, PP_SPLICE_CHIP,
    SE_BIN_CHIP, SE_LIT_CHIP, SE_RET_CHIP, TY_CONV_CHIP, TY_SCOPE_CHIP, TY_SYMBOL_CHIP,
    TY_TYPE_CHIP, VF01_CHIP, VF05_CHIP, VF06_CHIP, VF12_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{Snapshot, Trace};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{
    Payload, ResultValue, TaskDraft, TaskKind, TaskKindRegistry, TaskState,
};

/// Maximum drain ticks per pipeline step (M1 needs a handful for joins).
const DRAIN_BUDGET: u32 = 16;

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
}

fn usage() -> &'static str {
    "usage: candidate [-std=c11] [-O0] [-o <path>] [--emit-ir-snapshot] [--emit-trace] [--interpret-ir] file.c\n\
     Part A evidence driver (M1): snapshot/trace emission and IR interpretation.\n\
     -S and -c are refused while the target is unverified (fail-closed, Part B).\n\
     -E, -I/-D/-U, and multi-source compilations are deferred (explicit error)."
}

fn parse_args(args: &[String]) -> Result<Invocation, String> {
    let mut invocation = Invocation {
        input: String::new(),
        output: None,
        snapshot: false,
        trace: false,
        interpret: false,
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
            return Err(
                "-E refused in Part A: preprocessed emission (PP28) is deferred".to_string(),
            );
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
        } else if arg == "-I"
            || arg.starts_with("-I")
            || arg == "-D"
            || arg.starts_with("-D")
            || arg == "-U"
            || arg.starts_with("-U")
        {
            return Err(format!(
                "{arg} deferred: macro/include support is not in Part A"
            ));
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
    if !invocation.snapshot && !invocation.trace && !invocation.interpret {
        invocation.snapshot = true;
        invocation.trace = true;
        invocation.interpret = true;
    }
    Ok(invocation)
}

fn install(bus: &mut CompilerBus) {
    bus.kinds = TaskKindRegistry::pp_macro_slice();
    bus.schema = StoreSchema::pp_macro_slice();
    bus.registrations
        .register(FoldChip.manifest(), &bus.schema, &bus.kinds)
        .expect("fold manifest registers");
    for chip in [
        &PpNormalizeChip as &dyn Worker,
        &PpSpliceChip,
        &PpCommentChip,
        &PpScanChip,
        &PpConditionalChip,
        &PpDirectiveChip,
        &PpDiagnosticChip,
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
        (TaskKind::PREPROCESS_CONDITIONAL, PP19_CHIP, 1),
        (TaskKind::PREPROCESS_DIRECTIVE, PP05_CHIP, 1),
        (TaskKind::PREPROCESS_DIAGNOSTIC, PP26_CHIP, 1),
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
        let report = RoutingShell
            .clock_tick_with(&CompilerPins::default(), bus, handler_for(workers))
            .map_err(|error| format!("tick failed: {error}"))?;
        trace.record(bus, &report);
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

fn run(input: &str) -> Result<Evidence, String> {
    let bytes = std::fs::read(input).map_err(|error| format!("cannot read {input}: {error}"))?;
    let mut bus = CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ));
    install(&mut bus);
    let workers = {
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
            .register(PpConditionalChip)
            .map_err(|error| format!("worker registration failed: {error}"))?;
        workers
            .register(PpDirectiveChip)
            .map_err(|error| format!("worker registration failed: {error}"))?;
        workers
            .register(PpDiagnosticChip)
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
        workers
    };
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
    // Preprocess from real source bytes (no seeded fixtures).
    let artifact = |value: ResultValue| match value {
        ResultValue::Record(RecordRef::Artifact(id)) => Ok(id),
        other => Err(format!("expected artifact ref, got {other:?}")),
    };
    let normalized = artifact(step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::PREPROCESS_NORMALIZE,
        PP01_CHIP,
        Payload::from_refs(vec![RecordRef::Source(source)]),
    )?)?;
    let spliced = artifact(step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::PREPROCESS_SPLICE,
        PP_SPLICE_CHIP,
        Payload::from_refs(vec![RecordRef::Artifact(normalized)]),
    )?)?;
    let comment_free = artifact(step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::PREPROCESS_COMMENT,
        PP_COMMENT_CHIP,
        Payload::from_refs(vec![RecordRef::Artifact(spliced)]),
    )?)?;
    let scanned = step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::PREPROCESS_SCAN,
        PP_SCAN_CHIP,
        Payload::from_refs(vec![RecordRef::Artifact(comment_free)]),
    )?;
    let pp_tokens = match scanned {
        ResultValue::Records(refs) => refs,
        other => return Err(format!("expected pp-token refs, got {other:?}")),
    };
    // Conditional inclusion over the scanned stream (M1 sources carry no
    // conditionals and pass through; inactive lines are dropped).
    let active_tokens = match step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::PREPROCESS_CONDITIONAL,
        PP19_CHIP,
        Payload::from_refs(pp_tokens),
    )? {
        ResultValue::Records(refs) => refs,
        other => return Err(format!("expected active refs, got {other:?}")),
    };
    // Directive dispatch over every active pp-token (M1 sources carry no
    // directives and acknowledge here; `#error` fails with its message).
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::PREPROCESS_DIRECTIVE,
        PP05_CHIP,
        Payload::from_refs(active_tokens.clone()),
    )?;
    // Lex over every active pp-token (no fixed positions).
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::LEX_INTERN,
        LX_INTERN_CHIP,
        Payload::from_refs(active_tokens.clone()),
    )?;
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::LEX_CLASSIFY,
        LX_CLASSIFY_CHIP,
        Payload::from_refs(active_tokens),
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
    // Types and symbols by tree walk (no fixed node IDs).
    let declarators = nodes_of(&bus, NodeKind::Declarator);
    if declarators.len() != 1 {
        return Err("expected exactly one declarator".to_string());
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
    // Semantic checks over every literal/return/binary node found.
    for node in nodes_of(&bus, NodeKind::IntLiteral) {
        step(
            &mut bus,
            &workers,
            &mut trace,
            TaskKind::SEMANTIC_LITERAL_EXPR,
            SE_LIT_CHIP,
            Payload::from_refs(vec![RecordRef::Node(node)]),
        )?;
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
    // Typed-invariant gate, lowering, symbolic model, store check.
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::VERIFICATION_TYPED_INVARIANT,
        VF06_CHIP,
        Payload::from_refs(vec![RecordRef::Node(tu[0])]),
    )?;
    let funcdefs = nodes_of(&bus, NodeKind::FunctionDefinition);
    if funcdefs.len() != 1 {
        return Err("expected exactly one function definition".to_string());
    }
    let function = match step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::IR_FUNCTION,
        IR_FUNCTION_CHIP,
        Payload::from_refs(vec![RecordRef::Node(funcdefs[0])]),
    )? {
        ResultValue::Record(RecordRef::Function(id)) => id,
        other => return Err(format!("expected function ref, got {other:?}")),
    };
    let modeled = match step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::VERIFICATION_IR_INTERPRET,
        VF12_CHIP,
        Payload::from_refs(vec![RecordRef::Function(function)]),
    )? {
        ResultValue::Record(RecordRef::Const(id)) => id,
        other => return Err(format!("expected const ref, got {other:?}")),
    };
    step(
        &mut bus,
        &workers,
        &mut trace,
        TaskKind::VERIFICATION_STORE_INVARIANT,
        VF01_CHIP,
        Payload::empty(),
    )?;
    let snapshot = Snapshot::capture(&bus);
    let modeled_body = bus
        .arenas
        .consts
        .get(modeled)
        .map_err(|_| "modeled const vanished".to_string())?;
    Ok(Evidence {
        snapshot,
        trace,
        value: modeled_body.value.clone(),
        negative: modeled_body.negative,
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
    let args: Vec<String> = std::env::args().skip(1).collect();
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
    match run(&invocation.input) {
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
