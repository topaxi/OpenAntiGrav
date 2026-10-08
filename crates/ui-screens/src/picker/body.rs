//! [`body`] on its own: the panel, its rows, the arrows and the counter,
//! split out of `picker.rs` once that file passed the 1,000-line rule -
//! `scripts/check-file-size.py`, which is the rule as a gate. A move, not a
//! behaviour change.

use oag_ui::frontend::{Align, Draw, Placed};
use oag_ui::menu::Skin;
use oag_ui::screen::{Text, argb_to_rgba};

use super::{Details, Kind, Layout, Picker, Rating, entry_rows, fill_draw, is_title, wrap_name};

/// Everything under the title: the panel, its rows, the arrows and the
/// counter, in the XML's own paint order - fills, then images, then texts.
pub(super) fn body(
    picker: &Picker,
    layout: &Layout,
    skin: &Skin,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    measure: &dyn Fn(&str) -> f32,
) -> Vec<Draw> {
    let mut out = Vec::new();
    let screen = &layout.screen;
    // The loyalty block - its title row and its bar row - is drawn only for
    // a team that carries a running total ([`Details::Ship::loyalty`]); a
    // title that keeps none leaves it out whole, since an empty bar would
    // read as a loyalty of zero. Both rows are found by their named widget
    // and everything sharing that row goes.
    let loyalty = screen
        .images
        .iter()
        .find(|image| image.name.as_deref() == Some("Loyalty Bar"))
        .map(|image| [image.x, image.y]);
    let loyalty_rows: Vec<f32> = loyalty
        .map(|[_, y]| y)
        .into_iter()
        .chain(
            screen
                .texts
                .iter()
                .filter(|text| text.name.as_deref() == Some("Loyalty"))
                .map(|text| text.y),
        )
        .collect();
    let on_loyalty_row = |y: f32| loyalty_rows.iter().any(|row| (row - y).abs() < 1.0);

    for fill in &screen.fills {
        out.push(fade(
            fill_draw(fill),
            fade_in(fill.transition, picker.seconds()),
        ));
    }

    let rating = match picker.selected().map(|entry| &entry.details) {
        Some(Details::Ship { rating, .. }) => *rating,
        _ => None,
    };
    let loyalty_total = match picker.selected().map(|entry| &entry.details) {
        Some(Details::Ship { loyalty, .. }) => *loyalty,
        _ => None,
    };
    let variants = picker.variants().len();
    for image in &screen.images {
        // The loyalty bar and its own dim backing share a rect; neither is
        // drawn when there is no counter to show.
        if loyalty_total.is_none() && on_loyalty_row(image.y) {
            continue;
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        let mut fraction = 1.0;
        let mut color = argb_to_rgba(image.color);
        color[3] *= fade_in(image.transition, picker.seconds());
        match image.name.as_deref() {
            Some("Loyalty Bar") => {
                let (Some(total), Some(width)) = (loyalty_total, image.width) else {
                    continue;
                };
                fraction = loyalty_bar_pixels(total) / width.max(1.0);
            }
            Some(name) if name.ends_with(" Bar") => {
                let Some(rating) = rating else { continue };
                let stat = name.trim_end_matches(" Bar");
                let bars = rating.bars();
                let Some((_, value)) = bars.iter().find(|(bar, _)| *bar == stat) else {
                    continue;
                };
                fraction = Rating::fraction(*value);
            }
            Some("skin left arrow" | "skin right arrow") => {
                if variants == 0 {
                    continue;
                }
                // Dim with nothing to cycle to, the way the capture shows a
                // fresh profile's single livery.
                if variants < 2 {
                    color[3] *= 0.35;
                }
            }
            _ => {}
        }
        // A bar backing with no rating to draw beside it is left out too:
        // an empty ten-segment backing reads as a rating of zero.
        if image.name.is_none()
            && rating.is_none()
            && screen.images.iter().any(|bar| {
                bar.name.as_deref().is_some_and(|n| n.ends_with(" Bar"))
                    && bar.y == image.y
                    && bar.x == image.x
            })
        {
            continue;
        }
        let width = image.width.unwrap_or(placed.width as f32);
        let height = image.height.unwrap_or(placed.height as f32);
        let sampled = [
            image.texture_width.unwrap_or(placed.width as f32),
            image.texture_height.unwrap_or(placed.height as f32),
        ];
        // A sub-rect larger than the texture is the texture repeated:
        // `Infohexgrid` samples 340x120 of a 32x16 tile. Anything else is
        // the ordinary one-patch sprite.
        if sampled[0] > placed.width as f32 + 0.5 || sampled[1] > placed.height as f32 + 0.5 {
            out.push(Draw::TiledSprite {
                rect: [image.x, image.y, width * fraction, height],
                uv: [
                    placed.x as f32,
                    placed.y as f32,
                    placed.width as f32,
                    placed.height as f32,
                ],
                repeat: [
                    sampled[0] * fraction / placed.width.max(1) as f32,
                    sampled[1] / placed.height.max(1) as f32,
                ],
                color,
            });
            continue;
        }
        out.push(Draw::Sprite {
            rect: [image.x, image.y, width * fraction, height],
            uv: [
                placed.x as f32 + image.u.unwrap_or(0.0),
                placed.y as f32 + image.v.unwrap_or(0.0),
                sampled[0] * fraction,
                sampled[1],
            ],
            color,
        });
    }

    out.extend(entry_rows(picker, layout, skin));

    let Some(entry) = picker.selected() else {
        return out;
    };
    let name_lines = match picker.kind() {
        Kind::Track => wrap_name(&entry.label, layout, measure),
        Kind::Ship => vec![entry.label.clone()],
    };
    for text in &screen.texts {
        if loyalty_total.is_none() && on_loyalty_row(text.y) {
            continue;
        }
        // The title is chrome - see [`Layout::read`] - not a body widget.
        if is_title(text) {
            continue;
        }
        let name = text.name.as_deref().unwrap_or("");
        let content: Option<String> = match name {
            "honey" => Some(format!(
                "{} / {}",
                picker.index() + 1,
                picker.entries().len()
            )),
            // `Info Track <lines>.<line>`: the one block whose line count
            // matches the wrapped name is drawn, and only its own lines.
            _ if name.starts_with("Info Track ") => {
                let mut parts = name["Info Track ".len()..].split('.');
                let lines: usize = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                let line: usize = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                match picker.kind() {
                    Kind::Track if lines == name_lines.len() && line >= 1 => {
                        name_lines.get(line - 1).cloned()
                    }
                    _ => None,
                }
            }
            "Info1" | "Info2" | "Info3" => match &entry.details {
                Details::Track { info, .. } => {
                    let row = usize::from(name.as_bytes()[4] - b'1');
                    info.get(row).cloned()
                }
                Details::Ship { .. } => None,
            },
            "Speed" | "Thrust" | "Handling" | "Shield" => rating.and_then(|rating| {
                rating
                    .bars()
                    .iter()
                    .find(|(bar, _)| *bar == name)
                    .map(|(_, value)| value.to_string())
            }),
            "skin" => picker.variant().map(|(_, label)| label.clone()),
            // Only ever shown for a forced or suggested craft, and for a
            // team with nothing to cycle - neither of which this build has
            // a rule for yet.
            "skin fixed" | "Suggest" => None,
            "Loyalty" => loyalty_total.map(|total| total.to_string()),
            _ => text.string.clone(),
        };
        let Some(content) = content else { continue };
        // A stat title beside a row this entry has no value for is left
        // out with it, or the panel reads as a row with a blank.
        if name.is_empty()
            && rating.is_none()
            && screen.texts.iter().any(|value| {
                value.y == text.y
                    && matches!(
                        value.name.as_deref(),
                        Some("Speed" | "Thrust" | "Handling" | "Shield")
                    )
            })
        {
            continue;
        }
        out.push(fade(
            text_draw(text, &content, layout),
            fade_in(text.transition, picker.seconds()),
        ));
    }
    // The team's own name sits where the disc's `List` puts its rows - the
    // panel's inner left edge, three down - which no `Text` widget authors.
    if picker.kind() == Kind::Ship {
        // Not authored by any widget of its own - see above - so it fades
        // in with whatever panel text is, the `skin` label beside it.
        let skin_text = screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some("skin"));
        let color = skin_text.map_or([1.0, 1.0, 1.0, 1.0], |text| argb_to_rgba(text.color));
        let alpha = fade_in(
            skin_text.map_or(0.0, |text| text.transition),
            picker.seconds(),
        );
        out.push(fade(
            Draw::Text {
                x: layout.panel[0] + 14.0 * layout.scale[0],
                y: layout.panel[1] + 3.0 * layout.scale[1],
                scale: 1.0,
                color,
                border: None,
                align: Align::Left,
                text: entry.label.clone(),
                wrap_width: None,
            },
            alpha,
        ));
    }
    out
}

/// The loyalty bar's drawn width in pixels, `FEScreen_SetStatBar`'s own
/// `(scale * min(value, max)) / max` in integer arithmetic with `scale` 150
/// and `max` 100000 (`TeamSelection_Update`'s call for `Loyalty`): whole
/// pixels, rounded down. The widget authors 148, so a full bar is two
/// pixels wider than its rect, as the original sets both width and U extent
/// to this one number.
fn loyalty_bar_pixels(total: u32) -> f32 {
    const SCALE: u32 = 150;
    const MAX: u32 = 100_000;
    (SCALE * total.min(MAX) / MAX) as f32
}

/// How much of a widget's own alpha shows, `seconds` after its screen
/// opened, given the `transition` its enclosing `LeftLayer` authors.
///
/// A widget with no `transition` (the screen's title bar, `<LeftLayer
/// transition="0">` on both selection screens) draws at once, exactly as
/// before this existed. One with a `transition` ramps linearly from
/// invisible to its own alpha over that many seconds - **measured, not
/// chosen**: `Widget_UpdateTransitionFraction` (`0x0888d8e4`, confidence 80,
/// `docs/ghidra/functions/psp-pulse-usa/race-box-screens.md`), the generic
/// per-widget fade every widget kind's own `Update` calls, is `elapsed /
/// duration` on both its enable and disable branches with no curve anywhere,
/// confirmed live across several thousand breakpoint hits. A live PPSSPP
/// capture (`docs/ui/selection-screens.md`) independently pins the
/// *duration* against the authored `0.5`, settled by frame 15 of a 60 Hz
/// walk.
fn fade_in(transition: f32, seconds: f32) -> f32 {
    if transition <= 0.0 {
        1.0
    } else {
        (seconds / transition).clamp(0.0, 1.0)
    }
}

/// `draw` with every colour it carries multiplied by `alpha` - [`fade_in`]'s
/// own output applied at the one point every [`Draw`] variant this module
/// emits keeps its colour: `color`, or the gradient pair on a
/// [`Draw::GradientFill`].
fn fade(draw: Draw, alpha: f32) -> Draw {
    if alpha >= 1.0 {
        return draw;
    }
    let scale = |c: [f32; 4]| [c[0], c[1], c[2], c[3] * alpha];
    match draw {
        Draw::Fill { rect, color } => Draw::Fill {
            rect,
            color: scale(color),
        },
        Draw::GradientFill { rect, left, right } => Draw::GradientFill {
            rect,
            left: scale(left),
            right: scale(right),
        },
        Draw::Text {
            x,
            y,
            scale: text_scale,
            color,
            border,
            align,
            text,
            wrap_width,
        } => Draw::Text {
            x,
            y,
            scale: text_scale,
            color: scale(color),
            border,
            align,
            text,
            wrap_width,
        },
        // A `Default`-role text fades with the page like the plain one.
        faced @ Draw::FacedText { .. } => {
            let mut faded = faced;
            faded.fade(alpha);
            faded
        }
        other => other,
    }
}

fn text_draw(text: &Text, content: &str, layout: &Layout) -> Draw {
    let align = match text.align.to_ascii_lowercase().as_str() {
        "right" => Align::Right,
        "centre" | "center" => Align::Centre,
        _ => Align::Left,
    };
    let (x, y, scale) = (text.x, text.y, text.scale * layout.face_scale(&text.font));
    let color = argb_to_rgba(text.color);
    let text_in = content.to_string();
    // `font="default"` in its own face when the renderer loaded one: lowercase
    // is real glyph art there, where the menu face has only capitals.
    if layout.faces.native_default && text.font.eq_ignore_ascii_case("default") {
        return Draw::FacedText {
            role: oag_ui::language::roles::DEFAULT,
            x,
            y,
            scale,
            color,
            border: None,
            align,
            text: text_in,
            wrap_width: text.wrap_width,
        };
    }
    Draw::Text {
        x,
        y,
        scale,
        color,
        border: None,
        align,
        text: text_in,
        wrap_width: text.wrap_width,
    }
}
