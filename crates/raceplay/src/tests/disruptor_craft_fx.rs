//! The on-craft Disruptor effect (`DisruptorFx_Construct`, `0x08859b78`; update `0x0885a010`):
//! a disrupted craft carries it while the disruption runs and a fade after
//! (`docs/ghidra/functions/psp-pure-usa/weapon-effects.md`, "Disruptor").

use super::*;
use oag_gameplay::PlayerInputs;
use oag_tables::weapons::DisruptorEffectKind;
use oag_weapons::disruption::Disruption;

const LAW: oag_title::weapons::DisruptorCraftFx = oag_title::weapons::DisruptorCraftFx {
    model: "",
    target: 1.0,
    rate: 0.1,
    retire_below: 0.05,
    origin: oag_title::Origin::Measured,
};

/// A grid whose slot 1 is disrupted for `seconds`, with Pure's law in the view.
fn grid_with_a_disrupted_craft(law: Option<oag_title::weapons::DisruptorCraftFx>) -> Race {
    let mut race = race_with_a_grid();
    race.view.disruptor_craft_law = law;
    race.sim.world.ships[1].disruption = Disruption {
        kind: Some(DisruptorEffectKind::Stall),
        timer: 0.5,
    };
    race
}

/// The tint the craft's effect draws at after each tick, `None` once it is gone.
fn tints_over(race: &mut Race, ticks: usize) -> Vec<Option<f32>> {
    (0..ticks)
        .map(|_| {
            race.tick(&PlayerInputs::none());
            race.disruptor_craft_draws()[1].map(|(_, tint)| tint)
        })
        .collect()
}

/// **Red to green.** The effect eases up while the disruption runs (`1 - 0.9^n`, the update's
/// `cur += (1 - cur) * 0.1` per tick), fades once it ends and is gone when it falls under
/// `0.05`: present for the disruption's thirty ticks plus the fade, absent before and after.
#[test]
fn a_disrupted_craft_carries_the_effect_for_the_disruption_and_its_fade() {
    let mut race = grid_with_a_disrupted_craft(Some(LAW));
    assert_eq!(
        race.disruptor_craft_draws()[1],
        None,
        "nothing before a tick"
    );
    let tints = tints_over(&mut race, 80);

    let mut cur = 0.0_f32;
    let mut expect = Vec::new();
    for tick in 0..80 {
        let disrupted = tick < 30;
        let target = if disrupted { 1.0 } else { 0.0 };
        cur += (target - cur) * 0.1;
        if !disrupted && cur < 0.05 {
            expect.push(None);
            continue;
        }
        expect.push(
            expect
                .last()
                .map_or(Some(cur), |last: &Option<f32>| last.map(|_| cur)),
        );
    }
    for (tick, (got, want)) in tints.iter().zip(&expect).enumerate() {
        match (got, want) {
            (Some(got), Some(want)) => assert!((got - want).abs() < 1.0e-5, "tick {tick}"),
            (None, None) => {}
            _ => panic!("tick {tick}: drawn {got:?}, expected {want:?}"),
        }
    }
    assert!(tints[0].is_some(), "drawn from the first tick");
    assert!(tints[40].is_some(), "still fading ten ticks after the end");
    assert!(tints[79].is_none(), "gone well after the end");
}

/// A title with no on-craft effect draws none.
#[test]
fn a_title_with_no_craft_effect_draws_none() {
    let mut race = grid_with_a_disrupted_craft(None);
    assert!(tints_over(&mut race, 10).iter().all(Option::is_none));
}

/// A craft nothing disrupted draws none.
#[test]
fn an_undisrupted_craft_draws_none() {
    let mut race = grid_with_a_disrupted_craft(Some(LAW));
    race.sim.world.ships[1].disruption = Disruption::default();
    assert!(tints_over(&mut race, 10).iter().all(Option::is_none));
}
