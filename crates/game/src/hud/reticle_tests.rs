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
    Context {
        default_border: layout.default_border(),
        layout,
        strings,
        sheet,
        art: oag_pulse::hud::ART,
        hud_line_height: 25.0,
        small_line_height: 10.0,
    }
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
