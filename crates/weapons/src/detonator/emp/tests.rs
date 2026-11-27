//! The EMP law on a straight invented lap, against `detonator-emp.md`'s tables.

use super::*;
use crate::detonator::{Entry, Partner, Placed};
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
        ..Survey::default()
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

    let mut events = Vec::new();
    for _ in 0..130 {
        events.extend(step_wave(&mut field, &survey, &config(), 1.0 / 60.0));
    }
    let kinds: Vec<EventKind> = events.iter().map(|e| e.kind).collect();
    assert!(
        !field.emp.wave.live,
        "the wave ends once its age reaches the extent"
    );
    assert_eq!(field.cannon.score, 100 + 25 + 25 + 50 + 1000);
    assert_eq!(
        events.iter().filter_map(|e| e.points).sum::<i32>(),
        25 + 25 + 50 + 1000,
        "each kill's event carries what it paid"
    );
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
fn the_wave_runs_over_its_lifetime_not_at_once() {
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

/// `EMP_UpdateWave`: one entry a tick (`speed = 1`) for `extent` seconds. At an empty gauge
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

/// A 200-entry ring with a 20-entry branch that leaves at entry 50 and rejoins at entry 90.
fn forked_lap() -> Survey {
    let mut survey = lap(200);
    survey.branches.push(crate::detonator::SurveyBranch {
        split: 50,
        merge: 90,
        entries: (0..20)
            .map(|i| Entry {
                centre: Vec3::new(100.0, 0.0, 500.0 + i as f32 * 10.0),
                ..survey.entries[0]
            })
            .collect(),
        path: 1,
        to_primary: Vec::new(),
        from_primary: Vec::new(),
    });
    lay_the_whole_split(&mut survey);
    survey
}

/// Gives every entry of the ring and of each branch a record, as a lap whose walk stored them all.
fn lay_the_whole_split(survey: &mut Survey) {
    let all = survey.entries.len()
        + survey
            .branches
            .iter()
            .map(|b| b.entries.len())
            .sum::<usize>();
    survey.lap = (0..all as u16).collect();
}

fn branch_mine(survey: &Survey, offset: usize) -> Placed {
    Placed {
        entry: u16::try_from(survey.branch_entry(0, offset).expect("a branch entry"))
            .expect("fits"),
        lane: 0,
        live: false,
    }
}

fn run_wave(field: &mut Field, survey: &Survey, ticks: usize) {
    for _ in 0..ticks {
        step_wave(field, survey, &config(), 1.0 / 60.0);
    }
}

/// The wave splits where the original's does: the front that steps onto the split entry forks,
/// and each front clears the mines on its own route.
#[test]
fn a_wave_that_crosses_a_split_clears_both_branches() {
    let survey = forked_lap();
    let mut field = Field::new();
    field.mines[0] = mine(60, false);
    field.mines[1] = branch_mine(&survey, 4);
    field.mines[2] = branch_mine(&survey, 18);
    field.mines[3] = mine(120, false);
    field.live_mines = 4;
    charge(&mut field, 8);
    fire(&mut field, &survey, &config(), 40);
    run_wave(&mut field, &survey, 15);
    assert!(field.emp.wave.forked);
    run_wave(&mut field, &survey, 110);
    assert_eq!(field.live_mines, 0, "the ring's and the branch's mines");
}

/// The branch's mines survive without the fork: the same wave on the same lap with the
/// branch dropped from the survey leaves them (the test fails if the fork is not wired).
#[test]
fn without_the_fork_the_branch_is_left_alone() {
    let survey = forked_lap();
    let ring_only = Survey {
        branches: Vec::new(),
        ..survey.clone()
    };
    let mut field = Field::new();
    field.mines[0] = mine(60, false);
    field.mines[1] = Placed {
        entry: 200 + 4,
        ..mine(0, false)
    };
    field.live_mines = 2;
    charge(&mut field, 8);
    fire(&mut field, &ring_only, &config(), 40);
    run_wave(&mut field, &ring_only, 125);
    assert!(!field.emp.wave.forked);
    assert_eq!(field.live_mines, 1, "only the ring's mine went");
}

/// A wave launched past the split, or with too short a life to reach it, never forks, and the
/// fork is born once.
#[test]
fn a_wave_that_never_steps_onto_the_split_does_not_fork() {
    let survey = forked_lap();
    for from in [51, 100, 10] {
        let mut field = Field::new();
        charge(&mut field, 4);
        fire(&mut field, &survey, &config(), from);
        run_wave(&mut field, &survey, 15);
        assert!(!field.emp.wave.forked, "from {from}");
    }
    let mut field = Field::new();
    charge(&mut field, 8);
    fire(&mut field, &survey, &config(), 50);
    run_wave(&mut field, &survey, 125);
    assert!(
        !field.emp.wave.forked,
        "a wave that starts on the split steps onto it on tick zero"
    );
}

/// The fork walks the branch one entry a tick and is released as it steps off the merge, having
/// cleared the first entry past it.
#[test]
fn the_fork_walks_its_branch_and_is_released_at_the_merge() {
    let survey = forked_lap();
    let mut field = Field::new();
    charge(&mut field, 8);
    fire(&mut field, &survey, &config(), 40);
    run_wave(&mut field, &survey, 10);
    assert_eq!(fork_entry(&field.emp.wave, &survey), None, "front at 50");
    run_wave(&mut field, &survey, 1);
    assert_eq!(
        fork_entry(&field.emp.wave, &survey),
        survey.branch_entry(0, 0)
    );
    run_wave(&mut field, &survey, 19);
    assert_eq!(
        fork_entry(&field.emp.wave, &survey),
        survey.branch_entry(0, 19)
    );
    field.mines[0] = mine(90, false);
    field.live_mines = 1;
    run_wave(&mut field, &survey, 1);
    assert!(!field.emp.wave.forked, "released off the merge");
    assert_eq!(fork_entry(&field.emp.wave, &survey), None);
    assert_eq!(field.live_mines, 0, "the entry past the merge");
    assert!(field.emp.wave.live, "the main front runs on");
}

/// A main front stepping off a merge releases the second front, and a later split forks again.
#[test]
fn the_main_fronts_merge_releases_the_fork_and_a_later_split_forks_again() {
    let mut survey = forked_lap();
    // The second front is on a long branch, still out when the main front merges at 90.
    survey.branches[0].entries = (0..80).map(|_| survey.entries[0]).collect();
    let mut second = survey.branches[0].clone();
    second.split = 95;
    survey.branches.push(second);
    lay_the_whole_split(&mut survey);
    let mut field = Field::new();
    charge(&mut field, 8);
    fire(&mut field, &survey, &config(), 40);
    run_wave(&mut field, &survey, 49);
    assert!(field.emp.wave.forked, "front at 89, the merge's last entry");
    run_wave(&mut field, &survey, 2);
    assert!(!field.emp.wave.forked, "front past the merge");
    run_wave(&mut field, &survey, 5);
    assert!(field.emp.wave.forked, "the split at 95");
    assert_eq!(field.emp.wave.fork.branch, 1);
}

/// [`forked_lap`] with the two routes within reach of each other over the first and last four
/// sections of the split, as `EMP_ForkAtJunction` finds the disc's.
fn forked_lap_with_overlap() -> Survey {
    let mut survey = forked_lap();
    let branch = &mut survey.branches[0];
    branch.to_primary = (0..20)
        .map(|k| {
            Some(Partner {
                at: 50 + k * 2,
                near: !(4..16).contains(&k),
            })
        })
        .collect();
    branch.from_primary = (0..40)
        .map(|o| {
            Some(Partner {
                at: o / 2,
                near: !(4..36).contains(&o),
            })
        })
        .collect();
    survey
}

/// A craft on the branch launches there: the front walks the branch, then the ring from the
/// merge.
#[test]
fn a_launch_on_a_branch_walks_the_branch_then_the_ring() {
    let survey = forked_lap();
    let from = u16::try_from(survey.branch_entry(0, 5).expect("a branch entry")).expect("fits");
    let mut field = Field::new();
    field.mines[0] = branch_mine(&survey, 12);
    field.mines[1] = Placed {
        entry: 93,
        lane: 0,
        live: false,
    };
    field.mines[2] = Placed {
        entry: 6,
        lane: 0,
        live: false,
    };
    field.live_mines = 3;
    charge(&mut field, 8);
    fire(&mut field, &survey, &config(), from);
    assert!(!field.emp.wave.forked);
    assert_eq!(
        front_entry(&field.emp.wave, &survey),
        Some(usize::from(from))
    );
    run_wave(&mut field, &survey, 14);
    assert_eq!(front_entry(&field.emp.wave, &survey), Some(219));
    run_wave(&mut field, &survey, 1);
    assert_eq!(front_entry(&field.emp.wave, &survey), Some(90), "the merge");
    run_wave(&mut field, &survey, 30);
    assert_eq!(field.live_mines, 1, "the ring's far entry stays");
    assert_eq!(field.mines[0].entry, 6);
}

/// `EMP_ForkAtJunction`: within reach on the near half of the other route, a second front
/// starts on it at launch, from either side.
#[test]
fn a_launch_beside_the_other_route_forks_onto_it() {
    let survey = forked_lap_with_overlap();
    let ring = launch_fork(&survey, 52).expect("ring side");
    assert_eq!((ring.on_ring, ring.at), (false, 1));
    let branch = survey.branch_entry(0, 2).expect("a branch entry");
    let on_branch = launch_fork(&survey, branch).expect("branch side");
    assert_eq!((on_branch.on_ring, on_branch.at), (true, 54));

    let mut field = Field::new();
    field.mines[0] = branch_mine(&survey, 1);
    field.mines[1] = branch_mine(&survey, 10);
    field.live_mines = 2;
    charge(&mut field, 8);
    fire(&mut field, &survey, &config(), 52);
    assert!(field.emp.wave.forked);
    assert_eq!(
        fork_entry(&field.emp.wave, &survey),
        survey.branch_entry(0, 1)
    );
    run_wave(&mut field, &survey, 1);
    assert_eq!(
        field.live_mines, 1,
        "cleared where it stands on the first tick"
    );
    run_wave(&mut field, &survey, 20);
    assert_eq!(field.live_mines, 0);
}

/// A fork born on the far half of the other route travels the other way and is never advanced:
/// it walks nothing until the main front's merge releases it, and the next split forks as usual.
#[test]
fn a_launch_beside_the_far_half_walks_nothing_until_the_merge() {
    let mut survey = forked_lap_with_overlap();
    let mut second = survey.branches[0].clone();
    second.split = 95;
    second.to_primary = Vec::new();
    second.from_primary = Vec::new();
    survey.branches.push(second);
    lay_the_whole_split(&mut survey);
    assert_eq!(launch_fork(&survey, 88), Some(Fork::INERT));

    let mut field = Field::new();
    field.mines[0] = Placed {
        entry: u16::try_from(survey.branch_entry(1, 3).expect("an entry")).expect("fits"),
        lane: 0,
        live: false,
    };
    field.live_mines = 1;
    charge(&mut field, 8);
    fire(&mut field, &survey, &config(), 88);
    run_wave(&mut field, &survey, 2);
    assert_eq!(field.emp.wave.fork, Fork::INERT);
    assert_eq!(fork_entry(&field.emp.wave, &survey), None);
    assert_eq!(field.live_mines, 1);
    run_wave(&mut field, &survey, 1);
    assert!(!field.emp.wave.forked, "released at the merge");
    run_wave(&mut field, &survey, 30);
    assert_eq!(field.live_mines, 0, "the split at 95 forks");
}

/// A launch away from the split is as it was: no fork at launch, and nothing out of reach.
#[test]
fn a_launch_away_from_the_other_route_does_not_fork_at_once() {
    let survey = forked_lap_with_overlap();
    for from in [10, 60, 150] {
        assert_eq!(launch_fork(&survey, from), None, "from {from}");
    }
    let mid = survey.branch_entry(0, 9).expect("a branch entry");
    assert_eq!(launch_fork(&survey, mid), None);
}

/// A front on a point with no record (its own entry unusable) destroys the lanes of the record
/// at the other route's nearest point, whether or not the routes are within the fork's reach.
#[test]
fn a_point_with_no_record_takes_the_other_routes_record() {
    let mut survey = forked_lap_with_overlap();
    let hole = survey.branch_entry(0, 10).expect("a branch entry");
    survey.lap.retain(|&i| usize::from(i) != hole);
    assert!(!survey.has_record(hole));
    assert_eq!(survey.sibling(hole), Some(70));

    let mut field = Field::new();
    field.mines[0] = mine(70, false);
    field.live_mines = 1;
    charge(&mut field, 8);
    let from = u16::try_from(survey.branch_entry(0, 5).expect("an entry")).expect("fits");
    fire(&mut field, &survey, &config(), from);
    assert!(!field.emp.wave.forked, "branch 5 is out of reach");
    run_wave(&mut field, &survey, 6);
    assert_eq!(field.live_mines, 0, "taken from branch offset 10");
}

/// With its own record, a point never reaches across.
#[test]
fn a_point_with_a_record_leaves_the_other_route_alone() {
    let survey = forked_lap_with_overlap();
    let mut field = Field::new();
    field.mines[0] = mine(70, false);
    field.live_mines = 1;
    charge(&mut field, 8);
    let from = u16::try_from(survey.branch_entry(0, 5).expect("an entry")).expect("fits");
    fire(&mut field, &survey, &config(), from);
    run_wave(&mut field, &survey, 14);
    assert_eq!(field.live_mines, 1);
}
