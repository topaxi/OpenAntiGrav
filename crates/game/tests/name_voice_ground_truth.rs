//! Pulse's team and circuit name voices, off real discs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content. Run with
//! `OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game name_voice_ground_truth:: --run-ignored all`.
//!
//! `FRONT_END_VO.bnk`'s `TMTX_NAME` and `TRTX_NAME` are guarded by a runtime
//! global the selection screens write (`menu-sounds.md`, "The name voices").
//! These tests drop the law and fail: the guard reading the wrong slot, the
//! child chosen by position instead of by value, a team's pair collapsing to
//! one grain, the `soundregister` read off the wrong attribute.

use std::path::{Path, PathBuf};

use oag_formats::sblk::Bank;
use oag_formats::sblk::timeline::{GLOBAL_SLOTS, WalkModel};
use oag_sound::sfx::MenuSfx;

const BANK: &str = r"Data\Sound\FRONT_END_VO.bnk";
const PULSE: [&str; 3] = ["pulse-psp-usa.chd", "pulse-psp-eu.chd", "pulse-ps2-eu.chd"];

/// `soundregister` of the eight teams a fresh profile lists, read off the game
/// running on PPSSPP (a breakpoint on `Sound_PlayNamedInSlot`, `t1 = 2`,
/// `t2` = value, on each team's Confirm) and found again in `Definition.xml`.
const TEAMS: [(&str, u8); 8] = [
    ("Feisar", 1),
    ("AG_Systems", 3),
    ("Assegai", 4),
    ("Qirex", 5),
    ("Piranha", 7),
    ("Triakis", 8),
    ("EGX", 9),
    ("Goteki", 13),
];

/// The three circuits a fresh profile lists, measured the same way (`t1 = 1`).
const CIRCUITS: [(&str, u8); 3] = [("16_Track", 1), ("03_Track", 3), ("18_Track", 5)];

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(&format!("data/images/{name}"))
}

fn open(image: &Path) -> oag_source::title::Opened {
    oag_source::title::open_source(&image.display().to_string(), Vec::new(), Vec::new())
        .expect("opening the source")
}

fn load(image: &Path) -> MenuSfx {
    let mut opened = open(image);
    let tick = opened
        .title
        .race
        .zone_announcer
        .map_or(oag_title::SequenceTick::Unknown, |z| z.tick);
    let banks = *opened.title.race.sounds;
    MenuSfx::load(&mut opened.archives, &banks, tick, false)
}

fn model(slot: usize, value: u8) -> WalkModel {
    let mut globals = [0_i8; GLOBAL_SLOTS];
    globals[slot - 1] = value as i8;
    WalkModel {
        globals: Some(globals),
        ..WalkModel::default()
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_guard_selects_the_circuit_cue_by_value_and_the_team_pair_by_value() {
    for name in PULSE {
        let Some(image) = image(name) else { continue };
        let mut opened = open(&image);
        let blob = opened.archives.read_name(BANK).expect("the bank reads");
        let bank = Bank::parse(&blob).expect("parses");
        let names = bank.sound_names();
        let circuits = bank.cue_named("TRTX_NAME").expect("TRTX_NAME");
        // Circuit value v plays the bank's (v + 14)th cue: TALONS_WHITE for 1,
        // TALONS_BLACK for 2, .. ORCUS_BLACK for 32, in the bank's own order.
        for value in 1..=32_u8 {
            let walked = bank.cue_timeline_modelled(&circuits, &[], model(1, value));
            assert!(walked.is_complete(), "{name}: circuit {value}");
            let child = bank.cue(14 + u16::from(value)).expect("child cue");
            let direct = bank.cue_timeline(&child);
            let offsets = |t: &oag_formats::sblk::timeline::Timeline| {
                t.grains.iter().map(|g| g.sound.offset).collect::<Vec<_>>()
            };
            assert!(!walked.grains.is_empty(), "{name}: circuit {value} silent");
            assert_eq!(
                offsets(&walked),
                offsets(&direct),
                "{name}: circuit {value}"
            );
            // The three the original was watched speaking: the child is the
            // circuit the screen showed.
            let label = &names.iter().find(|n| n.cue == child.index).unwrap().name;
            let expected = match value {
                1 => Some("TALONS_WHIT"),
                3 => Some("MOA_WHITE"),
                5 => Some("METROPIA_WHITE"),
                _ => None,
            };
            if let Some(expected) = expected {
                assert_eq!(label, expected, "{name}: circuit {value}");
            }
        }
        // Slot 2 is the team's: a circuit value there plays nothing.
        assert!(
            bank.cue_timeline_modelled(&circuits, &[], model(2, 1))
                .grains
                .is_empty()
        );

        let teams = bank.cue_named("TMTX_NAME").expect("TMTX_NAME");
        let mut firsts = Vec::new();
        for value in (1..=13_u8).filter(|&v| v != 10) {
            let walked = bank.cue_timeline_modelled(&teams, &[], model(2, value));
            assert!(walked.is_complete(), "{name}: team {value}");
            let ticks: Vec<u32> = walked.grains.iter().map(|g| g.tick).collect();
            assert_eq!(
                ticks,
                [0, 10],
                "{name}: team {value} is a pair ten ticks apart"
            );
            firsts.push(walked.grains[0].sound.offset);
        }
        // Eleven teams, eleven different names - the guard did not collapse.
        let mut unique = firsts.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), firsts.len(), "{name}: {firsts:?}");
        // The original's own 10 selects no command.
        assert!(
            bank.cue_timeline_modelled(&teams, &[], model(2, 10))
                .grains
                .is_empty()
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_definition_authors_the_registers_the_original_wrote() {
    for name in PULSE {
        let Some(image) = image(name) else { continue };
        let mut opened = open(&image);
        let blob = opened
            .archives
            .read_name(r"Data\Plugins\PI001\Definition.xml")
            .expect("definition");
        let xml = oag_tables::fexml::text(&blob).expect("the definition is not shortened");
        let teams = oag_raceplay::catalogue::teams(&xml);
        for (id, register) in TEAMS {
            let team = teams
                .iter()
                .find(|t| t.id == id)
                .unwrap_or_else(|| panic!("{name}: {id}"));
            assert_eq!(team.sound_register, Some(register), "{name}: {id}");
        }
        let tracks = oag_raceplay::catalogue::tracks(&xml);
        for (id, register) in CIRCUITS {
            let track = tracks
                .iter()
                .find(|t| t.id == id)
                .unwrap_or_else(|| panic!("{name}: {id}"));
            assert_eq!(track.sound_register, Some(register), "{name}: {id}");
        }
        // Every circuit has its own voice among the bank's 32: the PSP's 24
        // are registers 1..=24 in order; the PS2 disc lists 32, each once.
        let registers: Vec<_> = tracks.iter().filter_map(|t| t.sound_register).collect();
        if name.contains("psp") {
            assert_eq!(registers, (1..=24).collect::<Vec<u8>>(), "{name}");
        } else {
            let mut sorted = registers.clone();
            sorted.sort_unstable();
            assert_eq!(sorted, (1..=32).collect::<Vec<u8>>(), "{name}");
        }
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_bank_loads_and_names_voice_every_team_and_circuit_but_the_missing_ten() {
    for name in PULSE {
        let Some(image) = image(name) else { continue };
        let sfx = load(&image);
        for line in sfx.report() {
            println!("{name}: {line}");
        }
        for value in (1..=13).filter(|&v| v != 10) {
            assert!(sfx.has_team_name(value), "{name}: team {value}");
        }
        assert!(!sfx.has_team_name(10), "{name}: the original's own gap");
        for value in 1..=32 {
            assert!(sfx.has_circuit_name(value), "{name}: circuit {value}");
        }
        assert!(!sfx.has_circuit_name(0) && !sfx.has_circuit_name(33));
    }
}

fn peak(pcm: &[i16], per_tick: usize, from: usize, to: usize) -> u16 {
    pcm[from * per_tick * 2..(to * per_tick * 2).min(pcm.len())]
        .iter()
        .map(|s| s.unsigned_abs())
        .max()
        .unwrap_or(0)
}

/// Two circuits then two teams confirmed on selection screens: silence before
/// each, energy after it, and a different waveform for a different value.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_confirm_speaks_the_name_and_nothing_sounds_before_it() {
    for name in PULSE {
        let Some(image) = image(name) else { continue };
        let mut sfx = load(&image);
        let wav = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/shots")
            .join(format!("name-voice-{}.wav", name.trim_end_matches(".chd")));
        std::fs::create_dir_all(wav.parent().unwrap()).unwrap();
        let mut audio = oag_sound::Audio::open(
            &oag_sound::settings::Settings::default(),
            Some(wav.clone()),
            None,
            oag_audio::MIN_BUFFER,
            false,
        );
        // (tick, is_team, register): Talon's Junction then Metropia, then
        // Assegai then Triakis, as the screens confirm them.
        let script = [
            (60, false, 1_u8),
            (240, false, 5),
            (420, true, 4),
            (600, true, 8),
        ];
        let end = 780;
        for tick in 0..end {
            for &(at, team, register) in &script {
                if tick == at {
                    let started = if team {
                        sfx.play_team_name(&audio, register)
                    } else {
                        sfx.play_circuit_name(&audio, register)
                    };
                    assert!(started, "{name}: {register} at {tick}");
                }
            }
            audio.tick();
        }
        audio.finish().expect("writing the dump");
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
            peak(&pcm, per_tick, 0, 60),
            0,
            "{name}: sound before the first confirm"
        );
        for &(at, _, register) in &script {
            assert!(
                peak(&pcm, per_tick, at + 2, at + 12) > 100,
                "{name}: {register} silent at {at}"
            );
        }
        println!("{name}: wrote {}", wav.display());
    }
}

/// A title that names no voice bank loads none.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_other_titles_load_no_name_voices() {
    for name in ["pure-psp-usa.chd", "hdfury-ps3-eu-dec.iso"] {
        let Some(image) = image(name) else { continue };
        let sfx = load(&image);
        assert!(!sfx.has_team_name(4) && !sfx.has_circuit_name(1), "{name}");
    }
}
