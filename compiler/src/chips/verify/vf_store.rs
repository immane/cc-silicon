// ============================================================================
// chips/verify/vf_store.rs — T13 VF01 store-invariant verifier
// (Wave 2 slice 10, `/19`)
//
// Reads the whole committed bus snapshot and checks the M1 store contract:
// every task-payload and result reference resolves to an allocated record
// (ID ownership, no dangling/cross-arena links), every span names a
// committed source with `start <= end <= source length` (and a committed
// expansion when present), and the still-reserved language stores
// (`layouts`, `inits`, `vregs`) hold no records. Completes `Ack`; any gap
// fails loudly. Two M1 boundaries are explicit: `HostRequest` references
// have no frozen schema yet and fail as out-of-scope (M1 produces none),
// and liveness past the allocated bound (tombstones) stays deferred to
// the lifetime verifier.
// ============================================================================

use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ContinuationId, ExpansionId, RecordRef, ResultId, SourceId, SpanId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, VF01_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Allocated bounds per record family, captured at dispatch (IDs are dense
/// and never reused, so `index < allocated` is the M1 ownership check).
#[derive(Clone, Copy, Debug)]
pub struct ArenaBounds {
    /// Sources.
    pub sources: u32,
    /// Spans.
    pub spans: u32,
    /// Expansions.
    pub expansions: u32,
    /// PP tokens.
    pub pp_tokens: u32,
    /// C tokens.
    pub tokens: u32,
    /// Scopes.
    pub scopes: u32,
    /// Scope events.
    pub scope_events: u32,
    /// Symbols.
    pub symbols: u32,
    /// Types.
    pub types: u32,
    /// Semantic facts.
    pub sem: u32,
    /// AST nodes.
    pub nodes: u32,
    /// Decoded literals.
    pub literals: u32,
    /// Folded constants.
    pub consts: u32,
    /// Reserved layout descriptors (M1: none).
    pub layouts: u32,
    /// Reserved init plans (M1: none).
    pub inits: u32,
    /// IR functions.
    pub functions: u32,
    /// IR blocks.
    pub blocks: u32,
    /// IR values.
    pub values: u32,
    /// IR instructions.
    pub instructions: u32,
    /// Reserved virtual registers (M1: none).
    pub vregs: u32,
    /// Continuations.
    pub continuations: u32,
    /// Tasks.
    pub tasks: u32,
    /// Results.
    pub results: u32,
    /// Diagnostics.
    pub diagnostics: u32,
    /// Host requests (unreadable in M1: no frozen schema).
    pub host_requests: u32,
    /// Macro definitions (`/23`).
    pub macros: u32,
    /// Artifacts.
    pub artifacts: u32,
    /// Interned names.
    pub intern_names: u32,
}

/// One task's ownership links: payload refs, parent, continuation.
type TaskLinks = (
    TaskId,
    Vec<RecordRef>,
    Option<TaskId>,
    Option<ContinuationId>,
);

/// Narrow projection for the store-invariant check.
#[derive(Clone, Debug)]
pub struct Vf01Input {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Arena ownership bounds.
    pub bounds: ArenaBounds,
    /// Every task: payload refs, parent, continuation.
    pub tasks: Vec<TaskLinks>,
    /// Every committed result value.
    pub results: Vec<(ResultId, ResultValue)>,
    /// Every span with its source link and byte range.
    pub spans: Vec<(SpanId, SourceId, u64, u64, Option<ExpansionId>)>,
    /// Every source with its byte length.
    pub source_lens: Vec<(SourceId, u64)>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_vf01_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<Vf01Input, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("vf01 of unknown task {}", task.index())))?;
    if record.kind != TaskKind::VERIFICATION_STORE_INVARIANT {
        return Err(protocol_fault(format!(
            "vf01 task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if !record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "vf01 task {} payload must be empty (global snapshot check)",
            task.index()
        )));
    }
    let bounds = ArenaBounds {
        sources: bus.arenas.sources.allocated(),
        spans: bus.arenas.spans.allocated(),
        expansions: bus.arenas.expansions.allocated(),
        pp_tokens: bus.arenas.pp_tokens.allocated(),
        tokens: bus.arenas.tokens.allocated(),
        scopes: bus.arenas.scopes.allocated(),
        scope_events: bus.arenas.scope_events.allocated(),
        symbols: bus.arenas.symbols.allocated(),
        types: bus.arenas.types.allocated(),
        sem: bus.arenas.sem.allocated(),
        nodes: bus.arenas.nodes.allocated(),
        literals: bus.arenas.literals.allocated(),
        consts: bus.arenas.consts.allocated(),
        layouts: bus.arenas.layouts.allocated(),
        inits: bus.arenas.inits.allocated(),
        functions: bus.arenas.functions.allocated(),
        blocks: bus.arenas.blocks.allocated(),
        values: bus.arenas.values.allocated(),
        instructions: bus.arenas.instructions.allocated(),
        vregs: bus.arenas.vregs.allocated(),
        continuations: bus.arenas.continuations.allocated(),
        tasks: bus.arenas.tasks.allocated(),
        results: bus.arenas.results.allocated(),
        diagnostics: bus.arenas.diagnostics.allocated(),
        host_requests: bus.arenas.host_requests.allocated(),
        macros: bus.arenas.macros.allocated(),
        artifacts: bus.arenas.artifacts.allocated(),
        intern_names: bus.intern.len(),
    };
    let mut tasks: Vec<TaskLinks> = bus
        .arenas
        .tasks
        .iter()
        .map(|(id, body)| {
            (
                id,
                body.payload.refs.clone(),
                body.parent,
                body.continuation,
            )
        })
        .collect();
    tasks.sort_by_key(|(id, _, _, _)| id.index());
    let mut results: Vec<(ResultId, ResultValue)> = bus
        .arenas
        .results
        .iter()
        .map(|(id, body)| (id, body.value.clone()))
        .collect();
    results.sort_by_key(|(id, _)| id.index());
    let mut spans = Vec::new();
    for (id, body) in bus.arenas.spans.iter() {
        spans.push((id, body.source, body.start, body.end, body.expansion));
    }
    spans.sort_by_key(|(id, _, _, _, _)| id.index());
    let mut source_lens: Vec<(SourceId, u64)> = bus
        .arenas
        .sources
        .iter()
        .map(|(id, body)| (id, body.bytes.len() as u64))
        .collect();
    source_lens.sort_by_key(|(id, _)| id.index());
    Ok(Vf01Input {
        task,
        state: record.state.clone(),
        bounds,
        tasks,
        results,
        spans,
        source_lens,
    })
}

/// Resolve one reference against the M1 ownership bounds.
fn resolve(reference: &RecordRef, bounds: &ArenaBounds) -> Result<(), &'static str> {
    let owned = match *reference {
        RecordRef::Source(id) => id.index() < bounds.sources,
        RecordRef::Span(id) => id.index() < bounds.spans,
        RecordRef::Expansion(id) => id.index() < bounds.expansions,
        RecordRef::PpToken(id) => id.index() < bounds.pp_tokens,
        RecordRef::Token(id) => id.index() < bounds.tokens,
        RecordRef::Name(id) => id.index() < bounds.intern_names,
        RecordRef::Scope(id) => id.index() < bounds.scopes,
        RecordRef::Symbol(id) => id.index() < bounds.symbols,
        RecordRef::Type(id) => id.index() < bounds.types,
        RecordRef::Node(id) => id.index() < bounds.nodes,
        RecordRef::Const(id) => id.index() < bounds.consts,
        RecordRef::Layout(id) => id.index() < bounds.layouts,
        RecordRef::Init(id) => id.index() < bounds.inits,
        RecordRef::Function(id) => id.index() < bounds.functions,
        RecordRef::Block(id) => id.index() < bounds.blocks,
        RecordRef::Value(id) => id.index() < bounds.values,
        RecordRef::Instruction(id) => id.index() < bounds.instructions,
        RecordRef::VReg(id) => id.index() < bounds.vregs,
        RecordRef::Continuation(id) => id.index() < bounds.continuations,
        RecordRef::Task(id) => id.index() < bounds.tasks,
        RecordRef::Result(id) => id.index() < bounds.results,
        RecordRef::Diagnostic(id) => id.index() < bounds.diagnostics,
        RecordRef::HostRequest(_) => return Err("host-request references are out of M1 scope"),
        RecordRef::Macro(id) => id.index() < bounds.macros,
        RecordRef::Artifact(id) => id.index() < bounds.artifacts,
        RecordRef::Literal(id) => id.index() < bounds.literals,
        RecordRef::Sem(id) => id.index() < bounds.sem,
        RecordRef::ScopeEvent(id) => id.index() < bounds.scope_events,
    };
    if owned {
        Ok(())
    } else {
        Err("dangling record reference")
    }
}

/// Build the loud failure for a store-contract gap (never a pass).
fn store_fail(task: TaskId, detail: &str) -> Proposal {
    fail(
        task,
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("vf01: broken store contract ({detail})"),
        ),
    )
}

/// The VF01 store-invariant verifier (M1 scope).
pub struct Vf01Chip;

impl Worker for Vf01Chip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: VF01_CHIP,
            chip_name: "Vf01Chip",
            group: TaskGroup::VERIFICATION,
            task_kinds: vec![TaskKind::VERIFICATION_STORE_INVARIANT],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Tasks, "active.parent"),
                FieldPath::new(StoreId::Tasks, "active.continuation"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "results"),
                FieldPath::new(StoreId::Tasks, "completed"),
                FieldPath::new(StoreId::Sources, "bytes"),
                FieldPath::new(StoreId::Sources, "spans"),
                FieldPath::new(StoreId::Sources, "expansion"),
                FieldPath::new(StoreId::Names, "entries"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Lex, "tokens"),
                FieldPath::new(StoreId::Lex, "literals"),
                FieldPath::new(StoreId::Parse, "nodes"),
                FieldPath::new(StoreId::Types, "records"),
                FieldPath::new(StoreId::Symbols, "symbols"),
                FieldPath::new(StoreId::Symbols, "scopes"),
                FieldPath::new(StoreId::Symbols, "scope_events"),
                FieldPath::new(StoreId::Sem, "records"),
                FieldPath::new(StoreId::Constants, "records"),
                FieldPath::new(StoreId::Ir, "functions"),
                FieldPath::new(StoreId::Ir, "blocks"),
                FieldPath::new(StoreId::Ir, "values"),
                FieldPath::new(StoreId::Ir, "instructions"),
                FieldPath::new(StoreId::Diagnostics, "entries"),
                FieldPath::new(StoreId::Artifacts, "fragments"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c19_vf01.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_vf01_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl Vf01Chip {
    /// Pure store-contract check over the narrow projection (no bus
    /// access, no store writes).
    pub fn compute(&self, input: &Vf01Input) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("vf01 task {} is not running", input.task.index()),
                ),
            )];
        }
        // Reserved language stores hold no records in M1.
        if input.bounds.layouts != 0 || input.bounds.inits != 0 || input.bounds.vregs != 0 {
            return vec![store_fail(input.task, "reserved store holds records")];
        }
        // Every task-payload reference, parent, and continuation resolves.
        for (id, refs, parent, continuation) in &input.tasks {
            for reference in refs {
                if let Err(detail) = resolve(reference, &input.bounds) {
                    return vec![store_fail(
                        input.task,
                        &format!("task {} payload: {detail}", id.index()),
                    )];
                }
            }
            if let Some(parent) = parent {
                if parent.index() >= input.bounds.tasks {
                    return vec![store_fail(
                        input.task,
                        &format!("task {} parent is dangling", id.index()),
                    )];
                }
            }
            if let Some(continuation) = continuation {
                if continuation.index() >= input.bounds.continuations {
                    return vec![store_fail(
                        input.task,
                        &format!("task {} continuation is dangling", id.index()),
                    )];
                }
            }
        }
        // Every committed result value resolves.
        for (id, value) in &input.results {
            let refs: &[RecordRef] = match value {
                ResultValue::Empty | ResultValue::Ack => &[],
                ResultValue::Record(reference) => std::slice::from_ref(reference),
                ResultValue::Records(references) => references,
                ResultValue::Diagnostic(_) => &[],
            };
            for reference in refs {
                if let Err(detail) = resolve(reference, &input.bounds) {
                    return vec![store_fail(
                        input.task,
                        &format!("result {} value: {detail}", id.index()),
                    )];
                }
            }
            if let ResultValue::Diagnostic(diagnostic) = value {
                if diagnostic.index() >= input.bounds.diagnostics {
                    return vec![store_fail(
                        input.task,
                        &format!("result {} diagnostic is dangling", id.index()),
                    )];
                }
            }
        }
        // Every span names a committed source within its byte length.
        for (id, source, start, end, expansion) in &input.spans {
            let Some((_, len)) = input.source_lens.iter().find(|(sid, _)| sid == source) else {
                return vec![store_fail(
                    input.task,
                    &format!("span {} source is dangling", id.index()),
                )];
            };
            if start > end || end > len {
                return vec![store_fail(
                    input.task,
                    &format!("span {} escapes its source", id.index()),
                )];
            }
            if let Some(expansion) = expansion {
                if expansion.index() >= input.bounds.expansions {
                    return vec![store_fail(
                        input.task,
                        &format!("span {} expansion is dangling", id.index()),
                    )];
                }
            }
        }
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Ack,
        }]
    }
}
