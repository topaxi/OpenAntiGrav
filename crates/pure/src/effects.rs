//! What Wipeout Pure throws and draws in a race: the `oag_title` tables the
//! raceplay loader reads instead of comparing a title's name (ADR-0058).
//!
//! Deliberately thin, like the rest of this crate: Pure's absorb burst is the
//! one effect trigger read (`FUN_08925e20`). Every other trigger is `None`, so
//! Pure draws nothing there rather than borrowing Pulse's law by a fall
//! through. A title package does not depend on another outside
//! `[dev-dependencies]`, so the one Pulse rule Pure takes, the shield tint, is
//! restated here and labelled as inherited.

use oag_title::{
    Burst, EffectSpec, Effects, Looks, Origin, ShieldPalette, ShieldPalettes, Trigger,
    engine_effects,
};

/// Pure's `FUN_08925e20`: the same loop over the same `Ship Collision Fx` class
/// as Pulse's, eight nodes instead of ten, read off its own disassembly -
/// `docs/ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md`.
pub const ABSORB_BURST: Burst = Burst::Sequential {
    cap: 8,
    stagger: 0.1,
};

/// Pure's tables. Hit sparks and the wreck are unread (`Ship_Damage` is not),
/// and Pure's Cannon throws no weapon spark of its own that has been read. The
/// engine's own names are Pulse's, taken by inheritance: Pure's loader tries
/// each and a disc without it costs one report line.
pub const EFFECTS: &Effects = &Effects::engine(Origin::InheritedFrom("Wipeout Pulse")).with(
    Trigger::ShieldAbsorb,
    EffectSpec::new(engine_effects::ABSORB_EFFECT, Origin::Measured).with_burst(ABSORB_BURST),
);

/// Pure's race looks: none read. The shield tint is Pulse's, as it has always
/// been for a title with no palette of its own - the project's rule that an
/// unmeasured title runs Pulse's (`docs/formats/pure-status.md` measures none
/// of the shell).
pub const LOOKS: &Looks = &Looks::unread(ShieldPalettes {
    ps2: ShieldPalette::Ps2Pulse,
    elsewhere: ShieldPalette::Pulse,
    origin: Origin::InheritedFrom("Wipeout Pulse"),
});
