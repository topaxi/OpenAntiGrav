//! A Zone craft's end on a title that never wrecks it: the timeline of
//! `docs/ghidra/functions/ps3-hdfury-eu/zone-destroy.md` (rows Z3 to Z11), run on the real tick.

use super::*;
use oag_gameplay::cue::Cue;
use oag_livery::ZoneEndModels;
use oag_title::destroyed::DestroyedCraft;
use std::sync::Arc;

/// Two pieces of three vertices each, and a shell with nothing to draw: the loader's own shape
/// without a disc.
fn models() -> Arc<ZoneEndModels> {
    let vertex = |x: f32| oag_mesh::mesh::GpuVertex {
        position: [x, 0.0, 0.0],
        normal: [0.0, 1.0, 0.0],
        ..bytemuck::Zeroable::zeroed()
    };
    let debris = oag_mesh::mesh::Model::from_draws(
        "debris",
        (0..6).map(|v| vertex(v as f32)).collect(),
        (0..6).collect(),
        Vec::new(),
        [0.0; 3],
        1.0,
        2,
    );
    Arc::new(ZoneEndModels {
        shell: oag_mesh::mesh::Model::none("shell"),
        debris,
        pieces: vec![vec![0, 1, 2], vec![3, 4, 5]],
    })
}

fn zone_race(destroyed: DestroyedCraft) -> Race {
    let mut setup = setup(hulled_handling());
    setup.destroyed = destroyed;
    setup.mode = Mode::Zone;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    let mut view = crate::tests::fixture_view();
    view.zone_end = Some(models());
    Race::start(setup, view, crate::extensions::Prepared::default())
}

/// What a tick showed.
#[derive(Debug, Default)]
struct Seen {
    /// Ticks since the blast was first seen, at the tick each thing first held.
    shell_first: Option<u32>,
    hidden_first: Option<u32>,
    thrown_first: Option<u32>,
    finished_first: Option<u32>,
    hud_off_first: Option<u32>,
    /// The shell's time at 0, 1.25 and 2.5 s.
    shell_at: [Option<f32>; 3],
    cues: Vec<Cue>,
    blasts: u32,
    /// The tick the race ended on, from the destroying.
    ended_at: Option<u32>,
}

/// Destroys the player and runs `ticks` ticks past the blast.
fn run(race: &mut Race, ticks: u32) -> Seen {
    let mut seen = Seen::default();
    race.core.sim.force_destroy(0);
    let mut since: Option<u32> = None;
    for tick in 0..ticks {
        if race.core.sim.finished() {
            race.tick_finished();
        } else {
            race.tick(&oag_gameplay::PlayerInputs::none());
        }
        seen.cues.extend(
            race.core
                .sim
                .drain_cues()
                .into_iter()
                .filter(|event| event.cue == Cue::ShipDestroy)
                .map(|event| event.cue),
        );
        if race.view.zone_end_age().is_some() && since.is_none() {
            since = Some(0);
        }
        if race.core.sim.finished() && seen.ended_at.is_none() {
            seen.ended_at = Some(tick);
        }
        let Some(n) = since.as_mut() else { continue };
        let n = {
            let now = *n;
            *n += 1;
            now
        };
        let first = |slot: &mut Option<u32>, holds: bool| {
            if holds && slot.is_none() {
                *slot = Some(n);
            }
        };
        first(
            &mut seen.shell_first,
            race.view.zone_end_draws_shell(&race.core),
        );
        first(
            &mut seen.hidden_first,
            race.view.zone_end_hides_craft(&race.core, 0),
        );
        first(&mut seen.thrown_first, race.view.zone_end_thrown());
        first(&mut seen.finished_first, race.core.sim.finished());
        first(&mut seen.hud_off_first, !race.hud_shown());
        for (slot, tick) in seen.shell_at.iter_mut().zip([0, 75, 149]) {
            if n == tick {
                *slot = race.view.zone_end_shell_time(&race.core);
            }
        }
        seen.blasts += race.wreck_fx_started_for_tests();
        assert!(
            !race.view.contact.shake.active(),
            "a Zone end throws no shake (row Z3), tick {n}"
        );
    }
    seen
}

/// Z7, Z8, Z10, Z9 and Z3: the shell animates for 2.5 s, then the craft hides and the pieces are
/// thrown once, the HUD is gone from the blast and the results open at 5.5 s, with no blast.
#[test]
fn a_zone_craft_ends_in_its_shell_then_its_debris_and_the_results_open_at_5_5_s() {
    let mut race = zone_race(DestroyedCraft::HD);
    assert!(race.hud_shown(), "the HUD is up until the blast");
    let seen = run(&mut race, 420);
    assert_eq!(seen.shell_first, Some(0), "the shell starts at the blast");
    assert_eq!(seen.hud_off_first, Some(0), "the HUD hides at the blast");
    let hidden = seen.hidden_first.expect("the craft hides");
    assert!((149..=151).contains(&hidden), "2.5 s: {hidden}");
    assert_eq!(
        seen.thrown_first,
        Some(hidden),
        "the debris is thrown as it hides"
    );
    assert_eq!(race.view.zone_end_throws_for_tests(), 1, "thrown once");
    assert!(
        !race.view.zone_end_draws_shell(&race.core),
        "the shell hides with the craft"
    );
    let results = seen.finished_first.expect("the race ends");
    assert!(
        (328..=332).contains(&results),
        "5.5 s to the results: {results}"
    );
    let [at_0, at_half, at_end] = seen.shell_at.map(|t| t.expect("the shell was playing"));
    assert!(at_0.abs() < 1e-6, "time 0 at the blast: {at_0}");
    assert!(
        (at_half - 0.5).abs() < 0.02,
        "time 0.5 at 1.25 s: {at_half}"
    );
    assert!((at_end - 1.0).abs() < 0.02, "time 1 at 2.5 s: {at_end}");
    assert_eq!(seen.blasts, 0, "no wreck effect is thrown");
    assert_eq!(race.wreck_explosion_at_for_tests(), None, "no big blast");
}

/// Z5: `shipdestroy` plays once on the blast where the title resolves it.
#[test]
fn the_shipdestroy_cue_plays_once_on_the_blast_where_the_title_names_it() {
    let seen = run(&mut zone_race(DestroyedCraft::HD), 420);
    assert_eq!(seen.cues, [Cue::ShipDestroy]);
    let vita = run(&mut zone_race(DestroyedCraft::HD_VITA), 420);
    assert_eq!(vita.cues, [Cue::ShipDestroy], "2048's own bank resolves it");
    let silent = run(&mut zone_race(DestroyedCraft::HD_BLOWUP), 420);
    assert_eq!(silent.cues, [], "Omega: the Wwise event is not resolved");
    assert_eq!(
        silent.finished_first, seen.finished_first,
        "the timeline is the same"
    );
}

/// A title that wrecks its Zone craft (Pulse, Pure) is untouched: no shell, no hide, and the
/// race ends as the wreck does.
#[test]
fn a_title_that_wrecks_its_zone_craft_plays_none_of_it() {
    let mut race = zone_race(DestroyedCraft::PULSE);
    let seen = run(&mut race, 200);
    assert_eq!(seen.shell_first, None);
    assert_eq!(seen.hidden_first, None);
    assert_eq!(seen.thrown_first, None);
    assert!(
        seen.ended_at.is_some_and(|tick| tick < 40),
        "the wreck ends the race at once"
    );
}
