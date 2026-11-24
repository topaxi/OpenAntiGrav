//! Pure's `<Weather>` element, pinned against its disc and played by the race.
//!
//! Sinucit authors rain (with the lens droplets) and Sebenco Climb snow, each with
//! the `Mist.mip` screen overlay; no other circuit the disc names does
//! (`docs/rendering/pure-weather.md`). The original draws the overlay as two additive
//! screen layers: a whole-frame lift of about 24 grey on the two compare pairs.
//!
//! Drop the `circuit_weather` rule on Pure (`oag_pure::TITLE.looks`) and the weather,
//! its mist and its effect never load, so the race tests below fail.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay pure_weather_ground_truth:: --run-ignored all
//! ```

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_tables::trackstartup::TrackStartup;

const IMAGE: &str = "pure-psp-usa.chd";

const CIRCUITS: [&str; 13] = [
    r"Environments\01_Vineta_K",
    r"Environments\03_Modesto_Heights",
    r"Environments\04_Chenghou_Project",
    r"Environments\07_Blue_Ridge",
    r"Environments\08_Sinucit",
    r"Environments\09_Citta_Nuova_V",
    r"Environments\10_Sebenco_Climb",
    r"Environments\12_Sol_2",
    r"Zone\01_Zone",
    r"ClassicsNS\01_Karbonis",
    r"ClassicsNS\02_Sagarmartha",
    r"ClassicsNS\03_Manor_Top",
    r"ClassicsNS\04_Mandrashee",
];

fn started(circuit: &str) -> Option<(race::Race, Vec<String>)> {
    let image = oag_testdata::image(IMAGE)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VECTOR".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(format!(r"Data\{circuit}\track.vex")),
        ..race::Options::default()
    })
    .expect("loading the race");
    let report = loaded.report.clone();
    Some((
        race::Race::start(loaded.setup, loaded.view, loaded.modules),
        report,
    ))
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn only_sinucit_and_sebenco_climb_author_weather() {
    let Some(path) = oag_testdata::image(IMAGE) else {
        return;
    };
    let mut archives = oag_pure::open(path.to_str().expect("utf-8 path")).expect("open");
    let mut found = Vec::new();
    for circuit in CIRCUITS {
        let name = format!(r"Data\{circuit}\TrackStartup.xml");
        let blob = archives.read_name(&name).expect(&name);
        let manifest = TrackStartup::parse(&String::from_utf8_lossy(&blob));
        if let Some(weather) = manifest.weather {
            found.push((
                circuit,
                weather.env_psys.unwrap_or_default(),
                weather.screen_psys,
                weather.tex.unwrap_or_default(),
            ));
        }
    }
    assert_eq!(
        found,
        [
            (
                r"Environments\08_Sinucit",
                r"data\psys\WO_RAIN.POB".to_string(),
                Some(r"data\psys\WO_RAIN_LENS.POB".to_string()),
                r"Data\Tex\ScreenFX\Mist.mip".to_string()
            ),
            (
                r"Environments\10_Sebenco_Climb",
                r"data\psys\WO_SNOW.POB".to_string(),
                None,
                r"Data\Tex\ScreenFX\Mist.mip".to_string()
            ),
        ]
    );
}

/// Ticks a race a second and reports what its weather holds.
fn weather_after_a_second(circuit: &str) -> Option<(bool, bool, bool, bool)> {
    let (mut race, _) = started(circuit)?;
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }
    let weather = race.scenery_fx().weather();
    Some((
        weather.is_authored(),
        weather.mist_texture().is_some() && weather.mist().is_some(),
        weather.is_playing(),
        weather.is_snow(),
    ))
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn sinucit_plays_rain_with_its_mist() {
    let Some(held) = weather_after_a_second(r"Environments\08_Sinucit") else {
        return;
    };
    assert_eq!(held, (true, true, true, false));
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn sebenco_climb_plays_snow_with_its_mist() {
    let Some(held) = weather_after_a_second(r"Environments\10_Sebenco_Climb") else {
        return;
    };
    assert_eq!(held, (true, true, true, true));
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn a_circuit_that_authors_no_weather_plays_none() {
    let Some(held) = weather_after_a_second(r"Environments\01_Vineta_K") else {
        return;
    };
    assert_eq!(held, (false, false, false, false));
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn the_loader_reports_the_weather_and_the_mist() {
    let Some((_, report)) = started(r"Environments\08_Sinucit") else {
        return;
    };
    let has = |text: &str| report.iter().any(|line| line.contains(text));
    assert!(
        has("weather: data\\psys\\WO_RAIN.POB"),
        "{}",
        report.join("\n")
    );
    assert!(
        has("mist: Data\\Tex\\ScreenFX\\Mist.mip"),
        "{}",
        report.join("\n")
    );
}

/// The weather emitters spread by the extent law Pulse's weather does: the rain's and the snow's
/// box and the lens's rectangle, not a point at the origin. Pure's three `.pob` files carry the
/// same emitter records as Pulse's (`docs/rendering/pure-weather.md`, "The particles").
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn the_weather_emitters_keep_their_spawn_shapes() {
    use oag_fx::psys::spawn::Spawn;
    for (circuit, env, lens) in [
        (r"Environments\08_Sinucit", 50.0, true),
        (r"Environments\10_Sebenco_Climb", 100.0, false),
    ] {
        let Some((mut race, _)) = started(circuit) else {
            return;
        };
        for _ in 0..2 {
            race.tick(&PlayerInputs::none());
        }
        let weather = race.scenery_fx().weather();
        let root = &weather.env_effect().expect("the env effect").emitters[0];
        assert_eq!(root.spawn, Spawn::Box { extent: env }, "{circuit}");
        if lens {
            let root = &weather.lens().effect().expect("the lens effect").emitters[0];
            assert_eq!(
                root.spawn,
                Spawn::Rect {
                    extent: 10.0,
                    depth: 5.625
                },
                "{circuit}"
            );
        }
    }
}

/// Both circuits' `weatherPos` anchors read off Pure's version-4 `.vex`, so the mist fades and the
/// rain anchors under cover as on Pulse.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn the_cover_anchors_are_read_on_pure() {
    let Some(path) = oag_testdata::image(IMAGE) else {
        return;
    };
    let mut archives = oag_pure::open(path.to_str().expect("utf-8 path")).expect("open");
    let mut held = Vec::new();
    for circuit in [r"Environments\08_Sinucit", r"Environments\10_Sebenco_Climb"] {
        let blob = archives
            .read_name(&format!(r"Data\{circuit}\track.vex"))
            .expect("track");
        let nodes = oag_vex::vex::nodes(&blob).expect("nodes");
        let anchors = oag_vex::weather::anchors(&blob, &nodes);
        held.push((anchors.len(), oag_vex::weather::covered_mask(&anchors)));
    }
    assert_eq!(held, [(12, 1_265_574), (2, 34_359_742_464)]);
}

/// Pure offers no WEATHER setting, so a profile that says `enhanced` (a hand-edited file or
/// `--weather enhanced`) does not wet its road or throw splashes: the title's weather is played at
/// `original` only. Sinucit's rain would reach a full wet road by 2,400 ticks.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn the_enhanced_weather_stays_off_on_a_title_that_does_not_offer_it() {
    let Some((mut race, _)) = started(r"Environments\08_Sinucit") else {
        return;
    };
    race.set_weather(oag_display::display::Weather::Enhanced);
    let mut splashes = 0;
    for _ in 0..2600 {
        race.tick(&PlayerInputs::none());
        splashes += race.scenery_fx().weather().splashes().len();
    }
    assert_eq!(race.wet_road(), 0.0);
    assert_eq!(splashes, 0);
}
