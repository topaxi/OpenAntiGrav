//! The marquee that names the song playing, along the foot of the race screen.
//!
//! Pulse authors it in the front end's `InGame_Definition.xml` (an `item` holding a
//! translucent bar and a text that starts just past the bar's right edge) and its executable
//! animates it: `RaceManager_ShowNowPlaying` fills the text and `RaceManager_UpdateNowPlaying`
//! fades it in while it scrolls left, lets it run off the bar, then fades it out
//! (`docs/ghidra/functions/psp-pulse-usa/now-playing.md`). The geometry is read from the
//! disc ([`geometry`]); the per-update law is [`Display::step`]; the title says where both
//! live ([`oag_title::now_playing::NowPlaying`]).
//!
//! It is not part of a mode's `*_HUD.xml`, so [`super::draw_list`] does not draw it. The host
//! appends it to the frame after any placement ([`append`]): the text runs off the left of the
//! bar, which a placement that drops a draw whose origin is off the screen would cut early.

use oag_tables::fexml::{Node, parse};
use oag_title::now_playing::NowPlaying;
use oag_ui::frontend::{Align, Draw};
use oag_ui::screen::{argb_to_rgba, parse_argb};

use crate::{Frame, Span};

/// Where the display sits, read off the disc's layout, in the HUD's grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    /// The bar: `[x, y, width, height]`.
    pub bar: [f32; 4],
    /// The bar's authored colour as `0xAARRGGBB`; its alpha is the ceiling the fade reaches.
    pub bar_argb: u32,
    /// The text's left edge when it appears, in the grid: the item's offset plus the text's own
    /// `x`, which is the viewport's width, so just past the bar's right edge.
    pub origin_x: f32,
    /// How far the text's left edge is from the item's: what the scroll subtracts from.
    pub item_x: f32,
    /// The text's top edge.
    pub text_y: f32,
    /// The text's authored size.
    pub scale: f32,
    /// The text's authored colour, `0xAARRGGBB`.
    pub text_argb: u32,
    /// The viewport the text is clipped to: `[x, y, width, height]`.
    pub clip: [f32; 4],
}

fn number(node: &Node, name: &str) -> Option<f32> {
    node.value(name)?.trim().parse().ok()
}

fn find<'a>(
    node: &'a Node,
    name: &str,
    parent: Option<&'a Node>,
) -> Option<(&'a Node, Option<&'a Node>)> {
    if node.attr("name") == Some(name) && node.name.eq_ignore_ascii_case("item") {
        return Some((node, parent));
    }
    node.children
        .iter()
        .find_map(|child| find(child, name, Some(node)))
}

/// The display's geometry in the layout `xml` (the expanded text of `now_playing.layout`), or
/// `None` when the layout has no such item - the caller reports it and draws nothing.
#[must_use]
pub fn geometry(xml: &str, authored: &NowPlaying) -> Option<Geometry> {
    let root = parse(xml);
    let (item, viewport) = find(&root, authored.item, None)?;
    let item_x = number(item, "OffsetX").unwrap_or(0.0);
    let item_y = number(item, "OffsetY").unwrap_or(0.0);
    let widget = |name: &str| {
        item.children
            .iter()
            .find(|child| child.attr("name") == Some(name))
    };
    let bar = widget(authored.bar)?;
    let text = widget(authored.text)?;
    let clip = viewport.map_or([0.0, 0.0, 480.0, 272.0], |viewport| {
        let values = |name| number(viewport, name).unwrap_or(0.0);
        [values("x"), values("y"), values("width"), values("height")]
    });
    Some(Geometry {
        bar: [
            item_x + number(bar, "x").unwrap_or(0.0),
            item_y + number(bar, "y").unwrap_or(0.0),
            number(bar, "width")?,
            number(bar, "height")?,
        ],
        bar_argb: bar
            .value("color")
            .and_then(parse_argb)
            .unwrap_or(0xFF00_0000),
        origin_x: item_x + number(text, "x").unwrap_or(0.0),
        item_x,
        text_y: item_y + number(text, "y").unwrap_or(0.0),
        scale: number(text, "scale").unwrap_or(1.0),
        text_argb: text
            .value("color")
            .and_then(parse_argb)
            .unwrap_or(0xFFFF_FFFF),
        clip,
    })
}

/// Where the animation is, in the executable's own states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// Fading in while scrolling.
    Entering,
    /// Fully shown and scrolling.
    Scrolling,
    /// Fading out.
    Leaving,
    /// Gone.
    Off,
}

/// The display's state: set by [`Self::show`] when a song starts, advanced once per update of
/// the race by [`Self::step`].
#[derive(Debug, Clone)]
pub struct Display {
    geometry: Geometry,
    fade_step: i32,
    scroll_step: f32,
    phase: Phase,
    text: String,
    width: f32,
    text_alpha: i32,
    bar_alpha: i32,
    x: f32,
}

impl Display {
    /// A display that shows nothing until the first [`Self::show`].
    #[must_use]
    pub fn new(geometry: Geometry, authored: &NowPlaying) -> Self {
        let x = geometry.origin_x - geometry.item_x;
        Self {
            geometry,
            fade_step: i32::from(authored.fade_step),
            scroll_step: f32::from(authored.scroll_step),
            phase: Phase::Off,
            text: String::new(),
            width: 0.0,
            text_alpha: 0,
            bar_alpha: 0,
            x,
        }
    }

    /// A song has started: shows `text`, `width` units wide, from the start.
    pub fn show(&mut self, text: String, width: f32) {
        self.text = text;
        self.width = width;
        self.text_alpha = 0;
        self.bar_alpha = 0;
        self.x = self.geometry.origin_x - self.geometry.item_x;
        self.phase = Phase::Entering;
    }

    /// The text's authored size, which scales a measured width.
    #[must_use]
    pub fn scale(&self) -> f32 {
        self.geometry.scale
    }

    /// Whether anything is up.
    #[must_use]
    pub fn is_showing(&self) -> bool {
        self.phase != Phase::Off
    }

    /// One update, `RaceManager_UpdateNowPlaying`'s law.
    ///
    /// The bar's alpha moves by `int(authored_alpha / 256 * step)` where the text's moves by
    /// `step`, so it climbs more slowly and is cut off where the text stops fading in; the
    /// executable leaves it at whatever it reached for the scroll.
    pub fn step(&mut self) {
        let authored = i32::try_from(self.geometry.bar_argb >> 24).unwrap_or(255);
        // `(int)((float)authored * 0.00390625 * (float)step)`, as the executable computes it.
        let bar_step = (authored as f32 * 0.003_906_25 * self.fade_step as f32) as i32;
        match self.phase {
            Phase::Entering => {
                self.text_alpha += self.fade_step;
                self.bar_alpha += bar_step;
                if self.text_alpha > 255 {
                    self.text_alpha = 255;
                    self.phase = Phase::Scrolling;
                }
                self.bar_alpha = self.bar_alpha.min(authored);
                self.x -= self.scroll_step;
            }
            Phase::Scrolling => {
                self.x -= self.scroll_step;
                if self.x <= -self.width {
                    self.phase = Phase::Leaving;
                }
            }
            Phase::Leaving => {
                self.text_alpha -= self.fade_step;
                self.bar_alpha -= bar_step;
                if self.text_alpha < 0 {
                    self.text_alpha = 0;
                    self.phase = Phase::Off;
                }
                self.bar_alpha = self.bar_alpha.max(0);
            }
            Phase::Off => {}
        }
    }

    /// What to draw now, or `None` while nothing is up.
    #[must_use]
    pub fn marquee(&self) -> Option<Marquee> {
        if self.phase == Phase::Off {
            return None;
        }
        let geometry = &self.geometry;
        let mut bar = argb_to_rgba(geometry.bar_argb);
        bar[3] = self.bar_alpha as f32 / 255.0;
        let mut color = argb_to_rgba(geometry.text_argb);
        color[3] *= self.text_alpha as f32 / 255.0;
        Some(Marquee {
            bar: geometry.bar,
            bar_color: bar,
            text: self.text.clone(),
            x: geometry.item_x + self.x,
            y: geometry.text_y,
            scale: geometry.scale,
            color,
            clip: geometry.clip,
        })
    }
}

/// The display as one frame draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct Marquee {
    /// The bar: `[x, y, width, height]`.
    pub bar: [f32; 4],
    /// The bar's colour with the fade applied.
    pub bar_color: [f32; 4],
    /// The song: `Artist - name`.
    pub text: String,
    /// The text's left edge.
    pub x: f32,
    /// The text's top edge.
    pub y: f32,
    /// The text's size.
    pub scale: f32,
    /// The text's colour with the fade applied.
    pub color: [f32; 4],
    /// What the text is clipped to: `[x, y, width, height]`.
    pub clip: [f32; 4],
}

impl Marquee {
    /// Moved and scaled by a placement, for a HUD drawn smaller about the screen's centre. The
    /// text is not dropped for starting off the placement's clip, which is the point of
    /// placing it here and not through [`crate::place`].
    #[must_use]
    pub fn placed(&self, at: &crate::Placement) -> Self {
        let moved = |rect: [f32; 4]| {
            [
                rect[0] * at.scale + at.offset[0],
                rect[1] * at.scale + at.offset[1],
                rect[2] * at.scale,
                rect[3] * at.scale,
            ]
        };
        Self {
            bar: moved(self.bar),
            x: self.x * at.scale + at.offset[0],
            y: self.y * at.scale + at.offset[1],
            scale: self.scale * at.scale,
            clip: moved(self.clip),
            ..self.clone()
        }
    }
}

/// Adds the display to `frame`: the bar under the text, the text in the `HUDSmall` face
/// clipped to the viewport the disc authors. `border` outlines the glyphs the way the rest of
/// the HUD's text is.
pub fn append(frame: &mut Frame, marquee: &Marquee, border: [f32; 4]) {
    if frame.spans.is_empty() {
        frame.spans.push(Span {
            clip: [-1.0e9, -1.0e9, 2.0e9, 2.0e9],
            start: [0; 5],
        });
    }
    frame.sprites.push(Draw::Fill {
        rect: marquee.bar,
        color: marquee.bar_color,
    });
    frame.spans.push(Span {
        clip: marquee.clip,
        start: [
            frame.sprites.len(),
            frame.hud_text.len(),
            frame.small_text.len(),
            frame.default_text.len(),
            frame.skin_text.len(),
        ],
    });
    frame.small_text.push(Draw::Text {
        x: marquee.x,
        y: marquee.y,
        scale: marquee.scale,
        color: marquee.color,
        border: Some(border),
        align: Align::Left,
        text: marquee.text.clone(),
        wrap_width: None,
    });
}

#[cfg(test)]
mod tests;
