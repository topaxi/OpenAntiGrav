//! What the handling-schema mapping in [`super`] is asserted to do.
//!
//! Split out of `handling.rs` under the 200-line cap on inline `#[cfg(test)]`
//! modules; see `scripts/check-file-size.py`.

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
        let mapped = handling_for(&stats, class.as_str(), pads(), special())
            .expect("the fixture authors this class");

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

        // `expect`, not a fallback: this fixture authors a `<pitch>` in
        // every class, so reaching `PITCH_STAND_IN` here would mean the
        // parser dropped a block it was given - which is the failure the
        // `Option` could otherwise hide.
        let pitch = block.pitch.expect("this fixture authors <pitch>");
        assert_eq!(mapped.pitch.pitch_air, pitch.pitch_air);
        assert_eq!(mapped.pitch.pitch_ground, pitch.pitch_ground);
        assert_eq!(mapped.pitch.pitch_damping, pitch.pitch_damping);
        assert_eq!(
            mapped.pitch.antigrav_height_adjust,
            pitch.antigrav_height_adjust
        );
    }
}

/// `<Misc>` is per team, so all four classes must see the same hull.
#[test]
fn the_hull_is_shared_by_every_speed_class() {
    let stats = stats();
    let venom = handling_for(&stats, "VENOM", pads(), special())
        .expect("the fixture authors VENOM")
        .dimensions;
    for class in SpeedClass::ALL {
        assert_eq!(
            handling_for(&stats, class.as_str(), pads(), special())
                .expect("the fixture authors this class")
                .dimensions,
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
        .map(|class| {
            handling_for(&stats, class.as_str(), pads(), special())
                .expect("the fixture authors this class")
        })
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
        handling_for(&stats, "VENOM", pads(), special())
            .expect("the fixture authors VENOM")
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
    let mapped =
        handling_for(&stats, "VENOM", pads(), special()).expect("the fixture authors VENOM");
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
        let mapped = handling_for(&stats, class.as_str(), pads(), special())
            .expect("the fixture authors this class");
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
