// ============================================================================
// prelude.rs — convenient re-exports for the compiler contract foundation.
// ============================================================================

//! The items most compiler-application code needs.

pub use crate::arena::{ArenaError, ArenaId, Reserved, ReservedArena, TypedArena};
pub use crate::bus::{
    ArtifactKind, ArtifactRecord, CompilerBus, CompilerPins, CompilerWires, Control,
    ExpansionRecord, HostResponse, JobState, SourceRecord, SpanRecord, Stage, TaggedProposal,
};
pub use crate::codec::{hex32, sha256, Writer};
pub use crate::commit::{
    commit_proposals, consume_result, CommitError, CommitReport, CommittedPatch, StoreVersions,
};
pub use crate::contract::{
    compute_contract_hash, CONTRACT_HASH, CONTRACT_VERSION, NORMATIVE_RULES,
};
pub use crate::diagnostic::{
    DiagGroup, DiagnosticCode, DiagnosticDraft, DiagnosticRecord, Severity,
};
pub use crate::ids::*;
pub use crate::intern::{InternError, InternTable};
pub use crate::limits::{LimitError, Limits};
pub use crate::manifest::{
    validate_manifest, BackendClass, Capability, ChipManifest, ChipPhase, FieldPath, ManifestError,
    ManifestRegistry, ManifestRegistryError, SchemaError, StoreSchema,
};
pub use crate::routing::{
    phase_priority, RouteEntry, RouteError, RoutingShell, RoutingTable, TickOutcome, TickReport,
};
pub use crate::snapshot::{config_hash, encode_config, Snapshot, Trace, TraceRecord};
pub use crate::target::{
    AbiSpec, CompilerConfig, ConfigError, CorpusPolicy, DataModel, Dialect, Endianness,
    FloatFormat, ObjectFormat, OptLevel, OptionFlag, ProbeError, ProbeReport, ProbeStatus,
    ProbeSubstrate, ScalarKind, ScalarModel, TargetProfile, TargetSpec, VerificationState,
    WideCharEncoding, MEASURED_SCALARS,
};
pub use crate::task::{
    ContinuationRecord, HostRequestDraft, HostRequestKind, HostRequestRecord, KindEntry,
    KindStatus, PatchOp, Payload, Proposal, RegistryError, ResultRecord, ResultValue, StoreId,
    StorePatch, Task, TaskDraft, TaskGroup, TaskKind, TaskKindRegistry, TaskState, WaitSet,
};
