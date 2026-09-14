//! [`Options::laps_override`]/[`Options::eliminator_kill_target`], the two
//! fields a campaign launch is the only real caller of - see
//! `crate::main::session::campaign::Session::launch_campaign_cell`. Neither
//! reaches a disc here: these are the same synthetic `setup` fixture every
//! other file in this directory shares.

use super::*;

/// A campaign `Time Trial` cell's own `laps` wins over
/// `Mode::TIME_TRIAL_LAPS_BY_CLASS`, which the unset default already proves
/// disagrees with `3` at Venom... except it does not here, so the assertion
/// below picks a value the table could never produce on its own (`5`, which
/// is Phantom's row, not Venom's) - a test that used `3` would pass whether
/// or not the override actually did anything.
#[test]
fn laps_override_wins_over_the_per_class_table_for_time_trial() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::TimeTrial;
    setup.class = "VENOM".to_string();
    setup.laps_override = Some(5);
    let race = Race::start(setup);
    assert_eq!(race.sim.world.race.laps_target, Some(5));
}

/// The same override, for [`Mode::SingleRace`] - the other mode
/// [`Mode::laps_target`] already answers `Some` for.
#[test]
fn laps_override_wins_over_the_per_class_table_for_single_race() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.class = "VENOM".to_string();
    setup.laps_override = Some(7);
    let race = Race::start(setup);
    assert_eq!(race.sim.world.race.laps_target, Some(7));
}

/// `None` changes nothing - the ordinary Custom Race path, still reading
/// straight off `Mode::TIME_TRIAL_LAPS_BY_CLASS`.
#[test]
fn no_override_leaves_the_per_class_table_in_charge() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::TimeTrial;
    setup.class = "VENOM".to_string();
    setup.laps_override = None;
    let race = Race::start(setup);
    assert_eq!(
        race.sim.world.race.laps_target,
        Some(Mode::TIME_TRIAL_LAPS_BY_CLASS[SpeedClass::Venom as usize])
    );
}

/// **Never applied to `Speed Lap` or `Zone`** - both author a `laps`
/// attribute on their own campaign cells (`7` and `0`) that is display
/// convention, not an ending; `Options::laps_override`'s own doc names this
/// as the one thing a caller must get right. This test is about
/// `Race::start` refusing to end a race those two modes never end on their
/// own even if a caller passed an override in error - `laps_target` must
/// stay `None` regardless, because `Mode::laps_target` already returns
/// `None` for both and nothing in `Race::start` conditions the override on
/// the mode.
///
/// Written as a guard on the *caller's* contract rather than on
/// `Race::start` itself: `Session::launch_campaign_cell` is what must never
/// construct this input, and the campaign-launch tests
/// (`crates/game/tests/campaign_medal.rs`) are where that half is proven.
/// What this half proves is narrower and still worth stating: if a caller
/// ever did pass one for Speed Lap, the race would end early, which is
/// exactly the bug `Options::laps_override`'s own doc warns against - so a
/// `Some` here still takes effect, on purpose, rather than being silently
/// ignored for the wrong mode. Silent ignoring would hide the caller's own
/// mistake instead of surfacing it as a race that visibly ends too soon.
#[test]
fn an_override_would_still_apply_to_speed_lap_which_is_exactly_why_the_caller_must_not_pass_one() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SpeedLap;
    setup.laps_override = Some(2);
    let race = Race::start(setup);
    assert_eq!(
        race.sim.world.race.laps_target,
        Some(2),
        "Race::start applies whatever it is given - the guard lives at the caller"
    );
}

/// The campaign's own Elimination kill target (the cell's gold figure)
/// reaches [`RaceSim::eliminator_kill_target`], overriding
/// [`Mode::ELIMINATOR_KILL_TARGET_DEFAULT`].
#[test]
fn eliminator_kill_target_overrides_the_default() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::Eliminator;
    setup.eliminator_kill_target = Some(7);
    let race = Race::start(setup);
    assert_eq!(race.sim.eliminator_kill_target, 7);
}

/// `None` falls back to the default, unchanged - every non-campaign
/// Eliminator launch today.
#[test]
fn no_eliminator_kill_target_falls_back_to_the_default() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::Eliminator;
    setup.eliminator_kill_target = None;
    let race = Race::start(setup);
    assert_eq!(
        race.sim.eliminator_kill_target,
        Mode::ELIMINATOR_KILL_TARGET_DEFAULT
    );
}
