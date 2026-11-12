use super::*;

const DT: f32 = 1.0 / 60.0;

#[test]
fn a_rocket_warns_from_behind_inside_four_hundred() {
    assert_eq!(classify(Weapon::Rocket, 399.0), Some(Icon::Rocket));
    assert_eq!(classify(Weapon::Rocket, 401.0), None);
    assert_eq!(classify(Weapon::Rocket, 400.0), None, "strict");
    assert_eq!(classify(Weapon::Rocket, 0.0), None, "strict");
    assert_eq!(
        classify(Weapon::Rocket, -100.0),
        None,
        "ahead of the viewer"
    );
}

#[test]
fn every_rear_window_matches_the_table() {
    let rows = [
        (Weapon::Missile, 399.0, 401.0),
        (Weapon::Quake, 799.0, 801.0),
        (Weapon::Cannon, 499.0, 501.0),
        (Weapon::Plasma, 499.0, 501.0),
        (Weapon::LeachBeam, 499.0, 501.0),
        (Weapon::Repulser, 499.0, 501.0),
        (Weapon::Shuriken, 499.0, 501.0),
    ];
    for (weapon, inside, outside) in rows {
        let icon = classify(weapon, inside).unwrap_or_else(|| panic!("{weapon:?} at {inside}"));
        assert_eq!(icon.side(), Side::Rear, "{weapon:?}");
        assert_eq!(classify(weapon, outside), None, "{weapon:?} at {outside}");
        assert_eq!(classify(weapon, 1.0).map(Icon::side), Some(Side::Rear));
        assert_eq!(classify(weapon, -1.0), None, "{weapon:?} ahead");
    }
}

#[test]
fn a_mine_or_bomb_warns_from_ahead_inside_five_hundred() {
    for (weapon, icon) in [(Weapon::Mine, Icon::Mine), (Weapon::Bomb, Icon::Bomb)] {
        assert_eq!(classify(weapon, -499.0), Some(icon));
        assert_eq!(classify(weapon, -501.0), None);
        assert_eq!(classify(weapon, -500.0), None, "strict");
        assert_eq!(classify(weapon, 100.0), None, "behind the viewer");
        assert_eq!(icon.side(), Side::Forward);
    }
}

#[test]
fn turbo_shield_and_autopilot_never_warn() {
    for weapon in [Weapon::Turbo, Weapon::Shield, Weapon::Autopilot] {
        for gap in [-300.0, -1.0, 1.0, 100.0, 399.0] {
            assert_eq!(classify(weapon, gap), None, "{weapon:?} at {gap}");
        }
    }
}

#[test]
fn every_icon_has_a_distinct_widget_in_its_own_group() {
    let icons = [
        Icon::Rocket,
        Icon::Missile,
        Icon::Quake,
        Icon::Cannon,
        Icon::Plasma,
        Icon::LeachBeam,
        Icon::Repulser,
        Icon::Shuriken,
        Icon::Mine,
        Icon::Bomb,
    ];
    let mut names: Vec<_> = icons.iter().map(|icon| icon.widget()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), icons.len());
    for icon in icons {
        let prefix = match icon.side() {
            Side::Forward => "ForwardWarning",
            Side::Rear => "RearWarning",
        };
        assert!(icon.widget().starts_with(prefix), "{icon:?}");
    }
}

/// The lit phases a raise at frame zero shows, one entry per frame.
fn lit_frames(frames: usize) -> Vec<bool> {
    let mut warnings = Warnings::default();
    assert!(warnings.raise_for_fire(Weapon::Rocket, 100.0));
    (0..frames)
        .map(|_| {
            warnings.advance(DT);
            warnings.shown().rear == Some(Look::Title(Icon::Rocket))
        })
        .collect()
}

#[test]
fn the_raise_frame_is_unlit_then_eight_phases_blink_and_it_clears() {
    let lit = lit_frames(140);
    assert!(!lit[0], "the raise frame is off");
    assert!(lit[1], "the next one is on");

    // The formula over the same accumulated timer, not hard-coded frame numbers.
    let mut timer = SECONDS;
    let expected: Vec<bool> = (0..140)
        .map(|_| {
            let on = (timer * 8.0) as i64 & 1 == 1;
            timer = (timer - DT).max(0.0);
            on
        })
        .collect();
    assert_eq!(lit, expected);

    let mut runs = Vec::new();
    let mut length = 0;
    for &frame in &lit {
        if frame {
            length += 1;
        } else if length > 0 {
            runs.push(length);
            length = 0;
        }
    }
    assert_eq!(runs.len(), 8, "eight lit phases: {runs:?}");
    assert!(runs.iter().all(|run| (7..=8).contains(run)), "{runs:?}");
    assert!(lit[121..].iter().all(|frame| !frame), "nothing after 2.0 s");
}

#[test]
fn the_slot_clears_to_nothing_after_two_seconds() {
    let mut warnings = Warnings::default();
    warnings.raise(Icon::Missile);
    for _ in 0..119 {
        warnings.advance(DT);
        assert!(warnings.active(), "still running");
    }
    for _ in 0..3 {
        warnings.advance(DT);
    }
    assert!(!warnings.active());
    assert_eq!(*warnings.shown(), Shown::default());
    // Raising again after it cleared starts from a full timer.
    warnings.raise(Icon::Quake);
    warnings.advance(DT);
    assert!(warnings.active());
}

#[test]
fn a_second_raise_in_the_slot_restarts_it_and_swaps_the_icon() {
    let mut warnings = Warnings::default();
    warnings.raise(Icon::Rocket);
    for _ in 0..40 {
        warnings.advance(DT);
    }
    warnings.raise(Icon::Quake);
    warnings.advance(DT);
    assert_eq!(
        warnings.shown().rear,
        None,
        "the raise frame is unlit again"
    );
    warnings.advance(DT);
    assert_eq!(warnings.shown().rear, Some(Look::Title(Icon::Quake)));
}

#[test]
fn the_two_groups_run_independently() {
    let mut warnings = Warnings::default();
    warnings.raise(Icon::Rocket);
    warnings.raise(Icon::Mine);
    warnings.advance(DT);
    warnings.advance(DT);
    assert_eq!(
        *warnings.shown(),
        Shown {
            forward: Some(Look::Title(Icon::Mine)),
            rear: Some(Look::Title(Icon::Rocket)),
        }
    );
}

#[test]
fn a_gap_out_of_reach_raises_nothing() {
    let mut warnings = Warnings::default();
    assert!(!warnings.raise_for_fire(Weapon::Rocket, 900.0));
    assert!(!warnings.raise_for_fire(Weapon::Mine, -900.0));
    assert!(!warnings.active());
}

fn warning_layout() -> crate::Layout {
    let image = |name: &str, x: i32| {
        format!(
            r#"<Image name="{name}"><Values x="{x}" y="0" width="20" height="20" U="0" V="0" TxtrWidth="20" TxtrHeight="20" src="Data\HUD\Textures\PulseHUD.mip"/></Image>"#
        )
    };
    crate::Layout::from_xml(&format!(
        r#"<Screen><Screen name="HUD">{}{}{}{}{}</Screen></Screen>"#,
        image("RearWarningBackground", 0),
        image("RearWarningRocketIcon", 30),
        image("RearWarningMissileIcon", 60),
        image("ForwardWarningBackground", 90),
        image("ForwardWarningMineIcon", 120),
    ))
}

fn sprites_drawn(readout: &crate::Readout) -> usize {
    let layout = warning_layout();
    let strings = oag_ui::language::StringTable::default();
    let cx = crate::tests::context(&layout, &strings);
    crate::draw_list(&cx, readout).sprites.len()
}

#[test]
fn a_lit_group_draws_its_background_and_its_own_icon_only() {
    let blank = crate::Readout::blank();
    let nothing = sprites_drawn(&blank);
    let lit = |forward, rear| crate::Readout {
        incoming: Shown { forward, rear },
        ..crate::Readout::blank()
    };
    assert_eq!(
        sprites_drawn(&lit(None, Some(Look::Title(Icon::Rocket)))),
        nothing + 2
    );
    assert_eq!(
        sprites_drawn(&lit(Some(Look::Title(Icon::Mine)), None)),
        nothing + 2
    );
    assert_eq!(
        sprites_drawn(&lit(
            Some(Look::Title(Icon::Mine)),
            Some(Look::Title(Icon::Missile))
        )),
        nothing + 4
    );
    // An icon the layout does not author draws nothing, and not a stand-in.
    assert_eq!(
        sprites_drawn(&lit(None, Some(Look::Title(Icon::Quake)))),
        nothing + 1
    );
}

fn mod_look(side: Side) -> Look {
    Look::Mod {
        side,
        src: "mod:test/portal".into(),
    }
}

/// The draws of a readout lighting `forward` and `rear` over a sheet that holds the title's
/// atlas and, when `art` is given, a mod icon of that size.
fn draws_with_art(
    forward: Option<Look>,
    rear: Option<Look>,
    art: Option<(u32, u32)>,
) -> Vec<oag_ui::frontend::Draw> {
    let layout = warning_layout();
    let strings = oag_ui::language::StringTable::default();
    let placed = |width, height| crate::sprite::Placed {
        x: 0,
        y: 0,
        width,
        height,
        quad_extent: None,
        quad_centre: None,
        blend: None,
    };
    let mut entries = vec![(oag_pulse::hud::ATLAS, placed(256, 256))];
    entries.extend(art.map(|(w, h)| ("mod:test/portal", placed(w, h))));
    let sheet = crate::sprite::Sheet::placed_at(&entries);
    let mut cx = crate::tests::context(&layout, &strings);
    cx.sheet = &sheet;
    let readout = crate::Readout {
        incoming: Shown { forward, rear },
        ..crate::Readout::blank()
    };
    let blank = crate::draw_list(&cx, &crate::Readout::blank())
        .sprites
        .len();
    crate::draw_list(&cx, &readout).sprites[blank..].to_vec()
}

fn rect_of(draw: &oag_ui::frontend::Draw) -> [f32; 4] {
    let oag_ui::frontend::Draw::Sprite { rect, .. } = draw else {
        panic!("a sprite: {draw:?}");
    };
    *rect
}

#[test]
fn a_mods_art_is_fitted_to_the_title_icons_box_on_its_group_and_not_to_its_own_size() {
    // The rear box is `RearWarningRocketIcon`'s 20 x 20 at (30, 0); art twice as wide as tall
    // fills its width, aspect kept, centred on the box.
    let rear = draws_with_art(None, Some(mod_look(Side::Rear)), Some((80, 40)));
    assert_eq!(rear.len(), 2, "the group's backdrop and the art");
    assert_eq!(
        rect_of(&rear[0]),
        [0.0, 0.0, 20.0, 20.0],
        "RearWarningBackground"
    );
    assert_eq!(rect_of(&rear[1]), [30.0, 5.0, 20.0, 10.0]);
    // A mod may light the forward group with its own art as well.
    let forward = draws_with_art(Some(mod_look(Side::Forward)), None, Some((8, 8)));
    assert_eq!(
        rect_of(&forward[0]),
        [90.0, 0.0, 20.0, 20.0],
        "ForwardWarningBackground"
    );
    assert_eq!(
        rect_of(&forward[1]),
        [120.0, 0.0, 20.0, 20.0],
        "ForwardWarningMineIcon's box"
    );
}

#[test]
fn art_the_sheet_lacks_lights_no_group_at_all() {
    assert!(draws_with_art(None, Some(mod_look(Side::Rear)), None).is_empty());
}

#[test]
fn a_mod_look_lights_its_group_and_is_what_the_hud_reads_back() {
    let mut warnings = Warnings::default();
    warnings.raise(mod_look(Side::Forward));
    warnings.advance(DT);
    warnings.advance(DT);
    assert_eq!(
        *warnings.shown(),
        Shown {
            forward: Some(mod_look(Side::Forward)),
            rear: None,
        }
    );
    warnings.raise(Icon::Rocket);
    warnings.advance(DT);
    warnings.advance(DT);
    assert_eq!(
        warnings.shown().forward,
        Some(mod_look(Side::Forward)),
        "a title icon on the other group leaves it alone"
    );
    assert_eq!(warnings.shown().rear, Some(Look::Title(Icon::Rocket)));
}
