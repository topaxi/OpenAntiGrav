use super::*;

const DT: f32 = 1.0 / 60.0;

#[test]
fn the_ports_tick_takes_exactly_one_step() {
    assert_eq!(steps(DT), 1);
    assert_eq!(steps(0.0), 0);
    assert_eq!(steps(2.0 * DT), 2);
}

#[test]
fn an_untouched_shell_is_hidden() {
    let mut shell = AbsorbShell::default();
    assert_eq!(shell.fader(), None);
    shell.advance(DT);
    assert_eq!(shell.fader(), None);
}

#[test]
fn the_fade_climbs_by_a_tenth_of_the_gap_per_tick() {
    let mut shell = AbsorbShell::default();
    shell.stamp();
    shell.advance(DT);
    assert_eq!(shell.fader(), Some(0.1));
    for _ in 1..10 {
        shell.advance(DT);
    }
    let after_ten = shell.fader().unwrap();
    assert!(
        (after_ten - (1.0 - 0.9f32.powi(10))).abs() < 1e-5,
        "{after_ten}"
    );
}

#[test]
fn the_shell_fades_out_after_the_second_and_hides_below_a_hundredth() {
    let mut shell = AbsorbShell::default();
    shell.stamp();
    let mut shown = 0;
    let mut peak = 0.0f32;
    for _ in 0..600 {
        shell.advance(DT);
        match shell.fader() {
            Some(fade) => {
                shown += 1;
                peak = peak.max(fade);
            }
            None => break,
        }
    }
    // Sixty ticks of timer, then 0.9^k falls to 0.01 after about 44 more.
    assert!((100..=108).contains(&shown), "{shown}");
    assert!(peak > 0.99, "{peak}");
    assert_eq!(shell.fader(), None);
}

#[test]
fn a_second_absorb_restarts_the_timer_and_keeps_the_fade() {
    let mut shell = AbsorbShell::default();
    shell.stamp();
    for _ in 0..30 {
        shell.advance(DT);
    }
    let before = shell.fader().unwrap();
    shell.stamp();
    shell.advance(DT);
    assert!(shell.fader().unwrap() > before);
    for _ in 0..59 {
        shell.advance(DT);
    }
    assert!(shell.fader().unwrap() > 0.99);
}
