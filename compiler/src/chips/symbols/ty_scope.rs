// ============================================================================
// chips/ty_scope.rs — T06 TY01/TY02 scope workers (Wave 2 slice 4, `/13`)
//
// `TyScopeChip` serves `symbol_type.scope_enter` (payload: exactly one
// committed node — the TU root for file scope, a `Block` node for a body
// scope; appends one `Scope` plus one `ScopeEvent(Enter)`; completes
// `Record`) and `symbol_type.scope_exit` (payload: exactly one committed
// scope; appends one `ScopeEvent(Exit)`; completes `Ack`). Guards: exactly
// one file scope (a second file `Enter` fails), one `Enter` per scope, an
// `Exit` needs a prior `Enter` with no earlier `Exit`, and a file scope
// never exits. Scopes are identified by owner node, never by
// `(parent, kind)`.
// ============================================================================

use crate::chips::{fail, protocol_fault, Worker};
use crate::bus::{NodeKind, ScopeEventKind, ScopeKind, ScopeRecord};
use crate::diagnostic::{DiagGroup, DiagnosticCode, DiagnosticDraft};
use crate::ids::{NodeId, RecordRef, ScopeId, TaskId};
use crate::manifest::{
    BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, TY_SCOPE_CHIP,
};
use crate::records::{G1DraftBody, RecordDraft};
use crate::task::{
    AppendBatch, DraftRef, Proposal, ResultValue, StoreId, TaskGroup, TaskKind, TaskState,
};

/// Narrow projection for the scope-enter computation.
#[derive(Clone, Debug)]
pub struct TyScopeEnterInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Anchor node (committed body).
    pub node: NodeId,
    /// Its kind.
    pub node_kind: NodeKind,
    /// Committed file scopes (normally zero or one).
    pub file_scopes: Vec<ScopeId>,
    /// Committed scopes owned by the anchor node.
    pub owner_scopes: Vec<ScopeId>,
    /// `scopes` arena count at dispatch (prediction base).
    pub scopes_allocated: u32,
    /// `scope_events` arena count at dispatch (prediction base).
    pub events_allocated: u32,
}

/// Build the enter projection for one dispatched task.
pub fn project_ty_scope_enter_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<TyScopeEnterInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("scope-enter of unknown task {}", task.index())))?;
    if record.kind != TaskKind::SYMBOL_SCOPE_ENTER {
        return Err(protocol_fault(format!(
            "scope-enter task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "scope-enter task {} payload must carry exactly one node",
            task.index()
        )));
    }
    let node = match record.payload.refs[0] {
        RecordRef::Node(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "scope-enter task {} payload must be a node",
                task.index()
            )));
        }
    };
    let node_kind = bus
        .arenas
        .nodes
        .get(node)
        .map(|body| body.kind)
        .map_err(|_| {
            DiagnosticDraft::error(
                DiagnosticCode::new(DiagGroup::Task, 4),
                format!("scope-enter reads missing node {}", node.index()),
            )
        })?;
    let mut file_scopes = Vec::new();
    let mut owner_scopes = Vec::new();
    for (id, scope) in bus.arenas.scopes.iter() {
        if scope.kind == ScopeKind::File {
            file_scopes.push(id);
        }
        if scope.owner == Some(node) {
            owner_scopes.push(id);
        }
    }
    Ok(TyScopeEnterInput {
        task,
        state: record.state.clone(),
        node,
        node_kind,
        file_scopes,
        owner_scopes,
        scopes_allocated: bus.arenas.scopes.allocated(),
        events_allocated: bus.arenas.scope_events.allocated(),
    })
}

/// Narrow projection for the scope-exit computation.
#[derive(Clone, Debug)]
pub struct TyScopeExitInput {
    /// Dispatched task.
    pub task: TaskId,
    /// Lifecycle state at dispatch (must be `Running`).
    pub state: TaskState,
    /// Scope under exit (committed body).
    pub scope: ScopeId,
    /// Its record.
    pub body: ScopeRecord,
    /// Whether an `Enter` event is committed for the scope.
    pub entered: bool,
    /// Whether an `Exit` event is already committed.
    pub exited: bool,
}

/// Build the exit projection for one dispatched task.
pub fn project_ty_scope_exit_input(
    bus: &crate::bus::CompilerBus,
    task: TaskId,
) -> Result<TyScopeExitInput, DiagnosticDraft> {
    let record = bus
        .arenas
        .tasks
        .get(task)
        .map_err(|_| protocol_fault(format!("scope-exit of unknown task {}", task.index())))?;
    if record.kind != TaskKind::SYMBOL_SCOPE_EXIT {
        return Err(protocol_fault(format!(
            "scope-exit task {} has unexpected kind {}",
            task.index(),
            record.kind.raw()
        )));
    }
    if record.payload.refs.len() != 1 {
        return Err(protocol_fault(format!(
            "scope-exit task {} payload must carry exactly one scope",
            task.index()
        )));
    }
    let scope = match record.payload.refs[0] {
        RecordRef::Scope(id) => id,
        _ => {
            return Err(protocol_fault(format!(
                "scope-exit task {} payload must be a scope",
                task.index()
            )));
        }
    };
    let body = bus.arenas.scopes.get(scope).map_err(|_| {
        DiagnosticDraft::error(
            DiagnosticCode::new(DiagGroup::Task, 4),
            format!("scope-exit reads missing scope {}", scope.index()),
        )
    })?;
    let mut entered = false;
    let mut exited = false;
    for (_, event) in bus.arenas.scope_events.iter() {
        if event.scope == scope {
            match event.kind {
                ScopeEventKind::Enter => entered = true,
                ScopeEventKind::Exit => exited = true,
            }
        }
    }
    Ok(TyScopeExitInput {
        task,
        state: record.state.clone(),
        scope,
        body: body.clone(),
        entered,
        exited,
    })
}

/// The T06 scope worker (TY01/TY02 slice scope).
pub struct TyScopeChip;

impl Worker for TyScopeChip {
    fn manifest(&self) -> ChipManifest {
        ChipManifest {
            id: TY_SCOPE_CHIP,
            chip_name: "TyScopeChip",
            group: TaskGroup::SYMBOL_TYPE,
            task_kinds: vec![TaskKind::SYMBOL_SCOPE_ENTER, TaskKind::SYMBOL_SCOPE_EXIT],
            reads: vec![
                FieldPath::new(StoreId::Tasks, "active.id"),
                FieldPath::new(StoreId::Tasks, "active.kind"),
                FieldPath::new(StoreId::Tasks, "active.payload"),
                FieldPath::new(StoreId::Tasks, "active.state"),
                FieldPath::new(StoreId::Tasks, "active.owner"),
                FieldPath::new(StoreId::Parse, "nodes"),
                FieldPath::new(StoreId::Symbols, "scopes"),
                FieldPath::new(StoreId::Symbols, "scope_events"),
            ],
            writes: vec![
                FieldPath::new(StoreId::Symbols, "scopes"),
                FieldPath::new(StoreId::Symbols, "scope_events"),
            ],
            capability: Capability::Emulable,
            backend_class: BackendClass::CpuReference,
            phase: ChipPhase::Propagation,
            deterministic: true,
            tests: vec!["compiler/tests/c13_ty.rs"],
            dependencies: vec![],
        }
    }

    fn handle(&self, task: TaskId, bus: &crate::bus::CompilerBus) -> Vec<Proposal> {
        let kind = match bus.arenas.tasks.get(task) {
            Ok(record) => record.kind,
            Err(_) => {
                return vec![fail(
                    task,
                    protocol_fault(format!("scope of unknown task {}", task.index())),
                )];
            }
        };
        match kind {
            TaskKind::SYMBOL_SCOPE_ENTER => match project_ty_scope_enter_input(bus, task) {
                Ok(input) => self.compute_enter(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            TaskKind::SYMBOL_SCOPE_EXIT => match project_ty_scope_exit_input(bus, task) {
                Ok(input) => self.compute_exit(&input),
                Err(diagnostic) => vec![fail(task, diagnostic)],
            },
            _ => vec![fail(
                task,
                protocol_fault(format!("scope task {} has unexpected kind", task.index())),
            )],
        }
    }
}

impl TyScopeChip {
    /// Enter computation: one `Scope` plus one `Enter` event, guarded once.
    pub fn compute_enter(&self, input: &TyScopeEnterInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("scope-enter task {} is not running", input.task.index()),
                ),
            )];
        }
        let (kind, parent, owner) = match input.node_kind {
            NodeKind::TranslationUnit => {
                if !input.file_scopes.is_empty() {
                    return vec![fail(
                        input.task,
                        DiagnosticDraft::error(
                            DiagnosticCode::new(DiagGroup::Task, 4),
                            "duplicate file-scope enter",
                        ),
                    )];
                }
                (ScopeKind::File, None, None)
            }
            NodeKind::Compound => {
                let Some(file) = input.file_scopes.first().copied() else {
                    return vec![fail(
                        input.task,
                        DiagnosticDraft::error(
                            DiagnosticCode::new(DiagGroup::Task, 4),
                            "block scope entered before any file scope",
                        ),
                    )];
                };
                if !input.owner_scopes.is_empty() {
                    return vec![fail(
                        input.task,
                        DiagnosticDraft::error(
                            DiagnosticCode::new(DiagGroup::Task, 4),
                            "duplicate block-scope enter for this owner",
                        ),
                    )];
                }
                (ScopeKind::Block, Some(file), Some(input.node))
            }
            _ => {
                return vec![fail(
                    input.task,
                    DiagnosticDraft::unsupported("scope enter for a non-TU non-block node"),
                )];
            }
        };
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![
                        RecordDraft {
                            family: crate::ids::RecordFamily::Scope,
                            index: DraftRef(0),
                        },
                        RecordDraft {
                            family: crate::ids::RecordFamily::ScopeEvent,
                            index: DraftRef(1),
                        },
                    ],
                    bodies: vec![
                        G1DraftBody::Scope(crate::bus::ScopeRecord {
                            kind,
                            parent,
                            owner,
                        }),
                        G1DraftBody::ScopeEvent(crate::bus::ScopeEventRecord {
                            scope: ScopeId::from_index(input.scopes_allocated),
                            kind: ScopeEventKind::Enter,
                            at: input.node,
                        }),
                    ],
                },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Record(RecordRef::Scope(ScopeId::from_index(
                    input.scopes_allocated,
                ))),
            },
        ]
    }

    /// Exit computation: one `Exit` event after a committed `Enter`, at most once.
    pub fn compute_exit(&self, input: &TyScopeExitInput) -> Vec<Proposal> {
        if input.state != TaskState::Running {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    format!("scope-exit task {} is not running", input.task.index()),
                ),
            )];
        }
        if input.body.kind == ScopeKind::File {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "file scope never exits in M1",
                ),
            )];
        }
        if !input.entered {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "scope exit without a committed enter",
                ),
            )];
        }
        if input.exited {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "duplicate scope exit",
                ),
            )];
        }
        let Some(owner) = input.body.owner else {
            return vec![fail(
                input.task,
                DiagnosticDraft::error(
                    DiagnosticCode::new(DiagGroup::Task, 4),
                    "exiting a scope with no owner node",
                ),
            )];
        };
        vec![
            Proposal::AppendRecords {
                task: input.task,
                batch: AppendBatch {
                    records: vec![RecordDraft {
                        family: crate::ids::RecordFamily::ScopeEvent,
                        index: DraftRef(0),
                    }],
                    bodies: vec![G1DraftBody::ScopeEvent(crate::bus::ScopeEventRecord {
                        scope: input.scope,
                        kind: ScopeEventKind::Exit,
                        at: owner,
                    })],
                },
            },
            Proposal::Complete {
                task: input.task,
                value: ResultValue::Ack,
            },
        ]
    }
}
