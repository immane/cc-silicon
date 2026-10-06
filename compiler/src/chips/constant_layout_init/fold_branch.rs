// ============================================================================
// chips/constant_layout_init/fold_branch.rs — T08 CL04/CL07 narrow
// (FROZEN `/37`): selected-branch-only `&&`/`||`/`?:` certification plus
// static-assert checks, clamped to the M1 exercised magnitude path.
//
// Frozen kinds: `constant_layout_init.const_branch_and` (local 17),
// `const_branch_or` (18), `const_branch_cond` (19),
// `const_static_assert` (20) — first codes after the Gate 1
// `const_fold` (local 16); `CONSTANT_LAYOUT_INIT` owners start new
// codes at local 21. Frozen chips: `CL04_AND_CHIP = 52`,
// `CL04_OR_CHIP = 53`, `CL04_COND_CHIP = 54`, `CL07_ASSERT_CHIP = 55`
// — first free IDs after the Wave 3 `PA38_CHIP = 51` reservation.
//
// Semantics (all four narrows are pure `Ack`/`Fail` certifications;
// `writes` is empty so no store-owner allowlist row is needed):
//
// * CL04 selected branch (`BranchAndChip`, `BranchOrChip`,
//   `BranchCondChip`, one fixed `BranchOp` per shell because the operator
//   cannot ride the frozen wire yet): evaluate the condition plus ONLY the
//   short-circuit-selected branch. The unselected operand is never
//   subset-checked, never budget-checked, and may even dangle — so `0 &&
//   <bad>` acks `0` and `1 ? 3 : <bad>` is never evaluated, mirroring the
//   `0&&1/0`, `1?3:1/0` acceptance. Logical results are canonical C `0`/`1`
//   magnitudes (`vec![0]` / `vec![1]`); `?:` passes the selected magnitude
//   through verbatim. Committing branch values stays future work pending the
//   result carrier freeze; this narrow certifies selection and budget.
// * CL07 assert (`StaticAssertChip`): one condition. Zero fails as a failed
//   assertion, nonzero passes (`Ack`), and a payload that does not name a
//   committed literal fails as `NotConstantExpression` (never ICE, never a
//   silent pass).
//
// Clamp: big-endian unsigned magnitudes only (`2`/`3`/folded `2+3` path);
// no new arithmetic. `check_subset` + `check_budget` (+
// `const_bits_required`, `missing`) are verbatim chip-local copies of the
// `fold.rs` gates: out-of-subset is `Unsupported`, over-budget is the typed
// `ConstOverflow` chip diagnostic via `Fail` — never a `Legal` success.
//
// Isolation: like `FoldChip`, the computation sees only its narrow input
// (task header facts, projected literal bodies, the configured bit budget).
// It never receives the full bus; the adapter owns the projection.
// ============================================================================

use crate::bus::LiteralRecord;
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{LiteralId, RecordRef, TaskId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, CL04_AND_CHIP, CL04_COND_CHIP,
    CL04_OR_CHIP, CL07_ASSERT_CHIP,
};
use crate::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Frozen `/37` task kind for the CL04 `&&` narrow.
pub const CL04_AND_TASK_KIND: TaskKind = TaskKind::CONSTANT_CONST_BRANCH_AND;
/// Frozen `/37` task kind for the CL04 `||` narrow.
pub const CL04_OR_TASK_KIND: TaskKind = TaskKind::CONSTANT_CONST_BRANCH_OR;
/// Frozen `/37` task kind for the CL04 `?:` narrow.
pub const CL04_COND_TASK_KIND: TaskKind = TaskKind::CONSTANT_CONST_BRANCH_COND;
/// Frozen `/37` task kind for the CL07 static-assert narrow.
pub const CL07_ASSERT_TASK_KIND: TaskKind = TaskKind::CONSTANT_CONST_STATIC_ASSERT;

/// The branch operator a shell evaluates (fixed per shell; the wire encoding
/// of the operator is a pending T01 co-freeze item).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchOp {
    /// Short-circuit `&&`.
    And,
    /// Short-circuit `||`.
    Or,
    /// Conditional `?:`.
    Cond,
}

impl BranchOp {
    /// Canonical encoding name (frozen `/37` co-freeze: `and` / `or` / `cond`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
            Self::Cond => "cond",
        }
    }

    /// Frozen payload arity: `[lhs, rhs]` for `&&`/`||` (`lhs` doubles as the
    /// condition), `[cond, then, else]` for `?:`.
    const fn arity(self) -> usize {
        match self {
            Self::And | Self::Or => 2,
            Self::Cond => 3,
        }
    }
}

/// A certified branch-result magnitude (big-endian bytes). Carried for
/// budget certification and tests only; committing branch values awaits the
/// result-carrier freeze.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BranchValue(pub Vec<u8>);

/// One projected operand: its ID plus its committed body, if any. An absent
/// body is only fatal when the operand is actually selected (short-circuit
/// skips never touch the unselected operand).
#[derive(Clone, Debug)]
pub struct BranchOperand {
    /// Referenced literal.
    pub id: LiteralId,
    /// Committed body (`None` for a dangling reference).
    pub body: Option<LiteralRecord>,
}

/// Narrow projection for the CL04 selected-branch certification.
#[derive(Clone, Debug)]
pub struct BranchInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Fixed operator of the handling shell.
    pub op: BranchOp,
    /// Lifecycle state at dispatch (must be `Running` to compute).
    pub state: TaskState,
    /// Condition operand (`&&`/`||` duplicate the `lhs` reference here).
    pub cond: BranchOperand,
    /// Left / `then` operand.
    pub lhs: BranchOperand,
    /// Right / `else` operand.
    pub rhs: BranchOperand,
    /// Configured constant bit budget (`limits.max_const_bits`).
    pub max_const_bits: u32,
}

/// The assert condition, typed: either a projected literal reference or an
/// explicitly non-constant use (never a silent default).
#[derive(Clone, Debug)]
pub enum AssertCond {
    /// Exactly one `RecordRef::Literal` was named.
    Literal(BranchOperand),
    /// Anything else (wrong arity, non-literal family): not an ICE.
    NotConstant,
}

/// Narrow projection for the CL07 static-assert check.
#[derive(Clone, Debug)]
pub struct AssertInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running` to compute).
    pub state: TaskState,
    /// Typed condition (see [`AssertCond`]).
    pub cond: AssertCond,
    /// Configured constant bit budget (`limits.max_const_bits`).
    pub max_const_bits: u32,
}

/// Build the narrow branch projection for one dispatched task under `op.
///
/// Reads exactly: the task record (dispatch header), the payload-referenced
/// literal bodies, and the configured limit. Dangling literals are `None`
/// bodies so `compute` can ignore an unselected one or fail loudly on a
/// selected one. Wrong arity or a non-literal family is a protocol fault
/// (frozen shape violation), never a silent reinterpretation.
pub fn project_branch_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
    op: BranchOp,
) -> Result<BranchInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("fold-branch of unknown task {}", task.index())))?;
    if record.payload.refs.len() != op.arity() {
        return Err(protocol_fault(format!(
            "fold-branch {} task {} carries {} references, expected {}",
            op.name(),
            task.index(),
            record.payload.refs.len(),
            op.arity()
        )));
    }
    let mut operands = Vec::with_capacity(op.arity());
    for reference in &record.payload.refs {
        let id = match reference {
            RecordRef::Literal(id) => *id,
            _ => {
                return Err(protocol_fault(format!(
                    "fold-branch {} task {} reference is not a literal",
                    op.name(),
                    task.index()
                )));
            }
        };
        let body = bus.arenas.literals.get(id).ok().cloned();
        operands.push(BranchOperand { id, body });
    }
    let (cond, lhs, rhs) = match op {
        BranchOp::And | BranchOp::Or => {
            let lhs = operands.remove(0);
            let rhs = operands.remove(0);
            let cond = lhs.clone();
            (cond, lhs, rhs)
        }
        BranchOp::Cond => {
            let cond = operands.remove(0);
            let lhs = operands.remove(0);
            let rhs = operands.remove(0);
            (cond, lhs, rhs)
        }
    };
    Ok(BranchInput {
        task,
        op,
        state: record.state.clone(),
        cond,
        lhs,
        rhs,
        max_const_bits: bus.limits().max_const_bits,
    })
}

/// Build the narrow assert projection for one dispatched task.
///
/// Exactly one `RecordRef::Literal` names the asserted ICE; any other shape
/// is the typed [`AssertCond::NotConstant`] path (not a protocol fault: the
/// use simply is not a constant expression).
pub fn project_assert_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<AssertInput, DiagnosticDraft> {
    let record =
        bus.arenas.tasks.get(task).map_err(|_| {
            protocol_fault(format!("static-assert of unknown task {}", task.index()))
        })?;
    let cond = match record.payload.refs.as_slice() {
        [RecordRef::Literal(id)] => {
            let body = bus.arenas.literals.get(*id).ok().cloned();
            AssertCond::Literal(BranchOperand { id: *id, body })
        }
        _ => AssertCond::NotConstant,
    };
    Ok(AssertInput {
        task,
        state: record.state.clone(),
        cond,
        max_const_bits: bus.limits().max_const_bits,
    })
}

/// C truthiness over a big-endian magnitude: any nonzero byte is true.
/// (Empty magnitudes read as false; T04 never commits them.)
fn is_nonzero(magnitude: &[u8]) -> bool {
    magnitude.iter().any(|&byte| byte != 0)
}

/// Fetch the committed body of a SELECTED operand: dangling is a loud
/// `Task,4` missing failure, out-of-subset is `Unsupported`, over-budget is
/// `ConstOverflow`. Unselected operands never reach this function.
fn selected_body<'a>(
    input: &'a BranchInput,
    operand: &'a BranchOperand,
) -> Result<&'a LiteralRecord, DiagnosticDraft> {
    let body = operand
        .body
        .as_ref()
        .ok_or_else(|| missing("literal", operand.id.index()))?;
    check_subset(body)?;
    check_budget(input.max_const_bits, &body.value)?;
    Ok(body)
}

/// Pure CL04 selected-branch evaluation over the narrow projection (no bus
/// access). Only the short-circuit-selected operand is gated; the other may
/// be absent or out-of-subset without effect.
pub fn eval_branch(input: &BranchInput) -> Result<BranchValue, DiagnosticDraft> {
    if input.state != TaskState::Running {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("fold-branch task {} is not running", input.task.index()),
        ));
    }
    let cond = input
        .cond
        .body
        .as_ref()
        .ok_or_else(|| missing("literal", input.cond.id.index()))?;
    check_subset(cond)?;
    check_budget(input.max_const_bits, &cond.value)?;
    let taken = is_nonzero(&cond.value);
    // Canonical C logical results are `0`/`1`; `?:` passes the selected
    // magnitude through verbatim (clamped path: no new arithmetic).
    let result = match (input.op, taken) {
        (BranchOp::And, false) => vec![0],
        (BranchOp::And, true) => {
            selected_body(input, &input.rhs)?;
            vec![1]
        }
        (BranchOp::Or, true) => vec![1],
        (BranchOp::Or, false) => {
            let rhs = selected_body(input, &input.rhs)?;
            if is_nonzero(&rhs.value) {
                vec![1]
            } else {
                vec![0]
            }
        }
        (BranchOp::Cond, true) => selected_body(input, &input.lhs)?.value.clone(),
        (BranchOp::Cond, false) => selected_body(input, &input.rhs)?.value.clone(),
    };
    check_budget(input.max_const_bits, &result)?;
    Ok(BranchValue(result))
}

/// Pure CL07 assert evaluation over the narrow projection (no bus access):
/// nonzero passes, zero fails as a failed assertion, non-ICE fails as
/// `NotConstantExpression`.
pub fn eval_assert(input: &AssertInput) -> Result<(), DiagnosticDraft> {
    if input.state != TaskState::Running {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("static-assert task {} is not running", input.task.index()),
        ));
    }
    let operand = match &input.cond {
        AssertCond::NotConstant => {
            return Err(DiagnosticDraft::error(
                DiagnosticCode::CONST_NOT_CONSTANT_EXPRESSION,
                "static assertion condition is not a constant expression",
            ));
        }
        AssertCond::Literal(operand) => operand,
    };
    let body = operand
        .body
        .as_ref()
        .ok_or_else(|| missing("literal", operand.id.index()))?;
    check_subset(body)?;
    check_budget(input.max_const_bits, &body.value)?;
    if is_nonzero(&body.value) {
        Ok(())
    } else {
        // Frozen code: `Task,3` is the const-eval failure slot shared with the
        // `NotConstantExpression` route (messages distinguish); the exact
        // assert-failure code stays open pending the T01 code freeze.
        Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 3),
            "static assertion failed: condition is zero",
        ))
    }
}

/// Shared terminal mapping: `Ok` certifies with `Ack`, `Err` fails loudly.
fn ack_or_fail(task: TaskId, outcome: Result<(), DiagnosticDraft>) -> Vec<Proposal> {
    match outcome {
        Ok(()) => vec![Proposal::Complete {
            task,
            value: ResultValue::Ack,
        }],
        Err(diagnostic) => vec![fail(task, diagnostic)],
    }
}

/// Declared reads shared by all four frozen shells (no writes: Ack-only).
fn frozen_reads() -> Vec<FieldPath> {
    vec![
        FieldPath::new(StoreId::Tasks, "active.id"),
        FieldPath::new(StoreId::Tasks, "active.kind"),
        FieldPath::new(StoreId::Tasks, "active.payload"),
        FieldPath::new(StoreId::Tasks, "active.state"),
        FieldPath::new(StoreId::Tasks, "active.owner"),
        FieldPath::new(StoreId::Lex, "literals"),
        FieldPath::new(StoreId::Config, "limits"),
    ]
}

/// Frozen manifest helper: each shell claims exactly its `/37` kind
/// (Ack-only, zero writes, no allowlist rows).
fn frozen_manifest(
    id: crate::ids::ChipId,
    chip_name: &'static str,
    kind: TaskKind,
) -> ChipManifest {
    ChipManifest {
        id,
        chip_name,
        group: TaskGroup::CONSTANT_LAYOUT_INIT,
        task_kinds: vec![kind],
        reads: frozen_reads(),
        writes: vec![],
        capability: Capability::Emulable,
        backend_class: BackendClass::CpuReference,
        phase: ChipPhase::Propagation,
        deterministic: true,
        tests: vec!["compiler/tests/c37_const_branch.rs"],
        dependencies: vec![],
    }
}

/// The CL04 `&&` narrow worker (frozen; fixed [`BranchOp::And`]).
pub struct BranchAndChip;

/// The CL04 `||` narrow worker (frozen; fixed [`BranchOp::Or`]).
pub struct BranchOrChip;

/// The CL04 `?:` narrow worker (frozen; fixed [`BranchOp::Cond`]).
pub struct BranchCondChip;

/// The CL07 static-assert narrow worker (frozen).
pub struct StaticAssertChip;

impl Worker for BranchAndChip {
    fn manifest(&self) -> ChipManifest {
        frozen_manifest(CL04_AND_CHIP, "BranchAndChip", CL04_AND_TASK_KIND)
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_branch_input(bus, task, BranchOp::And) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        Self::compute(&input)
    }
}

impl Worker for BranchOrChip {
    fn manifest(&self) -> ChipManifest {
        frozen_manifest(CL04_OR_CHIP, "BranchOrChip", CL04_OR_TASK_KIND)
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_branch_input(bus, task, BranchOp::Or) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        Self::compute(&input)
    }
}

impl Worker for BranchCondChip {
    fn manifest(&self) -> ChipManifest {
        frozen_manifest(CL04_COND_CHIP, "BranchCondChip", CL04_COND_TASK_KIND)
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_branch_input(bus, task, BranchOp::Cond) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        Self::compute(&input)
    }
}

impl Worker for StaticAssertChip {
    fn manifest(&self) -> ChipManifest {
        frozen_manifest(CL07_ASSERT_CHIP, "StaticAssertChip", CL07_ASSERT_TASK_KIND)
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_assert_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        Self::compute(&input)
    }
}

impl BranchAndChip {
    /// Pure `&&` certification over the narrow projection (no bus access).
    pub fn compute(input: &BranchInput) -> Vec<Proposal> {
        ack_or_fail(input.task, eval_branch(input).map(|_| ()))
    }
}

impl BranchOrChip {
    /// Pure `||` certification over the narrow projection (no bus access).
    pub fn compute(input: &BranchInput) -> Vec<Proposal> {
        ack_or_fail(input.task, eval_branch(input).map(|_| ()))
    }
}

impl BranchCondChip {
    /// Pure `?:` certification over the narrow projection (no bus access).
    pub fn compute(input: &BranchInput) -> Vec<Proposal> {
        ack_or_fail(input.task, eval_branch(input).map(|_| ()))
    }
}

impl StaticAssertChip {
    /// Pure assert check over the narrow projection (no bus access).
    pub fn compute(input: &AssertInput) -> Vec<Proposal> {
        ack_or_fail(input.task, eval_assert(input))
    }
}

// ---------------------------------------------------------------------------
// Chip-local verbatim copies of the `fold.rs` gates (same bodies, same
// messages, same codes). Copied rather than imported per the narrow scope:
// the branch/assert narrows must not couple to the fold chip's module.
// ---------------------------------------------------------------------------

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
fn const_bits_required(magnitude: &[u8]) -> u32 {
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
        format!("fold-branch reads missing {what} {index}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bus::CompilerBus;
    use crate::manifest::StoreSchema;
    use crate::task::{Payload, TaskDraft, TaskKindRegistry};

    /// An in-subset M1 literal fixture with the given magnitude bytes.
    fn subset_literal(magnitude: Vec<u8>) -> LiteralRecord {
        LiteralRecord {
            token: None,
            kind: LiteralKind::Integer,
            radix: 10,
            suffix: LiteralSuffix::None,
            value: magnitude,
            negative: false,
            spelling: vec![],
            candidate_type: Lx08CandidateType::Int,
        }
    }

    fn operand(id: u32, body: Option<LiteralRecord>) -> BranchOperand {
        BranchOperand {
            id: LiteralId::from_index(id),
            body,
        }
    }

    fn present(id: u32, magnitude: Vec<u8>) -> BranchOperand {
        operand(id, Some(subset_literal(magnitude)))
    }

    fn branch_input(
        op: BranchOp,
        cond: BranchOperand,
        lhs: BranchOperand,
        rhs: BranchOperand,
        max_const_bits: u32,
    ) -> BranchInput {
        BranchInput {
            task: TaskId::from_index(0),
            op,
            state: TaskState::Running,
            cond,
            lhs,
            rhs,
            max_const_bits,
        }
    }

    /// `&&`/`||` input: `lhs` doubles as the condition.
    fn logical_input(op: BranchOp, lhs: Vec<u8>, rhs_body: Option<LiteralRecord>) -> BranchInput {
        let lhs_operand = present(0, lhs);
        branch_input(
            op,
            lhs_operand.clone(),
            lhs_operand,
            operand(1, rhs_body),
            128,
        )
    }

    fn assert_input(cond: AssertCond) -> AssertInput {
        AssertInput {
            task: TaskId::from_index(0),
            state: TaskState::Running,
            cond,
            max_const_bits: 128,
        }
    }

    fn unsupported_out_of_subset() -> LiteralRecord {
        let mut record = subset_literal(vec![3]);
        record.radix = 16;
        record
    }

    #[test]
    fn and_true_yields_canonical_one() {
        let input = logical_input(BranchOp::And, vec![2], Some(subset_literal(vec![3])));
        assert_eq!(eval_branch(&input), Ok(BranchValue(vec![1])));
    }

    #[test]
    fn and_false_short_circuits_unselected_subset_fault() {
        // `0 && <radix-16>`: the unselected rhs is never gated.
        let input = logical_input(BranchOp::And, vec![0], Some(unsupported_out_of_subset()));
        assert_eq!(eval_branch(&input), Ok(BranchValue(vec![0])));
    }

    #[test]
    fn and_false_short_circuits_dangling_rhs() {
        let input = logical_input(BranchOp::And, vec![0], None);
        assert_eq!(eval_branch(&input), Ok(BranchValue(vec![0])));
    }

    #[test]
    fn and_true_selected_rhs_out_of_subset_is_unsupported() {
        let input = logical_input(BranchOp::And, vec![2], Some(unsupported_out_of_subset()));
        let error = eval_branch(&input).expect_err("selected bad rhs must fail");
        assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Unsupported, 1));
    }

    #[test]
    fn and_true_dangling_rhs_is_missing() {
        let input = logical_input(BranchOp::And, vec![2], None);
        let error = eval_branch(&input).expect_err("dangling selected rhs must fail");
        assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Task, 4));
    }

    #[test]
    fn or_true_short_circuits_unselected_subset_fault() {
        let input = logical_input(BranchOp::Or, vec![3], Some(unsupported_out_of_subset()));
        assert_eq!(eval_branch(&input), Ok(BranchValue(vec![1])));
    }

    #[test]
    fn or_false_evaluates_rhs() {
        let input = logical_input(BranchOp::Or, vec![0], Some(subset_literal(vec![3])));
        assert_eq!(eval_branch(&input), Ok(BranchValue(vec![1])));
        let input = logical_input(BranchOp::Or, vec![0], Some(subset_literal(vec![0])));
        assert_eq!(eval_branch(&input), Ok(BranchValue(vec![0])));
    }

    #[test]
    fn or_false_selected_rhs_out_of_subset_is_unsupported() {
        let input = logical_input(BranchOp::Or, vec![0], Some(unsupported_out_of_subset()));
        let error = eval_branch(&input).expect_err("selected bad rhs must fail");
        assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Unsupported, 1));
    }

    #[test]
    fn cond_passes_selected_magnitude_through() {
        let input = branch_input(
            BranchOp::Cond,
            present(0, vec![2]),
            present(1, vec![2]),
            present(2, vec![3]),
            128,
        );
        assert_eq!(eval_branch(&input), Ok(BranchValue(vec![2])));
        let input = branch_input(
            BranchOp::Cond,
            present(0, vec![0]),
            present(1, vec![2]),
            present(2, vec![3]),
            128,
        );
        assert_eq!(eval_branch(&input), Ok(BranchValue(vec![3])));
    }

    #[test]
    fn cond_false_ignores_unselected_then_branch() {
        // `0 ? <radix-16> : 3`: the unselected `then` is never gated.
        let input = branch_input(
            BranchOp::Cond,
            present(0, vec![0]),
            operand(1, Some(unsupported_out_of_subset())),
            present(2, vec![3]),
            128,
        );
        assert_eq!(eval_branch(&input), Ok(BranchValue(vec![3])));
    }

    #[test]
    fn cond_out_of_subset_condition_is_unsupported() {
        let input = branch_input(
            BranchOp::Cond,
            operand(0, Some(unsupported_out_of_subset())),
            present(1, vec![2]),
            present(2, vec![3]),
            128,
        );
        let error = eval_branch(&input).expect_err("bad cond must fail");
        assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Unsupported, 1));
    }

    #[test]
    fn branch_over_budget_result_is_const_overflow() {
        // Canonical `1` needs one bit; a zero budget cannot hold it.
        let input = logical_input(BranchOp::And, vec![2], Some(subset_literal(vec![3])));
        let input = BranchInput {
            max_const_bits: 0,
            ..input
        };
        let error = eval_branch(&input).expect_err("over-budget result must fail");
        assert_eq!(error.code, DiagnosticCode::CONST_OVERFLOW);
    }

    #[test]
    fn branch_over_budget_condition_is_const_overflow() {
        // Magnitude `2` needs two bits; a one-bit budget cannot hold it.
        let input = logical_input(BranchOp::And, vec![2], Some(subset_literal(vec![3])));
        let input = BranchInput {
            max_const_bits: 1,
            ..input
        };
        let error = eval_branch(&input).expect_err("over-budget cond must fail");
        assert_eq!(error.code, DiagnosticCode::CONST_OVERFLOW);
    }

    #[test]
    fn branch_not_running_fails() {
        let mut input = logical_input(BranchOp::And, vec![2], Some(subset_literal(vec![3])));
        input.state = TaskState::Ready;
        let error = eval_branch(&input).expect_err("non-running task must fail");
        assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Task, 4));
    }

    #[test]
    fn assert_nonzero_passes_and_zero_fails() {
        let input = assert_input(AssertCond::Literal(present(0, vec![3])));
        assert_eq!(eval_assert(&input), Ok(()));
        let input = assert_input(AssertCond::Literal(present(0, vec![0])));
        let error = eval_assert(&input).expect_err("zero assert must fail");
        assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Task, 3));
    }

    #[test]
    fn assert_non_constant_is_not_constant_expression() {
        let input = assert_input(AssertCond::NotConstant);
        let error = eval_assert(&input).expect_err("non-ICE assert must fail");
        assert_eq!(error.code, DiagnosticCode::CONST_NOT_CONSTANT_EXPRESSION);
    }

    #[test]
    fn assert_out_of_subset_is_unsupported_and_over_budget_is_overflow() {
        let input = assert_input(AssertCond::Literal(operand(
            0,
            Some(unsupported_out_of_subset()),
        )));
        let error = eval_assert(&input).expect_err("bad assert literal must fail");
        assert_eq!(error.code, DiagnosticCode::new(DiagGroup::Unsupported, 1));
        let input = AssertInput {
            max_const_bits: 1,
            ..assert_input(AssertCond::Literal(present(0, vec![2])))
        };
        let error = eval_assert(&input).expect_err("over-budget assert must fail");
        assert_eq!(error.code, DiagnosticCode::CONST_OVERFLOW);
    }

    #[test]
    fn shells_complete_ack_or_fail() {
        let ok = logical_input(BranchOp::And, vec![0], Some(unsupported_out_of_subset()));
        assert!(matches!(
            BranchAndChip::compute(&ok).as_slice(),
            [Proposal::Complete {
                value: ResultValue::Ack,
                ..
            }]
        ));
        let bad = logical_input(BranchOp::And, vec![2], Some(unsupported_out_of_subset()));
        assert!(matches!(
            BranchAndChip::compute(&bad).as_slice(),
            [Proposal::Fail { .. }]
        ));
        let ok = logical_input(BranchOp::Or, vec![3], Some(unsupported_out_of_subset()));
        assert!(matches!(
            BranchOrChip::compute(&ok).as_slice(),
            [Proposal::Complete {
                value: ResultValue::Ack,
                ..
            }]
        ));
        let bad = logical_input(BranchOp::Or, vec![0], Some(unsupported_out_of_subset()));
        assert!(matches!(
            BranchOrChip::compute(&bad).as_slice(),
            [Proposal::Fail { .. }]
        ));
        let ok = branch_input(
            BranchOp::Cond,
            present(0, vec![0]),
            operand(1, Some(unsupported_out_of_subset())),
            present(2, vec![3]),
            128,
        );
        assert!(matches!(
            BranchCondChip::compute(&ok).as_slice(),
            [Proposal::Complete {
                value: ResultValue::Ack,
                ..
            }]
        ));
        let pass = assert_input(AssertCond::Literal(present(0, vec![2])));
        assert!(matches!(
            StaticAssertChip::compute(&pass).as_slice(),
            [Proposal::Complete {
                value: ResultValue::Ack,
                ..
            }]
        ));
        let failed = assert_input(AssertCond::Literal(present(0, vec![0])));
        assert!(matches!(
            StaticAssertChip::compute(&failed).as_slice(),
            [Proposal::Fail { .. }]
        ));
    }

    /// Seed two in-subset literals (`2`, `0`) on a default bus.
    fn seeded_bus() -> (CompilerBus, LiteralId, LiteralId) {
        let mut bus = CompilerBus::default();
        bus.kinds = TaskKindRegistry::const_branch_slice();
        bus.schema = StoreSchema::m1_slice();
        let limits = bus.limits();
        let two = bus
            .arenas
            .literals
            .alloc(subset_literal(vec![2]), &limits)
            .expect("seed literal 2");
        let zero = bus
            .arenas
            .literals
            .alloc(subset_literal(vec![0]), &limits)
            .expect("seed literal 0");
        (bus, two, zero)
    }

    fn frozen_task(bus: &mut CompilerBus, refs: Vec<RecordRef>) -> TaskId {
        let id = bus
            .bootstrap_task(TaskDraft {
                kind: CL04_AND_TASK_KIND,
                payload: Payload::from_refs(refs),
                owner: CL04_AND_CHIP,
                parent: None,
                continuation: None,
            })
            .expect("bootstrap frozen task");
        bus.arenas.tasks.get_mut(id).expect("task").state = TaskState::Running;
        id
    }

    #[test]
    fn projector_decodes_branch_shapes_and_rejects_others() {
        let (mut bus, two, zero) = seeded_bus();
        let id = frozen_task(
            &mut bus,
            vec![
                RecordRef::Literal(zero),
                RecordRef::Literal(two),
                RecordRef::Literal(two),
            ],
        );
        let input = project_branch_input(&bus, id, BranchOp::Cond).expect("cond projects");
        assert_eq!(input.op, BranchOp::Cond);
        assert_eq!(input.cond.id, zero);
        let id = frozen_task(&mut bus, vec![RecordRef::Literal(two)]);
        assert!(project_branch_input(&bus, id, BranchOp::And).is_err());
        let id = frozen_task(
            &mut bus,
            vec![
                RecordRef::Literal(two),
                RecordRef::Node(crate::ids::NodeId::from_index(999)),
            ],
        );
        assert!(project_branch_input(&bus, id, BranchOp::And).is_err());
    }

    #[test]
    fn projector_routes_assert_literal_vs_non_constant() {
        let (mut bus, two, _) = seeded_bus();
        let id = frozen_task(&mut bus, vec![RecordRef::Literal(two)]);
        let input = project_assert_input(&bus, id).expect("assert projects");
        assert!(matches!(input.cond, AssertCond::Literal(_)));
        let id = frozen_task(
            &mut bus,
            vec![RecordRef::Node(crate::ids::NodeId::from_index(999))],
        );
        let input = project_assert_input(&bus, id).expect("non-literal projects");
        assert!(matches!(input.cond, AssertCond::NotConstant));
        let id = frozen_task(&mut bus, vec![]);
        let input = project_assert_input(&bus, id).expect("empty projects");
        assert!(matches!(input.cond, AssertCond::NotConstant));
    }
}
