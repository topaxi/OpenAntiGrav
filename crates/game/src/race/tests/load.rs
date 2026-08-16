//! What `load` reads off the disc, and what its report says about it.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`.

use super::*;

/// The hull and its plume must come from the same family, which is the
/// regression this guards: the plume load hardcoded `shipboost.vex` while
/// the hull switched on mode, so a Zone race drew a `Zone.vex` hull with the
/// `Ship.vex` plume. Asserted as the *pairing* rather than as two
/// independent strings, because the pairing is the thing that was wrong.
#[test]
fn the_boost_plume_follows_the_hull_its_mode_selects() {
    for mode in [Mode::TimeTrial, Mode::SpeedLap, Mode::Zone] {
        let hull = ship_entry_name("Feisar", mode);
        let plume = boost_entry_name("Feisar", mode);
        let stem = if mode == Mode::Zone { "Zone" } else { "ship" };
        assert!(
            plume.contains(stem),
            "{mode:?}: hull {hull} but plume {plume} - they are not the same family"
        );
    }
    assert_eq!(
        boost_entry_name("Feisar", Mode::Zone),
        r"Data\Ships\Feisar\Zoneboost.vex"
    );
    assert_eq!(
        boost_entry_name("Feisar", Mode::TimeTrial),
        r"Data\Ships\Feisar\shipboost.vex"
    );
}

/// The PS2 shape, measured: five slots declared and none of them filled.
#[test]
fn slots_that_all_failed_to_decode_are_reported_with_the_count() {
    let note = untextured_note(&model(5, 0)).expect("an untextured model is reported");
    assert!(note.contains("Ship.vex"), "{note}");
    assert!(note.contains("0 of 5 texture slot(s)"), "{note}");
    // The reader has to be able to get from the line to the finding, because
    // the line on its own reads like a decode failure and it is not one.
    assert!(note.contains("ps2-texture.md"), "{note}");
}

/// A model that got everything it asked for is silent, which is every PSP
/// model measured: 8 of 8 on the ship, 135 of 135 on the track.
#[test]
fn a_fully_textured_model_is_not_reported() {
    assert_eq!(untextured_note(&model(8, 8)), None);
}

#[test]
fn a_partly_textured_model_is_reported_too() {
    let note = untextured_note(&model(8, 3)).expect("a partial decode is worth saying");
    assert!(note.contains("3 of 8 texture slot(s)"), "{note}");
}

/// **The regression this rule exists for.** The driveable ribbon is generated
/// geometry with no texture slots at all, on either disc, and the earlier
/// "no textures" test reported it on the *PSP* disc - blaming a PS2 texture-set
/// gap for a mesh that was never textured and never came off a `.vex`.
#[test]
fn a_model_that_declares_no_slots_wanted_none_and_is_not_reported() {
    assert_eq!(untextured_note(&model(0, 0)), None);
}
