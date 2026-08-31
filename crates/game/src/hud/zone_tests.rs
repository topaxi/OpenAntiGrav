//! Zone's ladder: the `RotationTheta` the column and its ticks are authored
//! with, and the `ZonePlus<N>` arithmetic that numbers the rows.
//!
//! Its own file rather than more of [`super::tests`], which is at the
//! 1,000-line ceiling `scripts/check-file-size.py` enforces - the same reason
//! [`super::reticle_tests`] is one. Everything here is title-blind and needs no
//! disc; `crates/game/tests/hd_hud_ground_truth.rs` is where the same rules meet
//! HD's real layout and the reference frame.

use super::draw::*;
use super::*;

/// A `RotationTheta` is read off the widget and reaches the draw as a rotated
/// quad, in either of the two spellings the disc ships.
///
/// The Zone layout writes `rotationTheta` on `ZoneBG` and `RotationTheta` on the
/// ticks beside it; `Node::value` folds case, and this is the check that keeps
/// it folding.
#[test]
fn a_rotation_is_read_in_either_spelling_and_reaches_the_draw() {
    let layout = Layout::from_xml(
        r#"<Screen>
             <Image name="Turned">
               <Values rotationTheta="1.5708" x="10" y="20" width="100" height="10"
                       U="0" V="0" TxtrWidth="100" TxtrHeight="10" src="a.mip"/>
             </Image>
             <Image name="Tilted">
               <Values RotationTheta="-0.385" x="0" y="0" width="8" height="8"
                       U="0" V="0" TxtrWidth="8" TxtrHeight="8" src="a.mip"/>
             </Image>
             <Image name="Flat">
               <Values x="0" y="0" width="8" height="8" U="0" V="0"
                       TxtrWidth="8" TxtrHeight="8" src="a.mip"/>
             </Image>
           </Screen>"#,
    );
    // The disc's own spelling of a quarter turn, five decimals of it. Not
    // `FRAC_PI_2`, and `allow`ed rather than replaced by it: the point of the
    // assertion is that the parse returns *the number the layout writes*, and
    // substituting a more exact constant would be checking something else.
    #[allow(clippy::approx_constant)]
    const QUARTER_TURN: f32 = 1.5708;
    let angles: Vec<f32> = layout.sprites.iter().map(|s| s.rotation).collect();
    assert_eq!(angles, vec![QUARTER_TURN, -0.385, 0.0]);

    let sheet = crate::sprite::Sheet::placed_at(&[(
        "a.mip",
        crate::sprite::Placed {
            x: 0,
            y: 0,
            width: 128,
            height: 128,
        },
    )]);
    // A widget with no angle stays an unrotated quad; one with an angle does not.
    assert!(matches!(
        sprite_draw(&layout.sprites[2], &sheet),
        Some(Draw::Sprite { .. })
    ));
    let Some(Draw::RotatedSprite { rect, rotation, .. }) = sprite_draw(&layout.sprites[0], &sheet)
    else {
        panic!("expected a rotated draw");
    };
    // The **unrotated** rectangle, which is what the shader turns about its own
    // centre - see `Sprite::rotation`.
    assert_eq!(rect, [10.0, 20.0, 100.0, 10.0]);
    assert_eq!(rotation, QUARTER_TURN);
}

/// `ZonePlus<N>` counts up from the zone the craft is in, and shows nothing
/// before the first one.
///
/// The arithmetic on its own, without a disc: the ladder is eleven widgets whose
/// numbers are `zone + N`. `crates/game/tests/hd_hud_ground_truth.rs` is where
/// the same rule is checked against HD's real layout and the reference frame.
#[test]
fn the_zone_ladder_counts_up_from_the_current_zone() {
    let layout = Layout::from_xml(
        r#"<Screen>
             <Text name="ZonePlus0"><Values font="HUD" x="0" y="0"/></Text>
             <Text name="ZonePlus1"><Values string="1" font="HUDSmall" x="0" y="20"/></Text>
             <Text name="ZonePlus10"><Values string="10" font="HUDSmall" x="0" y="40"/></Text>
           </Screen>"#,
    );
    // No captions are involved: every widget here is either numbered or
    // resolved through a ladder this test does not supply.
    let strings = crate::language::StringTable::default();
    let text = |zone: u32| -> Vec<Option<String>> {
        layout
            .labels
            .iter()
            .map(|label| {
                text_for(
                    label,
                    &Readout {
                        zone,
                        ..Readout::blank()
                    },
                    &strings,
                    false,
                    true,
                    None,
                )
            })
            .collect()
    };
    // Zone 0 is the grid, before the first ten-second step. The rows ahead are
    // still numbered - `zone + N` with `zone = 0` is `1`..`10`, which is exactly
    // what the disc authors as their placeholder strings - and only the current
    // row is blank.
    assert_eq!(
        text(0),
        vec![None, Some("1".to_string()), Some("10".to_string())]
    );
    assert_eq!(
        text(1),
        vec![
            Some("1".to_string()),
            Some("2".to_string()),
            Some("11".to_string()),
        ]
    );
    assert_eq!(
        text(7),
        vec![
            Some("7".to_string()),
            Some("8".to_string()),
            Some("17".to_string()),
        ]
    );
}
