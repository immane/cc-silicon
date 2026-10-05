# Chip Plan: all 331 chips graded, waved, and unblocked

| Field | Value |
|---|---|
| Status | **PLAN** — readiness grades and wave assignment, not a freeze, not code authorization |
| Scope | All T02–T13 chip tables (331 unique chip IDs; grep finds 332 rows — see §7) |
| Method | Four parallel file surveys (2026-10-06) against `TASK_TEMPLATE.md` + `T01 §7` frozen baseline |
| Rule | No chip is generated before its wave's schemas, kinds, stages, and manifests are frozen (Gates per `GATE_1_M1_FIRST_SLICE.md` §9A pattern) |

## 1. Readiness grades

- **READY** — every interface the chip touches is frozen in code; only its
  kinds/stages/manifest rows (mechanical T01 serial work) are missing.
- **SPECCABLE** — the proposal shapes exist and are stable enough to write
  the full `TASK_TEMPLATE` header now; code waits on a named freeze.
- **BLOCKED** — an upstream schema, probe result, or corpus census is
  missing; even the header cannot be closed.

## 2. Wave order

| Wave | Content | Unblocks when |
|---|---|---|
| Wave 0 (done) | Foundation C01–C06 + Gate 1 types (`/7`) + worker integration (`/8`) + readiness fixes (`/9`) | — |
| Wave 1 | ONE chip: T08 fold (CL02/CL03 integer subset) on frozen types, enforced template (`/9`: narrow projection, ZST, stage/layer, lint) | template enforced with `c09_readiness` (13 tests); SE02/SE07 and IR03 need unfrozen inputs/outputs and belong to Wave 2 |
| Wave 2 (M1 frontend) | T03 PP ✅ (`/10` PP01 + `/16` PP02–04 M1 + `/20` full-token scan + `/21` dispatch + `/22` conditionals + `/23` macro definitions + `/24` expansion) → T04 LX ✅ (`/11`) → T05 PA ✅ (`/12`) → T06 TY ✅ (`/13`) → T07 SE ✅ (`/14` incl. VF06) → T09 IR ✅ (`/15`; M1-CL-05 handoff complete) → T13 VF12 ✅ (`/17`; symbolic model of return `5`) + VF05 ✅ (`/18`; token-AST contract) + VF01 ✅ (`/19`; store contract) + VF02–04/VF13–14 remainder | Per-slice serial freezes (schemas+kinds+stages+allowlist) in chain order; each slice lands with its own fixture |
| Wave 3 (full C) | Remainder of T02–T10 + T13 VF07–11 | All language schemas frozen; full kind/stage tables; `AppendRecords` for all families |
| Wave 4 (probe-gated) | T11 all; target-dependent T08/T10/T12 parts | Linux probe attested + C02 values incorporated |
| Wave 5 (corpus-gated) | T12 EX34–36 splits, torture-driven gaps | T00 census frozen; new chips registered with ledger entries |

"Generate everything in one pass" happens **per wave**: the plan below
is complete up front so each wave's dispatch is mechanical, but a wave
never starts before its freeze lands — generating against proposals
guarantees rework.

## 3. Group status

| Group | Chips | READY | SPECCABLE | BLOCKED | Wave |
|---|---|---|---|---|---|
| T02 Control | CT01–CT14 (14) | 0 | 14 headers | stage/limit/hook freezes | 2–3 (CT subset for M1 run: CT01/02/06/11/13) |
| T03 Preprocess | PP01–PP28 (28) | 0 | PP01 slice | `PpRequest`/`PpResult`, token-range, map rules | 2 (PP01 slice first) |
| T04 Lex | LX01–LX18 (18) | LX01/02/05–08/16/17 slice scope (3 workers) | LX09–15/18 + forward link + PP04 | records frozen, kinds/stages/allowlist frozen | 2 (M1-LX-01..07 slice ✅ `/11`) |
| T05 Parse | PA01–PA38 (38) | 0 | M1 paths | `NodeKind`, `NodeRecord`, kinds, TU carrier | 2 |
| T06 Symbol/Type | TY01–TY34 (34) | 0 | M1 subset | scope/symbol/type records, File-Enter payload | 2 |
| T07 Semantic | SE01–SE30 (30) | 0 (SE02/SE07 shapes need `SemRecord`) | M1 subset | `SemRecord` link encoding, conversion matrix | 2 |
| T08 Const/Layout/Init | CL01–CL26 (26) | CL02/CL03 int subset (one fold worker) | M1 subset | layout/init schemas, overflow formula | 1 (fold) → 2 |
| T09 IR | IR01–IR29 (29) | 0 (IR03 needs IR records) | M1 subset | IR records, `ir.*` kinds, FunctionEnd hook | 2 |
| T10 Optimize | OP01–OP22 (22) | 0 | all headers | IR freeze + version-guard protocol | 3 |
| T11 Target | CG01–CG39 (39) | 0 | spec text only | probe values, machine records | 4 |
| T12 GNU/Builtins | EX01–EX39 (39) | 0 | EX01–33/37–39 | census splits (EX34–36), sysroot, T11 handoff | 3 + 5 |
| T13 Verify | VF01–VF14 (14) | 0 | VF01–06/12–14 | registrations, H6 fixtures, hook contracts | 2 (subset) → 3 |

## 4. Per-chip plan

Format: `ID name: in → out | needs (unfrozen) | wave/status`.
“—” under *needs* means only mechanical kind/stage/manifest rows.

### T02 Control (CT01–CT14) — all SPECCABLE, Wave 2–3

- CT01 JobStartChip: StartJob+config → JobReady+first task | kinds/stages | 2
- CT02 SourceResponseChip: HostResponse → SourceImported | kinds/stages | 2
- CT03 TaskSelectChip: ready queues → SelectionBatch | `stage_queues`, quota semantics | 2–3
- CT04 TaskGuardChip: dispatched task → GuardDecision | kinds | 2–3
- CT05 ProposalValidateChip: wire proposals → ValidatedBatch | kinds | 2–3
- CT06 TaskCommitChip: ValidatedBatch → CommitDecision | H6 atomic realization | 3
- CT07 ResultResumeChip: child results → join decision | OPEN-02 carrier, replay-retry fixture | 2–3
- CT08 HostRequestChip: AwaitSource/Artifact → HostRequestRecord | kinds | 2
- CT09 PhaseAdvanceChip: PhaseFacts → NextPhase | stage-queues-empty gate | 3
- CT10 ProgressBudgetChip: TickFacts → Progress/Fault | budget semantics | 3
- CT11 DiagnosticCommitChip: diag proposals → DiagnosticIds | kinds | 2
- CT12 RecoverySelectChip: RecoverableFault → RecoveryTask | recovery semantics | 3
- CT13 CancelJobChip: CancelPin → Cancelled | cancel precedence | 2
- CT14 ArtifactFinalizeChip: completed stages → ArtifactReady | kinds | 3

### T03 Preprocess (PP01–PP28) — Wave 2 (PP01 slice first)

- PP01 SourceNormalizeChip: bytes → Normalized+map | ✅ DONE (`/10` PpNormalizeChip, `c10_pp01` 7 tests; lone-CR explicit unsupported; multi-source deferred)
- PP02 LineSpliceChip: Normalized → Spliced | ✅ DONE (`/16` PpSpliceChip, real splice with composed maps) | 2
- PP03 CommentReplaceChip: Spliced → CommentFree | ✅ DONE (`/16` M1 scope + `/20` literal/header-name protection, shared predicate with PP04) | 2
- PP04 PpTokenScanChip: stream → PpToken | ✅ DONE (`/16` M1 subset + `/20` full C11 table, literals, header names) | 2
- PP05 DirectiveDispatchChip: line tokens → DirectiveTask | ✅ DONE (`/21` PpDirectiveChip, `c21_directive` 8 tests; raw walk-back recognition, fan-out + await-all, frozen taxonomy) | 2–3
- PP26 PpDiagnosticChip | ✅ DONE (`/21` PpDiagnosticChip; `#error` fails with the joined message, negative-only by design) | 2
- PP06 MacroDefinitionChip → MacroDef | ✅ DONE (`/23` PpDefineChip, `c23_macro` 10 tests; per-line fan-out, fresh/await/resume, adjacency rule) | 3
- PP07 MacroRedefinitionChip | ✅ DONE (`/23` PpRedefineChip; benign-equivalence verifier, names the differing part) | 3
- PP08 MacroUndefChip | ✅ DONE (`/23` PpUndefChip; tombstones + ignore-unknown) | 3
- PP09 MacroInvocationChip | ✅ DONE (`/24` PpInvokeChip, `c24_expand` 12 tests; fan-out + stitch, directive-verbatim) | 3
- PP10 MacroArgumentCollectChip | folded into the `/24` substituter (promotion criteria in the slice doc) | 3
- PP11 MacroArgumentExpandChip | folded into the `/24` substituter (promotion criteria in the slice doc) | 3
- PP12 MacroSubstituteChip | ✅ DONE (`/24` PpSubstituteChip; prescan + `#`/`##` + blue-paint rescan) | 3
- PP13 MacroStringifyChip | folded into the `/24` substituter (promotion criteria in the slice doc) | 3
- PP14 MacroPasteChip | folded into the `/24` substituter (promotion criteria in the slice doc) | 3
- PP15 MacroRescanChip | folded into the `/24` substituter (promotion criteria in the slice doc) | 3
- PP16 VariadicMacroChip | kinds, VA_OPT policy (variadic USE deferred; definitions recorded) | 3
- PP17 IncludeResolveChip | kinds, path policy | 3
- PP18 IncludeEnterExitChip | kinds, host protocol | 3
- PP19 ConditionalDirectiveChip | ✅ DONE (`/22` PpConditionalChip, `c22_conditional` 8 tests; stack + inline PP-int evaluator, defined frozen-false; PP20–22 folded as pure helpers) | 2–3
- PP20 DefinedOperatorChip | folded into the `/22` evaluator (promotion criteria in the slice doc) | 3
- PP21 PpExpressionParseChip | folded into the `/22` evaluator (promotion criteria in the slice doc) | 3
- PP22 PpExpressionEvaluateChip (PP-int domain, never C evaluator) | folded into the `/22` evaluator (promotion criteria in the slice doc) | 3
- PP23 LineDirectiveChip | kinds | 3
- PP24 BuiltinMacroChip | frozen-target macros | 3
- PP25 PragmaDispatchChip | kinds | 3
- PP27 ExpansionSourceMapChip | origin-chain carrier | 3
- PP28 PreprocessedEmitChip | kinds | 3

### T04 Lex (LX01–LX18) — Wave 2 (M1-LX-01..07 first)

- LX01 TokenClassifyChip | ✅ DONE (`/11` LxClassifyChip: keyword/identifier/punct/integer/Eof, C11 table) | 2
- LX02 IdentifierDecodeChip: spelling → NameId | ✅ DONE (`/11` LxInternChip: first-seen order, 4 M1 names) | 2
- LX03 KeywordClassifyChip | dialect gates | 2
- LX04 PunctuatorDecodeChip | — | 2
- LX05 IntegerRadixChip | ✅ DONE (`/11` decimal-only gate in classify+decode) | 2
- LX06 IntegerSuffixChip | ✅ DONE (`/11` None-only gate) | 2
- LX07 IntegerValueChip (big-int, no host overflow) | ✅ DONE (`/11` decimal_magnitude in decode) | 2
- LX08 IntegerTypeSelectChip → `Int` only | ✅ DONE (`/11` Int-only in decode) | 2
- LX09 FloatSyntaxChip | float schemas | 3
- LX10 FloatValueChip (correct rounding) | target formats | 3–4
- LX11 EscapeDecodeChip | — | 3
- LX12 CharacterLiteralChip | char schemas | 3
- LX13 StringLiteralChip | string schemas | 3
- LX14 AdjacentStringChip | provenance carrier (T03/T04) | 3
- LX15 LiteralExtensionChip | GNU modes | 3–5
- LX16 TokenLocationChip | ✅ DONE (`/11` committed-span reuse, no T04 span writes) | 2
- LX17 TokenPublishChip (sole ordered publisher) | ✅ PARTIAL (`/11` deterministic append/dispatch order; orchestrator fan-in deferred) | 2
- LX18 LexErrorChip (finite advance) | diagnostic codes | 2

### T05 Parse (PA01–PA38) — Wave 2

- PA01 TranslationUnitChip | TU carrier, File-Enter edge | 2 FIRST
- PA02 ExternalDeclarationChip | kinds | 2
- PA03 DeclarationSpecifiersChip | kinds | 2
- PA04 TypedefDisambiguationChip | TY03 query protocol | 2
- PA05 DeclaratorChip (+PA06/07 parts) | kinds | 2
- PA08 ArrayDeclaratorChip | kinds | 2–3
- PA09 FunctionDeclaratorChip | kinds | 2
- PA10 AbstractDeclaratorChip | kinds | 2–3
- PA11 AggregateSpecifierChip | kinds | 2–3
- PA12 EnumSpecifierChip | kinds | 2–3
- PA13 MemberDeclarationChip | kinds | 3
- PA14 DeclarationFinishChip | POD protocol | 2
- PA15 InitializerParseChip | init-tree schema | 2–3
- PA16 PrimaryExpressionChip | kinds | 2
- PA17/18/19 Postfix/Call/MemberSubscript | kinds | 2
- PA20 UnaryExpressionChip | kinds | 2
- PA21 CastExpressionChip | kinds | 2
- PA22 BinaryExpressionChip (precedence climb) | kinds | 2 FIRST
- PA23 ConditionalExpressionChip | kinds | 2
- PA24 AssignmentExpressionChip | kinds | 2
- PA25 CommaExpressionChip | kinds | 2
- PA26 GenericSelectionParseChip | kinds | 3
- PA27 StatementDispatchChip | kinds | 2
- PA28 CompoundStatementChip | scope protocol | 2
- PA29 IfStatementChip | kinds | 2
- PA30 SwitchStatementChip | kinds | 2–3
- PA31 LoopStatementChip (may split ×3 later) | kinds | 2
- PA32 JumpStatementChip | kinds | 2
- PA33 LabelStatementChip | kinds | 2–3
- PA34 ExpressionStatementChip (no infinite retry) | kinds | 2
- PA35 StaticAssertParseChip | kinds | 2–3
- PA36 AttributeParseChip | kinds | 3
- PA37 ExtensionSyntaxDispatchChip → T12 | kinds | 3
- PA38 ParseRecoveryChip (sync `;/)/}`, EOF) | kinds | 2
- All need: `NodeKind` + `NodeRecord` + request/result encoding + kind registrations (one serial freeze).

### T06 Symbol/Type (TY01–TY34) — Wave 2

- TY01 ScopeEnterChip / TY02 ScopeExitChip | scope records, File-Enter payload | 2 FIRST
- TY03 OrdinaryNameLookupChip (active-chain, POD order) | declarator tuples | 2 FIRST
- TY04 TagNameLookupChip / TY05 LabelNameLookupChip / TY06 MemberNameLookupChip | symbol schemas | 2–3
- TY07 SymbolDeclareChip / TY08 TypedefRegisterChip / TY09 RedeclarationChip | symbol schemas, conflict boundary | 2
- TY10 LinkageResolveChip / TY11 StorageDurationChip / TY12 TentativeDefinitionChip | symbol schemas | 2–3
- TY13 BuiltinTypeChip (canonical `int` single-producer) | type schemas | 2 FIRST
- TY14 QualifiedTypeChip / TY15 PointerTypeChip / TY16 ArrayTypeChip | type schemas | 2–3
- TY17 FunctionTypeChip (single-producer `int(void)`) | type schemas | 2
- TY18 AggregateTypeChip / TY19 EnumTypeChip | type schemas | 2–3
- TY20 DeclaratorBindChip | PA05 tree protocol | 2
- TY21 TypeCompatibilityChip / TY22 CompositeTypeChip | M1 deferred (no general model) | 3
- TY23 LvalueConversionChip / TY24 ArrayFunctionDecayChip | conversion schemas | 2–3
- TY25 IntegerPromotionChip (M1 `int→int` identity) | — | 2
- TY26 ArithmeticConversionChip (M1 `int/int`) | — | 2
- TY27 AssignmentConversionChip (M1 `int→int`) | — | 2
- TY28 ArgumentConversionChip | conversion schemas | 2–3
- TY29 ExplicitCastChip | conversion schemas | 3
- TY30 AtomicTypeChip | atomic schemas | 3
- TY31 DeclarationConstraintChip (separate validator) | decl schemas | 2–3
- TY32 InlineLinkageChip | link policy | 3
- TY33 EffectiveTypeChip / TY34 RestrictContractChip (conservative) | access histories | 3

### T07 Semantic (SE01–SE30) — Wave 2 (SE02/SE07 emit requests once `SemRecord` lands)

- SE01 NameExpressionChip | symbol protocol | 2
- SE02 LiteralExpressionChip → emits `ConstantRequest::Literal` | `SemRecord` link encoding | 2
- SE03 UnaryArithmeticChip | conversion schemas | 2–3
- SE04 LogicalOperationChip | kinds | 2
- SE05 AddressExpressionChip / SE06 DereferenceExpressionChip | kinds | 2–3
- SE07 ArithmeticBinaryChip → emits `ConstantRequest::Binary` | `SemRecord` link encoding | 2
- SE08 PointerArithmeticChip | layout protocol | 2–3
- SE09 ShiftBitwiseChip | kinds | 2–3
- SE10 ComparisonChip | kinds | 2
- SE11 AssignmentChip / SE12 CompoundAssignmentChip / SE13 IncrementChip | kinds | 2
- SE14 ConditionalExpressionChip / SE15 CommaExpressionChip | kinds | 2
- SE16 CallExpressionChip | kinds | 2
- SE17 MemberExpressionChip / SE18 SubscriptExpressionChip | kinds | 2–3
- SE19 SizeAlignExpressionChip | layout protocol | 2–3
- SE20 GenericSelectionChip | kinds | 3
- SE21 ReturnStatementChip (M1 `int→int`) | `SemRecord` link | 2
- SE22 LoopJumpChip / SE23 SwitchCaseChip / SE24 LabelGotoChip / SE25 ConditionStatementChip | kinds | 2–3
- SE26 EffectSequencingChip (stable order, UB not defined away) | effect schemas | 2–3
- SE27 VolatileAccessChip / SE28 AtomicAccessChip | effect schemas | 2–3
- SE29 FunctionDefinitionChip (signature carrier, no Return-role) | kinds | 2
- SE30 AlignmentSpecifierChip | layout protocol | 3
- All (except SE02/SE07): `SemRecord` carrier/link encoding + conversion matrix.

### T08 Const/Layout/Init (CL01–CL26) — Wave 1 (fold) → 2

- CL01 ConstantContextChip (legality gate) | const kinds | 2
- CL02 ConstantUnaryChip / CL03 ConstantBinaryChip (int subset; no host overflow; one fold worker proves the pattern) | — | 1
- CL04 ConstantBranchChip | kinds | 2
- CL05 ConstantCastChip | conversion schemas | 2–3
- CL06 AddressConstantChip | symbol protocol | 3
- CL07 StaticAssertChip | kinds | 2–3
- CL08 ScalarLayoutChip … CL13 FlexibleArrayChip (six layout facets) | layout schemas | 2–3
- CL14 MemberOffsetChip | layout schemas | 2–3
- CL15 VlaBoundChip / CL25 VlaLifetimeChip | VLA schemas | 3
- CL16 ScalarInitializerChip … CL21 ZeroInitializeChip (six init facets) | init schemas | 2–3
- CL22 StaticDataEmitChip → T11 | bytes+relocs schema | 3–4
- CL23 AutomaticInitPlanChip | init schemas | 2–3
- CL24 CompoundLiteralChip | object schemas | 3
- CL26 LayoutValidateChip (re-validates CL08–14) | layout schemas | 2–3
- All: overflow formula/enforcement split still open (post-Gate-1 decision).

### T09 IR (IR01–IR29) — Wave 2 (IR03 consumes once IR records land)

- IR01 FunctionBeginChip / IR28 FunctionEndChip (terminal-fact, no marker) | IR records, hook contract | 2
- IR02 BlockCreateChip | IR records | 2
- IR03 ConstantLowerChip (consumes `ConstId`, no refold) | IR records, `ir.*` kinds | 2
- IR04 ObjectAddressLowerChip … IR06 StoreLowerChip | IR records | 2
- IR07 ConversionLowerChip (M1 identity only) | — | 2
- IR08–IR11 Unary/Arithmetic/PointerArith/Compare | IR records | 2
- IR12 ShortCircuitLowerChip / IR13 ConditionalLowerChip (non-SSA join slots) | join-slot freeze | 2
- IR14 AssignmentLowerChip / IR15 CompoundRmwLowerChip | IR records | 2
- IR16 MemberSubscriptLowerChip / IR17 BitFieldLowerChip | IR records | 2–3
- IR18 CallLowerChip | IR records | 2
- IR19 ReturnLowerChip (+ scope/VLA cleanup) | IR records | 2
- IR20 IfLowerChip / IR21 LoopLowerChip / IR22 SwitchLowerChip / IR23 JumpLabelLowerChip | IR records | 2
- IR24 InitLowerChip / IR25 AggregateCopyLowerChip | IR records | 2
- IR26 DynamicStackLowerChip | IR records | 3
- IR27 AtomicLowerChip | IR records | 3
- IR29 ExtensionPlanLowerChip (T12 plans in) | T12 plans | 3

### T10 Optimize (OP01–OP22) — Wave 3 (all SPECCABLE)

- OP01 PassSelectChip | pass-plan matrix | 3
- OP02 FoldIntegerChip / OP03 FoldFloatChip (never call T08; context-free lib only) | IR freeze | 3
- OP04 SimplifyIdentityChip | IR freeze | 3
- OP05 CopyPropagateChip / OP06 LocalValueNumberChip | IR freeze | 3
- OP07 FoldBranchChip / OP08 RemoveUnreachableChip / OP10 CfgSimplifyChip | IR freeze | 3
- OP09 DeadInstructionChip (pure-only delete) | IR freeze | 3
- OP11–OP15 UseDef/Dominators/Frontier/PhiPlacement/SsaRename (one SSA pipeline) | IR freeze, non-SSA input | 3
- OP16 SparseConstantChip | IR freeze | 3
- OP17 AliasAnalysisChip (unknown=may-alias) | IR freeze | 3
- OP18 LoopAnalysisChip / OP19 LoopInvariantChip (no extra faults) | IR freeze | 3
- OP20 ApplyRewriteChip (transactional) / OP21 AnalysisInvalidateChip / OP22 OptFixpointChip (deterministic budget) | version-guard protocol | 3
- None has named fixtures yet — headers must add them (template §5).

### T11 Target (CG01–CG39) — Wave 4, all BLOCKED on probe

- CG01 ScalarAbiClass / CG02 AggregateAbiClass | probe values | 4
- CG03 ArgumentLocation / CG04 ReturnLocation / CG05 VariadicAbi | probe values | 4
- CG06 CallPreservation / CG07 ParameterEntry / CG08 CallSequence / CG09 ReturnSequence | machine records | 4
- CG10–CG25 Select chips (const/alu/mul/div/shift/cmp/branch/addr/mem/float/convert/copy/atomic/intrinsic/asm) | machine records + IR | 4 (CG24/25 need T12 plans)
- CG26 UseDef / CG27 Liveness / CG28 Intervals / CG29 Assign / CG30 Spill / CG31 ParallelMove | machine records | 4
- CG32 FrameLayout / CG33 PrologueEpilogue | machine records | 4
- CG34 SymbolRelocation / CG35 AsmInstruction / CG36 AsmData / CG37 AsmFunction / CG38 TargetVerify | machine records, reloc policy | 4
- CG39 PhiElimination | SSA records | 4
- Spec text writable now against AAPCS64 expectations; values stay expectations until attested.

### T12 GNU/Builtins (EX01–EX39) — Wave 3 + 5

- EX01 ExtensionMode (Std-reject/GNU-allow) | dialect gates | 3
- EX02 Typeof … EX08 SymbolAttribute | type/symbol protocols | 3
- EX09/10 Vector, EX11/12 Complex | target caps | 3–4
- EX13/14/15 Asm chain (syntax→constraints→effect) | target | 3–4
- EX16 BuiltinDispatch (never hides specialists) … EX29 BuiltinControl | builtin protocols | 3
- EX30 NonlocalJump / EX31 GnuInitializer | protocols | 3
- EX32 RuntimeHelper (H01 fixed lib) | helper ABI | 3–4
- EX33 AutoType | dialect gates | 3
- EX34 BitInt / EX35 ExtendedFloat / EX36 C23Compat — **entry dispatchers only; must split per census before execution** | census | 5
- EX37 BuiltinString / EX38 BuiltinMath / EX39 BuiltinPrefetch | protocols | 3

### T13 Verify (VF01–VF14) — Wave 2 (subset) → 3

- VF01 StoreInvariant | ✅ DONE (`/19` Vf01Chip, `c19_vf01` 7 tests; refs-resolve + span-bounds + reserved-unused M1 rules; host-request/tombstone deferred) | 2
- VF02 TaskInvariant / VF03 WireLifetime / VF04 AccessContract | registrations | 2–3 (VF02–04 need H6 fixtures)
- VF05 TokenAstInvariant | ✅ DONE (`/18` Vf05Chip, `c18_vf05` 6 tests; ranges/order/parent-kind + required fields over the closed M1 set; wider syntax deferred) | 2
- VF06 TypedAstInvariant | ✅ DONE (`/14` Vf06Chip incl. in the SE slice; M1 identity-only checks after `SemRecord`s) | 2
- VF07 CfgInvariant / VF08 IrInvariant / VF09 SsaInvariant / VF10 MachineInvariant / VF11 AbiInvariant | IR/machine records | 3 (out of M1)
- VF12 IrInterpret (M1: symbolic, models return `5`) | ✅ DONE (`/17` Vf12Chip, `c17_vf12` 6 tests; single entry block, `Constant`+`Return`, completes modeled `Const`; wider ops deferred) | 2
- VF13 ReplayCompare (canonical projection) | trace protocol | 2
- VF14 EvidenceClassify (never PASS on missing evidence) | T00 protocol | 2–3

## 5. Unblock checklist (serial T01 work before each wave)

- **Before Wave 1 dispatch:** the fold-chip manifest + routing row (kind already frozen). G1-CL-01 stays the acceptance; the test-driver fold is replaced by the real chip through the host driver (Phase 0 template).
- **Before Wave 2:** per-slice freezes in chain order — PP01 slice, LX slice, PA kinds+`NodeRecord`, TY scope/symbol/type records, SE `SemRecord` link, CL M1 subset, IR M1 subset, VF subset registrations. Each lands like Gate 1 (own fixture, own hash section if the seed moves).
- **Before Wave 3:** remaining language schemas, full kind/stage tables, all-family `AppendRecords`, T10 version-guard protocol, named fixtures for every OP chip.
- **Before Wave 4:** attested probe + C02 incorporation (fail-closed until then).
- **Before Wave 5:** frozen T00 census + registered split chips with ledger entries.

## 6. Generation discipline (when waves dispatch)

- One wave at a time; Wave N chips never assume Wave N+1 interfaces.
- Every chip: `TASK_TEMPLATE` header + manifest + normal/boundary/invalid/unsupported/replay/unauthorized-field tests + integration test on real upstream artifacts (fixtures never substitute past Gate 2).
- Shared schemas/routes/registrations stay integrator-owned; chip authors touch only assigned files (`PARALLEL_EXECUTION.md`).
- Unsupported stays a diagnostic + retained failure; faked success is a breach.

## 7. Count note

Table-ID grep finds 332 rows for 331 unique chip IDs: the T07 table lists
SE01–SE30 (30 chips) plus one `VF06` cross-reference row (verifier
registration, T01+T13 owned) that the chip-ID pattern also matches. README's
331 unique count is correct; the one-row delta is that cross-reference, not a
missing chip and not a wrapped row.
