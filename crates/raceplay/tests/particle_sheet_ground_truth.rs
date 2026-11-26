//! Every sprite a title's race effects read is on the particle sprite sheet: one
//! that does not fit is drawn as the procedural profile, with only a report line
//! to say so.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.

use oag_raceplay as race;

/// Loads a race on `source` and returns its loader report, checked for dropped
/// sprites, or `None` where the source is not here.
fn report(source: &str) -> Option<String> {
    let source = oag_testdata::image(source)?;
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let report = loaded.report.join("\n");
    let dropped: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("did not fit the sprite sheet"))
        .collect();
    assert!(dropped.is_empty(), "{dropped:#?}");
    Some(report)
}

/// The loader report's line for `effect`.
fn line<'a>(report: &'a str, effect: &str) -> &'a str {
    let head = format!("{effect}: ");
    report
        .lines()
        .find(|line| line.starts_with(&head) && line.contains("emitter(s)"))
        .unwrap_or_else(|| panic!("{effect} was not loaded"))
}

/// HD (with Fury's content, the same disc): the Rocket's craft hit reads all
/// twelve of its sprites, and it and the Detonator's waves are all on the sheet.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn every_hd_sprite_is_on_the_sheet_and_the_craft_blast_reads_twelve() {
    let Some(report) = report("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let craft = line(&report, "WO_ROCKET_EXPLO");
    assert!(
        craft.contains("12 emitter(s)") && craft.ends_with("12 sprite(s) read"),
        "{craft}"
    );
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
