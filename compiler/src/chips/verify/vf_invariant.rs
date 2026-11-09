// ============================================================================
// chips/verify/vf_invariant.rs — T13 VF06 typed-invariant verifier
// (Wave 2 slice 5, `/14`; four-kind checked set at `/40`; `/55` calls)
//
// Reads one committed TU node, walks the committed tree, and checks the
// checked facts: each checked node needs exactly one committed `SemRecord`
// with `NonLvalue`, zero effects (except `Assignment`, which records the
// local write), and its expected type. Expression/statement kinds expect the
// canonical `int`; each `FunctionDefinition` expects the committed function
// signature matching its parameter-list arity. Parameter nodes carry no
// semantic fact, and assignment LHS identifiers and call callee identifiers
// are skipped. The TU must contain 1–32 `FunctionDefinition` nodes; every
// committed function signature must be used by a definition. Completes
// `Ack`; any gap fails loudly. Runs after committed `SemRecord`s, before T09.
// ============================================================================

use crate::bus::{EffectMask, NodeKind, TypeKind, ValueCategory, EFFECT_WRITES_LOCAL};
use crate::chips::{fail, protocol_fault, Worker};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NodeId, RecordRef, TaskId};
use crate::limits::{CALL_MAX_FUNCTIONS, CALL_MAX_PARAMETERS};
use crate::manifest::{BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, VF06_CHIP};
use crate::task::{Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState};
use std::collections::{BTreeMap, BTreeSet};

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
    /// Committed function signatures by parameter count.
    pub signatures: BTreeMap<usize, crate::ids::TypeId>,
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
    // `/55` per-function signatures: every committed function type must be
    // a prototype taking canonical `int` parameters (at most eight) and
    // returning canonical `int`; the arity key must be unique. A function
    // definition is then matched to the signature of its parameter count.
    let mut signatures: BTreeMap<usize, crate::ids::TypeId> = BTreeMap::new();
    for (id, record) in bus.arenas.types.iter() {
        if let TypeKind::Function {
            result,
            params,
            prototype,
            variadic,
        } = &record.kind
        {
            if *result != int
                || !*prototype
                || *variadic
                || params.len() > CALL_MAX_PARAMETERS
                || params.iter().any(|param| *param != int)
            {
                return Err(DiagnosticDraft::unsupported(
                    "vf06 of a non-M1 function signature: exactly `int(int...)` required",
                ));
            }
            if signatures.insert(params.len(), id).is_some() {
                let message = if params.is_empty() {
                    "vf06 reads duplicate int(void) signature types"
                } else {
                    "vf06 reads duplicate function signature types"
                };
                return Err(DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    message,
                ));
            }
        }
    }
    if signatures.is_empty() {
        return Err(DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            "vf06 reads missing int(void) signature type",
        ));
    }
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
        signatures,
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
        // The `/55` TU owns 1–32 function definitions. VF05 remains the AST
        // authority, but VF06 must fail closed here rather than pass
        // vacuously when a node its lowering consumer needs is absent or
        // cannot be matched to a committed signature.
        let function_ids: Vec<NodeId> = input
            .tree
            .iter()
            .filter(|(_, node)| node.kind == NodeKind::FunctionDefinition)
            .map(|(id, _)| *id)
            .collect();
        if function_ids.is_empty() {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "vf06: missing function definition",
                ),
            )];
        }
        if function_ids.len() > CALL_MAX_FUNCTIONS {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "vf06: function definition count exceeds the budget",
                ),
            )];
        }
        let mut function_signatures: BTreeMap<NodeId, crate::ids::TypeId> = BTreeMap::new();
        let mut used_arities: BTreeSet<usize> = BTreeSet::new();
        for id in &function_ids {
            match function_arity(&input.tree, *id) {
                Ok(arity) => {
                    let Some(signature) = input.signatures.get(&arity).copied() else {
                        return vec![fail(
                            input.task,
                            DiagnosticDraft::error(
                                DiagnosticCode::new(DiagGroup::Task, 4),
                                format!("vf06: missing function signature for node {}", id.index()),
                            ),
                        )];
                    };
                    function_signatures.insert(*id, signature);
                    used_arities.insert(arity);
                }
                Err(detail) => {
                    // Preserve the `/40` rejection shape for an extra,
                    // malformed definition appended to an otherwise M1 TU;
                    // valid multi-function definitions take the path above.
                    if function_ids.len() > 1 {
                        return vec![fail(
                            input.task,
                            DiagnosticDraft::unsupported(
                                "multiple function definitions outside M1",
                            ),
                        )];
                    }
                    return vec![fail(
                        input.task,
                        DiagnosticDraft::error(
                            DiagnosticCode::new(DiagGroup::Task, 4),
                            format!(
                                "vf06: malformed function definition for node {} ({detail})",
                                id.index()
                            ),
                        ),
                    )];
                }
            }
        }
        for arity in input.signatures.keys() {
            if !used_arities.contains(arity) {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::unsupported(
                        "vf06: non-M1 function signature is not used by a definition",
                    ),
                )];
            }
        }
        let parameter_nodes: BTreeSet<NodeId> = input
            .tree
            .iter()
            .filter(|(_, node)| node.kind == NodeKind::Parameter)
            .map(|(id, _)| *id)
            .collect();
        for sem in &input.sems {
            if parameter_nodes.contains(&sem.node) {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!(
                            "vf06: unexpected typed fact for parameter node {}",
                            sem.node.index()
                        ),
                    ),
                )];
            }
        }
        let assignment_lhs: BTreeSet<crate::ids::NodeId> = input
            .tree
            .iter()
            .filter(|(_, node)| node.kind == NodeKind::Assignment)
            .filter_map(|(_, node)| node.children.first().copied())
            .collect();
        let call_callees: BTreeSet<crate::ids::NodeId> = input
            .tree
            .iter()
            .filter(|(_, node)| node.kind == NodeKind::Call)
            .filter_map(|(_, node)| node.children.first().copied())
            .collect();
        for (id, node) in &input.tree {
            if node.kind == NodeKind::Identifier
                && (assignment_lhs.contains(id) || call_callees.contains(id))
            {
                continue;
            }
            let Some(expected) = expected_type(input, node.kind, *id, &function_signatures) else {
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
            let expected_effects = if node.kind == NodeKind::Assignment {
                EffectMask(EFFECT_WRITES_LOCAL)
            } else {
                EffectMask(0)
            };
            if sem.effects != expected_effects {
                let detail = if node.kind == NodeKind::Assignment {
                    "wrong effect mask"
                } else {
                    "nonzero effect mask"
                };
                return vec![fail(
                    input.task,
                    DiagnosticDraft::error(
                        DiagnosticCode::new(DiagGroup::Task, 4),
                        format!("vf06: {detail} for node {}", id.index()),
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

/// Resolve one function definition's parameter count from its declarator.
fn function_arity(tree: &[(NodeId, crate::bus::NodeRecord)], id: NodeId) -> Result<usize, String> {
    let body = tree
        .iter()
        .find(|(nid, _)| *nid == id)
        .map(|(_, body)| body)
        .ok_or_else(|| "function definition is not listed".to_string())?;
    if body.children.len() != 3 {
        return Err("wrong function definition child count".to_string());
    }
    let declarator = tree
        .iter()
        .find(|(nid, _)| *nid == body.children[1])
        .map(|(_, body)| body)
        .ok_or_else(|| "function declarator is not listed".to_string())?;
    if declarator.kind != NodeKind::Declarator {
        return Err("function declarator has the wrong kind".to_string());
    }
    match declarator.children.len() {
        0 => Ok(0),
        1 => {
            let parameter_list = tree
                .iter()
                .find(|(nid, _)| *nid == declarator.children[0])
                .map(|(_, body)| body)
                .ok_or_else(|| "parameter list is not listed".to_string())?;
            if parameter_list.kind != NodeKind::ParameterList {
                return Err("function declarator child is not a parameter list".to_string());
            }
            if parameter_list.children.is_empty()
                || parameter_list.children.len() > CALL_MAX_PARAMETERS
            {
                return Err("parameter list count is outside the budget".to_string());
            }
            Ok(parameter_list.children.len())
        }
        _ => Err("function declarator has extra children".to_string()),
    }
}

/// Per-kind expected `SemRecord.ty`.
///
/// Expression/statement kinds require the canonical `int`; each
/// `FunctionDefinition` requires the signature matching its parameter count.
/// Parameters carry no typed fact.
fn expected_type(
    input: &Vf06Input,
    kind: NodeKind,
    node: NodeId,
    signatures: &BTreeMap<NodeId, crate::ids::TypeId>,
) -> Option<crate::ids::TypeId> {
    match kind {
        NodeKind::IntLiteral
        | NodeKind::Identifier
        | NodeKind::Return
        | NodeKind::BinaryAdd
        | NodeKind::BinarySub
        | NodeKind::BinaryMul
        | NodeKind::BinaryDiv
        | NodeKind::BinaryRem
        | NodeKind::UnaryPlus
        | NodeKind::UnaryMinus
        | NodeKind::Assignment
        | NodeKind::Call => Some(input.int),
        NodeKind::FunctionDefinition => signatures.get(&node).copied(),
        _ => None,
    }
}
