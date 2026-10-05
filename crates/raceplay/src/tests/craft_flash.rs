//! The washes a craft's own state edges start - see `race::craft_flash`.

use super::*;
use oag_physics::CraftState;

const DT: f32 = 1.0 / 60.0;

fn with_flash() -> Race {
    let mut race = race_with_a_grid();
    assert!(race.sim.world.ship_count > 3, "the fixture has opponents");
    race.view.screen_flash = Some(oag_fx::flash::ScreenFlash::default());
    race
}

/// What the wash draws this tick, seen from a craft's own position so no
/// falloff applies.
fn drawn(race: &mut Race) -> Option<[f32; 4]> {
    let flash = race.view.screen_flash.as_mut().unwrap();
    flash.advance(DT, Vec3::ZERO);
    flash.colour()
}

/// The explosion ending starts the yellow kind-0 wash, and only then - for
/// the player as for an opponent, now that the destroy camera is the active
/// camera the wash's falloff is measured from.
#[test]
fn a_craft_going_out_of_the_race_washes_the_screen_yellow() {
    let mut race = with_flash();
    let at = race.sim.world.ships[3].physics.body.position;
    race.view.screen_flash = Some(oag_fx::flash::ScreenFlash::default());

    let player_at = race.sim.world.ships[0].physics.body.position;
    race.sim.world.ships[0].physics.craft_state = CraftState::Destroyed;
    race.advance_craft_flashes();
    assert_eq!(drawn(&mut race), None, "state 4 starts no wash");
    race.sim.world.ships[0].physics.craft_state = CraftState::Eliminated;
    race.advance_craft_flashes();
    let flash = race.view.screen_flash.as_mut().unwrap();
    flash.advance(DT, player_at);
    let colour = flash.colour().expect("the player's own state 5 washes too");
    assert_eq!(colour[2], 0.0, "yellow to red: never any blue");
    race.sim.world.ships[0].physics.craft_state = CraftState::Racing;
    race.advance_craft_flashes();
    race.view.screen_flash = Some(oag_fx::flash::ScreenFlash::default());

    race.sim.world.ships[3].physics.craft_state = CraftState::Destroyed;
    race.advance_craft_flashes();
    assert_eq!(drawn(&mut race), None, "state 4 starts no wash");

    race.sim.world.ships[3].physics.craft_state = CraftState::Eliminated;
    race.advance_craft_flashes();
    let flash = race.view.screen_flash.as_mut().unwrap();
    flash.advance(DT, at);
    let colour = flash.colour().expect("state 5 starts kind 0");
    assert_eq!(colour[2], 0.0, "yellow to red: never any blue");
    assert!(colour[0] > 0.9 && colour[1] > 0.9, "{colour:?}");
}

/// 1.5 s after that, the big explosion: white for the local player, kind 0
/// for anyone else.
#[test]
fn the_big_explosion_follows_after_the_dwell_and_is_white_for_the_player() {
    for (slot, white) in [(0_usize, true), (3, false)] {
        let mut race = with_flash();
        race.sim.world.ships[slot].physics.craft_state = CraftState::Destroyed;
        race.advance_craft_flashes();
        race.sim.world.ships[slot].physics.craft_state = CraftState::Eliminated;
        race.advance_craft_flashes();
        // Let the small explosion's wash run out (0.5 s) before the big one.
        let mut seen = None;
        for tick in 0..100 {
            race.advance_craft_flashes();
            if let Some(colour) = drawn(&mut race)
                && tick >= 60
            {
                seen = Some((tick, colour));
                break;
            }
        }
        let (tick, colour) = seen.expect("the big explosion washes the screen");
        assert!((88..=92).contains(&tick), "fired at tick {tick}, not 1.5 s");
        assert_eq!(colour[2] > 0.9, white, "{slot}: {colour:?}");
    }
}

/// A reset washes the player's screen white and nobody else's.
#[test]
fn only_the_local_players_reset_washes_the_screen() {
    let mut race = with_flash();
    race.flash_player_reset(3);
    assert_eq!(drawn(&mut race), None, "an opponent's reset is not seen");

    race.flash_player_reset(0);
    let colour = drawn(&mut race).expect("the player's reset washes");
    assert!(colour[..3].iter().all(|c| *c > 0.97), "white: {colour:?}");
}
