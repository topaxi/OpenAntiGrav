//! Wipeout: Omega Collection's weapon bodies and magstrip arc wake, loaded
//! through a race off the extracted package pair.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^omega_weapon_bodies_ground_truth::/)'
//! ```
//!
//! What it pins that `oag-omega`'s own archive test cannot: that the loader's
//! Vita/PS4 branch builds each body at all. The same names behind HD's PS3
//! path fail on Omega with `version 0xedad5cca, expected 0x000a0000` (the
//! shield shell still does), so a title table that names the right files is
//! not a body that draws.

use oag_gameplay::PlayerInputs;

fn source() -> Option<String> {
    Some(
        oag_testdata::exact("data/extracted/ps4")?
            .display()
            .to_string(),
    )
}

fn loaded() -> Option<oag_raceplay::Loaded> {
    let source = source()?;
    Some(
        oag_raceplay::load(&oag_raceplay::Options {
            source,
            class: "venom".to_string(),
            mode: oag_race::Mode::SingleRace,
            track: Some(r"Data\environments\talons_junction\track.vex".to_string()),
            seed: Some(1),
            ..oag_raceplay::Options::default()
        })
        .unwrap_or_else(|e| panic!("loading Talon's Junction: {e:#}")),
    )
}

/// Every body the table names builds, and none is reported absent.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn every_body_omega_names_builds_from_its_own_archives() {
    let Some(loaded) = loaded() else {
        return;
    };
    let wired = [
        ("rocket", loaded.rocket_model.is_some()),
        ("mine", loaded.mine_model.is_some()),
        ("bomb", loaded.bomb_model.is_some()),
        ("cannon muzzle flash", loaded.cannon_model.is_some()),
        ("plasma ring", loaded.plasma_blast_models.halo.is_some()),
        (
            "plasma sphere",
            loaded.plasma_blast_models.hemisphere2.is_some(),
        ),
        (
            "plasma halo",
            loaded.plasma_blast_models.hemisphere1.is_some(),
        ),
        ("missile blast", loaded.bomb_blast_models.hd[5].is_some()),
    ];
    let missing: Vec<_> = wired.iter().filter(|(_, ok)| !ok).map(|(n, _)| n).collect();
    assert!(missing.is_empty(), "built nothing for: {missing:?}");

    let absent: Vec<_> = loaded
        .report
        .iter()
        .filter(|line| {
            line.contains(r"Data\Weapons\") && oag_raceplay::loader_log::is_absence(line)
        })
        .collect();
    assert!(
        absent.is_empty(),
        "a weapon body is reported absent: {absent:?}"
    );
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.starts_with(r"Data\Weapons\hd_Rocket.vex")
                && line.contains("Vita/PS4 container")),
        "the rocket did not take the Vita/PS4 branch"
    );
}

/// The four bodies held back stay held back until their programs are read: a
/// table that names one again must also change this test, which is the point.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_bodies_held_back_for_want_of_their_programs_are_not_drawn() {
    let Some(loaded) = loaded() else {
        return;
    };
    assert!(
        loaded.plasma_blast_models.ball.is_none(),
        "the plasma bolt's head"
    );
    assert!(loaded.leach_ball_model.is_none(), "the leachbeam ball");
    assert!(
        loaded.bomb_blast_models.hd[..5].iter().all(Option::is_none),
        "the bomb's blast set and its halo"
    );
    assert!(
        loaded.ribbon_textures.leach_strip.is_none(),
        "the leach strip"
    );
}

/// An HD-lineage mode builds the arc wake, and a craft that meets a magstrip
/// draws it: the atlas and contact textures decode from `.gnf`, and the wake's
/// draw list is non-empty while a craft is over the strip.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn a_craft_over_a_magstrip_draws_the_arc_wake() {
    let Some(loaded) = loaded() else {
        return;
    };
    assert!(
        loaded.ribbon_textures.magstrip.is_some(),
        "HD_electric_arc_8x8.gnf and HD_ElectricArc_Contact.gnf must both decode"
    );
    let mut race = oag_raceplay::Race::start(loaded.setup);
    race.set_autopilot(true);
    assert!(race.has_magstrip_wake(), "Omega builds the wake");
    let slots = usize::from(race.ship_count());
    let mut drawn = None;
    for tick in 0..2400_u64 {
        race.tick(&PlayerInputs::none());
        race.drain_cues();
        let (atlas, _) = race.magstrip_wake_vertices();
        if (0..slots).any(|slot| race.over_magstrip(slot)) && !atlas.is_empty() {
            drawn = Some(tick);
            break;
        }
    }
    println!("first tick a craft is over a strip with the wake drawn: {drawn:?}");
    assert!(drawn.is_some(), "no craft drew an arc within 2400 ticks");
}
