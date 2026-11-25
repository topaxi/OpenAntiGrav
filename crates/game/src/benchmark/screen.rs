//! The stats screen: a panel over the frozen race with the run's figures and two buttons.
//!
//! The same shape as the finished race's results table (`crate::scoreboard`): a draw list in the
//! built-in 480x272 grid, rasterised by that table's own overlay (`Overlay::draw_list`). The buttons have hit rectangles
//! in that grid, so a click or a touch lands on them the way a press does.

use oag_ui::frontend::{Align, Draw};

use super::report::{Figures, Report};

/// The panel: left edge, top edge and width in the 480x272 grid.
const PANEL_X: f32 = 30.0;
const PANEL_Y: f32 = 10.0;
const PANEL_W: f32 = 420.0;
const PAD: f32 = 8.0;

const PANEL: [f32; 4] = [0.0, 0.0, 0.0, 0.82];
const TEXT: [f32; 4] = [0.92, 0.95, 1.0, 1.0];
const HEAD: [f32; 4] = [1.0, 0.83, 0.24, 1.0];
const CAPTION: [f32; 4] = [0.62, 0.68, 0.78, 1.0];
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
}

/// Where each button sits, `[x, y, width, height]` in the 480x272 grid.
#[must_use]
pub fn button_rect(choice: Choice) -> [f32; 4] {
    let y = 250.0;
    match choice {
        Choice::Back => [PANEL_X + PAD, y, 110.0, 16.0],
        Choice::Again => [PANEL_X + PANEL_W - PAD - 110.0, y, 110.0, 16.0],
    }
}

/// The button under a point of the grid, if any.
#[must_use]
pub fn hit(x: f32, y: f32) -> Option<Choice> {
    [Choice::Back, Choice::Again].into_iter().find(|choice| {
        let [bx, by, bw, bh] = button_rect(*choice);
        x >= bx && x < bx + bw && y >= by && y < by + bh
    })
}

/// Settings, joined into lines of at most `width` characters.
fn settings_lines(settings: &[(String, String)], width: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for (key, value) in settings {
        let item = format!("{} {}", key.replace('_', " "), value).to_uppercase();
        match lines.last_mut() {
            Some(line) if line.len() + item.len() + 2 <= width => {
                line.push_str("  ");
                line.push_str(&item);
            }
            _ => lines.push(item),
        }
    }
    lines
}

/// The circuit's own name out of `"Title: Data\\Environments\\16_Track\\track.vex"`: `16_Track`.
fn track_label(track: &str) -> String {
    let entry = track.rsplit(": ").next().unwrap_or(track);
    let parts: Vec<&str> = entry.split(['\\', '/']).collect();
    match parts.len() {
        0 | 1 => entry.to_string(),
        n => parts[n - 2].to_string(),
    }
}

/// The last three components of a path: `bench/run-1795576464/pulse.json`.
fn short_path(path: &str) -> String {
    let parts: Vec<&str> = path.split(['\\', '/']).filter(|part| !part.is_empty()).collect();
    parts[parts.len().saturating_sub(3)..].join("/")
}

/// Characters a line of the panel holds at text scale one: measured against the Pulse HUD face
/// at the panel's width.
const WIDTH_AT_ONE: f32 = 52.0;

/// The draw list of the screen for `report`, with `focus` on a button.
#[must_use]
pub fn draw_list(report: &Report, line_height: f32, focus: Choice, saved: Option<&str>) -> Vec<Draw> {
    let context = &report.context;
    let figures = report.figures();
    // Rows the panel owes, in lines: heading, circuit line, the figures (or one sentence), the
    // settings, the adapter, the device and the file. The text is scaled down until they fit
    // above the buttons.
    let base = (line_height + 2.0).max(10.0);
    let rows = if figures.is_some() { 9.5 } else { 2.5 };
    let wrapped = |scale: f32| settings_lines(&context.settings, (WIDTH_AT_ONE / scale) as usize).len();
    let lines_at = |scale: f32| 2.7 + rows + wrapped(scale) as f32 + 0.5 + 3.0;
    let mut scale = 1.0_f32;
    while scale > 0.45 && lines_at(scale) * base * scale > 232.0 - PANEL_Y {
        scale -= 0.05;
    }
    let line = base * scale;
    let text = |x: f32, y: f32, align: Align, color: [f32; 4], text: String| Draw::Text {
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
    let mut y = PANEL_Y + PAD;
    out.push(text(
        PANEL_X + PANEL_W / 2.0,
        y,
        Align::Centre,
        HEAD,
        "BENCHMARK COMPLETE".to_string(),
    ));
    y += line * 1.3;
    let head = format!(
        "{}  {}  {}",
        context.title.to_uppercase(),
        track_label(context.track.as_deref().unwrap_or("")).to_uppercase(),
        context
            .preset
            .as_deref()
            .map_or_else(|| "YOUR SETTINGS".to_string(), |name| format!("PRESET {}", name.to_uppercase())),
    );
    out.push(text(PANEL_X + PANEL_W / 2.0, y, Align::Centre, CAPTION, head));
    y += line * 1.4;

    let left = PANEL_X + PAD + 6.0;
    let right = PANEL_X + PANEL_W - PAD - 6.0;
    let row = |out: &mut Vec<Draw>, y: f32, label: &str, value: String| {
        out.push(text(left, y, Align::Left, CAPTION, label.to_string()));
        out.push(text(right, y, Align::Right, TEXT, value));
    };
    if let Some(Figures {
        counted,
        average_fps,
        low_1,
        low_01,
        median_ms,
        p95_ms,
        p99_ms,
        worst_ms,
        over_budget,
        budget_ms,
    }) = figures
    {
        row(&mut out, y, "AVERAGE FPS", format!("{average_fps:.1}"));
        y += line;
        row(&mut out, y, "1% LOW FPS", format!("{low_1:.1}"));
        y += line;
        row(&mut out, y, "0.1% LOW FPS", format!("{low_01:.1}"));
        y += line;
        row(&mut out, y, "FRAME TIME MEDIAN", format!("{median_ms:.2} MS"));
        y += line;
        row(&mut out, y, "FRAME TIME P95", format!("{p95_ms:.2} MS"));
        y += line;
        row(&mut out, y, "FRAME TIME P99", format!("{p99_ms:.2} MS"));
        y += line;
        row(&mut out, y, "FRAME TIME WORST", format!("{worst_ms:.2} MS"));
        y += line;
        row(
            &mut out,
            y,
            &format!("OVER THE {budget_ms:.1} MS BUDGET"),
            format!("{over_budget} OF {counted}"),
        );
        y += line * 1.5;
    } else {
        out.push(text(
            left,
            y,
            Align::Left,
            TEXT,
            "THE RACE ENDED BEFORE ENOUGH FRAMES WERE DRAWN".to_string(),
        ));
        y += line * 1.5;
    }
    for settings in settings_lines(&context.settings, (WIDTH_AT_ONE / scale) as usize) {
        out.push(text(left, y, Align::Left, CAPTION, settings));
        y += line;
    }
    y += line * 0.5;
    out.push(text(
        left,
        y,
        Align::Left,
        TEXT,
        format!("{} ({})", context.adapter_name, context.driver).to_uppercase(),
    ));
    y += line;
    out.push(text(left, y, Align::Left, CAPTION, context.device.to_uppercase()));
    y += line;
    if let Some(saved) = saved {
        out.push(text(left, y, Align::Left, CAPTION, format!("SAVED {}", short_path(saved)).to_uppercase()));
    }
    for (choice, label) in [(Choice::Back, "BACK"), (Choice::Again, "RUN AGAIN")] {
        let rect = button_rect(choice);
        out.push(Draw::Fill {
            rect,
            color: if choice == focus { BUTTON_FOCUSED } else { BUTTON },
        });
        out.push(text(
            rect[0] + rect[2] / 2.0,
            rect[1] + 3.0,
            Align::Centre,
            TEXT,
            label.to_string(),
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_on_a_button_is_that_button() {
        for choice in [Choice::Back, Choice::Again] {
            let [x, y, w, h] = button_rect(choice);
            assert_eq!(hit(x + w / 2.0, y + h / 2.0), Some(choice));
            assert_eq!(hit(x - 1.0, y + h / 2.0), None);
        }
        assert_eq!(hit(240.0, 100.0), None);
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
    fn the_circuit_is_named_by_its_directory() {
        assert_eq!(
            track_label("Wipeout Pulse: Data\\Environments\\16_Track\\track.vex"),
            "16_Track"
        );
        assert_eq!(track_label(""), "");
    }

    #[test]
    fn settings_wrap_inside_the_panel() {
        let settings: Vec<(String, String)> = (0..12)
            .map(|n| (format!("setting_{n}"), "value".to_string()))
            .collect();
        let lines = settings_lines(&settings, 52);
        assert!(lines.len() > 1);
        assert!(lines.iter().all(|line| line.len() <= 52), "{lines:?}");
    }
}
