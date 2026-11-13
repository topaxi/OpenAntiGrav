use super::*;

const PULSE: Demo = Demo::PULSE;

fn idle_for(attract: &mut Attract, ticks: u32) -> Option<u32> {
    (1..=ticks).find(|_| attract.tick(PULSE.idle_ticks(), true, false))
}

#[test]
fn an_idle_main_menu_launches_a_demo_after_the_titles_threshold() {
    let mut attract = Attract::default();
    assert_eq!(
        idle_for(&mut attract, PULSE.idle_ticks()),
        Some(PULSE.idle_ticks())
    );
}

#[test]
fn a_press_before_the_threshold_restarts_the_count() {
    let mut attract = Attract::default();
    assert_eq!(idle_for(&mut attract, PULSE.idle_ticks() - 1), None);
    assert!(
        !attract.tick(PULSE.idle_ticks(), true, true),
        "the press itself launches nothing"
    );
    assert_eq!(attract.idle_ticks(), 0);
    assert_eq!(
        idle_for(&mut attract, PULSE.idle_ticks() - 1),
        None,
        "the clock started over"
    );
    assert!(attract.tick(PULSE.idle_ticks(), true, false));
}

#[test]
fn a_menu_with_something_over_it_never_counts() {
    let mut attract = Attract::default();
    for _ in 0..PULSE.idle_ticks() * 2 {
        assert!(!attract.tick(PULSE.idle_ticks(), false, false));
    }
    assert_eq!(attract.idle_ticks(), 0);
}

#[test]
fn input_ends_a_demo_on_its_first_tick_even_when_heard_during_the_load() {
    let mut run = Run::new(false);
    run.hear(true);
    assert_eq!(
        run.tick(PULSE.duration_ticks(), false, true),
        Some(Leave::Input)
    );
    assert_eq!(run.ticks(), 0);
}

#[test]
fn a_demo_runs_its_length_and_then_is_over() {
    let mut run = Run::new(false);
    for _ in 1..PULSE.duration_ticks() {
        assert_eq!(run.tick(PULSE.duration_ticks(), false, true), None);
    }
    assert_eq!(
        run.tick(PULSE.duration_ticks(), false, true),
        Some(Leave::Over)
    );
}

#[test]
fn a_race_that_finishes_first_ends_the_demo() {
    let mut run = Run::new(false);
    assert_eq!(run.tick(PULSE.duration_ticks(), false, true), None);
    assert_eq!(
        run.tick(PULSE.duration_ticks(), true, true),
        Some(Leave::Over)
    );
}

#[test]
fn a_wallpaper_hears_no_input() {
    let mut run = Run::new(true);
    run.hear(true);
    assert_eq!(run.tick(PULSE.duration_ticks(), false, true), None);
    assert!(run.wallpaper());
}

#[test]
fn a_demo_that_ran_out_goes_where_the_title_says_and_a_cli_run_always_chains() {
    assert_eq!(
        after_over(&PULSE, false),
        Next::Demo,
        "Pulse goes on to a new circuit"
    );
    assert_eq!(after_over(&PULSE, true), Next::Demo);
    let to_menu = Demo {
        after_end: oag_title::pre_race::Sourced::chosen(AfterEnd::MainMenu),
        ..PULSE
    };
    assert_eq!(after_over(&to_menu, false), Next::Menu);
    assert_eq!(after_over(&to_menu, true), Next::Demo);
}

#[test]
fn the_pick_is_a_function_of_the_seed_and_stays_in_range() {
    for seed in 0..200 {
        let a = pick(&PULSE, seed, 7, 5, None).expect("something to pick");
        assert_eq!(Some(a), pick(&PULSE, seed, 7, 5, None));
        assert!(a.track < 7 && a.team < 5);
    }
    let tracks: std::collections::BTreeSet<_> = (0..200)
        .filter_map(|s| pick(&PULSE, s, 7, 5, None))
        .map(|p| p.track)
        .collect();
    assert!(tracks.len() > 3, "the seed varies the circuit: {tracks:?}");
}

#[test]
fn a_chain_does_not_repeat_the_circuit_it_just_flew() {
    let mut last = pick(&PULSE, chain_seed(9, 0), 4, 3, None);
    for n in 1..200 {
        let next = pick(&PULSE, chain_seed(9, n), 4, 3, last);
        assert_ne!(next.map(|p| p.track), last.map(|p| p.track), "demo {n}");
        last = next;
    }
}

#[test]
fn nothing_to_pick_from_is_none() {
    assert_eq!(pick(&PULSE, 1, 0, 3, None), None);
    assert_eq!(pick(&PULSE, 1, 3, 0, None), None);
}

fn authored(start: Option<f32>, end: Option<f32>, tracks: &[&str]) -> DemoConfig {
    DemoConfig {
        start_seconds: start,
        end_seconds: end,
        tracks: tracks.iter().map(ToString::to_string).collect(),
    }
}

#[test]
fn the_authored_timeouts_replace_the_titles_and_a_missing_one_does_not() {
    assert_eq!(
        Timing::of(&PULSE, None),
        Timing {
            idle: 3600,
            length: 7200
        }
    );
    let both = authored(Some(30.0), Some(120.0), &[]);
    assert_eq!(
        Timing::of(&PULSE, Some(&both)),
        Timing {
            idle: 1800,
            length: 7200
        }
    );
    let start_only = authored(Some(10.0), None, &[]);
    assert_eq!(
        Timing::of(&PULSE, Some(&start_only)),
        Timing {
            idle: 600,
            length: 7200
        }
    );
    let nonsense = authored(Some(-1.0), Some(f32::NAN), &[]);
    assert_eq!(
        Timing::of(&PULSE, Some(&nonsense)),
        Timing::of(&PULSE, None)
    );
}

#[test]
fn the_authored_cycle_names_circuits_by_id_or_environment_directory() {
    let circuits = [
        ("a", "Data\\Environments\\Alpha"),
        ("b", "data/environments/Tower"),
        ("c", "x"),
    ];
    let config = authored(None, None, &["tower", "C", "missing"]);
    assert_eq!(authored_circuit(Some(&config), &circuits, 0), Some(1));
    assert_eq!(authored_circuit(Some(&config), &circuits, 1), Some(2));
    assert_eq!(
        authored_circuit(Some(&config), &circuits, 2),
        Some(1),
        "wraps over the two on offer"
    );
    assert_eq!(
        authored_circuit(Some(&authored(None, None, &["gone"])), &circuits, 0),
        None
    );
    assert_eq!(authored_circuit(None, &circuits, 0), None);
}

#[test]
fn the_measured_rule_ignores_unlocks_and_draws_multiplayer_teams_and_never_repeats_either() {
    assert!(unlocks_ignored(&PULSE));
    let teams = [("a", true), ("b", false), ("c", true), ("d", true)];
    assert_eq!(offered_teams(&PULSE, &teams), ["a", "c", "d"]);
    let none_marked = [("a", false), ("b", false)];
    assert_eq!(
        offered_teams(&PULSE, &none_marked),
        ["a", "b"],
        "a title that marks none offers all"
    );
    let mut last = None;
    for n in 0..300 {
        let next = pick(&PULSE, chain_seed(3, n), 24, 8, last).expect("a pick");
        if let Some(previous) = last {
            assert_ne!(next.track, previous.track, "demo {n}");
            assert_ne!(next.team, previous.team, "demo {n}");
        }
        last = Some(next);
    }
}

#[test]
fn a_title_with_the_chosen_rule_offers_every_team_and_respects_unlocks() {
    let chosen = Demo {
        selection: oag_title::pre_race::Sourced::chosen(Selection::SeededFromPlayable),
        ..PULSE
    };
    assert!(!unlocks_ignored(&chosen));
    assert_eq!(
        offered_teams(&chosen, &[("a", true), ("b", false)]),
        ["a", "b"]
    );
}

#[test]
fn a_race_held_still_does_not_run_down_the_demos_length() {
    let mut run = Run::new(false);
    for _ in 0..10 {
        assert_eq!(run.tick(5, false, false), None);
    }
    assert_eq!(run.ticks(), 0);
    for _ in 0..4 {
        assert_eq!(run.tick(5, false, true), None);
    }
    assert_eq!(run.tick(5, false, true), Some(Leave::Over));
}

#[test]
fn the_titles_rules_become_the_races_options() {
    let mut options = oag_raceplay::Options {
        class: "Venom".to_string(),
        weapons_override: Some(false),
        eliminator_kill_target: Some(3),
        ..oag_raceplay::Options::default()
    };
    apply_rules(&mut options, &oag_pulse::TITLE);
    assert_eq!(options.class, "Flash");
    assert_eq!(options.weapons_override, Some(true));
    assert_eq!(options.eliminator_kill_target, None);
    assert_eq!(
        options.laps_override,
        Some(UNENDING_LAPS),
        "a demo that does not end on laps is given more than it can fly"
    );
}

#[test]
fn a_title_whose_demo_ends_on_laps_is_set_to_its_laps() {
    let mut title = oag_pulse::TITLE.clone();
    title.demo.rules.ends_on_laps = oag_title::pre_race::Sourced::chosen(true);
    let mut options = oag_raceplay::Options::default();
    apply_rules(&mut options, &title);
    assert_eq!(options.laps_override, Some(3));
}

#[test]
fn a_class_the_titles_ladder_does_not_author_leaves_the_players() {
    let mut title = oag_pure::TITLE.clone();
    title.demo.rules.class = oag_title::pre_race::Sourced::chosen("Warp");
    let mut options = oag_raceplay::Options {
        class: "Venom".to_string(),
        ..oag_raceplay::Options::default()
    };
    apply_rules(&mut options, &title);
    assert_eq!(options.class, "Venom");
}
