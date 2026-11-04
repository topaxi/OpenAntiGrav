//! A disc's rumble pattern, read by the probe's own reader and played by the
//! player, on HD/Fury's real image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game rumble_ground_truth:: --run-ignored all
//! ```
//!
//! What only real data says: the path spellings `oag_raceplay::rumble_library::disc_pattern`
//! tries resolve a pattern in HD's archive, and the player plays the file's own
//! bytes out under HD's small-motor law.

use oag_input::rumble::{Mode, Player, StepContext};
use oag_tables::rumble::{Kind, Library};

fn hd() -> Option<oag_assets::Archives> {
    let image = oag_testdata::image("hdfury-ps3-eu.iso")?;
    Some(
        oag_source::title::open_source(&image.to_string_lossy(), Vec::new(), Vec::new())
            .expect("HD opens")
            .archives,
    )
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu.iso"]
fn hd_plays_its_own_missile_pattern_under_the_ps3_law() {
    let Some(mut archives) = hd() else { return };
    let pattern = oag_raceplay::rumble_library::disc_pattern(&mut archives, "Missile_Fired_Rumble")
        .expect("the disc has it");
    assert_eq!(pattern.kind, Kind::OneShot);
    let (large, small) = (pattern.large.clone(), pattern.small.clone());
    assert!(!large.is_empty() && small.iter().any(|&b| b != 0));

    let law = Mode::Original
        .law(oag_game::rumble::original_law(oag_hd::TITLE.rumble))
        .expect("HD's original plays patterns");
    let mut library = Library::new();
    library.insert("missile", pattern);
    let mut player = Player::new(library, law);
    assert!(player.one_shot(0, None));
    let ctx = StepContext {
        race: true,
        ..StepContext::default()
    };
    for (frame, &want) in large.iter().enumerate() {
        let motors = player.step(ctx);
        assert_eq!(motors.large, want, "large, frame {frame}");
        let on = small.get(frame).copied().unwrap_or(0) != 0;
        assert_eq!(
            motors.small,
            if on { 255 } else { 0 },
            "small, frame {frame}"
        );
    }
    assert_eq!(player.step(ctx), oag_input::rumble::Motors::OFF);
    assert_eq!(player.active_voices(), 0);
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu.iso"]
fn a_name_the_disc_does_not_have_is_refused_by_name() {
    let Some(mut archives) = hd() else { return };
    let err = oag_raceplay::rumble_library::disc_pattern(&mut archives, "No_Such_Pattern")
        .expect_err("not on the disc");
    assert!(err.to_string().contains("No_Such_Pattern"), "{err}");
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn pulse_has_no_patterns_and_its_original_offers_none() {
    let Some(image) = oag_testdata::image("pulse-psp-eu.chd") else {
        return;
    };
    let mut opened =
        oag_source::title::open_source(&image.to_string_lossy(), Vec::new(), Vec::new())
            .expect("Pulse opens");
    assert!(
        oag_raceplay::rumble_library::disc_pattern(&mut opened.archives, "Missile_Fired_Rumble")
            .is_err()
    );
    assert_eq!(
        Mode::Original.law(oag_game::rumble::original_law(opened.title.rumble)),
        None
    );
}

mod library {
    use oag_input::rumble::map::pattern_names;

    fn open(image: &str) -> Option<oag_assets::Archives> {
        let image = oag_testdata::image(image)?;
        Some(
            oag_source::title::open_source(&image.to_string_lossy(), Vec::new(), Vec::new())
                .expect("the title opens")
                .archives,
        )
    }

    fn distinct() -> usize {
        pattern_names().count()
    }

    fn load(image: &str) -> Option<(oag_tables::rumble::Library, Vec<String>)> {
        let mut archives = open(image)?;
        let mut report = Vec::new();
        let library = oag_raceplay::rumble_library::load(&mut archives, &mut report);
        Some((library, report))
    }

    #[test]
    #[ignore = "needs data/images/pulse-psp-eu.chd"]
    fn pulse_plays_every_mapped_pattern_from_our_own_fallback() {
        let Some((library, report)) = load("pulse-psp-eu.chd") else {
            return;
        };
        assert_eq!(library.len(), distinct(), "{report:?}");
        assert!(report[0].contains(&format!("0 patterns from the disc, {} from", distinct())));
    }

    #[test]
    #[ignore = "needs data/images/pure-psp-eu.chd"]
    fn pure_plays_every_mapped_pattern_from_our_own_fallback() {
        let Some((library, report)) = load("pure-psp-eu.chd") else {
            return;
        };
        assert_eq!(library.len(), distinct(), "{report:?}");
    }

    #[test]
    #[ignore = "needs data/images/pulse-ps2-eu.chd"]
    fn pulse_ps2_has_the_same_library_as_the_psp() {
        let Some((ps2, _)) = load("pulse-ps2-eu.chd") else {
            return;
        };
        let Some((psp, _)) = load("pulse-psp-eu.chd") else {
            return;
        };
        assert_eq!(ps2, psp);
    }

    /// The fallback answers only a name the disc lacks: on HD every mapped name is the disc's.
    #[test]
    #[ignore = "needs data/images/hdfury-ps3-eu.iso"]
    fn hd_keeps_its_own_patterns_over_ours() {
        let Some((_, report)) = load("hdfury-ps3-eu.iso") else {
            return;
        };
        assert!(
            !report[0].contains(" 0 patterns from the disc"),
            "{report:?}"
        );
        assert!(report[0].contains("0 from overrides"), "{report:?}");
    }
}

mod race {
    use oag_gameplay::PlayerInputs;
    use oag_input::rumble::{MotorOutput, Motors};

    #[derive(Default)]
    struct Recorder {
        sent: Vec<Motors>,
        silenced: u32,
    }

    impl MotorOutput for Recorder {
        fn set(&mut self, _seat: usize, motors: Motors) {
            self.sent.push(motors);
        }
        fn silence(&mut self) {
            self.silenced += 1;
        }
    }

    /// A lone craft on the autopilot for `ticks`, its pad recorded under `token`.
    fn fly(
        image: &str,
        title: &oag_title::Title,
        token: &str,
        ticks: usize,
    ) -> Option<(Option<Vec<(&'static str, u32)>>, Recorder)> {
        let image = oag_testdata::image(image)?;
        let loaded = oag_raceplay::load(&oag_raceplay::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            mode: oag_race::Mode::TimeTrial,
            seed: Some(1),
            ..oag_raceplay::Options::default()
        })
        .expect("the race loads");
        let mut race = oag_raceplay::Race::start(loaded.setup);
        race.set_autopilot(true);
        let mut out = Recorder::default();
        let mut rumbler = oag_game::rumble::Rumbler::new(&race, token, title.rumble)?;
        for _ in 0..ticks {
            race.tick(&PlayerInputs::none());
            rumbler.tick(&race, false, &mut out);
            race.drain_cues();
        }
        let fired = rumbler.fired();
        rumbler.stop(&mut out);
        Some((Some(fired), out))
    }

    fn assert_rumbles(image: &str, title: &oag_title::Title) {
        let Some((fired, out)) = fly(image, title, "ps4", 3000) else {
            return;
        };
        let fired = fired.unwrap();
        assert!(!fired.is_empty(), "no pattern fired in 3000 ticks");
        assert!(out.sent.iter().any(|m| m.large > 0), "{fired:?}");
        assert_eq!(out.silenced, 1, "the race's end silences the pad");
        eprintln!("{} fired {fired:?}", title.name);
    }

    #[test]
    #[ignore = "needs data/images/pulse-psp-eu.chd"]
    fn a_pulse_race_rumbles_from_our_own_patterns() {
        assert_rumbles("pulse-psp-eu.chd", oag_pulse::TITLE);
    }

    #[test]
    #[ignore = "needs data/images/pulse-ps2-eu.chd"]
    fn a_pulse_ps2_race_rumbles_from_our_own_patterns() {
        assert_rumbles("pulse-ps2-eu.chd", oag_pulse::TITLE);
    }

    #[test]
    #[ignore = "needs data/images/pure-psp-eu.chd"]
    fn a_pure_race_rumbles_from_our_own_patterns() {
        assert_rumbles("pure-psp-eu.chd", oag_pure::TITLE);
    }

    #[test]
    #[ignore = "needs data/images/pulse-psp-eu.chd"]
    fn the_none_setting_and_original_play_nothing_on_pulse() {
        for token in ["none", "original"] {
            assert!(
                fly("pulse-psp-eu.chd", oag_pulse::TITLE, token, 10).is_none()
                    || oag_testdata::image("pulse-psp-eu.chd").is_none(),
                "{token}"
            );
        }
    }
}
