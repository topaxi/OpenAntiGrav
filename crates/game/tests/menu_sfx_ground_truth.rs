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

fn load(image: &Path, fury: bool) -> (MenuSfx, &'static oag_title::Title) {
    let opened =
        oag_source::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let title = opened.title;
    let mut archives = opened.archives;
    let tick = title
        .race
        .zone_announcer
        .map_or(oag_title::SequenceTick::Unknown, |z| z.tick);
    (
        MenuSfx::load(&mut archives, title.race.sounds, tick, fury),
        title,
    )
}

/// The waveforms a role loaded, found by the name the title's data gives it.
fn waveforms(sfx: &MenuSfx, title: &oag_title::Title, cue: Cue, fury: bool) -> Option<usize> {
    let front = title.race.sounds.frontend?;
    let name = cue.front_end_name(&front.cues, fury)?;
    let prefix = format!("sfx: {name} -> ");
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
/// The order is `Cue::FRONT_END`'s: four moves, two steps, accept, decline.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn each_title_with_a_frontend_bank_resolves_its_menu_cues() {
    for (name, expected) in [
        ("pulse-psp-usa.chd", [3, 3, 3, 3, 1, 1, 1, 3]),
        ("pulse-psp-eu.chd", [3, 3, 3, 3, 1, 1, 1, 3]),
        ("pulse-ps2-eu.chd", [3, 3, 3, 3, 1, 1, 1, 3]),
        ("pure-psp-usa.chd", [1; 8]),
        ("pure-psp-eu.chd", [1; 8]),
    ] {
        let Some(image) = image(name) else { continue };
        let (sfx, title) = load(&image, false);
        let found: Vec<usize> = Cue::FRONT_END
            .iter()
            .map(|&cue| waveforms(&sfx, title, cue, false).unwrap_or(0))
            .collect();
        for line in sfx.report() {
            println!("{name}: {line}");
        }
        assert_eq!(found, expected, "{name}: {:?}", sfx.report());
    }
}

/// The timeline a role loaded, as the report words it: variants of voices.
fn timeline(sfx: &MenuSfx, title: &oag_title::Title, cue: Cue, fury: bool) -> Option<String> {
    let front = title.race.sounds.frontend?;
    let name = cue.front_end_name(&front.cues, fury)?;
    let prefix = format!("sfx: {name} -> ");
    sfx.report().iter().find_map(|line| {
        let rest = line.strip_prefix(&prefix)?;
        Some(rest.split_once("plays its timeline: ")?.1.to_string())
    })
}

/// HD's `frontend.bnk` carries a name table of 27 cues (the reviewer's claim
/// that it has none is wrong). Its menu cues are timelines, not flat sets:
/// each `nav*` keys a stereo whoosh and one of seven stereo ticks (seven
/// variants of six voices), `accept` a stereo pair, `reject` two groups of
/// seven stereo ticks (49 variants of six voices), `reject_fury` three stereo
/// hits. The count is in the `0x19` operand's high byte on HD
/// (`WalkModel::hd_alternates`); read as Pulse's it leaves them unread and the
/// loader falls back to a flat pick of all 18 waveforms.
/// Both styles are loaded, because the style is a boot-time choice.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_resolves_a_cue_per_direction_and_a_style_of_accept_and_reject() {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let nav = "7 variant(s) of 6 voice(s)";
    for (fury, accept, decline) in [
        (
            false,
            "1 variant(s) of 2 voice(s)",
            "49 variant(s) of 6 voice(s)",
        ),
        (
            true,
            "1 variant(s) of 2 voice(s)",
            "1 variant(s) of 3 voice(s)",
        ),
    ] {
        let (sfx, title) = load(&image, fury);
        let found: Vec<Option<String>> = Cue::FRONT_END
            .iter()
            .map(|&cue| timeline(&sfx, title, cue, fury))
            .collect();
        for line in sfx.report() {
            println!("fury={fury}: {line}");
        }
        let want: Vec<Option<String>> = [nav, nav, nav, nav, nav, nav, accept, decline]
            .iter()
            .map(|s| Some(s.to_string()))
            .collect();
        assert_eq!(found, want, "fury={fury}: {:?}", sfx.report());
    }
}

/// A title that names no front-end bank plays nothing, and says so.
#[test]
fn titles_without_a_frontend_bank_stay_silent() {
    assert!(oag_2048::race::SOUND_BANKS.frontend.is_none());
    assert!(oag_omega::race::SOUND_BANKS.frontend.is_none());
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
        cue: Cue::MenuDown,
    },
    Step {
        tick: 150,
        button: Button::Cross,
        cue: Cue::MenuAccept,
    },
    Step {
        tick: 270,
        button: Button::Right,
        cue: Cue::MenuStepRight,
    },
    Step {
        tick: 390,
        button: Button::Down,
        cue: Cue::MenuDown,
    },
    Step {
        tick: 510,
        button: Button::Circle,
        cue: Cue::MenuDecline,
    },
];

/// Walks the shipped menu tree, writes the mix to
/// a `menu-walk-<disc>.wav` under `data/shots/`, and reads it back.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_walk_of_the_menus_sounds_each_cue_on_the_tick_of_its_press() {
    for (name, fury) in [
        ("pulse-psp-usa.chd", false),
        ("pure-psp-usa.chd", false),
        ("hdfury-ps3-eu-dec.iso", false),
        ("hdfury-ps3-eu-dec.iso", true),
    ] {
        let Some(image) = image(name) else { continue };
        let (mut sfx, _) = load(&image, fury);
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
            .join(format!(
                "menu-walk-{}{}.wav",
                name.trim_end_matches(".chd").trim_end_matches(".iso"),
                if fury { "-fury" } else { "" }
            ));
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
                let cue = oag_game::sound::menu_cue(nav, None);
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
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_le_bytes(*c))
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

/// Every front-end cue frees its voices. HD's cues key six or more voices a
/// press against a pool of 32, so a looping grain would starve the menus after
/// a handful of presses; twenty presses of each role must all start, and the
/// mixer must be empty once the longest tail has played out.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pressing_every_menu_cue_twenty_times_leaves_no_voice_behind() {
    for (name, fury) in [
        ("pulse-psp-usa.chd", false),
        ("hdfury-ps3-eu-dec.iso", false),
        ("hdfury-ps3-eu-dec.iso", true),
    ] {
        let Some(image) = image(name) else { continue };
        let (mut sfx, _) = load(&image, fury);
        let dump = std::env::temp_dir().join(format!("menu-leak-{}.wav", std::process::id()));
        let mut audio = oag_sound::Audio::open(
            &oag_sound::settings::Settings::default(),
            Some(dump.clone()),
            None,
            oag_audio::MIN_BUFFER,
            false,
        );
        let mut tick = 0;
        for round in 0..20 {
            for cue in Cue::FRONT_END {
                assert!(
                    sfx.play(&audio, cue),
                    "{name} fury={fury}: {cue:?} round {round}"
                );
                for _ in 0..30 {
                    audio.tick();
                    tick += 1;
                }
            }
        }
        for _ in 0..600 {
            audio.tick();
            tick += 1;
        }
        let left = audio.output().with_mixer(|mixer| mixer.active_voices());
        let _ = std::fs::remove_file(&dump);
        assert_eq!(
            left, 0,
            "{name} fury={fury}: voices still held after {tick} ticks"
        );
    }
}
