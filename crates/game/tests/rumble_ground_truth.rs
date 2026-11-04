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
    use oag_game::rumble_library::{Report, load};
    use oag_input::rumble::map::ROWS;

    fn open(image: &str) -> Option<oag_assets::Archives> {
        let image = oag_testdata::image(image)?;
        Some(
            oag_source::title::open_source(&image.to_string_lossy(), Vec::new(), Vec::new())
                .expect("the title opens")
                .archives,
        )
    }

    fn distinct() -> usize {
        let mut names: Vec<_> = ROWS.iter().map(|r| r.pattern).collect();
        names.sort_unstable();
        names.dedup();
        names.len()
    }

    #[test]
    #[ignore = "needs data/images/pulse-psp-eu.chd"]
    fn pulse_plays_every_mapped_pattern_from_our_own_fallback() {
        let Some(mut archives) = open("pulse-psp-eu.chd") else {
            return;
        };
        let (library, report) = load(&mut archives);
        assert_eq!((report.disc, report.overridden), (0, 0));
        assert_eq!(report.fallback, distinct(), "{}", report.line());
        assert_eq!(library.len(), distinct());
    }

    #[test]
    #[ignore = "needs data/images/pure-psp-eu.chd"]
    fn pure_plays_every_mapped_pattern_from_our_own_fallback() {
        let Some(mut archives) = open("pure-psp-eu.chd") else {
            return;
        };
        let (_, report) = load(&mut archives);
        assert_eq!(
            (report.disc, report.fallback),
            (0, distinct()),
            "{}",
            report.line()
        );
    }

    #[test]
    #[ignore = "needs data/images/pulse-ps2-eu.chd"]
    fn pulse_ps2_has_the_same_library_as_the_psp() {
        let Some(mut archives) = open("pulse-ps2-eu.chd") else {
            return;
        };
        let (_, report) = load(&mut archives);
        assert_eq!(report.fallback, distinct(), "{}", report.line());
    }

    /// The fallback answers only a name the disc lacks: on HD every mapped name is the disc's.
    #[test]
    #[ignore = "needs data/images/hdfury-ps3-eu.iso"]
    fn hd_keeps_its_own_patterns_over_ours() {
        let Some(mut archives) = open("hdfury-ps3-eu.iso") else {
            return;
        };
        let (_, report) = load(&mut archives);
        assert!(report.disc > 0, "{}", report.line());
        assert_eq!(report.overridden, 0);
    }

    #[test]
    fn a_title_with_no_pattern_says_so_and_plays_nothing() {
        let line = Report::default().line();
        assert!(line.contains("no pattern found"), "{line}");
    }
}
