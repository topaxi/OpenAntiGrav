//! What `load` reads off the disc, and what its report says about it.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`.

use super::*;
use crate::hud::hud_roots;
use oag_livery::entry::{boost_entry_name, shield_entry_names, ship_entry_name};

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

/// The PSP's shield shell follows the player's hull stem, because
/// `ShipShield_Construct` formats the `FE_TeamModel` registry value into
/// `%s\%sshield.vex`; the PS2 build passes the literal `extra` whatever was
/// picked.
#[test]
fn the_psp_shield_shell_follows_the_players_hull_stem_and_the_ps2_one_does_not() {
    use oag_assets::Platform::{Ps2, Psp};
    let first = |platform, model| {
        shield_entry_names(oag_title::race::SHIP_DIR, "Assegai", platform, model)[0].clone()
    };
    assert_eq!(first(Psp, None), r"Data\Ships\Assegai\shipshield.vex");
    assert_eq!(
        first(Psp, Some("Ship")),
        r"Data\Ships\Assegai\shipshield.vex"
    );
    assert_eq!(
        first(Psp, Some("extra")),
        r"Data\Ships\Assegai\extrashield.vex"
    );
    assert_eq!(first(Ps2, None), r"Data\Ships\Assegai\extrashield.vex");
    assert_eq!(
        first(Ps2, Some("Ship")),
        r"Data\Ships\Assegai\extrashield.vex"
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
            let entry = hud_layout(title.hud, mode);
            assert!(!entry.is_empty(), "{} authors no {mode:?} HUD", title.name);
        }
    }

    // Both PSP discs ship no `SpeedLap_HUD.xml` - the name hashes to
    // `1af0a646` and neither carries it - so speed lap draws the time trial's.
    for psp in [oag_pulse::TITLE, oag_pure::TITLE] {
        assert_eq!(
            hud_layout(psp.hud, Mode::SpeedLap),
            hud_layout(psp.hud, Mode::TimeTrial),
            "{} ships no separate speed lap layout",
            psp.name
        );
    }

    // HD does ship one, which is the divergence that made this an axis. Before
    // it existed, an HD speed lap was served Pulse's time trial name.
    assert_ne!(
        hud_layout(oag_hd::TITLE.hud, Mode::SpeedLap),
        hud_layout(oag_hd::TITLE.hud, Mode::TimeTrial),
        "HD ships speedlap_hud.xml separately"
    );
    assert_eq!(
        hud_layout(oag_hd::TITLE.hud, Mode::SpeedLap),
        "/data/xml/speedlap_hud.xml"
    );

    // And nothing is quietly serving Pulse's tables to HD any more.
    assert_ne!(
        hud_layout(oag_hd::TITLE.hud, Mode::SingleRace),
        hud_layout(oag_pulse::TITLE.hud, Mode::SingleRace),
        "HD's layouts are spelled as its own manifest stores them"
    );
}

/// The `HUD Style` list picks the roots a race reads. HD's executable names no skinned
/// Eliminator literal, so every HD skin keeps the default Eliminator root; Omega's names
/// all five modes. A title with no list (Pulse) ignores a style and says nothing.
#[test]
fn a_hud_style_picks_the_skins_roots_by_mode() {
    let mut report = Vec::new();
    let default = oag_hd::TITLE.hud;
    for (style, dir) in [
        ("2097", "/data/xml/2097_hud/"),
        ("wip3out", "/data/xml/wo3_hud/"),
    ] {
        let (hud, skin) = hud_roots(oag_hd::TITLE, Some(style), &mut report);
        assert!(skin.expect("a skin").font_role.is_some());
        for mode in [
            Mode::SingleRace,
            Mode::TimeTrial,
            Mode::SpeedLap,
            Mode::Zone,
        ] {
            assert!(hud_layout(hud, mode).starts_with(dir), "{style} {mode:?}");
        }
        assert_eq!(
            hud_layout(hud, Mode::Eliminator),
            hud_layout(default, Mode::Eliminator),
            "{style}: HD's executable names no skinned Eliminator"
        );
        assert!(hud.split.expect("split").arcade[0].starts_with(dir));
    }
    for (style, dir) in [
        ("2097", r"Data\XML\2097_HUD\"),
        ("WIP3OUT", r"Data\XML\wo3_HUD\"),
    ] {
        let (hud, _) = hud_roots(oag_omega::TITLE, Some(style), &mut report);
        for mode in Mode::ALL {
            assert!(
                hud_layout(hud, mode).starts_with(dir),
                "Omega {style} {mode:?}"
            );
        }
    }

    // No name, or the list's own default: the title's roots, no font slot.
    for style in [None, Some(""), Some("HD")] {
        let (hud, skin) = hud_roots(oag_hd::TITLE, style, &mut report);
        assert_eq!(
            hud_layout(hud, Mode::SingleRace),
            "/data/xml/arcade_hud.xml"
        );
        assert_eq!(skin.and_then(|skin| skin.font_role), None);
    }

    // An unknown name draws the default and is reported; a title with no list does neither.
    let mut report = Vec::new();
    let (hud, _) = hud_roots(oag_hd::TITLE, Some("nonsense"), &mut report);
    assert_eq!(
        hud_layout(hud, Mode::SingleRace),
        "/data/xml/arcade_hud.xml"
    );
    assert!(
        report
            .iter()
            .any(|line| line.contains("not in this title's list"))
    );
    let mut report = Vec::new();
    let (hud, skin) = hud_roots(oag_pulse::TITLE, Some("2097"), &mut report);
    assert!(skin.is_none() && report.is_empty());
    assert_eq!(
        hud_layout(hud, Mode::SingleRace),
        hud_layout(oag_pulse::TITLE.hud, Mode::SingleRace)
    );
}
