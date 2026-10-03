// ============================================================================
// chip.rs — LogicChip: a stateless transition unit
// ============================================================================

//! Logic chips are the combinational logic of the circuit.
//!
//! A chip is a zero-field unit struct that performs exactly one deduction per
//! clock tick. It reads the frozen [`crate::Bus::Pins`] and the current bus,
//! then writes wires and (where appropriate) registers.
//!
//! # Rules
//!
//! * **Stateless**: the implementing type carries no fields.
//! * **No chip-to-chip calls**: chips communicate only through the bus.
//! * **Deterministic**: identical `(pins, bus)` input yields identical output.
//! * **No privilege escalation**: a chip touches only the fields relevant to
//!   its single responsibility.

use crate::bus::Bus;

/// A stateless logic gate that performs one pure deduction per clock tick.
///
/// # Borrow-checker safety
///
/// `&self` (a zero-sized value) and `&mut B` are distinct allocations, so
/// there is no aliasing conflict. The motherboard executes chips sequentially,
/// which means exactly one `&mut B` exists at any moment. No `RefCell`, no
/// `Mutex`, no `unsafe`.
pub trait LogicChip<B: Bus> {
    /// Execute one clock cycle of logic for this chip.
    ///
    /// Reads the frozen pins and the current bus (including wires written by
    /// earlier chips this tick), computes new values, and writes them back to
    /// the bus. Returns `()`. Errors are modelled as "blown fuse" signals on
    /// the bus, never as panics or `Result` used for control flow.
    fn tick(&self, pins: &B::Pins, bus: &mut B);
}

/// A restricted, side-effect-free chip computation.
///
/// Unlike [`LogicChip`], this trait does not expose the bus. It consumes an
/// immutable projection prepared by a [`ChipAdapter`] and returns an output
/// proposal. The chip's output cannot mutate application state by itself.
///
/// Use [`crate::silicon_chip!`] to declare chips as unit structs and get a
/// compile-time zero-size assertion. This cannot prove that an implementation
/// avoids global state, I/O, clocks, or nondeterministic dependencies; those
/// require linting and tests.
pub trait RestrictedChip: Sized + 'static {
    /// Compile-time assertion evaluated when the chip is installed. This
    /// rejects non-zero-sized implementations even when they do not use the
    /// declaration macro.
    const ASSERT_STATELESS: () = assert!(::core::mem::size_of::<Self>() == 0);

    /// The exact read-only input view for this chip.
    type Input;

    /// The proposal emitted by this chip, applied by its adapter.
    type Output;

    /// Compute a proposal from an immutable input snapshot.
    fn compute(&self, input: &Self::Input) -> Self::Output;
}

/// Trusted boundary between the application's full bus and one restricted
/// chip. Implementations define the chip read set in [`ChipAdapter::read`] and
/// its write set in [`ChipAdapter::commit`]. The framework cannot inspect
/// field accesses inside this adapter; keep it small and test both sets.
pub trait ChipAdapter<B: Bus, C: RestrictedChip> {
    /// Project only the fields declared in this chip's read manifest.
    fn read(&self, pins: &B::Pins, bus: &B) -> C::Input;

    /// Validate and apply this chip's typed proposal to declared write fields.
    fn commit(&self, proposal: C::Output, bus: &mut B);
}

/// Adapter exposing a restricted chip through the existing motherboard API.
pub struct ProjectedChip<B: Bus, C: RestrictedChip, A: ChipAdapter<B, C>> {
    chip: C,
    adapter: A,
    marker: std::marker::PhantomData<fn(&mut B)>,
}

impl<B: Bus, C: RestrictedChip, A: ChipAdapter<B, C>> ProjectedChip<B, C, A> {
    /// Construct a restricted chip stage.
    pub fn new(chip: C, adapter: A) -> Self {
        let () = C::ASSERT_STATELESS;
        Self {
            chip,
            adapter,
            marker: std::marker::PhantomData,
        }
    }
}

impl<B, C, A> LogicChip<B> for ProjectedChip<B, C, A>
where
    B: Bus,
    C: RestrictedChip,
    C::Input: 'static,
    C::Output: 'static,
    A: ChipAdapter<B, C> + 'static,
{
    fn tick(&self, pins: &B::Pins, bus: &mut B) {
        let input = self.adapter.read(pins, bus);
        let proposal = self.chip.compute(&input);
        self.adapter.commit(proposal, bus);
    }
}

/// Declare a restricted chip as a zero-field unit struct.
///
/// The macro accepts only a unit-struct declaration and emits a compile-time
/// zero-size assertion. The chip body receives only its immutable input and
/// can return only its typed proposal.
///
/// ```
/// use cc_silicon::{silicon_chip, RestrictedChip};
/// struct Input { amount: u32 }
/// #[derive(Debug, PartialEq)] struct Proposal(u32);
/// silicon_chip! {
///     struct AddChip;
///     impl RestrictedChip for AddChip {
///         type Input = Input;
///         type Output = Proposal;
///         fn compute(&self, input: &Self::Input) -> Self::Output {
///             Proposal(input.amount + 1)
///         }
///     }
/// }
/// assert_eq!(std::mem::size_of::<AddChip>(), 0);
/// assert_eq!(AddChip.compute(&Input { amount: 2 }), Proposal(3));
/// ```
///
/// A field-bearing chip declaration is rejected:
///
/// ```compile_fail
/// use cc_silicon::{silicon_chip, RestrictedChip};
/// struct Input; struct Proposal;
/// silicon_chip! {
///     struct StatefulChip { hidden: u32 };
///     impl RestrictedChip for StatefulChip {
///         type Input = Input; type Output = Proposal;
///         fn compute(&self, _: &Input) -> Proposal { Proposal }
///     }
/// }
/// ```
///
/// A manual implementation with state is also rejected when used:
///
/// ```compile_fail
/// use cc_silicon::RestrictedChip;
/// struct StatefulChip(u32);
/// impl RestrictedChip for StatefulChip {
///     type Input = ();
///     type Output = ();
///     fn compute(&self, _: &()) {}
/// }
/// fn require_stateless<C: RestrictedChip>() { let () = C::ASSERT_STATELESS; }
/// require_stateless::<StatefulChip>();
/// ```
///
/// Direct bus access is unavailable inside a chip body:
///
/// ```compile_fail
/// use cc_silicon::{silicon_chip, RestrictedChip};
/// struct Input; struct Proposal;
/// silicon_chip! {
///     struct IsolatedChip;
///     impl RestrictedChip for IsolatedChip {
///         type Input = Input; type Output = Proposal;
///         fn compute(&self, _: &Input) -> Proposal { bus.secret = 1; Proposal }
///     }
/// }
/// ```
///
/// The projected input is immutable:
///
/// ```compile_fail
/// use cc_silicon::{silicon_chip, RestrictedChip};
/// struct Input { value: u32 }
/// struct Proposal;
/// silicon_chip! {
///     struct MutatingChip;
///     impl RestrictedChip for MutatingChip {
///         type Input = Input; type Output = Proposal;
///         fn compute(&self, input: &Self::Input) -> Proposal {
///             input.value = 1;
///             Proposal
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! silicon_chip {
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident;
        impl RestrictedChip for $name2:ident {
            type Input = $input:ty;
            type Output = $output:ty;
            fn compute(&self, $arg:ident: &Self::Input) -> Self::Output $body:block
        }
    ) => {
        $(#[$meta])*
        $vis struct $name;
        const _: () = assert!(::core::mem::size_of::<$name>() == 0);
        impl $crate::chip::RestrictedChip for $name2 {
            type Input = $input;
            type Output = $output;
            fn compute(&self, $arg: &Self::Input) -> Self::Output $body
        }
    };
}
