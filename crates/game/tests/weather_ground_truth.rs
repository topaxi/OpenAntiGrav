//! Which circuits author a `<Weather>` element, pinned against the discs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test weather_ground_truth --run-ignored all
//! ```
//!
//! `TrackStartup_Parse`'s `LevelFx` branch builds `Weather_Construct`'s node
//! from it. Only Outpost 7 (`07_Track`, snow) and Fort Gale (`14_Track`, rain
//! and its lens) author one on the PSP; the PS2 disc authors the same two and
//! a third (`11_Track`). See
//! `docs/ghidra/functions/psp-pulse-usa/weather.md`.

use oag_tables::trackstartup::TrackStartup;

fn weather_circuits(image: &str) -> Option<Vec<(String, String, Option<String>)>> {
    let path = oag_testdata::image(image)?;
    let mut archives = oag_pulse::open(path.to_str().expect("utf-8 path")).expect("open");
    let mut found = Vec::new();
    for n in 1..=16 {
        let name = format!(r"Data\Environments\{n:02}_Track\trackstartup.xml");
        let Ok(blob) = archives.read_name(&name) else {
            continue;
        };
        let manifest = TrackStartup::parse(&String::from_utf8_lossy(&blob));
        if let Some(weather) = manifest.weather {
            found.push((
                format!("{n:02}"),
                weather.env_psys.unwrap_or_default(),
                weather.screen_psys,
            ));
        }
    }
    Some(found)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn only_outpost_seven_and_fort_gale_author_weather_on_the_psp() {
    let Some(found) = weather_circuits("pulse-psp-usa.chd") else {
        return;
    };
    assert_eq!(
        found,
        [
            ("07".to_string(), r"data\psys\WO_SNOW.POB".to_string(), None),
            (
                "14".to_string(),
                r"data\psys\WO_RAIN.POB".to_string(),
                Some(r"data\psys\WO_RAIN_LENS.POB".to_string())
            ),
        ]
    );
}

/// The PS2 disc authors the same element with the same attributes, and one
/// circuit more: `11_Track`, which the PSP disc does not ship. Outpost 7 and
/// Fort Gale are the same on both.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn the_ps2_disc_authors_the_same_element_and_a_third_circuit() {
    let Some(found) = weather_circuits("pulse-ps2-eu.chd") else {
        return;
    };
    let names: Vec<&str> = found.iter().map(|(n, _, _)| n.as_str()).collect();
    assert_eq!(names, ["07", "11", "14"]);
    assert_eq!(found[1].1, r"data\psys\WO_RAIN.POB");
    assert_eq!(found[1].2.as_deref(), Some(r"data\psys\WO_RAIN_LENS.POB"));
}
