//! What the display vocabulary in [`super`] is asserted to do: the letterboxed
//! viewport, every setting's round trip through its own spelling, the
//! camera-view cycle, the gamma and field-of-view curves, and picking a monitor
//! or a renderer by name.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `display.rs`: the tests are 543 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

/// The aspect of a rectangle `viewport` returned.
fn ratio(rect: (f32, f32, f32, f32)) -> f32 {
    rect.2 / rect.3
}

#[test]
fn free_fills_whatever_it_is_given() {
    assert_eq!(
        viewport((1920, 1080), Aspect::Free),
        (0.0, 0.0, 1920.0, 1080.0)
    );
    assert_eq!(viewport((100, 900), Aspect::Free), (0.0, 0.0, 100.0, 900.0));
}

/// The window this build has always opened at is exactly the PSP's shape,
/// so the default setting on the default window draws no bars at all.
#[test]
fn the_default_window_needs_no_bars_at_the_default_aspect() {
    let size = Size::default();
    let rect = viewport((size.width, size.height), Aspect::Psp);
    assert_eq!(rect.0, 0.0, "{rect:?}");
    assert_eq!(rect.1, 0.0, "{rect:?}");
    assert!(
        (rect.2 - size.width as f32).abs() < 1.0 && (rect.3 - size.height as f32).abs() < 1.0,
        "{rect:?}"
    );
}

#[test]
fn a_wide_window_gets_bars_down_the_sides() {
    let rect = viewport((3440, 1440), Aspect::Psp);
    assert!(rect.0 > 0.0, "no left bar: {rect:?}");
    assert_eq!(rect.1, 0.0, "and no top bar: {rect:?}");
    assert!((rect.3 - 1440.0).abs() < 1e-3, "full height: {rect:?}");
    assert!(
        (ratio(rect) - Aspect::Psp.ratio().expect("psp has a ratio")).abs() < 1e-3,
        "{rect:?}"
    );
    // Centred, so both bars are the same width.
    assert!((rect.0 * 2.0 + rect.2 - 3440.0).abs() < 1e-3, "{rect:?}");
}

#[test]
fn a_tall_window_gets_bars_above_and_below() {
    let rect = viewport((1080, 1920), Aspect::Ps2);
    assert_eq!(rect.0, 0.0, "{rect:?}");
    assert!(rect.1 > 0.0, "{rect:?}");
    assert!((ratio(rect) - 4.0 / 3.0).abs() < 1e-3, "{rect:?}");
    assert!((rect.1 * 2.0 + rect.3 - 1920.0).abs() < 1e-3, "{rect:?}");
}

/// The two fixed shapes really are different, or the setting does nothing.
#[test]
fn psp_and_ps2_are_not_the_same_shape() {
    let psp = viewport((1920, 1080), Aspect::Psp);
    let ps2 = viewport((1920, 1080), Aspect::Ps2);
    assert!((ratio(psp) - ratio(ps2)).abs() > 0.1, "{psp:?} {ps2:?}");
    assert!(ps2.0 > psp.0, "4:3 is the narrower of the two");
}

/// A minimised window reports zero, and a zero-area viewport is a wgpu
/// validation error rather than a blank frame.
#[test]
fn a_degenerate_window_still_gives_a_drawable_rectangle() {
    for target in [(0, 0), (1, 4000), (4000, 1)] {
        for aspect in Aspect::ALL {
            let rect = viewport(target, aspect);
            assert!(
                rect.2 >= 1.0 && rect.3 >= 1.0,
                "{target:?} {aspect}: {rect:?}"
            );
            assert!(rect.0.is_finite() && rect.1.is_finite());
        }
    }
}

/// The rectangle has to stay inside the surface, or `set_viewport` fails
/// validation - which is the one way this can be wrong without looking
/// wrong in a screenshot.
#[test]
fn the_rectangle_never_leaves_the_surface() {
    for target in [(1920, 1080), (800, 600), (3440, 1440), (7, 5000)] {
        for aspect in Aspect::ALL {
            let (x, y, w, h) = viewport(target, aspect);
            assert!(x >= 0.0 && y >= 0.0, "{target:?} {aspect}");
            assert!(
                x + w <= target.0.max(1) as f32 + 1e-3,
                "{target:?} {aspect}: {x} + {w}"
            );
            assert!(
                y + h <= target.1.max(1) as f32 + 1e-3,
                "{target:?} {aspect}: {y} + {h}"
            );
        }
    }
}

#[test]
fn a_size_round_trips_through_its_own_spelling() {
    let size: Size = "1920x1080".parse().expect("parse");
    assert_eq!(size, Size::new(1920, 1080));
    assert_eq!(size.to_string(), "1920x1080");
    assert_eq!("2560X1440".parse::<Size>().expect("parse").width, 2560);
}

#[test]
fn a_malformed_size_says_what_it_wanted() {
    for text in ["1920", "axb", "1920x", "0x0", ""] {
        let error = text.parse::<Size>().expect_err(text);
        assert!(!error.is_empty(), "{text}");
    }
}

#[test]
fn every_offered_size_is_the_psp_shape() {
    let wanted = Aspect::Psp.ratio().expect("psp has a ratio");
    for size in Size::OFFERED {
        let have = size.width as f32 / size.height as f32;
        assert!(
            (have - wanted).abs() < 0.02,
            "{size} is {have}, wanted about {wanted}"
        );
    }
}

#[test]
fn a_scale_round_trips_and_refuses_what_it_cannot_draw() {
    assert_eq!("100".parse::<Scale>(), Ok(Scale::FULL));
    assert_eq!("50%".parse::<Scale>().expect("parse").factor(), 0.5);
    assert_eq!(Scale::FULL.to_string(), "100");
    for bad in ["0", "24", "201", "1000", "half", ""] {
        assert!(bad.parse::<Scale>().is_err(), "{bad}");
    }
}

#[test]
fn every_offered_scale_is_one_this_build_accepts() {
    for scale in Scale::OFFERED {
        assert_eq!(scale.to_string().parse::<Scale>(), Ok(scale));
    }
    assert!(
        Scale::OFFERED.contains(&Scale::FULL),
        "100% must be offered"
    );
}

/// The three percentages the macro writes out behave the way the render
/// scale already did, which is the whole reason they share it.
#[test]
fn every_percentage_round_trips_and_refuses_what_is_outside_its_range() {
    assert_eq!("100".parse::<Brightness>(), Ok(Brightness::NEUTRAL));
    assert_eq!("100".parse::<Gamma>(), Ok(Gamma::NEUTRAL));
    assert_eq!("100%".parse::<Fov>(), Ok(Fov::AUTHORED));
    assert_eq!(Brightness::default(), Brightness::NEUTRAL);
    assert_eq!(Gamma::default(), Gamma::NEUTRAL);
    assert_eq!(Fov::default(), Fov::AUTHORED);

    for bad in ["0", "49", "201", "bright", ""] {
        assert!(bad.parse::<Brightness>().is_err(), "{bad}");
        assert!(bad.parse::<Gamma>().is_err(), "{bad}");
        assert!(bad.parse::<Fov>().is_err(), "{bad}");
    }

    for value in Brightness::OFFERED {
        assert_eq!(value.to_string().parse::<Brightness>(), Ok(value));
    }
    for value in Gamma::OFFERED {
        assert_eq!(value.to_string().parse::<Gamma>(), Ok(value));
    }
    for value in Fov::OFFERED {
        assert_eq!(value.to_string().parse::<Fov>(), Ok(value));
    }
}

/// Each list has to offer the value that changes nothing, or a player who
/// moves one of these has no way back to how the game shipped.
#[test]
fn every_percentage_offers_its_own_neutral() {
    assert!(Brightness::OFFERED.contains(&Brightness::NEUTRAL));
    assert!(Gamma::OFFERED.contains(&Gamma::NEUTRAL));
    assert!(Fov::OFFERED.contains(&Fov::AUTHORED));
    assert!(BoostFovKick::OFFERED.contains(&BoostFovKick::DEFAULT));
}

/// The same shape as [`every_percentage_round_trips_and_refuses_what_is_outside_its_range`],
/// pulled out on its own because `0` is one of *this* type's own valid
/// values rather than one of the bad ones every other percentage refuses.
#[test]
fn boost_fov_kick_round_trips_and_refuses_what_is_outside_its_range() {
    assert_eq!("8".parse::<BoostFovKick>(), Ok(BoostFovKick::SUBTLE));
    assert_eq!("16".parse::<BoostFovKick>(), Ok(BoostFovKick::DEFAULT));
    assert_eq!("0".parse::<BoostFovKick>(), Ok(BoostFovKick::OFF));
    assert_eq!(BoostFovKick::default(), BoostFovKick::DEFAULT);

    for bad in ["33", "-1", "kick", ""] {
        assert!(bad.parse::<BoostFovKick>().is_err(), "{bad}");
    }

    for value in BoostFovKick::OFFERED {
        assert_eq!(value.to_string().parse::<BoostFovKick>(), Ok(value));
    }
}

/// The recovered order, stated as a test because it is the one fact about
/// this type that came out of the binary and the one a refactor could
/// silently reorder.
#[test]
fn the_camera_view_cycle_is_the_originals_order_and_wraps() {
    assert_eq!(CameraView::Internal.next(), CameraView::Close);
    assert_eq!(CameraView::Close.next(), CameraView::Far);
    assert_eq!(CameraView::Far.next(), CameraView::Internal);

    // Three presses from anywhere is back where it started, and every view
    // is reachable - so the cycle is a rotation of the whole set rather
    // than a rotation of a subset with an orphan.
    for start in CameraView::ALL {
        let mut seen = vec![start];
        let mut view = start;
        for _ in 0..3 {
            view = view.next();
            if view != start {
                seen.push(view);
            }
        }
        assert_eq!(view, start, "from {start}");
        seen.sort_unstable_by_key(|v| v.name());
        let mut all: Vec<CameraView> = CameraView::ALL.to_vec();
        all.sort_unstable_by_key(|v| v.name());
        assert_eq!(seen, all, "from {start}");
    }

    // `ALL` is the cycle, in order, so a menu row built from it and the
    // button move through the same sequence.
    for pair in CameraView::ALL.windows(2) {
        assert_eq!(pair[0].next(), pair[1]);
    }
}

#[test]
fn camera_views_round_trip_and_refuse_nonsense() {
    for view in CameraView::ALL {
        assert_eq!(view.to_string().parse::<CameraView>(), Ok(view));
    }
    assert_eq!("FAR".parse::<CameraView>(), Ok(CameraView::Far));
    for bad in ["", "cockpit", "opt_int", "external"] {
        assert!(bad.parse::<CameraView>().is_err(), "{bad}");
    }
}

/// The default is a *choice* - see the type's docs - and it is the one that
/// leaves every existing capture alone, so it is worth pinning.
#[test]
fn the_default_camera_view_is_the_far_chase() {
    assert_eq!(CameraView::default(), CameraView::Far);
}

/// The cockpit view is the only one that hides the hull. Getting this
/// backwards fills the frame with the inside of a ship in the two views
/// where a player expects to see it from behind.
#[test]
fn only_the_internal_view_hides_the_players_own_ship() {
    assert!(!CameraView::Internal.draws_own_ship());
    assert!(CameraView::Close.draws_own_ship());
    assert!(CameraView::Far.draws_own_ship());
}

/// The settings file round-trips through serde, not through `FromStr`, so
/// the two spellings have to agree or a saved view comes back as the
/// default.
#[test]
fn camera_views_survive_a_settings_file() {
    for view in CameraView::ALL {
        let toml = toml::to_string(&Wrapper { view }).expect("serialises");
        assert!(toml.contains(view.name()), "{toml}");
        let back: Wrapper = toml::from_str(&toml).expect("deserialises");
        assert_eq!(back.view, view);
    }
}

#[derive(Serialize, Deserialize)]
struct Wrapper {
    view: CameraView,
}

/// The point of turning the toggle into a magnitude: each tier has to be
/// an actually larger multiplier, not just a different label on the same
/// number.
#[test]
fn boost_fov_kick_tiers_increase() {
    let gains: Vec<f32> = BoostFovKick::OFFERED.iter().map(|t| t.gain()).collect();
    assert!(
        gains.windows(2).all(|pair| pair[0] < pair[1]),
        "the tiers must strictly increase: {gains:?}"
    );
    assert_eq!(BoostFovKick::OFF.gain(), 0.0);
}

/// The one place gamma could be the wrong way round: the setting names the
/// gamma, the shader wants its reciprocal, and a gamma above 1 has to
/// brighten.
#[test]
fn a_higher_gamma_is_a_lower_exponent_and_brightens() {
    assert_eq!(Gamma::NEUTRAL.exponent(), 1.0);
    let up: Gamma = "140".parse().expect("parse");
    let down: Gamma = "60".parse().expect("parse");
    assert!(up.exponent() < 1.0, "{}", up.exponent());
    assert!(down.exponent() > 1.0, "{}", down.exponent());
    // A midtone, raised to each exponent. Above 1.0 gamma it has to come
    // out lighter, below it darker, and black and white must not move.
    let grey = 0.25f32;
    assert!(grey.powf(up.exponent()) > grey);
    assert!(grey.powf(down.exponent()) < grey);
    for exponent in [up.exponent(), down.exponent()] {
        assert_eq!(0.0f32.powf(exponent), 0.0);
        assert_eq!(1.0f32.powf(exponent), 1.0);
    }
}

/// The authored field has to come back untouched, bit for bit: every
/// capture in `data/traces/` was taken before this setting existed.
#[test]
fn the_authored_field_of_view_is_returned_unchanged() {
    for degrees in [30.0f32, 45.0, 60.0, 91.5] {
        let fov = degrees.to_radians();
        assert_eq!(Fov::AUTHORED.apply(fov), fov, "{degrees}");
    }
}

/// The property the tangent scaling exists for: a percentage means that
/// much more across the screen, and the result stays a projection.
#[test]
fn a_wider_field_of_view_scales_the_tangent_and_stays_below_half_a_turn() {
    let fov = 60.0f32.to_radians();
    let authored = (fov * 0.5).tan();
    for percent in ["50", "75", "125", "150", "200"] {
        let setting: Fov = percent.parse().expect("parse");
        let applied = setting.apply(fov);
        let scaled = (applied * 0.5).tan();
        assert!(
            (scaled - authored * setting.factor()).abs() < 1e-5,
            "{percent}: {scaled} vs {}",
            authored * setting.factor()
        );
        assert!(applied > 0.0 && applied < std::f32::consts::PI, "{percent}");
    }
    // Monotone, or the row would not read as a slider.
    let narrow: Fov = "75".parse().expect("parse");
    let wide: Fov = "150".parse().expect("parse");
    assert!(narrow.apply(fov) < fov && fov < wide.apply(fov));
}

/// The second monitor in a left-to-right pair, which is the case the whole
/// setting exists for: the window has to land inside *its* rectangle, not
/// at the same offset on the first one.
#[test]
fn a_window_is_centred_inside_the_monitor_it_is_given() {
    assert_eq!(centred((0, 0), (1920, 1080), (1440, 816)), (240, 132));
    assert_eq!(centred((1920, 0), (2560, 1440), (1440, 816)), (2480, 312));
    // A monitor above and left of the origin, which is where a compositor
    // puts a second screen arranged that way.
    assert_eq!(
        centred((-1920, -180), (1920, 1080), (1920, 1080)),
        (-1920, -180)
    );
}

/// A window bigger than the screen is pinned rather than centred off the
/// top-left edge, where the title bar cannot be reached.
#[test]
fn an_oversized_window_stays_at_the_monitors_own_corner() {
    assert_eq!(centred((1920, 0), (1280, 720), (2560, 1440)), (1920, 0));
    assert_eq!(centred((0, 0), (0, 0), (1440, 816)), (0, 0));
}

#[test]
fn a_monitor_round_trips_through_its_own_spelling() {
    let default: Monitor = "default".parse().expect("infallible");
    assert!(default.is_default());
    assert_eq!(default, Monitor::default());
    assert_eq!(default.to_string(), Monitor::DEFAULT);
    // Whitespace and an empty file value are the default too, not a screen
    // named "".
    for text in ["", "   ", "DEFAULT"] {
        assert!(
            text.parse::<Monitor>().expect("infallible").is_default(),
            "{text:?}"
        );
    }

    let named: Monitor = " DP-2 ".parse().expect("infallible");
    assert_eq!(named.name(), Some("DP-2"));
    assert_eq!(named.to_string(), "DP-2");
    assert_eq!(named.to_string().parse::<Monitor>(), Ok(named));
}

/// The offered list is what a player has to be able to get back to the
/// default from, so the default is on it whatever the machine has.
#[test]
fn the_default_is_always_the_first_monitor_offered() {
    assert_eq!(Monitor::offered(&[]), [Monitor::DEFAULT]);
    assert_eq!(
        Monitor::offered(&["eDP-1".to_string(), "DP-2".to_string()]),
        [Monitor::DEFAULT, "eDP-1", "DP-2"]
    );
    // Not repeated, however a compositor spells it.
    assert_eq!(
        Monitor::offered(&["Default".to_string(), "DP-2".to_string()]),
        [Monitor::DEFAULT, "DP-2"]
    );
    // And every entry on it round-trips to a setting that picks something.
    let available = ["eDP-1".to_string(), "DP-2".to_string()];
    for offered in Monitor::offered(&available) {
        let monitor: Monitor = offered.parse().expect("infallible");
        assert!(
            monitor.is_default() || monitor.choose(&available).is_some(),
            "{offered} is offered and selects nothing"
        );
    }
}

#[test]
fn a_monitor_picks_the_screen_it_names_and_nothing_else() {
    let available = ["eDP-1".to_string(), "DP-2".to_string()];
    assert_eq!(
        "DP-2".parse::<Monitor>().unwrap().choose(&available),
        Some(1)
    );
    // Case-insensitive, because a compositor is free to change how it
    // capitalises between releases.
    assert_eq!(
        "dp-2".parse::<Monitor>().unwrap().choose(&available),
        Some(1)
    );
    // The default never names one.
    assert_eq!(Monitor::default_monitor().choose(&available), None);
    // A screen this machine does not have falls back rather than failing,
    // and `is_default` is what tells the two apart.
    let missing: Monitor = "HDMI-A-1".parse().unwrap();
    assert_eq!(missing.choose(&available), None);
    assert!(!missing.is_default());
    // Nothing partial: `DP` must not select `DP-2`.
    assert_eq!("DP".parse::<Monitor>().unwrap().choose(&available), None);
    assert_eq!(missing.choose(&[]), None);
}

/// The RENDERER row's whole safety property: what the settings file holds
/// has to come back as the same string, because `Menu::seed` matches it
/// exactly and shows the first row when it does not.
#[test]
fn a_renderer_round_trips_through_its_own_spelling() {
    let default: Renderer = "default".parse().expect("infallible");
    assert!(default.is_default());
    assert_eq!(default, Renderer::default());
    assert_eq!(default.to_string(), Renderer::DEFAULT);
    for text in ["", "   ", "DEFAULT"] {
        assert!(
            text.parse::<Renderer>().expect("infallible").is_default(),
            "{text:?}"
        );
    }

    // A real label, punctuation and all: this is what `adapter::label`
    // produces and what the file therefore has to carry unchanged.
    let named: Renderer = "vulkan: llvmpipe (LLVM 22.1.8, 256 bits) (cpu)"
        .parse()
        .expect("infallible");
    assert_eq!(
        named.name(),
        Some("vulkan: llvmpipe (LLVM 22.1.8, 256 bits) (cpu)")
    );
    assert_eq!(named.to_string().parse::<Renderer>(), Ok(named));
}

#[test]
fn the_default_is_always_the_first_renderer_offered() {
    assert_eq!(Renderer::offered(&[]), [Renderer::DEFAULT]);
    let available = [
        "vulkan: AMD Radeon Graphics (RADV RENOIR)".to_string(),
        "vulkan: llvmpipe (LLVM 22.1.8, 256 bits) (cpu)".to_string(),
    ];
    assert_eq!(
        Renderer::offered(&available),
        [
            Renderer::DEFAULT,
            "vulkan: AMD Radeon Graphics (RADV RENOIR)",
            "vulkan: llvmpipe (LLVM 22.1.8, 256 bits) (cpu)",
        ]
    );
    for offered in Renderer::offered(&available) {
        let renderer: Renderer = offered.parse().expect("infallible");
        assert!(
            renderer.is_default() || renderer.choose(&available).is_some(),
            "{offered} is offered and selects nothing"
        );
    }
}

#[test]
fn a_renderer_picks_the_adapter_it_names_and_nothing_else() {
    let available = [
        "vulkan: AMD Radeon Graphics (RADV RENOIR)".to_string(),
        "vulkan: llvmpipe (LLVM 22.1.8, 256 bits) (cpu)".to_string(),
    ];
    assert_eq!(
        "vulkan: llvmpipe (LLVM 22.1.8, 256 bits) (cpu)"
            .parse::<Renderer>()
            .unwrap()
            .choose(&available),
        Some(1)
    );
    assert_eq!(Renderer::default_renderer().choose(&available), None);
    // A driver that is no longer installed falls back rather than failing:
    // uninstalling `vulkan-swrast` must not stop the game starting.
    let missing: Renderer = "vulkan: llvmpipe (LLVM 21.0.0, 256 bits) (cpu)"
        .parse()
        .unwrap();
    assert_eq!(missing.choose(&available), None);
    assert!(!missing.is_default());
    // Nothing partial. The driver version lives *inside* the name, so a
    // prefix match would happily select a different llvmpipe build - and,
    // worse, the wrong backend's copy of a card that has two.
    assert_eq!(
        "vulkan: llvmpipe"
            .parse::<Renderer>()
            .unwrap()
            .choose(&available),
        None
    );
}

#[test]
fn modes_and_aspects_round_trip_through_their_names() {
    for mode in WindowMode::ALL {
        assert_eq!(mode.name().parse::<WindowMode>(), Ok(mode));
    }
    for aspect in Aspect::ALL {
        assert_eq!(aspect.name().parse::<Aspect>(), Ok(aspect));
    }
    assert!("exclusive".parse::<WindowMode>().is_err());
    assert!("16:9".parse::<Aspect>().is_err());
}

#[test]
fn a_shadow_tier_round_trips_and_refuses_what_it_does_not_offer() {
    for tier in Shadows::ALL {
        assert_eq!(tier.name().parse::<Shadows>(), Ok(tier));
        assert_eq!(tier.to_string(), tier.name());
    }
    assert_eq!("BLOB".parse::<Shadows>(), Ok(Shadows::Blob));
    // `original` landed 2026-09-04 and its refusal line went with it.
    assert_eq!("original".parse::<Shadows>(), Ok(Shadows::Original));
    // The one designed tier nothing has built yet. Naming it here is what makes
    // the refusal deliberate rather than incidental: whoever lands it deletes
    // this line beside their new variant.
    assert!("mapped".parse::<Shadows>().is_err());
}

#[test]
fn nothing_casts_a_shadow_by_default() {
    // `off` until `original` exists - a generated falloff cannot be the
    // default on a title whose own hulls are sitting on the disc. See
    // `docs/rendering/shadows.md`.
    assert_eq!(Shadows::default(), Shadows::Off);
    assert!(!Shadows::default().draws());
    assert!(Shadows::Blob.draws());
}
