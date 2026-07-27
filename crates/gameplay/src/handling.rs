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
//! The original's own XML parsers do not store four of these values verbatim:
//! `Engine.amount` is stored as `xml * 0.001`, `Brakes.amount` as
//! `xml * -0.01`, and both `Airbrake.amount` and `Airbrake.slidegrip` as
//! `xml * 0.0001`. Recovered from `HandlingXml_ParseEngine`,
//! `HandlingXml_ParseBrakes` and `HandlingXml_ParseAirbrake` at confidence 88;
//! see `docs/ghidra/functions/psp-pulse/engine.md`.
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
    Airbrake, Antigrav, Brakes, Dimensions, Engine, Handling, Physical, Pitch, SpeedClass, Turning,
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

/// The four fields that are not stored verbatim, for grepping.
///
/// Exists so that "which parameters are pre-scaled?" has one answer in the
/// repository rather than four scattered constants, and so a test can assert the
/// set has not silently grown.
pub const SCALED_FIELDS: [&str; 4] = [
    "Engine.amount",
    "Brakes.amount",
    "Airbrake.amount",
    "Airbrake.slidegrip",
];

/// The physics parameter set for one team in one speed class.
///
/// `<Misc>` is per team rather than per class, so [`Dimensions`] comes from the
/// enclosing [`fmt::Stats`] while everything else comes from the class block.
/// That asymmetry is the format's, not ours.
///
/// The four pre-scaled fields are converted here; every other field is moved
/// unchanged. See the module docs.
#[must_use]
pub fn handling_for(stats: &fmt::Stats, class: SpeedClass) -> Handling {
    let block = stats.class(to_format_class(class));
    Handling {
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
            sideshift: block.airbrake.sideshift,
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
            easyshield: stats.misc.easyshield,
            weight_distribution: stats.misc.weight_distribution,
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
            let block = stats.class(to_format_class(class));
            let mapped = handling_for(&stats, class);

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
            assert_eq!(mapped.airbrake.sideshift, block.airbrake.sideshift);

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
        let venom = handling_for(&stats, SpeedClass::Venom).dimensions;
        for class in SpeedClass::ALL {
            assert_eq!(handling_for(&stats, class).dimensions, venom);
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
            .map(|class| handling_for(&stats, class))
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
        let block = stats.class(FmtClass::Venom);
        assert!(
            block.brakes.amount > 0.0,
            "the fixture's XML value is positive, or this test proves nothing"
        );
        assert!(handling_for(&stats, SpeedClass::Venom).brakes.amount < 0.0);
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
    fn exactly_four_fields_are_pre_scaled() {
        assert_eq!(SCALED_FIELDS.len(), 4);
        assert_eq!(
            SCALED_FIELDS,
            [
                "Engine.amount",
                "Brakes.amount",
                "Airbrake.amount",
                "Airbrake.slidegrip"
            ]
        );
    }

    /// Every field that is *not* in [`SCALED_FIELDS`] must arrive verbatim. The
    /// three unscaled siblings of the scaled fields are the ones worth naming,
    /// since a scale applied to the wrong member of a block would be invisible
    /// otherwise.
    #[test]
    fn the_siblings_of_the_scaled_fields_are_untouched() {
        let stats = stats();
        let block = stats.class(FmtClass::Venom);
        let mapped = handling_for(&stats, SpeedClass::Venom);
        assert_eq!(mapped.engine.accelcap, block.engine.accelcap);
        assert_eq!(mapped.engine.turbo, block.engine.turbo);
        assert_eq!(mapped.airbrake.drag, block.airbrake.drag);
        assert_eq!(mapped.airbrake.turn, block.airbrake.turn);
        assert_eq!(mapped.airbrake.sideshift, block.airbrake.sideshift);
        assert_eq!(mapped.physical.mass, block.physical.mass);
    }

    #[test]
    fn the_two_speed_class_enums_round_trip() {
        for class in SpeedClass::ALL {
            assert_eq!(from_format_class(to_format_class(class)), class);
        }
    }
}
