//! `Cell Help`'s three paragraphs flow: each authors `PushHeight="true"`, so its `y` is a gap below
//! the paragraph before (`docs/ui/campaign-screens.md`, "Back and Help on the PS2, and how `Cell Help` is laid out").

use super::*;
use oag_ui::font::{Atlas, wrap};

const WORDS: &str = "details and medal requirements for this event are listed to the right of \
                     the grid and for races that feature ships you can change their difficulty";

fn help_layout() -> Layout {
    let xml = format!(
        r#"<Screen name="Top"><Screen name="Cell Help">
<Viewport><Values x="0" y="10" height="252" width="300"></Values>
<Animation focus="true">
<Text name="Main Help" PushHeight="true"><Values String="{WORDS}" widthLimited="1" font="small" x="20" y="15"></Values></Text>
<Text name="Speed Class Help" PushHeight="true"><Values String="{WORDS}" widthLimited="1" font="small" x="20" y="10"></Values></Text>
<Text name="Event Help" PushHeight="true"><Values String="short" widthLimited="1" font="small" x="20" y="10"></Values></Text>
</Animation></Viewport></Screen></Screen>"#
    );
    Layout::read(
        &Screens::from_xml(&xml),
        "Cell Help",
        &strings(),
        crate::picker::FaceScales::default(),
        PSP_GRID,
    )
    .unwrap()
}

/// `(top, scale, wrapped text)` of every text draw.
fn paragraphs(draws: &[Draw]) -> Vec<(f32, f32, String)> {
    draws
        .iter()
        .filter_map(|draw| match draw {
            Draw::Text { y, scale, text, .. } => Some((*y, *scale, text.clone())),
            _ => None,
        })
        .collect()
}

/// Each paragraph starts its authored `y` below the last line of the one before. Fails if the
/// paragraphs are drawn at their raw `y`, which stacks all three near the panel's top.
#[test]
fn each_paragraph_starts_below_the_one_before_by_its_authored_gap() {
    let atlas = Atlas::build();
    let draws = cell_help_draw(&help_layout(), &|_| None, &atlas);
    let paragraphs = paragraphs(&draws);
    assert_eq!(paragraphs.len(), 3);
    let authored_gap = [15.0, 10.0, 10.0];
    let mut bottom = 0.0;
    for (index, (top, scale, text)) in paragraphs.iter().enumerate() {
        assert!(
            (top - (bottom + authored_gap[index])).abs() < 1e-3,
            "paragraph {index} starts at {top}, the one before ends at {bottom}"
        );
        let lines = wrap(&atlas, text, *scale, 300.0).len();
        bottom = top + lines as f32 * atlas.line_height * scale;
    }
    assert!(
        wrap(&atlas, WORDS, paragraphs[0].1, 300.0).len() > 1,
        "the fixture's first paragraph wraps, or the test proves nothing"
    );
}

/// The panel's own backdrop is among the draws: it is what dims the page behind the text.
#[test]
fn the_panel_draws_its_dimming_backdrop() {
    let xml = r#"<Screen name="Top"><Screen name="Cell Help">
<Image><Values y="10" width="480" height="252" Color="0xE0000000"></Values></Image>
</Screen></Screen>"#;
    let layout = Layout::read(
        &Screens::from_xml(xml),
        "Cell Help",
        &strings(),
        crate::picker::FaceScales::default(),
        PSP_GRID,
    )
    .unwrap();
    let draws = cell_help_draw(&layout, &|_| None, &Atlas::build());
    assert!(
        draws.iter().any(|draw| matches!(
            draw,
            Draw::Fill { color, .. } if (color[3] - 224.0 / 255.0).abs() < 1e-3
        )),
        "{draws:?}"
    );
}
