//! The RACE page's WEAPONS row, on a real circuit out of a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! The row is worth nothing if a launch ignores it: a single race with weapons
//! off has to lose its pads (nothing to pick up, nothing to trigger) and report
//! weapons off to the damage rules, where the untouched default keeps both.

use oag_game::race;

fn started(weapons_override: Option<bool>) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        weapons_override,
        ..race::Options::default()
    })
    .expect("loading the race");
    assert_eq!(loaded.setup.weapons_on(), weapons_override.unwrap_or(true));
    Some(race::Race::start(loaded.setup))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_single_race_with_weapons_off_has_no_pads_on_pulse() {
    let (Some(on), Some(off)) = (started(None), started(Some(false))) else {
        return;
    };
    assert!(
        !on.weapon_pad_refresh_left().is_empty(),
        "the default single race keeps its weapon pads"
    );
    assert!(
        off.weapon_pad_refresh_left().is_empty(),
        "weapons OFF drops them"
    );
}
