//! Hillside's rain on a real race of the track file: dry on every lap but the last, the
//! host title's own rain (Fort Gale's `WO_RAIN` and `WO_RAIN_LENS`) from the tick the
//! first craft enters its final lap, and never on a disc circuit.
//!
//! **`#[ignore]`d and never run in CI**: it needs `data/images/pulse-psp-eu.chd`, the
//! `assets/mods/examples` submodule and the `extensions` feature:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --features extensions \
//!     --run-ignored only -E 'test(/hillside_rain_ground_truth::/)' --no-capture
//! ```
//!
//! The module is the committed build (`assets/mods/examples/hillside/ext/hillside_rain.wasm`),
//! loaded as the game loads it: its manifest through `oag-mod`, bound to its track file
//! by `[extension.track]`. A one-lap Single Race has every racing craft on its final lap
//! from the green flag, a two-lap one has none for a while.
#![cfg(feature = "extensions")]

use std::sync::Arc;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

const MOD: &str = "assets/mods/examples/hillside";

fn hillside_rain() -> Option<(race::extensions::Source, std::path::PathBuf)> {
    let manifest = oag_testdata::exact(&format!("{MOD}/oag-mod.toml"))?;
    let module = oag_mod::Mod::open(manifest.parent().unwrap()).expect("the example mod opens");
    let extension = module.manifest.extension.clone().expect("[extension]");
    let track = extension.track.expect("an [extension.track] module");
    let wasm = module.read(&track.module).unwrap().expect("the module");
    let file = module.local_path(&track.track).expect("a folder mod");
    let source = race::extensions::Source {
        mod_id: module.manifest.id.clone(),
        module: track.module,
        titles: module.manifest.titles.clone(),
        api: extension.api,
        memory_pages: extension.memory_pages,
        state_bytes: extension.state_bytes,
        scope: race::extensions::Scope::Track(Some(file.clone())),
        read: Arc::new(move || Ok(wasm.clone())),
        strings: Arc::new(|_| Default::default()),
        icons: Vec::new(),
        effects: Vec::new(),
        files: Arc::new(|_| Err("no files".into())),
        pickup: module.manifest.pickup.clone(),
        channels: race::extensions::Declared {
            publishes: module.manifest.channels.publishes.clone(),
            listens: module.manifest.channels.listens.clone(),
        },
    };
    Some((source, file))
}

fn options(
    laps: u32,
    track: Option<&std::path::Path>,
    with: Vec<race::extensions::Source>,
) -> Option<race::Options> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    Some(race::Options {
        source: image.display().to_string(),
        track: track
            .is_none()
            .then(|| "Data\\Environments\\16_Track\\track.vex".to_string()),
        foreign_track: track.map(std::path::Path::to_path_buf),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        laps_override: Some(laps),
        opponents: true,
        extensions: with,
        ..race::Options::default()
    })
}

/// What the weather holds after `ticks`: raining, drops alive in the air and on the
/// glass, the wet road, drops the enhanced splashes can find, the modded flag.
fn weather(
    options: race::Options,
    ticks: u32,
) -> (bool, usize, usize, f32, usize, bool, race::extensions::Sky) {
    let loaded = race::load(&options).expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
    }
    let weather = race.scenery_fx().weather();
    (
        weather.is_driven_active(),
        weather.pool().alive_count(),
        weather.lens().alive_count(),
        weather.wet_road(),
        weather.drop_segments().len(),
        race.is_modded(),
        race.extension_sky(),
    )
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn the_final_lap_rains_the_host_titles_own_rain_and_an_earlier_lap_does_not() {
    let Some((source, file)) = hillside_rain() else {
        return;
    };
    // Two laps: nobody has finished the first within 600 ticks (the countdown alone
    // is most of them), so nothing is sent, nothing rains, and nothing draws.
    let early = weather(options(2, Some(&file), vec![source.clone()]).unwrap(), 600);
    println!("lap 1 of 2: {early:?}");
    assert!(early.5, "the track module loaded");
    assert_eq!(
        (early.0, early.1, early.2, early.3, early.4, early.6),
        (false, 0, 0, 0.0, 0, race::extensions::Sky::default())
    );
    // One lap: every racing craft is on its final lap from the green flag.
    let late = weather(options(1, Some(&file), vec![source]).unwrap(), 900);
    println!("final lap: {late:?}");
    let (raining, drops, glass, wet, hits, _, sky) = late;
    assert!(raining, "a craft is on its final lap");
    assert!(drops > 0, "the rain's own drops are alive in the air");
    assert!(glass > 0, "WO_RAIN_LENS's drops are on the glass");
    assert!(wet > 0.0, "the road is wetting");
    assert!(hits > 0, "the enhanced splashes have drops to find");
    assert_eq!(sky.sun, 0.0, "the ramp has run its 3 s");
    assert_eq!((sky.rain, sky.grey), (Some(1.0), 1.0));
}

#[test]
#[ignore = "needs a Pulse disc image, the mod examples and --features extensions"]
fn a_disc_circuit_never_runs_the_track_module() {
    let Some((source, _)) = hillside_rain() else {
        return;
    };
    let loaded = race::load(&options(1, None, vec![source]).unwrap()).expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..900 {
        race.tick(&PlayerInputs::none());
    }
    assert!(!race.is_modded(), "the module is bound to Hillside's file");
    assert!(!race.scenery_fx().weather().is_driven_active());
    assert_eq!(race.extension_sky(), race::extensions::Sky::default());
}
