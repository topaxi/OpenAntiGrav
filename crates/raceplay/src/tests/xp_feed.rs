//! The race feeds its XP tally from the edges the simulation reports.

use super::*;
use crate::xp_tally::XpTally;
use oag_gameplay::PlayerInputs;
use oag_tables::xp::{Awards, Column};

const XML: &str = r#"<XPAwards>
<Award index="6" name="kill" zone="50" elim="50" zombie="50" race="50" speed="50"/>
<Award index="9" name="zone" zone="10" elim="10" zombie="10" race="10" speed="10"/>
<Award index="10" name="pzone" zone="30" elim="30" zombie="30" race="30" speed="30"/>
<Award index="48" name="hz10" zone="10" elim="0" zombie="0" race="0" speed="0"/>
</XPAwards>"#;

fn armed_zone_race() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::Zone;
    let mut race = Race::start(setup);
    race.arm_xp(XpTally::new(
        Awards::parse(XML).expect("fixture"),
        Column::Zone,
    ));
    race
}

fn run_past_the_countdown(race: &mut Race, ticks: u64) {
    for _ in 0..oag_race::COUNTDOWN_TICKS + ticks {
        race.tick(&PlayerInputs::none());
    }
}

#[test]
fn each_zone_step_pays_perfect_zone_until_a_wall_is_touched() {
    let mut race = armed_zone_race();
    run_past_the_countdown(&mut race, 1300);
    let zones = u32::from(race.sim.world.primary_race().zone);
    assert!(
        zones >= 2,
        "sanity: two zone steps in 1300 ticks, got {zones}"
    );
    let tally = race.xp_tally().expect("armed");
    assert_eq!(tally.times_paid(10), zones);
    assert_eq!(tally.times_paid(9), 0);
    let perfect = Awards::parse(XML).expect("fixture").xp(10, Column::Zone);
    assert!(tally.total() >= zones * perfect);
}

#[test]
fn a_zone_with_a_wall_contact_pays_the_plain_zone_award() {
    let mut race = armed_zone_race();
    run_past_the_countdown(&mut race, 100);
    race.sim.world.race[0].zone_dirty = true;
    run_past_the_countdown(&mut race, 600);
    let tally = race.xp_tally().expect("armed");
    assert_eq!(tally.times_paid(9), 1, "the dirty zone");
}

#[test]
fn a_kill_pays_once_and_only_after_the_countdown() {
    let mut race = armed_zone_race();
    race.sim.world.ships[0].standing.kills = 1;
    run_past_the_countdown(&mut race, 5);
    assert_eq!(race.xp_tally().expect("armed").times_paid(6), 1);
}

#[test]
fn an_unarmed_race_gathers_nothing() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::Zone;
    let mut race = Race::start(setup);
    run_past_the_countdown(&mut race, 700);
    assert!(race.xp_tally().is_none());
}

fn progress(awards: Option<Awards>) -> crate::Campaign2048Progress {
    crate::Campaign2048Progress {
        name: "2048 - Event 1".to_owned(),
        objectives: None,
        ai: None,
        xp_awards: awards,
        xp_column: Column::Zone,
    }
}

#[test]
fn an_event_with_a_table_arms_the_tally_in_its_column_and_one_without_does_not() {
    let mut race = Race::start(setup(hulled_handling()));
    race.arm_event_xp(&progress(None));
    assert!(race.xp_tally().is_none());
    race.arm_event_xp(&progress(Awards::parse(XML)));
    assert!(race.xp_tally().is_some());
}

/// The HUD's `RaceXP` reads the tally: a kill raises the readout by the parsed award's value, and
/// an unarmed race reads `None`. Fails if `Race::readout` drops the tally.
#[test]
fn a_kill_raises_the_readouts_xp_by_the_authored_amount() {
    let mut race = armed_zone_race();
    run_past_the_countdown(&mut race, 2);
    assert_eq!(race.readout().race_xp, Some(0));
    race.sim.world.ships[0].standing.kills = 1;
    run_past_the_countdown(&mut race, 2);
    let kill = Awards::parse(XML).expect("fixture").xp(6, Column::Zone);
    assert_eq!(kill, 50);
    assert_eq!(race.readout().race_xp, Some(kill));

    let mut setup = setup(hulled_handling());
    setup.mode = Mode::Zone;
    assert_eq!(Race::start(setup).readout().race_xp, None);
}
