//! What the lock-on reticle in [`super::draw`] is asserted to put on screen.
//!
//! Its own file rather than more of `hud/tests.rs`, which is at 926 lines
//! against the 1,000-line cap in `scripts/check-file-size.py`.
//!
//! The law the reticle follows is asserted in `crate::race::sight`; this is only
//! about the five sprites it becomes - four instances of one model at four
//! quarter turns, and one of another.

use super::*;
use crate::race::sight;

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

/// A sheet holding the two sight models' art, the way the loader packs it.
fn sheet() -> crate::sprite::Sheet {
    let mut report = Vec::new();
    crate::sprite::Sheet::build_with(
        &[],
        vec![
            (
                r"Data\HUD\missile_sight_outer.vex".to_string(),
                8,
                8,
                vec![255u8; 8 * 8 * 4],
            ),
            (
                r"Data\HUD\missile_sight_inner.vex".to_string(),
                8,
                8,
                vec![255u8; 8 * 8 * 4],
            ),
        ],
        &mut report,
    )
}

fn strings() -> crate::language::StringTable {
    crate::language::StringTable::default()
}

fn context<'a>(
    layout: &'a Layout,
    strings: &'a crate::language::StringTable,
    sheet: &'a crate::sprite::Sheet,
) -> Context<'a> {
    context_with(layout, strings, sheet, oag_pulse::hud::ART)
}

fn context_with<'a>(
    layout: &'a Layout,
    strings: &'a crate::language::StringTable,
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
    sights: &oag_title::hud::Sights::Concentric {
        seeking: &["MissileSightBG", "MissileSightOuter"],
        locked: &["MissileSightLockedOnLines", "MissileSightLockedOnMiddle"],
    },
    pickup_backdrop_colour: None,
    pickup_colours: None,
    zone_speed_classes: None,
};

fn hd_sheet() -> crate::sprite::Sheet {
    let mut report = Vec::new();
    crate::sprite::Sheet::build_with(
        &[],
        vec![(
            "missile_reticule.gtf".to_string(),
            256,
            256,
            vec![255u8; 256 * 256 * 4],
        )],
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
            Draw::RotatedSprite { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

fn rotated(frame: &Frame) -> Vec<(f32, f32, f32)> {
    frame
        .sprites
        .iter()
        .filter_map(|draw| match draw {
            Draw::RotatedSprite { rect, rotation, .. } => Some((rect[0], rect[1], *rotation)),
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

/// A title whose reticle has not been read draws nothing rather than guessing.
#[test]
fn an_unread_sight_dialect_draws_nothing() {
    static UNREAD: oag_title::HudArt = oag_title::HudArt {
        texture_extension: None,
        always_on: &[],
        sights: &oag_title::hud::Sights::Unread,
        pickup_backdrop_colour: None,
        pickup_colours: None,
        zone_speed_classes: None,
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
