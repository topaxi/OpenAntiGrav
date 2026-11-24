//! Pure's `<Weather>` element, pinned against its disc and played by the race.
//!
//! Sinucit authors rain (with the lens droplets) and Sebenco Climb snow, each with
//! the `Mist.mip` screen overlay; no other circuit the disc names does
//! (`docs/rendering/pure-weather.md`). The original draws the overlay as two additive
//! screen layers, which is the whole-frame lift of about 24 grey the two compare pairs
//! carried before it was played.
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
