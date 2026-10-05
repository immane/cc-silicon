// ============================================================================
// bus.rs — the CompilerBus storage profile (T01 C01/C03)
//
// The approved CPU storage extension keeps every piece of semantic state in a
// single `CompilerBus`: append-only Vec arenas addressed by stable newtype IDs,
// an intern table, persistent queues/results, diagnostics, routing, and a
// per-tick wire bundle. Registers persist across ticks; wires reset at tick
// start.
//
// The bus never uses `Rc`/`Arc`/`RefCell`/`Mutex`, never treats an address as
// an ID, and never lets map iteration decide ordering. Every configured
// resource bound is enforced through a structured [`LimitError`] before any
// mutation.
// ============================================================================

use crate::arena::{ReservedArena, TypedArena};
use crate::diagnostic::DiagnosticRecord;
use crate::ids::{
    ArtifactId, BlockId, ConstId, ContinuationId, DiagnosticId, ExpansionId, FunctionId,
    HostRequestId, InitId, InstructionId, LayoutId, LiteralId, MacroId, NameId, NodeId, PpTokenId,
    ResultId, ScopeEventId, ScopeId, SemId, SourceId, SpanId, SymbolId, TaskId, TokenId, TypeId,
    VRegId, ValueId,
};
use crate::intern::InternTable;
use crate::limits::{LimitError, Limits};
use crate::manifest::ManifestRegistry;
use crate::routing::RoutingTable;
use crate::snapshot::{LiteralKind, LiteralSuffix, Lx08CandidateType};
use crate::target::CompilerConfig;
use crate::task::{
    ContinuationRecord, HostRequestRecord, ResultRecord, StoreId, Task, TaskDraft, TaskKind,
    TaskKindRegistry,
};
use cc_silicon::Bus;

/// A source file record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceRecord {
    /// Interned file name.
    pub name: NameId,
    /// Raw file bytes.
    pub bytes: Vec<u8>,
    /// SHA-256 of the raw bytes.
    pub content_hash: [u8; 32],
}

/// A source span record.
///
/// Offsets are `u64` half-open raw-byte offsets bounded by
/// `limits.max_source_bytes` (the `/6` widening of the `/5` `u32` offsets;
/// T03 finding 5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpanRecord {
    /// Owning source.
    pub source: SourceId,
    /// Start byte offset.
    pub start: u64,
    /// End byte offset (exclusive).
    pub end: u64,
    /// Expansion that produced this span, if any.
    pub expansion: Option<ExpansionId>,
}

/// A macro expansion provenance record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExpansionRecord {
    /// Parent expansion.
    pub parent: Option<ExpansionId>,
    /// Spelling span.
    pub spelling: SpanId,
    /// Expanded span.
    pub expanded: SpanId,
    /// Deterministic expansion ordinal.
    pub ordinal: u32,
}

/// Kind of output artifact (`/10` total 8-variant set, rev-44).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactKind {
    /// Newline-normalized single-source bytes (M1 PP01 exercised).
    Normalized,
    /// Line-spliced bytes (declared, unexercised in M1).
    Spliced,
    /// Comment-free bytes (declared, unexercised in M1).
    CommentFree,
    /// Preprocessed source.
    Preprocessed,
    /// Generated assembly.
    Assembly,
    /// Object file.
    Object,
    /// A deterministic snapshot.
    Snapshot,
    /// A deterministic trace.
    Trace,
}

impl ArtifactKind {
    /// Whether this kind requires a location map (`/10` rev-44 total rule).
    pub const fn requires_map(self) -> bool {
        match self {
            Self::Normalized | Self::Spliced | Self::CommentFree | Self::Preprocessed => true,
            Self::Assembly | Self::Object | Self::Snapshot | Self::Trace => false,
        }
    }
}

/// An output artifact fragment (`/10` rev-44 shape).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactRecord {
    /// Artifact kind.
    pub kind: ArtifactKind,
    /// Owning source (`Some` required for map-mandatory kinds).
    pub source: Option<SourceId>,
    /// Fragment bytes.
    pub bytes: Vec<u8>,
    /// Output-boundary to raw-source-boundary map.
    pub raw_offsets: Vec<u64>,
}

impl ArtifactRecord {
    /// Validate the rev-45 mandatory-map invariants plus the rev-47
    /// optional-kind rule: map-mandatory kinds need `raw_offsets.len() ==
    /// bytes.len()+1`, first `== 0`, monotonic nondecreasing, last `<=`
    /// source length, and a valid `source`; map-optional kinds need empty
    /// `raw_offsets` with an optional valid `source`.
    pub fn check_map(&self, source_len: u64) -> Result<(), &'static str> {
        if self.kind.requires_map() {
            if self.source.is_none() {
                return Err("map-mandatory artifact requires a source");
            }
            if self.raw_offsets.len() as u64 != self.bytes.len() as u64 + 1 {
                return Err("raw_offsets length must equal bytes length plus one");
            }
            if self.raw_offsets.first() != Some(&0) {
                return Err("raw_offsets must start at zero");
            }
            let mut prev = 0u64;
            for offset in &self.raw_offsets {
                if *offset < prev {
                    return Err("raw_offsets must be monotonic nondecreasing");
                }
                prev = *offset;
            }
            if self
                .raw_offsets
                .last()
                .is_some_and(|last| *last > source_len)
            {
                return Err("raw_offsets end must not exceed source length");
            }
            // Note: the inserted-LF zero-width case ends at raw EOF, which
            // equals `source_len`; the CRLF collapse ends after the raw LF,
            // also `<= source_len`. Both satisfy the bound above.
            Ok(())
        } else {
            if !self.raw_offsets.is_empty() {
                return Err("map-optional artifact requires empty raw_offsets");
            }
            Ok(())
        }
    }
}

/// A T03-owned preprocessing token (`/11` LX-slice freeze).
///
/// M1 produces only `Identifier`, `PpNumber`, `Punctuator`, and `Eof`;
/// string/character literals and header names are deferred as explicit
/// unsupported. `Eof` carries empty spelling and a zero-width span at the
/// source end.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PpTokenRecord {
    /// Preprocessing-token kind.
    pub kind: PpTokenKind,
    /// Committed T03-owned span (byte offsets into the owning source).
    pub span: SpanId,
    /// Raw spelling bytes.
    pub spelling: Vec<u8>,
}

/// A T03-owned macro definition (`/23` PP macro-definition slice freeze).
///
/// Names and parameters stay raw spelling bytes (never interned): lookup
/// compares spellings, so no cross-batch intern-ID prediction is ever
/// needed. Lookup takes the greatest `MacroId` with equal spelling; a
/// latest tombstone (`undefined`) — or absence — means undefined.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MacroRecord {
    /// Macro name spelling (raw bytes).
    pub spelling: Vec<u8>,
    /// Parameter spellings in order (`[]` for object-like).
    pub params: Vec<Vec<u8>>,
    /// Variadic (`...`/`__VA_ARGS__` present; use deferred to PP16).
    pub variadic: bool,
    /// Replacement list (committed pp-token IDs).
    pub replacement: Vec<PpTokenId>,
    /// `#undef` tombstone (no definition while set).
    pub undefined: bool,
}

/// Preprocessing-token kinds (`/11` M1 closed set extended in `/20`
/// with literal and header-name kinds for the full-token scan; LX decode
/// of the new kinds stays deferred).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PpTokenKind {
    /// `int`, `main`, `void`, `return` at PP level (keywords not yet distinguished).
    Identifier,
    /// `2`, `3` (M1 decimal only; other numeric forms are explicit unsupported).
    PpNumber,
    /// `(`, `)`, `{`, `+`, `;`, `}` (M1 subset; the full C11 table lives
    /// chip-local in the `/20` scanner and produces the same kind).
    Punctuator,
    /// `"..."` with escapes preserved (`/20`; LX decode deferred).
    StringLiteral,
    /// `'...'` with escapes preserved (`/20`; LX decode deferred).
    CharLiteral,
    /// `<...>` or `"..."` after `#include` (`/20`; PP17 consumes it).
    HeaderName,
    /// End of input (zero-width span at the source end).
    Eof,
}

/// A T04-owned C token (`/11` LX-slice freeze).
///
/// `span` reuses the committed T03 PP span; T04 writes no spans. `name` is
/// the interned spelling for `Identifier`/`Keyword` tokens. There is no
/// forward `literal` link in this slice: literals point back at their token
/// (`LiteralRecord.token`), and the reciprocal link stays deferred.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TokenRecord {
    /// C-token kind.
    pub kind: TokenKind,
    /// Committed T03-owned PP span reused verbatim.
    pub span: SpanId,
    /// Interned spelling for `Identifier`/`Keyword` tokens.
    pub name: Option<NameId>,
    /// Originating committed PP token (committed before classification).
    pub pp_token: PpTokenId,
}

/// C-token kinds (`/11` M1-closed produced subset in doc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    /// `int`, `void`, `return` (full C11 keyword table, membership-tested).
    Keyword,
    /// `main`.
    Identifier,
    /// `(`, `)`, `{`, `+`, `;`, `}`.
    Punctuator,
    /// `2`, `3` (M1 decimal no-suffix only).
    Integer,
    /// End of input.
    Eof,
}

/// Integer rank for `TypeKind::Int` (`/13`; symbolic, no target width).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntRank {
    /// `short`.
    Short,
    /// `int`.
    Int,
    /// `long`.
    Long,
    /// `long long`.
    LongLong,
}

/// Character kind (`/13`; plain signedness stays probe-gated).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharKind {
    /// Plain `char` (signedness target-defined).
    Plain,
    /// `signed char`.
    Signed,
    /// `unsigned char`.
    Unsigned,
}

/// A T06-owned canonical type record (`/13` slice freeze).
///
/// M1 produces only `Int { rank: Int, signed: true }` (TY13, single
/// producer with reuse scan) and `Function { result: int, params: [],
/// prototype: true, variadic: false }` (TY17). All other type forms are
/// explicit unsupported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeRecord {
    /// Type kind.
    pub kind: TypeKind,
}

/// Canonical type kinds (`/13` M1-closed set in doc; only `Int` and the
/// M1 `Function` shape are produced).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeKind {
    /// `void`.
    Void,
    /// `_Bool`.
    Bool,
    /// Character type with explicit kind.
    Char(CharKind),
    /// Signed or unsigned integer with rank.
    Int {
        /// Integer rank.
        rank: IntRank,
        /// Signedness.
        signed: bool,
    },
    /// Function type.
    Function {
        /// Result type.
        result: crate::ids::TypeId,
        /// Parameter types (empty with `prototype: true` means `(void)`).
        params: Vec<crate::ids::TypeId>,
        /// Whether the parameter list is a prototype.
        prototype: bool,
        /// Whether the function is variadic.
        variadic: bool,
    },
}

/// Symbol kinds (`/13`; only `Function` is produced in M1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    /// A declared function (`main`).
    Function,
    /// A declared object (deferred past M1).
    Object,
}

/// Linkage (`/13`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Linkage {
    /// No linkage.
    None,
    /// Internal linkage (`static`).
    Internal,
    /// External linkage (M1 `main` default).
    External,
}

/// Storage duration (`/13`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageDuration {
    /// No storage (type names, enumerators).
    None,
    /// Static storage duration (M1 file-scope `main`).
    Static,
    /// Automatic storage duration.
    Automatic,
    /// Thread-local storage duration.
    Thread,
    /// Allocated storage duration.
    Allocated,
}

/// A T06-owned declared-symbol record (`/13` slice freeze).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolRecord {
    /// Declared name.
    pub name: crate::ids::NameId,
    /// Scope of declaration.
    pub scope: crate::ids::ScopeId,
    /// Symbol kind.
    pub kind: SymbolKind,
    /// Declared type (`None` only for labels; M1 always `Some`).
    pub ty: Option<crate::ids::TypeId>,
    /// Linkage.
    pub linkage: Linkage,
    /// Storage duration.
    pub storage: StorageDuration,
    /// Declaring node (M1: the `Declarator` node).
    pub decl: crate::ids::NodeId,
}

/// Scope kinds (`/13` M1-closed: file + block).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeKind {
    /// File scope (one per TU in M1, never exits).
    File,
    /// Block scope (M1 function body).
    Block,
}

/// A T06-owned scope record (`/13` slice freeze; identified by its owner
/// lexical node, never by `(parent, kind)`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeRecord {
    /// Scope kind.
    pub kind: ScopeKind,
    /// Parent scope (`None` for the file scope).
    pub parent: Option<crate::ids::ScopeId>,
    /// Owner lexical node (`None` for the file scope; the `Block` node for
    /// a body scope).
    pub owner: Option<crate::ids::NodeId>,
}

/// Scope event kinds (`/13`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScopeEventKind {
    /// Scope entered.
    Enter,
    /// Scope exited.
    Exit,
}

/// A T06-owned scope-lifecycle event (`/13` slice freeze; append-only,
/// order = `ScopeEventId`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeEventRecord {
    /// Entered/exited scope.
    pub scope: crate::ids::ScopeId,
    /// Event kind.
    pub kind: ScopeEventKind,
    /// Lexical node the event is anchored at (committed).
    pub at: crate::ids::NodeId,
}

/// Value category (`/14` SE-slice freeze; M1 produces `NonLvalue` only).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueCategory {
    /// An lvalue.
    Lvalue,
    /// A non-lvalue (M1 integer constants and `2+3`).
    NonLvalue,
    /// A function designator.
    FunctionDesignator,
    /// A void expression.
    Void,
}

/// Effect mask (`/14`; M1 allows only `0` — a nonzero mask is a typed chip
/// failure, never a pass).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectMask(pub u32);

/// IR opcode (`/15` M1-closed: constant materialization + function return).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IrOp {
    /// Materialize a committed `ConstRecord` as a value (never refolds).
    Constant,
    /// Return a value (the unique terminator in M1).
    Return,
}

/// A T09-owned IR function (`/15` slice freeze; one per M1 `main`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FunctionRecord {
    /// Checked `main` symbol.
    pub symbol: crate::ids::SymbolId,
    /// ABI-neutral signature (`int(void)`).
    pub signature: crate::ids::TypeId,
    /// Single entry block.
    pub entry: crate::ids::BlockId,
    /// Linkage (from the symbol).
    pub linkage: Linkage,
}

/// A T09-owned IR basic block (`/15`; instructions derived by ascending
/// `InstructionId`, never stored on the block).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRecord {
    /// Owning function.
    pub function: crate::ids::FunctionId,
    /// Ordinal within the function (M1: 0).
    pub ordinal: u32,
}

/// A T09-owned IR value (`/15`; the producer is the unique producing
/// `Instruction.result`, never a stored back-link).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueRecord {
    /// Value type (M1: `int`).
    pub ty: crate::ids::TypeId,
}

/// A T09-owned IR instruction (`/15`; M1 emits exactly `Constant` then
/// `Return`, so the terminator is the greatest `InstructionId`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstructionRecord {
    /// Operation.
    pub op: IrOp,
    /// Parent block.
    pub block: crate::ids::BlockId,
    /// Operand values (`Return` carries exactly one in M1).
    pub operands: Vec<crate::ids::ValueId>,
    /// Folded constant (`Some` only for `Constant`).
    pub immediate: Option<crate::ids::ConstId>,
    /// Result value (`None` for `Return`).
    pub result: Option<crate::ids::ValueId>,
}

/// A T07-owned checked-node fact (`/14` SE-slice freeze; exactly one per
/// checked `NodeId`).
///
/// M1 checks `{IntLiteral, BinaryAdd, Return}` with `ty = int`,
/// `category = NonLvalue`, `effects = 0`. There is no `conversions` field
/// at this slice: M1 identity is the absence of a plan (TC-02 answered),
/// and the shared plan type freezes later with non-identity conversions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemRecord {
    /// Checked node.
    pub node: NodeId,
    /// Committed canonical type.
    pub ty: TypeId,
    /// Value category.
    pub category: ValueCategory,
    /// Effects.
    pub effects: EffectMask,
}

/// A T05-owned AST node (`/12` PA-slice freeze).
///
/// M1 produces only the nine-node `int main(void){return 2+3;}` tree; all
/// other syntactic forms are explicit unsupported. `parent`/`children` are
/// committed-or-predicted node IDs (single-batch pre-order allocation, TU
/// first); `first_token`/`last_token` are committed tokens; `name` carries
/// the declarator name; `literal` carries the committed literal for
/// `IntLiteral` leaves. Coherence (reciprocal parent/children, token ranges)
/// is worker-enforced and test-pinned; commit-side link validation stays a
/// T01/T13 open item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeRecord {
    /// AST node kind.
    pub kind: NodeKind,
    /// Parent node (`None` for the TU root).
    pub parent: Option<NodeId>,
    /// Ordered child nodes.
    pub children: Vec<NodeId>,
    /// First covered token (committed).
    pub first_token: TokenId,
    /// Last covered token (committed, inclusive).
    pub last_token: TokenId,
    /// Declarator name (`Some` only for `Declarator`).
    pub name: Option<NameId>,
    /// Committed literal (`Some` only for `IntLiteral`).
    pub literal: Option<LiteralId>,
}

/// AST node kinds (`/12` M1-closed produced subset in doc).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// The whole translation unit.
    TranslationUnit,
    /// `int main(void){...}`.
    FunctionDefinition,
    /// The `int` specifier bundle.
    Specifiers,
    /// `main(void)` (prototype, zero parameters).
    Declarator,
    /// `{ return 2+3; }`.
    Compound,
    /// `return 2+3;`.
    Return,
    /// `2+3` (M1-fixed `Add`; other operators deferred).
    BinaryAdd,
    /// A committed integer literal leaf.
    IntLiteral,
}

/// A T04-owned decoded literal: raw lexical facts plus the symbolic `LX08`
/// candidate type (Gate 1 `/7` freeze of the rev-45 exact ordered fields).
///
/// Field order is frozen as declared: `token`, `kind`, `radix`, `suffix`,
/// `value`, `negative`, `spelling`, `candidate_type`. No `node` and no
/// `required_kind`: those belong to the sem-stage `ConstantRequest`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiteralRecord {
    /// Originating token, if any (`None` for synthetic literals).
    pub token: Option<TokenId>,
    /// Literal kind (M1 produces only `Integer`).
    pub kind: LiteralKind,
    /// Numeric radix (`2`, `8`, `10`, `16`; M1 decimal only).
    pub radix: u8,
    /// Literal suffix (M1 produces only `None`).
    pub suffix: LiteralSuffix,
    /// Big-endian magnitude bytes (no sign, no width prefix).
    pub value: Vec<u8>,
    /// Whether the literal was preceded by `-` in the source.
    pub negative: bool,
    /// Original spelling bytes.
    pub spelling: Vec<u8>,
    /// Lexical candidate type (symbolic; M1-closed `{Int}`, no bit width).
    pub candidate_type: Lx08CandidateType,
}

/// A T08-owned folded constant: big-endian magnitude plus sign (Gate 1 `/7`
/// freeze; user-selected magnitude-bytes carrier, 2026-10-06).
///
/// T08 decodes committed operands, folds with checked addition, and commits
/// exactly one `ConstRecord` per evaluation; T09 consumes it without
/// re-folding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstRecord {
    /// Big-endian magnitude bytes (no width prefix).
    pub value: Vec<u8>,
    /// Sign of the folded value.
    pub negative: bool,
}

/// Every typed arena owned by the compiler bus.
///
/// Foundation stores carry real record types. Language stores whose record
/// schema is owned by a later task group use [`ReservedArena`] so their stable
/// IDs exist without fabricating language semantics.
#[derive(Default)]
pub struct Arenas {
    /// Source files.
    pub sources: TypedArena<SourceId, SourceRecord>,
    /// Source spans.
    pub spans: TypedArena<SpanId, SpanRecord>,
    /// Macro expansion provenance.
    pub expansions: TypedArena<ExpansionId, ExpansionRecord>,
    /// Preprocessing tokens (`/11` T03/T04 co-freeze: typed on freeze).
    pub pp_tokens: TypedArena<PpTokenId, PpTokenRecord>,
    /// Macro definitions and `#undef` tombstones (`/23` slice freeze:
    /// typed on freeze).
    pub macros: TypedArena<MacroId, MacroRecord>,
    /// C tokens (`/11` LX-slice freeze: typed on freeze).
    pub tokens: TypedArena<TokenId, TokenRecord>,
    /// Scopes (`/13` slice freeze: typed on freeze).
    pub scopes: TypedArena<ScopeId, ScopeRecord>,
    /// Scope lifecycle events (`/13` slice freeze: typed on freeze).
    pub scope_events: TypedArena<ScopeEventId, ScopeEventRecord>,
    /// Symbols (`/13` slice freeze: typed on freeze).
    pub symbols: TypedArena<SymbolId, SymbolRecord>,
    /// Canonical types (`/13` slice freeze: typed on freeze).
    pub types: TypedArena<TypeId, TypeRecord>,
    /// Semantic facts, one per checked node (`/14` slice freeze: typed on
    /// freeze).
    pub sem: TypedArena<SemId, SemRecord>,
    /// AST nodes (`/12` PA-slice freeze: typed on freeze).
    pub nodes: TypedArena<NodeId, NodeRecord>,
    /// T04-owned decoded literals (Gate 1 `/7` typed schema; rev-45 exact
    /// ordered fields). The T04 production chips land in slice 2; Gate 1
    /// seeds `G1-CL-01` fixtures directly.
    pub literals: TypedArena<LiteralId, LiteralRecord>,
    /// Folded constants (Gate 1 `/7` typed schema; magnitude-bytes carrier).
    /// T08 appends exactly one `ConstRecord` per evaluation through the
    /// commit materialization path.
    pub consts: TypedArena<ConstId, ConstRecord>,
    /// Layout descriptors (schema owned by T08).
    pub layouts: ReservedArena<LayoutId>,
    /// Initialization plans (schema owned by T08).
    pub inits: ReservedArena<InitId>,
    /// IR functions (`/15` slice freeze: typed on freeze).
    pub functions: TypedArena<FunctionId, FunctionRecord>,
    /// IR basic blocks (`/15` slice freeze: typed on freeze).
    pub blocks: TypedArena<BlockId, BlockRecord>,
    /// IR values (`/15` slice freeze: typed on freeze).
    pub values: TypedArena<ValueId, ValueRecord>,
    /// IR instructions (`/15` slice freeze: typed on freeze).
    pub instructions: TypedArena<InstructionId, InstructionRecord>,
    /// Virtual registers (schema owned by T11).
    pub vregs: ReservedArena<VRegId>,
    /// Continuations (mechanical resume state).
    pub continuations: TypedArena<ContinuationId, ContinuationRecord>,
    /// Tasks.
    pub tasks: TypedArena<TaskId, Task>,
    /// Committed results.
    pub results: TypedArena<ResultId, ResultRecord>,
    /// Committed diagnostics.
    pub diagnostics: TypedArena<DiagnosticId, DiagnosticRecord>,
    /// Host requests.
    pub host_requests: TypedArena<HostRequestId, HostRequestRecord>,
    /// Output artifacts.
    pub artifacts: TypedArena<ArtifactId, ArtifactRecord>,
}

impl Arenas {
    /// Create an empty arena set.
    pub fn new() -> Self {
        Self::default()
    }

    /// Total allocated slots across every arena (never reused).
    pub fn allocated_total(&self) -> u64 {
        self.sources.allocated() as u64
            + self.spans.allocated() as u64
            + self.expansions.allocated() as u64
            + self.pp_tokens.allocated() as u64
            + self.tokens.allocated() as u64
            + self.scopes.allocated() as u64
            + self.scope_events.allocated() as u64
            + self.symbols.allocated() as u64
            + self.types.allocated() as u64
            + self.sem.allocated() as u64
            + self.nodes.allocated() as u64
            + self.literals.allocated() as u64
            + self.consts.allocated() as u64
            + self.layouts.allocated() as u64
            + self.inits.allocated() as u64
            + self.functions.allocated() as u64
            + self.blocks.allocated() as u64
            + self.values.allocated() as u64
            + self.instructions.allocated() as u64
            + self.vregs.allocated() as u64
            + self.continuations.allocated() as u64
            + self.tasks.allocated() as u64
            + self.results.allocated() as u64
            + self.diagnostics.allocated() as u64
            + self.host_requests.allocated() as u64
            + self.artifacts.allocated() as u64
    }
}

/// Job lifecycle state.
///
/// The `Idle` default is the pre-job state. The CT01 job-start transition
/// (`Idle` → `Running`) is control-chip work owned by T02 and is NOT performed
/// here; this type only records the state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobState {
    /// No job has been started.
    Idle,
    /// A job is running.
    Running,
    /// A job finished successfully.
    Finished,
    /// A job finished with errors.
    Failed,
}

impl JobState {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Finished => "finished",
            Self::Failed => "failed",
        }
    }
}

/// Current compiler stage. Phase advance is a control decision owned by T02;
/// this type only records the stage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    /// Job setup.
    Init,
    /// Preprocessing.
    Preprocess,
    /// Lexical analysis.
    Lex,
    /// Parsing.
    Parse,
    /// Symbols and types.
    SymbolsTypes,
    /// Semantic analysis.
    Semantic,
    /// Layout/constants/initialization.
    LayoutInit,
    /// IR lowering.
    Ir,
    /// Optimization.
    Optimize,
    /// Target code generation.
    Target,
    /// Finished.
    Done,
}

impl Stage {
    /// Stable label used in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Preprocess => "preprocess",
            Self::Lex => "lex",
            Self::Parse => "parse",
            Self::SymbolsTypes => "symbols_types",
            Self::Semantic => "semantic",
            Self::LayoutInit => "layout_init",
            Self::Ir => "ir",
            Self::Optimize => "optimize",
            Self::Target => "target",
            Self::Done => "done",
        }
    }
}

/// Persistent control registers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Control {
    /// Lamport tick.
    pub tick: u64,
    /// Job lifecycle state.
    pub job_state: JobState,
    /// Current stage.
    pub stage: Stage,
    /// The task selected this tick, if any.
    pub selected: Option<TaskId>,
    /// Enqueue ordinal counter.
    pub next_enqueue_ordinal: u64,
    /// Diagnostic ordinal counter.
    pub next_diagnostic_ordinal: u64,
    /// Whether the tick/task budget was reported exhausted.
    pub budget_exhausted: bool,
    /// Whether the host requested cancellation (consumed by T02 CT13).
    pub cancel_requested: bool,
}

impl Default for Control {
    fn default() -> Self {
        Self {
            tick: 0,
            job_state: JobState::Idle,
            stage: Stage::Init,
            selected: None,
            next_enqueue_ordinal: 0,
            next_diagnostic_ordinal: 0,
            budget_exhausted: false,
            cancel_requested: false,
        }
    }
}

/// Persistent task queue state. Task records live in `Arenas::tasks`.
///
/// Result consumption is tracked on each [`crate::task::ResultRecord::consumed`]
/// flag, not duplicated here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskStore {
    /// Ready queue in enqueue order.
    pub ready: Vec<TaskId>,
    /// Task selected this tick.
    pub active: Option<TaskId>,
    /// Ephemeral per-tick scheduler batch: tasks dispatched this tick.
    ///
    /// Populated by the dispatcher's distinct pre-worker `Ready → Running`
    /// mutation and cleared at latch only after every dispatched task has a
    /// terminal/`Waiting`/`Progress` outcome (or the H6 bounded recovery).
    /// Clearing this set is not itself a transition and never clears a task's
    /// `Running` state. `reset_wires` at tick start is a wire reset only and
    /// is not an in-flight lifecycle step; a next-tick-start clear is rejected
    /// as latch-residual-incompatible. The exact clear owner/order is a T01
    /// decision. `Waiting` tasks are never in-flight.
    pub in_flight: Vec<TaskId>,
}

/// Minimal per-tick dispatch metrics (`/6` working basis).
///
/// This is the deferred-minimal carrier: dispatched counts only. The full
/// `PipelineMetrics` shape (per-stage counts, fairness cursor, backpressure
/// counters) is deferred to Part B and is NOT defined here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TickMetrics {
    /// Number of tasks dispatched this tick.
    pub dispatched: u32,
}

/// One tick's dispatch record for the canonical bounded bus report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TickRecord {
    /// Tasks dispatched this tick, in dispatch order (canonical).
    pub dispatched: Vec<TaskId>,
    /// Minimal per-tick metrics (dispatched counts only; see [`TickMetrics`]).
    pub metrics: TickMetrics,
    /// Quota-1 projection of the dispatch (derived view of `dispatched`).
    pub selected: Option<TaskId>,
}

/// Structured bus-report failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReportCapacityError {
    /// The bounded report (`max_ticks + 1` records) is full.
    ReportFull {
        /// Configured bound (`max_ticks + 1`).
        limit: u64,
        /// Records that would result.
        requested: u64,
    },
}

impl std::fmt::Display for ReportCapacityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReportFull { limit, requested } => {
                write!(f, "bus report {requested} exceeds limit {limit}")
            }
        }
    }
}

impl std::error::Error for ReportCapacityError {}

/// A worker proposal tagged with its producing chip and task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaggedProposal {
    /// Producing chip.
    pub chip: crate::ids::ChipId,
    /// Producing/selected task.
    pub task: TaskId,
    /// The proposal.
    pub proposal: crate::task::Proposal,
}

/// Per-tick wire bundle. Reset to default at tick start.
#[derive(Clone, Debug, Default)]
pub struct CompilerWires {
    /// Proposals recorded this tick, in emission order.
    pub proposals: Vec<TaggedProposal>,
    /// Task selected this tick.
    pub selected: Option<TaskId>,
    /// A phase advance was requested this tick.
    pub phase_advance_requested: bool,
}

/// A host response frozen into pins for one tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostResponse {
    /// Request being answered.
    pub request: HostRequestId,
    /// Response bytes.
    pub bytes: Vec<u8>,
}

/// Frozen external inputs for one tick.
#[derive(Clone, Debug, Default)]
pub struct CompilerPins {
    /// Host requested a job start (consumed by T02 CT01).
    pub start_job: bool,
    /// Host requested cancellation; the shell records it in `control` for T02.
    pub cancel: bool,
    /// Host responses available this tick (consumed by T02 CT02).
    pub host_responses: Vec<HostResponse>,
    /// The tick budget was reached; the shell marks the job budget-exhausted.
    pub tick_budget_reached: bool,
}

/// The single canonical state carrier for the compiler application.
pub struct CompilerBus {
    /// Read-only job configuration; structurally immutable after construction.
    config: CompilerConfig,
    /// Persistent control registers.
    pub control: Control,
    /// Typed arenas.
    pub arenas: Arenas,
    /// Intern table (the `names` store).
    pub intern: InternTable,
    /// Task queue state.
    pub tasks: TaskStore,
    /// Registered task kinds.
    pub kinds: TaskKindRegistry,
    /// Declared store fields.
    pub schema: crate::manifest::StoreSchema,
    /// Registered chip manifests.
    pub registrations: ManifestRegistry,
    /// Routing table; part of the bus so replay cannot diverge from it.
    pub routing: RoutingTable,
    /// Per-tick proposals.
    pub wires: CompilerWires,
    /// Validated store patches, in commit order.
    pub patch_log: Vec<crate::commit::CommittedPatch>,
    /// Per-store revision counters.
    pub store_versions: crate::commit::StoreVersions,
    /// Canonical bounded tick report (`/6` working basis).
    ///
    /// Bounded by `max_ticks + 1` records. Sole-append-site rule: only the
    /// driver step 6 appends here (the proposed `report.rs`/`driver.rs`; T02).
    /// The current quota-1 routing shell acts as that driver until the
    /// proposed modules land, and appends exactly one record per tick through
    /// [`CompilerBus::push_tick_record`].
    pub report: Vec<TickRecord>,
}

impl CompilerBus {
    /// Create a bus from a configuration, initializing the foundation schema
    /// and the frozen task-kind registry.
    pub fn new(config: CompilerConfig) -> Self {
        Self {
            config,
            control: Control::default(),
            arenas: Arenas::new(),
            intern: InternTable::new(),
            tasks: TaskStore::default(),
            kinds: TaskKindRegistry::foundation(),
            schema: crate::manifest::StoreSchema::foundation(),
            registrations: ManifestRegistry::new(),
            routing: RoutingTable::new(),
            wires: CompilerWires::default(),
            patch_log: Vec::new(),
            store_versions: crate::commit::StoreVersions::new(),
            report: Vec::new(),
        }
    }

    /// Append one tick record to the canonical bounded report.
    ///
    /// Fails with [`ReportCapacityError::ReportFull`] once `max_ticks + 1`
    /// records are retained. The bound uses saturating arithmetic so a
    /// `u64::MAX` tick budget cannot wrap.
    pub fn push_tick_record(&mut self, record: TickRecord) -> Result<(), ReportCapacityError> {
        let limit = self.limits().max_ticks.saturating_add(1);
        let requested = self.report.len() as u64 + 1;
        if requested > limit {
            return Err(ReportCapacityError::ReportFull { limit, requested });
        }
        self.report.push(record);
        Ok(())
    }

    /// The configured resource limits.
    pub fn limits(&self) -> Limits {
        self.config.limits()
    }

    /// The read-only configuration.
    ///
    /// The field is private so a job cannot swap its target/dialect/limits
    /// after initialization; commit also rejects writes to the `config` store.
    pub fn config(&self) -> &CompilerConfig {
        &self.config
    }

    /// Total records counts all arena slots plus the committed patch log.
    pub fn total_records(&self) -> u64 {
        self.arenas.allocated_total() + self.patch_log.len() as u64
    }

    /// Fail if `additional` more records would exceed the total bound.
    pub fn ensure_total_records(&self, additional: u64) -> Result<(), LimitError> {
        let limit = self.limits().max_records_total;
        let requested = self.total_records().saturating_add(additional);
        if requested > limit {
            Err(LimitError::TotalRecords { limit, requested })
        } else {
            Ok(())
        }
    }

    /// Total source bytes held.
    pub fn source_bytes(&self) -> u64 {
        self.arenas
            .sources
            .iter()
            .map(|(_, source)| source.bytes.len() as u64)
            .sum()
    }

    /// Fail if `additional` more source bytes would exceed the bound.
    pub fn ensure_source_bytes(&self, additional: u64) -> Result<(), LimitError> {
        let limit = self.limits().max_source_bytes;
        let requested = self.source_bytes().saturating_add(additional);
        if requested > limit {
            Err(LimitError::SourceBytes { limit, requested })
        } else {
            Ok(())
        }
    }

    /// Ancestor depth of a task.
    ///
    /// Fails with [`LimitError::DanglingParent`] when a parent reference points
    /// at no live task, and with [`LimitError::TaskDepth`] when the bound is
    /// hit. A missing parent is never silently treated as the root.
    pub fn task_depth(&self, task: Option<TaskId>) -> Result<u32, LimitError> {
        let limit = self.limits().max_task_depth;
        let mut depth = 0u32;
        let mut current = task;
        while let Some(id) = current {
            depth = depth.saturating_add(1);
            if depth > limit {
                return Err(LimitError::TaskDepth {
                    limit,
                    requested: depth,
                });
            }
            current = match self.arenas.tasks.get(id) {
                Ok(record) => record.parent,
                Err(_) => return Err(LimitError::DanglingParent { parent: id }),
            };
        }
        Ok(depth)
    }

    /// Fail if `additional` diagnostics would exceed the bound.
    pub fn ensure_diagnostics(&self, additional: u32) -> Result<(), LimitError> {
        let limit = self.limits().max_diagnostics;
        let requested = self
            .arenas
            .diagnostics
            .allocated()
            .saturating_add(additional);
        if requested > limit {
            Err(LimitError::Diagnostics { limit, requested })
        } else {
            Ok(())
        }
    }

    /// Intern a name.
    pub fn intern_name(&mut self, bytes: &[u8]) -> Result<NameId, crate::intern::InternError> {
        let limits = self.limits();
        self.intern.intern(bytes, &limits)
    }

    /// Allocate a source record, enforcing the source-byte and total bounds.
    ///
    /// The content hash is computed here from the bytes; callers cannot supply
    /// an unvalidated hash. Hosts that verified a downloaded artifact must do
    /// so before handing the bytes to the bus.
    pub fn alloc_source(&mut self, name: NameId, bytes: Vec<u8>) -> Result<SourceId, LimitError> {
        self.ensure_source_bytes(bytes.len() as u64)?;
        self.ensure_total_records(1)?;
        let content_hash = crate::codec::sha256(&bytes);
        let limits = self.limits();
        Ok(self.arenas.sources.alloc(
            SourceRecord {
                name,
                bytes,
                content_hash,
            },
            &limits,
        )?)
    }

    /// Seed a task that is immediately eligible.
    ///
    /// **Integration/job-bootstrap only (CT01).** Worker chips must never call
    /// this: they may only propose `Enqueue` through the commit path, which
    /// makes new tasks visible one tick later and enforces the registered
    /// field/kind manifests. This entry point bypasses that next-tick rule for
    /// the initial job task and is used by the host/control integration and by
    /// test fixtures.
    pub fn bootstrap_task(&mut self, draft: TaskDraft) -> Result<TaskId, LimitError> {
        let limit = self.limits().max_queue_len;
        let requested = self.tasks.ready.len() as u32 + 1;
        if requested > limit {
            return Err(LimitError::Queue { limit, requested });
        }
        let id = self.alloc_task(draft, self.control.tick)?;
        self.tasks.ready.push(id);
        Ok(id)
    }

    /// Allocate a task record with an enqueue ordinal, enforcing task bounds.
    pub(crate) fn alloc_task(
        &mut self,
        draft: TaskDraft,
        ready_tick: u64,
    ) -> Result<TaskId, LimitError> {
        let limits = self.limits();
        let depth = self.task_depth(draft.parent)? + 1;
        if depth > limits.max_task_depth {
            return Err(LimitError::TaskDepth {
                limit: limits.max_task_depth,
                requested: depth,
            });
        }
        let requested_tasks = self.arenas.tasks.allocated() as u64 + 1;
        if requested_tasks > limits.max_tasks_total {
            return Err(LimitError::TasksTotal {
                limit: limits.max_tasks_total,
                requested: requested_tasks,
            });
        }
        self.ensure_total_records(1)?;

        let ordinal = self.control.next_enqueue_ordinal;
        let id = self.arenas.tasks.alloc(
            Task {
                id: TaskId::from_index(0),
                kind: draft.kind,
                payload: draft.payload,
                owner: draft.owner,
                parent: draft.parent,
                continuation: draft.continuation,
                state: crate::task::TaskState::Ready,
                enqueue_ordinal: ordinal,
                ready_tick,
                progress_count: 0,
                progress_ordinal: 0,
            },
            &limits,
        )?;
        // Fix up the self ID now that the index is known.
        self.arenas.tasks.get_mut(id)?.id = id;
        self.control.next_enqueue_ordinal += 1;
        Ok(id)
    }

    /// Append a task after a commit preflight has reserved capacity.
    ///
    /// Crate-private and infallible: the commit apply pass must not perform a
    /// fallible step after its first mutation. The caller guarantees the bound
    /// checks were already done.
    pub(crate) fn push_task_reserved(&mut self, draft: TaskDraft, ready_tick: u64) -> TaskId {
        let id = TaskId::from_index(self.arenas.tasks.allocated());
        let ordinal = self.control.next_enqueue_ordinal;
        self.arenas.tasks.push(Task {
            id,
            kind: draft.kind,
            payload: draft.payload,
            owner: draft.owner,
            parent: draft.parent,
            continuation: draft.continuation,
            state: crate::task::TaskState::Ready,
            enqueue_ordinal: ordinal,
            ready_tick,
            progress_count: 0,
            progress_ordinal: 0,
        });
        self.control.next_enqueue_ordinal += 1;
        id
    }

    /// Look up an artifact by key for tests.
    pub fn task_store(&self) -> &TaskStore {
        &self.tasks
    }

    /// Borrow a task.
    pub fn get_task(&self, id: TaskId) -> Result<&Task, crate::arena::ArenaError> {
        self.arenas.tasks.get(id)
    }

    /// Store field-path lookup used by commit validation.
    pub fn store_is_declared(&self, store: StoreId, field: &'static str) -> bool {
        self.schema.fields(store).contains(&field)
    }

    /// The task kind name, when registered.
    pub fn kind_name(&self, kind: TaskKind) -> &'static str {
        self.kinds
            .lookup(kind)
            .map(|entry| entry.name)
            .unwrap_or("unregistered")
    }
}

impl Bus for CompilerBus {
    type Pins = CompilerPins;
    type Wires = CompilerWires;

    fn wires(&self) -> &Self::Wires {
        &self.wires
    }

    fn wires_mut(&mut self) -> &mut Self::Wires {
        &mut self.wires
    }

    fn tick_count(&self) -> u64 {
        self.control.tick
    }

    fn advance_tick(&mut self) {
        self.control.tick = self.control.tick.wrapping_add(1);
    }
}

impl Default for CompilerBus {
    fn default() -> Self {
        Self::new(CompilerConfig::default())
    }
}
