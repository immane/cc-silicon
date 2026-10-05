// ============================================================================
// c13_ty.rs — Wave 2 (`/13`) TY scope/symbol/type slice acceptance.
//
// Covers the frozen closure: canonical `int` reuse (lowest-match scan),
// `int(void)` single production, file/body scope enter with once-guards,
// scope-exit lifecycle, `main` declaration with duplicate rejection,
// innermost-first lookup with a typed miss, identity-only conversions,
// kind-to-stage assignment, the TY store-owner allowlist rows, stage/layer
// agreement, snapshot bodies, tick-lifecycle integration, and the `/13`
// hash participation. The automatic File-Enter edge stays deferred: tests
// dispatch scope tasks after the TU commit (ordering holds, exactly-once is
// worker-guarded); T06 owns no Host import.
// ============================================================================

use cc_silicon_compiler::bus::{
    CharKind, CompilerBus, CompilerPins, IntRank, Linkage, PpTokenKind, PpTokenRecord, SpanRecord,
    StorageDuration, SymbolKind, TypeKind, TypeRecord,
};
use cc_silicon_compiler::chips::{
    handler_for, LxClassifyChip, LxDecodeLiteralChip, LxInternChip, PaTuChip, TyConvChip,
    TyScopeChip, TySymbolChip, TyTypeChip, Worker, WorkerRegistry,
};
use cc_silicon_compiler::diagnostic::DiagGroup;
use cc_silicon_compiler::ids::{
    NameId, NodeId, RecordRef, ScopeId, SourceId, TaskId, TokenId, TypeId,
};
use cc_silicon_compiler::limits::Limits;
use cc_silicon_compiler::manifest::{
    is_ty_slice_kind, stage_of, FieldPath, ManifestRegistryError, StoreSchema, PA_TU_CHIP,
    STORE_OWNER_ALLOWLIST, TY_CONV_CHIP, TY_SCOPE_CHIP, TY_SYMBOL_CHIP, TY_TYPE_CHIP,
};
use cc_silicon_compiler::routing::{RoutingShell, TickOutcome};
use cc_silicon_compiler::snapshot::{
    decode_scope, decode_scope_event, decode_symbol, decode_type, encode_scope, encode_scope_event,
    encode_symbol, encode_type, Snapshot,
};
use cc_silicon_compiler::target::{CompilerConfig, Dialect, OptLevel, TargetSpec};
use cc_silicon_compiler::task::{Payload, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry};

fn new_bus() -> CompilerBus {
    CompilerBus::new(CompilerConfig::new(
        TargetSpec::aarch64_unknown_linux_gnu_unverified(),
        Dialect::C11,
        OptLevel::O0,
        vec![],
        Limits::fixture(),
    ))
}

fn install_ty(bus: &mut CompilerBus, layer: u16) {
    let _ = layer;
    bus.kinds = TaskKindRegistry::ty_slice();
    bus.schema = StoreSchema::ty_slice();
    for chip in [
        &LxInternChip as &dyn Worker,
        &LxClassifyChip,
        &LxDecodeLiteralChip,
        &PaTuChip,
        &TyTypeChip,
        &TyScopeChip,
        &TySymbolChip,
        &TyConvChip,
    ] {
        bus.registrations
            .register(chip.manifest(), &bus.schema, &bus.kinds)
            .unwrap();
    }
    bus.routing
        .register(
            TaskKind::LEX_INTERN,
            cc_silicon_compiler::manifest::LX_INTERN_CHIP,
            2,
        )
        .unwrap();
    bus.routing
        .register(
            TaskKind::LEX_CLASSIFY,
            cc_silicon_compiler::manifest::LX_CLASSIFY_CHIP,
            2,
        )
        .unwrap();
    bus.routing
        .register(
            TaskKind::LEX_DECODE_LITERAL,
            cc_silicon_compiler::manifest::LX_DECODE_CHIP,
            2,
        )
        .unwrap();
    // Routes pin each kind to its frozen stage (driver-enforced):
    // LX/PA kinds live at layer 2, TY kinds at layer 3.
    bus.routing
        .register(TaskKind::PARSE_TU, PA_TU_CHIP, 2)
        .unwrap();
    for (kind, chip) in [
        (TaskKind::SYMBOL_INT_TYPE, TY_TYPE_CHIP),
        (TaskKind::SYMBOL_FUNC_TYPE, TY_TYPE_CHIP),
        (TaskKind::SYMBOL_SCOPE_ENTER, TY_SCOPE_CHIP),
        (TaskKind::SYMBOL_SCOPE_EXIT, TY_SCOPE_CHIP),
        (TaskKind::SYMBOL_DECLARE, TY_SYMBOL_CHIP),
        (TaskKind::SYMBOL_LOOKUP, TY_SYMBOL_CHIP),
        (TaskKind::SYMBOL_PROMOTE, TY_CONV_CHIP),
        (TaskKind::SYMBOL_COMMON_TYPE, TY_CONV_CHIP),
        (TaskKind::SYMBOL_RETURN_CONVERT, TY_CONV_CHIP),
    ] {
        bus.routing.register(kind, chip, layer).unwrap();
    }
}

fn workers() -> WorkerRegistry {
    let mut workers = WorkerRegistry::new();
    workers.register(LxInternChip).unwrap();
    workers.register(LxClassifyChip).unwrap();
    workers.register(LxDecodeLiteralChip).unwrap();
    workers.register(PaTuChip).unwrap();
    workers.register(TyTypeChip).unwrap();
    workers.register(TyScopeChip).unwrap();
    workers.register(TySymbolChip).unwrap();
    workers.register(TyConvChip).unwrap();
    workers
}

fn seed_source(bus: &mut CompilerBus) -> SourceId {
    let mut raw = b"int main(void){return 2+3;}".to_vec();
    raw.push(b'\n');
    let name = bus.intern_name(b"main.c").unwrap();
    bus.alloc_source(name, raw).unwrap()
}

fn seed_pp_layer(
    bus: &mut CompilerBus,
    source: SourceId,
) -> Vec<cc_silicon_compiler::ids::PpTokenId> {
    let limits = bus.limits();
    let spans: Vec<(u64, u64)> = vec![
        (0, 3),
        (4, 8),
        (8, 9),
        (9, 13),
        (13, 14),
        (14, 15),
        (15, 21),
        (22, 23),
        (23, 24),
        (24, 25),
        (25, 26),
        (26, 27),
        (28, 28),
    ];
    let mut span_ids = Vec::new();
    for (start, end) in spans {
        span_ids.push(
            bus.arenas
                .spans
                .alloc(
                    SpanRecord {
                        source,
                        start,
                        end,
                        expansion: None,
                    },
                    &limits,
                )
                .unwrap(),
        );
    }
    let tokens: Vec<(PpTokenKind, usize, &[u8])> = vec![
        (PpTokenKind::Identifier, 0, b"int"),
        (PpTokenKind::Identifier, 1, b"main"),
        (PpTokenKind::Punctuator, 2, b"("),
        (PpTokenKind::Identifier, 3, b"void"),
        (PpTokenKind::Punctuator, 4, b")"),
        (PpTokenKind::Punctuator, 5, b"{"),
        (PpTokenKind::Identifier, 6, b"return"),
        (PpTokenKind::PpNumber, 7, b"2"),
        (PpTokenKind::Punctuator, 8, b"+"),
        (PpTokenKind::PpNumber, 9, b"3"),
        (PpTokenKind::Punctuator, 10, b";"),
        (PpTokenKind::Punctuator, 11, b"}"),
        (PpTokenKind::Eof, 12, b""),
    ];
    let mut ids = Vec::new();
    for (kind, span, spelling) in tokens {
        ids.push(
            bus.arenas
                .pp_tokens
                .alloc(
                    PpTokenRecord {
                        kind,
                        span: span_ids[span],
                        spelling: spelling.to_vec(),
                    },
                    &limits,
                )
                .unwrap(),
        );
    }
    ids
}

fn bootstrap(
    bus: &mut CompilerBus,
    kind: TaskKind,
    owner: cc_silicon_compiler::ids::ChipId,
    payload: Payload,
) -> TaskId {
    bus.bootstrap_task(TaskDraft {
        kind,
        owner,
        parent: None,
        payload,
        continuation: None,
    })
    .unwrap()
}

fn tick(
    bus: &mut CompilerBus,
    workers: &WorkerRegistry,
) -> cc_silicon_compiler::routing::TickReport {
    RoutingShell
        .clock_tick_with(&CompilerPins::default(), bus, handler_for(workers))
        .unwrap()
}

/// Run the full PP→LX→PA chain plus int/func types; return handles.
struct Chain {
    bus: CompilerBus,
    tu: NodeId,
    int: TypeId,
    func: TypeId,
    file_scope: ScopeId,
}

fn full_chain() -> Chain {
    let mut bus = new_bus();
    install_ty(&mut bus, 3);
    let source = seed_source(&mut bus);
    let pp = seed_pp_layer(&mut bus, source);
    let workers = workers();
    let refs = pp.iter().map(|id| RecordRef::PpToken(*id)).collect();
    bootstrap(
        &mut bus,
        TaskKind::LEX_INTERN,
        cc_silicon_compiler::manifest::LX_INTERN_CHIP,
        Payload::from_refs(refs),
    );
    tick(&mut bus, &workers);
    let refs = pp.iter().map(|id| RecordRef::PpToken(*id)).collect();
    bootstrap(
        &mut bus,
        TaskKind::LEX_CLASSIFY,
        cc_silicon_compiler::manifest::LX_CLASSIFY_CHIP,
        Payload::from_refs(refs),
    );
    let report = tick(&mut bus, &workers);
    let tokens: Vec<TokenId> = match report.outcome {
        TickOutcome::Executed { commit, .. } => commit
            .appended
            .iter()
            .map(|(_, reference)| match reference {
                RecordRef::Token(id) => *id,
                other => panic!("expected token ref, got {other:?}"),
            })
            .collect(),
        other => panic!("expected executed, got {other:?}"),
    };
    for index in [7, 9] {
        let pp_token = bus.arenas.tokens.get(tokens[index]).unwrap().pp_token;
        bootstrap(
            &mut bus,
            TaskKind::LEX_DECODE_LITERAL,
            cc_silicon_compiler::manifest::LX_DECODE_CHIP,
            Payload::from_refs(vec![
                RecordRef::Token(tokens[index]),
                RecordRef::PpToken(pp_token),
            ]),
        );
        tick(&mut bus, &workers);
    }
    bootstrap(
        &mut bus,
        TaskKind::PARSE_TU,
        PA_TU_CHIP,
        Payload::from_refs(tokens.iter().map(|id| RecordRef::Token(*id)).collect()),
    );
    let report = tick(&mut bus, &workers);
    let tu = match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            let result = bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .clone();
            match result.value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Node(id)) => id,
                other => panic!("expected node ref, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    // int + func types.
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_INT_TYPE,
        TY_TYPE_CHIP,
        Payload::empty(),
    );
    let report = tick(&mut bus, &workers);
    let int = match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            let result = bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .clone();
            match result.value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Type(id)) => id,
                other => panic!("expected type ref, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_FUNC_TYPE,
        TY_TYPE_CHIP,
        Payload::from_refs(vec![RecordRef::Type(int)]),
    );
    let report = tick(&mut bus, &workers);
    let func = match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            let result = bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .clone();
            match result.value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Type(id)) => id,
                other => panic!("expected type ref, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    // File scope enter after the TU commit (ordering holds; the automatic
    // edge stays deferred, so the test dispatches explicitly).
    assert_eq!(bus.arenas.scopes.allocated(), 0);
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_SCOPE_ENTER,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Node(tu)]),
    );
    let report = tick(&mut bus, &workers);
    let file_scope = match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            let result = bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .clone();
            match result.value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Scope(id)) => id,
                other => panic!("expected scope ref, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    Chain {
        bus,
        tu,
        int,
        func,
        file_scope,
    }
}

#[test]
fn ty_kinds_stages_and_allowlist_are_frozen() {
    let kinds = [
        (TaskKind::SYMBOL_INT_TYPE, "symbol_type.int_type", 16),
        (TaskKind::SYMBOL_FUNC_TYPE, "symbol_type.func_type", 17),
        (TaskKind::SYMBOL_SCOPE_ENTER, "symbol_type.scope_enter", 18),
        (TaskKind::SYMBOL_SCOPE_EXIT, "symbol_type.scope_exit", 19),
        (TaskKind::SYMBOL_DECLARE, "symbol_type.declare", 20),
        (TaskKind::SYMBOL_LOOKUP, "symbol_type.lookup", 21),
        (TaskKind::SYMBOL_PROMOTE, "symbol_type.promote", 22),
        (TaskKind::SYMBOL_COMMON_TYPE, "symbol_type.common_type", 23),
        (
            TaskKind::SYMBOL_RETURN_CONVERT,
            "symbol_type.return_convert",
            24,
        ),
    ];
    for (kind, _, local) in kinds {
        assert_eq!(kind.group(), TaskGroup::SYMBOL_TYPE);
        assert_eq!(kind.local(), local);
        assert!(is_ty_slice_kind(kind));
        assert_eq!(stage_of(kind), Some(3));
    }
    assert!(!is_ty_slice_kind(TaskKind::CONTROL_NOOP));
    let registry = TaskKindRegistry::ty_slice();
    assert_eq!(registry.len(), 21);
    for (kind, name, _) in kinds {
        assert_eq!(registry.lookup(kind).unwrap().name, name);
    }
    for row in [
        (
            TY_TYPE_CHIP,
            cc_silicon_compiler::task::StoreId::Types,
            "records",
            TaskKind::SYMBOL_INT_TYPE,
        ),
        (
            TY_TYPE_CHIP,
            cc_silicon_compiler::task::StoreId::Types,
            "records",
            TaskKind::SYMBOL_FUNC_TYPE,
        ),
        (
            TY_SCOPE_CHIP,
            cc_silicon_compiler::task::StoreId::Symbols,
            "scopes",
            TaskKind::SYMBOL_SCOPE_ENTER,
        ),
        (
            TY_SCOPE_CHIP,
            cc_silicon_compiler::task::StoreId::Symbols,
            "scope_events",
            TaskKind::SYMBOL_SCOPE_ENTER,
        ),
        (
            TY_SCOPE_CHIP,
            cc_silicon_compiler::task::StoreId::Symbols,
            "scope_events",
            TaskKind::SYMBOL_SCOPE_EXIT,
        ),
        (
            TY_SYMBOL_CHIP,
            cc_silicon_compiler::task::StoreId::Symbols,
            "symbols",
            TaskKind::SYMBOL_DECLARE,
        ),
    ] {
        assert!(STORE_OWNER_ALLOWLIST.contains(&row));
    }
}

#[test]
fn ty_int_reuse_and_func_shape() {
    let mut chain = full_chain();
    let workers = workers();
    // A second int request reuses the committed canonical id: one record.
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_INT_TYPE,
        TY_TYPE_CHIP,
        Payload::empty(),
    );
    let report = tick(&mut chain.bus, &workers);
    let again = match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            let result = chain
                .bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .clone();
            match result.value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Type(id)) => id,
                other => panic!("expected type ref, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    assert_eq!(again, chain.int);
    assert_eq!(chain.bus.arenas.types.allocated(), 2);
    // The int record is symbolic with no width.
    let int_body = chain.bus.arenas.types.get(chain.int).unwrap();
    assert_eq!(
        int_body.kind,
        TypeKind::Int {
            rank: IntRank::Int,
            signed: true
        }
    );
    let func_body = chain.bus.arenas.types.get(chain.func).unwrap();
    match &func_body.kind {
        TypeKind::Function {
            result,
            params,
            prototype,
            variadic,
        } => {
            assert_eq!(*result, chain.int);
            assert!(params.is_empty());
            assert!(*prototype);
            assert!(!*variadic);
        }
        other => panic!("expected function type, got {other:?}"),
    }
    // Snapshot round-trips the committed type bodies.
    let bytes = encode_type(int_body);
    assert_eq!(decode_type(&bytes).unwrap(), *int_body);
    let _ = Snapshot::capture(&chain.bus);
}

#[test]
fn ty_file_enter_once_and_body_lifecycle() {
    let mut chain = full_chain();
    let workers = workers();
    // M1-START-01 shape: one file scope + one Enter anchored at the TU.
    assert_eq!(chain.bus.arenas.scopes.allocated(), 1);
    assert_eq!(chain.bus.arenas.scope_events.allocated(), 1);
    let scope = chain.bus.arenas.scopes.get(chain.file_scope).unwrap();
    assert_eq!(scope.kind, cc_silicon_compiler::bus::ScopeKind::File);
    assert_eq!(scope.parent, None);
    // A second file Enter fails (duplicate guard): exactly-once holds.
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_SCOPE_ENTER,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Node(chain.tu)]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(chain.bus.arenas.scopes.allocated(), 1);
    // Body scope: enter on the Compound node, then exit.
    let compound = NodeId::from_index(4);
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_SCOPE_ENTER,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Node(compound)]),
    );
    let report = tick(&mut chain.bus, &workers);
    let body_scope = match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            let result = chain
                .bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .clone();
            match result.value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Scope(id)) => id,
                other => panic!("expected scope ref, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    let body = chain.bus.arenas.scopes.get(body_scope).unwrap().clone();
    assert_eq!(body.kind, cc_silicon_compiler::bus::ScopeKind::Block);
    assert_eq!(body.parent, Some(chain.file_scope));
    assert_eq!(body.owner, Some(compound));
    // Duplicate body enter fails; exit works once, then fails twice-ish.
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_SCOPE_ENTER,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Node(compound)]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_SCOPE_EXIT,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Scope(body_scope)]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.completed.len(), 1),
        other => panic!("expected executed, got {other:?}"),
    }
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_SCOPE_EXIT,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Scope(body_scope)]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // File scope never exits.
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_SCOPE_EXIT,
        TY_SCOPE_CHIP,
        Payload::from_refs(vec![RecordRef::Scope(chain.file_scope)]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // Snapshot round-trips scope bodies.
    let bytes = encode_scope(&body);
    assert_eq!(decode_scope(&bytes).unwrap(), body);
    let event = chain
        .bus
        .arenas
        .scope_events
        .iter()
        .next()
        .unwrap()
        .1
        .clone();
    let bytes = encode_scope_event(&event);
    assert_eq!(decode_scope_event(&bytes).unwrap(), event);
}

#[test]
fn ty_declare_lookup_and_conflicts() {
    let mut chain = full_chain();
    let workers = workers();
    let declarator = NodeId::from_index(3);
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        Payload::from_refs(vec![
            RecordRef::Node(declarator),
            RecordRef::Type(chain.func),
            RecordRef::Scope(chain.file_scope),
        ]),
    );
    let report = tick(&mut chain.bus, &workers);
    let symbol = match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            let result = chain
                .bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .clone();
            match result.value {
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Symbol(id)) => id,
                other => panic!("expected symbol ref, got {other:?}"),
            }
        }
        other => panic!("expected executed, got {other:?}"),
    };
    let body = chain.bus.arenas.symbols.get(symbol).unwrap();
    let main_name = chain.bus.intern.lookup(b"main").unwrap();
    assert_eq!(body.name, main_name);
    assert_eq!(body.kind, SymbolKind::Function);
    assert_eq!(body.ty, Some(chain.func));
    assert_eq!(body.scope, chain.file_scope);
    assert_eq!(body.linkage, Linkage::External);
    assert_eq!(body.storage, StorageDuration::Static);
    assert_eq!(body.decl, declarator);
    let bytes = encode_symbol(body);
    assert_eq!(decode_symbol(&bytes).unwrap(), *body);
    // Duplicate declaration fails as a typed conflict, committing nothing.
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_DECLARE,
        TY_SYMBOL_CHIP,
        Payload::from_refs(vec![
            RecordRef::Node(declarator),
            RecordRef::Type(chain.func),
            RecordRef::Scope(chain.file_scope),
        ]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.failed.len(), 1);
            let (_, diagnostic) = commit.failed[0];
            let record = chain.bus.arenas.diagnostics.get(diagnostic).unwrap();
            assert_eq!(record.code.group, DiagGroup::Semantic);
        }
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    assert_eq!(chain.bus.arenas.symbols.allocated(), 1);
    // Lookup hits after declaration.
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_LOOKUP,
        TY_SYMBOL_CHIP,
        Payload::from_refs(vec![
            RecordRef::Scope(chain.file_scope),
            RecordRef::Name(main_name),
        ]),
    );
    let report = tick(&mut chain.bus, &workers);
    match report.outcome {
        TickOutcome::Executed { commit, .. } => {
            let result = chain
                .bus
                .arenas
                .results
                .get(commit.completed[0].1)
                .unwrap()
                .clone();
            assert_eq!(
                result.value,
                cc_silicon_compiler::task::ResultValue::Record(RecordRef::Symbol(symbol))
            );
        }
        other => panic!("expected executed, got {other:?}"),
    }
    // Lookup of an uninterned-but-valid name misses with a typed diagnostic.
    let other = chain.bus.intern_name(b"other").unwrap();
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_LOOKUP,
        TY_SYMBOL_CHIP,
        Payload::from_refs(vec![
            RecordRef::Scope(chain.file_scope),
            RecordRef::Name(other),
        ]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => {
            assert_eq!(commit.failed.len(), 1);
            let (_, diagnostic) = commit.failed[0];
            let record = chain.bus.arenas.diagnostics.get(diagnostic).unwrap();
            // M1-NEG-14 carrier: a real undeclared-identifier error, never
            // an `Unsupported` gap claim.
            assert_eq!(record.code.group, DiagGroup::Semantic);
            assert_ne!(record.code.group, DiagGroup::Unsupported);
        }
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
}

#[test]
fn ty_identity_conversions_complete_without_plans() {
    let mut chain = full_chain();
    let workers = workers();
    for (kind, refs) in [
        (TaskKind::SYMBOL_PROMOTE, vec![RecordRef::Type(chain.int)]),
        (
            TaskKind::SYMBOL_COMMON_TYPE,
            vec![RecordRef::Type(chain.int), RecordRef::Type(chain.int)],
        ),
        (
            TaskKind::SYMBOL_RETURN_CONVERT,
            vec![RecordRef::Type(chain.int), RecordRef::Type(chain.int)],
        ),
    ] {
        bootstrap(&mut chain.bus, kind, TY_CONV_CHIP, Payload::from_refs(refs));
        let report = tick(&mut chain.bus, &workers);
        match report.outcome {
            TickOutcome::Executed { commit, .. } => {
                let result = chain
                    .bus
                    .arenas
                    .results
                    .get(commit.completed[0].1)
                    .unwrap()
                    .clone();
                // Identity: the unchanged committed id, no plan recorded.
                assert_eq!(
                    result.value,
                    cc_silicon_compiler::task::ResultValue::Record(RecordRef::Type(chain.int))
                );
            }
            other => panic!("expected executed, got {other:?}"),
        }
    }
    // A non-`Int` operand is explicit unsupported (deferred past M1).
    bootstrap(
        &mut chain.bus,
        TaskKind::SYMBOL_PROMOTE,
        TY_CONV_CHIP,
        Payload::from_refs(vec![RecordRef::Type(chain.func)]),
    );
    match tick(&mut chain.bus, &workers).outcome {
        TickOutcome::Executed { commit, .. } => assert_eq!(commit.failed.len(), 1),
        other => panic!("expected executed-with-failure, got {other:?}"),
    }
    // CharKind vocabulary is frozen even though `char` has no M1 producer.
    let char_ty = TypeRecord {
        kind: cc_silicon_compiler::bus::TypeKind::Char(CharKind::Plain),
    };
    let bytes = encode_type(&char_ty);
    assert_eq!(decode_type(&bytes).unwrap(), char_ty);
    assert_ne!(CharKind::Plain, CharKind::Signed);
}

#[test]
fn ty_char_kind_vocabulary_is_distinct() {
    use cc_silicon_compiler::snapshot::{char_kind_name, int_rank_name};
    assert_eq!(int_rank_name(IntRank::Short), "short");
    assert_eq!(int_rank_name(IntRank::Int), "int");
    assert_eq!(char_kind_name(CharKind::Plain), "plain");
    assert_eq!(char_kind_name(CharKind::Signed), "signed");
    assert_eq!(char_kind_name(CharKind::Unsigned), "unsigned");
    let _ = NameId::from_index(0);
}

#[test]
fn ty_stage_layer_and_manifest_gates() {
    let mut bus = new_bus();
    install_ty(&mut bus, 9);
    bootstrap(
        &mut bus,
        TaskKind::SYMBOL_INT_TYPE,
        TY_TYPE_CHIP,
        Payload::empty(),
    );
    let workers = workers();
    // drive_task refuses before any worker runs.
    let task = bus.arenas.tasks.iter().next().unwrap().0;
    let error = cc_silicon_compiler::chips::drive_task(&bus, task, &workers).unwrap_err();
    assert!(matches!(
        error,
        cc_silicon_compiler::chips::DriveError::StageLayerMismatch { .. }
    ));
    // A manifest missing its write is rejected at registration.
    let mut manifest = TySymbolChip.manifest();
    manifest.writes = vec![];
    assert!(matches!(
        bus.registrations.register(
            manifest,
            &StoreSchema::ty_slice(),
            &TaskKindRegistry::ty_slice()
        ),
        Err(ManifestRegistryError::Registry(_))
    ));
    let _ = FieldPath::new(cc_silicon_compiler::task::StoreId::Symbols, "symbols");
}

#[test]
fn ty_snapshot_replay_is_deterministic() {
    let run = || {
        let mut chain = full_chain();
        let workers = workers();
        bootstrap(
            &mut chain.bus,
            TaskKind::SYMBOL_DECLARE,
            TY_SYMBOL_CHIP,
            Payload::from_refs(vec![
                RecordRef::Node(NodeId::from_index(3)),
                RecordRef::Type(chain.func),
                RecordRef::Scope(chain.file_scope),
            ]),
        );
        tick(&mut chain.bus, &workers);
        Snapshot::capture(&chain.bus)
    };
    assert_eq!(run(), run());
}
