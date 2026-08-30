//! A Zone race reads its title's own stage table and grades the circuit with it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
//!     --run-ignored all -E 'binary(zone_grade_ground_truth)'
//! ```
//!
//! # What this file is the evidence for
//!
//! `zone_ground_truth.rs` proves a Zone race opens the right *circuit*. This
//! one proves the escalation laid over it is the disc's: HD/Fury's
//! `/data/environments/zonemode.effectsettings` reaches
//! `oag_game::race::Loaded` through a real race load, its fifteen stages are
//! the file's own, and two adjacent stages produce two visibly different fogs
//! whose numbers are the ones the file authors - not a pair this test made up.
//!
//! # What is deliberately not asserted
//!
//! **Nothing about when a stage changes.** No trigger is recovered on either
//! title that ships one of these tables, so a race rests on stage `0` and this
//! file drives the stage by hand to check the plumbing. See
//! `oag_game::race::zone_grade`'s module docs and
//! `docs/formats/effectsettings.md`'s `## Open`; the one assertion here about
//! it is that a freshly loaded race really is on stage `0` and stays there.

use std::path::{Path, PathBuf};

use oag_game::race;
use oag_physics::SpeedClass;

/// One image, or `None` with a printed reason when it is not present.
fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name);
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn load(mode: oag_race::Mode) -> Option<race::Loaded> {
    let path = image("data/images/hdfury-ps3-eu-dec.iso")?;
    Some(
        race::load(&race::Options {
            source: path.display().to_string(),
            class: SpeedClass::Venom,
            mode,
            ..race::Options::default()
        })
        .expect("loading the race"),
    )
}

/// **The table reaches a real race load**, with the ladder the file names, and
/// the load report says so.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn an_hd_zone_race_loads_the_titles_own_stage_table() {
    let Some(loaded) = load(oag_race::Mode::Zone) else {
        return;
    };
    for line in &loaded.report {
        println!("{line}");
    }
    let grade = loaded
        .zone_grade
        .as_ref()
        .expect("HD ships /data/environments/zonemode.effectsettings");
    // Fifteen stages, `0` through `14` - `Start` to `Supersonic`.
    assert_eq!(grade.last_stage(), 14);
    // And the race rests on the first of them, because nothing selects a
    // stage: that is the open question, not an oversight. HD's own loader
    // leaves `+0x00` at zero the same way.
    assert_eq!(grade.blend().current, 0);
    assert_eq!(grade.blend().requested, 0);
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("the Zone colour grade")),
        "the load report should say what it read"
    );
}

/// **Two adjacent stages grade the circuit differently, off the file's own
/// numbers.** `Start` authors no fog of its own and leaves the circuit's
/// standing; `Sub Venom` authors the cyan the file states, at the density the
/// file states.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn two_stages_of_the_real_table_produce_two_different_fogs() {
    let Some(loaded) = load(oag_race::Mode::Zone) else {
        return;
    };
    let mut grade = loaded.zone_grade.expect("the table loads");
    // The circuit's own fog, whatever this circuit authors - the thing a stage
    // lays over. A `zone_N` circuit authors one; if it ever stops, this test
    // says so rather than silently comparing two `None`s.
    let base = loaded.authored_fog.expect("a zone_N circuit fogs");

    let start = grade.fog(Some(base)).expect("stage 0 leaves the circuit's");
    assert_eq!(
        start.colour, base.colour,
        "Start authors Fog density 0, so the circuit's own fog stands"
    );

    grade.request_stage(1);
    assert!(grade.commit());
    grade.set_weight(1.0);
    let sub_venom = grade.fog(Some(base)).expect("stage 1 authors its own");
    // Straight out of `/data/environments/zonemode.effectsettings`:
    // `"1 Sub Venom.Lighting.Fog colour"=0.000000 1.305882 1.800000 0.000000`
    // and `"1 Sub Venom.Lighting.Fog density"=0.002100`.
    assert_eq!(sub_venom.colour, [0.0, 1.305_882, 1.8]);
    assert_eq!(sub_venom.density, 0.002_1);
    assert_ne!(
        sub_venom.colour, start.colour,
        "a stage change has to be visible, or the table is doing nothing"
    );

    // Halfway between them is halfway between the two authored colours - the
    // recovered cross-fade, on real data.
    grade.set_weight(0.5);
    let half = grade.fog(Some(base)).expect("still over zero density");
    assert_eq!(half.colour, [0.0, 1.305_882 / 2.0, 0.9]);
}

/// **A stage recolours the circuit's rig without aiming it.** The schema has no
/// `Sun direction` key, so a graded rig keeps the circuit's own direction and
/// takes the stage's own ambient.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_stage_tints_the_circuits_own_light_rig() {
    let Some(loaded) = load(oag_race::Mode::Zone) else {
        return;
    };
    let mut grade = loaded.zone_grade.expect("the table loads");
    let base = loaded.light;
    grade.request_stage(1);
    grade.commit();
    grade.set_weight(1.0);
    let lit = grade.light(base);
    assert_eq!(lit.direction, base.direction, "the circuit aims the sun");
    // `"1 Sub Venom.Lighting.Constant Ambient Colour"=1.500000 x3`.
    assert_eq!(lit.ambient, [1.5, 1.5, 1.5]);
}

/// **Only Zone reads it.** A single race on the same title loads no grade at
/// all, which is what keeps a table meant for one mode out of the other three.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_race_outside_zone_loads_no_grade() {
    let Some(loaded) = load(oag_race::Mode::SingleRace) else {
        return;
    };
    assert!(loaded.zone_grade.is_none());
}
