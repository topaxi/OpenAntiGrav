//! Head to Head with two people in a split: each person's view draws the gap readout
//! measured against the other person, in both cuts. Disc-backed.
//!
//! The two-person rules are **chosen, not measured** (the original's own two-player Head
//! to Head is unknown); the readout itself is the one a lone person gets against an AI
//! opponent (`docs/ghidra/functions/psp-pulse-usa/head2head.md`).
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game split_head_to_head_ground_truth:: --run-ignored all
//! ```

use oag_game::hud_overlay::views_frame;
use oag_gameplay::{Controller, PlayerInputs};
use oag_hud::Readout;
use oag_raceplay as race;
use oag_render::viewport::Cut;
use oag_ui::frontend::Draw;

const VIEWPORT: (f32, f32, f32, f32) = (0.0, 0.0, 1280.0, 720.0);

/// One view's text draws: position, then the words.
fn texts(frame: &oag_hud::Frame) -> Vec<Word> {
    frame
        .hud_text
        .iter()
        .chain(&frame.small_text)
        .chain(&frame.default_text)
        .filter_map(|draw| match draw {
            Draw::Text { x, y, text, .. } => Some((*x, *y, text.clone())),
            _ => None,
        })
        .collect()
}

/// A two-person Head to Head split in `cut`, the first person flown by the autopilot and
/// the second sat still (a network seat with no input), for `ticks`.
fn two_people(cut: Cut, ticks: usize) -> Option<(race::Race, oag_hud::Assets)> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Head2Head,
        views: 2,
        opponents: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    let hud = loaded.hud;
    let mut race = race::Race::start(loaded.setup);
    oag_game::split_screen::apply(&mut race, 2, cut);
    race.sim.world.controllers[1] = Controller::Remote;
    race.set_autopilot(true);
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
    }
    Some((race, hud))
}

/// One text draw: where it sits and what it says.
type Word = (f32, f32, String);

/// Each view's words and the rectangle it was clipped to.
fn per_view(race: &race::Race, hud: &oag_hud::Assets) -> Vec<([f32; 4], Vec<Word>)> {
    let readouts: Vec<Readout> = (0..2).map(|slot| race.readout_for(slot)).collect();
    let draws = oag_game::hud_views::view_draws(
        hud.context().expect("the layout parsed"),
        hud.split.as_ref(),
        hud.split_hud.as_ref(),
        readouts,
        race.cut(),
        VIEWPORT,
        hud.space,
    );
    let clips = [
        draws[0].placement.clip,
        draws.last().unwrap().placement.clip,
    ];
    clips
        .iter()
        .map(|clip| {
            let words = draws
                .iter()
                .filter(|draw| draw.placement.clip == *clip)
                .flat_map(|draw| texts(&views_frame(std::slice::from_ref(draw))))
                .collect();
            (*clip, words)
        })
        .collect()
}

/// Both views draw the Head to Head swap - `1st`, `2nd` and a gap label, with no place
/// out of eight - each against the other person: the same distance, the opposite sign,
/// and every word inside its own view, in both cuts, with the gap short (tick 300) and
/// long enough for the second row to sit at its clamp (tick 1200). Fails if the bar's
/// widgets are placed apart from the place widgets (the split-HUD file the build before
/// this one shipped), which leaves each view on a Single Race place.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn each_view_draws_the_gap_to_the_other_person_in_both_cuts() {
    for cut in [Cut::Vertical, Cut::Horizontal] {
        for ticks in [300, 1200] {
            let Some((race, hud)) = two_people(cut, ticks) else {
                return;
            };
            let views = per_view(&race, &hud);
            let mut gaps = Vec::new();
            for (view, (clip, words)) in views.iter().enumerate() {
                let label = format!("{cut:?} tick {ticks} view {view}");
                let names: Vec<&str> = words.iter().map(|w| w.2.as_str()).collect();
                // Each row is the ordinal then the craft's team name (`1st Triakis`).
                assert!(
                    names.iter().any(|n| n.starts_with("1st ") && n.len() > 4)
                        && names.iter().any(|n| n.starts_with("2nd ") && n.len() > 4),
                    "{label}: {names:?}"
                );
                assert!(
                    !names.contains(&"Pos"),
                    "{label}: a Single Race place: {names:?}"
                );
                let gap = words
                    .iter()
                    .find(|w| w.2.ends_with('m') && (w.2.starts_with('+') || w.2.starts_with('-')))
                    .unwrap_or_else(|| panic!("{label}: no gap label in {names:?}"));
                for (x, y, text) in words {
                    assert!(
                        *x >= clip[0]
                            && *x < clip[0] + clip[2]
                            && *y >= clip[1]
                            && *y < clip[1] + clip[3],
                        "{label}: {text:?} at ({x}, {y}) is outside its view {clip:?}"
                    );
                }
                // The second row stays clear of the shield and speed bars in the lower
                // quarter of the view, even at the clamp (tick 1200): in the top and
                // bottom cut a flat view put it on them at the first scale tried.
                let second = words.iter().find(|w| w.2.starts_with("2nd")).unwrap();
                assert!(
                    (second.1 - clip[1]) / clip[3] < 0.75,
                    "{label}: the second row is on the shield and speed bars"
                );
                gaps.push(gap.2.clone());
            }
            assert_eq!(gaps[0][1..], gaps[1][1..], "one gap, seen from both seats");
            assert_ne!(
                gaps[0].as_bytes()[0],
                gaps[1].as_bytes()[0],
                "one leads, one trails"
            );
            let expected = if ticks == 300 { ['+', '-'] } else { ['-', '+'] };
            assert_eq!(
                [
                    gaps[0].chars().next().unwrap(),
                    gaps[1].chars().next().unwrap()
                ],
                expected,
                "{cut:?} tick {ticks}: who leads"
            );
        }
    }
}

/// Each window of `--window-seats 1,1` shows its own person's readout: the whole-screen HUD
/// built from that person's slot reads the gap to the other, not the first person's.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn each_window_draws_its_own_persons_gap() {
    let Some((race, hud)) = two_people(Cut::Vertical, 300) else {
        return;
    };
    let cx = hud.context().expect("the layout parsed");
    let gap = |slot: usize| {
        let frame = oag_hud::draw_list(&cx, &race.readout_for(slot));
        let words: Vec<String> = texts(&frame).into_iter().map(|word| word.2).collect();
        assert!(
            words.iter().any(|w| w.starts_with("1st"))
                && words.iter().any(|w| w.starts_with("2nd"))
        );
        words
            .into_iter()
            .find(|w| w.ends_with('m') && (w.starts_with('+') || w.starts_with('-')))
            .expect("a gap label")
    };
    let (first, second) = (gap(0), gap(1));
    assert_eq!(first[1..], second[1..], "one gap");
    assert_eq!(
        (&first[..1], &second[..1]),
        ("+", "-"),
        "person 1 trails at tick 300"
    );
}

/// Three people are refused by name, from the command line, before anything loads: the
/// mode is two craft. Needs no disc. Fails if the check is dropped (the build before this
/// one fielded a full grid for them).
#[test]
fn head_to_head_refuses_three_people_by_name() {
    for seats in [["--players", "3"], ["--window-seats", "1,2"]] {
        let out = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
            .args(["--no-audio", "--race", "--mode", "head_to_head"])
            .args(seats)
            .output()
            .expect("running oag-game");
        assert!(!out.status.success(), "{seats:?} was accepted");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("head_to_head takes at most 2 people and 3 are seated"),
            "{seats:?}: {stderr}"
        );
    }
}
