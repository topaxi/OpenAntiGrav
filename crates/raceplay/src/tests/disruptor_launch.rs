//! `WO_DISRUPTOR_LAUNCH`: `Race::fire_disruptor` plays it at the firing craft's
//! `Ship Muzzle` node, and only when the hull authors one
//! (`docs/ghidra/functions/psp-pure-usa/weapon-effects.md`, "Disruptor").

use super::*;

const TABLE: &str = r#"<WeaponStats>
<Weapon type="Global"><Stats slowdown_limit="1"/></Weapon>
<Weapon type="Disruptor">
  <Stats absorb="2" speed="300"/>
  <Effect type="Stall"><EffStats time="10"/></Effect>
  <Effect type="Mirror Left Right"><EffStats time="11"/></Effect>
  <Effect type="No Airbrakes"><EffStats time="12"/></Effect>
  <Effect type="Autopilot Slow"><EffStats speed_percent="13" time="14"/></Effect>
  <Effect type="Autopilot Fast"><EffStats speed_percent="15" time="16"/></Effect>
  <Effect type="Drunk"><EffStats amount="18" time="19"/></Effect>
  <Effect type="Rubber Ship"><EffStats amount="20" time="21"/></Effect>
  <Effect type="Drunk Camera"><EffStats amount="23" time="24"/></Effect>
</Weapon>
</WeaponStats>"#;

/// A grid whose slot 0 can fire a Disruptor, `launch` lent to the trigger when given and
/// the hull's muzzle at `muzzle`.
fn grid_firing_a_disruptor(launch: bool, muzzle: Option<Vec3>) -> Race {
    let mut race = race_with_a_grid();
    race.sim.weapons = Some(Arc::new(
        oag_tables::weapons::parse(TABLE).expect("the table parses"),
    ));
    race.sim.world.ships[0].muzzle = muzzle;
    if launch {
        let effect =
            super::trail_hits::a_one_shot_burst(oag_title::engine_effects::DISRUPTOR_LAUNCH_EFFECT);
        race.view.handles = std::mem::take(&mut race.view.handles)
            .lending(Trigger::DisruptorLaunch, Some(Arc::new(effect)));
    }
    race
}

/// **Red to green.** The launch effect plays on the tick a bolt leaves a hull that
/// authors a `Ship Muzzle`, once per bolt.
#[test]
fn a_disruptor_leaving_a_hull_with_a_muzzle_plays_the_launch_effect() {
    let mut race = grid_firing_a_disruptor(true, Some(Vec3::new(0.0, 0.0, 1.0)));
    assert_eq!(race.view.stage.playing_count(), 0);
    assert!(race.fire_disruptor(0), "a bolt left");
    assert_eq!(race.view.stage.playing_count(), 1);
    assert!(race.fire_disruptor(0), "a second bolt left");
    assert_eq!(race.view.stage.playing_count(), 2, "one per bolt");
}

/// `*(craft+0xe0)` is the `Ship Muzzle` node: a hull with none plays nothing.
#[test]
fn a_hull_with_no_muzzle_plays_no_launch_effect() {
    let mut race = grid_firing_a_disruptor(true, None);
    assert!(race.fire_disruptor(0), "the bolt still leaves");
    assert_eq!(race.view.stage.playing_count(), 0);
}

/// A title whose table names no launch effect plays nothing.
#[test]
fn a_title_with_no_launch_effect_plays_none() {
    let mut race = grid_firing_a_disruptor(false, Some(Vec3::new(0.0, 0.0, 1.0)));
    assert!(race.fire_disruptor(0));
    assert_eq!(race.view.stage.playing_count(), 0);
}
