//! The "medal awarded" line, on Pulse's own HUD layout and string table.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run it with
//! `just test-data`, or only this file:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(medal_message_ground_truth)'
//! ```
//!
//! # What only real data can say here
//!
//! That `ER_GMA`/`ER_SMA`/`ER_BMA` resolve in the table the HUD draws with, so
//! the line reads "Gold medal awarded" and not the bare id; that the Zone and
//! Speed Lap layouts author the `Info1`..`Info4` widgets it is drawn in; and that a message raised on a real `Race` reaches the drawn frame
//! and is gone four seconds on. The trigger - a medal earned in a campaign
//! Zone or Speed Lap - is chosen, not measured; see
//! `oag_hud::messages` for what is.

use oag_gameplay::PlayerInputs;
use oag_ui::frontend::Draw;

fn load(image: &std::path::Path, mode: oag_race::Mode) -> oag_raceplay::Loaded {
    oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode,
        track: Some(r"Data\Environments\16_Track\track.vex".to_string()),
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading 16_Track")
}

fn drawn(loaded: &oag_raceplay::Loaded, readout: &oag_hud::Readout) -> Vec<String> {
    let context = loaded.hud.context().expect("Pulse's layout parses");
    let frame = oag_hud::draw_list(&context, readout);
    frame
        .hud_text
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_raised_medal_phrase_reads_as_the_disc_words_it_and_expires() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    for mode in [oag_race::Mode::Zone, oag_race::Mode::SpeedLap] {
        let loaded = load(&image, mode);
        let mut race = oag_raceplay::Race::start(loaded.setup.clone());
        let context = loaded.hud.context().expect("Pulse's layout parses");
        for id in ["ER_GMA", "ER_SMA", "ER_BMA"] {
            assert_ne!(
                context.strings.get_or_id(id),
                id,
                "{id} resolves ({mode:?})"
            );
        }
        let line = |race: &oag_raceplay::Race| {
            drawn(&loaded, &race.readout())
                .into_iter()
                .find(|text| text.to_lowercase().contains("medal"))
        };
        assert_eq!(line(&race), None, "nothing before a medal ({mode:?})");

        race.raise_message("ER_GMA", true);
        for _ in 0..30 {
            race.tick(&PlayerInputs::none());
        }
        let shown = line(&race).unwrap_or_else(|| panic!("a raised medal draws ({mode:?})"));
        assert_eq!(shown, context.strings.get_or_id("ER_GMA"));

        for _ in 0..300 {
            race.tick(&PlayerInputs::none());
        }
        assert_eq!(line(&race), None, "gone after its four seconds ({mode:?})");
    }
}
