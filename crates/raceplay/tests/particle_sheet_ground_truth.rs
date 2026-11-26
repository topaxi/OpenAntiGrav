//! Every sprite a title's race effects read is on the particle sprite sheet: one
//! that does not fit is drawn as the procedural profile, with only a report line
//! to say so.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.

use oag_raceplay as race;

/// Loads a race on `source` (on `track`, else the title's own first circuit),
/// or `None` where the source is not here.
fn load_on(source: &str, track: Option<&str>) -> Option<race::Loaded> {
    let source = oag_testdata::image(source)?;
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        track: track.map(str::to_string),
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(loaded)
}

/// The loader report of [`load_on`], checked for dropped sprites.
fn checked_report(loaded: &race::Loaded) -> String {
    let report = loaded.report.join("\n");
    let dropped: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("did not fit the sprite sheet"))
        .collect();
    assert!(dropped.is_empty(), "{dropped:#?}");
    report
}

fn report_on(source: &str, track: Option<&str>) -> Option<String> {
    load_on(source, track).map(|loaded| checked_report(&loaded))
}

fn report(source: &str) -> Option<String> {
    report_on(source, None)
}

/// The loader report's line for `effect`.
fn line<'a>(report: &'a str, effect: &str) -> &'a str {
    let head = format!("{effect}: ");
    report
        .lines()
        .find(|line| line.starts_with(&head) && line.contains("emitter(s)"))
        .unwrap_or_else(|| panic!("{effect} was not loaded"))
}

/// Every HD effect whose camera-facing emitters a player sees draw the disc's
/// own sprite: named one by one, so dropping any from `oag_hd`'s `SPRITES`
/// turns this red.
const HD_WIRED: &[&str] = &[
    "WO_MINE_EXPLO",
    "WO_BOMB_SMOKERING",
    "WO_MISSILE_EXPLO",
    "WO_MISSILE_BOUNCE",
    "WO_MINE_EXPLO_DETONATOR_DEAD",
    "WO_MINE_EXPLO_DETONATOR",
    "WO_BOMB_EXPLO_DETONATOR",
    "WO_ROCKET_FLARE",
    "WO_CANNON_SPARKS",
    "WO_SHURIKEN_HEAD",
    "WO_SHURIKEN_TRAIL",
    "WO_SHURIKEN_BOUNCE",
    "WO_SHURIKEN_EXPIRE",
    "WO_REPULSER_BLAST",
    "WO_WEAPON_ABSORB",
    "WO_SHIP_COLL_SPARK_DAMAGE",
    "WO_SHIP_DEATH_DAMAGE_PLUME",
    "WO_DEBRIS_FIRE",
    "WO_DAMAGE_MILD",
    "WO_DAMAGE_MODERATE",
    "WO_MODESTO_STEAM_A",
];

/// Every camera-facing emitter of each [`HD_WIRED`] effect has its sprite decoded and
/// placed on the sheet, and the report names none it could not read.
fn check_hds_billboards_draw_their_sprites(loaded: &race::Loaded, report: &str) {
    for name in HD_WIRED {
        let effect = loaded
            .view
            .effects
            .get(name)
            .unwrap_or_else(|| panic!("{name} was not loaded"));
        let billboards: Vec<_> = effect
            .emitters
            .iter()
            .filter(|spec| spec.render == oag_fx::psys::Render::Billboard)
            .collect();
        assert!(
            !billboards.is_empty(),
            "{name} has no camera-facing emitter"
        );
        for spec in billboards {
            assert!(
                spec.sprite.is_some() && spec.sheet_rect.is_some(),
                "{name}: {} draws no sprite",
                spec.name
            );
        }
        let line = line(report, name);
        assert!(
            line.contains("sprite(s) read") && !line.contains("not read"),
            "{line}"
        );
    }
}

/// HD (with Fury's content, the same disc): the Rocket's craft hit reads all
/// twelve of its emitters' sprites and its three templates', the weapon explosions
/// and ship damage effects draw theirs, and all of it is on the sheet.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn every_hd_sprite_is_on_the_sheet_and_the_weapon_blasts_draw_theirs() {
    let Some(loaded) = load_on("data/images/hdfury-ps3-eu-dec.iso", None) else {
        return;
    };
    let report = checked_report(&loaded);
    let craft = line(&report, "WO_ROCKET_EXPLO");
    assert!(
        craft.contains("15 emitter(s)") && craft.ends_with("15 sprite(s) read"),
        "{craft}"
    );
    check_hds_billboards_draw_their_sprites(&loaded, &report);
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn every_2048_sprite_is_on_the_sheet() {
    let Some(report) = report("data/extracted/vita/PCSF00007") else {
        return;
    };
    let craft = line(&report, "WO_ROCKET_EXPLO");
    assert!(craft.contains("sprite(s) read"), "{craft}");
}

#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn every_omega_sprite_is_on_the_sheet() {
    let Some(report) = report("data/images/omega-ps4-eu.pkg") else {
        return;
    };
    let craft = line(&report, "WO_ROCKET_EXPLO");
    assert!(craft.contains("sprite(s) read"), "{craft}");
}

/// Omega ships a second effect set for its 2048-era circuits
/// (`Data\particles2048`), which a race on one of them loads instead.
#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn every_omega_2048_era_sprite_is_on_the_sheet() {
    let Some(report) = report_on(
        "data/images/omega-ps4-eu.pkg",
        Some(r"Data\Environments2048\tower\track.vex"),
    ) else {
        return;
    };
    // Four emitters here against the other set's two: the 2048-era file.
    let plasma = line(&report, "WO_PLASMA_LIGHTNING_EXPAND");
    assert!(plasma.contains("4 emitter(s)"), "{plasma}");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and pulse-ps2-eu.chd"]
fn every_pulse_sprite_is_on_the_sheet() {
    report("data/images/pulse-psp-eu.chd");
    report("data/images/pulse-ps2-eu.chd");
}

#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn every_pure_sprite_is_on_the_sheet() {
    report("data/images/pure-psp-eu.chd");
}
