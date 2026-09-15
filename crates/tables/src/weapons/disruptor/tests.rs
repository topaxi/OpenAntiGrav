//! What the Disruptor decode in [`super`] is asserted to do. Its own file for
//! the reason `weapons/tests.rs` is: the 200-line inline ceiling.

use super::*;
use crate::weapons::{Weapon, parse as parse_table};

/// Pure's shape - `<Stats absorb speed>` and fourteen `<Effect>` children, of
/// which the original's parser reads ten - with **invented** numbers. The four
/// dead blocks are authored here too, because the fixture has to prove they
/// are walked past rather than merely never written.
const FIXTURE: &str = r#"<WeaponStats>
<Weapon type="Global"><Stats slowdown_limit="1"/></Weapon>
<Weapon type="Disruptor">
  <Stats absorb="2" speed="300"/>
  <Effect type="Stall"><EffStats time="10"/></Effect>
  <Effect type="Fire Weapon"/>
  <Effect type="Mirror Left Right"><EffStats time="11"/></Effect>
  <Effect type="No Airbrakes"><EffStats time="12"/></Effect>
  <Effect type="Turbo Now"/>
  <Effect type="Autopilot Slow"><EffStats speed_percent="13" time="14"/></Effect>
  <Effect type="Autopilot Fast"><EffStats speed_percent="15" time="16"/></Effect>
  <Effect type="HUD Flicker"><EffStats time="17"/></Effect>
  <Effect type="Drunk"><EffStats amount="18" time="19"/></Effect>
  <Effect type="Steal Weapon"/>
  <Effect type="Rubber Ship"><EffStats amount="20" time="21"/></Effect>
  <Effect type="Adjust Gravity"><EffStats amount="22"/></Effect>
  <Effect type="Drunk Camera"><EffStats amount="23" time="24"/></Effect>
  <Effect type="Trippy"><EffStats time="25"/></Effect>
</Weapon>
<Pickupodds class="Vector">
<Weapon type="Disruptor"><Stats ai="30" back="31" front="32" human="33"/></Weapon>
</Pickupodds>
</WeaponStats>"#;

fn fixture() -> DisruptorStats {
    parse_table(FIXTURE)
        .expect("the fixture parses")
        .disruptor()
        .expect("the fixture authors a Disruptor")
}

#[test]
fn the_stats_pair_and_all_ten_parsed_effects_are_read() {
    let stats = fixture();
    assert_eq!(stats.absorb, 2.0);
    assert_eq!(stats.speed, 300.0);
    let expect = |kind: EffectKind, time: f32, amount: Option<f32>, percent: Option<f32>| {
        assert_eq!(
            stats.effect(kind),
            Some(Effect {
                time,
                amount,
                speed_percent: percent
            }),
            "{kind:?}"
        );
    };
    expect(EffectKind::Stall, 10.0, None, None);
    expect(EffectKind::MirrorLeftRight, 11.0, None, None);
    expect(EffectKind::NoAirbrakes, 12.0, None, None);
    expect(EffectKind::AutopilotSlow, 14.0, None, Some(13.0));
    expect(EffectKind::AutopilotFast, 16.0, None, Some(15.0));
    expect(EffectKind::HudFlicker, 17.0, None, None);
    expect(EffectKind::Drunk, 19.0, Some(18.0), None);
    expect(EffectKind::RubberShip, 21.0, Some(20.0), None);
    expect(EffectKind::DrunkCamera, 24.0, Some(23.0), None);
    expect(EffectKind::Trippy, 25.0, None, None);
}

/// The four dead blocks are walked past: nothing about them reaches the
/// struct, and their presence does not fail the file.
#[test]
fn the_four_unparsed_effects_are_walked_past() {
    let stats = fixture();
    assert_eq!(stats.effects.iter().filter(|e| e.is_some()).count(), 10);
    let dead = ["Fire Weapon", "Turbo Now", "Steal Weapon", "Adjust Gravity"];
    for name in dead {
        assert!(
            EffectKind::ALL.iter().all(|k| k.as_type() != name),
            "{name} has grown a parser branch the original does not have"
        );
    }
}

#[test]
fn the_eight_rolled_effects_exclude_the_two_parsed_dead_ones() {
    assert_eq!(EffectKind::ROLLED.len(), 8);
    assert!(!EffectKind::ROLLED.contains(&EffectKind::HudFlicker));
    assert!(!EffectKind::ROLLED.contains(&EffectKind::Trippy));
    // `Disruptor_RollEffect`'s switch order, kind 0..7.
    assert_eq!(EffectKind::ROLLED[0], EffectKind::Stall);
    assert_eq!(EffectKind::ROLLED[5], EffectKind::Drunk);
    assert_eq!(EffectKind::ROLLED[6], EffectKind::DrunkCamera);
    assert_eq!(EffectKind::ROLLED[7], EffectKind::RubberShip);
}

#[test]
fn every_effect_type_name_round_trips() {
    for kind in EffectKind::ALL {
        assert_eq!(
            EffectKind::ALL
                .into_iter()
                .find(|k| k.as_type() == kind.as_type()),
            Some(kind)
        );
    }
}

/// `speed + 80 * class`, Vector first.
#[test]
fn the_speed_climbs_eighty_a_rung() {
    let stats = fixture();
    assert_eq!(stats.speed_for_class(0), 300.0);
    assert_eq!(stats.speed_for_class(4), 300.0 + 4.0 * PER_CLASS_KMH);
}

/// An effect the original reads an `amount` for must author one; one it does
/// not is free to omit it. Both halves of the parser's shape.
#[test]
fn an_effect_missing_an_attribute_its_branch_reads_skips_the_weapon() {
    let broken = FIXTURE.replace(
        r#"<EffStats amount="18" time="19"/>"#,
        r#"<EffStats time="19"/>"#,
    );
    let stats = parse_table(&broken).expect("one undecodable weapon does not fail the file");
    assert!(stats.disruptor().is_none());
    assert!(stats.skipped.contains(&(Weapon::Disruptor, "amount")));

    let fine = FIXTURE.replace(
        r#"<EffStats time="10"/>"#,
        r#"<EffStats time="10" amount="99"/>"#,
    );
    let stats = parse_table(&fine).expect("parses");
    assert_eq!(
        stats.disruptor().and_then(|d| d.effect(EffectKind::Stall)),
        Some(Effect {
            time: 10.0,
            amount: Some(99.0),
            speed_percent: None
        })
    );
}

#[test]
fn an_unauthored_effect_is_none_not_zero() {
    let without = FIXTURE.replace(
        r#"<Effect type="Trippy"><EffStats time="25"/></Effect>"#,
        "",
    );
    let stats = parse_table(&without)
        .expect("parses")
        .disruptor()
        .expect("a Disruptor");
    assert_eq!(stats.effect(EffectKind::Trippy), None);
    assert_eq!(stats.effects.iter().filter(|e| e.is_some()).count(), 9);
}

/// Pulse's shape: no Disruptor block at all, and that is `None` rather than an
/// error, with no `skipped` row - the file simply does not author one.
#[test]
fn a_file_without_a_disruptor_has_none_and_skips_nothing() {
    let none =
        "<WeaponStats><Weapon type=\"Global\"><Stats slowdown_limit=\"1\"/></Weapon></WeaponStats>";
    let stats = parse_table(none).expect("parses");
    assert_eq!(stats.disruptor(), None);
    assert!(stats.skipped.is_empty());
}

#[test]
fn the_disruptor_is_in_the_pool_last_and_carries_absorb_and_odds() {
    assert_eq!(Weapon::ALL[Weapon::ALL.len() - 1], Weapon::Disruptor);
    assert_eq!(Weapon::from_type("Disruptor"), Some(Weapon::Disruptor));
    let stats = parse_table(FIXTURE).expect("parses");
    assert_eq!(stats.absorb(Weapon::Disruptor), Some(2.0));
    let odds = stats
        .pickups_for("Vector")
        .and_then(|t| t.get(Weapon::Disruptor))
        .expect("the Vector table weights the Disruptor");
    assert_eq!(
        (odds.ai, odds.back, odds.front, odds.human),
        (30.0, 31.0, 32.0, 33.0)
    );
}
