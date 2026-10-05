# Glossary

Every term below is given first in plain language, then with the code
name it maps to. Code names are exact; the plain version is the one to
learn first.

| Term | Plain meaning | Code name |
|---|---|---|
| Pins | The outside world's input for this tick, sampled once and frozen; chips can read it but never change it | `Bus::Pins` |
| Bus | The one shared state carrier: everything persistent plus the current tick's signals | `Bus` (`src/bus.rs`); compiler application: `CompilerBus` |
| Registers | State that persists across ticks (records, queues, results, diagnostics) | fields of the bus struct |
| Wires | Signals valid for the current tick only; reset at every tick start | `Bus::Wires` |
| Chip | A stateless unit that does one deduction per tick; carries no fields | `LogicChip`, `RestrictedChip` |
| Projection | The narrow read-only view prepared for one chip — the only thing it may look at | `RestrictedChip::Input`, built by `ChipAdapter::read` |
| Proposal | A chip's change request: what it wants done, not the deed itself | `RestrictedChip::Output`; compiler vocabulary: `Proposal` |
| Adapter | Trusted application code that builds a chip's projection and commits its proposal | `ChipAdapter` |
| Motherboard | The pipeline array plus clock driver; the only entity that may run chips or reset/latch the bus | `Motherboard` |
| Layer | One pipeline stage; chips in a layer run in insertion order, layers run 0 → 1 → … | `Motherboard::install` |
| Tick | One full sample → propagate → latch cycle | `Motherboard::clock_tick` |
| Backend | The mechanism that executes chips (CPU reference today) — **not** the compiler's codegen target | `Backend`, `CpuBackend` |
| Target | The machine the *compiled program* runs on (AArch64 Linux first) | `compiler` target model; identity `aarch64-unknown-linux-gnu` |
| Host | File access, toolchain calls, artifact storage — everything outside the semantic core | host code / `HostRequest` |
| Task | A unit of compiler work moving Ready → Running → Waiting → Completed/Failed | `Task`, `TaskState` |
| Result | The tagged output of a finished task, consumed exactly once | `ResultValue`, `ResultRecord` |
| Proposal (compiler) | The eight frozen change-request kinds (enqueue, complete, fail, await-host, store-patch, append-records, progress, await-children) | `Proposal`, wire tags 0–7 |
| Commit | The single path that validates proposals and applies them, enforcing ordering and ownership | `compiler/src/commit.rs` |
| Arena | One typed record store; records address each other only through stable IDs | `TypedArena`, `ReservedArena` |
| Record reference | A tagged pointer to a record in exactly one arena (never a raw index) | `RecordRef`, wire tags 0–26 |
| Manifest | A chip's declaration: ID, task kind, exact reads/writes, phase, capability, backend class | `compiler/src/manifest.rs` |
| Snapshot | The canonical byte encoding of all observable state, used for replay and comparison | `compiler/src/snapshot.rs` |
| Frozen (of a contract) | The agreed shapes/names/rules are versioned and hash-pinned; changing them needs an amendment | `t01-c01-c06/6`, hash in `contracts/CONTRACT_VERSION` |
| Probe | Measuring the real target's ABI values on Linux; until attested, codegen stays fail-closed | `tools/torture/probe/` |
| Torture suite | GCC's C test collection; the final acceptance exam, not yet run | [tasks/T00_GCC_TORTURE_GATE.md](../tasks/T00_GCC_TORTURE_GATE.md) |
| SFL | Silicon Formal Language: the semantic source of truth; Rust/C/CUDA/HDL are realizations of it | [architecture/SFL_CONTRACT.md](../architecture/SFL_CONTRACT.md) |

## Two warnings that save confusion

1. **"Backend" ≠ "target."** The framework backend runs chips; the
   compiler target runs compiled programs. They meet only at the edge
   where target emission happens.
2. **"Frozen" ≠ "finished."** A frozen contract fixes the agreement;
   the 331 language chips governed by future agreements do not exist
   yet.
