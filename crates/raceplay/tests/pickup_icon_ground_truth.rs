//! Pure's weapon-icon `<Model>`s, against the disc that authors them.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^pickup_icon_ground_truth::/)'
//! ```
//!
//! # What only real data can say here
//!
//! `crates/pure/src/hud.rs`'s `PICKUP_ICON_MODELS` and
//! `oag_hud::pickup_model_tests` both assert against a hand-written
//! fixture - three widgets and an invented `.vex` for the backdrop grid. What
//! they cannot say is the thing `docs/gameplay/pickups.md`'s retracted claim
//! got wrong in the first place: **that the ten real icon models actually
//! resolve, decode, and land in the sheet with a usable quad extent.** This
//! is what checks the real disc.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pure-psp-usa.chd")
}

fn single_race() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// The ten weapon-icon widgets `oag_pure::hud::PICKUP_ICON_MODELS` names, plus
/// the backdrop grid, all resolve into the HUD sheet with a real quad extent -
/// not the invented one the unit fixture uses.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_named_icon_model_decodes_with_a_real_quad_extent() {
    let Some(loaded) = single_race() else { return };
    let layout = loaded
        .hud
        .layout
        .as_ref()
        .expect("Pure's arcade layout parsed");

    let mut checked = 0;
    for name in oag_pure::hud::PICKUP_ICON_MODELS.into_iter().flatten() {
        let model = layout
            .models
            .iter()
            .find(|m| m.name == name)
            .unwrap_or_else(|| panic!("Pure's arcade layout has no {name}"));
        let placed = loaded.hud.sheet.get(&model.src).unwrap_or_else(|| {
            panic!(
                "{name}'s model {} did not reach the HUD sheet - a weapon \
                 held on a real race would draw nothing for it",
                model.src
            )
        });
        let extent = placed
            .quad_extent
            .unwrap_or_else(|| panic!("{name}'s model {} decoded with no quad extent", model.src));
        assert!(
            extent[0] > 0.0 && extent[1] > 0.0,
            "{name} decoded to a degenerate quad {extent:?}"
        );
        checked += 1;
    }
    assert_eq!(
        checked, 10,
        "PICKUP_ICON_MODELS should name ten icons on the real table - Pure's \
         own roster, the Disruptor included (Cannon/LeachBeam/Repulser/Shuriken \
         excluded)"
    );

    let grid = oag_pure::hud::ART
        .pickup_icon_backdrop_model
        .expect("Pure names a backdrop grid model");
    let model = layout
        .models
        .iter()
        .find(|m| m.name == grid)
        .unwrap_or_else(|| panic!("Pure's arcade layout has no {grid}"));
    assert!(
        loaded.hud.sheet.get(&model.src).is_some(),
        "{grid}'s model {} did not reach the HUD sheet",
        model.src
    );
}

/// A held Turbo draws the grid and its own coloured icon on a real race, in
/// `TURBO_icon`'s disc-authored `0xff40ff40`.
///
/// The end-to-end path: real archive, real layout, real sheet, real
/// `oag_hud::draw::pickup_model_draws` - nothing here is a fixture.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_held_turbo_draws_its_own_authored_green_on_a_real_race() {
    let Some(loaded) = single_race() else { return };
    let layout = loaded
        .hud
        .layout
        .as_ref()
        .expect("Pure's arcade layout parsed");
    let turbo_model = layout
        .models
        .iter()
        .find(|m| m.name == "TURBO_icon")
        .expect("Pure's arcade layout has TURBO_icon");
    let expected_color = turbo_model
        .colour
        .expect("TURBO_icon authors its own colour");

    let context = loaded
        .hud
        .context()
        .expect("Pure's arcade layout parses; a context is buildable");

    let mut race = race::Race::start(loaded.setup);
    race.sim.world.ships[0].pickup.weapon = Some(oag_tables::weapons::Weapon::Turbo);
    race.tick(&PlayerInputs::none());

    let readout = race.readout();
    assert_eq!(
        readout.pickup,
        Some(oag_tables::weapons::Weapon::Turbo),
        "the readout must carry the held weapon for the draw list to find it"
    );
    let frame = oag_hud::draw_list(&context, &readout);

    // A `<Mode3D><Model>` widget, so a `BlendedSprite`: the quad carries the
    // blend its own model declares rather than taking the pipeline's. See
    // `oag_hud::sprite::Placed::blend`.
    let icon_draw = frame
        .sprites
        .iter()
        .find(|draw| matches!(draw, oag_ui::frontend::Draw::BlendedSprite { color, .. } if *color == expected_color))
        .unwrap_or_else(|| {
            panic!(
                "no sprite drew in TURBO_icon's own colour {expected_color:?}; \
                 the frame carried: {:?}",
                frame.sprites
            )
        });
    // **Measured on Pure's own disc, and it differs from Pulse's sights.**
    // `TURBO_icon`'s model declares `AlphaOver` (`pass_mask & 0x100`), where
    // all three of Pulse's lock-on sight models declare `Additive` (`& 0x200`)
    // - which is exactly why the class is read off each model rather than
    // tabulated per widget. Pinned so a change to that reading has to say so
    // here.
    let oag_ui::frontend::Draw::BlendedSprite { blend, .. } = icon_draw else {
        unreachable!("found by that pattern");
    };
    assert_eq!(
        *blend,
        Some(oag_vex::vex::BlendClass::AlphaOver),
        "TURBO_icon's own model declares this blend"
    );
    println!("{icon_draw:?}");
}

/// Pure's speed and shield gauge is three `<Mode3D>` models, `oag_title::BarModels`, and a race
/// draws them where the original's frames put them: the yellow outline's box at
/// `(309, 239)`-`(477, 252)` and the shield's at `(309, 255)`-`(474, 263)` on all four pairs in
/// `data/reference/compare-page/pure-psp/`, which this reproduces from the disc's own models.
///
/// Dropping `HudArt::bar_models` leaves no gauge at all, and this fails.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_speed_gauge_is_three_models_drawn_flat_where_the_original_draws_them() {
    use oag_ui::frontend::Draw;
    let Some(loaded) = single_race() else { return };
    let context = loaded.hud.context().expect("a context is buildable");
    let mut readout = race::Race::start(loaded.setup).readout();
    readout.shield_max = 100.0;
    readout.shield = 100.0;
    let rects = |speed: f32, shield: f32| -> Vec<[f32; 4]> {
        let mut readout = readout.clone();
        readout.speed_kmh = speed;
        readout.shield = shield;
        oag_hud::draw_list(&context, &readout)
            .sprites
            .into_iter()
            .filter_map(|draw| match draw {
                Draw::Sprite { rect, .. } if rect[1] >= 236.0 && rect[0] >= 266.0 => Some(rect),
                _ => None,
            })
            .collect()
    };

    // At a standstill with a full shield: the shield fill and the outline, no speed fill.
    let at_rest = rects(0.0, 100.0);
    assert_eq!(
        at_rest,
        vec![[309.0, 255.0, 166.0, 9.0], [266.0, 236.0, 256.0, 32.0]],
        "the gauge at a standstill: the shield's full fill under the outline, nothing for the speed"
    );
    // 350 km/h is half of the 700 the executable divides by; a half shield is half the fill.
    let half = rects(350.0, 50.0);
    assert_eq!(
        half,
        vec![
            [309.0, 239.0, 84.5, 14.0],
            [309.0, 255.0, 83.0, 9.0],
            [266.0, 236.0, 256.0, 32.0]
        ]
    );

    // The digits: `0` in `SpeedR` alone at rest, `1` over `23` split across the two at 123.
    let digits = |speed: f32| -> Vec<(String, f32)> {
        let mut readout = readout.clone();
        readout.speed_kmh = speed;
        oag_hud::draw_list(&context, &readout)
            .hud_text
            .into_iter()
            .filter_map(|draw| match draw {
                Draw::Text { text, x, .. } if (430.0..=440.0).contains(&x) => Some((text, x)),
                _ => None,
            })
            .collect()
    };
    assert_eq!(digits(0.0), vec![("0".to_string(), 436.0)]);
    let texts: Vec<String> = digits(123.4).into_iter().map(|(text, _)| text).collect();
    assert_eq!(texts, vec!["1".to_string(), "23".to_string()]);
}

/// Each weapon's icon is its own picture, drawn where and in the colour the original draws it.
///
/// Measured on PPSSPP (software renderer, Pure USA, Vineta K, each weapon poked into the held
/// slot, 2026-10-30): the pickup box sits at the top middle (the model's orthographic `y` counts
/// up), the glyphs differ per weapon, and the authored colour word is ABGR (rocket, missile and
/// disruptor draw orange, quake and plasma red, mine and bomb light blue, the rest green).
///
/// Before this the ten icons were one stretched 128x16 strip each, drawn as a plain rectangle
/// the size of the mesh's bounding box at the bottom middle, so they differed by tint alone.
/// Dropping `icon_srcs` in `oag_raceplay::hud::vex_model_art` leaves the strips and fails the
/// pairwise-mask check.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn each_held_weapon_draws_its_own_glyph_in_its_own_colour_at_the_top_middle() {
    use oag_tables::weapons::Weapon as W;
    use oag_ui::frontend::Draw;
    let Some(loaded) = single_race() else { return };
    let context = loaded.hud.context().expect("a context is buildable");
    let sheet = &loaded.hud.sheet;
    let weapons = [
        (W::Rocket, [1.0, 0.6745098, 0.2509804]),
        (W::Missile, [1.0, 0.6745098, 0.2509804]),
        (W::Quake, [1.0, 0.0, 0.0]),
        (W::Disruptor, [1.0, 0.6745098, 0.2509804]),
        (W::Turbo, [0.2509804, 1.0, 0.2509804]),
        (W::Shield, [0.2509804, 1.0, 0.2509804]),
        (W::Autopilot, [0.2509804, 1.0, 0.2509804]),
        (W::Plasma, [1.0, 0.0, 0.0]),
        (W::Mine, [0.2509804, 0.7529412, 1.0]),
        (W::Bomb, [0.2509804, 0.7529412, 1.0]),
    ];
    let mut masks: Vec<(W, Vec<bool>)> = Vec::new();
    for (weapon, rgb) in weapons {
        let mut race = race::Race::start(loaded.setup.clone());
        race.sim.world.ships[0].pickup.weapon = Some(weapon);
        race.tick(&PlayerInputs::none());
        let frame = oag_hud::draw_list(&context, &race.readout());
        let draws: Vec<_> = frame
            .sprites
            .iter()
            .filter_map(|d| match d {
                Draw::BlendedSprite {
                    rect, uv, color, ..
                } => Some((*rect, *uv, *color)),
                _ => None,
            })
            .collect();
        let (rect, uv, color) = *draws.last().expect("the icon draws after its grid");
        assert_eq!(&color[..3], &rgb, "{weapon:?}'s colour (ABGR word)");
        let (cx, cy) = (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5);
        assert!(
            (cx - 240.0).abs() < 8.0 && (cy - 22.0).abs() < 4.0,
            "{weapon:?} centred at ({cx}, {cy}); the original's box is at the top middle (240, 22), a glyph may sit off-centre in its canvas"
        );
        let mask: Vec<bool> = (0..uv[3] as usize)
            .flat_map(|row| (0..uv[2] as usize).map(move |col| (row, col)))
            .map(|(row, col)| {
                let at = ((uv[1] as usize + row) * sheet.width as usize + uv[0] as usize + col) * 4;
                sheet.rgba[at + 3] > 0
            })
            .collect();
        let lit = mask.iter().filter(|&&on| on).count();
        assert!(
            lit > 20 && lit < mask.len() * 9 / 10,
            "{weapon:?}'s icon is {lit} of {} texels: empty or a plain rectangle",
            mask.len()
        );
        masks.push((weapon, mask));
    }
    for (i, (a, ma)) in masks.iter().enumerate() {
        for (b, mb) in &masks[i + 1..] {
            assert!(ma != mb, "{a:?} and {b:?} draw the same icon");
        }
    }
}
