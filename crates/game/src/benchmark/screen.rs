//! The stats screen: a panel over the frozen race with a verdict, a few plain lines and four
//! buttons; DETAILS swaps the lines for the full numbers and the settings.
//!
//! The same shape as the finished race's results table (`crate::scoreboard`): a draw list in the
//! built-in 480x272 grid, rasterised by that table's own overlay (`Overlay::draw_list`) in the face
//! that overlay brought into the grid (`crate::scoreboard::panel_face`). Every string is fitted to
//! its room by its measured width, so no face or translation can spill (`misfits`). The buttons have
//! hit rectangles in that grid, so a click or a touch lands on them the way a press does. Every word
//! is [`super::text`]'s, so each goes through a string id.

use oag_ui::font::Atlas;
use oag_ui::frontend::{Align, Draw};

use super::report::{Report, Verdict};
use super::text::{self, Say, words};

/// The panel: left edge, top edge and width in the 480x272 grid.
const PANEL_X: f32 = 30.0;
const PANEL_Y: f32 = 10.0;
const PANEL_W: f32 = 420.0;
const PAD: f32 = 8.0;
/// The panel's bottom edge.
const PANEL_BOTTOM: f32 = 268.0;
/// The panel in the grid, `[x, y, width, height]`: everything the screen draws is inside it.
pub const PANEL: [f32; 4] = [PANEL_X, PANEL_Y, PANEL_W, PANEL_BOTTOM - PANEL_Y];
/// The room between a button's edges and its label.
const BUTTON_PAD: f32 = 3.0;

const BACKDROP: [f32; 4] = [0.0, 0.0, 0.0, 0.82];
const TEXT: [f32; 4] = [0.92, 0.95, 1.0, 1.0];
const HEAD: [f32; 4] = [1.0, 0.83, 0.24, 1.0];
const CAPTION: [f32; 4] = [0.62, 0.68, 0.78, 1.0];
const GOOD: [f32; 4] = [0.5, 0.92, 0.55, 1.0];
const BAD: [f32; 4] = [1.0, 0.45, 0.4, 1.0];
const BORDER: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
const BUTTON: [f32; 4] = [0.16, 0.2, 0.28, 0.95];
const BUTTON_FOCUSED: [f32; 4] = [0.26, 0.4, 0.62, 1.0];

/// What the stats screen can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    /// Leave: back to where the benchmark was started from.
    Back,
    /// Run the same benchmark again.
    Again,
    /// Swap the plain lines for the full numbers and the settings, and back.
    Details,
    /// Put the result on the clipboard as text.
    Copy,
}

/// What the screen shows besides the report.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct View {
    /// The full numbers, not the plain lines.
    pub details: bool,
    /// Whether this platform can write the clipboard: COPY TO CLIPBOARD is on the screen only
    /// where it can.
    pub can_copy: bool,
    /// How the last copy went: `Some(true)` copied, `Some(false)` refused.
    pub copied: Option<bool>,
}

/// The buttons on screen, in the order the pad's focus walks them.
#[must_use]
pub fn buttons(can_copy: bool) -> Vec<Choice> {
    let mut out = vec![Choice::Details];
    if can_copy {
        out.push(Choice::Copy);
    }
    out.extend([Choice::Back, Choice::Again]);
    out
}

/// Where each button sits, `[x, y, width, height]` in the 480x272 grid: DETAILS and COPY TO
/// CLIPBOARD over BACK and RUN AGAIN. DETAILS takes its whole row where there is no COPY.
#[must_use]
pub fn button_rect(choice: Choice, can_copy: bool) -> [f32; 4] {
    let half = (PANEL_W - 3.0 * PAD) / 2.0;
    let left = PANEL_X + PAD;
    let right = PANEL_X + 2.0 * PAD + half;
    let (top, bottom) = (232.0, 250.0);
    match choice {
        Choice::Details if can_copy => [left, top, half, 16.0],
        Choice::Details => [left, top, PANEL_W - 2.0 * PAD, 16.0],
        Choice::Copy => [right, top, half, 16.0],
        Choice::Back => [left, bottom, half, 16.0],
        Choice::Again => [right, bottom, half, 16.0],
    }
}

/// The button under a point of the grid, if any.
#[must_use]
pub fn hit(x: f32, y: f32, can_copy: bool) -> Option<Choice> {
    buttons(can_copy).into_iter().find(|choice| {
        let [bx, by, bw, bh] = button_rect(*choice, can_copy);
        x >= bx && x < bx + bw && y >= by && y < by + bh
    })
}

/// The button the pad's focus moves to from `from`: forward or back round [`buttons`].
#[must_use]
pub fn step(from: Choice, forward: bool, can_copy: bool) -> Choice {
    let order = buttons(can_copy);
    let at = order.iter().position(|c| *c == from).unwrap_or(0);
    let next = if forward {
        (at + 1) % order.len()
    } else {
        (at + order.len() - 1) % order.len()
    };
    order[next]
}

/// The last three components of a path: `bench/run-1795576464/pulse.json`.
fn short_path(path: &str) -> String {
    let parts: Vec<&str> = path
        .split(['\\', '/'])
        .filter(|part| !part.is_empty())
        .collect();
    parts[parts.len().saturating_sub(3)..].join("/")
}

/// The width the panel's text runs across, in grid units.
const INNER_WIDTH: f32 = PANEL_W - 2.0 * PAD - 12.0;

/// `text` broken at spaces into lines `fits` accepts. A single word it refuses keeps a line to
/// itself, which the caller shrinks.
fn wrap(text: &str, fits: impl Fn(&str) -> bool) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if fits(&format!("{line} {word}")) => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

/// The text scale `text` is set at: `scale`, or smaller where it would run past `room`.
fn fitted(face: &Atlas, text: &str, scale: f32, room: f32) -> f32 {
    let width = oag_ui::font::measure(face, text);
    if width * scale > room {
        room / width
    } else {
        scale
    }
}

fn line_of(
    face: &Atlas,
    x: f32,
    y: f32,
    scale: f32,
    align: Align,
    color: [f32; 4],
    text: String,
) -> Draw {
    Draw::Text {
        x,
        y,
        scale: fitted(face, &text, scale, INNER_WIDTH),
        color,
        border: Some(BORDER),
        align,
        text,
        wrap_width: None,
    }
}

fn verdict_colour(verdict: Verdict) -> [f32; 4] {
    match verdict {
        Verdict::Smooth => GOOD,
        Verdict::MostlySmooth => HEAD,
        Verdict::Choppy => BAD,
    }
}

/// What the panel reads over its buttons at one text scale, and where the last line ends.
struct Body {
    draws: Vec<Draw>,
    bottom: f32,
}

fn body(report: &Report, face: &Atlas, scale: f32, say: Say<'_>, view: View) -> Body {
    let line = (face.line_height + 2.0).max(10.0) * scale;
    let centre = PANEL_X + PANEL_W / 2.0;
    let left = PANEL_X + PAD + 6.0;
    let right = PANEL_X + PANEL_W - PAD - 6.0;
    let at = |x: f32, y: f32, align: Align, color: [f32; 4], text: String| {
        line_of(face, x, y, scale, align, color, text)
    };
    let wrapped = |text: &str| {
        wrap(text, |part| {
            oag_ui::font::measure(face, part) * scale <= INNER_WIDTH
        })
    };
    let title = text::say(
        say,
        if view.details {
            words::DETAILS
        } else {
            words::COMPLETE
        },
        &[],
    );
    let mut y = PANEL_Y + PAD;
    let mut draws = vec![at(centre, y, Align::Centre, HEAD, title)];
    y += line * 1.3;
    draws.push(at(
        centre,
        y,
        Align::Centre,
        CAPTION,
        text::head(report, say),
    ));
    y += line * 1.5;
    if view.details {
        let details = text::details(report, say);
        let split = details.rows.len().div_ceil(2);
        let column = (right - left - 8.0) / 2.0;
        for (n, (label, value)) in details.rows.iter().enumerate() {
            let (x, row) = if n < split {
                (left, n)
            } else {
                (left + column + 8.0, n - split)
            };
            let ry = y + line * row as f32;
            // The label and its value share the column, with a space between them.
            let pair = format!("{label}  {value}");
            let row_scale = fitted(face, &pair, scale, column);
            for (x, align, color, text) in [
                (x, Align::Left, CAPTION, label.clone()),
                (x + column, Align::Right, TEXT, value.clone()),
            ] {
                draws.push(line_of(face, x, ry, row_scale, align, color, text));
            }
        }
        y += line * (split as f32 + 0.5);
        for (label, value) in &details.machine {
            for part in wrapped(&format!("{label}: {value}")) {
                draws.push(at(left, y, Align::Left, CAPTION, part));
                y += line;
            }
        }
    } else {
        let verdict = report.figures().map(|figures| figures.verdict());
        for (n, summary) in text::summary(report, say).iter().enumerate() {
            let colour = verdict.filter(|_| n == 0).map(verdict_colour);
            for part in wrapped(summary) {
                draws.push(at(left, y, Align::Left, colour.unwrap_or(TEXT), part));
                y += line;
            }
            // A gap under the verdict, which stands apart from the lines.
            if colour.is_some() {
                y += line * 0.6;
            }
        }
    }
    Body { draws, bottom: y }
}

/// The draw list of the screen for `report`, set in `face`, with `focus` on a button.
#[must_use]
pub fn draw_list(
    report: &Report,
    face: &Atlas,
    focus: Choice,
    saved: Option<&str>,
    say: Say<'_>,
    view: View,
) -> Vec<Draw> {
    // The saved path sits on its own line over the buttons, and the body ends above it.
    let saved_at = |scale: f32| 222.0 - (face.line_height + 2.0).max(10.0) * scale;
    let limit = |scale: f32| {
        if saved.is_some() {
            saved_at(scale)
        } else {
            226.0
        }
    };
    let mut scale = 1.0_f32;
    let mut text = body(report, face, scale, say, view);
    while scale > 0.1 && text.bottom > limit(scale) {
        scale -= 0.05;
        text = body(report, face, scale, say, view);
    }
    let mut out = vec![Draw::Fill {
        rect: PANEL,
        color: BACKDROP,
    }];
    out.append(&mut text.draws);
    if let Some(saved) = saved {
        out.push(line_of(
            face,
            PANEL_X + PAD + 6.0,
            saved_at(scale),
            scale,
            Align::Left,
            CAPTION,
            text::say(say, words::SAVED, &[&short_path(saved)]).to_uppercase(),
        ));
    }
    let label = |choice: Choice| -> String {
        let word = match (choice, view.copied) {
            (Choice::Back, _) => words::BACK,
            (Choice::Again, _) => words::AGAIN,
            (Choice::Details, _) if view.details => words::SUMMARY,
            (Choice::Details, _) => words::DETAILS,
            (Choice::Copy, Some(true)) => words::COPIED,
            (Choice::Copy, Some(false)) => words::COPY_FAILED,
            (Choice::Copy, None) => words::COPY,
        };
        text::say(say, word, &[])
    };
    for choice in buttons(view.can_copy) {
        let rect = button_rect(choice, view.can_copy);
        out.push(Draw::Fill {
            rect,
            color: if choice == focus {
                BUTTON_FOCUSED
            } else {
                BUTTON
            },
        });
        let text = label(choice);
        let scale = fitted(face, &text, 1.0, rect[2] - 2.0 * BUTTON_PAD);
        out.push(Draw::Text {
            x: rect[0] + rect[2] / 2.0,
            y: rect[1] + (rect[3] - face.line_height * scale) / 2.0,
            scale,
            color: TEXT,
            border: Some(BORDER),
            align: Align::Centre,
            text,
            wrap_width: None,
        });
    }
    out
}

/// What of `list`, set in `face`, does not fit the screen: a string outside the panel, a button
/// label outside its button, text over the buttons, or two strings drawn over each other. Empty
/// when all of it fits.
#[must_use]
pub fn misfits(list: &[Draw], face: &Atlas, can_copy: bool) -> Vec<String> {
    // Rounding slack, in grid units.
    const SLACK: f32 = 0.01;
    let boxes: Vec<(&str, [f32; 4])> = list
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text {
                x,
                y,
                scale,
                align,
                text,
                ..
            } => {
                let width = oag_ui::font::measure(face, text) * scale;
                let left = match align {
                    Align::Left => *x,
                    Align::Centre => *x - width / 2.0,
                    Align::Right => *x - width,
                };
                Some((
                    text.as_str(),
                    [left, *y, left + width, *y + face.line_height * scale],
                ))
            }
            _ => None,
        })
        .collect();
    let inside = |b: &[f32; 4], r: [f32; 4]| {
        b[0] >= r[0] - SLACK && b[1] >= r[1] - SLACK && b[2] <= r[2] + SLACK && b[3] <= r[3] + SLACK
    };
    let rects: Vec<[f32; 4]> = buttons(can_copy)
        .into_iter()
        .map(|choice| {
            let [x, y, w, h] = button_rect(choice, can_copy);
            [x, y, x + w, y + h]
        })
        .collect();
    let buttons_top = rects.iter().map(|r| r[1]).fold(PANEL_BOTTOM, f32::min);
    let mut out = Vec::new();
    for (n, (text, b)) in boxes.iter().enumerate() {
        if !inside(b, [PANEL_X, PANEL_Y, PANEL_X + PANEL_W, PANEL_BOTTOM]) {
            out.push(format!("{text:?} leaves the panel: {b:?}"));
        }
        // A string reaching the button rows is a button's label and stays inside its button.
        if b[3] > buttons_top + SLACK && !rects.iter().any(|r| inside(b, *r)) {
            out.push(format!("{text:?} runs over the buttons: {b:?}"));
        }
        for (other, c) in &boxes[n + 1..] {
            if b[0] < c[2] - SLACK
                && c[0] < b[2] - SLACK
                && b[1] < c[3] - SLACK
                && c[1] < b[3] - SLACK
            {
                out.push(format!("{text:?} is drawn over {other:?}"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_on_a_button_is_that_button() {
        for can_copy in [true, false] {
            for choice in buttons(can_copy) {
                let [x, y, w, h] = button_rect(choice, can_copy);
                assert_eq!(hit(x + w / 2.0, y + h / 2.0, can_copy), Some(choice));
                assert_eq!(hit(x - 1.0, y + h / 2.0, can_copy), None);
            }
        }
        assert_eq!(hit(240.0, 100.0, true), None);
    }

    #[test]
    fn copy_is_a_button_only_where_the_clipboard_can_be_written() {
        assert!(buttons(true).contains(&Choice::Copy));
        assert!(!buttons(false).contains(&Choice::Copy));
        let [x, y, w, h] = button_rect(Choice::Copy, true);
        assert_ne!(hit(x + w / 2.0, y + h / 2.0, false), Some(Choice::Copy));
    }

    /// The face the metrics of a title's caption face give, after the overlay's own conversion
    /// into the panel's grid: `line` units tall, a capital `advance` wide.
    fn face_of(line: f32, advance: f32) -> Atlas {
        let built_in = Atlas::build();
        let mean = oag_ui::font::measure(&built_in, "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ") / 36.0;
        let mut face = built_in.with_texel_scale(advance / mean);
        face.line_height = line;
        crate::scoreboard::panel_face(&face)
    }

    /// A report with the long strings a phone or a laptop reports.
    fn long_report() -> Report {
        use crate::benchmark::report::tests::{COUNTDOWN_FRAMES, report};
        let frames = std::iter::repeat_n(70_000.0, COUNTDOWN_FRAMES + 1500);
        let mut report = report(frames);
        report.context.title = "Wipeout: Omega Collection";
        report.context.track_name = Some("Talon's Junction White Reverse".into());
        report.context.adapter_name = "AMD Radeon RX 7800 XT (RADV NAVI32)".into();
        report.context.driver = "radv Mesa 25.3.1-arch1.1".into();
        report.context.device = "linux x86_64, AMD Ryzen 9 7900 12-Core Processor".into();
        report.context.details = (0..18)
            .map(|n| (format!("ANISOTROPIC FILTERING {n}"), "ORIGINAL".to_string()))
            .collect();
        report
    }

    #[test]
    fn every_title_face_fits_both_pages_even_in_a_long_language() {
        let report = long_report();
        let english = |_: &str, fallback: &str| fallback.to_string();
        // Every string twice over stands in for a translation that runs long.
        let long = |_: &str, fallback: &str| format!("{fallback} {fallback}");
        // Pulse, Pure, HD and 2048's caption faces: line height and mean capital advance.
        for (line, advance) in [(10.0, 6.3), (12.0, 7.0), (34.0, 21.25), (34.0, 24.7)] {
            let face = face_of(line, advance);
            for details in [false, true] {
                for say in [&english as Say<'_>, &long as Say<'_>] {
                    let view = View {
                        details,
                        can_copy: true,
                        copied: None,
                    };
                    let list = draw_list(
                        &report,
                        &face,
                        Choice::Back,
                        Some("/a/bench/run-1795576464/omega.json"),
                        say,
                        view,
                    );
                    let bad = misfits(&list, &face, true);
                    assert!(
                        bad.is_empty(),
                        "face {line}/{advance} details={details}: {bad:#?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_face_the_grid_was_not_drawn_for_is_reported_as_not_fitting() {
        // The check itself: HD's face at its own size spills out of the panel.
        let mut face = Atlas::build().with_texel_scale(3.5);
        face.line_height = 34.0;
        let draws = [Draw::Text {
            x: 240.0,
            y: 235.0,
            scale: 1.0,
            color: TEXT,
            border: None,
            align: Align::Centre,
            text: "COPY TO CLIPBOARD".into(),
            wrap_width: None,
        }];
        assert!(!misfits(&draws, &face, true).is_empty());
    }

    fn texts(can_copy: bool, details: bool) -> Vec<String> {
        use crate::benchmark::report::tests::{COUNTDOWN_FRAMES, report};
        let frames = std::iter::repeat_n(16_600.0, COUNTDOWN_FRAMES + 50);
        let english = |_: &str, fallback: &str| fallback.to_string();
        let view = View {
            can_copy,
            details,
            copied: None,
        };
        draw_list(
            &report(frames),
            &Atlas::build(),
            Choice::Back,
            None,
            &english,
            view,
        )
        .into_iter()
        .filter_map(|draw| match draw {
            Draw::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect()
    }

    #[test]
    fn the_screen_offers_copy_only_where_the_clipboard_can_be_written() {
        assert!(texts(true, false).contains(&"COPY TO CLIPBOARD".to_string()));
        assert!(!texts(false, false).contains(&"COPY TO CLIPBOARD".to_string()));
    }

    #[test]
    fn details_swaps_the_plain_lines_for_the_numbers() {
        let plain = texts(true, false);
        assert!(plain.iter().any(|t| t.starts_with("SMOOTH")), "{plain:?}");
        assert!(
            plain.contains(&"DETAILS".to_string()),
            "the button: {plain:?}"
        );
        let full = texts(true, true);
        assert!(full.contains(&"AVERAGE FPS".to_string()), "{full:?}");
        assert!(
            full.contains(&"SUMMARY".to_string()),
            "the button: {full:?}"
        );
        assert!(!full.iter().any(|t| t.starts_with("SMOOTH")), "{full:?}");
    }

    #[test]
    fn the_pad_walks_every_button_and_wraps() {
        for can_copy in [true, false] {
            let order = buttons(can_copy);
            let mut at = order[0];
            for expected in order.iter().skip(1).chain(std::iter::once(&order[0])) {
                at = step(at, true, can_copy);
                assert_eq!(at, *expected);
            }
            assert_eq!(step(order[0], false, can_copy), *order.last().unwrap());
        }
    }

    #[test]
    fn a_saved_path_is_shown_by_its_tail() {
        assert_eq!(
            short_path("/home/a/.local/share/oag/bench/run-1/pulse.json"),
            "bench/run-1/pulse.json"
        );
        assert_eq!(short_path("pulse.json"), "pulse.json");
    }

    #[test]
    fn long_lines_wrap_inside_the_panel() {
        let lines = wrap("one two three four five six seven", |line| line.len() <= 12);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.len() <= 12), "{lines:?}");
        assert_eq!(lines.join(" "), "one two three four five six seven");
    }
}
