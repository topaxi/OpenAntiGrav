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
    let craft = oag_pulse::race::DEFAULTS.ships();
    for mode in [Mode::TimeTrial, Mode::SpeedLap, Mode::Zone] {
        let hull = ship_entry_name(craft, "Feisar", mode, None);
        let plume = boost_entry_name(craft, "Feisar", mode).expect("Pulse ships one");
        let stem = if mode == Mode::Zone { "Zone" } else { "ship" };
        assert!(
            plume.contains(stem),
            "{mode:?}: hull {hull} but plume {plume} - they are not the same family"
        );
    }
    assert_eq!(
        boost_entry_name(craft, "Feisar", Mode::Zone),
        Some(r"Data\Ships\Feisar\Zoneboost.vex".to_string())
    );
    assert_eq!(
        boost_entry_name(craft, "Feisar", Mode::TimeTrial),
        Some(r"Data\Ships\Feisar\shipboost.vex".to_string())
    );
}

/// Pure ships a dedicated Zone ship directory (`ZoneCraft::OwnShip`) and no
/// standalone boost model at all - so `boost_entry_name` must answer `None`
/// in Zone mode too, not fall back to Pulse's `shipboost` stem under a ship
/// directory that never carried one. Regression for the bug `assets.rs`'s own
/// doc comment on the Zone fallback describes.
#[test]
fn a_title_with_no_ordinary_boost_model_has_none_in_zone_mode_either() {
    let craft = oag_pure::race::DEFAULTS.ships();
    assert_eq!(boost_entry_name(craft, "Feisar", Mode::TimeTrial), None);
    assert_eq!(boost_entry_name(craft, "Feisar", Mode::Zone), None);
}

/// `hull_variant` swaps the file stem inside the team's own directory, and
/// leaves everything else about the path - the directory, the team - alone.
#[test]
fn a_hull_variant_override_replaces_only_the_file_stem() {
    let craft = oag_pulse::race::DEFAULTS.ships();
    assert_eq!(
        ship_entry_name(craft, "Assegai", Mode::TimeTrial, Some("extra")),
        r"Data\Ships\Assegai\extra.vex"
    );
    assert_eq!(
        ship_entry_name(craft, "Assegai", Mode::TimeTrial, None),
        r"Data\Ships\Assegai\Ship.vex",
        "no override is exactly today's baseline"
    );
}

/// A Zone run ignores the override - see [`ship_entry_name`]'s own doc
/// comment for why: Zone's hull is a title fact of its own, unaffected by
/// which model variant the RACE page picked.
#[test]
fn a_hull_variant_override_does_not_reach_zone_mode() {
    let craft = oag_pulse::race::DEFAULTS.ships();
    assert_eq!(
        ship_entry_name(craft, "Assegai", Mode::Zone, Some("extra")),
        ship_entry_name(craft, "Assegai", Mode::Zone, None),
        "Zone's own hull selection does not consult the override at all"
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

/// Finding S3's guard: an unnamed team resolves per title, not to Pulse's.
///
/// The resolution itself needs a disc, so what is checkable without one is the
/// thing that was actually wrong - `Options` carrying a Pulse constant instead
/// of "ask the title" - plus the fact that every title package answers when
/// asked. HD's answer is the one that matters: it is `assegai` where Pulse says
/// `Assegai`, and the old code only worked because a PSARC folds case.
#[test]
fn an_unnamed_team_is_the_titles_own_and_every_title_has_one() {
    assert_eq!(
        Options::default().team,
        None,
        "a default that names a team is a default that names *one title's* team"
    );
    for title in [oag_pulse::TITLE, oag_pure::TITLE, oag_hd::TITLE] {
        assert!(
            !title.race.team.is_empty(),
            "{} authors no default team, so an unnamed --team has nothing to \
             resolve to",
            title.name
        );
    }
    assert_eq!(oag_hd::TITLE.race.team, "assegai");
    assert_eq!(oag_pulse::TITLE.race.team, "Assegai");
}

/// Finding S2's guard: every title answers the HUD question for itself, and
/// speed lap is the row where the answers differ.
///
/// The composed read needs a disc; what is checkable without one is the routing
/// that was wrong - `hud_layout` returning Pulse's names whatever was open.
#[test]
fn each_title_serves_its_own_hud_layouts() {
    for title in [oag_pulse::TITLE, oag_pure::TITLE, oag_hd::TITLE] {
        for mode in Mode::ALL {
            let entry = hud_layout(title, mode);
            assert!(!entry.is_empty(), "{} authors no {mode:?} HUD", title.name);
        }
    }

    // Both PSP discs ship no `SpeedLap_HUD.xml` - the name hashes to
    // `1af0a646` and neither carries it - so speed lap draws the time trial's.
    for psp in [oag_pulse::TITLE, oag_pure::TITLE] {
        assert_eq!(
            hud_layout(psp, Mode::SpeedLap),
            hud_layout(psp, Mode::TimeTrial),
            "{} ships no separate speed lap layout",
            psp.name
        );
    }

    // HD does ship one, which is the divergence that made this an axis. Before
    // it existed, an HD speed lap was served Pulse's time trial name.
    assert_ne!(
        hud_layout(oag_hd::TITLE, Mode::SpeedLap),
        hud_layout(oag_hd::TITLE, Mode::TimeTrial),
        "HD ships speedlap_hud.xml separately"
    );
    assert_eq!(
        hud_layout(oag_hd::TITLE, Mode::SpeedLap),
        "/data/xml/speedlap_hud.xml"
    );

    // And nothing is quietly serving Pulse's tables to HD any more.
    assert_ne!(
        hud_layout(oag_hd::TITLE, Mode::SingleRace),
        hud_layout(oag_pulse::TITLE, Mode::SingleRace),
        "HD's layouts are spelled as its own manifest stores them"
    );
}
