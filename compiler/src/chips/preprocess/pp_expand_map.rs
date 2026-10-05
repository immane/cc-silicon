// ============================================================================
// chips/preprocess/pp_expand_map.rs — T03 PP27 expansion-source-map worker
//
// Pure query over committed expansion provenance: for every pp-token ref in
// the payload, rebuild its per-token origin chain from the frozen
// `SpanRecord { source, start, end, expansion }` links and the committed
// `ExpansionRecord { parent, spelling, expanded, ordinal }` records, then
// complete `Ack`. The chains themselves are returned through the pure
// accessor [`origin_chain`] for the wiring layer; nothing is persisted
// because no `OriginChain` record family/store exists yet (see the
// annotator-vs-query note below).
//
// Narrow projection (task + state + pp-token refs + span/expansion
// records), pure `compute`, ZST, no closures, reads-only, typed `Fail`
// only. One tick completes `Ack` or fails; there is no fan-out, so the
// frozen-join obligation is vacuous (same position as the PP25 worker in
// `pp_pragma.rs`). Deterministic: every chain is a pure function of the
// payload order plus ascending-ID expansion order. No I/O: the chip never
// reads files, samples the environment, or calls other chips.
//
// Annotator-vs-query decision (deliberate, not an oversight): PP12 stamps
// every synthesized token with the invocation name token's span
// (`PP_EXPAND_SLICE.md` §4), so per-token provenance is a query over
// already-committed spans/expansions, not a new append. An annotator chip
// would need a frozen `OriginChain` carrier plus a consumer task; until
// the integrator freezes that carrier, persisting chains here would invent
// schema. Promotion criteria (all required): a frozen chain carrier, an
// independent consumer task for the chains (e.g. PP28 emit or LX16
// location), and integrator-owned kind/chip registration.
//
// The five T03:35 query cases map to structural chain fields (no operator
// tags are invented — `ExpansionRecord` carries none, so classification
// beyond structure would be fabrication):
// * `#` stringify-raw: the frame's `spelling` span names the raw argument
//   bytes; the stringized token's own span is the invocation name span.
// * `##` paste: the frame's `expanded` span names the paste product; both
//   operand spans stay reachable through the same frame.
// * prescan-vs-raw: `spelling` (raw form) and `expanded` (prescanned form)
//   are kept side by side per frame instead of collapsing to one.
// * blue-paint rescan: `parent` linkage plus per-frame `ordinal`/`depth`
//   recover the rescan nesting without re-running substitution.
// * nested include origins: every frame resolves to a `SpanRecord`, whose
//   `source` identifies the physical file at that nesting level.
//
// Frozen registration (`/30` integrator-owned): `PP27_TASK_KIND` aliases
// `TaskKind::PREPROCESS_EXPAND_MAP` (`PREPROCESS` local 34),
// `PP27_CHIP` is `crate::manifest::PP27_CHIP` (`ChipId(37)`), the
// kind-registry row lives in `TaskKindRegistry::pp_expand_map_slice()`,
// the stage-1 row in `STAGE_ASSIGNMENT`, the routed layer is 1, and the
// acceptance test is `compiler/tests/c30_expand_map.rs`. There is
// deliberately no bus write and hence no allowlist row — same as
// PP23/PP25: success completes `Ack` (the chains are returned by
// [`origin_chain`] for the wiring layer) and every failure path is a
// typed `Fail`. Dangling links are never silently truncated.
// ============================================================================

use crate::bus::{ExpansionRecord, PpTokenRecord, SpanRecord};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{ExpansionId, PpTokenId, RecordRef, SourceId, SpanId, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, PP27_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Frozen PP27 task kind (aliases `TaskKind::PREPROCESS_EXPAND_MAP`,
/// frozen by the `/30` integrator; the local code is `PREPROCESS` 34,
/// first code after `/29`).
pub const PP27_TASK_KIND: TaskKind = TaskKind::PREPROCESS_EXPAND_MAP;

/// Narrow projection for the expansion-source-map computation.
#[derive(Clone, Debug)]
pub struct PpExpandMapInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Payload token bodies in payload order.
    pub tokens: Vec<(PpTokenId, PpTokenRecord)>,
    /// Span records: payload token spans in first-use order, then the
    /// spelling/expanded spans of every committed expansion in
    /// ascending-`ExpansionId` order for IDs not already covered.
    pub spans: Vec<(SpanId, SpanRecord)>,
    /// Committed expansion records in ascending-ID order.
    pub expansions: Vec<(ExpansionId, ExpansionRecord)>,
    /// Committed sources referenced by the projected spans, in first-use
    /// order over `spans` (existence proof for nested-include origins).
    pub sources: Vec<SourceId>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_pp_expand_map_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<PpExpandMapInput, DiagnosticDraft> {
    let record = match bus.arenas.tasks.get(task) {
        Ok(record) => record,
        Err(_) => {
            return Err(protocol_fault(format!(
                "expand_map of unknown task {}",
                task.index()
            )));
        }
    };
    if record.kind != PP27_TASK_KIND {
        return Err(protocol_fault(format!(
            "expand_map task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.is_empty() {
        return Err(protocol_fault(format!(
            "expand_map task {} payload must carry pp-token refs",
            task.index()
        )));
    }
    let mut tokens = Vec::with_capacity(record.payload.refs.len());
    for reference in &record.payload.refs {
        match reference {
            RecordRef::PpToken(id) => match bus.arenas.pp_tokens.get(*id) {
                Ok(found) => tokens.push((*id, found.clone())),
                Err(_) => {
                    return Err(invalid(format!(
                        "expand_map reads missing pp-token {}",
                        id.index()
                    )));
                }
            },
            _ => {
                return Err(protocol_fault(format!(
                    "expand_map task {} payload must be pp-token refs",
                    task.index()
                )));
            }
        }
    }
    let mut expansions: Vec<(ExpansionId, ExpansionRecord)> = Vec::new();
    for (id, body) in bus.arenas.expansions.iter() {
        expansions.push((id, *body));
    }
    sort_expansions(&mut expansions);
    let mut spans: Vec<(SpanId, SpanRecord)> = Vec::new();
    for (_, token) in &tokens {
        if !span_is_projected(&spans, token.span) {
            match bus.arenas.spans.get(token.span) {
                Ok(found) => spans.push((token.span, *found)),
                Err(_) => {
                    return Err(invalid(format!(
                        "expand_map reads missing span {}",
                        token.span.index()
                    )));
                }
            }
        }
    }
    for (_, expansion) in &expansions {
        if !span_is_projected(&spans, expansion.spelling) {
            match bus.arenas.spans.get(expansion.spelling) {
                Ok(found) => spans.push((expansion.spelling, *found)),
                Err(_) => {
                    return Err(invalid(format!(
                        "expand_map reads missing span {}",
                        expansion.spelling.index()
                    )));
                }
            }
        }
        if !span_is_projected(&spans, expansion.expanded) {
            match bus.arenas.spans.get(expansion.expanded) {
                Ok(found) => spans.push((expansion.expanded, *found)),
                Err(_) => {
                    return Err(invalid(format!(
                        "expand_map reads missing span {}",
                        expansion.expanded.index()
                    )));
                }
            }
        }
    }
    let mut sources: Vec<SourceId> = Vec::new();
    for (_, span) in &spans {
        if !source_is_projected(&sources, span.source) {
            match bus.arenas.sources.get(span.source) {
                Ok(_) => sources.push(span.source),
                Err(_) => {
                    return Err(invalid(format!(
                        "expand_map reads missing source {}",
                        span.source.index()
                    )));
                }
            }
        }
    }
    Ok(PpExpandMapInput {
        task,
        state: record.state.clone(),
        tokens,
        spans,
        expansions,
        sources,
    })
}

/// Sort expansions into ascending-`ExpansionId` order (arena iteration
/// already yields it; the sort pins the projector contract explicitly).
fn sort_expansions(expansions: &mut [(ExpansionId, ExpansionRecord)]) {
    let mut index = 1usize;
    while index < expansions.len() {
        let mut cursor = index;
        while cursor > 0 && expansions[cursor - 1].0.index() > expansions[cursor].0.index() {
            expansions.swap(cursor - 1, cursor);
            cursor -= 1;
        }
        index += 1;
    }
}

/// True when `id` already has a projected span record.
fn span_is_projected(spans: &[(SpanId, SpanRecord)], id: SpanId) -> bool {
    for (known, _) in spans {
        if *known == id {
            return true;
        }
    }
    false
}

/// True when `id` already has a projected source.
fn source_is_projected(sources: &[SourceId], id: SourceId) -> bool {
    for known in sources {
        if *known == id {
            return true;
        }
    }
    false
}

/// Typed `Invalid` failure for dangling record references.
fn invalid(message: impl Into<String>) -> DiagnosticDraft {
    DiagnosticDraft::error(DiagnosticCode::new(DiagGroup::Task, 4), message)
}

/// One expansion frame of an origin chain (one `ExpansionRecord` hop).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginFrame {
    /// Expansion record this frame replays.
    pub expansion: ExpansionId,
    /// Raw-form span (`#` operands read this side).
    pub spelling: SpanId,
    /// Product-form span (`##` products and prescanned forms land here).
    pub expanded: SpanId,
    /// Deterministic expansion ordinal from the committed record.
    pub ordinal: u32,
    /// Walk depth (`0` is the token's own expansion, innermost first).
    pub depth: u32,
}

/// Per-token origin chain: the token's own span plus its expansion frames
/// innermost-first. An unexpanded token carries no frames; its origin is
/// its own span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginChain {
    /// Token this chain was built for.
    pub token: PpTokenId,
    /// The token's own committed span.
    pub span: SpanId,
    /// Expansion frames, innermost first.
    pub frames: Vec<OriginFrame>,
}

/// Build the origin chain for one projected token.
///
/// Pure over the narrow projection: follows `span.expansion` through
/// committed `parent` links, resolving every spelling/expanded span on
/// the way. Any dangling link (unprojected span, dangling expansion, or
/// dangling parent) is a typed `Invalid` failure, never a silent
/// truncation. A repeated expansion ID on one walk is an `Invalid` cycle
/// failure, and the walk additionally stops loudly past
/// `expansions.len()` frames, so a corrupt parent graph can neither loop
/// forever nor spin silently.
pub fn origin_chain(
    input: &PpExpandMapInput,
    token: PpTokenId,
) -> Result<OriginChain, DiagnosticDraft> {
    let body = match find_token(input, token) {
        Some(found) => found,
        None => {
            return Err(invalid(format!(
                "expand_map reads unprojected pp-token {}",
                token.index()
            )));
        }
    };
    let span = match find_span(input, body.span) {
        Some(found) => found,
        None => {
            return Err(invalid(format!(
                "expand_map reads unprojected span {}",
                body.span.index()
            )));
        }
    };
    let mut frames: Vec<OriginFrame> = Vec::new();
    let mut visited: Vec<ExpansionId> = Vec::new();
    let mut current = span.expansion;
    let mut depth = 0u32;
    while let Some(id) = current {
        if expansion_is_visited(&visited, id) {
            return Err(invalid(format!(
                "expand_map expansion {} has a cyclic parent link",
                id.index()
            )));
        }
        if frames.len() > input.expansions.len() {
            return Err(invalid(format!(
                "expand_map expansion {} walk exceeds the committed expansion count",
                id.index()
            )));
        }
        let expansion = match find_expansion(input, id) {
            Some(found) => found,
            None => {
                return Err(invalid(format!(
                    "expand_map reads dangling expansion {}",
                    id.index()
                )));
            }
        };
        if find_span(input, expansion.spelling).is_none() {
            return Err(invalid(format!(
                "expand_map reads unprojected span {}",
                expansion.spelling.index()
            )));
        }
        if find_span(input, expansion.expanded).is_none() {
            return Err(invalid(format!(
                "expand_map reads unprojected span {}",
                expansion.expanded.index()
            )));
        }
        visited.push(id);
        frames.push(OriginFrame {
            expansion: id,
            spelling: expansion.spelling,
            expanded: expansion.expanded,
            ordinal: expansion.ordinal,
            depth,
        });
        current = expansion.parent;
        depth = depth.saturating_add(1);
    }
    Ok(OriginChain {
        token,
        span: body.span,
        frames,
    })
}

/// Outermost origin span of a chain: the outermost frame's `expanded` span,
/// or the token's own span when the token is unexpanded.
pub fn origin_root(chain: &OriginChain) -> SpanId {
    let mut root = chain.span;
    for frame in &chain.frames {
        root = frame.expanded;
    }
    root
}

/// Look up a projected token body by ID.
fn find_token(input: &PpExpandMapInput, id: PpTokenId) -> Option<PpTokenRecord> {
    for (known, body) in &input.tokens {
        if *known == id {
            return Some(body.clone());
        }
    }
    None
}

/// Look up a projected span record by ID.
fn find_span(input: &PpExpandMapInput, id: SpanId) -> Option<SpanRecord> {
    for (known, body) in &input.spans {
        if *known == id {
            return Some(*body);
        }
    }
    None
}

/// Look up a projected expansion record by ID.
fn find_expansion(input: &PpExpandMapInput, id: ExpansionId) -> Option<ExpansionRecord> {
    for (known, body) in &input.expansions {
        if *known == id {
            return Some(*body);
        }
    }
    None
}

/// True when `id` was already visited on the current parent walk.
fn expansion_is_visited(visited: &[ExpansionId], id: ExpansionId) -> bool {
    for known in visited {
        if *known == id {
            return true;
        }
    }
    false
}

/// The T03 expansion-source-map worker (PP27 frozen slice).
pub struct PpExpandMapChip;

impl Worker for PpExpandMapChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: PP27_CHIP,
            chip_name: "PpExpandMapChip",
            group: TaskGroup::PREPROCESS,
            task_kinds: vec![PP27_TASK_KIND],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Pp, "tokens"),
                FieldPath::new(StoreId::Sources, "bytes"),
                FieldPath::new(StoreId::Sources, "spans"),
                FieldPath::new(StoreId::Sources, "expansion"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c30_expand_map.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_pp_expand_map_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl PpExpandMapChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    ///
    /// Rebuilds the origin chain for every payload token in payload order:
    /// the first dangling link fails the task as typed `Invalid`. A fully
    /// validated stream completes `Ack` with no bus writes — the chains are
    /// served through [`origin_chain`] for the wiring layer.
    pub fn compute(&self, input: &PpExpandMapInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("expand_map task {} is not running", input.task.index()),
                ),
            )];
        }
        for (id, _) in &input.tokens {
            if let Err(diagnostic) = origin_chain(input, *id) {
                return vec![fail(input.task, diagnostic)];
            }
        }
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Ack,
        }]
    }
}
