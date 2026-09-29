//! The Zone announcer's cues, against the banks on the discs that ship them.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test zone_announcer_ground_truth --run-ignored all
//! ```
//!
//! # What this is for
//!
//! The maintainer heard Pulse's milestone lines cut: "clear" alone at zone 5,
//! "zone" alone or a bare number elsewhere. The cue is not a set of takes. It is
//! a timeline of three words, and this pins the shape it was found in so a
//! regression reads as a changed grain list rather than as a voice that sounds
//! slightly wrong.

use oag_formats::sblk::timeline::{Timeline, tick_seconds};
use oag_formats::sblk::{Bank, Sound};
use oag_game::audio::sfx::Announcer;

/// The milestone numbers Pulse's own ladder names.
const PULSE_MILESTONES: [u16; 13] = [5, 10, 15, 20, 25, 30, 40, 50, 60, 70, 80, 90, 100];

fn open(image: &str) -> Option<oag_game::title::Opened> {
    let path = oag_testdata::image(image)?;
    Some(
        oag_game::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("the image opens"),
    )
}

fn bank_of(opened: &mut oag_game::title::Opened) -> Vec<u8> {
    let table = opened.title.race.zone_announcer.expect("a ladder");
    opened.archives.read_name(table.bank).expect("bank reads")
}

/// The waveform offset each grain keys, in tick order.
fn shape(timeline: &Timeline) -> Vec<(u32, u32, i32)> {
    timeline
        .grains
        .iter()
        .map(|g| (g.tick, g.sound.offset, g.angle))
        .collect()
}

fn offset_of(bank: &Bank, cue: &str) -> u32 {
    let cue = bank.cue_named(cue).expect("cue");
    let sounds: Vec<Sound> = bank.cue_sounds(&cue);
    assert_eq!(sounds.len(), 2, "a left and a right key-on");
    assert_eq!(
        sounds[0].offset, sounds[1].offset,
        "the same waveform twice, not two takes"
    );
    sounds[0].offset
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_s_milestones_are_zone_then_a_number_then_clear() {
    for image in ["pulse-psp-usa.chd", "pulse-psp-eu.chd"] {
        let Some(mut opened) = open(image) else {
            continue;
        };
        let blob = bank_of(&mut opened);
        let bank = Bank::parse(&blob).expect("bank parses");
        let zone = offset_of(&bank, "ZONE");
        let clear = offset_of(&bank, "CLEAR");

        for milestone in PULSE_MILESTONES {
            let name = format!("zone_{milestone}");
            let cue = bank.cue_named(&name).expect("the milestone's cue");
            let timeline = bank.cue_timeline(&cue);
            assert!(timeline.is_complete(), "{image} {name}: {timeline:?}");

            let got = shape(&timeline);
            assert_eq!(got.len(), 6, "{image} {name}");
            let number = got[2].1;
            assert_ne!(number, zone);
            assert_ne!(number, clear);
            // ZONE first, on both sides, ten ticks apart; the number 90-105
            // ticks after; CLEAR at least 200 ticks after that, on both sides
            // again. The exact delays differ per number and are authored.
            assert_eq!(
                (got[0], got[1]),
                ((0, zone, 30), (10, zone, 330)),
                "{image} {name}"
            );
            assert!((95..=105).contains(&got[2].0), "{image} {name}: {got:?}");
            assert_eq!((got[2].1, got[2].2), (number, 30));
            assert_eq!((got[3].0 - got[2].0, got[3].1, got[3].2), (10, number, 330));
            assert_eq!((got[4].1, got[4].2), (clear, 30));
            assert_eq!((got[5].0 - got[4].0, got[5].1, got[5].2), (10, clear, 330));
            assert!(got[4].0 - got[3].0 >= 140, "{image} {name}: {got:?}");
        }
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_announcer_plays_the_whole_line_as_one_voice() {
    for image in ["pulse-psp-usa.chd", "pulse-psp-eu.chd"] {
        let Some(mut opened) = open(image) else {
            continue;
        };
        let mut announcer = Announcer::load(&mut opened.archives, opened.title.race.zone_announcer);
        for milestone in PULSE_MILESTONES {
            let name = format!("zone_{milestone}");
            assert!(
                announcer
                    .report
                    .iter()
                    .any(|l| l.starts_with(&format!("announcer: {name} -> sequence of 6 grain(s)"))),
                "{image} {name}: {:?}",
                announcer.report
            );
            let mut rng = oag_core::Rng::new(u64::from(milestone));
            // Every draw is the same line: there is nothing left to choose.
            let (first, looping) = announcer.pick(milestone, &mut rng).expect("a line");
            assert!(!looping);
            for _ in 0..8 {
                let (again, _) = announcer.pick(milestone, &mut rng).expect("a line");
                assert!(std::sync::Arc::ptr_eq(&first, &again));
            }
            // A stereo line longer than any one of its words: ZONE, the number
            // and CLEAR are 0.47 s, 0.5-0.9 s and 0.56 s and the line is ZONE
            // to CLEAR's end, not the longest word.
            assert_eq!(first.channels(), 2);
            assert!(
                (1.5..2.1).contains(&first.seconds()),
                "{image} {name}: {} s",
                first.seconds()
            );
        }
        announcer.report.clear();
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_tick_is_only_claimed_for_the_bank_it_was_measured_on() {
    let Some(mut opened) = open("pulse-psp-usa.chd") else {
        return;
    };
    let blob = bank_of(&mut opened);
    let bank = Bank::parse(&blob).expect("parse");
    assert!(tick_seconds(bank.order).is_some());
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_s_lines_are_single_grains_and_keep_the_flat_pick() {
    for image in ["pure-psp-usa.chd", "pure-psp-eu.chd"] {
        let Some(mut opened) = open(image) else {
            continue;
        };
        let blob = bank_of(&mut opened);
        let bank = Bank::parse(&blob).expect("parse");
        for milestone in [5, 10, 15, 20, 25, 30, 40, 50, 75, 100] {
            let cue = bank
                .cue_named(&format!("zone_{milestone}"))
                .expect("a Pure milestone");
            let timeline = bank.cue_timeline(&cue);
            assert_eq!(timeline.grains.len(), 1, "{image} zone_{milestone}");
        }
        let announcer = Announcer::load(&mut opened.archives, opened.title.race.zone_announcer);
        assert!(
            announcer.report.iter().all(|l| !l.contains("sequence of")),
            "Pure's one-word lines must not be recomposed: {:?}",
            announcer.report
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_s_milestones_are_left_on_the_flat_pick_until_its_tick_is_measured() {
    let Some(mut opened) = open("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let blob = bank_of(&mut opened);
    let bank = Bank::parse(&blob).expect("parse");
    assert_eq!(tick_seconds(bank.order), None);
    // HD's `zone_5` is the number twice and a child, `c_CLEAR`, whose own two
    // grains are a goto and a marker: no waveform to lay down, and opcodes this
    // walk reports rather than skips.
    let cue = bank.cue_named("zone_5").expect("zone_5");
    let timeline = bank.cue_timeline(&cue);
    assert_eq!(timeline.grains.len(), 2);
    assert_eq!(timeline.unread, vec![0x24, 0x23]);
    let announcer = Announcer::load(&mut opened.archives, opened.title.race.zone_announcer);
    assert!(
        announcer.report.iter().all(|l| !l.contains("sequence of")),
        "{:?}",
        announcer.report
    );
}
