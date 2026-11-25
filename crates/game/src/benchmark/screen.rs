//! The stats screen: a panel over the frozen race with a verdict, a few plain lines and four
//! buttons; DETAILS swaps the lines for the full numbers and the settings.
//!
//! The same shape as the finished race's results table (`crate::scoreboard`): a draw list in the
//! built-in 480x272 grid, rasterised by that table's own overlay (`Overlay::draw_list`). The buttons have hit rectangles
//! in that grid, so a click or a touch lands on them the way a press does. Every word is
//! [`super::text`]'s, so each goes through a string id.

use oag_ui::frontend::{Align, Draw};

use super::report::{Report, Verdict};
use super::text::{self, Say, words};

/// The panel: left edge, top edge and width in the 480x272 grid.
const PANEL_X: f32 = 30.0;
const PANEL_Y: f32 = 10.0;
const PANEL_W: f32 = 420.0;
const PAD: f32 = 8.0;

const PANEL: [f32; 4] = [0.0, 0.0, 0.0, 0.82];
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

/// How many characters of `char_width` a line holds at text `scale`.
fn chars_per_line(char_width: f32, scale: f32) -> usize {
    (INNER_WIDTH / (char_width.max(1.0) * scale)) as usize
}

/// `text` broken at spaces into lines of at most `width` characters.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split_whitespace() {
        match lines.last_mut() {
            Some(line) if line.len() + 1 + word.len() <= width => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    lines
}

fn verdict_colour(verdict: Verdict) -> [f32; 4] {
    match verdict {
        Verdict::Smooth => GOOD,
        Verdict::MostlySmooth => HEAD,
        Verdict::Choppy => BAD,
    }
}

/// The text lines under the heading at one text scale, each with the colour that marks it, if
/// any: the verdict's, or nothing.
type Lines = Vec<(Option<[f32; 4]>, String)>;

/// The draw list of the screen for `report`, with `focus` on a button.
#[must_use]
pub fn draw_list(
    report: &Report,
    line_height: f32,
    char_width: f32,
    focus: Choice,
    saved: Option<&str>,
    say: Say<'_>,
    view: View,
) -> Vec<Draw> {
    let base = (line_height + 2.0).max(10.0);
    let verdict = report.figures().map(|figures| figures.verdict());
    let summary = text::summary(report, say);
    let details = text::details(report, say);
    let lines_at = |scale: f32| -> Lines {
        let width = chars_per_line(char_width, scale);
        if view.details {
            details
                .machine
                .iter()
                .flat_map(|(label, value)| wrap(&format!("{label}: {value}"), width))
                .map(|line| (None, line))
                .collect()
        } else {
            summary
                .iter()
                .enumerate()
                .flat_map(|(n, line)| {
                    let colour = if n == 0 {
                        verdict.map(verdict_colour)
                    } else {
                        None
                    };
                    wrap(line, width)
                        .into_iter()
                        .map(move |part| (colour, part))
                })
                .collect()
        }
    };
    let split = details.rows.len().div_ceil(2);
    // Rows the panel owes, in lines: heading, caption, the body, a gap, the saved path.
    let owed = |lines: &Lines, scale: f32| -> f32 {
        let body = if view.details {
            split as f32 + 0.5 + lines.len() as f32
        } else {
            lines.len() as f32 + 0.5
        };
        (2.8 + body + 1.0) * base * scale
    };
    let free = 226.0 - PANEL_Y - PAD;
    let mut scale = 1.0_f32;
    let mut lines = lines_at(scale);
    while scale > 0.1 && owed(&lines, scale) > free {
        scale -= 0.05;
        lines = lines_at(scale);
    }
    let line = base * scale;
    let t = |x: f32, y: f32, align: Align, color: [f32; 4], text: String| Draw::Text {
        x,
        y,
        scale,
        color,
        border: Some(BORDER),
        align,
        text,
        wrap_width: None,
    };
    let mut out = vec![Draw::Fill {
        rect: [PANEL_X, PANEL_Y, PANEL_W, 268.0 - PANEL_Y],
        color: PANEL,
    }];
    let centre = PANEL_X + PANEL_W / 2.0;
    let left = PANEL_X + PAD + 6.0;
    let right = PANEL_X + PANEL_W - PAD - 6.0;
    let mut y = PANEL_Y + PAD;
    let title = text::say(
        say,
        if view.details {
            words::DETAILS
        } else {
            words::COMPLETE
        },
        &[],
    );
    out.push(t(centre, y, Align::Centre, HEAD, title));
    y += line * 1.3;
    out.push(t(
        centre,
        y,
        Align::Centre,
        CAPTION,
        text::head(report, say),
    ));
    y += line * 1.5;
    if view.details {
        let column = (right - left - 8.0) / 2.0;
        for (n, (label, value)) in details.rows.iter().enumerate() {
            let (x, row) = if n < split {
                (left, n)
            } else {
                (left + column + 8.0, n - split)
            };
            let ry = y + line * row as f32;
            out.push(t(x, ry, Align::Left, CAPTION, label.clone()));
            out.push(t(x + column, ry, Align::Right, TEXT, value.clone()));
        }
        y += line * (split as f32 + 0.5);
        for (_, text) in lines {
            out.push(t(left, y, Align::Left, CAPTION, text));
            y += line;
        }
    } else {
        for (colour, text) in lines {
            let first = colour.is_some();
            out.push(t(left, y, Align::Left, colour.unwrap_or(TEXT), text));
            y += line * if first { 1.6 } else { 1.0 };
        }
    }
    if let Some(saved) = saved {
        out.push(t(
            left,
            222.0 - line,
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
        out.push(Draw::Text {
            x: rect[0] + rect[2] / 2.0,
            y: rect[1] + 3.0,
            scale: 1.0,
            color: TEXT,
            border: Some(BORDER),
            align: Align::Centre,
            text: label(choice),
            wrap_width: None,
        });
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
            8.0,
            6.0,
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
        let lines = wrap("one two three four five six seven", 12);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.len() <= 12), "{lines:?}");
        assert_eq!(lines.join(" "), "one two three four five six seven");
    }
}
