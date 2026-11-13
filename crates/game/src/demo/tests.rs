use super::*;

const PULSE: Demo = Demo::PULSE;

fn idle_for(attract: &mut Attract, ticks: u32) -> Option<u32> {
    (1..=ticks).find(|_| attract.tick(&PULSE, true, false))
}

#[test]
fn an_idle_main_menu_launches_a_demo_after_the_titles_threshold() {
    let mut attract = Attract::default();
    assert_eq!(idle_for(&mut attract, PULSE.idle_ticks()), Some(PULSE.idle_ticks()));
}

#[test]
fn a_press_before_the_threshold_restarts_the_count() {
    let mut attract = Attract::default();
    assert_eq!(idle_for(&mut attract, PULSE.idle_ticks() - 1), None);
    assert!(!attract.tick(&PULSE, true, true), "the press itself launches nothing");
    assert_eq!(attract.idle_ticks(), 0);
    assert_eq!(idle_for(&mut attract, PULSE.idle_ticks() - 1), None, "the clock started over");
    assert!(attract.tick(&PULSE, true, false));
}

#[test]
fn a_menu_with_something_over_it_never_counts() {
    let mut attract = Attract::default();
    for _ in 0..PULSE.idle_ticks() * 2 {
        assert!(!attract.tick(&PULSE, false, false));
    }
    assert_eq!(attract.idle_ticks(), 0);
}

#[test]
fn input_ends_a_demo_on_its_first_tick_even_when_heard_during_the_load() {
    let mut run = Run::new(false);
    run.hear(true);
    assert_eq!(run.tick(&PULSE, false), Some(Leave::Input));
    assert_eq!(run.ticks(), 0);
}

#[test]
fn a_demo_runs_its_length_and_then_is_over() {
    let mut run = Run::new(false);
    for _ in 1..PULSE.duration_ticks() {
        assert_eq!(run.tick(&PULSE, false), None);
    }
    assert_eq!(run.tick(&PULSE, false), Some(Leave::Over));
}

#[test]
fn a_race_that_finishes_first_ends_the_demo() {
    let mut run = Run::new(false);
    assert_eq!(run.tick(&PULSE, false), None);
    assert_eq!(run.tick(&PULSE, true), Some(Leave::Over));
}

#[test]
fn a_wallpaper_hears_no_input() {
    let mut run = Run::new(true);
    run.hear(true);
    assert_eq!(run.tick(&PULSE, false), None);
    assert!(run.wallpaper());
}

#[test]
fn a_demo_that_ran_out_goes_where_the_title_says_and_a_cli_run_always_chains() {
    assert_eq!(after_over(&PULSE, false), Next::Menu);
    assert_eq!(after_over(&PULSE, true), Next::Demo);
    let chained = Demo {
        after_end: oag_title::pre_race::Sourced::chosen(AfterEnd::NextDemo),
        ..PULSE
    };
    assert_eq!(after_over(&chained, false), Next::Demo);
}

#[test]
fn the_pick_is_a_function_of_the_seed_and_stays_in_range() {
    for seed in 0..200 {
        let a = pick(&PULSE, seed, 7, 5, None).expect("something to pick");
        assert_eq!(Some(a), pick(&PULSE, seed, 7, 5, None));
        assert!(a.track < 7 && a.team < 5);
    }
    let tracks: std::collections::BTreeSet<_> =
        (0..200).filter_map(|s| pick(&PULSE, s, 7, 5, None)).map(|p| p.track).collect();
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
