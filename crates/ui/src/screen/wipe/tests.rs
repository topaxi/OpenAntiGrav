use super::*;

fn window(s: f32) -> (f32, f32) {
    let window = Vwipe::window(s).expect("a window below s = 1");
    (window.top, window.height)
}

fn close(got: (f32, f32), want: (f32, f32)) {
    assert!(
        (got.0 - want.0).abs() < 1e-5 && (got.1 - want.1).abs() < 1e-5,
        "{got:?} vs {want:?}"
    );
}

/// "What is drawn: `Image_Render`": a 0.1 H slab at `2.25 s H` for `s` below 0.4.
#[test]
fn the_slab_sweeps_down_for_the_first_two_fifths() {
    close(window(0.0), (0.0, 0.1));
    close(window(0.2), (0.45, 0.1));
    close(window(0.39), (0.8775, 0.1));
}

/// The same slab back up at `(0.9 - 2.25 (s - 0.4)) H` for `s` from 0.4 to 0.8, joined to the
/// first sweep at the apex.
#[test]
fn the_slab_comes_back_up_to_the_top() {
    close(window(0.4), (0.9, 0.1));
    close(window(0.6), (0.45, 0.1));
    close(window(0.79), (0.0225, 0.1));
}

/// Then the quad grows from its top edge: `(0.1 + 4.5 (s - 0.8)) H`, whole at `s = 1`.
#[test]
fn the_quad_grows_down_from_its_top_and_is_plain_at_one() {
    close(window(0.8), (0.0, 0.1));
    close(window(0.9), (0.0, 0.55));
    close(window(0.99), (0.0, 0.1 + 0.19 * 4.5));
    assert_eq!(Vwipe::window(1.0), None);
    assert_eq!(Vwipe::window(3.0), None);
}

#[test]
fn the_alpha_is_a_linear_ramp_over_the_in_time() {
    let wipe = Vwipe {
        seconds: 0.7,
        delay: 0.0,
    };
    assert_eq!(wipe.alpha(-1.0), 0.0);
    assert!((wipe.alpha(0.35) - 0.5).abs() < 1e-6);
    assert_eq!(wipe.alpha(0.7), 1.0);
    assert_eq!(wipe.alpha(9.0), 1.0);
    assert_eq!(
        Vwipe {
            seconds: 0.0,
            delay: 0.0
        }
        .alpha(0.0),
        1.0
    );
}

fn parse_one(values: &str) -> Option<Vwipe> {
    let xml = format!(r#"<Image name="Logo"><Values {values}></Values></Image>"#);
    let root = crate::screen::parse(&xml);
    let node = root.children.first().expect("the image");
    Screens::from_xml_with_fallback_globals("", &[]).vwipe_from_node(node)
}

/// The Team Selection `Logo` authors `enebletransition="0.5"`, which matches no key, so the class
/// default stands ("How the XML selects the transition"); a correctly spelled key is read.
#[test]
fn a_misspelt_enable_transition_leaves_the_class_default() {
    let class = Some(Vwipe {
        seconds: IMAGE_RAMP_SECONDS,
        delay: 0.5,
    });
    assert_eq!(
        parse_one(r#"transitiontype="vwipe" delay="0.5" enebletransition="0.5""#),
        class
    );
    assert_eq!(
        parse_one(r#"TransitionType="VWipe" EnableTransition="0.5""#),
        Some(Vwipe {
            seconds: 0.5,
            delay: 0.0
        })
    );
    assert_eq!(
        parse_one(r#"transitiontype="vwipe" transition="0""#),
        Some(Vwipe {
            seconds: 0.0,
            delay: 0.0
        })
    );
}

#[test]
fn only_vwipe_is_a_vwipe() {
    assert_eq!(parse_one(r#"transitiontype="hscale""#), None);
    assert_eq!(parse_one(r#"x="0""#), None);
}

/// Racebox's `Item 315` (`delay` 0.45, in 0.5), read in RAM at 60 Hz: nothing for 26 frames, the
/// first ramp frame at 27 holding one `dt` (`s` = 1/30), whole 30 frames later
/// (`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`, "Page entry").
#[test]
fn the_ramp_starts_on_the_frame_the_countdown_crosses_zero() {
    let wipe = Vwipe {
        seconds: 0.5,
        delay: 0.45,
    };
    let at = |frame: u32| wipe.entry_alpha(frame as f32 / 60.0);
    assert_eq!(at(0), None);
    assert_eq!(at(26), None);
    assert!((at(27).expect("first ramp frame") - 1.0 / 30.0).abs() < 1e-5);
    assert!((at(42).expect("half way") - 16.0 / 30.0).abs() < 1e-5);
    assert_eq!(at(56), Some(1.0));
    assert_eq!(at(600), Some(1.0));
}

/// A clock summed a tick at a time lands within a rounding error of the frame, never a frame off.
#[test]
fn an_accumulated_clock_starts_the_ramp_on_the_same_frame() {
    for (delay, frames) in [(0.25_f32, 15), (0.45, 27), (0.5, 30), (0.6, 36), (1.2, 72)] {
        let wipe = Vwipe {
            seconds: 0.5,
            delay,
        };
        let mut clock = EntryClock::default();
        let mut first = None;
        for frame in 0..=frames + 2 {
            if first.is_none() && wipe.entry_alpha(clock.seconds()).is_some() {
                first = Some(frame);
            }
            clock.tick(FRAME_SECONDS);
        }
        assert_eq!(first, Some(frames), "delay {delay}");
    }
}

/// No delay authored: the ramp starts on the entry frame itself.
#[test]
fn a_widget_with_no_delay_starts_on_the_entry_frame() {
    let wipe = Vwipe {
        seconds: 0.5,
        delay: 0.0,
    };
    assert!(wipe.entry_alpha(0.0).is_some());
}

fn sprite() -> Draw {
    Draw::Sprite {
        rect: [10.0, 100.0, 200.0, 50.0],
        uv: [64.0, 32.0, 100.0, 40.0],
        color: [1.0; 4],
    }
}

/// The delay draws nothing (not the slab `Vwipe::window(0.0)` would give), the ramp cuts the
/// rect and the sampled rows together, and the finished page draws the quad as authored.
#[test]
fn an_entry_draw_waits_cuts_and_finishes() {
    let wipe = Some(Vwipe {
        seconds: 0.5,
        delay: 0.45,
    });
    assert_eq!(entry_draw(wipe, 0.2, sprite()), None);
    let first = entry_draw(wipe, 0.45, sprite()).expect("the first ramp frame");
    let Draw::Sprite { rect, uv, .. } = first else {
        panic!("a sprite");
    };
    let top = 2.25 / 30.0;
    assert!((rect[1] - (100.0 + 50.0 * top)).abs() < 1e-3, "{rect:?}");
    assert!((rect[3] - 5.0).abs() < 1e-4, "{rect:?}");
    assert!((uv[1] - (32.0 + 40.0 * top)).abs() < 1e-3, "{uv:?}");
    assert!((uv[3] - 4.0).abs() < 1e-4, "{uv:?}");
    assert_eq!((rect[0], rect[2], uv[0], uv[2]), (10.0, 200.0, 64.0, 100.0));
    assert_eq!(entry_draw(wipe, 3.0, sprite()), Some(sprite()));
    assert_eq!(entry_draw(None, 0.0, sprite()), Some(sprite()));
    // A tiled pattern is cut at the same fractions and keeps its tile size: 2 tiles down the
    // whole quad, a tenth of that down a tenth of it.
    let tiled = Draw::TiledSprite {
        rect: [10.0, 100.0, 200.0, 50.0],
        uv: [64.0, 32.0, 16.0, 16.0],
        repeat: [12.5, 2.0],
        color: [1.0; 4],
    };
    assert_eq!(entry_draw(wipe, 0.2, tiled.clone()), None);
    let Some(Draw::TiledSprite { rect, uv, repeat, .. }) = entry_draw(wipe, 0.45, tiled.clone())
    else {
        panic!("a tiled slab");
    };
    assert!((rect[1] - (100.0 + 50.0 * top)).abs() < 1e-3 && (rect[3] - 5.0).abs() < 1e-4);
    assert_eq!((uv, repeat[0]), ([64.0, 32.0, 16.0, 16.0], 12.5));
    assert!((repeat[1] - 0.2).abs() < 1e-5, "{repeat:?}");
    assert_eq!(entry_draw(wipe, 3.0, tiled.clone()), Some(tiled));
    assert_eq!(
        entry_draw(wipe, EntryClock::finished().seconds(), sprite()),
        Some(sprite())
    );
}

/// `Widget_ResetTransition` leaves the countdown alone: a page past its delay replays from the
/// restart (a slab on the press, whole after the in-time), a page still inside its delay keeps
/// waiting, and a widget never restarted draws as on entry.
#[test]
fn a_restart_replays_from_the_press_and_keeps_the_countdown() {
    let wipe = Some(Vwipe {
        seconds: 0.5,
        delay: 0.45,
    });
    let height = |draw: Option<Draw>| match draw {
        Some(Draw::Sprite { rect, .. }) => Some(rect[3]),
        other => panic!("{other:?}"),
    };
    let slab = height(restart_draw(wipe, 3.0, Some(0.0), sprite())).expect("drawn");
    assert!((slab - 5.0).abs() < 1e-4, "{slab}: a tenth of the 50 tall quad");
    assert_eq!(
        restart_draw(wipe, 3.0, Some(0.6), sprite()),
        Some(sprite()),
        "whole 0.5 s on"
    );
    assert_eq!(
        restart_draw(wipe, 0.2, Some(0.0), sprite()),
        None,
        "the opening delay still runs"
    );
    assert_eq!(
        restart_draw(wipe, 0.5, Some(0.3), sprite()),
        entry_draw(wipe, 0.5, sprite()),
        "a press from before the delay ended changes nothing: the ramp counts from the delay's end"
    );
    assert_eq!(
        restart_draw(wipe, 3.0, None, sprite()),
        entry_draw(wipe, 3.0, sprite())
    );
    assert_eq!(restart_draw(None, 0.0, Some(0.0), sprite()), Some(sprite()));
}
