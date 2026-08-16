//! What the loading screen in [`super`] is asserted to do: the disc's own tips
//! and how they wrap and elide, the bar and the counts each phase draws, and
//! the fade that ends it.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `loading.rs`: the tests are 569 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

fn progress(done: usize, total: usize) -> Progress {
    Progress {
        planning: false,
        total,
        done,
        cached: 0,
        failed: 0,
        current: None,
        finished: false,
    }
}

fn text_of(list: &[Draw]) -> Vec<String> {
    list.iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

/// The tips come out of the XML in the order the file lists them, with the
/// title's own id left alone.
#[test]
fn a_loading_definition_yields_one_tip_per_entry() {
    let xml = r#"<Screen name="Top">
            <PI_LoadingScreen name="Bomb">
              <Title TitleSrc="PRO_STATS_BOMB"></Title>
              <Text StringSrc="TIP_ONE"></Text>
            </PI_LoadingScreen>
            <PI_LoadingScreen name="Mines">
              <Text StringSrc="TIP_TWO"></Text>
            </PI_LoadingScreen>
          </Screen>"#;
    let strings = StringTable::from_xml(
        r#"<Entries>
                 <Entry ID="TIP_ONE" String="first"></Entry>
                 <Entry ID="TIP_TWO" String="second"></Entry>
                 <Entry ID="PRO_STATS_BOMB" String="BOMB"></Entry>
               </Entries>"#,
    );
    assert_eq!(tips(xml, &strings), ["first", "second"]);
}

/// An id the chosen language does not carry drops out rather than being
/// drawn as itself - see [`tips`].
#[test]
fn an_unresolved_tip_is_dropped_rather_than_shown_as_its_id() {
    let xml = r#"<Screen><PI_LoadingScreen><Text StringSrc="MSC_LOAD_MISSING"></Text>
          </PI_LoadingScreen></Screen>"#;
    assert!(tips(xml, &StringTable::default()).is_empty());
}

/// The tips this project's own file nests one level down; a reader that
/// only looked at the root's children would find them anyway, so the guard
/// is a deeper nesting.
#[test]
fn tips_are_found_however_deeply_the_xml_nests_them() {
    let xml = r#"<Screen><Group><Inner>
            <PI_LoadingScreen><Text StringSrc="TIP"></Text></PI_LoadingScreen>
          </Inner></Group></Screen>"#;
    let strings = StringTable::from_xml(r#"<E><Entry ID="TIP" String="deep"></Entry></E>"#);
    assert_eq!(tips(xml, &strings), ["deep"]);
}

/// Every line fits, and nothing is lost or duplicated in the middle.
#[test]
fn wrapping_fits_every_line_and_keeps_every_word() {
    let atlas = Atlas::build();
    let text = "Actuating the left or right airbrake through a corner turns tighter \
                without shedding speed";
    let lines = wrap(&atlas, text, 1.0, 200.0);
    assert!(lines.len() > 1, "{lines:?}");
    for line in &lines {
        assert!(
            font::measure(&atlas, line) <= 200.0,
            "{line:?} is {} wide",
            font::measure(&atlas, line)
        );
    }
    assert_eq!(
        lines.join(" "),
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    );
}

/// A word that cannot fit is left over-long rather than cut in half or
/// dropped.
#[test]
fn a_word_wider_than_the_column_is_kept_whole() {
    let atlas = Atlas::build();
    let lines = wrap(&atlas, "antigravitational", 1.0, 10.0);
    assert_eq!(lines, ["antigravitational"]);
}

/// The tail survives, which is the half that names the asset.
#[test]
fn eliding_keeps_the_end_of_a_label() {
    let atlas = Atlas::build();
    let label = r"Data.wad Data\Movies\Intro.PMF";
    // Wide enough for the marker and the tail together: a column narrower
    // than `ELISION` plus what is worth keeping has nothing to return but
    // the marker and a fragment, which is the degenerate case rather than
    // the one worth pinning.
    let short = elide(&atlas, label, 1.0, 120.0);
    assert!(short.starts_with(ELISION), "{short:?}");
    assert!(short.ends_with("Intro.PMF"), "{short:?}");
    assert!(short.len() < label.len(), "{short:?}");
    assert!(font::measure(&atlas, &short) <= 120.0);
}

/// The marker has to be drawable, which is the whole reason it is three
/// periods - see [`ELISION`]. Checked against the built-in atlas here; the
/// disc's fonts are a superset of ASCII, and a font missing `.` could not
/// draw a file name either.
#[test]
fn the_elision_marker_is_a_glyph_the_atlas_can_draw() {
    let atlas = Atlas::build();
    assert!(font::measure(&atlas, ELISION) > 0.0);
    for ch in ELISION.chars() {
        assert!(atlas.cell(ch).is_some(), "{ch:?} has no cell");
    }
}

#[test]
fn a_label_that_already_fits_is_left_alone() {
    let atlas = Atlas::build();
    assert_eq!(elide(&atlas, "Data.wad", 1.0, 400.0), "Data.wad");
}

/// The three things the screen exists to say are all in the list.
#[test]
fn the_draw_list_states_the_counts_the_fraction_and_the_asset() {
    let atlas = Atlas::build();
    let screen = Screen::new(vec!["a tip".to_string()]);
    let mut state = progress(37, 115);
    state.cached = 12;
    state.current = Some("Data.wad hash:71d3c1ec".to_string());
    let drawn = text_of(&screen.draw_list(Phase::Prefetch, &state, &atlas));

    assert!(
        drawn.iter().any(|line| line.contains("37 / 115")),
        "{drawn:?}"
    );
    assert!(
        drawn.iter().any(|line| line.contains("12 already cached")),
        "{drawn:?}"
    );
    assert!(drawn.iter().any(|line| line == "32%"), "{drawn:?}");
    assert!(
        drawn.iter().any(|line| line.contains("hash:71d3c1ec")),
        "{drawn:?}"
    );
    assert!(drawn.iter().any(|line| line == "a tip"), "{drawn:?}");
}

/// **Nothing to count draws no counter at all.** A title whose chain names
/// no movie, on a run with no `--prefetch`, has nothing behind this screen
/// and every number on it would be zero. The bar, the `0 / 0` and the `100%`
/// are one row and are all absent together; the tip and the wave are what is
/// left.
#[test]
fn a_boot_with_nothing_to_convert_draws_no_bar_and_no_counts() {
    let atlas = Atlas::build();
    let screen = Screen::new(vec!["a tip".to_string()]);
    // What `Session::prefetch_progress` reports on a run with no worker,
    // and what `MediaPlan::loads` counts for a plan that names nothing.
    let state = Progress {
        finished: true,
        ..Progress::default()
    };
    let list = screen.draw_list(Phase::Media(None), &state, &atlas);
    let drawn = text_of(&list);

    assert!(
        !drawn.iter().any(|line| line.contains("loaded")),
        "no done/total pair: {drawn:?}"
    );
    assert!(
        !drawn.iter().any(|line| line.ends_with('%')),
        "no percentage: {drawn:?}"
    );
    assert!(drawn.iter().any(|line| line == "a tip"), "{drawn:?}");
    assert!(
        drawn.iter().any(|line| line == "LOADING"),
        "the heading says what is happening rather than READY: {drawn:?}"
    );
    // The backdrop fill is the only one left: no trough, no filled half.
    let fills = list
        .iter()
        .filter(|draw| matches!(draw, Draw::Fill { .. }))
        .count();
    assert_eq!(fills, 1, "only the backdrop, no bar: {list:?}");
}

/// **The ordinary boot draws the bar again**, because the media phase now
/// counts its own loads: `Session::loading_progress` folds
/// [`crate::boot::MediaWorker::progress`] in, and a plan naming both movies
/// and a backdrop is five loads rather than nothing.
///
/// The regression this pins is the one that removed it: the phase reported
/// no counts, `counted` read that as the empty case, and the whole row went
/// with it.
#[test]
fn the_media_phase_draws_the_bar_and_counts_its_own_loads() {
    let atlas = Atlas::build();
    let screen = Screen::new(Vec::new());
    let state = Progress {
        total: 5,
        done: 2,
        current: Some("WO_INTRO.PMF".to_string()),
        ..Progress::default()
    };
    let list = screen.draw_list(Phase::Media(None), &state, &atlas);
    let drawn = text_of(&list);

    assert!(
        drawn.iter().any(|line| line == "2 / 5 loaded"),
        "the media phase loads rather than converts: {drawn:?}"
    );
    assert!(drawn.iter().any(|line| line == "40%"), "{drawn:?}");
    assert!(
        drawn.iter().any(|line| line.contains("WO_INTRO.PMF")),
        "the reel being read is named: {drawn:?}"
    );
    assert!(
        drawn.iter().any(|line| line == "LOADING MOVIES"),
        "not CONVERTING ASSETS, which is the other phase: {drawn:?}"
    );
    // The backdrop, the trough and the filled part.
    let fills = list
        .iter()
        .filter(|draw| matches!(draw, Draw::Fill { .. }))
        .count();
    assert_eq!(fills, 3, "{list:?}");
}

/// Nothing counted yet is not `100%` - see [`percentage`].
#[test]
fn planning_reports_no_percentage_it_cannot_know() {
    let atlas = Atlas::build();
    let screen = Screen::new(Vec::new());
    let state = Progress {
        planning: true,
        ..Progress::default()
    };
    let drawn = text_of(&screen.draw_list(Phase::Prefetch, &state, &atlas));
    assert!(!drawn.iter().any(|line| line.ends_with('%')), "{drawn:?}");
    assert!(
        drawn.iter().any(|line| line == "READING THE ARCHIVES"),
        "{drawn:?}"
    );
}

/// The bar's filled half is drawn only once there is a real fraction, and
/// it is exactly the fraction wide.
#[test]
fn the_bar_is_the_fraction_wide() {
    let atlas = Atlas::build();
    let screen = Screen::new(Vec::new());
    let fills: Vec<[f32; 4]> = screen
        .draw_list(Phase::Prefetch, &progress(1, 4), &atlas)
        .iter()
        .filter_map(|draw| match draw {
            Draw::Fill { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    // The backdrop, the trough and the fill.
    assert_eq!(fills.len(), 3, "{fills:?}");
    assert!((fills[2][2] - BAR_WIDTH * 0.25).abs() < 0.01, "{fills:?}");
}

/// **A transcode says so, and says how far.** The complaint this answers is
/// that a cache hit and an eighty-second `ffmpeg` run looked identical: same
/// heading, same entry name, one of them over before it was read.
#[test]
fn a_transcode_is_headed_and_counted_apart_from_a_cache_hit() {
    let atlas = Atlas::build();
    let screen = Screen::new(Vec::new());
    let state = Progress {
        total: 5,
        done: 2,
        current: Some(r"Data\Movies\Intro.pss".to_string()),
        ..Progress::default()
    };

    let transcoding = text_of(&screen.draw_list(
        Phase::Media(Some(crate::movie::Step::Transcoding {
            done: 340,
            total: Some(1200),
        })),
        &state,
        &atlas,
    ));
    assert!(
        transcoding.iter().any(|l| l == "TRANSCODING MOVIES"),
        "{transcoding:?}"
    );
    assert!(
        transcoding
            .iter()
            .any(|l| l == "transcoding - frame 340 of 1200"),
        "the frame counter is the anti-hang signal: {transcoding:?}"
    );

    let cached = text_of(&screen.draw_list(
        Phase::Media(Some(crate::movie::Step::Cached)),
        &state,
        &atlas,
    ));
    assert!(cached.iter().any(|l| l == "LOADING MOVIES"), "{cached:?}");
    assert!(
        cached.iter().any(|l| l.contains("reading the cache")),
        "{cached:?}"
    );
    assert!(
        !cached.iter().any(|l| l.contains("transcoding")),
        "a cache hit must not borrow the last transcode's caption: {cached:?}"
    );

    // Nothing reported yet, and the sound loads, which never report.
    let quiet = text_of(&screen.draw_list(Phase::Media(None), &state, &atlas));
    assert!(quiet.iter().any(|l| l == "LOADING MOVIES"), "{quiet:?}");
    assert!(
        !quiet
            .iter()
            .any(|l| l.contains("transcoding") || l.contains("decoding") || l.contains("cache")),
        "no step is no line, not a line saying there is no step: {quiet:?}"
    );
}

/// An uncapped conversion has a numerator and no denominator, and must not
/// invent one - see [`crate::movie::Step::Transcoding`].
#[test]
fn an_uncapped_transcode_counts_frames_without_claiming_a_total() {
    let atlas = Atlas::build();
    let screen = Screen::new(Vec::new());
    let drawn = text_of(&screen.draw_list(
        Phase::Media(Some(crate::movie::Step::Transcoding {
            done: 91,
            total: None,
        })),
        &progress(1, 2),
        &atlas,
    ));
    assert!(
        drawn.iter().any(|l| l == "transcoding - frame 91"),
        "{drawn:?}"
    );
}

/// **The bar crosses its slice as the transcode runs**, rather than sitting
/// still for a minute and then stepping. See [`fraction`].
#[test]
fn a_transcode_fills_its_own_slice_of_the_bar() {
    // Two of five loads done, so the third owns 40%..60%.
    let state = Progress {
        total: 5,
        done: 2,
        ..Progress::default()
    };
    let at = |done, total| {
        fraction(
            Phase::Media(Some(crate::movie::Step::Transcoding { done, total })),
            &state,
        )
    };

    // The head, before a frame is encoded: started, but barely.
    let started = at(0, Some(1200));
    assert!(
        (started - (0.4 + 0.2 / 5.0)).abs() < 1e-6,
        "the read-or-decode head is LOADING_SHARE of one slice: {started}"
    );
    // Half the frames is half of what is left of the slice.
    let half = at(600, Some(1200));
    assert!(
        (half - (0.4 + (0.2 + 0.8 * 0.5) / 5.0)).abs() < 1e-6,
        "{half}"
    );
    // The last frame lands exactly on the slice boundary, which is where
    // `done + 1` puts it a moment later.
    let end = at(1200, Some(1200));
    assert!((end - 0.6).abs() < 1e-6, "{end}");

    assert!(started < half && half < end, "{started} {half} {end}");
}

/// A slice is never left behind and never overshot, whatever a step reports.
///
/// The property the whole scheme rests on: a bar that went backwards would
/// be worse than one that only stepped, and `Cached` taking a whole slice is
/// the case where that is easiest to get wrong.
#[test]
fn no_step_moves_the_bar_backwards_or_past_its_own_slice() {
    use crate::movie::Step;

    let state = Progress {
        total: 4,
        done: 1,
        ..Progress::default()
    };
    let floor = fraction(Phase::Media(None), &state);
    let ceiling = fraction(
        Phase::Media(Some(Step::Cached)),
        &Progress {
            done: 2,
            ..state.clone()
        },
    );
    assert!((floor - 0.25).abs() < 1e-6, "{floor}");

    for step in [
        Step::Cached,
        Step::Decoding,
        Step::Transcoding {
            done: 0,
            total: None,
        },
        Step::Transcoding {
            done: 999,
            total: None,
        },
        // A stated total of zero: nobody could divide by it, and it must not
        // produce a NaN width either.
        Step::Transcoding {
            done: 0,
            total: Some(0),
        },
        Step::Transcoding {
            done: 7,
            total: Some(7),
        },
        // More frames than the total claimed, which `ffmpeg` has no reason
        // to report but which must not push past the slice if it does.
        Step::Transcoding {
            done: 99,
            total: Some(7),
        },
    ] {
        let at = fraction(Phase::Media(Some(step)), &state);
        assert!(at.is_finite(), "{step:?} gave {at}");
        assert!(
            (floor..=floor + 0.25 + 1e-6).contains(&at),
            "{step:?} left the slice 0.25..0.50: {at}"
        );
        assert!(at <= ceiling + 1e-6, "{step:?} passed the next load: {at}");
    }
}

/// The figure and the width are one statement, so they are one number.
#[test]
fn the_percentage_is_the_width_the_bar_was_drawn_at() {
    let atlas = Atlas::build();
    let screen = Screen::new(Vec::new());
    let phase = Phase::Media(Some(crate::movie::Step::Transcoding {
        done: 600,
        total: Some(1200),
    }));
    let state = Progress {
        total: 5,
        done: 2,
        ..Progress::default()
    };
    let list = screen.draw_list(phase, &state, &atlas);

    let widths: Vec<f32> = list
        .iter()
        .filter_map(|draw| match draw {
            Draw::Fill { rect, .. } => Some(rect[2]),
            _ => None,
        })
        .collect();
    // The backdrop, the trough and the fill.
    assert_eq!(widths.len(), 3, "{list:?}");
    let drawn = (widths[2] / BAR_WIDTH * 100.0).round();

    let text = text_of(&list);
    assert!(
        text.iter().any(|line| line == &format!("{drawn}%")),
        "the bar is {drawn}% wide and the figures beside it say: {text:?}"
    );
}

/// The wave freezes when the work does, and only then - `Wave::finish` is
/// the original's `g_loading_finished` and this is the thing that sets it.
#[test]
fn finishing_freezes_the_wave_and_starts_the_fade() {
    let mut screen = Screen::new(Vec::new());
    for _ in 0..5 {
        screen.advance(false);
    }
    let running = screen.wave;
    assert!(!running.is_finished());
    assert_eq!(screen.opacity(), 1.0);
    assert!(!screen.is_done());

    screen.advance(true);
    assert!(screen.wave.is_finished());
    assert_eq!(
        screen.wave.phase(),
        running.phase(),
        "a frozen wave stands still"
    );
    assert!(screen.opacity() < 1.0);

    for _ in 0..FADE_FRAMES {
        screen.advance(true);
    }
    assert!(screen.is_done());
    assert_eq!(screen.opacity(), 0.0);
}

/// The fade reaches every colour on the screen, not only the wave's tint.
#[test]
fn the_fade_dims_the_whole_list() {
    let atlas = Atlas::build();
    let mut screen = Screen::new(Vec::new());
    for _ in 0..FADE_FRAMES / 2 {
        screen.advance(true);
    }
    let list = screen.draw_list(Phase::Prefetch, &progress(1, 2), &atlas);
    let alpha = list
        .iter()
        .find_map(|draw| match draw {
            Draw::Text { color, .. } => Some(color[3]),
            _ => None,
        })
        .expect("a line of text");
    assert!(alpha > 0.0 && alpha < 1.0, "{alpha}");
}

/// Rotation is by time, wraps round, and a source with no tips draws none
/// rather than dividing by zero.
#[test]
fn tips_rotate_and_wrap() {
    let mut screen = Screen::new(vec!["one".to_string(), "two".to_string()]);
    assert_eq!(screen.tip(), Some("one"));
    for _ in 0..TIP_FRAMES {
        screen.advance(false);
    }
    assert_eq!(screen.tip(), Some("two"));
    for _ in 0..TIP_FRAMES {
        screen.advance(false);
    }
    assert_eq!(screen.tip(), Some("one"));

    assert_eq!(Screen::new(Vec::new()).tip(), None);
}

/// Every column of the wave is there, three bands deep, and two frames of a
/// running wave differ - the walk is re-drawn rather than integrated.
#[test]
fn the_wave_produces_a_fresh_band_every_frame() {
    let mut screen = Screen::new(Vec::new());
    for _ in 0..5 {
        screen.advance(false);
    }
    let first = screen.quads();
    let second = screen.quads();
    assert_eq!(
        first.len(),
        oag_render::loading::COLUMNS * oag_render::loading::BANDS
    );
    assert_ne!(first, second);
}
