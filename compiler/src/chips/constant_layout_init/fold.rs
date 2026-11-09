// ============================================================================
// chips/fold.rs — T08 const-fold worker (Wave 1 template chip, CL02/CL03
// integer subset)
//
// Reads committed literals and the committed constant type, folds with
// checked addition, appends exactly one typed `ConstRecord`, and completes
// with its predicted reference. Only the M1 exercised subset is produced:
// decimal `Integer` literals, `None`
// suffix, `Int` candidate, unsigned operands, `Add` operator. Anything else
// fails with an explicit `Unsupported` diagnostic — never a silent default,
// never a fabricated value.
//
// Isolation: the semantic computation sees only `FoldInput` (task header,
// payload, the referenced literal bodies, the const-arena count, and the
// configured bit budget). It never receives the full bus; the `Worker`
// adapter below owns the narrow projection.
// ============================================================================

use crate::bus::{
    ConstRecord, ConstValue, IntRank, IntegerConst, LiteralRecord, TypeKind, TypeRecord,
};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, NodeId, RecordRef, TaskId, TypeId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, G1_FOLD_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use crate::task::{
    AppendBatch, ConstLegality, ConstantRequest, ConstantResult, DraftRef, Payload, Proposal,
    StoreId, TaskGroup, TaskKind, TaskState,
};
use std::collections::BTreeMap;

/// Narrow projection for the fold computation: the only bus facts the chip
/// logic may observe.
///
/// Built by the adapter from manifest-declared reads (`lex.literals` for
/// operand bodies, `sem.records`/`types.records` for the committed constant
/// type) plus mechanical dispatch facts (task header, const-arena count,
/// configured bit budget). No other arena, queue, or config field is visible
/// here by construction.
#[derive(Clone, Debug)]
pub struct FoldInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Task kind (drives strict `/8` decode).
    pub kind: TaskKind,
    /// Lifecycle state at dispatch (must be `Running` to compute).
    pub state: TaskState,
    /// Typed input references.
    pub payload: Payload,
    /// Referenced literal bodies present in `lex.literals` (absent IDs fail
    /// loudly in `compute`, never silently default).
    pub literals: BTreeMap<LiteralId, LiteralRecord>,
    /// Payload-referenced nodes present in `parse.nodes` (`/9` PCR-08: a
    /// `Binary` request names a committed node; a dangling node fails loudly
    /// instead of folding literals around it).
    pub nodes_present: std::collections::BTreeSet<NodeId>,
    /// `consts` arena count at dispatch (single-append prediction base).
    pub consts_allocated: u32,
    /// Resolved committed constant type and body, if the request's type
    /// facts are present (`/42`). Missing/ambiguous facts stay `None` and
    /// fail loudly in `compute`.
    pub constant_type: Option<(TypeId, TypeRecord)>,
    /// Configured constant bit budget (`limits.max_const_bits`).
    pub max_const_bits: u32,
}

/// Resolve the committed constant type for one decoded request.
///
/// Binary requests use the committed `SemRecord.ty` of their
/// `BinaryExpression` node. Literal requests use the unique committed
/// canonical `int` type because the frozen literal-form wire payload carries
/// no node. Missing or ambiguous facts yield `None`.
fn resolve_constant_type(
    bus: &crate::bus::CompilerBus,
    request: &ConstantRequest,
) -> Option<(TypeId, TypeRecord)> {
    match request {
        ConstantRequest::Binary { node, .. } => {
            let mut found = None;
            for (_, sem) in bus.arenas.sem.iter() {
                if sem.node == *node {
                    if found.is_some() {
                        return None;
                    }
                    found = Some(sem.ty);
                }
            }
            let ty = found?;
            bus.arenas
                .types
                .get(ty)
                .ok()
                .cloned()
                .map(|body| (ty, body))
        }
        ConstantRequest::Literal { .. } => {
            let mut found = None;
            for (id, body) in bus.arenas.types.iter() {
                if matches!(
                    body.kind,
                    TypeKind::Int {
                        rank: IntRank::Int,
                        signed: true
                    }
                ) {
                    if found.is_some() {
                        return None;
                    }
                    found = Some((id, body.clone()));
                }
            }
            found
        }
    }
}

/// Build the narrow projection for one dispatched task.
///
/// Reads exactly: the task record (dispatch header), the payload-referenced
/// literal bodies, the const-arena count, and the configured limit. Missing
/// literals are represented as absent map entries so `compute` can fail
/// loudly; an unknown task is the only adapter-level fault here.
pub fn project_fold_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<FoldInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("fold of unknown task {}", task.index())))?;
    let mut literals = BTreeMap::new();
    let mut nodes_present = std::collections::BTreeSet::new();
    for reference in &record.payload.refs {
        if let RecordRef::Node(id) = reference {
            if bus.arenas.nodes.get(*id).is_ok() {
                nodes_present.insert(*id);
            }
        }
        if let RecordRef::Literal(id) = reference {
            if let Ok(body) = bus.arenas.literals.get(*id) {
                literals.insert(*id, body.clone());
            }
        }
    }
    let constant_type = ConstantRequest::decode(record.kind, &record.payload)
        .ok()
        .and_then(|request| resolve_constant_type(bus, &request));
    Ok(FoldInput {
        task,
        kind: record.kind,
        state: record.state.clone(),
        payload: record.payload.clone(),
        literals,
        nodes_present,
        consts_allocated: bus.arenas.consts.allocated(),
        constant_type,
        max_const_bits: bus.limits().max_const_bits,
    })
}

/// The T08 const-fold worker: the first chip behind a frozen interface.
///
/// Stateless unit struct: all inputs arrive through the narrow `FoldInput`;
/// all outputs leave as proposals.
pub struct FoldChip;

impl Worker for FoldChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: G1_FOLD_CHIP,
            chip_name: "FoldChip",
            group: TaskGroup::CONSTANT_LAYOUT_INIT,
            task_kinds: vec![TaskKind::CONSTANT_CONST_FOLD],
            // `/9` PCR-10: the narrow projection reads the dispatched task
            // header (`tasks.active.*`), the referenced literal bodies
            // (`lex.literals`), the const-arena count (`constants.records`
            // count only, no body), and the configured bit budget
            // (`config.limits`). `parse.nodes` existence is a mechanical
            // dispatch fact pending the T05 `parse.nodes` field freeze; the
            // check is existence-only, never a body read.
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Lex, "literals"),
                FieldPath::new(StoreId::Constants, "records"),
                FieldPath::new(StoreId::Sem, "records"),
                FieldPath::new(StoreId::Types, "records"),
                FieldPath::new(StoreId::Config, "limits"),
            ],
            writes: vec![FieldPath::new(StoreId::Constants, "records")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c08_gate1.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_fold_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl FoldChip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &FoldInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("fold task {} is not running", input.task.index()),
                ),
            )];
        }
        let request = match ConstantRequest::decode(input.kind, &input.payload) {
            Ok(request) => request,
            Err(error) => return vec![fail(input.task, error.to_diagnostic())],
        };
        let folded = match self.evaluate(input, &request) {
            Ok(folded) => folded,
            Err(diagnostic) => return vec![fail(input.task, diagnostic)],
        };
        // Single-append prediction: this batch appends exactly one record,
        // so its ID is the arena count. (The general rule lives in
        // `chips/mod.rs`; the commit verifies the prediction loudly.)
        let predicted = crate::ids::ConstId::from_index(input.consts_allocated);
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![RecordDraft {
                        family: crate::ids::RecordFamily::Const,
                        index: DraftRef(0),
                    }],
                    bodies: vec![G1DraftBody::Const(folded)],
                },
            },
            ConstantResult {
                value: RecordRef::Const(predicted),
                legality: ConstLegality::Legal,
            }
            .route(input.task),
        ]
    }

    /// Fold one decoded request to its `ConstRecord`.
    ///
    /// Reads projected literal bodies only; computes with checked big-endian
    /// addition; enforces the configured bit budget (insufficient budget is a
    /// typed overflow `Fail`, never a `Legal` success); commits exactly one
    /// record per evaluation.
    fn evaluate(
        &self,
        input: &FoldInput,
        request: &ConstantRequest,
    ) -> Result<ConstRecord, DiagnosticDraft> {
        let (constant_ty, constant_type) = input
            .constant_type
            .as_ref()
            .ok_or_else(|| missing("constant type", 0))?;
        if !matches!(constant_type.kind, TypeKind::Int { .. }) {
            return Err(DiagnosticDraft::unsupported(
                "integer constant does not have an integer type",
            ));
        }
        match request {
            ConstantRequest::Literal { literal, .. } => {
                let record = input
                    .literals
                    .get(literal)
                    .ok_or_else(|| missing("literal", literal.index()))?;
                check_subset(record)?;
                let value = IntegerConst::new(record.value.clone(), record.negative);
                check_budget(input.max_const_bits, &value.magnitude)?;
                Ok(ConstRecord {
                    ty: *constant_ty,
                    value: ConstValue::Integer(value),
                    origins: vec![*literal],
                })
            }
            ConstantRequest::Binary { node, lhs, rhs, .. } => {
                // `/9` PCR-08: the binary node must be a live committed node.
                // Folding around a dangling node would accept an input the
                // Gate 1 contract requires as committed.
                if !input.nodes_present.contains(node) {
                    return Err(missing("node", node.index()));
                }
                let left = input
                    .literals
                    .get(lhs)
                    .ok_or_else(|| missing("literal", lhs.index()))?;
                let right = input
                    .literals
                    .get(rhs)
                    .ok_or_else(|| missing("literal", rhs.index()))?;
                check_subset(left)?;
                check_subset(right)?;
                if left.negative || right.negative {
                    return Err(DiagnosticDraft::unsupported(
                        "signed literals are outside the M1 exercised subset",
                    ));
                }
                let magnitude = add_magnitudes(&left.value, &right.value);
                check_budget(input.max_const_bits, &magnitude)?;
                Ok(ConstRecord {
                    ty: *constant_ty,
                    value: ConstValue::Integer(IntegerConst {
                        magnitude,
                        negative: false,
                    }),
                    origins: vec![*lhs, *rhs],
                })
            }
        }
    }
}

/// The M1 exercised-subset gate: decimal `Integer`, no suffix, `Int`
/// candidate. Anything else is explicit unsupported, never a default.
fn check_subset(record: &LiteralRecord) -> Result<(), DiagnosticDraft> {
    if record.kind != LiteralKind::Integer
        || record.suffix != LiteralSuffix::None
        || record.radix != 10
        || record.candidate_type != Lx08CandidateType::Int
    {
        return Err(DiagnosticDraft::unsupported(
            "literal outside the M1 exercised subset",
        ));
    }
    Ok(())
}

/// Enforce the configured constant bit budget against a magnitude.
///
/// The required width is the minimal big-endian bit length (all-zero
/// magnitudes need one bit). A result that does not fit is the accepted
/// `ConstOverflow` chip diagnostic via `Fail` — never a `Legal` success and
/// never a `CommitError` (the commit does not recompute constant values).
fn check_budget(max_const_bits: u32, magnitude: &[u8]) -> Result<(), DiagnosticDraft> {
    let required = const_bits_required(magnitude);
    if required > max_const_bits {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::CONST_OVERFLOW,
            format!("folded constant needs {required} bits, budget is {max_const_bits}"),
        ));
    }
    Ok(())
}

/// Minimal bit length of a big-endian magnitude (all-zero needs one bit).
pub fn const_bits_required(magnitude: &[u8]) -> u32 {
    let first = magnitude.iter().position(|&byte| byte != 0);
    let Some(start) = first else {
        return 1;
    };
    let head = magnitude[start];
    let head_bits = 8 - head.leading_zeros();
    ((magnitude.len() - start - 1) as u32) * 8 + head_bits
}

fn missing(what: &str, index: u32) -> DiagnosticDraft {
    DiagnosticDraft::error(
        DiagnosticCode::new(DiagGroup::Task, 4),
        format!("fold reads missing {what} {index}"),
    )
}

/// Checked big-endian magnitude addition with carry; minimal bytes out
/// (at least one zero byte). Unbounded inputs cannot overflow: `n`-byte +
/// `m`-byte always fits in `max(n, m) + 1` bytes.
fn add_magnitudes(lhs: &[u8], rhs: &[u8]) -> Vec<u8> {
    let width = lhs.len().max(rhs.len()).max(1);
    let mut out = vec![0u8; width + 1];
    let mut carry: u16 = 0;
    for i in 0..width {
        let left = if i < lhs.len() {
            lhs[lhs.len() - 1 - i]
        } else {
            0
        };
        let right = if i < rhs.len() {
            rhs[rhs.len() - 1 - i]
        } else {
            0
        };
        let sum = left as u16 + right as u16 + carry;
        out[width - i] = (sum & 0xff) as u8;
        carry = sum >> 8;
    }
    out[0] = carry as u8;
    let first = out.iter().position(|&byte| byte != 0).unwrap_or(width);
    out[first..].to_vec()
}
