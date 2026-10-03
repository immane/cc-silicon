// ============================================================================
// motherboard.rs — Motherboard: pipeline array + clock tick driver
// ============================================================================

//! The motherboard physically arranges chips into a layered pipeline and owns
//! the only legal tick ordering. It is the *only* entity that may invoke a
//! chip's `tick`, and the only entity that performs wire reset and latching.

use crate::backend::{Backend, CpuBackend};
use crate::bus::Bus;
use crate::chip::{ChipAdapter, LogicChip, ProjectedChip, RestrictedChip};

/// A deterministic layered pipeline of logic chips.
///
/// `layers[i]` holds every chip at pipeline stage `i`. Chips within a layer
/// execute in insertion order; layers execute `0 → 1 → … → N`. Data flows
/// strictly forward: a chip may read wires written by any earlier chip in the
/// same tick, and registers written by later layers are visible only after
/// the tick completes.
pub struct Motherboard<B: Bus> {
    /// Pipeline stages. Allocated once at construction; never resized during
    /// a tick, so the hot path performs no heap allocation.
    pub layers: Vec<Vec<Box<dyn LogicChip<B>>>>,
    backend: Box<dyn Backend<B>>,
}

impl<B: Bus> Motherboard<B> {
    /// Build a motherboard with `layer_count` empty layers and the CPU backend.
    pub fn new(layer_count: usize) -> Self {
        Self::with_backend(layer_count, Box::new(CpuBackend))
    }

    /// Build a motherboard with `layer_count` empty layers and a chosen backend.
    pub fn with_backend(layer_count: usize, backend: Box<dyn Backend<B>>) -> Self {
        Self {
            layers: (0..layer_count).map(|_| Vec::new()).collect(),
            backend,
        }
    }

    /// Number of pipeline stages.
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Append an empty layer and return its index.
    pub fn push_layer(&mut self) -> usize {
        self.layers.push(Vec::new());
        self.layers.len() - 1
    }

    /// Append a boxed chip to a layer.
    ///
    /// Panics if `layer` is out of range. Panicking on a programming error at
    /// construction time is acceptable; runtime business logic must never
    /// panic.
    pub fn install_chip(&mut self, layer: usize, chip: Box<dyn LogicChip<B>>) {
        assert!(
            layer < self.layers.len(),
            "layer index {layer} out of range (0..{})",
            self.layers.len()
        );
        self.layers[layer].push(chip);
    }

    /// Convenience wrapper that boxes a chip of a concrete type.
    pub fn install<C>(&mut self, layer: usize, chip: C)
    where
        C: LogicChip<B> + 'static,
    {
        self.install_chip(layer, Box::new(chip));
    }

    /// Install a restricted computation with an application-owned projection
    /// and proposal adapter.
    ///
    /// The chip itself receives no bus reference. `adapter.read` defines its
    /// read projection and `adapter.commit` applies its typed proposal. The
    /// adapter is a trusted boundary and must be independently tested.
    pub fn install_projected<C, A>(&mut self, layer: usize, chip: C, adapter: A)
    where
        C: RestrictedChip,
        C::Input: 'static,
        C::Output: 'static,
        A: ChipAdapter<B, C> + 'static,
    {
        self.install(layer, ProjectedChip::new(chip, adapter));
    }

    /// Total number of chips across all layers.
    pub fn chip_count(&self) -> usize {
        self.layers.iter().map(Vec::len).sum()
    }

    /// Label of the active backend.
    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }

    /// Execute one full clock cycle.
    ///
    /// # Phases
    ///
    /// 0. **Wire reset** — pull all wires to ground ([`Bus::reset_wires`]).
    /// 1. **Combinational propagation** — signals flow through the pipeline
    ///    ([`Backend::execute_layers`]).
    /// 2. **Sequential latching** — commit falling-edge state
    ///    ([`Bus::latch`]) and advance the Lamport clock
    ///    ([`Bus::advance_tick`]).
    ///
    /// The pins are already frozen by the caller (the sampling phase lives at
    /// the host boundary, outside the semantic core).
    pub fn clock_tick(&mut self, pins: &B::Pins, bus: &mut B) {
        // Phase 0: wire reset.
        bus.reset_wires();

        // Phase 1: combinational propagation.
        self.backend.execute_layers(&self.layers, pins, bus);

        // Phase 2: sequential latching (falling edge).
        bus.latch(pins);
        bus.advance_tick();
    }
}

impl<B: Bus> Default for Motherboard<B> {
    fn default() -> Self {
        Self::new(4)
    }
}
