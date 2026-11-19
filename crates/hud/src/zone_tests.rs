//! Zone's ladder: the `RotationTheta` the column and its ticks are authored
//! with, and the `ZonePlus<N>` arithmetic that numbers the rows.
//!
//! Its own file rather than more of [`super::tests`], which is at the
//! 1,000-line ceiling `scripts/check-file-size.py` enforces - the same reason
//! [`super::reticle_tests`] is one. Everything here is title-blind and needs no
//! disc; `crates/raceplay/tests/hd_hud_ground_truth.rs` is where the same rules meet
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
            quad_extent: None,
            quad_centre: None,
            blend: None,
            scale: [1.0, 1.0],
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
/// numbers are `zone + N`. `crates/raceplay/tests/hd_hud_ground_truth.rs` is where
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
    let strings = oag_ui::language::StringTable::default();
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
                    true,
                    false,
                    crate::Precision::Tenths,
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

/// Pulse's Zone line names the class off the zone, in ten bands, and blanks the
/// caption over it from zone 35 - `FUN_0881ad10`'s own thresholds.
///
/// Without `rung_zones` the class would read the grade's stage, which Pulse
/// leaves at zero: every zone would say `sub-venom`. The widget text is the
/// string id, as the empty table here leaves it; the two Zone pairs
/// (`sub-rapier` at zone 8, no caption at zone 50) are the live check.
#[test]
fn pulse_names_the_speed_class_off_the_zone_and_blanks_the_caption_from_zen() {
    let layout = Layout::from_xml(
        r#"<Screen>
             <Text name="SpeedClassTxt"><Values idstring="RC_SC" font="HUDSmall" x="0" y="0"/></Text>
             <Text name="SpeedClass"><Values font="HUD" x="0" y="0"/></Text>
           </Screen>"#,
    );
    let strings = oag_ui::language::StringTable::default();
    let art = oag_pulse::hud::ART;
    let text = |zone: u32, widget: usize| {
        text_for(
            &layout.labels[widget],
            &Readout {
                zone,
                ..Readout::blank()
            },
            &strings,
            false,
            true,
            art.zone_speed_classes,
            true,
            false,
            crate::Precision::Tenths,
        )
    };
    let class = |zone| text(zone, 1);
    assert_eq!(class(0), None, "not before the first zone, like `Zone`");
    assert_eq!(class(1).as_deref(), Some("MSC_SVENOM"));
    assert_eq!(class(2).as_deref(), Some("Venom"));
    assert_eq!(class(7).as_deref(), Some("MSC_SRAPIER"));
    assert_eq!(class(8).as_deref(), Some("MSC_SRAPIER"), "the Talon's pair");
    assert_eq!(class(11).as_deref(), Some("MSC_SRAPIER"));
    assert_eq!(class(12).as_deref(), Some("Rapier"));
    assert_eq!(class(21).as_deref(), Some("Phantom"));
    assert_eq!(class(34).as_deref(), Some("MSC_SPPHANTOM"));
    assert_eq!(class(35).as_deref(), Some("MSC_ZEN"));
    assert_eq!(class(50).as_deref(), Some("MSC_ZEN"), "the Moa pair");
    assert!(text(34, 0).is_some(), "the caption is up below zone 35");
    assert_eq!(text(35, 0), None, "and gone from it");
}

/// 2048's HD-layout Zone line names the class off the zone on HD's fourteen thresholds and never
/// blanks the caption: `FUN_811538c2`'s table, and the frame `68-zone-5.png` reads `FLASH` at zone 5.
///
/// Fails if `oag_2048::hud::ART` drops `zone_speed_classes` (no class at all, the gap the
/// campaign frames showed), if it falls back to the grade's stage (which 2048's 17-record
/// `*_CLASS` ladder would drive to a different name), or if a thresholds row drifts from HD's.
#[test]
fn the_2048_zone_hud_names_the_class_off_the_zone_on_hds_thresholds() {
    let layout = Layout::from_xml(
        r#"<Screen>
             <Text name="SpeedClassTxt"><Values idstring="RC_SC" font="HUDSmall" x="0" y="0"/></Text>
             <Text name="SpeedClass"><Values font="HUD" x="0" y="0"/></Text>
           </Screen>"#,
    );
    let strings = oag_ui::language::StringTable::default();
    let art = oag_2048::hud::ART;
    let text = |zone: u32, widget: usize| {
        text_for(
            &layout.labels[widget],
            &Readout {
                zone,
                zone_stage: 9,
                ..Readout::blank()
            },
            &strings,
            false,
            true,
            art.zone_speed_classes,
            true,
            false,
            crate::Precision::Tenths,
        )
    };
    let class = |zone| text(zone, 1);
    assert_eq!(class(0), None, "not before the first zone, like `Zone`");
    assert_eq!(class(1).as_deref(), Some("MSC_SVENOM"));
    assert_eq!(class(2).as_deref(), Some("Venom"));
    assert_eq!(class(4).as_deref(), Some("MSC_SFLASH"));
    assert_eq!(class(5).as_deref(), Some("Flash"), "68-zone-5.png");
    assert_eq!(class(34).as_deref(), Some("MSC_SPPHANTOM"));
    assert_eq!(class(35).as_deref(), Some("MSC_ZEN"));
    assert_eq!(class(74).as_deref(), Some("IG_HUD_MACH1"));
    assert_eq!(class(75).as_deref(), Some("IG_HUD_SUPSON"));
    assert_eq!(class(300).as_deref(), Some("IG_HUD_SUPSON"));
    assert!(text(80, 0).is_some(), "no zone blanks the caption");
    let hd = oag_hd::hud::ZONE_SPEED_CLASSES;
    let ours = art.zone_speed_classes.expect("2048 names the class");
    for zone in 1..=100 {
        let rung = ours.rung(0, zone);
        let hd_stage = (0..hd.ids.len() as u32)
            .rev()
            .find(|&s| oag_hd::race::ZONE_STAGES.stage_for(zone as u16) == Some(s))
            .expect("HD's table covers every zone");
        assert_eq!(ours.id_for(rung), hd.id_for(hd_stage), "zone {zone}");
    }
}

/// Omega's Zone HUD names the class off the zone on a fourteen-record table that is HD's byte for
/// byte (`docs/ghidra/functions/ps4-omega-eu/zone-speed-class-ladder.md`), so the zone-keyed reading
/// must name at every zone the class HD's stage ladder does, and Omega's circuit grade
/// (`oag_omega::race::ZONE_FALLBACK`) must step on the same thresholds.
///
/// Fails if a threshold or id of `oag_omega::hud::ZONE_SPEED_CLASSES` drifts from the table, if the
/// table is dropped from `oag_omega::hud::ART`, or if the grade's ladder and the HUD's disagree.
#[test]
fn omegas_zone_hud_names_the_class_off_the_zone_on_the_grades_thresholds() {
    let art = oag_omega::hud::ART;
    let ours = art.zone_speed_classes.expect("Omega names the class");
    let stages = oag_omega::race::ZONE_FALLBACK.stages;
    assert_eq!(ours.ids.len(), stages.records.len());
    assert_eq!(ours.id_for(ours.rung(0, 5)), Some("Flash"));
    assert_eq!(ours.id_for(ours.rung(0, 7)), Some("MSC_SRAPIER"));
    assert_eq!(ours.id_for(ours.rung(0, 6)), Some("Flash"));
    assert_eq!(ours.id_for(ours.rung(0, 999)), Some("IG_HUD_SUPSON"));
    for zone in 0..=100u16 {
        assert_eq!(
            ours.id_for(ours.rung(0, u32::from(zone))),
            stages.class_for(zone),
            "zone {zone}"
        );
    }
    let hd = oag_hd::hud::ZONE_SPEED_CLASSES;
    for zone in 0..=100u16 {
        let stage = oag_hd::race::ZONE_STAGES
            .stage_for(zone)
            .expect("HD's table covers every zone");
        assert_eq!(
            ours.id_for(ours.rung(0, u32::from(zone))),
            hd.id_for(stage),
            "zone {zone}"
        );
    }
}

/// The next-class row names the rung **above the one the HUD names**: on a title whose class is
/// keyed on the zone (Omega's walker, `rung_zones`) that is the zone's rung plus one, not the
/// grade's stage plus one, or the original's `SUB-RAPIER` beside row 7 at zone 5 reads `RAPIER`.
///
/// Fails if `NextSpeedClass` goes back to indexing by the stage on a zone-keyed ladder; HD's
/// stage-keyed ladder is the control.
#[test]
fn the_next_class_row_names_the_rung_above_the_one_the_hud_names() {
    let layout = Layout::from_xml(
        r#"<Screen>
             <Text name="NextSpeedClass"><Values font="HUD" x="0" y="0"/></Text>
           </Screen>"#,
    );
    let strings = oag_ui::language::StringTable::default();
    let next = |art: &oag_title::HudArt, zone: u32, stage: u32| {
        text_for(
            &layout.labels[0],
            &Readout {
                zone,
                zone_stage: stage,
                zone_next_in: Some(2),
                ..Readout::blank()
            },
            &strings,
            false,
            true,
            art.zone_speed_classes,
            true,
            false,
            crate::Precision::Tenths,
        )
    };
    assert_eq!(
        next(oag_omega::hud::ART, 5, 4).as_deref(),
        Some("MSC_SRAPIER"),
        "zone 5 is FLASH, the next class is SUB-RAPIER"
    );
    assert_eq!(
        next(oag_hd::hud::ART, 5, 4).as_deref(),
        Some("MSC_SRAPIER"),
        "HD keys the rung on the stage"
    );
}
