// ============================================================================
// backend.rs — Backend realization layer
// ============================================================================

//! Backends realize the semantic core on a concrete execution substrate.
//!
//! The reference [`CpuBackend`] runs every chip sequentially, in layer order.
//! Alternative backends (SIMD, GPU, HDL, reversible, remote) may batch, fuse,
//! or emulate chips, but they must preserve the observable tick order and the
//! bus semantics. A backend may keep private caches or device handles as long
//! as they never change observable meaning.
//!
//! See `docs/architecture/SFL_CONTRACT.md` for the full boundary rules.

use crate::bus::Bus;
use crate::chip::LogicChip;

/// A concrete execution strategy for a motherboard's layers.
///
/// Implementors must at minimum report a [`Backend::name`]. The provided
/// [`Backend::execute_layers`] is the reference semantics: every chip runs
/// exactly once, layers in order, chips within a layer in insertion order.
pub trait Backend<B: Bus> {
    /// Human-readable label, e.g. `"cpu"` or `"gpu [device]"`.
    fn name(&self) -> &str;

    /// Execute all layers for one tick.
    ///
    /// Override this to batch, fuse, offload, or emulate work. The observable
    /// effect must remain equivalent to the default implementation.
    fn execute_layers(
        &mut self,
        layers: &[Vec<Box<dyn LogicChip<B>>>],
        pins: &B::Pins,
        bus: &mut B,
    ) {
        for layer in layers {
            for chip in layer {
                chip.tick(pins, bus);
            }
        }
    }
}

/// The reference backend: deterministic, single-threaded, scalar CPU.
#[derive(Clone, Copy, Debug, Default)]
pub struct CpuBackend;

impl<B: Bus> Backend<B> for CpuBackend {
    fn name(&self) -> &str {
        "cpu"
    }
}
