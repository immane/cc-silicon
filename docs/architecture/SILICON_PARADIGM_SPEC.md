# The Silicon-Based Software Architecture Specification

## 1. Abstract

In traditional software engineering — particularly object-oriented programming —
state is encapsulated deep within nested objects. Modules interact through complex
API calls, callbacks, and multithreaded locks. This paradigm inevitably leads to
implicit state explosions, race conditions, unpredictable execution order, and
massive context-switching overhead for both human developers and AI coding agents.

The **Silicon-Based Software Architecture Paradigm** brings genuine **FPGA / ASIC
hardware design philosophy** into pure software development. It completely
eradicates "objects, methods, and API call chains," re-abstracting the software
system into a digital logic circuit composed of a **System Bus, Input Pins, Logic
Gate Chips, and a unified Clock**.

Characterized by **absolute determinism, zero hidden state, and perfect
single-responsibility isolation**, this paradigm is a strong architectural fit for
the era of AI-assisted development (agentic coding).

`cc-silicon` is the reference realization of this specification: a domain-free
Rust framework providing the four primitives, the clock lifecycle, and the
backend boundary.

---

## 2. Core Architectural Principles

The paradigm strictly adheres to three fundamental laws:

1. **Absolute state–logic isolation.** All state data MUST be centrally defined in
   a flat "System Bus / Register File". All business logic MUST be encapsulated in
   pure, stateless "Logic Chips".
2. **Zero API call chains.** Modules are **strictly forbidden** from calling one
   another. Chips may only read signals from the pins and the bus, perform a
   deduction, and write back to the bus. Inter-module communication happens
   exclusively through the topological flow of data on the bus.
3. **Discrete clock ticking.** There is no event-driven mechanism and no thread
   preemption. Everything is triggered by the edge of a unified virtual logic
   clock (a Lamport clock). Every tick enforces a system-wide state alignment
   (latching).

---

## 3. The Four Hardware Primitives

### 3.1 External Input Pins (`Pins`)

Represents the absolute physical stimuli from the external environment during the
current clock cycle (keystrokes, oscillator pulses, network arrivals, sensor
readings).

- **Constraint:** a strictly read-only snapshot. Sampled and frozen by the host
  before every tick. Chips treat these pins as absolute truth.
- **In cc-silicon:** the [`Bus::Pins`] associated type. Applications define a
  concrete plain-old-data struct.

### 3.2 System Bus & Register File (`Bus`)

The memory snapshot of the entire machine, equivalent to PCB traces and state
latches.

- **Registers:** values that persist across clock cycles.
- **Wires (intermediates):** temporary signal lines used within a single clock
  cycle to connect upstream and downstream chips.
- **Constraint:** must be a flat struct — primitives, fixed-size arrays, and plain
  enums. No heap, no interior mutability, no threads.
- **In cc-silicon:** the [`Bus`] trait, with an embedded
  [`Bus::Wires`] bundle so it can be reset in a single assignment.

### 3.3 Micro-Architecture Chips (`LogicChip` / `RestrictedChip`)

Stateless code blocks executing a single, pure logical deduction.

- **Statelessness:** the implementing struct contains zero fields.
- **Single responsibility:** designed to do exactly one concrete thing.
- **Interface:** implements [`LogicChip`], receiving only the pins and the bus.
- **Stricter variant:** [`RestrictedChip`] receives only a read-only input
  projection and returns a typed proposal; a `ChipAdapter` projects the bus
  fields and commits the proposal. Prefer it where field-scoped isolation is
  required; [`LogicChip`] remains the base interface.
- **No return value:** chips communicate by writing to the bus. Errors are "blown
  fuse" wires, never panics or `Result`-driven control flow.

### 3.4 The Timing Motherboard (`Motherboard`)

The pipeline that physically arranges the chips and provides the clock driver. It
is the only entity that may reset wires, define the chip invocation order, or
latch state.

- **In cc-silicon:** [`Motherboard::clock_tick`] with a layered
  `Vec<Vec<Box<dyn LogicChip<B>>>>` pipeline, plus a pluggable
  [`Backend`] that realizes execution.

---

## 4. The Clock Cycle Lifecycle

A single tick must strictly follow three hardware phases:

1. **Sampling phase.** The host polls external I/O and freezes the result into
   `pins`. This lives at the host boundary, outside the semantic core, before
   the tick begins.
2. **Combinational propagation phase.** Signal flows through the pipeline. The
   motherboard iterates the chip layers; each chip reads the pins and the current
   bus, computes, and writes wires (and, where appropriate, registers).
3. **Sequential latching phase.** At the end of the tick (the falling edge), the
   motherboard commits falling-edge state for the next tick and advances the
   Lamport clock.

In cc-silicon, `clock_tick` runs the in-tick phases it numbers 0–2: phase 0
resets every wire ([`Bus::reset_wires`]), phase 1 is the combinational
propagation delegated to the [`Backend`], and phase 2 commits falling-edge state
([`Bus::latch`]) and advances the Lamport clock ([`Bus::advance_tick`]).
Sampling is the host-boundary step that precedes the tick.

---

## 5. Recommended Directory Topology

A compliant project keeps an extremely flat and predictable structure:

```text
src/
├── bus.rs          # [PCB traces] Pins, Wires, and the Bus implementation
├── chips/          # [Logic gates] one file per stateless chip
│   ├── mod.rs
│   ├── decode.rs
│   └── resolve.rs
├── motherboard.rs  # [Top module] pipeline assembly (may live in main)
└── main.rs         # [Peripherals] host sampling, I/O drivers, display output
```

`cc-silicon` supplies the bus/chip/motherboard/backend machinery; an application
supplies the domain types and the pipeline topology.

---

## 6. Why AI Agents Excel in this Paradigm

When assigning a feature request in traditional OOP, an agent must comprehend
complex inheritance trees and side effects. Under the silicon paradigm an agent
factory operates far more reliably:

1. **Extreme context locality.** The bus file contains the entire universe of the
   application. It is often sufficient context on its own.
2. **Concrete instructions.** Prompts become mathematically precise: *"Write a
   chip that reads the `request` wire and the `queue` register, and writes the
   `grant` wire."* Generation accuracy improves sharply.
3. **Non-destructive plug-and-play.** Once generated, a chip is simply pushed into
   the motherboard's layer array. There are no module APIs to break.

---

## 7. The Absolute Commandments (Constraints)

To maintain the physical purity of the system, developers and agents must never
cross these red lines:

- **NO global mutable state.** With the exception of the bus passed during a tick,
  `static mut` and singleton patterns are strictly prohibited.
- **NO implicit control flow.** Do not use exceptions or `panic!` to control
  business logic. Model errors as blown-fuse signals on the bus.
- **NO privilege escalation.** A chip must only read/write bus fields relevant to
  its specific duty.
- **NO hidden semantic state.** A backend may keep private caches or device
  handles only if they never change observable meaning. Scheduling and ordering
  state is semantic state: it belongs on the bus, not in the backend.
- **Testbenches over anecdotes.** Verify via simulation: inject deterministic (and
  randomized) pin sequences and assert that the bus never violates its
  invariants.

Many of these rules are enforced structurally by Rust and are documented in the
[architectural blueprint](../design/ARCHITECTURAL_BLUEPRINT.md).
