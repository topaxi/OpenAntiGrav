//! What the lock-on reticle in [`super::draw`] is asserted to put on screen.
//!
//! Its own file rather than more of `hud/tests.rs`, which is at 926 lines
//! against the 1,000-line cap in `scripts/check-file-size.py`.
//!
//! The law the reticle follows is asserted in `oag_race::sight`; this is only
//! about the five sprites it becomes - four instances of one model at four
//! quarter turns, and one of another.

use super::*;
use oag_race::sight;

/// The `<Mode3D>` block the disc authors for the sights, cut to the Missile's
/// five. Every attribute is as it appears in `Arcade_HUD.xml`, the shared
/// `Src` and the identical placeholder position included - the runtime writes
/// over the position every frame, which is why all nine carry the same one.
const SIGHTS: &str = r#"
<Screen>
<Screen name="HUD">
<Mode3D>
<Values mode="orthographic"></Values>
<Model name="missile_sight_inner">
<Values Src="Data\HUD\missile_sight_inner.vex" x="-240" y="136.0" z="0" ztest="0"></Values>
</Model>
<Model name="missile_sight_1">
<Values Src="Data\HUD\missile_sight_outer.vex" x="-240" y="136.0" z="0" ztest="0"></Values>
</Model>
<Model name="missile_sight_2">
<Values Src="Data\HUD\missile_sight_outer.vex" x="-240" y="136.0" z="0" ztest="0"></Values>
</Model>
<Model name="missile_sight_3">
<Values Src="Data\HUD\missile_sight_outer.vex" x="-240" y="136.0" z="0" ztest="0"></Values>
</Model>
<Model name="missile_sight_4">
<Values Src="Data\HUD\missile_sight_outer.vex" x="-240" y="136.0" z="0" ztest="0"></Values>
</Model>
</Mode3D>
</Screen>
</Screen>
"#;

/// `mode="orthographic"` on `SIGHTS`'s own block parses to the dialect
/// `bracket_draws`/`model_draw` assume - real HUD pixels, not the countdown's
/// perspective-centred `(0, 0)`. See `docs/ui/hud.md`'s "Two `<Mode3D>`
/// dialects" section.
#[test]
fn the_sight_bracket_dialect_reads_as_orthographic() {
    let layout = Layout::from_xml(SIGHTS);
    assert_eq!(layout.models.len(), 5);
    for model in &layout.models {
        assert!(model.orthographic, "{model:?}");
    }
}

/// A `<Model>` with no enclosing `<Mode3D>` at all - unreachable on any
/// shipped layout, where every `<Model>` lives inside one, but
/// `Layout::from_tree`'s own doc comment claims a specific fallback for it
/// rather than leaving the claim unchecked.
#[test]
fn a_model_outside_any_mode3d_takes_the_orthographic_default() {
    let layout = Layout::from_xml(
        r#"<Screen><Model name="Stray"><Values Src="Data\HUD\Stray.vex" x="1" y="2"/></Model></Screen>"#,
    );
    assert_eq!(layout.models.len(), 1);
    assert!(layout.models[0].orthographic);
    assert_eq!(layout.models[0].origin, [0.0, 0.0]);
}

/// The countdown's own block, verbatim off `Arcade_HUD.xml`: no `mode`
/// attribute at all, `FirstPass`/`OriginX`/`OriginY` instead - the opposite
/// end of the fork from [`SIGHTS`].
#[test]
fn the_countdown_dialect_reads_as_not_orthographic() {
    let layout = Layout::from_xml(
        r#"<Screen><Mode3D>
           <Values FirstPass="yes" OriginX="0.0" OriginY="35.0"></Values>
           <Model name="Cockpit321Go"><Values Src="Data\HUD\Cockpit_321GO.vex" x="0.0" y="0.0" z="-70.0"/></Model>
           </Mode3D></Screen>"#,
    );
    assert_eq!(layout.models.len(), 1);
    assert!(!layout.models[0].orthographic);
    assert_eq!(layout.models[0].origin, [0.0, 35.0]);
}

/// Pure's weapon-icon block also declares `mode="orthographic"` on the real
/// disc - `model_draw`'s dialect, not the countdown's, despite being a
/// per-title `<Model>` extension the sights don't use.
#[test]
fn pures_weapon_icon_dialect_reads_as_orthographic_too() {
    let layout = Layout::from_xml(
        r#"<Screen><Mode3D>
           <Values mode="orthographic"></Values>
           <Model name="TURBO_icon"><Values Src="Data\HUD\Weapon_turbo.vex" colour="0xff40ff40" x="240" y="250" z="-10"/></Model>
           </Mode3D></Screen>"#,
    );
    assert_eq!(layout.models.len(), 1);
    assert!(layout.models[0].orthographic);
}

/// A sheet holding the two sight models' art, the way the loader packs it.
fn sheet() -> crate::sprite::Sheet {
    let mut report = Vec::new();
    crate::sprite::Sheet::build_with(
        &[],
        vec![
            crate::sprite::DecodedImage {
                src: r"Data\HUD\missile_sight_outer.vex".to_string(),
                width: 8,
                height: 8,
                rgba: vec![255u8; 8 * 8 * 4],
                quad_extent: None,
                blend: None,
            },
            crate::sprite::DecodedImage {
                src: r"Data\HUD\missile_sight_inner.vex".to_string(),
                width: 8,
                height: 8,
                rgba: vec![255u8; 8 * 8 * 4],
                quad_extent: None,
                blend: None,
            },
        ],
        &mut report,
    )
}

fn strings() -> oag_ui::language::StringTable {
    oag_ui::language::StringTable::default()
}

fn context<'a>(
    layout: &'a Layout,
    strings: &'a oag_ui::language::StringTable,
    sheet: &'a crate::sprite::Sheet,
) -> Context<'a> {
    context_with(layout, strings, sheet, oag_pulse::hud::ART)
}

fn context_with<'a>(
    layout: &'a Layout,
    strings: &'a oag_ui::language::StringTable,
    sheet: &'a crate::sprite::Sheet,
    art: &'static oag_title::HudArt,
) -> Context<'a> {
    Context {
        default_border: layout.default_border(),
        layout,
        strings,
        sheet,
        art,
        hud_line_height: 25.0,
        small_line_height: 10.0,
        default_line_height: 10.0,
    }
}

/// The shape Wipeout HD authors its reticle in, cut to four widgets.
///
/// Concentric `<Image>` sprites rather than `<Mode3D>` models, at the sizes and
/// the colours the real layout carries - a red outer and a green inner - and all
/// centred on `(-960, 540)`, which is the negated centre of HD's own 1920x1080
/// screen and the same placeholder idiom the PSP titles use at `(-240, 136)`.
const HD_SIGHTS: &str = r#"
<Screen>
<Screen name="HUD">
<Image name="MissileSightBG">
<Values x="-1024" y="476" width="128" height="128" U="0" V="0" TxtrWidth="128" TxtrHeight="128" Color="0xFFFFFFFF" Src="missile_reticule.gtf"></Values>
</Image>
<Image name="MissileSightOuter">
<Values x="-1024" y="476" width="128" height="128" U="0" V="128" TxtrWidth="128" TxtrHeight="128" Color="0xFFFF0000" Src="missile_reticule.gtf"></Values>
</Image>
<Image name="MissileSightLockedOnLines">
<Values x="-1024" y="476" width="128" height="128" U="128" V="128" TxtrWidth="128" TxtrHeight="128" Color="0xFFFFFFFF" Src="missile_reticule.gtf"></Values>
</Image>
<Image name="MissileSightLockedOnMiddle">
<Values x="-992" y="508" width="64" height="64" U="0" V="0" TxtrWidth="128" TxtrHeight="128" Color="0xFFFF0000" Src="missile_reticule.gtf"></Values>
</Image>
</Screen>
</Screen>
"#;

/// Wipeout HD's art rows, with only the sight axis filled in for real.
static HD_ART: oag_title::HudArt = oag_title::HudArt {
    texture_extension: None,
    always_on: &[],
    raster: false,
    sights: &oag_title::hud::Sights::Concentric {
        seeking: &["MissileSightBG", "MissileSightOuter"],
        locked: &["MissileSightLockedOnLines", "MissileSightLockedOnMiddle"],
        leach: None,
    },
    pickup_backdrop_colour: None,
    pickup_colours: None,
    pickup_icon_models: None,
    pickup_icon_backdrop_model: None,
    pickup_icon_uv: None,
    zone_speed_classes: None,
    shield_percent: false,
    hud_font_role: "HUD",
    hud_small_font_role: Some("HUDSmall"),
    total_time_timed_modes_only: false,
    kill_column: false,
    message_slots: false,
    runtime: None,
};

fn hd_sheet() -> crate::sprite::Sheet {
    let mut report = Vec::new();
    crate::sprite::Sheet::build_with(
        &[],
        vec![crate::sprite::DecodedImage {
            src: "missile_reticule.gtf".to_string(),
            width: 256,
            height: 256,
            rgba: vec![255u8; 256 * 256 * 4],
            quad_extent: None,
            blend: None,
        }],
        &mut report,
    )
}

/// HD's LeachBeam reticle, cut to its own four - the real names, off the real
/// widths a composed `Arcade_HUD.xml` carries (`oag_hd::hud::ART`'s own doc
/// comment). Unlike [`HD_SIGHTS`] there is no `LockedOn` pair: neither this
/// fixture nor the real disc authors one for the LeachBeam.
const HD_LEACH_SIGHTS: &str = r#"
<Screen>
<Screen name="HUD">
<Image name="LeachBeamSightBG">
<Values x="-1048" y="452" width="176" height="176" U="0" V="0" TxtrWidth="256" TxtrHeight="256" Color="0xFFFFFFFF" Src="leach_reticule.gtf"></Values>
</Image>
<Image name="LeachBeamSightOuter">
<Values x="-1048" y="452" width="176" height="176" U="0" V="0" TxtrWidth="256" TxtrHeight="256" Color="0xFFFFFFFF" Src="leach_reticule.gtf"></Values>
</Image>
<Image name="LeachBeamSightMiddle">
<Values x="-1024" y="476" width="128" height="128" U="0" V="0" TxtrWidth="256" TxtrHeight="256" Color="0xFFFFFFFF" Src="leach_reticule.gtf"></Values>
</Image>
<Image name="LeachBeamSightInner">
<Values x="-1000" y="500" width="80" height="80" U="0" V="0" TxtrWidth="256" TxtrHeight="256" Color="0xFFFFFFFF" Src="leach_reticule.gtf"></Values>
</Image>
</Screen>
</Screen>
"#;

/// [`HD_ART`] with a `leach` set filled in, the shape `oag_hd::hud::ART` and
/// `oag_2048::hud::ART` both carry as of 2026-09-15.
static HD_LEACH_ART: oag_title::HudArt = oag_title::HudArt {
    texture_extension: None,
    always_on: &[],
    raster: false,
    sights: &oag_title::hud::Sights::Concentric {
        seeking: &["MissileSightBG", "MissileSightOuter"],
        locked: &["MissileSightLockedOnLines", "MissileSightLockedOnMiddle"],
        leach: Some([
            "LeachBeamSightBG",
            "LeachBeamSightOuter",
            "LeachBeamSightMiddle",
            "LeachBeamSightInner",
        ]),
    },
    pickup_backdrop_colour: None,
    pickup_colours: None,
    pickup_icon_models: None,
    pickup_icon_backdrop_model: None,
    pickup_icon_uv: None,
    zone_speed_classes: None,
    shield_percent: false,
    hud_font_role: "HUD",
    hud_small_font_role: Some("HUDSmall"),
    total_time_timed_modes_only: false,
    kill_column: false,
    message_slots: false,
    runtime: None,
};

fn hd_leach_sheet() -> crate::sprite::Sheet {
    let mut report = Vec::new();
    crate::sprite::Sheet::build_with(
        &[],
        vec![crate::sprite::DecodedImage {
            src: "leach_reticule.gtf".to_string(),
            width: 256,
            height: 256,
            rgba: vec![255u8; 256 * 256 * 4],
            quad_extent: None,
            blend: None,
        }],
        &mut report,
    )
}

fn plain(frame: &Frame) -> Vec<([f32; 4], [f32; 4])> {
    frame
        .sprites
        .iter()
        .filter_map(|draw| match draw {
            Draw::Sprite { rect, color, .. } => Some((*rect, *color)),
            _ => None,
        })
        .collect()
}

/// A reticle that has been holding a target long enough to have locked it.
fn locked_on(screen: [f32; 2]) -> sight::Sight {
    let mut reticle = sight::Sight::default();
    let target = sight::Projected {
        screen,
        distance: 60.0,
    };
    for _ in 0..180 {
        reticle.update(1.0 / 60.0, Some(target));
    }
    reticle
}

/// A reticle that has a target on screen but has not held it long enough.
fn seeking(screen: [f32; 2]) -> sight::Sight {
    let mut reticle = sight::Sight::default();
    let target = sight::Projected {
        screen,
        distance: 60.0,
    };
    // Well inside the 0.8 s hold, so it is still closing.
    for _ in 0..12 {
        assert_eq!(
            reticle.update(1.0 / 60.0, Some(target)),
            sight::State::Seeking
        );
    }
    reticle
}

/// One more frame of the same target, which is how the blink phase is advanced.
fn one_more(reticle: &mut sight::Sight, screen: [f32; 2]) {
    reticle.update(
        1.0 / 60.0,
        Some(sight::Projected {
            screen,
            distance: 60.0,
        }),
    );
}

fn colours(frame: &Frame) -> Vec<[f32; 4]> {
    frame
        .sprites
        .iter()
        .filter_map(|draw| match draw {
            Draw::BlendedSprite { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

fn rotated(frame: &Frame) -> Vec<(f32, f32, f32)> {
    frame
        .sprites
        .iter()
        .filter_map(|draw| match draw {
            Draw::BlendedSprite { rect, rotation, .. } => Some((rect[0], rect[1], *rotation)),
            _ => None,
        })
        .collect()
}

/// A locked reticle draws five pieces: four brackets and the inner box.
#[test]
fn a_locked_reticle_draws_its_four_brackets_and_its_inner_box() {
    let layout = Layout::from_xml(SIGHTS);
    let sheet = sheet();
    let strings = strings();
    let readout = Readout {
        sight: Some(locked_on([240.0, 136.0])),
        ..Readout::blank()
    };

    let frame = draw_list(&context(&layout, &strings, &sheet), &readout);
    let pieces = rotated(&frame);
    assert_eq!(pieces.len(), 5, "{frame:?}");

    // The four brackets carry the four quarter turns; the inner carries none.
    let mut turns: Vec<f32> = pieces.iter().map(|&(_, _, r)| r).collect();
    turns.sort_by(f32::total_cmp);
    let quarter = std::f32::consts::FRAC_PI_2;
    // Zero twice - the inner box and the bracket the original writes `0` to.
    assert!((turns[0] - 0.0).abs() < 1e-4, "{turns:?}");
    assert!((turns[1] - 0.0).abs() < 1e-4, "{turns:?}");
    assert!((turns[2] - quarter).abs() < 1e-4, "{turns:?}");
    assert!((turns[3] - 2.0 * quarter).abs() < 1e-4, "{turns:?}");
    assert!((turns[4] - 3.0 * quarter).abs() < 1e-4, "{turns:?}");
}

/// The brackets straddle the target, and the whole reticle sits over it.
///
/// The check that the piece centres are used as centres rather than as
/// top-left corners: a quad placed by its corner would put the whole reticle
/// half a bracket down and to the right of the craft.
#[test]
fn the_reticle_is_centred_on_the_craft_it_locked() {
    let layout = Layout::from_xml(SIGHTS);
    let sheet = sheet();
    let strings = strings();
    let at = [180.0, 100.0];
    let readout = Readout {
        sight: Some(locked_on(at)),
        ..Readout::blank()
    };

    let frame = draw_list(&context(&layout, &strings, &sheet), &readout);
    let pieces = rotated(&frame);
    assert_eq!(pieces.len(), 5);

    let xs: Vec<f32> = pieces.iter().map(|&(x, _, _)| x).collect();
    let ys: Vec<f32> = pieces.iter().map(|&(_, y, _)| y).collect();
    let mid_x = (xs.iter().cloned().fold(f32::MAX, f32::min)
        + xs.iter().cloned().fold(f32::MIN, f32::max))
        * 0.5;
    let mid_y = (ys.iter().cloned().fold(f32::MAX, f32::min)
        + ys.iter().cloned().fold(f32::MIN, f32::max))
        * 0.5;
    // Plus half a sprite, because a rect's `x`/`y` is its top-left.
    assert!((mid_x + 4.0 - at[0]).abs() < 1.0, "{pieces:?}");
    assert!((mid_y + 4.0 - at[1]).abs() < 1.0, "{pieces:?}");
}

/// A reticle with nothing to lock draws nothing at all.
#[test]
fn an_idle_reticle_draws_nothing() {
    let layout = Layout::from_xml(SIGHTS);
    let sheet = sheet();
    let strings = strings();
    let mut reticle = sight::Sight::default();
    for _ in 0..180 {
        reticle.update(1.0 / 60.0, None);
    }
    let readout = Readout {
        sight: Some(reticle),
        ..Readout::blank()
    };

    let frame = draw_list(&context(&layout, &strings, &sheet), &readout);
    assert!(rotated(&frame).is_empty(), "{frame:?}");
}

/// A layout that authors no sight widgets draws no reticle, and does not panic.
///
/// `TimeTrial_HUD.xml` and `Zone_HUD.xml` are that case on the shipped disc:
/// two `<Mode3D>` models apiece, both the countdown's.
#[test]
fn a_layout_with_no_sight_widgets_draws_no_reticle() {
    let layout = Layout::from_xml(
        r#"<Screen><Screen name="HUD"><Mode3D>
           <Model name="ReadyGo"><Values Src="Data\HUD\Pulse_Ready_Go.vex" x="0" y="0"/></Model>
           </Mode3D></Screen></Screen>"#,
    );
    let sheet = sheet();
    let strings = strings();
    let readout = Readout {
        sight: Some(locked_on([240.0, 136.0])),
        ..Readout::blank()
    };

    let frame = draw_list(&context(&layout, &strings, &sheet), &readout);
    assert!(rotated(&frame).is_empty(), "{frame:?}");
}

/// And a sheet whose sight art did not decode draws nothing rather than a
/// substitute.
///
/// The rule `CLAUDE.md` states for every authored asset: an effect that will not
/// load draws nothing and the loader report says why. A visible stand-in for a
/// lock-on reticle would be exactly the plausible-looking invention that rule
/// exists to keep out.
#[test]
fn a_sight_model_that_did_not_decode_draws_nothing() {
    let layout = Layout::from_xml(SIGHTS);
    let empty = crate::sprite::Sheet::default();
    let strings = strings();
    let readout = Readout {
        sight: Some(locked_on([240.0, 136.0])),
        ..Readout::blank()
    };

    let frame = draw_list(&context(&layout, &strings, &empty), &readout);
    assert!(rotated(&frame).is_empty(), "{frame:?}");
}

/// A seeking reticle is drawn on **both** halves of its blink, in two tints.
///
/// **The blink is a tint and never a hide**, and this test exists because the
/// first version of the draw got that backwards and strobed the reticle off
/// every 0.1 s. `HudSight_Update` writes a colour on both phases -
/// `value | 0xff000000` on one and `value * 0xc0 >> 8` in all three channels on
/// the other - and drops the draw on neither.
///
/// Every other test in this file uses a *locked* reticle, whose blink is not
/// spent at all, which is exactly why none of them noticed.
#[test]
fn a_seeking_reticle_is_drawn_on_both_halves_of_its_blink() {
    let layout = Layout::from_xml(SIGHTS);
    let sheet = sheet();
    let strings = strings();
    let at = [240.0, 136.0];

    let mut reticle = seeking(at);
    let mut seen: Vec<f32> = Vec::new();
    // A blink half is 0.1 s, so a fifth of a second covers both of them twice.
    for _ in 0..12 {
        let readout = Readout {
            sight: Some(reticle),
            ..Readout::blank()
        };
        let frame = draw_list(&context(&layout, &strings, &sheet), &readout);
        let drawn = colours(&frame);
        assert_eq!(
            drawn.len(),
            5,
            "a seeking reticle drew {} pieces on one frame, not five - the \
             blink is hiding it rather than tinting it",
            drawn.len()
        );
        let tint = drawn[0][0];
        assert!(
            drawn.iter().all(|c| (c[0] - tint).abs() < 1e-6),
            "the five pieces disagree about their tint: {drawn:?}"
        );
        if !seen.iter().any(|&s| (s - tint).abs() < 1e-6) {
            seen.push(tint);
        }
        one_more(&mut reticle, at);
    }

    seen.sort_by(f32::total_cmp);
    assert_eq!(
        seen.len(),
        2,
        "the seeking blink produced {} tint(s), not two: {seen:?}",
        seen.len()
    );
    // The recovered pair: the full value, and it scaled by `0xc0 >> 8`.
    assert!((seen[0] - 0.75).abs() < 1e-6, "{seen:?}");
    assert!((seen[1] - 1.0).abs() < 1e-6, "{seen:?}");
}

/// A locked reticle stops blinking and holds one tint.
#[test]
fn a_locked_reticle_holds_one_tint() {
    let layout = Layout::from_xml(SIGHTS);
    let sheet = sheet();
    let strings = strings();
    let at = [240.0, 136.0];

    let mut reticle = locked_on(at);
    for _ in 0..24 {
        let readout = Readout {
            sight: Some(reticle),
            ..Readout::blank()
        };
        let frame = draw_list(&context(&layout, &strings, &sheet), &readout);
        let drawn = colours(&frame);
        assert_eq!(drawn.len(), 5);
        assert!(
            (drawn[0][0] - 1.0).abs() < 1e-6,
            "a locked reticle dimmed to {}, so the blink is still being spent",
            drawn[0][0]
        );
        one_more(&mut reticle, at);
    }
}

/// The PSP bracket dialect's recovered hue: red once locked, and yellow
/// alternating with white while seeking.
///
/// **Recovered, confidence 88** - `HudSight_Update`'s packed colour word, read
/// against this project's own established byte order for it (byte0 = R,
/// byte1 = G, byte2 = B, byte3 = A - the same one `Loading_DrawWave`'s
/// `0xff000000` -> `0xff808080` ramp already established). See
/// `oag_race::sight::Sight::tint` and
/// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md#colour-and-blink-resolved-the-byte-order-and-the-two-tints`.
#[test]
fn the_bracket_dialect_tints_locked_red_and_seeking_yellow_and_white() {
    let layout = Layout::from_xml(SIGHTS);
    let sheet = sheet();
    let strings = strings();
    let at = [240.0, 136.0];

    let locked = locked_on(at);
    let readout = Readout {
        sight: Some(locked),
        ..Readout::blank()
    };
    let frame = draw_list(&context(&layout, &strings, &sheet), &readout);
    let drawn = colours(&frame);
    assert_eq!(drawn.len(), 5);
    for c in &drawn {
        assert!(
            c[0] > 0.5 && c[1] < 1e-6 && c[2] < 1e-6,
            "a locked reticle should be red, not {c:?}"
        );
    }

    let mut reticle = seeking(at);
    let mut seen_yellow = false;
    let mut seen_white = false;
    for _ in 0..12 {
        let readout = Readout {
            sight: Some(reticle),
            ..Readout::blank()
        };
        let frame = draw_list(&context(&layout, &strings, &sheet), &readout);
        let drawn = colours(&frame);
        assert_eq!(drawn.len(), 5);
        let c = drawn[0];
        assert!(
            drawn.iter().all(|other| other == &c),
            "the five pieces disagree about their colour: {drawn:?}"
        );
        if c[2] < 1e-6 {
            // Blue channel unlit: yellow, at full brightness.
            assert!(
                (c[0] - 1.0).abs() < 1e-6 && (c[1] - 1.0).abs() < 1e-6,
                "{c:?}"
            );
            seen_yellow = true;
        } else {
            // All three channels lit equally: white, dimmed.
            assert!(
                (c[0] - c[1]).abs() < 1e-6 && (c[1] - c[2]).abs() < 1e-6,
                "the dim phase should be neutral grey, not {c:?}"
            );
            seen_white = true;
        }
        one_more(&mut reticle, at);
    }
    assert!(
        seen_yellow && seen_white,
        "the seeking blink should show both recovered tints"
    );
}

/// Wipeout HD's dialect: concentric sprites at one centre, no rotation.
///
/// **The layout carries the geometry here and the law carries the centre.** Each
/// widget keeps its authored size and its authored colour - HD authors a red
/// outer where the PSP titles author one white bracket four times - and only the
/// position is the reticle's.
#[test]
fn the_concentric_dialect_centres_each_sprite_at_its_authored_size() {
    let layout = Layout::from_xml(HD_SIGHTS);
    let sheet = hd_sheet();
    let strings = strings();
    let at = [700.0, 400.0];

    let mut reticle = sight::Sight::new([1920.0, 1080.0]);
    let target = sight::Projected {
        screen: at,
        distance: 60.0,
    };
    for _ in 0..12 {
        reticle.update(1.0 / 60.0, Some(target));
    }
    assert!(
        !reticle.locked(),
        "the fixture was meant to still be seeking"
    );

    let readout = Readout {
        sight: Some(reticle),
        ..Readout::blank()
    };
    let frame = draw_list(&context_with(&layout, &strings, &sheet, &HD_ART), &readout);

    // Seeking: the two seeking widgets and neither locked one.
    let drawn = plain(&frame);
    assert_eq!(drawn.len(), 2, "{frame:?}");
    let centre = reticle.centre();
    for (rect, colour) in &drawn {
        assert!(
            (rect[0] + rect[2] * 0.5 - centre[0]).abs() < 0.01
                && (rect[1] + rect[3] * 0.5 - centre[1]).abs() < 0.01,
            "{rect:?} is not centred on {centre:?}"
        );
        assert!(
            (rect[2] - 128.0).abs() < 0.01,
            "{rect:?} is not the authored 128 wide"
        );
        assert!(colour[3] > 0.0);
    }
    // The authored red survives: one of the two is not white.
    assert!(
        drawn.iter().any(|(_, c)| c[1] < 0.5 && c[0] > 0.5),
        "HD's authored red outer came out white: {drawn:?}"
    );

    // Nothing rotates in this dialect.
    assert!(rotated(&frame).is_empty(), "{frame:?}");
}

/// And the locked pair is added once the lock is taken.
#[test]
fn the_concentric_dialect_adds_its_locked_widgets_on_the_lock() {
    let layout = Layout::from_xml(HD_SIGHTS);
    let sheet = hd_sheet();
    let strings = strings();
    let at = [700.0, 400.0];

    let mut reticle = sight::Sight::new([1920.0, 1080.0]);
    let target = sight::Projected {
        screen: at,
        distance: 60.0,
    };
    for _ in 0..600 {
        if reticle.update(1.0 / 60.0, Some(target)) == sight::State::Locked {
            break;
        }
    }
    assert!(reticle.locked(), "the reticle never locked");

    let readout = Readout {
        sight: Some(reticle),
        ..Readout::blank()
    };
    let frame = draw_list(&context_with(&layout, &strings, &sheet, &HD_ART), &readout);
    assert_eq!(
        plain(&frame).len(),
        4,
        "a locked HD reticle draws the seeking pair and the locked pair: {frame:?}"
    );
}

/// A held LeachBeam on the concentric dialect draws its own backdrop and
/// current ring while seeking, not the Missile's `MissileSight*` set and not
/// all four of its own widgets at once.
///
/// See `oag_title::hud::Sights::Concentric::leach`'s own doc for the measured
/// law this ports: `Hud_UpdateLeachBeamSight` reveals Outer, Middle and Inner
/// one at a time, one per quarter of the hold window - checked in full by
/// [`the_concentric_dialect_reveals_the_leachbeams_rings_one_quarter_at_a_time`]
/// below. This test only checks that the *set* is the LeachBeam's own, not
/// the Missile's - `HD_LEACH_SIGHTS` authors no `MissileSight*` widget at all,
/// so a wrong set here would simply draw nothing.
#[test]
fn the_concentric_dialect_draws_the_leachbeams_own_widgets_not_the_missiles() {
    let layout = Layout::from_xml(HD_LEACH_SIGHTS);
    let sheet = hd_leach_sheet();
    let strings = strings();
    let at = [700.0, 400.0];

    let mut reticle = sight::Sight::new([1920.0, 1080.0]);
    reticle.set_held(sight::Held::LeachBeam);
    let target = sight::Projected {
        screen: at,
        distance: 60.0,
    };
    reticle.update(1.0 / 60.0, Some(target));
    assert!(
        !reticle.locked(),
        "the fixture was meant to still be seeking"
    );

    let readout = Readout {
        sight: Some(reticle),
        ..Readout::blank()
    };
    let frame = draw_list(
        &context_with(&layout, &strings, &sheet, &HD_LEACH_ART),
        &readout,
    );
    let drawn = plain(&frame);
    // The backdrop plus whichever ring owns the first quarter - see the test
    // below for exactly which.
    assert_eq!(drawn.len(), 2, "{frame:?}");
    let centre = reticle.centre();
    for (rect, _) in &drawn {
        assert!(
            (rect[0] + rect[2] * 0.5 - centre[0]).abs() < 0.01
                && (rect[1] + rect[3] * 0.5 - centre[1]).abs() < 0.01,
            "{rect:?} is not centred on {centre:?}"
        );
    }
    assert!(rotated(&frame).is_empty(), "{frame:?}");
}

/// HD's own LeachBeam reticle reveals one ring at a time as the hold
/// progresses - the backdrop plus whichever of Outer/Middle/Inner owns the
/// current quarter of [`oag_race::sight::Sight::hold_progress`] - rather than
/// showing all four together, and shows the backdrop alone for the last
/// quarter and once locked.
///
/// **Recovered mechanism, ported at reduced confidence** -
/// `docs/ghidra/functions/ps3-hdfury-eu/hud-sight.md`'s
/// `Hud_UpdateLeachBeamSight` reading, scaled onto this engine's own
/// `HOLD_SECONDS` rather than HD's own `0.5` s. `HD_LEACH_SIGHTS` gives Outer,
/// Middle and Inner distinct authored widths (176/128/80) so each stage is
/// identified by its rect rather than by a widget name `plain()` does not
/// expose.
#[test]
fn the_concentric_dialect_reveals_the_leachbeams_rings_one_quarter_at_a_time() {
    let layout = Layout::from_xml(HD_LEACH_SIGHTS);
    let sheet = hd_leach_sheet();
    let strings = strings();
    let at = [700.0, 400.0];

    let mut reticle = sight::Sight::new([1920.0, 1080.0]);
    reticle.set_held(sight::Held::LeachBeam);
    let target = sight::Projected {
        screen: at,
        distance: 60.0,
    };

    let draw = |reticle: &sight::Sight| -> Vec<([f32; 4], [f32; 4])> {
        let readout = Readout {
            sight: Some(*reticle),
            ..Readout::blank()
        };
        plain(&draw_list(
            &context_with(&layout, &strings, &sheet, &HD_LEACH_ART),
            &readout,
        ))
    };

    // The first frame: zero hold is still inside the first quarter, so the
    // backdrop (176 wide) plus the Outer ring (also 176 wide in this fixture,
    // so the two are indistinguishable by rect alone - the count is the check).
    reticle.update(1.0 / 60.0, Some(target));
    let drawn = draw(&reticle);
    assert_eq!(
        drawn.len(),
        2,
        "hold_progress {}: {drawn:?}",
        reticle.hold_progress()
    );
    assert!(
        drawn.iter().all(|(rect, _)| (rect[2] - 176.0).abs() < 0.01),
        "the backdrop and the Outer ring are both authored at 176 wide: {drawn:?}"
    );

    // Past the first quarter: the Middle ring (128 wide) replaces Outer.
    while reticle.hold_progress() <= 0.25 {
        reticle.update(1.0 / 60.0, Some(target));
    }
    let drawn = draw(&reticle);
    assert_eq!(
        drawn.len(),
        2,
        "hold_progress {}: {drawn:?}",
        reticle.hold_progress()
    );
    assert!(
        drawn.iter().any(|(rect, _)| (rect[2] - 128.0).abs() < 0.01),
        "expected the Middle ring (128 wide) at hold_progress {}: {drawn:?}",
        reticle.hold_progress()
    );

    // Past the half mark: the Inner ring (80 wide).
    while reticle.hold_progress() <= 0.5 {
        reticle.update(1.0 / 60.0, Some(target));
    }
    let drawn = draw(&reticle);
    assert_eq!(
        drawn.len(),
        2,
        "hold_progress {}: {drawn:?}",
        reticle.hold_progress()
    );
    assert!(
        drawn.iter().any(|(rect, _)| (rect[2] - 80.0).abs() < 0.01),
        "expected the Inner ring (80 wide) at hold_progress {}: {drawn:?}",
        reticle.hold_progress()
    );

    // Past three-quarters of the hold, only the backdrop shows - the
    // original's own table stops there, and this project draws nothing
    // invented for what (if anything) comes after.
    while reticle.hold_progress() <= 0.75 {
        reticle.update(1.0 / 60.0, Some(target));
    }
    let drawn = draw(&reticle);
    assert_eq!(
        drawn.len(),
        1,
        "past three-quarters of the hold only the backdrop should show: {drawn:?}"
    );
}

/// The LeachBeam reticle still shows only the backdrop once the lock is
/// actually taken - there is no fifth, "locked" widget to add, unlike the
/// Missile's set, and the last pre-lock quarter already dropped to backdrop
/// alone (see the test above).
#[test]
fn the_concentric_dialect_shows_only_the_backdrop_once_the_leachbeam_locks() {
    let layout = Layout::from_xml(HD_LEACH_SIGHTS);
    let sheet = hd_leach_sheet();
    let strings = strings();
    let at = [700.0, 400.0];

    let mut reticle = sight::Sight::new([1920.0, 1080.0]);
    reticle.set_held(sight::Held::LeachBeam);
    let target = sight::Projected {
        screen: at,
        distance: 60.0,
    };
    for _ in 0..600 {
        if reticle.update(1.0 / 60.0, Some(target)) == sight::State::Locked {
            break;
        }
    }
    assert!(reticle.locked(), "the reticle never locked");

    let readout = Readout {
        sight: Some(reticle),
        ..Readout::blank()
    };
    let frame = draw_list(
        &context_with(&layout, &strings, &sheet, &HD_LEACH_ART),
        &readout,
    );
    assert_eq!(
        plain(&frame).len(),
        1,
        "a locked LeachBeam reticle draws only its backdrop: {frame:?}"
    );
}

/// A title with no `leach` set (Pure-shaped, or any Concentric title that has
/// not authored the widgets) draws nothing for a held LeachBeam rather than
/// lending it the Missile's rings.
#[test]
fn the_concentric_dialect_draws_nothing_for_a_leachbeam_with_no_leach_set() {
    let layout = Layout::from_xml(HD_SIGHTS);
    let sheet = hd_sheet();
    let strings = strings();
    let at = [700.0, 400.0];

    let mut reticle = sight::Sight::new([1920.0, 1080.0]);
    reticle.set_held(sight::Held::LeachBeam);
    let target = sight::Projected {
        screen: at,
        distance: 60.0,
    };
    for _ in 0..600 {
        reticle.update(1.0 / 60.0, Some(target));
    }

    let readout = Readout {
        sight: Some(reticle),
        ..Readout::blank()
    };
    // HD_ART's `leach` is `None` in this fixture.
    let frame = draw_list(&context_with(&layout, &strings, &sheet, &HD_ART), &readout);
    assert!(
        plain(&frame).is_empty() && rotated(&frame).is_empty(),
        "{frame:?}"
    );
}

/// A title whose reticle has not been read draws nothing rather than guessing.
#[test]
fn an_unread_sight_dialect_draws_nothing() {
    static UNREAD: oag_title::HudArt = oag_title::HudArt {
        texture_extension: None,
        always_on: &[],
        raster: false,
        sights: &oag_title::hud::Sights::Unread,
        pickup_backdrop_colour: None,
        pickup_colours: None,
        pickup_icon_models: None,
        pickup_icon_backdrop_model: None,
        pickup_icon_uv: None,
        zone_speed_classes: None,
        shield_percent: true,
        hud_font_role: "HUD",
        hud_small_font_role: Some("HUDSmall"),
        total_time_timed_modes_only: false,
        kill_column: false,
        message_slots: false,
        runtime: None,
    };

    let layout = Layout::from_xml(SIGHTS);
    let sheet = sheet();
    let strings = strings();
    let readout = Readout {
        sight: Some(locked_on([240.0, 136.0])),
        ..Readout::blank()
    };

    let frame = draw_list(&context_with(&layout, &strings, &sheet, &UNREAD), &readout);
    assert!(
        rotated(&frame).is_empty() && plain(&frame).is_empty(),
        "{frame:?}"
    );
}
