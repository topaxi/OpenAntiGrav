//! Pulse's end-of-race voice lines, off real discs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content. Run with
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test endrace_voice_ground_truth --run-ignored all`.
//!
//! Does `speech_results.bnk` load, does each place and medal name resolve, and
//! does the trigger put the right line into the mixer on the tick its screen
//! comes up, and nothing before it? The trigger is `oag_game::endrace_voice`,
//! the session's own, driven here with the screen sequence a finished race walks.

use std::path::{Path, PathBuf};

use oag_game::endrace_voice::{Screen, Voice};
use oag_sound::sfx::{Cue, MenuSfx};
use oag_tables::race_campaign::Medal;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(&format!("data/images/{name}"))
}

fn load(image: &Path, probe: Option<&str>) -> MenuSfx {
    let opened =
        oag_source::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let title = opened.title;
    let mut archives = opened.archives;
    let tick = title
        .race
        .zone_announcer
        .map_or(oag_title::SequenceTick::Unknown, |z| z.tick);
    let mut banks = *title.race.sounds;
    if let (Some(bank), Some(front)) = (probe, banks.frontend.as_mut()) {
        // A census: a bank this title's data does not name for the lines.
        front.results = Some(Box::leak(bank.to_string().into_boxed_str()));
    }
    MenuSfx::load(&mut archives, &banks, tick, false)
}

const PULSE: [&str; 3] = ["pulse-psp-usa.chd", "pulse-psp-eu.chd", "pulse-ps2-eu.chd"];

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_loads_speech_results_and_every_place_and_medal_name_resolves() {
    for name in PULSE {
        let Some(image) = image(name) else { continue };
        let sfx = load(&image, None);
        for line in sfx.report() {
            println!("{name}: {line}");
        }
        for cue in Cue::RESULTS {
            assert!(sfx.has(cue), "{name}: {} did not load", cue.name());
        }
    }
}

/// The census: what a title that does not name the bank would do if it did.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_other_titles_do_not_name_the_bank_and_load_none_of_its_cues() {
    for name in [
        "pure-psp-usa.chd",
        "pure-psp-eu.chd",
        "hdfury-ps3-eu-dec.iso",
    ] {
        let Some(image) = image(name) else { continue };
        let sfx = load(&image, None);
        for cue in Cue::RESULTS {
            assert!(!sfx.has(cue), "{name}: {} loaded unnamed", cue.name());
        }
        for bank in [r"Data\Sound\speech_results.bnk", r"Data\Sound\speech.bnk"] {
            let probed = load(&image, Some(bank));
            let found: Vec<&str> = Cue::RESULTS
                .iter()
                .filter(|&&cue| probed.has(cue))
                .map(|cue| cue.name())
                .collect();
            println!("{name}: probing {bank} finds {found:?}");
        }
    }
}

/// Ticks per second of the mix the dump is read in.
fn peak_of(pcm: &[i16], per_tick: usize, from: usize, to: usize) -> u16 {
    pcm[from * per_tick * 2..(to * per_tick * 2).min(pcm.len())]
        .iter()
        .map(|s| s.unsigned_abs())
        .max()
        .unwrap_or(0)
}

/// A finished campaign Single Race, second place, a silver medal: nothing while
/// the photo is up, the place line on the tick `Results` comes up, the medal on
/// the tick `Rewards` does, and a held `Results` says it once.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_place_and_medal_lines_sound_on_the_tick_their_screen_opens() {
    for name in PULSE {
        let Some(image) = image(name) else { continue };
        let mut sfx = load(&image, None);
        let wav = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/shots")
            .join(format!(
                "endrace-voice-{}.wav",
                name.trim_end_matches(".chd")
            ));
        std::fs::create_dir_all(wav.parent().unwrap()).unwrap();
        let mut audio = oag_sound::Audio::open(
            &oag_sound::settings::Settings::default(),
            Some(wav.clone()),
            None,
            oag_audio::MIN_BUFFER,
            false,
        );
        let mut voice = Voice::new(
            oag_race::Mode::SingleRace,
            Some(2),
            None,
            Some(Medal::Silver),
        );
        let (results_at, rewards_at, end) = (60, 300, 540);
        let mut played = Vec::new();
        for tick in 0..end {
            let screen = match tick {
                t if t < results_at => Screen::Other,
                t if t < rewards_at => Screen::Results,
                _ => Screen::Rewards,
            };
            if let Some(cue) = voice.tick(screen, false) {
                assert!(
                    sfx.play(&audio, cue),
                    "{name}: {} did not start",
                    cue.name()
                );
                played.push((tick, cue));
            }
            audio.tick();
        }
        audio.finish().expect("writing the dump");
        assert_eq!(
            played,
            [
                (results_at, Cue::ResultsSecond),
                (rewards_at, Cue::MedalSilver)
            ],
            "{name}"
        );

        let file = std::fs::read(&wav).expect("the dump");
        let rate = u32::from_le_bytes(file[24..28].try_into().unwrap()) as usize;
        let per_tick = rate / 60;
        let pcm: Vec<i16> = file[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_le_bytes(*c))
            .collect();
        assert_eq!(
            peak_of(&pcm, per_tick, 0, results_at),
            0,
            "{name}: sound before the place line"
        );
        assert!(
            peak_of(&pcm, per_tick, results_at, results_at + 6) > 100,
            "{name}: place line silent"
        );
        assert_eq!(
            peak_of(&pcm, per_tick, rewards_at - 30, rewards_at),
            0,
            "{name}: the place line is still sounding where the medal is measured"
        );
        assert!(
            peak_of(&pcm, per_tick, rewards_at, rewards_at + 6) > 100,
            "{name}: medal silent"
        );
        println!("{name}: wrote {}", wav.display());
    }
}
