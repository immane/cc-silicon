# SFL Schema Draft

This draft turns the SFL contract into a structured document shape that can be
validated by tooling. It is intentionally conservative: the schema favors
explicitness, backend capability declarations, and diff-friendly static data over
clever abstractions.

The draft is not a final parser spec. It is the nearest practical boundary for an
executable contract.

---

## 1. Document Shape

An SFL document is a single YAML object with these top-level keys:

- `sfl_version`
- `project`
- `semantic_core`
- `bus`
- `pins`
- `wires`
- `chips`
- `motherboard`
- `backends`
- `verification`
- `experimental`

Minimal shape:

```yaml
sfl_version: 0.1-draft
project:
  name: example-silicon
  mode: silicon-formal
semantic_core: {}
bus: {}
pins: {}
wires: {}
chips: []
motherboard: {}
backends: []
verification: {}
experimental: {}
```

---

## 2. Normative Field Rules

### 2.1 `sfl_version`

Required string. Identifies the schema family, allows compatibility checks, and
enables future migration. Recommended values: `0.1-draft`, `0.2-draft`, `1.0`.

### 2.2 `project`

Required object. Suggested keys:

- `name`: string
- `mode`: string
- `owner`: string or array of strings
- `description`: string

The `mode` value should describe the semantic domain, not the runtime backend.

### 2.3 `semantic_core`

Required object. States the invariant meaning of the document. Suggested keys:

- `tick_model`: `sample-propagate-latch`
- `state_model`: `pins-bus-wires`
- `equivalence_target`: `SFL`
- `determinism`: boolean
- `hidden_state_policy`: `allow-implementation-only`
- `mutation_policy`: `bus-only`
- `ordering_policy`: `layered`

### 2.4 `pins`

Required object. Defines frozen external inputs for one tick.

Required keys: `fields` (array of field declarations).

Field declaration shape:

```yaml
- name: frame_delta_ns
  type: u64
  readonly: true
  frozen_per_tick: true
  description: Wall-clock time since previous tick
```

Required field properties: `name`, `type`, `readonly` (must be true),
`frozen_per_tick` (must be true), `description`.

Allowed examples for `type`: `bool`, `u8`, `u16`, `u32`, `u64`, `i8`, `i32`,
`enum:<Name>`.

### 2.5 `wires`

Required object. Defines per-tick transient signals.

Required keys:

- `reset_policy`: must describe default reset at tick start
- `fields`: array of field declarations

Each wire field should include `name`, `type`, `default`, `scope` (which must be
`tick`), and `description`.

### 2.6 `bus`

Required object. Defines persistent machine state.

Required keys:

- `layout`: `flat`
- `fields`: array of register declarations
- `derived_fields`: optional array of computed values
- `reset_policy`: optional, but if present it must be explicit per field

Each bus field declaration should include `name`, `type`, `kind`
(`register` | `derived` | `cache` | `private_backend`), `mutability`
(`readonly` | `mutable`), `lifetime` (`tick` | `persistent`), and `description`.

### 2.7 `chips`

Required array. Each chip entry describes one logical transform.

Required keys: `name`, `category`, `phase`, `reads`, `writes`, `deterministic`,
`backend_class`, `priority`, `description`.

Chip entry shape:

```yaml
- name: EdgeDetect
  category: portable
  phase: combinational
  reads:
    pins: [frame_delta_ns]
    bus: [prev_level]
  writes:
    bus: []
    wires: [rising_edge]
  deterministic: true
  backend_class: portable
  priority: 20
  description: Detects a rising edge on the sampled level input
```

#### Chip field rules

- `reads.pins` must list only declared pin fields.
- `reads.bus` must list only declared bus fields.
- `writes.bus` must list only declared bus fields.
- `writes.wires` must list only declared wire fields.
- `writes` may be empty only for pure observer chips.
- `deterministic` should be true for all default-runtime chips.
- `backend_class` must match the compatibility tier.

### 2.8 `motherboard`

Required object. Declares the legal tick order.

Required keys:

- `layering`: array of layer objects
- `latch_policy`: description of end-of-tick commits
- `wire_reset_policy`: description of tick-start reset

Each layer object should include `index`, `name`, `chips` (ordered array of chip
names), and `semantic_role`.

### 2.9 `backends`

Required array. Each backend entry describes how it realizes the contract.

Required keys: `name`, `kind`, `status`, `supports`, `fallback_policy`,
`adapter_policy`, `verification`.

Backend entry shape:

```yaml
- name: cuda
  kind: gpu
  status: experimental
  supports:
    pins_sampling: true
    flat_bus: true
    wire_reset: true
    per_chip_execution: partial
    layer_batching: true
    deterministic_replay: true
    hardware_synthesis: false
  fallback_policy:
    unsupported_chip: emulate_or_cpu_fallback
    runtime_failure: cpu_fallback
  adapter_policy:
    allowed: [batching, emulation, host_shim]
    forbidden: [silent_semantic_drift]
  verification:
    diff_against: cpu-reference
```

Suggested `kind` values: `reference`, `cpu`, `gpu`, `hdl`, `quantum`, `host`.

Suggested `status` values: `stable`, `experimental`, `partial`, `unsupported`.

### 2.10 `verification`

Required object. Makes the schema executable in practice.

Required keys: `property_tests`, `replay_tests`, `diff_tests`,
`capability_tests`, `synthesis_tests`.

Suggested test declaration shape:

```yaml
- name: deterministic_same_input_same_output
  target: tick
  kind: property
  backend: cpu-reference
  inputs:
    strategy: random_pins
  assert:
    - state_equivalence
    - no_out_of_bounds
```

### 2.11 `experimental`

Optional object. Holds work intentionally outside default semantics. Suggested
keys: `enabled`, `notes`, `scoped_features`, `backend_only`.

---

## 3. Validation Rules

An SFL validator should reject documents that violate any of the following:

1. A pin field is not marked read-only.
2. A wire field does not reset at tick start.
3. A bus field is not classified.
4. A chip omits read or write declarations.
5. A chip writes to undeclared fields.
6. A backend claims a capability it does not actually support.
7. A backend is marked stable while relying on silent fallback semantics.
8. A layer order is unspecified or cyclic.
9. Verification entries do not point at a concrete target.

---

## 4. Example Draft

```yaml
sfl_version: 0.1-draft
project:
  name: example-silicon
  mode: silicon-formal
  description: A generic synchronous circuit with multiple backend realizations

semantic_core:
  tick_model: sample-propagate-latch
  state_model: pins-bus-wires
  equivalence_target: SFL
  determinism: true
  hidden_state_policy: allow-implementation-only
  mutation_policy: bus-only
  ordering_policy: layered

pins:
  fields:
    - name: frame_delta_ns
      type: u64
      readonly: true
      frozen_per_tick: true
      description: Wall-clock nanoseconds since previous tick
    - name: pulse
      type: bool
      readonly: true
      frozen_per_tick: true
      description: Level-sensitive input

wires:
  reset_policy: reset-to-default-at-tick-start
  fields:
    - name: rising_edge
      type: bool
      default: false
      scope: tick
      description: High for one tick on a rising edge

bus:
  layout: flat
  fields:
    - name: value
      type: u32
      kind: register
      mutability: mutable
      lifetime: persistent
      description: Accumulated value
    - name: wires
      type: Wires
      kind: cache
      mutability: mutable
      lifetime: tick
      description: Per-tick signal bundle

chips:
  - name: EdgeDetect
    category: portable
    phase: combinational
    reads:
      pins: [pulse]
      bus: [prev_pulse]
    writes:
      bus: []
      wires: [rising_edge]
    deterministic: true
    backend_class: portable
    priority: 10
    description: Detects a rising edge on the sampled input

motherboard:
  layering:
    - index: 0
      name: decode
      chips: [EdgeDetect]
      semantic_role: sampling-to-decoding
  latch_policy: commit-registers-and-edge-latches-at-tick-end
  wire_reset_policy: reset-all-wires-before-layer-execution

backends:
  - name: cpu-reference
    kind: reference
    status: stable
    supports:
      pins_sampling: true
      flat_bus: true
      wire_reset: true
      per_chip_execution: true
      layer_batching: false
      deterministic_replay: true
      hardware_synthesis: false
    fallback_policy:
      unsupported_chip: reject
      runtime_failure: fail-fast
    adapter_policy:
      allowed: [none]
      forbidden: [silent_semantic_drift]
    verification:
      diff_against: itself

verification:
  property_tests:
    - name: deterministic_same_input_same_output
      target: tick
      kind: property
      backend: cpu-reference
      inputs:
        strategy: random_pins
      assert:
        - state_equivalence
        - no_out_of_bounds
  replay_tests: []
  diff_tests: []
  capability_tests: []
  synthesis_tests: []

experimental:
  enabled: false
  notes: Experimental sections are opt-in only.
  scoped_features: []
  backend_only: []
```

---

## 5. Practical Boundary Guidance

When the schema feels too strict, prefer one of these outcomes:

- add a capability flag,
- add a backend-specific adapter,
- split a chip into a portable core and a device-specific wrapper,
- or classify the feature as experimental.

The draft should not be relaxed by removing semantic information. It should be
relaxed by making the boundary explicit.
