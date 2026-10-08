//! Wipeout HD's Rocket smoke ribbon on a real disc: the opacity ramp read the
//! way `RibbonEffects_LoadOpacityRamp` reads it, and a fired volley laying
//! three ribbons that outlive the rockets and then free themselves.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_rocket_smoke_ground_truth)'
//! ```
//!
//! The ramp entries asserted are the alphas the live RPCS3 dump read off the
//! original's nodes (`docs/ghidra/functions/ps3-hdfury-eu/rocket-trail.md`):
//! a table read from the TGA's header end (byte 18) instead of byte 20 fails.

use oag_gameplay::{InputSnapshot, PlayerInputs};
use oag_raceplay as race;

/// The tick the volley is fired on, after the countdown.
const FIRE: u64 = 421;

fn loaded() -> Option<race::Loaded> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in loaded.report.iter().filter(|l| l.contains("rocket smoke")) {
        println!("{line}");
    }
    Some(loaded)
}

#[test]
#[ignore = "needs a disc image"]
fn the_ramp_is_read_two_bytes_past_the_header_and_matches_the_live_alphas() {
    let Some(loaded) = loaded() else {
        return;
    };
    let assets = loaded
        .ribbon_textures
        .rocket_smoke
        .as_ref()
        .expect("smoke_trails_frame2_alpha.gtf and the opacity ramp must both load");
    let table = assets.ramp.table();
    // Live: 0xf1, 0xdf and 0xc0 at ramp indices 2, 11 and 20; the engine's
    // first entry is 250 and its last two are the TGA footer's zeros.
    assert_eq!(
        [table[0], table[2], table[11], table[20]],
        [250, 241, 223, 192]
    );
    assert_eq!([table[254], table[255]], [0, 0]);
    assert!(assets.texture.width > 0 && assets.texture.height > 0);
}

#[test]
#[ignore = "needs a disc image"]
fn a_volley_lays_three_ribbons_that_age_out_after_the_rockets() {
    let Some(loaded) = loaded() else {
        return;
    };
    let ramp = loaded
        .ribbon_textures
        .rocket_smoke
        .as_ref()
        .expect("assets")
        .ramp
        .clone();
    let mut race = race::Race::start(loaded.setup);
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Rocket);
    let mut buttons = oag_core::buttons::Input::new();
    let mut peak = 0;
    let mut drew = false;
    let mut emptied_at = None;
    for tick in 1..=FIRE + 400 {
        let held = if tick == FIRE {
            1 << oag_core::buttons::Button::Square as u32
        } else {
            0
        };
        buttons.begin_frame(held);
        race.tick(&PlayerInputs::single(InputSnapshot {
            buttons,
            ..InputSnapshot::EMPTY
        }));
        let live = race.rocket_smoke_live();
        peak = peak.max(live);
        drew |= !race.rocket_smoke_vertices(&ramp).is_empty();
        if tick > FIRE + 1 && live == 0 && emptied_at.is_none() {
            emptied_at = Some(tick);
        }
    }
    assert_eq!(peak, 3, "one ribbon per rocket of the volley");
    assert!(drew, "the ribbons drew no geometry");
    let emptied = emptied_at.expect("the ribbons never freed themselves");
    // Each lives 1.85 s past its last node, about 111 ticks after its rocket.
    assert!(emptied > FIRE + 111, "freed at {emptied}");
}
