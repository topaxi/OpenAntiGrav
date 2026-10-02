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

mod race {
    use oag_game::race::{self, scenery_fx::Weather, scenery_fx::weather::Setup};
    use oag_gameplay::PlayerInputs;
    use oag_render::psys::field::Frame;

    fn load(track: &str) -> Option<race::Loaded> {
        let image = oag_testdata::image("pulse-psp-usa.chd")?;
        Some(
            race::load(&race::Options {
                source: image.display().to_string(),
                class: "VENOM".to_string(),
                mode: oag_race::Mode::TimeTrial,
                track: Some(format!(r"Data\Environments\{track}")),
                ..race::Options::default()
            })
            .expect("loading the race"),
        )
    }

    fn weather_after(track: &str, ticks: u32) -> Option<(bool, bool, usize)> {
        let loaded = load(track)?;
        let mut race = race::Race::start(loaded.setup);
        for _ in 0..ticks {
            race.tick(&PlayerInputs::none());
        }
        let weather = race.scenery_fx().weather();
        Some((
            weather.is_authored(),
            weather.is_playing(),
            weather.pool().alive_count(),
        ))
    }

    /// Fort Gale's rain keeps 32 drops alive - 8 a tick for the 4 ticks of a
    /// drop's life, read live - and Outpost 7's snow its own; a circuit that
    /// authors no weather plays none.
    #[test]
    #[ignore = "needs data/images/pulse-psp-usa.chd"]
    fn the_two_circuits_play_their_weather_and_the_rest_do_not() {
        let Some((authored, playing, rain)) = weather_after(r"14_Track\track_reversed.vex", 90)
        else {
            return;
        };
        assert!(authored && playing, "Fort Gale authors rain");
        assert!(
            (28..=32).contains(&rain),
            "{rain} raindrops alive; the original holds 32"
        );
        let (authored, playing, snow) = weather_after(r"07_Track\track.vex", 92).expect("image");
        assert!(authored && playing, "Outpost 7 authors snow");
        // 64 a burst with a cap of 64 and a life of 5: the pool is full for five
        // ticks and empty for the sixth.
        assert!(snow > 0 && snow <= 64, "{snow} flakes alive");
        let (authored, playing, none) = weather_after(r"01_Track\track.vex", 90).expect("image");
        assert!(
            !authored && !playing && none == 0,
            "Basilico has no weather"
        );
    }

    /// The mode is held state, switched on a section change only: the first
    /// frame leaves "covered" (the startup section is below zero) for an open
    /// section, a covered one puts the field at its anchor, and leaving it puts
    /// the field back on the camera.
    #[test]
    #[ignore = "needs data/images/pulse-psp-usa.chd"]
    fn the_field_sits_at_the_anchor_only_while_the_camera_is_in_a_covered_section() {
        let Some(loaded) = load(r"14_Track\track_reversed.vex") else {
            return;
        };
        let setup: Setup = loaded.setup.scenery_fx.weather.clone().expect("weather");
        let covered = setup.anchors.first().expect("an anchor").section;
        let open = (0..64u8)
            .find(|s| !setup.anchors.iter().any(|a| a.section == *s))
            .expect("an open section");
        let mut weather = Weather::new(Some(setup));
        let library = &loaded.setup.effects;
        let camera = Frame::IDENTITY;
        let step = |weather: &mut Weather, section: u8| {
            weather.advance(library, 1.0 / 60.0, camera, i32::from(section));
        };
        step(&mut weather, open);
        assert!(!weather.is_anchored(), "an open section rides the camera");
        for _ in 0..3 {
            step(&mut weather, open);
        }
        step(&mut weather, covered);
        assert!(weather.is_anchored(), "a covered section holds the anchor");
        step(&mut weather, covered);
        assert!(
            weather.is_anchored(),
            "and keeps it while the section holds"
        );
        step(&mut weather, open);
        assert!(!weather.is_anchored(), "leaving it returns to the camera");
    }
}
