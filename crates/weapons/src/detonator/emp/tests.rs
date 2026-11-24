//! The EMP law on a straight invented lap, against `detonator-emp.md`'s tables.

use super::*;
use crate::detonator::{Entry, Placed};
use oag_core::math::Vec3;

/// The disc's `<Weapon type="EMP">` and `<Emp>` values.
fn config() -> Config {
    Config {
        lo: 4,
        hi: 8,
        min_distance: 0.25,
        max_distance: 2.0,
        speed: 1,
        lane_width: 3.5,
        scoring: Scoring {
            emp_new_mine_score: 50,
            emp_old_mine_score: 25,
            emp_bomb_score: 1000,
            ..Scoring::default()
        },
    }
}

fn lap(entries: usize) -> Survey {
    Survey {
        entries: (0..entries)
            .map(|i| Entry {
                usable: true,
                lanes: 5,
                blocked: 0,
                centre: Vec3::new(0.0, 0.0, i as f32 * 10.0),
                lateral: Vec3::X,
                up: Vec3::Y,
            })
            .collect(),
    }
}

fn charge(field: &mut Field, times: u32) -> Vec<EventKind> {
    let mut kinds = Vec::new();
    for _ in 0..times {
        field.emp.pending = true;
        kinds.extend(step_charge(field, &config()).into_iter().map(|e| e.kind));
    }
    kinds
}

/// The live table (`detonator-emp.md`, "Live"): `0..4` read `0.0`, `5` `0.25`, `6` `0.5`,
/// `7` `0.75`, `8..39` `1.0`.
#[test]
fn the_gauge_follows_the_live_table_from_zero_to_thirty_nine_charges() {
    for n in 0..40 {
        let expected = match n {
            0..=4 => 0.0,
            5 => 0.25,
            6 => 0.5,
            7 => 0.75,
            _ => 1.0,
        };
        assert_eq!(gauge(n, 4, 8), expected, "n = {n}");
    }
}

#[test]
fn the_gauge_is_the_literal_law_not_the_linear_one() {
    // `hi - lo = 3`, `lo = 5`: at `n = 5` the literal law gives `(5 - 3) / 3`, the linear one 0.
    assert_eq!(gauge(5, 5, 8), 2.0 / 3.0);
    assert_eq!(gauge(4, 5, 8), 0.0);
}

#[test]
fn each_charge_counts_once_and_the_cues_sound_on_their_edges() {
    let mut field = Field::new();
    assert!(charge(&mut field, 3).is_empty());
    assert_eq!(field.emp.charge, 3);
    assert_eq!(charge(&mut field, 1), [EventKind::EmpReady]);
    assert_eq!(field.emp.gauge, 0.0);
    assert!(charge(&mut field, 3).is_empty());
    assert_eq!(charge(&mut field, 1), [EventKind::EmpFull]);
    assert_eq!((field.emp.charge, field.emp.gauge), (8, 1.0));
    assert!(
        charge(&mut field, 5).is_empty(),
        "both latched until under lo"
    );
    // No pending flag, no charge.
    assert!(step_charge(&mut field, &config()).is_empty());
    assert_eq!(field.emp.charge, 13);
}

#[test]
fn firing_below_the_threshold_does_nothing() {
    let survey = lap(100);
    let mut field = Field::new();
    charge(&mut field, 3);
    assert!(fire(&mut field, &survey, &config(), 0).is_empty());
    assert_eq!(field.emp.charge, 3);
    assert!(!field.emp.wave.live);
}

#[test]
fn firing_resets_the_charge_and_the_latches_rearm_below_lo() {
    let survey = lap(100);
    let mut field = Field::new();
    charge(&mut field, 8);
    let fired = fire(&mut field, &survey, &config(), 10);
    assert_eq!(fired.len(), 1);
    assert_eq!(fired[0].kind, EventKind::EmpLaunch);
    assert_eq!((field.emp.charge, field.emp.gauge), (0, 0.0));
    assert!(field.emp.wave.live);
    assert_eq!(field.emp.wave.lifetime, 2.0, "extent at a full gauge");
    assert_eq!(field.cannon.clip, 0, "the cannon is untouched");
    assert!(charge(&mut field, 3).is_empty());
    assert_eq!(charge(&mut field, 1), [EventKind::EmpReady], "re-armed");
}

#[test]
fn the_extent_follows_the_gauge() {
    let survey = lap(100);
    let mut field = Field::new();
    charge(&mut field, 4);
    fire(&mut field, &survey, &config(), 0);
    assert_eq!(field.emp.wave.lifetime, 0.25, "extent at an empty gauge");
}

fn mine(entry: u16, live: bool) -> Placed {
    Placed {
        entry,
        lane: 0,
        live,
    }
}

/// A wave from entry 10 clears every mine and bomb ahead within its reach, scoring a dark mine
/// `25`, a live one `50` and a bomb `1000`, counting the mines; one behind it or past its reach
/// stays.
#[test]
fn the_wave_clears_what_lies_ahead_and_scores_the_emp_values() {
    let survey = lap(200);
    let mut field = Field::new();
    for (index, (entry, live)) in [
        (12, false),
        (20, false),
        (30, true),
        (5, false),
        (150, false),
    ]
    .into_iter()
    .enumerate()
    {
        field.mines[index] = mine(entry, live);
    }
    field.live_mines = 5;
    field.bombs[0] = mine(40, true);
    field.live_bombs = 1;
    field.cannon.score = 100;
    field.cannon.destroyed_mines = 3;
    charge(&mut field, 8);
    fire(&mut field, &survey, &config(), 10);

    let mut kinds = Vec::new();
    for _ in 0..130 {
        kinds.extend(
            step_wave(&mut field, &survey, &config(), 1.0 / 60.0)
                .into_iter()
                .map(|e| e.kind),
        );
    }
    assert!(
        !field.emp.wave.live,
        "the wave ends once its age reaches the extent"
    );
    assert_eq!(field.cannon.score, 100 + 25 + 25 + 50 + 1000);
    assert_eq!(
        field.cannon.destroyed_mines,
        3 + 3,
        "mines count, the bomb does not"
    );
    let left: Vec<u16> = field.live_mines().iter().map(|m| m.entry).collect();
    assert_eq!(left.len(), 2);
    assert!(left.contains(&5) && left.contains(&150), "{left:?}");
    assert_eq!(field.live_bombs, 0);
    assert_eq!(
        field.cannon.destroyed_bombs, 1,
        "the wave's bomb kill counts through the shared destroy path, as the cannon's does"
    );
    assert_eq!(
        kinds.iter().filter(|k| **k == EventKind::BombEmped).count(),
        1
    );
}

#[test]
fn the_wave_runs_over_wave_time_not_at_once() {
    let survey = lap(200);
    let mut field = Field::new();
    field.mines[0] = mine(80, false);
    field.live_mines = 1;
    charge(&mut field, 8);
    fire(&mut field, &survey, &config(), 0);
    step_wave(&mut field, &survey, &config(), 1.0 / 60.0);
    assert_eq!(
        field.live_mines, 1,
        "the front has not reached entry 80 yet"
    );
}

/// `EMP_Update`: one entry a tick (`speed = 1`) for `extent` seconds. At an empty gauge
/// (`0.25` s, fifteen ticks) the front clears the fifteen entries from where it was fired and
/// no further; at a full gauge (`2.0` s) it runs about 120.
#[test]
fn the_wave_steps_one_entry_a_tick_for_extent_seconds() {
    let run = |charges: u32, mines: &[u16]| {
        let survey = lap(400);
        let mut field = Field::new();
        for (index, &entry) in mines.iter().enumerate() {
            field.mines[index] = mine(entry, false);
        }
        field.live_mines = mines.len() as u16;
        charge(&mut field, charges);
        fire(&mut field, &survey, &config(), 10);
        let mut ticks = 0;
        while field.emp.wave.live {
            let before = field.emp.wave.front;
            step_wave(&mut field, &survey, &config(), 1.0 / 60.0);
            assert_eq!(field.emp.wave.front, before + 1, "one entry a tick");
            ticks += 1;
        }
        let mut left: Vec<u16> = field.live_mines().iter().map(|m| m.entry).collect();
        left.sort_unstable();
        (ticks, left)
    };
    let (ticks, left) = run(4, &[24, 25, 26]);
    assert_eq!(ticks, 15, "0.25 s at 60 Hz");
    assert_eq!(left, [25, 26], "entries 10 to 24 cleared, 25 not reached");
    let (ticks, left) = run(8, &[128, 140]);
    assert!((120..=121).contains(&ticks), "2.0 s at 60 Hz: {ticks}");
    assert_eq!(left, [140]);
}

#[test]
fn the_ramp_climbs_over_one_point_three_seconds_from_a_dead_tenth() {
    let mut ramp = 0.0;
    let first = step_ramp(&mut ramp, 1.0 / 60.0);
    assert_eq!(first, 0.0, "the first 0.13 s give nothing");
    let mut last = first;
    for _ in 0..90 {
        last = step_ramp(&mut ramp, 1.0 / 60.0);
    }
    assert_eq!(ramp, 1.0);
    assert!(last >= 1.0);
    assert_eq!(step_ramp(&mut ramp, 1.0 / 60.0), 1.0);
}
