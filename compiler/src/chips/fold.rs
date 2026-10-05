// ============================================================================
// chips/fold.rs — T08 const-fold worker (Wave 1 template chip, CL02/CL03
// integer subset)
//
// Reads committed literals, folds with checked addition, appends exactly
// one `ConstRecord`, and completes with its predicted reference. Only the
// M1 exercised subset is produced: decimal `Integer` literals, `None`
// suffix, `Int` candidate, unsigned operands, `Add` operator. Anything else
// fails with an explicit `Unsupported` diagnostic — never a silent default,
// never a fabricated value.
// ============================================================================

use super::{fail, protocol_fault, Worker};
use crate::bus::{CompilerBus, ConstRecord};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{RecordRef, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, G1_FOLD_CHIP};
use crate::records::{G1DraftBody, RecordDraft};
use crate::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use crate::task::{
    AppendBatch, ConstLegality, ConstantRequest, ConstantResult, DraftRef, Proposal, StoreId,
    TaskGroup, TaskKind, TaskState,
};

/// The T08 const-fold worker: the first chip behind a frozen interface.
///
/// Stateless unit struct: all inputs arrive through the task payload and
/// the read-only bus; all outputs leave as proposals.
pub struct FoldChip;

impl Worker for FoldChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: G1_FOLD_CHIP,
            chip_name: "FoldChip",
            group: TaskGroup::CONSTANT_LAYOUT_INIT,
            task_kinds: vec![TaskKind::CONSTANT_CONST_FOLD],
            reads: vec![FieldPath::new(StoreId::Lex, "literals")],
            writes: vec![FieldPath::new(StoreId::Constants, "records")],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c08_gate1.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &CompilerBus) -> Vec<Proposal> {
        let record = match bus.arenas.tasks.get(task) {
            Ok(record) => record,
            Err(_) => {
                return vec![fail(
                    task,
                    protocol_fault(format!("fold of unknown task {}", task.index())),
                )];
            }
        };
        if record.state != TaskState::Running {
            return vec![fail(
                task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("fold task {} is not running", task.index()),
                ),
            )];
        }
        let request = match ConstantRequest::decode(record.kind, &record.payload) {
            Ok(request) => request,
            Err(error) => return vec![fail(task, error.to_diagnostic())],
        };
        let folded = match self.evaluate(bus, &request) {
            Ok(folded) => folded,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        // Single-append prediction: this batch appends exactly one record,
        // so its ID is the arena count. (The general rule lives in
        // `chips/mod.rs`; the commit verifies the prediction loudly.)
        let predicted = crate::ids::ConstId::from_index(bus.arenas.consts.allocated());
        vec![
            Proposal::AppendRecords {
                task,
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
            .route(task),
        ]
    }
}

impl FoldChip {
    /// Fold one decoded request to its `ConstRecord`.
    ///
    /// Reads committed literals only; computes with checked big-endian
    /// addition; commits exactly one record per evaluation.
    fn evaluate(
        &self,
        bus: &CompilerBus,
        request: &ConstantRequest,
    ) -> Result<ConstRecord, DiagnosticDraft> {
        match request {
            ConstantRequest::Literal { literal, .. } => {
                let record = bus
                    .arenas
                    .literals
                    .get(*literal)
                    .map_err(|_| missing("literal", literal.index()))?;
                check_subset(record)?;
                Ok(ConstRecord {
                    value: record.value.clone(),
                    negative: record.negative,
                })
            }
            ConstantRequest::Binary { lhs, rhs, .. } => {
                let left = bus
                    .arenas
                    .literals
                    .get(*lhs)
                    .map_err(|_| missing("literal", lhs.index()))?;
                let right = bus
                    .arenas
                    .literals
                    .get(*rhs)
                    .map_err(|_| missing("literal", rhs.index()))?;
                check_subset(left)?;
                check_subset(right)?;
                if left.negative || right.negative {
                    return Err(DiagnosticDraft::unsupported(
                        "signed literals are outside the M1 exercised subset",
                    ));
                }
                Ok(ConstRecord {
                    value: add_magnitudes(&left.value, &right.value),
                    negative: false,
                })
            }
        }
    }
}

/// The M1 exercised-subset gate: decimal `Integer`, no suffix, `Int`
/// candidate. Anything else is explicit unsupported, never a default.
fn check_subset(record: &crate::bus::LiteralRecord) -> Result<(), DiagnosticDraft> {
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
