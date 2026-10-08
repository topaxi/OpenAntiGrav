//! HD's start gantry after the first line crossing: the race manager's three
//! later windows, each showing the board state the original shows there.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run it with
//! `just test-data`. The test skips with a message when the image is absent
//! and fails instead under `OAG_REQUIRE_GAME_DATA=1`.
//!
//! # What this pins
//!
//! `RaceManager_Update` (`0x0005e948`, read from the EBOOT, confidence 85)
//! picks a window by the player's lap and resets the gantry's time to its
//! start: `[6.017, 9.3)` between laps, `[9.5, 9.9)` on the lap before the
//! last, `[12.35, 13.3)` on the last. Forced on RPCS3 from the grid
//! (2026-10-04, Talon's Junction), the original shows the `FX-350` sponsor art,
//! a strobing `FINAL LAP` and a scrolling chequered flag, with nothing beside
//! or above the board. Through this build's loader, on the same circuit:
//!
//! - between laps the `FX-350` art (`fx350_nomip.gtf`) is drawn, and neither
//!   the `FINAL LAP` letters nor the flag;
//! - on the lap before the last the `FINAL LAP` letters are drawn and lit for
//!   part of the window's 24-tick loop, dark for another part;
//! - on the last lap the flag (`checkered.gtf`) is drawn, and no `FX-350`.
//!
//! Every window's start is also checked to be the authored state's: move a
//! window's bounds and a different board shows there.

use oag_mesh::mesh::{DrawCall, Model};
use oag_raceplay as race;
use oag_raceplay::gantry;

fn placed() -> Option<oag_raceplay::adverts::Card> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some("/data/environments/talons_junction/track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let line = loaded
        .report
        .iter()
        .find(|l| l.starts_with("start gantry /"))
        .unwrap_or_else(|| panic!("no gantry line: {:#?}", loaded.report));
    assert!(line.contains("plays the FX-350 board"), "{line}");
    assert!(line.contains("fx350_nomip.gtf"), "{line}");
    loaded
        .billboards
        .adverts
        .into_iter()
        .find(|card| card.slot == 8)
}

fn draws(model: &Model) -> impl Iterator<Item = &DrawCall> {
    model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
}

fn bound_to<'a>(model: &'a Model, file: &'a str) -> impl Iterator<Item = u32> + 'a {
    draws(model)
        .filter(move |d| {
            d.texture
                .and_then(|t| model.textures.get(t)?.as_ref())
                .is_some_and(|t| t.label.ends_with(file))
        })
        .map(gantry_key)
}

fn gantry_key(draw: &DrawCall) -> u32 {
    oag_render::gantry::panel::key(draw)
}

/// How many of `keys` are drawn at `seconds`.
fn shown(cull: &gantry::PanelCull, keys: &[u32], seconds: f32) -> usize {
    let hidden = cull.hidden(seconds);
    keys.iter().filter(|k| !hidden.contains(k)).count()
}

/// The lit white a set of draws samples at `seconds`, as the clock test
/// measures `GO`.
fn lit_white(model: &Model, keys: &[u32], seconds: f32) -> u32 {
    let table = oag_mesh::mesh_render::TexAnims::sample(model, seconds);
    let mut total = 0;
    for draw in draws(model).filter(|d| keys.contains(&gantry_key(d))) {
        let Some(texture) = draw.texture.and_then(|t| model.textures.get(t)?.as_ref()) else {
            continue;
        };
        let Some(rgba) = texture.to_rgba() else {
            continue;
        };
        let (w, h) = (texture.width as usize, texture.height as usize);
        for index in &model.indices[draw.range.start as usize..draw.range.end as usize] {
            let vertex = &model.vertices[*index as usize];
            let [su, sv, ou, ov] = table.transform[vertex.anim as usize];
            let u = (vertex.texcoord[0] * su + ou).rem_euclid(1.0);
            let v = (vertex.texcoord[1] * sv + ov).rem_euclid(1.0);
            let x = ((u * w as f32) as usize).min(w - 1);
            let y = ((v * h as f32) as usize).min(h - 1);
            let p = &rgba[(y * w + x) * 4..(y * w + x) * 4 + 4];
            if p[0] >= 200 && p[1] >= 200 && p[2] >= 200 {
                total += u32::from(p[3]);
            }
        }
    }
    total
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn each_lap_window_draws_the_board_the_original_shows() {
    let Some(placed) = placed() else { return };
    let model = &placed.model;
    let cull = placed
        .timeline
        .as_ref()
        .expect("the gantry card is clocked")
        .cull();
    let fx350: Vec<u32> = bound_to(model, "fx350_nomip.gtf").collect();
    let flag: Vec<u32> = bound_to(model, "checkered.gtf").collect();
    // `FINAL LAP` shares `GO`'s texture file; its letters are the draws of
    // that file parked off the panel when the race loads.
    let letters: Vec<u32> = bound_to(model, "321_go_64.gtf")
        .filter(|k| cull.countdown.contains(k))
        .collect();
    assert_eq!(fx350.len(), 7);
    assert_eq!(flag.len(), 1);
    assert_eq!(letters.len(), 8);

    let (between, ahead, chequered) = (
        gantry::HD_BETWEEN_LAPS_WINDOW,
        gantry::HD_FINAL_LAP_AHEAD_WINDOW,
        gantry::HD_CHEQUERED_WINDOW,
    );
    let span = |(from, to): (f32, f32)| {
        let n = ((to - from) * 60.0).round() as u32;
        (0..n).map(move |k| from + k as f32 / 60.0)
    };

    // Before the first crossing: none of the three.
    for seconds in [0.0, 3.83, 5.2] {
        assert_eq!(shown(cull, &fx350, seconds), 0, "{seconds}");
        assert_eq!(shown(cull, &flag, seconds), 0, "{seconds}");
        assert_eq!(shown(cull, &letters, seconds), 0, "{seconds}");
    }
    // Between laps: the FX-350 art, all seven pieces by the window's end.
    for seconds in span(between) {
        assert_eq!(shown(cull, &flag, seconds), 0, "{seconds}");
        assert_eq!(shown(cull, &letters, seconds), 0, "{seconds}");
    }
    assert_eq!(shown(cull, &fx350, between.1 - 0.05), 7);
    // The lap before the last: the letters, strobing.
    let lit: Vec<u32> = span(ahead).map(|s| lit_white(model, &letters, s)).collect();
    for seconds in span(ahead) {
        assert_eq!(shown(cull, &letters, seconds), 8, "{seconds}");
        assert_eq!(shown(cull, &fx350, seconds), 0, "{seconds}");
        assert_eq!(shown(cull, &flag, seconds), 0, "{seconds}");
    }
    let (max, min) = (*lit.iter().max().unwrap(), *lit.iter().min().unwrap());
    assert!(max > 0, "FINAL LAP never lights: {lit:?}");
    assert!(min < max / 4, "FINAL LAP does not strobe: {lit:?}");
    // The last lap: the flag.
    for seconds in span(chequered) {
        assert_eq!(shown(cull, &flag, seconds), 1, "{seconds}");
        assert_eq!(shown(cull, &fx350, seconds), 0, "{seconds}");
    }
}
