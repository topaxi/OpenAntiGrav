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
/// `special` carries one field, `speedpad_jump`, and unlike `speedup_pads` it is
/// **not per speed class**: `g_speedpad_jump` (`0x08b36bec`) is a single float,
/// not a four-entry table.
///
/// The four pre-scaled fields are converted here; every other field is moved
/// unchanged, both of these included. See the module docs.
#[must_use]
pub fn handling_for(
    stats: &fmt::Stats,
    class: SpeedClass,
    speedup_pads: fmt::SpeedupPads,
    special: fmt::Special,
) -> Handling {
    // Pulse ships all four rungs on every team, which `handling_ground_truth`
    // checks against the disc. A file whose ladder does not reach this class is
    // another generation of the schema, and this conversion is Pulse's.
    let block = stats
        .class(to_format_class(class))
        .expect("a Pulse handlingstats.xml carries all four speed classes");
    Handling {
        speedup_pads: SpeedupPads {
            amount: speedup_pads.amount,
            time: speedup_pads.time,
        },
        speedpad_jump: special.speedpad_jump,
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
        pitch: Pitch {
            pitch_air: block.pitch.pitch_air,
            pitch_ground: block.pitch.pitch_ground,
            pitch_damping: block.pitch.pitch_damping,
            antigrav_height_adjust: block.pitch.antigrav_height_adjust,
        },
        dimensions: Dimensions {
            height: stats.misc.height,
            length: stats.misc.length,
            width: stats.misc.width,
            shield: stats.misc.shield,
            // Both absent before Pulse added them; `oag_physics::Misc` already
            // defaults each to 0.0, so an earlier schema lands on that default
            // rather than on a value invented here.
            easyshield: stats.misc.easyshield.unwrap_or(0.0),
            weight_distribution: stats.misc.weight_distribution.unwrap_or(0.0),
        },
    }
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
mod tests {
    use super::*;

    /// Invented values, one per field, all distinct and none of them the game's.
    /// Distinctness is the point: it is what makes a transposed pair of fields in
    /// [`handling_for`] visible instead of harmless.
    fn stats() -> fmt::Stats {
        fmt::parse(&fixture()).expect("the fixture must parse")
    }

    /// Two more distinct invented numbers, well clear of the fixture's 1..=174
    /// so a value that arrived from the wrong side is obvious.
    fn pads() -> fmt::SpeedupPads {
        fmt::SpeedupPads {
            amount: 1000.0,
            time: 2000.0,
        }
    }

    /// A third, from the same out-of-range block as [`pads`] and for the same
    /// reason.
    fn special() -> fmt::Special {
        fmt::Special {
            speedpad_jump: 3000.0,
        }
    }

    /// Builds a document whose every numeric attribute is a distinct integer,
    /// counting up from 1 in schema order.
    fn fixture() -> String {
        let mut next = 0.0f32;
        let mut value = move || {
            next += 1.0;
            next
        };
        let attrs = |names: &[&str], value: &mut dyn FnMut() -> f32| {
            names
                .iter()
                .map(|name| format!("{name}=\"{}\"", value()))
                .collect::<Vec<_>>()
                .join(" ")
        };

        let mut out = String::from("<Handling><Stats team=\"Invented\">");
        for element in ["InternalCamera", "BackwardCamera"] {
            let a = attrs(
                &["fov", "headtilt", "height", "length", "pitch"],
                &mut value,
            );
            out.push_str(&format!("<{element} {a}/>"));
        }
        let a = attrs(&["fov", "height", "length", "pitch"], &mut value);
        out.push_str(&format!("<BonnetCamera {a}/>"));
        for element in ["ExternalCameraFar", "ExternalCameraClose"] {
            let a = attrs(
                &[
                    "fov",
                    "lookat_height",
                    "lookat_length",
                    "pos_height",
                    "pos_length",
                    "spring_horiz",
                    "spring_vert",
                ],
                &mut value,
            );
            out.push_str(&format!("<{element} {a}/>"));
        }
        let a = attrs(&["amount", "down_speed", "up_speed"], &mut value);
        out.push_str(&format!("<AirbrakeGraphics {a}/>"));
        let a = attrs(
            &[
                "height",
                "length",
                "shield",
                "easyshield",
                "width",
                "weight_distribution",
            ],
            &mut value,
        );
        out.push_str(&format!("<Misc {a}/>"));
        let a = attrs(&["speed", "thrust", "handling", "shield"], &mut value);
        out.push_str(&format!("<FE {a}/>"));

        for class in ["VENOM", "FLASH", "RAPIER", "PHANTOM"] {
            out.push_str(&format!("<Class name=\"{class}\">"));
            for (element, names) in [
                (
                    "Engine",
                    &["accelcap", "amount", "falloff", "gain", "turbo"][..],
                ),
                ("Brakes", &["amount", "falloff", "gain"][..]),
                ("Turning", &["amount", "falloff", "gain"][..]),
                (
                    "Airbrake",
                    &[
                        "amount",
                        "drag",
                        "falloff",
                        "gain",
                        "turn",
                        "slidegrip",
                        "sideshift",
                    ][..],
                ),
                (
                    "Antigrav",
                    &[
                        "grip_air",
                        "grip_ground",
                        "landing_rebound",
                        "rebound",
                        "rebound_jump_time",
                        "ride_height",
                    ][..],
                ),
                (
                    "Physical",
                    &["flight_gravity", "mass", "normal_gravity", "track_gravity"][..],
                ),
                (
                    "pitch",
                    &[
                        "pitch_air",
                        "pitch_ground",
                        "pitch_damping",
                        "antigrav_height_adjust",
                    ][..],
                ),
            ] {
                let a = attrs(names, &mut value);
                out.push_str(&format!("<{element} {a}/>"));
            }
            out.push_str("</Class>");
        }
        out.push_str("</Stats></Handling>");
        out
    }

    /// Every field the force law reads must arrive with the value the document
    /// carried. Because the fixture's values are all distinct, this catches a
    /// transposition, which is the only interesting failure mode a mapping this
    /// mechanical has - and it is the failure the WAD size fields already
    /// demonstrated is easy to make.
    #[test]
    fn every_parameter_arrives_with_the_documents_value() {
        let stats = stats();
        for class in SpeedClass::ALL {
            // Pulse ships all four rungs on every team, which `handling_ground_truth`
            // checks against the disc. A file whose ladder does not reach this class is
            // another generation of the schema, and this conversion is Pulse's.
            let block = stats
                .class(to_format_class(class))
                .expect("a Pulse handlingstats.xml carries all four speed classes");
            let mapped = handling_for(&stats, class, pads(), special());

            assert_eq!(mapped.engine.accelcap, block.engine.accelcap);
            assert_eq!(
                mapped.engine.amount,
                block.engine.amount * ENGINE_AMOUNT_SCALE
            );
            assert_eq!(mapped.engine.falloff, block.engine.falloff);
            assert_eq!(mapped.engine.gain, block.engine.gain);
            assert_eq!(mapped.engine.turbo, block.engine.turbo);

            assert_eq!(
                mapped.brakes.amount,
                block.brakes.amount * BRAKES_AMOUNT_SCALE
            );
            assert_eq!(mapped.brakes.falloff, block.brakes.falloff);
            assert_eq!(mapped.brakes.gain, block.brakes.gain);

            assert_eq!(mapped.turning.amount, block.turning.amount);
            assert_eq!(mapped.turning.falloff, block.turning.falloff);
            assert_eq!(mapped.turning.gain, block.turning.gain);

            assert_eq!(
                mapped.airbrake.amount,
                block.airbrake.amount * AIRBRAKE_AMOUNT_SCALE
            );
            assert_eq!(mapped.airbrake.drag, block.airbrake.drag);
            assert_eq!(mapped.airbrake.falloff, block.airbrake.falloff);
            assert_eq!(mapped.airbrake.gain, block.airbrake.gain);
            assert_eq!(mapped.airbrake.turn, block.airbrake.turn);
            assert_eq!(
                mapped.airbrake.slidegrip,
                block.airbrake.slidegrip * SLIDEGRIP_SCALE
            );
            assert_eq!(Some(mapped.airbrake.sideshift), block.airbrake.sideshift);

            assert_eq!(mapped.antigrav.grip_air, block.antigrav.grip_air);
            assert_eq!(mapped.antigrav.grip_ground, block.antigrav.grip_ground);
            assert_eq!(
                mapped.antigrav.landing_rebound,
                block.antigrav.landing_rebound
            );
            assert_eq!(mapped.antigrav.rebound, block.antigrav.rebound);
            assert_eq!(
                mapped.antigrav.rebound_jump_time,
                block.antigrav.rebound_jump_time
            );
            assert_eq!(mapped.antigrav.ride_height, block.antigrav.ride_height);

            assert_eq!(
                mapped.physical.flight_gravity,
                block.physical.flight_gravity
            );
            assert_eq!(mapped.physical.mass, block.physical.mass);
            assert_eq!(
                mapped.physical.normal_gravity,
                block.physical.normal_gravity
            );
            assert_eq!(mapped.physical.track_gravity, block.physical.track_gravity);

            assert_eq!(mapped.pitch.pitch_air, block.pitch.pitch_air);
            assert_eq!(mapped.pitch.pitch_ground, block.pitch.pitch_ground);
            assert_eq!(mapped.pitch.pitch_damping, block.pitch.pitch_damping);
            assert_eq!(
                mapped.pitch.antigrav_height_adjust,
                block.pitch.antigrav_height_adjust
            );
        }
    }

    /// `<Misc>` is per team, so all four classes must see the same hull.
    #[test]
    fn the_hull_is_shared_by_every_speed_class() {
        let stats = stats();
        let venom = handling_for(&stats, SpeedClass::Venom, pads(), special()).dimensions;
        for class in SpeedClass::ALL {
            assert_eq!(
                handling_for(&stats, class, pads(), special()).dimensions,
                venom
            );
        }
        assert_eq!(venom.height, stats.misc.height);
        assert_eq!(venom.length, stats.misc.length);
        assert_eq!(venom.width, stats.misc.width);
    }

    /// The four classes must be distinguishable, or `handling_for` is ignoring
    /// its argument and every ship is a Venom.
    #[test]
    fn each_speed_class_maps_to_a_different_parameter_set() {
        let stats = stats();
        let mapped: Vec<_> = SpeedClass::ALL
            .into_iter()
            .map(|class| handling_for(&stats, class, pads(), special()))
            .collect();
        for (i, a) in mapped.iter().enumerate() {
            for b in &mapped[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }

    /// The brake parameter must arrive **negative**, because the force law
    /// applies it along `+normalize(velocity)` and takes its sign from here.
    /// A positive value would accelerate the ship under braking, which is the
    /// single worst outcome available in this file.
    #[test]
    fn the_brake_parameter_arrives_negative() {
        let stats = stats();
        let block = stats.class(FmtClass::Venom).expect("four rungs");
        assert!(
            block.brakes.amount > 0.0,
            "the fixture's XML value is positive, or this test proves nothing"
        );
        assert!(
            handling_for(&stats, SpeedClass::Venom, pads(), special())
                .brakes
                .amount
                < 0.0
        );
    }

    /// `slidegrip` on `0..100` must land on `0..0.01`, because the grip term's
    /// `(0.01 - slidegrip)` is what has to reach exactly zero at 100. This is the
    /// arithmetic that turned the "percent of grip retained" reading from an
    /// interpretation into a determination, so it is pinned here rather than only
    /// described.
    #[test]
    fn full_slidegrip_cancels_the_grip_coefficient_exactly() {
        let scaled = 100.0 * SLIDEGRIP_SCALE;
        assert_eq!(0.01 - scaled, 0.0);
        // And the other end of the range leaves the coefficient untouched.
        assert_eq!(0.01 - 0.0 * SLIDEGRIP_SCALE, 0.01);
    }

    /// Guards against the double-application bug the module docs warn about: if
    /// the force law ever also scales, this set is where somebody will look, so
    /// it must not drift.
    #[test]
    fn exactly_five_fields_are_pre_scaled() {
        assert_eq!(SCALED_FIELDS.len(), 5);
        assert_eq!(
            SCALED_FIELDS,
            [
                "Engine.amount",
                "Brakes.amount",
                "Airbrake.amount",
                "Airbrake.slidegrip",
                "AirbrakeGraphics.amount"
            ]
        );
    }

    /// The flap deflection is an angle, and the two rates beside it are not.
    ///
    /// The `25 -> 0.4363323` pair is the corroboration `camera.md` records off
    /// the live block, so it is the one worth asserting rather than an arbitrary
    /// input: if the scale is ever dropped or applied twice, this is the number
    /// that moves.
    #[test]
    fn the_airbrake_flap_deflection_is_converted_to_radians() {
        let mut stats = stats();
        stats.airbrake_graphics.amount = 25.0;
        let graphics = airbrake_graphics_for(&stats);

        assert!(
            (graphics.amount - 0.436_332_3).abs() < 1e-6,
            "25 degrees should reach the game as {} rad, got {}",
            0.436_332_3,
            graphics.amount
        );
        assert_eq!(graphics.up_speed, stats.airbrake_graphics.up_speed);
        assert_eq!(graphics.down_speed, stats.airbrake_graphics.down_speed);
    }

    /// Every field that is *not* in [`SCALED_FIELDS`] must arrive verbatim. The
    /// three unscaled siblings of the scaled fields are the ones worth naming,
    /// since a scale applied to the wrong member of a block would be invisible
    /// otherwise.
    #[test]
    fn the_siblings_of_the_scaled_fields_are_untouched() {
        let stats = stats();
        let block = stats.class(FmtClass::Venom).expect("four rungs");
        let mapped = handling_for(&stats, SpeedClass::Venom, pads(), special());
        assert_eq!(mapped.engine.accelcap, block.engine.accelcap);
        assert_eq!(mapped.engine.turbo, block.engine.turbo);
        assert_eq!(mapped.airbrake.drag, block.airbrake.drag);
        assert_eq!(mapped.airbrake.turn, block.airbrake.turn);
        assert_eq!(Some(mapped.airbrake.sideshift), block.airbrake.sideshift);
        assert_eq!(mapped.physical.mass, block.physical.mass);
    }

    /// The one parameter that does not come from `stats`. It must arrive
    /// unscaled and untransposed, and it must be the same for every class the
    /// caller asks for - the class is chosen when the caller looks the block up,
    /// not here.
    #[test]
    fn the_speed_pad_tunables_arrive_from_the_argument_unscaled() {
        let stats = stats();
        for class in SpeedClass::ALL {
            let mapped = handling_for(&stats, class, pads(), special());
            assert_eq!(mapped.speedup_pads.amount, pads().amount);
            assert_eq!(mapped.speedup_pads.time, pads().time);
        }
        assert!(
            !SCALED_FIELDS.iter().any(|f| f.starts_with("SpeedupPads")),
            "if this ever becomes a scaled field the mapping above has to change"
        );
    }

    #[test]
    fn the_two_speed_class_enums_round_trip() {
        for class in SpeedClass::ALL {
            assert_eq!(from_format_class(to_format_class(class)), class);
        }
    }
}
