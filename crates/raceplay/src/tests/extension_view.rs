//! A module's view outputs: `screen-fill` washes its own slot's view, `fov-offset` moves
//! its own slot's field of view (and zero changes no bit of it), and
//! `fire-mod-effect-ahead` queues the mod's effect at the road ahead of its craft.

use oag_ext_abi::{Command, Distance, Rgba, Slot};
use oag_gameplay::PlayerInputs;

use super::extension_faults::scripted;
use super::extensions::grid;
use super::extensions::on::prepared;
use super::local_seat_fx::local_people;

fn slot(index: usize) -> Slot {
    Slot::new(index).unwrap()
}

const FILL: Rgba = Rgba::new(255, 0, 0, 128);

#[test]
fn a_screen_fill_washes_its_slots_view_alone() {
    let fill = Command::ScreenFill {
        slot: slot(1),
        rgba: FILL,
    };
    for views in [2, 4] {
        let mut race = local_people(views, false);
        race.sim.extensions = crate::extensions::Extensions::new(prepared(&[scripted(
            "test.fill",
            "",
            &[fill],
            &[],
        )]));
        race.tick(&PlayerInputs::none());
        for view in 0..views {
            race.set_viewpoint(view);
            let wash = race.screen_wash();
            race.reset_viewpoint();
            let expected = (view == 1).then_some([1.0, 0.0, 0.0, 128.0 / 255.0]);
            assert_eq!(wash, expected, "{views} views, view {view}");
        }
    }
}

#[test]
fn a_screen_fill_beside_a_flash_adds_to_it_and_without_one_the_flash_is_untouched() {
    let mut race = local_people(2, false);
    race.view.screen_flash = Some(oag_fx::flash::ScreenFlash::default());
    let at = race.sim.world.ships[0].physics.body.position;
    race.view
        .screen_flash
        .as_mut()
        .unwrap()
        .start(oag_fx::flash::BLAST, at);
    race.view
        .screen_flash
        .as_mut()
        .unwrap()
        .advance(1.0 / 60.0, at);
    let flash = race.screen_wash().expect("a flash is running");
    race.sim.extensions = crate::extensions::Extensions::new(prepared(&[scripted(
        "test.fill",
        "",
        &[Command::ScreenFill {
            slot: slot(0),
            rgba: FILL,
        }],
        &[],
    )]));
    race.tick(&PlayerInputs::none());
    let both = race.screen_wash().expect("both wash");
    assert_eq!(both[3], 1.0, "one pass, summed");
    assert!(
        both[0] >= flash[0] * flash[3] + FILL.r as f32 / 255.0 * 128.0 / 255.0 - 1e-5
            || both[0] == 1.0,
        "{both:?} over {flash:?}"
    );
}

#[test]
fn a_zero_fov_offset_is_bit_identical_and_an_offset_widens() {
    let aspect = 16.0 / 9.0;
    let authored = 1.2_f32;
    let plain = crate::camera::finish_vertical_fov(authored, 0.0, 0.0, 0.0, aspect);
    assert_eq!(
        plain,
        oag_render::camera::fit_vertical_fov(authored, crate::AUTHORED_ASPECT, aspect)
    );
    let wide = crate::camera::finish_vertical_fov(authored, 10.0, 0.0, 0.0, aspect);
    let narrow = crate::camera::finish_vertical_fov(authored, -10.0, 0.0, 0.0, aspect);
    assert!(narrow < plain && plain < wide, "{narrow} {plain} {wide}");
}

#[test]
fn a_fov_offset_reaches_its_slots_view_alone() {
    let offset = Command::FovOffset {
        slot: slot(1),
        degrees: 12.0,
    };
    let mut race = local_people(2, false);
    race.sim.extensions =
        crate::extensions::Extensions::new(prepared(&[scripted("test.fov", "", &[offset], &[])]));
    race.tick(&PlayerInputs::none());
    assert_eq!(race.extension_fov_offset(1), 12.0);
    assert_eq!(race.extension_fov_offset(0), 0.0);
}

#[test]
fn a_fire_effect_ahead_queues_the_mods_effect_on_the_road_in_front_of_its_craft() {
    let ahead = Command::FireModEffectAhead {
        slot: slot(1),
        index: 2,
        distance: Distance::Units(20.0),
    };
    let mut race = grid(prepared(&[scripted("test.portal", "", &[ahead], &[])]));
    let ship = &mut race.sim.world.ships[1];
    ship.physics.body.position.x = 10.0;
    ship.physics.body.position.z = 0.0;
    ship.standing = oag_race::Standing::default();
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }
    race.tick_sim_only(&PlayerInputs::none());
    let from = race.sim.world.ships[1].physics.body.position;
    let queued = race.sim.extensions.effects_for_tests();
    let [
        (
            1,
            crate::extension_effects::Fired::OwnAt {
                mod_id, index, at, ..
            },
        ),
    ] = &queued[..]
    else {
        panic!("one effect at the road ahead: {queued:?}");
    };
    assert_eq!((mod_id.as_str(), *index), ("test.portal", 2));
    assert!(
        (at.x - from.x - 20.0).abs() < 3.0,
        "20 units ahead of {from:?}: {at:?}"
    );
}

#[test]
fn a_mods_pickup_look_is_its_row_for_the_title_and_the_base_look_elsewhere() {
    use super::extensions::on::{module, prepared_for};
    use oag_tables::pickup_look::{Colour, Icon, OddsRow, PickupLook};
    use oag_tables::weapons::{PickupOdds, Weapon};
    let mut source = module("test.looks", "i32.const 0");
    source.pickup = Some(PickupLook {
        icon: Icon::Title(Weapon::Turbo),
        colour: Colour::Rgba([1, 2, 3, 4]),
    });
    source.pickup_titles = vec![(
        "wipeout-hd".to_string(),
        PickupLook {
            icon: Icon::Mod("hd".to_string()),
            colour: Colour::Rgba([1, 2, 3, 4]),
        },
    )];
    // The odds make this module own the mod pickup from tick 0.
    source.pickup_odds = vec![OddsRow {
        title: None,
        class: None,
        odds: PickupOdds::default(),
    }];
    let look = |title| {
        crate::extensions::Extensions::new(prepared_for(title, &[source.clone()]))
            .pickup_look(0)
            .map(|look| look.icon)
    };
    assert_eq!(
        look("wipeout-hd"),
        Some(Icon::Mod(crate::extensions::mod_icon_src(
            "test.looks",
            "hd"
        )))
    );
    assert_eq!(look("wipeout-pulse"), Some(Icon::Title(Weapon::Turbo)));
}

#[test]
fn an_icons_art_box_is_the_drawn_pixels_and_where_they_sit_on_the_canvas() {
    // 8 by 6, a 2 by 3 block at x 4..6, y 1..4.
    let mut rgba = vec![0_u8; 8 * 6 * 4];
    for y in 1..4 {
        for x in 4..6 {
            rgba[(y * 8 + x) * 4 + 3] = 255;
        }
    }
    let (extent, centre) = crate::extensions::art_box(8, 6, &rgba).unwrap();
    assert_eq!(extent, [2.0, 3.0]);
    assert_eq!(centre, [5.0 - 4.0, 2.5 - 3.0]);
    assert_eq!(crate::extensions::art_box(8, 6, &[0; 8 * 6 * 4]), None);
}

/// A guest's `clear-pickup`, `scale-speed` and `transit` for a wrecked craft are ignored
/// engine-side (`Race::may_use_pickup`), and the same commands act on a racing one.
#[test]
fn a_guests_pickup_and_motion_commands_for_a_wrecked_craft_are_ignored() {
    use oag_physics::CraftState;
    let cmds = [
        Command::ClearPickup { slot: slot(1) },
        Command::ScaleSpeed {
            slot: slot(1),
            factor: 0.0,
        },
    ];
    let run = |state: CraftState| {
        let mut race = grid(prepared(&[scripted("test.wreck", "", &cmds, &[])]));
        let ship = &mut race.sim.world.ships[1];
        ship.pickup.weapon = Some(oag_tables::weapons::Weapon::Extension);
        ship.physics.body.linear_velocity = oag_core::math::Vec3::new(30.0, 0.0, 0.0);
        ship.physics.craft_state = state;
        race.tick_sim_only(&PlayerInputs::none());
        let ship = &race.sim.world.ships[1];
        (
            ship.pickup.weapon,
            ship.physics.body.linear_velocity.length(),
        )
    };
    let (held, speed) = run(CraftState::Destroyed);
    assert_eq!(
        held,
        Some(oag_tables::weapons::Weapon::Extension),
        "kept for after the respawn"
    );
    assert!(speed > 1.0, "not scaled to zero: {speed}");
    let (held, _) = run(CraftState::Racing);
    assert_eq!(held, None, "a racing craft's pickup is cleared");
}
