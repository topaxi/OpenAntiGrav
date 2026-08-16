//! What the frame pacing in [`super`] is asserted to do: what the ring
//! remembers and what the percentile reports, what each overlay mode draws and
//! where, and the frame limit's own spelling and range.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `perf.rs`: the tests are 295 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

/// Sixty frames of exactly 1/60 s.
fn steady(meter: &mut Meter, frames: usize, seconds: f32) {
    for _ in 0..frames {
        meter.record(seconds);
    }
}

#[test]
fn an_empty_meter_reports_nothing_rather_than_infinity() {
    let meter = Meter::new();
    assert!(meter.is_empty());
    assert_eq!(meter.stats(), None);
    assert!(draw_list(&meter, Overlay::Pacing, 60, None, None).is_empty());
}

#[test]
fn a_steady_sixty_reads_as_sixty() {
    let mut meter = Meter::new();
    steady(&mut meter, WINDOW, 1.0 / 60.0);
    let stats = meter.stats().expect("fed");
    assert!((stats.fps - 60.0).abs() < 0.01, "{}", stats.fps);
    assert!((stats.mean_ms - 16.667).abs() < 0.01, "{}", stats.mean_ms);
    assert!((stats.worst_ms - 16.667).abs() < 0.01, "{}", stats.worst_ms);
}

/// The whole reason the graph exists: alternating 8 ms and 25 ms averages
/// out to a comfortable 16.5 ms and looks awful.
#[test]
fn an_average_hides_the_stutter_the_percentile_shows() {
    let mut meter = Meter::new();
    for i in 0..WINDOW {
        meter.record(if i % 2 == 0 { 0.008 } else { 0.025 });
    }
    let stats = meter.stats().expect("fed");
    assert!((stats.mean_ms - 16.5).abs() < 0.01, "{}", stats.mean_ms);
    assert!(stats.fps > 60.0, "{}", stats.fps);
    assert!((stats.p99_ms - 25.0).abs() < 0.01, "{}", stats.p99_ms);
}

/// One catastrophic frame must not be what the p99 reports, or the figure
/// is just `worst_ms` under another name.
#[test]
fn one_outlier_moves_the_maximum_and_not_the_percentile() {
    let mut meter = Meter::new();
    steady(&mut meter, WINDOW - 1, 1.0 / 60.0);
    meter.record(0.5);
    let stats = meter.stats().expect("fed");
    assert!((stats.worst_ms - 500.0).abs() < 0.01, "{}", stats.worst_ms);
    assert!(stats.p99_ms < 17.0, "{}", stats.p99_ms);
}

#[test]
fn the_ring_forgets_the_oldest_frame_and_keeps_the_order() {
    let mut meter = Meter::new();
    for i in 0..WINDOW + 10 {
        meter.record((i + 1) as f32 / 10_000.0);
    }
    let frames: Vec<f32> = meter.frames().collect();
    assert_eq!(frames.len(), WINDOW);
    assert!((frames[0] - 11.0 / 10_000.0).abs() < 1e-9, "{}", frames[0]);
    let last = frames[WINDOW - 1];
    assert!(
        (last - (WINDOW + 10) as f32 / 10_000.0).abs() < 1e-9,
        "{last}"
    );
}

/// A clock that goes backwards, a zero-length frame and a NaN are all
/// things a wall clock can hand over, and none of them is a frame.
#[test]
fn a_nonsense_duration_is_dropped_and_a_stall_is_clamped() {
    let mut meter = Meter::new();
    meter.record(-1.0);
    meter.record(0.0);
    meter.record(f32::NAN);
    meter.record(f32::INFINITY);
    assert!(meter.is_empty());

    meter.record(30.0);
    let stats = meter.stats().expect("fed");
    assert!(
        (stats.worst_ms - LONGEST * 1000.0).abs() < 0.01,
        "{}",
        stats.worst_ms
    );
}

#[test]
fn clearing_forgets_the_load_that_was_not_a_frame() {
    let mut meter = Meter::new();
    meter.record(0.9);
    meter.clear();
    assert!(meter.is_empty());
    steady(&mut meter, 10, 1.0 / 60.0);
    let stats = meter.stats().expect("fed");
    assert!(stats.worst_ms < 17.0, "{}", stats.worst_ms);
}

#[test]
fn off_draws_nothing_however_full_the_meter_is() {
    let mut meter = Meter::new();
    steady(&mut meter, WINDOW, 1.0 / 60.0);
    assert!(draw_list(&meter, Overlay::Off, 60, None, None).is_empty());
}

/// The counter is one panel and one line; the pacing view adds a second
/// line, the rule and one column per remembered frame.
#[test]
fn each_mode_draws_exactly_what_it_promises() {
    let mut meter = Meter::new();
    steady(&mut meter, WINDOW, 1.0 / 60.0);

    let fps = draw_list(&meter, Overlay::Fps, 60, None, None);
    assert_eq!(fps.len(), 2);
    assert!(matches!(fps[0], Draw::Fill { .. }));
    assert!(matches!(&fps[1], Draw::Text { text, .. } if text.starts_with("60 FPS")));

    let pacing = draw_list(&meter, Overlay::Pacing, 60, None, None);
    // panel + two lines + the rule + one column per frame
    assert_eq!(pacing.len(), 4 + WINDOW);
    assert!(matches!(&pacing[2], Draw::Text { text, .. } if text.starts_with("P99")));
}

/// `Dev` adds the scene line and the video line, and only when a caller
/// actually has one to give it - a stage with no scene and no movie
/// still gets exactly what `Pacing` would have shown it.
#[test]
fn dev_adds_a_line_for_whichever_of_scene_and_video_it_is_given() {
    let mut meter = Meter::new();
    steady(&mut meter, WINDOW, 1.0 / 60.0);

    let bare = draw_list(&meter, Overlay::Dev, 60, None, None);
    // Same shape as `Pacing` with nothing extra: panel + two lines + the
    // rule + one column per frame.
    assert_eq!(bare.len(), 4 + WINDOW);

    let scene = crate::race::SceneStats {
        draws_submitted: 12,
        draws_culled: 3,
        triangles: 4096,
    };
    let with_scene = draw_list(&meter, Overlay::Dev, 60, Some(scene), None);
    assert!(
        with_scene
            .iter()
            .any(|d| matches!(d, Draw::Text { text, .. } if text == "DRAWS 12/15  TRIS 4096"))
    );

    let with_video = draw_list(&meter, Overlay::Dev, 60, None, Some("gstreamer"));
    assert!(
        with_video
            .iter()
            .any(|d| matches!(d, Draw::Text { text, .. } if text == "VIDEO gstreamer"))
    );

    let with_both = draw_list(&meter, Overlay::Dev, 60, Some(scene), Some("av1 cache"));
    assert!(
        with_both
            .iter()
            .any(|d| matches!(d, Draw::Text { text, .. } if text == "VIDEO av1 cache"))
    );
    assert!(
        with_both
            .iter()
            .any(|d| matches!(d, Draw::Text { text, .. } if text == "DRAWS 12/15  TRIS 4096"))
    );
}

/// Every rectangle has to land inside the panel it is drawn under, or the
/// overlay writes over the game somewhere nobody looked.
#[test]
fn nothing_is_drawn_outside_the_panel() {
    let mut meter = Meter::new();
    for i in 0..WINDOW {
        meter.record(if i == 7 { 0.5 } else { 1.0 / 60.0 });
    }
    let list = draw_list(&meter, Overlay::Pacing, 60, None, None);
    let Draw::Fill { rect: panel, .. } = list[0] else {
        panic!("the panel comes first");
    };
    for draw in &list[1..] {
        let Draw::Fill { rect, .. } = draw else {
            continue;
        };
        assert!(rect[0] >= panel[0], "{rect:?} starts left of {panel:?}");
        assert!(
            rect[0] + rect[2] <= panel[0] + panel[2],
            "{rect:?} runs past {panel:?}"
        );
        assert!(rect[1] >= panel[1], "{rect:?} starts above {panel:?}");
        assert!(
            rect[1] + rect[3] <= panel[1] + panel[3] + 0.001,
            "{rect:?} runs below {panel:?}"
        );
    }
}

/// The graph is drawn against the target, not against 60 Hz, and the
/// difference is the whole readability of it: at a 240 limit a 4.2 ms frame
/// has to fill half the box, and against 60 the same frame is a sliver at
/// the floor that says nothing about pacing at all.
#[test]
fn the_graph_is_scaled_to_the_target_it_is_given() {
    let mut meter = Meter::new();
    steady(&mut meter, WINDOW, 1.0 / 240.0);

    let heights = |target| {
        draw_list(&meter, Overlay::Pacing, target, None, None)
            .into_iter()
            .skip(4) // the panel, two lines of text and the rule
            .filter_map(|draw| match draw {
                Draw::Fill { rect, color } => Some((rect[3], color)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };

    let (tall, colour) = heights(240)[0];
    assert!((tall - GRAPH_H / 2.0).abs() < 0.1, "{tall}");
    assert_eq!(colour, GOOD, "a frame exactly at the target is on time");

    let (sliver, _) = heights(60)[0];
    assert!(sliver < GRAPH_H / 6.0, "{sliver}");
}

/// A frame at the target is on time, one a little over is late, and one
/// past the graph's ceiling is the colour that says so.
#[test]
fn the_bands_split_where_they_say_they_do() {
    assert_eq!(band(16.6, 16.667), GOOD);
    assert_eq!(band(17.4, 16.667), GOOD);
    assert_eq!(band(20.0, 16.667), WARN);
    assert_eq!(band(33.3, 16.667), WARN);
    assert_eq!(band(40.0, 16.667), BAD);
}

#[test]
fn every_frame_limit_survives_a_round_trip_through_its_own_spelling() {
    for limit in FrameLimit::OFFERED {
        assert_eq!(limit.to_string().parse::<FrameLimit>(), Ok(limit));
    }
    assert_eq!(FrameLimit::UNLIMITED.to_string(), "unlimited");
    assert_eq!("UNLIMITED".parse(), Ok(FrameLimit::UNLIMITED));
}

/// Zero is not a second spelling of unlimited, and the ceiling is a
/// ceiling. Both would otherwise land in the settings file as a value the
/// menus cannot select and the loop cannot honour.
#[test]
fn a_frame_limit_outside_the_range_is_refused() {
    assert!("0".parse::<FrameLimit>().is_err());
    assert!("1001".parse::<FrameLimit>().is_err());
    assert!("-1".parse::<FrameLimit>().is_err());
    assert!("fast".parse::<FrameLimit>().is_err());
    assert_eq!(
        "1000".parse(),
        Ok(FrameLimit::OFFERED[FrameLimit::OFFERED.len() - 1])
    );
}

/// The default has to be one of the values the menus offer, or the row
/// draws whatever happens to be first and the first nudge of it persists
/// that as a deliberate choice.
#[test]
fn the_default_limit_is_one_the_menus_can_show() {
    assert_eq!(FrameLimit::default(), FrameLimit::DEFAULT);
    assert!(
        FrameLimit::OFFERED.contains(&FrameLimit::DEFAULT),
        "{} is not on the list the menus offer",
        FrameLimit::DEFAULT
    );
    assert_eq!(FrameLimit::DEFAULT.hz(), Some(240));
}

#[test]
fn unlimited_has_no_period_to_wait_out() {
    assert_eq!(FrameLimit::UNLIMITED.hz(), None);
    assert_eq!(FrameLimit::UNLIMITED.period(), None);
    let sixty: FrameLimit = "60".parse().expect("parse");
    assert_eq!(sixty.hz(), Some(60));
    assert_eq!(
        sixty.period(),
        Some(std::time::Duration::from_nanos(16_666_666))
    );
}

#[test]
fn every_mode_survives_a_round_trip_through_its_own_spelling() {
    for mode in Overlay::ALL {
        assert_eq!(mode.to_string().parse::<Overlay>(), Ok(mode));
    }
    assert!("graph".parse::<Overlay>().is_err());
}
