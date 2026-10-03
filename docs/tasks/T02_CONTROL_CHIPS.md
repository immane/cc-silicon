# T02: Control, Scheduling, and Diagnostic Chips

Prerequisite T01/C01–C06. Directory `chips/control/`. Permission upper bound: read `config/control/tasks`, write its own control records and proposals; queues/results may only be modified by the designated commit chips. The chips below are all CPU reference, propagation; called by the motherboard, workers must not call them.

Fixed topology: TaskSelect → TaskGuard → the corresponding worker sub-pipeline → ProposalValidate → TaskCommit → ResultResume/PhaseAdvance. Route by task rather than traversing the entire catalog every tick. PhaseAdvance only looks at completion facts and never performs compilation on its behalf.

| ID / Chip | Input → Output | Function and Goal That Must Be Implemented | Specified Acceptance |
|---|---|---|---|
| CT01 JobStartChip | StartJob → JobReady | Validate configuration, initialize this job's registers, produce the first source task | Duplicate start diagnostic; no leakage across different jobs |
| CT02 SourceResponseChip | HostResponse → SourceImported | Validate by request ID, import immutable bytes/hash and span root | Out-of-order/duplicate responses; identical replay for identical content |
| CT03 TaskSelectChip | ReadyQueue → SelectedTask | Select one task by fixed priority/ordinal; Wait or Done when none ready | Identical selection for identical queues; must not iterate randomly |
| CT04 TaskGuardChip | SelectedTask → GuardDecision | Check owner/kind/preconditions/version; wrong tasks do not enter the worker | Wrong owner, stale version, invalid ID |
| CT05 ProposalValidateChip | Proposals → ValidatedBatch | Check write sets, task result tag, conflicts, single completion | Double Complete, wrong result, unauthorized patch rejected |
| CT06 TaskCommitChip | ValidatedBatch → CommittedTasks | Commit store patches and enqueue/complete/fail in fixed order; new tasks visible next tick | Tasks remain after reset; a single task terminates once |
| CT07 ResultResumeChip | ChildResults → ParentReady | Check that all dependencies are complete, resume the explicit continuation; propagate failures | Multiple child tasks out of order; one failure; result consumed once |
| CT08 HostRequestChip | AwaitSource/Artifact → HostRequestRecord | Form a persistent request and pins/host-readable output; perform no I/O | Request exactly once; Host wait is replayable |
| CT09 PhaseAdvanceChip | PhaseFacts → NextPhase | Enter the next phase only when prerequisite tasks/error policy are satisfied | Do not advance early while ready/wait tasks remain |
| CT10 ProgressBudgetChip | TickFacts → Progress/Fault | Check task/stack/memory budgets and non-advancing loops; do not use wall-clock | Fixed tick budget; legitimate waits are not misjudged as busy-loops |
| CT11 DiagnosticCommitChip | DiagnosticProposals → DiagnosticIds | Commit severity/span/code/notes by stable ordinal and deduplicate | Macro origin, associated task, stable order for multiple errors |
| CT12 RecoverySelectChip | RecoverableFault → RecoveryTask | Choose an explicit recovery strategy by phase, such as synchronizing tokens or terminating the function | Can still report the next error after an error; does not swallow the original diagnostic |
| CT13 CancelJobChip | CancelPin → Cancelled | Cancel ready tasks, wind down Host waits, keep diagnostics and trace | Cancel mid-way; repeated cancel; no spurious artifact |
| CT14 ArtifactFinalizeChip | CompletedStages → ArtifactReady | Validate that no tasks are incomplete, collect output fragments, and publish the Host write request | Missing fragments/failed tasks must not report success |

Standalone tests must construct a miniature task graph and do not require an existing parser. The integrator owns the actual motherboard and Bus::latch; this package must not add implicit worker threads. Before throughput optimization, record the tick/token ratio of each phase and the count of selected chips.
