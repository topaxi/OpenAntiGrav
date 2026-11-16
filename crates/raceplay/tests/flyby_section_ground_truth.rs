//! The section the pre-race flyby is masked by, against the original's own.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay flyby_section_ground_truth:: --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! The original's render-view publisher locates the flyby's camera on the AI track every frame
//! and masks the draws with that one section's row. The camera flies far from the craft, so this
//! is what separates a flyby that draws the original's copies of a building from one that draws
//! every copy the circuit authors. The sections below are `0x08ab10a0`, read per frame off
//! PPSSPP for a whole flyby (`docs/gameplay/race-intro.md`, "The flyby is masked by the section
//! under its camera"). A flyby that loses the rule reads `None`; a locate that is not the
//! original's reads another sequence.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// The sections `track` masks its flyby by, in order, each once per stretch.
fn sections_of_the_flyby(track: &str) -> Option<Vec<u8>> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: Some(track.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(race.flyby_camera_section(), None, "before the flyby");
    assert!(race.begin_intro(), "{track} plays a flyby");
    let mut sections = Vec::new();
    loop {
        race.tick_intro(&PlayerInputs::none());
        if !race.in_intro() {
            break;
        }
        let section = race
            .flyby_camera_section()
            .expect("a camera the spline can place");
        if sections.last() != Some(&section) {
            sections.push(section);
        }
    }
    assert_eq!(race.flyby_camera_section(), None, "after the flyby");
    Some(sections)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn platinum_rush_white_masks_its_flyby_by_the_originals_sections() {
    let Some(sections) = sections_of_the_flyby(r"Data\Environments\13_Track\track.vex") else {
        return;
    };
    assert_eq!(sections, [8, 1, 17, 14, 23]);
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn platinum_rush_black_masks_its_flyby_by_the_originals_sections() {
    let Some(sections) = sections_of_the_flyby(r"Data\Environments\13_Track\track_reversed.vex")
    else {
        return;
    };
    assert_eq!(sections, [8, 1, 17, 14, 23]);
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn talons_junction_masks_its_flyby_by_the_originals_sections() {
    let Some(sections) = sections_of_the_flyby(r"Data\Environments\16_Track\track.vex") else {
        return;
    };
    assert_eq!(sections, [0, 27, 25, 18, 19, 20, 8]);
}
