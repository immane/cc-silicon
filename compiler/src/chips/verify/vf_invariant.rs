// ============================================================================
// chips/verify/vf_invariant.rs — T13 VF06 typed-invariant verifier
// (Wave 2 slice 5, `/14`; four-kind checked set at `/40`)
//
// Reads one committed TU node, walks the committed tree, and checks the M1
// checked set (`IntLiteral`, `BinaryAdd`, `Return`, `FunctionDefinition`):
// each needs exactly one committed `SemRecord` with `NonLvalue` and
// `EffectMask(0)`. The three expression/statement kinds expect the canonical
// `int` type; the function definition expects the unique committed M1
// `int(void)` signature `TypeId` (`Vf06Input.signature`), which the
// projection resolves mechanically from the committed `TypeKind::Function`
// records. The TU must also contain exactly one `FunctionDefinition` node:
// a verifier that skips the lowering consumer's node must not pass
// vacuously. Required-conversion completeness is M1-minimal (identity needs
// no plan, so there is nothing to miss; non-identity conversions are
// chip-level `Unsupported` elsewhere, never VF06 failures). Completes `Ack`;
// any gap fails loudly. Runs after committed `SemRecord`s, before T09.
// ============================================================================

use crate::bus::{EffectMask, NodeKind, TypeKind, ValueCategory};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NodeId, RecordRef, TaskId};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, VF06_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};

/// Narrow projection for the typed-invariant check.
#[derive(Clone, Debug)]
pub struct Vf06Input {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// TU root.
    pub tu: NodeId,
    /// Canonical `int` type.
    pub int: crate::ids::TypeId,
    /// The unique committed M1 `int(void)` signature `TypeId`.
    pub signature: crate::ids::TypeId,
    /// Reachable nodes in pre-order (committed bodies).
    pub tree: Vec<(NodeId, crate::bus::NodeRecord)>,
    /// Committed `SemRecord`s.
    pub sems: Vec<crate::bus::SemRecord>,
}

/// Build the narrow projection for one dispatched task.
pub fn project_vf06_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<Vf06Input, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("vf06 of unknown task {}", task.index())))?;
    if record.kind != TaskKind::VERIFICATION_TYPED_INVARIANT {
        return Err(protocol_fault(format!(
            "vf06 task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "vf06 task {} payload must carry exactly one TU node",
            task.index()
        )));
    }
    let tu = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "vf06 task {} payload must be a node",
                task.index()
            )));
        }
    };
    let tu_body = bus.arenas.nodes.get(tu).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("vf06 reads missing TU node {}", tu.index()),
        )
    })?;
    if tu_body.kind != NodeKind::TranslationUnit {
        return Err(DiagnosticDraft::unsupported("vf06 of a non-TU root"));
    }
    // Canonical `int` must be committed (the only M1 type).
    let mut int = None;
    for (id, record) in bus.arenas.types.iter() {
        if record.kind
            == (crate::bus::TypeKind::Int {
                rank: crate::bus::IntRank::Int,
                signed: true,
            })
        {
            int = Some(id);
            break;
        }
    }
    let Some(int) = int else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "vf06 reads missing canonical int type",
        ));
    };
    // The unique committed M1 `int(void)` signature, resolved mechanically
    // from the committed `Types` (the shape was already checked by TY17 /
    // SE29; VF06 only needs the `TypeId` to compare against the function
    // fact). Zero function types is missing input; any non-M1 function shape
    // is unsupported; duplicate M1 signatures are an ambiguous handoff.
    let mut signature = None;
    for (id, record) in bus.arenas.types.iter() {
        if let TypeKind::Function {
            result,
            params,
            prototype,
            variadic,
        } = &record.kind
        {
            if *result == int && params.is_empty() && *prototype && !*variadic {
                if signature.is_some() {
                    return Err(DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        "vf06 reads duplicate int(void) signature types",
                    ));
                }
                signature = Some(id);
            } else {
                return Err(DiagnosticDraft::unsupported(
                    "vf06 of a non-M1 function signature: exactly `int(void)` required",
                ));
            }
        }
    }
    let Some(signature) = signature else {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "vf06 reads missing int(void) signature type",
        ));
    };
    // Pre-order walk over committed children (deterministic).
    let mut tree = Vec::new();
    let mut stack = vec![tu];
    while let Some(id) = stack.pop() {
        let body = bus.arenas.nodes.get(id).map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("vf06 reads missing node {}", id.index()),
            )
        })?;
        for child in body.children.iter().rev() {
            stack.push(*child);
        }
        tree.push((id, body.clone()));
    }
    let sems = bus
        .arenas
        .sem
        .iter()
        .map(|(_, record)| record.clone())
        .collect();
    Ok(Vf06Input {
        task,
        state: record.state.clone(),
        tu,
        int,
        signature,
        tree,
        sems,
    })
}

/// The VF06 typed-invariant verifier (M1 scope).
pub struct Vf06Chip;

impl Worker for Vf06Chip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: VF06_CHIP,
            chip_name: "Vf06Chip",
            group: TaskGroup::VERIFICATION,
            task_kinds: vec![TaskKind::VERIFICATION_TYPED_INVARIANT],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Parse, "nodes"),
                FieldPath::new(StoreId::Types, "records"),
                FieldPath::new(StoreId::Sem, "records"),
            ],
            writes: vec![],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c14_se.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let input = match project_vf06_input(bus, task) {
            Ok(input) => input,
            Err(diagnostic) => return vec![fail(task, diagnostic)],
        };
        self.compute(&input)
    }
}

impl Vf06Chip {
    /// Pure semantic computation over the narrow projection (no bus access).
    pub fn compute(&self, input: &Vf06Input) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("vf06 task {} is not running", input.task.index()),
                ),
            )];
        }
        // The M1 TU owns exactly one function definition. VF05 remains the
        // AST authority, but VF06 must fail closed here rather than pass
        // vacuously when the node its lowering consumer needs is.
        let function_definitions = input
            .tree
            .iter()
            .filter(|(_, node)| node.kind == NodeKind::FunctionDefinition)
            .count();
        if function_definitions == 0 {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "vf06: missing function definition",
                ),
            )];
        }
        if function_definitions > 1 {
            return vec![fail(
                input.task,
                DiagnosticDraft::unsupported("multiple function definitions outside M1"),
            )];
        }
        for (id, node) in &input.tree {
            let Some(expected) = expected_type(input, node.kind) else {
                continue;
            };
            let mut matches = input.sems.iter().filter(|sem| sem.node == *id);
            let Some(sem) = matches.next() else {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("vf06: missing typed fact for node {}", id.index()),
                    ),
                )];
            };
            if matches.next().is_some() {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("vf06: duplicate typed fact for node {}", id.index()),
                    ),
                )];
            }
            if sem.ty != expected {
                let what = if node.kind == NodeKind::FunctionDefinition {
                    "non-signature type"
                } else {
                    "non-int type"
                };
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("vf06: {what} for node {}", id.index()),
                    ),
                )];
            }
            if sem.category != ValueCategory::NonLvalue {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("vf06: bad value category for node {}", id.index()),
                    ),
                )];
            }
            if sem.effects != EffectMask(0) {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("vf06: nonzero effect mask for node {}", id.index()),
                    ),
                )];
            }
        }
        vec![Proposal::Complete {
            task: input.task,
            value: ResultValue::Ack,
        }]
    }
}

/// Per-kind expected `SemRecord.ty` for the four-kind M1 checked set.
///
/// The expression/statement kinds keep the frozen `/14` canonical `int`
/// requirement; the function definition expects the unique committed M1
/// `int(void)` signature.
fn expected_type(input: &Vf06Input, kind: NodeKind) -> Option<crate::ids::TypeId> {
    match kind {
        NodeKind::IntLiteral | NodeKind::BinaryAdd | NodeKind::Return => Some(input.int),
        NodeKind::FunctionDefinition => Some(input.signature),
        _ => None,
    }
}
