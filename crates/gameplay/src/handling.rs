//! Mapping the on-disc handling schema onto the physics parameter set.
//!
//! Two types describe the same numbers: [`oag_formats::handling::Stats`] is the
//! **document**, cameras and front-end bars included, and
//! [`oag_physics::Handling`] is what the **force law consumes**. They are
//! deliberately separate so `oag-physics` depends on nothing but `oag-core`, and
//! this module is the only bridge between them. See
//! `docs/architecture/workspace-layout.md`.
//!
//! The cost of that separation is this file, and the cost is paid on purpose:
//! it means an attribute added to the XML cannot silently reach the simulation,
//! and it gives one place to look when a parameter behaves unexpectedly. The
//! field names on both sides are identical to the XML's own attribute names, so
//! the mapping is deliberately boring - if a line here is doing anything other
//! than moving one number, that is a bug or an undocumented decision.
//!
//! # Four fields are scaled, and that is not this layer taking a liberty
//!
//! The original's own XML parsers do not store five of these values verbatim:
//! `Engine.amount` is stored as `xml * 0.001`, `Brakes.amount` as
//! `xml * -0.01`, and both `Airbrake.amount` and `Airbrake.slidegrip` as
//! `xml * 0.0001`. Recovered from `HandlingXml_ParseEngine`,
//! `HandlingXml_ParseBrakes` and `HandlingXml_ParseAirbrake` at confidence 88;
//! see `docs/ghidra/functions/psp-pulse-usa/engine.md`.
//!
//! The fifth is `AirbrakeGraphics.amount`, stored as `xml * 0.017453292` -
//! `pi/180`, so the parameter is an **angle in radians** and not the bare number
//! the document holds. `HandlingXml_ParseAirbrakeGraphics` at confidence 92; see
//! `docs/ghidra/functions/psp-pulse-usa/camera.md`, which found it while mapping the
//! camera block it sits directly after. It is converted by
//! [`airbrake_graphics_for`] rather than by [`handling_for`], because the flaps
//! are graphics and the physics parameter set has no business holding them.
//!
//! So there are genuinely two forms of these numbers - the document's and the
//! one the craft code reads - and the conversion has to happen somewhere. It
//! happens here, at the same point in the pipeline the original does it, which
//! leaves `oag_formats::handling` describing the document faithfully (its
//! ground-truth test asserts `slidegrip` lies in `0..=100`, which is the XML
//! range) and `oag_physics::Handling` holding the in-memory form the force law
//! expects.
//!
//! **The scaling must be applied exactly once.** Applying it in the force law as
//! well is the most likely integration bug in this whole area, which is why
//! `oag_physics::params` says outright that it holds the scaled form, and why
//! [`SCALED_FIELDS`] exists to make the set greppable.
//!
//! Nothing else is scaled, converted or defaulted. Units of the remaining
//! fields are the game's own; see `docs/formats/handling-stats.md`.
//!
//! # This file *is* the seam
//!
//! Stage 6 of the engine/title split ([ADR-0022]) asked where the boundary
//! between "a number off the player's disc" and "a number out of the original's
//! code" runs. For the per-team parameters the answer is here and needed no
//! change: [`handling_for`] is the only place a document value becomes a
//! simulation value, [`SCALED_FIELDS`] is the exhaustive list of the ones that
//! are transformed on the way, and a test asserts that list has not grown.
//!
//! What that buys when a second title arrives: its `handlingstats.xml` reaches
//! the force law through this one function, so a schema that authors a field
//! differently is a change to one mapping rather than a hunt through
//! `oag-physics`. What it does **not** buy is the constants the original
//! compiled in - those sit beside the laws that read them in `oag-physics`, and
//! `oag_physics::params`' module docs say which side of the line they are on and
//! why none of them moved.
//!
//! [ADR-0022]: ../../../docs/architecture/adr/0022-title-packages.md

use oag_formats::handling::{self as fmt, SpeedClass as FmtClass};
use oag_physics::params::{
    Airbrake, Antigrav, Brakes, Dimensions, Engine, Handling, Physical, Pitch, SpeedClass,
    SpeedupPads, Turning,
};

/// Load-time factor applied to `<Engine amount>`.
///
/// `HandlingXml_ParseEngine`. See the module docs.
pub const ENGINE_AMOUNT_SCALE: f32 = 0.001;

/// Load-time factor applied to `<Brakes amount>`.
///
/// **Negative**: the stored value is negative, and the brake term applies force
/// along `+normalize(velocity)`, so the sign lives in the parameter rather than
/// at the use site. Negating again where it is used would accelerate under
/// braking. `HandlingXml_ParseBrakes`.
pub const BRAKES_AMOUNT_SCALE: f32 = -0.01;

/// Load-time factor applied to `<Airbrake amount>`.
///
/// `HandlingXml_ParseAirbrake`.
pub const AIRBRAKE_AMOUNT_SCALE: f32 = 0.0001;

/// Load-time factor applied to `<Airbrake slidegrip>`.
///
/// This factor is what turns `slidegrip` from an interpretation into
/// arithmetic: an XML value on `0..100` becomes `0..0.01`, so the grip term's
/// `(0.01 - slidegrip)` reaches exactly zero at `slidegrip = 100`, which is the
/// "percent of grip retained" reading `docs/physics/README.md` had only been
/// able to infer. `HandlingXml_ParseAirbrake`.
pub const SLIDEGRIP_SCALE: f32 = 0.0001;

/// Load-time factor applied to `<AirbrakeGraphics amount>`: degrees to radians.
///
/// The odd one out, and the reason it is worth a constant of its own: the other
/// four scales are unit-less magnitude conversions, and this one says the
/// parameter is an **angle**. `HandlingXml_ParseAirbrakeGraphics` (`0x08839c68`)
/// stores `amount * 0.017453292`, and the value is `pi/180` to seven digits;
/// `up_speed` and `down_speed` beside it are stored raw. Read out of the loader
/// and corroborated against the live block, which holds one team's authored `25`
/// as `0.4363323`. Confidence **92**; see
/// `docs/ghidra/functions/psp-pulse-usa/camera.md`.
pub const AIRBRAKE_GRAPHICS_AMOUNT_SCALE: f32 = 0.017_453_292;

/// The five fields that are not stored verbatim, for grepping.
///
/// Exists so that "which parameters are pre-scaled?" has one answer in the
/// repository rather than five scattered constants, and so a test can assert the
/// set has not silently grown.
pub const SCALED_FIELDS: [&str; 5] = [
    "Engine.amount",
    "Brakes.amount",
    "Airbrake.amount",
    "Airbrake.slidegrip",
    "AirbrakeGraphics.amount",
];

/// How far and how fast the airbrake flaps move, in the form the game holds.
///
/// The runtime twin of [`fmt::AirbrakeGraphics`], which describes the document.
/// The one difference is the whole reason this type exists: `amount` here is
/// **radians**, because that is what the original's loader stores.
///
/// Deliberately **not** part of `oag_physics::params::Handling`. The flaps are a
/// model animation and touch no force term - `Ship_UpdateAirbrakes` never reads
/// this block - so putting it in the physics parameter set would be claiming a
/// coupling that is not there, and would make a graphics tweak move the
/// determinism hashes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AirbrakeGraphics {
    /// Flap deflection at full airbrake, in **radians**.
    pub amount: f32,
    /// Rate the flap returns at. Stored raw.
    pub down_speed: f32,
    /// Rate the flap deploys at. Stored raw.
    pub up_speed: f32,
}

/// Converts `<AirbrakeGraphics/>` into the form the game holds.
///
/// Separate from [`handling_for`] because the block is **per team**, on the
/// enclosing [`fmt::Stats`], rather than per speed class - the same asymmetry
/// `<Misc>` has - and because nothing in the physics parameter set wants it.
#[must_use]
pub fn airbrake_graphics_for(stats: &fmt::Stats) -> AirbrakeGraphics {
    AirbrakeGraphics {
        amount: stats.airbrake_graphics.amount * AIRBRAKE_GRAPHICS_AMOUNT_SCALE,
        down_speed: stats.airbrake_graphics.down_speed,
        up_speed: stats.airbrake_graphics.up_speed,
    }
}

/// The physics parameter set for one team in one speed class.
///
/// `<Misc>` is per team rather than per class, so [`Dimensions`] comes from the
/// enclosing [`fmt::Stats`] while everything else comes from the class block.
/// That asymmetry is the format's, not ours.
///
/// `speedup_pads` and `special` are arguments rather than lookups for a sharper
/// version of the same reason: both are authored in a **different file**,
/// `Data\XML\HandlingStats.xml`, which [`fmt::Stats`] does not describe. Passing
/// them in makes the caller decide what a ship gets when that file is unreadable,
/// which is a decision worth being explicit about - a defaulted zero here would
/// be a game whose speed pads silently do nothing.
///
/// `special` carries four fields this crate reads - `speedpad_jump` and the
/// barrel roll's `roll_cost`/`roll_speed`/`roll_turbotime` - and unlike
/// `speedup_pads` none of them is **per speed class**: each is a single
/// `.bss` float in the original (`g_speedpad_jump` at `0x08b36bec` and its
/// three neighbours), not a four-entry table.
///
/// The four pre-scaled fields are converted here; every other field is moved
/// unchanged, both of these included. See the module docs.
/// The `SkillLevel` race option's own default, which is what picks a ship's
/// shield pool out of `<Misc>`'s three slots.
///
/// `Race_ReadSetupOptions` (`0x08896b84`) sets `g_skill_level` to **1** whenever
/// the setup carries no `SkillLevel` key, and 1 is the middle rung - the one no
/// shipped file authors, so it falls back to plain `<Misc shield>`. See
/// `docs/ghidra/functions/psp-pulse-usa/shield.md`.
///
/// **A constant rather than a parameter, deliberately.** Nothing selects a skill
/// level yet: there is no menu row for it and no race option carrying it, so a
/// parameter would be four call sites all passing the same literal. Wiring the
/// option is the follow-up, and `oag_formats::handling::Misc::shield_for` already
/// takes the index so that when it lands, only this line moves.
pub const DEFAULT_SKILL_LEVEL: u8 = 1;

/// The pitch response used for a `<Class>` block that authors no `<pitch>`.
///
/// **An invention, and labelled one.** Wipeout Pure omits the element from every
/// `<Class>` on both pressings, so a Pure race needs *some* pitch response and
/// the disc supplies none. What the original Pure engine does instead has not
/// been read out of its binary - it may carry a compiled-in table, or a
/// different pitch model, or none at all - so nothing here claims to reproduce
/// it. This is a stand-in that keeps a ship flyable, not a recovered value, and
/// `oag_game::race::load` reports it by name whenever it is reached.
///
/// The numbers are **Pulse's**, and near-uniform there: sweeping all 32
/// `<pitch>` blocks on `pulse-psp-eu.chd` (8 teams x 4 classes) gives
/// `pitch_air="0.3" pitch_ground="1.0" pitch_damping="3"` on **32 of 32**, and
/// `antigrav_height_adjust="1.0"` on 29 with `0.75` on the other three. So the
/// borrowed block is the one Pulse itself uses almost everywhere, which is the
/// most defensible stand-in available - and still a different title's tuning
/// applied to a title whose own is unknown.
///
/// Borrowing Pulse's numbers here rather than in `oag-formats` is the point of
/// the split: the parser reports what the file says, and choosing what to do
/// about a file that says nothing is a decision about *titles*, which belongs
/// above it. See [ADR-0022] and `docs/formats/pure-status.md`.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
pub const PITCH_STAND_IN: Pitch = Pitch {
    pitch_air: 0.3,
    pitch_ground: 1.0,
    pitch_damping: 3.0,
    antigrav_height_adjust: 1.0,
};

/// # The class arrives as a **name**, not as [`SpeedClass`]
///
/// The rung is looked up with [`fmt::Stats::class_named`], which reaches a
/// `<Class>` block whether or not `SpeedClass` has a variant for it. That is
/// what lets a race run on Wipeout Pure's `VECTOR`, a fifth rung below `VENOM`
/// that every Pure race team authors in full and that no four-variant enum can
/// name. Widening the enum instead would have meant a five-wide table on every
/// title, including the ones that author four - a fabricated fifth entry, which
/// is the invented stand-in this project forbids.
///
/// So the ladder's *length* stays the title's own measured business
/// (`oag_title::SpeedClasses`), and this function resolves whatever name that
/// ladder yields against the file that authored it.
///
/// # `None` rather than a panic
///
/// A file whose ladder does not reach the requested rung returns `None`. It used
/// to `expect`, which was safe only while the menu could not offer a fifth rung;
/// now that it can, a mismatched title and class has to degrade visibly rather
/// than abort. The caller reports the absence - it must not substitute a
/// neighbouring rung's tuning.
#[must_use]
pub fn handling_for(
    stats: &fmt::Stats,
    class: &str,
    speedup_pads: fmt::SpeedupPads,
    special: fmt::Special,
) -> Option<Handling> {
    let block = stats.class_named(class)?;
    Some(Handling {
        speedup_pads: SpeedupPads {
            amount: speedup_pads.amount,
            time: speedup_pads.time,
        },
        speedpad_jump: special.speedpad_jump,
        roll_cost: special.roll_cost,
        roll_speed: special.roll_speed,
        roll_turbotime: special.roll_turbotime,
        engine: Engine {
            accelcap: block.engine.accelcap,
            amount: block.engine.amount * ENGINE_AMOUNT_SCALE,
            falloff: block.engine.falloff,
            gain: block.engine.gain,
            turbo: block.engine.turbo,
        },
        brakes: Brakes {
            amount: block.brakes.amount * BRAKES_AMOUNT_SCALE,
            falloff: block.brakes.falloff,
            gain: block.brakes.gain,
        },
        turning: Turning {
            amount: block.turning.amount,
            falloff: block.turning.falloff,
            gain: block.turning.gain,
        },
        airbrake: Airbrake {
            amount: block.airbrake.amount * AIRBRAKE_AMOUNT_SCALE,
            drag: block.airbrake.drag,
            falloff: block.airbrake.falloff,
            gain: block.airbrake.gain,
            turn: block.airbrake.turn,
            slidegrip: block.airbrake.slidegrip * SLIDEGRIP_SCALE,
            // Absent before Pulse added it; `oag_physics::Airbrake`'s own default
            // for "no sideshift" is 0.0, so an earlier schema lands on it.
            sideshift: block.airbrake.sideshift.unwrap_or(0.0),
        },
        antigrav: Antigrav {
            grip_air: block.antigrav.grip_air,
            grip_ground: block.antigrav.grip_ground,
            landing_rebound: block.antigrav.landing_rebound,
            rebound: block.antigrav.rebound,
            rebound_jump_time: block.antigrav.rebound_jump_time,
            ride_height: block.antigrav.ride_height,
        },
        physical: Physical {
            flight_gravity: block.physical.flight_gravity,
            mass: block.physical.mass,
            normal_gravity: block.physical.normal_gravity,
            track_gravity: block.physical.track_gravity,
        },
        // **A stand-in when the file authors no block, not a zero.** See
        // [`PITCH_STAND_IN`]: zeroing would give a ship no pitch authority and no
        // damping at all, which is not what an unauthored element means.
        pitch: block.pitch.map_or(PITCH_STAND_IN, |pitch| Pitch {
            pitch_air: pitch.pitch_air,
            pitch_ground: pitch.pitch_ground,
            pitch_damping: pitch.pitch_damping,
            antigrav_height_adjust: pitch.antigrav_height_adjust,
        }),
        dimensions: Dimensions {
            height: stats.misc.height,
            length: stats.misc.length,
            width: stats.misc.width,
            shield: stats.misc.shield_for(DEFAULT_SKILL_LEVEL),
            // Absent before Pulse added it; `oag_physics::Dimensions` already
            // defaults it to 0.0, so an earlier schema lands on that default
            // rather than on a value invented here.
            weight_distribution: stats.misc.weight_distribution.unwrap_or(0.0),
        },
    })
}

/// The document's speed class for a physics one.
///
/// Two enums for the same four classes, for the same layering reason as the two
/// parameter sets. Exhaustive on both sides so adding a class to either is a
/// compile error rather than a silent fallthrough.
#[must_use]
pub fn to_format_class(class: SpeedClass) -> FmtClass {
    match class {
        SpeedClass::Venom => FmtClass::Venom,
        SpeedClass::Flash => FmtClass::Flash,
        SpeedClass::Rapier => FmtClass::Rapier,
        SpeedClass::Phantom => FmtClass::Phantom,
    }
}

/// The physics speed class for a document one.
#[must_use]
pub fn from_format_class(class: FmtClass) -> SpeedClass {
    match class {
        FmtClass::Venom => SpeedClass::Venom,
        FmtClass::Flash => SpeedClass::Flash,
        FmtClass::Rapier => SpeedClass::Rapier,
        FmtClass::Phantom => SpeedClass::Phantom,
    }
}

#[cfg(test)]
mod tests;
