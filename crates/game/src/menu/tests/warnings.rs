//! What the graphics page's `warn_when` lists are asserted to say.
//!
//! Its own file rather than more of `definition.rs`, which is at the
//! 1,000-line ratchet `just check-size` enforces - and a real seam besides:
//! everything here is about one mechanism, and about the same hazard. A
//! warning names *values* on other rows, so every one of them is a hand-written
//! enumeration of a comparison the menu module deliberately cannot make, and
//! every one of these tests generates the same enumeration from the type that
//! owns the values and demands they match.
//!
//! **Since dynamic resolution there is a second hazard on top of that.**
//! `graphics.render_scale` names a *ceiling*, and three of these warnings were
//! written when it was the size the frame is drawn at. Each is now two entries
//! sharing one message - one for a controller that is off, one for a floor
//! high enough that the statement holds anyway - because `Condition.all` is an
//! AND and "off, or floored high" is an OR.

use super::*;

/// The upscaler's warning must name exactly the scales it does nothing at.
///
/// Two places encode "FSR 1 is a magnifier": `upscale::magnifies`, which
/// declines to run it, and this warning, which says so. They are pinned to
/// each other here because a drift between them is invisible either way -
/// a missing value warns nobody at a scale where the setting is dead, and a
/// spare value warns at a scale where it works.
#[test]
fn the_upscaler_warns_at_exactly_the_scales_it_does_nothing_at() {
    let definition = built_in();
    let entry = definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some("graphics.upscaler"))
        .expect("nothing edits graphics.upscaler");
    let warning = off_variant(entry.warnings(), |w| {
        w.all.iter().any(|c| c.setting == "graphics.upscaler")
    });
    // One condition names this row's own offending value, the other the
    // scales. Without the first, the warning fires with the upscaler off.
    let own = warning
        .all
        .iter()
        .find(|c| c.setting == "graphics.upscaler")
        .expect("the warning must name the upscaler's own value");
    assert_eq!(
        own.values,
        vec![Value::Text(crate::display::Upscaler::Fsr1.to_string())]
    );
    let scales = warning
        .all
        .iter()
        .find(|c| c.setting == "graphics.render_scale")
        .expect("the warning must name the scales");

    let warned: Vec<crate::display::Scale> = scales
        .values
        .iter()
        .map(|value| value.to_string().parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    // Every offered scale is on exactly the side the guard puts it: warned
    // when a 1000-wide rectangle rendered at that scale is not smaller than
    // the rectangle, and unwarned when it is.
    let rect = (1000, 1000);
    for scale in crate::display::Scale::OFFERED {
        let scene = crate::upscale::target_size((0.0, 0.0, 1000.0, 1000.0), scale, 8192);
        let magnifies = crate::upscale::magnifies(scene, rect);
        assert_eq!(
            !magnifies,
            warned.contains(&scale),
            "{scale}: the guard and the warning disagree"
        );
    }
}

/// The anti-aliasing row warns against FXAA/SMAA at exactly the scales
/// FSR 1 actually magnifies at - the same set the row above is pinned to
/// above. Below that render scale a spatial post-process pass blurs the
/// scene before EASU ever reads it, fighting the very edges it reasons
/// about; at 100 % and above FSR 1 does not run at all (see the test
/// above) and there is nothing to fight.
#[test]
fn anti_aliasing_warns_against_the_upscaler_at_exactly_the_scales_it_fights_it_at() {
    let definition = built_in();
    let entry = definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some("graphics.anti_aliasing"))
        .expect("nothing edits graphics.anti_aliasing");
    assert_eq!(
        entry.warnings().len(),
        5,
        "the FSR 1 conflict and the 200%-redundancy warning, each in two \
         variants since RENDER SCALE became a ceiling, plus the FSR 3.1 one - \
         which needs only one variant because it does not read RENDER SCALE at \
         all: `resolve_scene` skips the pass outright when FSR 3.1 resolves, \
         at every scale. A \
         sixth would go unnoticed by the rest of this test"
    );
    // Picked out by the condition this test is actually about: the row also
    // carries the 200%-redundancy warning below, which names no
    // `graphics.upscaler` condition at all, and the FSR 3.1 one, which names no
    // `graphics.render_scale` condition.
    let warning = entry
        .warnings()
        .iter()
        .find(|w| {
            w.all.iter().any(|c| c.setting == "graphics.upscaler")
                && w.all.iter().any(|c| c.setting == "graphics.render_scale")
        })
        .expect("anti-aliasing warns against the upscaler");

    let own = warning
        .all
        .iter()
        .find(|c| c.setting == "graphics.anti_aliasing")
        .expect("the warning must name anti-aliasing's own offending values");
    let warned_modes: Vec<crate::display::AntiAliasing> = own
        .values
        .iter()
        .map(|value| value.to_string().parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    // Every value named must actually be a spatial post-process pass, and
    // every spatial post-process pass must be named - not a subset either
    // way, or the warning would mislead about MSAA or miss FXAA/SMAA.
    for mode in crate::display::AntiAliasing::ALL {
        assert_eq!(
            mode.is_spatial_post_process(),
            warned_modes.contains(&mode),
            "{mode}: the warning and `is_spatial_post_process` disagree"
        );
    }

    let upscaler = warning
        .all
        .iter()
        .find(|c| c.setting == "graphics.upscaler")
        .expect("the warning must require the upscaler to be fsr1");
    assert_eq!(
        upscaler.values,
        vec![Value::Text(crate::display::Upscaler::Fsr1.to_string())]
    );

    let scales = warning
        .all
        .iter()
        .find(|c| c.setting == "graphics.render_scale")
        .expect("the warning must name the scales fsr1 fights at");
    let warned_scales: Vec<crate::display::Scale> = scales
        .values
        .iter()
        .map(|value| value.to_string().parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    let rect = (1000, 1000);
    for scale in crate::display::Scale::OFFERED {
        let scene = crate::upscale::target_size((0.0, 0.0, 1000.0, 1000.0), scale, 8192);
        let magnifies = crate::upscale::magnifies(scene, rect);
        assert_eq!(
            magnifies,
            warned_scales.contains(&scale),
            "{scale}: the guard and the anti-aliasing warning disagree"
        );
    }
}

/// A second, independently-triggered warning on the same row: 200% render
/// scale oversamples enough on its own that FXAA/SMAA on top of it is
/// redundant work, not a broken combination - real product policy per
/// ADR-0013's "What the render-scale tiers mean for FXAA/SMAA" section. This
/// is what `Entry::warnings` being a list is for: the test above already
/// pins one warning about the *bottom* of the render-scale range, and this
/// is an unrelated combination at the *top* of it, on the same row.
#[test]
fn anti_aliasing_also_warns_that_it_is_redundant_on_top_of_the_ceiling_render_scale() {
    let definition = built_in();
    let entry = definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some("graphics.anti_aliasing"))
        .expect("nothing edits graphics.anti_aliasing");
    // Picked out by the condition that actually distinguishes it from the
    // upscaler-conflict warning above: this one names no upscaler at all,
    // because supersampling is redundant with a spatial pass whatever the
    // upscaler is set to.
    let warning = off_variant(entry.warnings(), |w| {
        !w.all.iter().any(|c| c.setting == "graphics.upscaler")
    });
    assert_eq!(
        warning.all.len(),
        3,
        "the row's own mode, the render scale, and that the controller is off"
    );

    let own = warning
        .all
        .iter()
        .find(|c| c.setting == "graphics.anti_aliasing")
        .expect("the warning must name anti-aliasing's own offending values");
    let warned_modes: Vec<crate::display::AntiAliasing> = own
        .values
        .iter()
        .map(|value| value.to_string().parse().unwrap_or_else(|e| panic!("{e}")))
        .collect();
    for mode in crate::display::AntiAliasing::ALL {
        assert_eq!(
            mode.is_spatial_post_process(),
            warned_modes.contains(&mode),
            "{mode}: the redundancy warning and `is_spatial_post_process` disagree"
        );
    }

    let scales = warning
        .all
        .iter()
        .find(|c| c.setting == "graphics.render_scale")
        .expect("the warning must name the redundant render scale");
    let top = *crate::display::Scale::OFFERED
        .last()
        .expect("render scale offers at least one value");
    assert_eq!(
        scales.values,
        vec![Value::Text(top.to_string())],
        "the redundancy warning must name exactly the ceiling render scale, not a hardcoded 200"
    );
}

/// The floor warns at exactly the render scales it cannot fall below.
///
/// Two places encode "a floor at or above the ceiling leaves the controller
/// nowhere to go": `drs::Limits::new`, which collapses the floor onto the
/// ceiling so the frame loop cannot panic on it, and this warning, which tells
/// the player. `Condition` compares values and has no ordering - deliberately,
/// it is what keeps the menu module ignorant of what a setting means - so the
/// pairs are enumerated in `menu.toml` by hand, and a hand-written list of
/// thirty-six comparisons is exactly the kind that drifts silently. This
/// generates the same list from `Scale::OFFERED` and demands they match.
///
/// A missing pair warns nobody about a floor that does nothing; a spare one
/// warns about a pairing that works.
#[test]
fn the_floor_warns_at_exactly_the_scales_it_cannot_fall_below() {
    let definition = built_in();
    let entry = definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some("graphics.minimum_resolution"))
        .expect("nothing edits graphics.minimum_resolution");

    // What the TOML says: floor -> the scales it is warned against.
    let mut declared: Vec<(crate::display::Scale, Vec<crate::display::Scale>)> = Vec::new();
    for warning in entry.warnings() {
        let parse = |c: &Condition| -> Vec<crate::display::Scale> {
            c.values
                .iter()
                .map(|value| value.to_string().parse().unwrap_or_else(|e| panic!("{e}")))
                .collect()
        };
        let floor = warning
            .all
            .iter()
            .find(|c| c.setting == "graphics.minimum_resolution")
            .map(parse)
            .expect("the warning must name its own row's value");
        let scales = warning
            .all
            .iter()
            .find(|c| c.setting == "graphics.render_scale")
            .map(parse)
            .expect("the warning must name the scales");
        assert_eq!(floor.len(), 1, "one warning names one floor: {floor:?}");
        declared.push((floor[0], scales));
    }

    for floor in crate::display::Scale::OFFERED {
        let conflicting: Vec<crate::display::Scale> = crate::display::Scale::OFFERED
            .into_iter()
            .filter(|ceiling| ceiling.percent() <= floor.percent())
            .collect();
        let found = declared.iter().find(|(named, _)| *named == floor);
        match found {
            Some((_, scales)) => assert_eq!(
                *scales, conflicting,
                "{floor} is warned against the wrong set of render scales"
            ),
            None => assert!(
                conflicting.is_empty(),
                "{floor} conflicts with {conflicting:?} and has no warning"
            ),
        }
    }
    assert_eq!(
        declared.len(),
        crate::display::Scale::OFFERED.len(),
        "every offered floor conflicts with at least itself, so every one warns"
    );
}

/// Picks the `target_fps = off` variant of a warning that comes in two.
///
/// Since dynamic resolution, three warnings on this page state something about
/// *the size the frame is drawn at* while `graphics.render_scale` only names
/// the **ceiling**. `Condition.all` is an AND and the statement needs an OR -
/// "the controller is off, **or** its floor is high enough" - so each is two
/// entries sharing one message. These tests were written about the first, and
/// [`floor_variant`] covers the second.
fn off_variant(warnings: &[Warning], distinguish: impl Fn(&Warning) -> bool) -> &Warning {
    warnings
        .iter()
        .find(|w| {
            distinguish(w)
                && w.all.iter().any(|c| {
                    c.setting == "graphics.target_fps"
                        && c.values == vec![Value::Text("off".to_string())]
                })
        })
        .expect("the dynamic-resolution-off variant of the warning")
}

/// The other variant: the one that fires because the controller's floor is
/// high enough that the statement is true anyway.
fn floor_variant(warnings: &[Warning], distinguish: impl Fn(&Warning) -> bool) -> &Warning {
    warnings
        .iter()
        .find(|w| {
            distinguish(w)
                && w.all
                    .iter()
                    .any(|c| c.setting == "graphics.minimum_resolution")
        })
        .expect("the floor variant of the warning")
}

/// Every scale in a condition, parsed.
fn scales_of(condition: &Condition) -> Vec<crate::display::Scale> {
    condition
        .values
        .iter()
        .map(|value| value.to_string().parse().unwrap_or_else(|e| panic!("{e}")))
        .collect()
}

/// A warning about the size the frame is drawn at must read the **floor**, not
/// `RENDER SCALE`.
///
/// Since dynamic resolution `RENDER SCALE` names a ceiling, and every warning
/// on this page that treats it as the drawn size is wrong the moment the
/// controller can go below it. The one that made this visible: UPSCALER at a
/// 100 % ceiling with a 50 % minimum says "NO EFFECT AT RENDER SCALE 100 OR
/// ABOVE" while FSR 1 is magnifying every frame - a working setting reported
/// as dead.
///
/// So each of the three is two entries sharing a message: one requiring the
/// controller off, one requiring its floor high enough that the statement
/// holds anyway. This pins the second against `upscale::magnifies`, which is
/// the same guard the `off` variants are already pinned to - and it is the
/// pairing rather than either list that drifts.
#[test]
fn a_warning_about_the_drawn_size_reads_the_floor_and_not_the_ceiling() {
    let definition = built_in();
    let entry = |setting: &str| {
        definition
            .pages
            .iter()
            .flat_map(|page| page.entries.iter())
            .find(|entry| entry.setting() == Some(setting))
            .unwrap_or_else(|| panic!("nothing edits {setting}"))
            .warnings()
            .to_vec()
    };

    // The floors at which FSR 1 still does nothing are exactly the scales at
    // which it does nothing, because a floor *is* a render scale - the two
    // rows offer the same list for that reason.
    let rect = (1000, 1000);
    let dead: Vec<crate::display::Scale> = crate::display::Scale::OFFERED
        .into_iter()
        .filter(|scale| {
            let scene = crate::upscale::target_size((0.0, 0.0, 1000.0, 1000.0), *scale, 8192);
            !crate::upscale::magnifies(scene, rect)
        })
        .collect();
    assert!(!dead.is_empty() && dead.len() < crate::display::Scale::OFFERED.len());

    let upscaler = entry("graphics.upscaler");
    let floored = floor_variant(&upscaler, |w| {
        w.all.iter().any(|c| c.setting == "graphics.upscaler")
    });
    let floor = floored
        .all
        .iter()
        .find(|c| c.setting == "graphics.minimum_resolution")
        .expect("the floor variant names the floor");
    assert_eq!(
        scales_of(floor),
        dead,
        "the floor variant must name exactly the floors at which FSR 1 still does nothing"
    );
    // And it must still require the ceiling to be there too, or it would fire
    // at a 200 % ceiling with a 100 % floor, where FSR 1 does magnify.
    let ceiling = floored
        .all
        .iter()
        .find(|c| c.setting == "graphics.render_scale")
        .expect("the floor variant still names the ceiling");
    assert_eq!(scales_of(ceiling), dead);

    // The anti-aliasing row's own floor variant is the converse: it fires
    // where the controller *will* magnify, so it names the complement.
    let magnified: Vec<crate::display::Scale> = crate::display::Scale::OFFERED
        .into_iter()
        .filter(|scale| !dead.contains(scale))
        .collect();
    let aa = entry("graphics.anti_aliasing");
    let fights = floor_variant(&aa, |w| {
        w.all.iter().any(|c| c.setting == "graphics.upscaler")
    });
    let floor = fights
        .all
        .iter()
        .find(|c| c.setting == "graphics.minimum_resolution")
        .expect("the floor variant names the floor");
    assert_eq!(
        scales_of(floor),
        magnified,
        "FXAA/SMAA fight the upscaler at exactly the floors it magnifies from"
    );
    // It has to name a target too, or it would fire with the controller off,
    // where the floor means nothing at all.
    let target = fights
        .all
        .iter()
        .find(|c| c.setting == "graphics.target_fps")
        .expect("the floor variant must require a controller to be running");
    assert!(
        !target
            .values
            .contains(&Value::Text(crate::drs::Target::OFF.to_string())),
        "a floor is only meaningful while the controller runs"
    );
    assert_eq!(
        target.values.len(),
        crate::drs::Target::OFFERED.len() - 1,
        "every target except off, generated from Target::OFFERED"
    );
}

/// The target warns at exactly the frame limits it is above.
///
/// Two halves of one answer, and this pins the visible one: the row says what
/// was asked for and `drs::Target::at_most` holds the controller to what the
/// loop will actually produce. Aiming at 144 behind a 60 limit would otherwise
/// give up pixels to buy frames the limiter forbids.
///
/// **Only where the limiter is in force.** Under `vsync = on` the display is
/// the bound and this build cannot ask a surface what its refresh is, so
/// neither the warning nor the clamp fires - a wrong clamp at 60 on a 144 Hz
/// panel is worse than none.
#[test]
fn the_target_warns_at_exactly_the_frame_limits_it_is_above() {
    let definition = built_in();
    let entry = definition
        .pages
        .iter()
        .flat_map(|page| page.entries.iter())
        .find(|entry| entry.setting() == Some("graphics.target_fps"))
        .expect("nothing edits graphics.target_fps");

    let mut declared: Vec<(crate::drs::Target, Vec<crate::perf::FrameLimit>)> = Vec::new();
    for warning in entry.warnings() {
        let target: Vec<crate::drs::Target> = warning
            .all
            .iter()
            .find(|c| c.setting == "graphics.target_fps")
            .expect("the warning names its own row's value")
            .values
            .iter()
            .map(|value| value.to_string().parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        assert_eq!(target.len(), 1, "one warning names one target");
        let limits: Vec<crate::perf::FrameLimit> = warning
            .all
            .iter()
            .find(|c| c.setting == "display.frame_limit")
            .expect("the warning names the limits")
            .values
            .iter()
            .map(|value| value.to_string().parse().unwrap_or_else(|e| panic!("{e}")))
            .collect();
        // The third condition is what keeps it off the vsync path.
        let vsync = warning
            .all
            .iter()
            .find(|c| c.setting == "display.vsync")
            .expect("the warning must exclude the vsync-paced modes");
        assert!(
            !vsync
                .values
                .contains(&Value::Text(crate::perf::Vsync::On.to_string())),
            "under classic vsync the display is the bound and the limiter is not"
        );
        declared.push((target[0], limits));
    }

    for target in crate::drs::Target::OFFERED {
        let Some(hz) = target.hz() else {
            assert!(
                !declared.iter().any(|(named, _)| *named == target),
                "off cannot be above anything"
            );
            continue;
        };
        // Unlimited is not below any target, so it is never in a list.
        let above: Vec<crate::perf::FrameLimit> = crate::perf::FrameLimit::OFFERED
            .into_iter()
            .filter(|limit| limit.hz().is_some_and(|limit| limit < hz))
            .collect();
        match declared.iter().find(|(named, _)| *named == target) {
            Some((_, limits)) => assert_eq!(
                *limits, above,
                "{target} is warned against the wrong set of frame limits"
            ),
            None => assert!(
                above.is_empty(),
                "{target} is above {above:?} and has no warning"
            ),
        }
    }
}
