//! The front end's navigation sounds, off real discs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content. Run with
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test menu_sfx_ground_truth --run-ignored all`.
//!
//! Two questions only a disc can answer. Does each title's `frontend.bnk`
//! resolve to the four cues the menus raise, with the waveform counts the bank
//! authors? And does a scripted walk of the shipped menu tree put each cue into
//! the mixer on the tick the press happened, and nothing on the ticks between?
//! The second drives the same `Menu`, `take_nav`, `menu_cue` and `MenuSfx::play`
//! the session does, and renders through the dump backend.

use std::path::{Path, PathBuf};

use oag_gameplay::input::{Button, Input};
use oag_sound::sfx::{Cue, MenuSfx};
use oag_ui::menu::{BUILT_IN, Definition, Menu};

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(&format!("data/images/{name}"))
}

fn load(image: &Path) -> (MenuSfx, &'static oag_title::Title) {
    let opened =
        oag_source::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let title = opened.title;
    let mut archives = opened.archives;
    let tick = title
        .race
        .zone_announcer
        .map_or(oag_title::SequenceTick::Unknown, |z| z.tick);
    (MenuSfx::load(&mut archives, title.race.sounds, tick), title)
}

fn waveforms(sfx: &MenuSfx, cue: Cue) -> Option<usize> {
    let prefix = format!("sfx: {} -> ", cue.name());
    sfx.report().iter().find_map(|line| {
        line.strip_prefix(&prefix)?
            .split_whitespace()
            .next()?
            .parse()
            .ok()
    })
}

/// Pulse's `frontend.bnk` binds three waveforms to `UPDOWN` and `DECLINE` and
/// one to the others; Pure's binds one each. `oag-wad sounds ... --bank FRNTEND`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn each_title_with_a_frontend_bank_resolves_the_four_menu_cues() {
    for (name, expected) in [
        ("pulse-psp-usa.chd", [3, 1, 1, 3]),
        ("pulse-psp-eu.chd", [3, 1, 1, 3]),
        ("pulse-ps2-eu.chd", [3, 1, 1, 3]),
        ("pure-psp-usa.chd", [1, 1, 1, 1]),
        ("pure-psp-eu.chd", [1, 1, 1, 1]),
    ] {
        let Some(image) = image(name) else { continue };
        let (sfx, _) = load(&image);
        let found: Vec<usize> = Cue::FRONT_END
            .iter()
            .map(|&cue| waveforms(&sfx, cue).unwrap_or(0))
            .collect();
        for line in sfx.report() {
            println!("{name}: {line}");
        }
        assert_eq!(found, expected, "{name}: {:?}", sfx.report());
    }
}

/// A title that names no front-end bank plays nothing, and says so.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn titles_without_a_frontend_bank_stay_silent() {
    for name in ["hdfury-ps3-eu-dec.iso"] {
        let Some(image) = image(name) else { continue };
        let (sfx, title) = load(&image);
        assert!(title.race.sounds.frontend.is_none(), "{name}");
        for cue in Cue::FRONT_END {
            assert!(!sfx.has(cue), "{name}: {} must not load", cue.name());
        }
        assert!(
            sfx.report()
                .iter()
                .any(|l| l.contains("no front-end sound bank")),
            "{name}: {:?}",
            sfx.report()
        );
    }
}

struct Step {
    tick: usize,
    button: Button,
    cue: Cue,
}

const WALK: [Step; 5] = [
    Step {
        tick: 30,
        button: Button::Down,
        cue: Cue::MenuUpDown,
    },
    Step {
        tick: 150,
        button: Button::Cross,
        cue: Cue::MenuAccept,
    },
    Step {
        tick: 270,
        button: Button::Right,
        cue: Cue::MenuLeftRight,
    },
    Step {
        tick: 390,
        button: Button::Down,
        cue: Cue::MenuUpDown,
    },
    Step {
        tick: 510,
        button: Button::Circle,
        cue: Cue::MenuDecline,
    },
];

/// Walks the shipped menu tree, writes the mix to
/// `data/shots/menu-walk-<disc>.wav`, and reads it back.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_walk_of_the_menus_sounds_each_cue_on_the_tick_of_its_press() {
    for name in ["pulse-psp-usa.chd", "pure-psp-usa.chd"] {
        let Some(image) = image(name) else { continue };
        let (mut sfx, _) = load(&image);
        let strings = oag_ui::strings::project_table(None);
        let mut menu = Menu::new(Definition::parse(BUILT_IN, &strings).expect("menu.toml"));
        // The MODE row's values come from the disc at run time; two stand-ins
        // are enough for it to step.
        menu.supply(
            oag_ui::menu::ValueSource::RaceModes,
            &[
                oag_ui::menu::Choice::plain("single_race"),
                oag_ui::menu::Choice::plain("time_trial"),
            ],
        );
        let wav = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/shots")
            .join(format!("menu-walk-{}.wav", name.trim_end_matches(".chd")));
        std::fs::create_dir_all(wav.parent().unwrap()).unwrap();
        let mut audio = oag_sound::Audio::open(
            &oag_sound::settings::Settings::default(),
            Some(wav.clone()),
            None,
            oag_audio::MIN_BUFFER,
            false,
        );
        let mut input = Input::new();
        let mut played = Vec::new();
        for tick in 0..WALK.last().unwrap().tick + 180 {
            let pressed = WALK
                .iter()
                .find(|s| s.tick == tick)
                .map_or(0, |s| s.button.bit());
            input.begin_frame(pressed);
            menu.update(&mut input);
            for nav in menu.take_nav() {
                let cue = oag_game::sound::menu_cue(nav);
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

        let expected: Vec<(usize, Cue)> = WALK.iter().map(|s| (s.tick, s.cue)).collect();
        assert_eq!(played, expected, "{name}");
        assert_eq!(menu.page().id, "main", "{name}: Circle left the race page");

        let file = std::fs::read(&wav).expect("the dump");
        let rate = u32::from_le_bytes(file[24..28].try_into().unwrap()) as usize;
        let per_tick = rate / 60;
        let pcm: Vec<i16> = file[44..]
            .chunks_exact(2)
            .map(|c| i16::from_le_bytes([c[0], c[1]]))
            .collect();
        let peak = |from: usize, to: usize| {
            pcm[from * per_tick * 2..(to * per_tick * 2).min(pcm.len())]
                .iter()
                .map(|s| s.unsigned_abs())
                .max()
                .unwrap_or(0)
        };
        assert_eq!(
            peak(0, WALK[0].tick),
            0,
            "{name}: sound before the first press"
        );
        for step in &WALK {
            assert!(
                peak(step.tick, step.tick + 6) > 100,
                "{name}: {} is silent at tick {}",
                step.cue.name(),
                step.tick
            );
        }
        println!(
            "{name}: wrote {} ({} frames at {rate} Hz)",
            wav.display(),
            pcm.len() / 2
        );
    }
}
