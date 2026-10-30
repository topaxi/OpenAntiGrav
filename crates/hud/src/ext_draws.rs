//! What a simulation mod does to the HUD: the outputs of `oag-ext-abi`'s `hud-*`
//! commands, carried per slot in a [`Readout`](super::Readout) and applied while the
//! draw list is built. Title-agnostic: [`Element`] names what a mod addresses, and
//! [`element_of`] maps a widget of the title's own layout to it. An element a layout
//! has no widget for draws nothing, and that is not an error.
//!
//! Everything here is render-side. The simulation never sees it and nothing in it is
//! hashed. The shake law is **chosen, not measured**.

use oag_ui::frontend::{Align, Draw};

use super::draw::{Context, Frame, crop_horizontally, lit_colour, sprite_draw, text_shadow};
use super::{Sprite, VertAlign};

/// A HUD element a mod addresses, in `oag_ext_abi::HudElement`'s order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Element {
    /// The speed bar and its readout.
    Speed,
    /// The thrust bar.
    Thrust,
    /// The energy bar: Pulse's `ShieldBar`, HD's `EnergyBar`.
    Energy,
    /// The held pickup's backdrop and icon.
    Pickup,
    /// The race position.
    Position,
    /// The lap counter.
    Lap,
    /// The race and lap times.
    Time,
}

impl Element {
    /// How many there are.
    pub const COUNT: usize = 7;

    fn index(self) -> usize {
        self as usize
    }
}

/// Where an item's offset is measured from, in the layout's grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Top left corner.
    TopLeft,
    /// Middle of the top edge.
    Top,
    /// Top right corner.
    TopRight,
    /// Middle of the left edge.
    Left,
    /// Centre.
    Center,
    /// Middle of the right edge.
    Right,
    /// Bottom left corner.
    BottomLeft,
    /// Middle of the bottom edge.
    Bottom,
    /// Bottom right corner.
    BottomRight,
}

/// What an item draws.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum What {
    /// A string in the title's HUD font.
    Text(String),
    /// The title's HUD sprite of this name; nothing if the layout has none.
    Icon(String),
    /// A mod's own PNG, by its key in the HUD sheet; nothing if the sheet lacks it.
    ModIcon(String),
}

/// One `hud-text` or `hud-icon`.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// The anchor.
    pub anchor: Anchor,
    /// Offset right of the anchor, grid units.
    pub x: f32,
    /// Offset down from the anchor, grid units.
    pub y: f32,
    /// Size as a multiple of the title's own text height or the sprite's own size.
    pub scale: f32,
    /// The colour, `[r, g, b, a]` in `0..=1`.
    pub color: [f32; 4],
    /// What it draws.
    pub what: What,
}

/// A bar's added share and the colour it blends into.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarExtra {
    /// The share added on top of the value: 0.9 for x1.9.
    pub extra: f32,
    /// The colour the added share blends to.
    pub color: [f32; 4],
}

/// One slot's HUD outputs for this tick. `Default` changes nothing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Outputs {
    /// Per element, a recolour.
    pub tint: [Option<[f32; 4]>; Element::COUNT],
    /// Per element, the jitter amplitude in grid units; 0 holds still.
    pub shake: [f32; Element::COUNT],
    /// Per bar element, the added share.
    pub bar_extra: [Option<BarExtra>; Element::COUNT],
    /// The text and icons.
    pub items: Vec<Item>,
    /// The look of the slot's held mod pickup: this tick's `pickup-look`, else the
    /// manifest's `[pickup]`, kept while the pickup fades out after its use.
    pub pickup_look: Option<oag_tables::pickup_look::PickupLook>,
    /// The world tick, which seeds the jitter.
    pub tick: u64,
    /// The slot, which seeds the jitter.
    pub slot: u8,
}

impl Outputs {
    /// Whether nothing changes.
    #[must_use]
    pub fn is_quiet(&self) -> bool {
        self.tint.iter().all(Option::is_none)
            && self.shake.iter().all(|a| *a == 0.0)
            && self.bar_extra.iter().all(Option::is_none)
            && self.items.is_empty()
            && self.pickup_look.is_none()
    }

    /// The recolour of `element`.
    #[must_use]
    pub fn tint(&self, element: Element) -> Option<[f32; 4]> {
        self.tint[element.index()]
    }

    /// The added share of bar `element`.
    #[must_use]
    pub fn extra(&self, element: Element) -> Option<BarExtra> {
        self.bar_extra[element.index()]
    }

    /// How far `element` is moved this tick, grid units `[dx, dy]`; `[0, 0]` when it
    /// holds still.
    ///
    /// **Chosen, not measured.** An integer hash of the tick, slot and element gives
    /// each axis a magnitude in `[0.5, 1]` of the amplitude and a sign, so a shaking
    /// element moves on every tick and never lands on zero. Pure arithmetic on
    /// integers: no clock, no `Rng`.
    #[must_use]
    pub fn shake_offset(&self, element: Element) -> [f32; 2] {
        let amplitude = self.shake[element.index()];
        if amplitude <= 0.0 {
            return [0.0, 0.0];
        }
        let mut h = self.tick ^ (u64::from(self.slot) << 56) ^ ((element.index() as u64) << 48);
        h = h.wrapping_add(0x9e37_79b9_7f4a_7c15);
        h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        h ^= h >> 31;
        let axis = |bits: u64| {
            let magnitude = 0.5 + 0.5 * ((bits & 0xff) as f32 / 255.0);
            let sign = if bits & 0x100 == 0 { 1.0 } else { -1.0 };
            amplitude * magnitude * sign
        };
        [axis(h), axis(h >> 16)]
    }
}

/// The element a widget of a title's layout is, if any.
///
/// By name: the layouts author the bars as `SpeedBar`, `ThrustBar`, `ShieldBar` (Pulse)
/// or `EnergyBar` (2048) with a `Bg`, `Mark` or `Text` companion, the retro skins as
/// `SpeedBarWo3`, `ThrustBarWo3` and `nrgBarWo3`.
#[must_use]
pub fn element_of(name: &str) -> Option<Element> {
    if name.starts_with("SpeedBar") {
        Some(Element::Speed)
    } else if name.starts_with("ThrustBar") {
        Some(Element::Thrust)
    } else if name.starts_with("ShieldBar")
        || name.starts_with("EnergyBar")
        || name.starts_with("nrgBar")
    {
        Some(Element::Energy)
    } else if name == "Position" || name == "Position Outof" {
        Some(Element::Position)
    } else if name == "Lap" {
        Some(Element::Lap)
    } else if matches!(name, "CurrentTime" | "BestTime" | "TotalTime") {
        Some(Element::Time)
    } else {
        None
    }
}

/// The added share a bar named `name` shows, if the mod asked for one for its element.
/// Each element means the same on every title: a title with no thrust bar draws nothing
/// for [`Element::Thrust`].
pub(super) fn bar_extra(outputs: &Outputs, name: &str) -> Option<BarExtra> {
    outputs.extra(element_of(name)?)
}

/// `draw` recoloured and shaken by what the mod asked of `element`.
pub(super) fn style(outputs: &Outputs, element: Element, recolour: bool, draw: Draw) -> Draw {
    let tint = outputs.tint(element).filter(|_| recolour);
    let [dx, dy] = outputs.shake_offset(element);
    let moved = |rect: [f32; 4]| [rect[0] + dx, rect[1] + dy, rect[2], rect[3]];
    match draw {
        Draw::Sprite { rect, uv, color } => Draw::Sprite {
            rect: moved(rect),
            uv,
            color: tint.unwrap_or(color),
        },
        Draw::GradientSprite {
            rect,
            uv,
            left,
            right,
        } => Draw::GradientSprite {
            rect: moved(rect),
            uv,
            left: tint.unwrap_or(left),
            right: tint.unwrap_or(right),
        },
        Draw::RotatedSprite {
            rect,
            uv,
            color,
            rotation,
        } => Draw::RotatedSprite {
            rect: moved(rect),
            uv,
            color: tint.unwrap_or(color),
            rotation,
        },
        Draw::Text {
            x,
            y,
            scale,
            color,
            border,
            align,
            text,
            wrap_width,
        } => Draw::Text {
            x: x + dx,
            y: y + dy,
            scale,
            color: tint.unwrap_or(color),
            border,
            align,
            text,
            wrap_width,
        },
        other => other,
    }
}

/// A bar of `fraction` of its width, rescaled so that value times (1 + extra) fills it:
/// the base takes `fraction / (1 + extra)` in the bar's own colour and the added share
/// the rest, blending from the colour the bar has there into `extra.color`. The two
/// together reach `fraction`.
pub(super) fn split_bar(
    sprite: &Sprite,
    fraction: f32,
    extra: BarExtra,
) -> (Sprite, Option<Sprite>) {
    let base_fraction = fraction / (1.0 + extra.extra);
    let base = crop_horizontally(sprite, base_fraction);
    let span = fraction - base_fraction;
    if span <= 0.0 {
        return (base, None);
    }
    let at = |t: f32| match sprite.gradient {
        Some([left, right]) => std::array::from_fn(|i| left[i] + (right[i] - left[i]) * t),
        None => sprite.color,
    };
    let mut added = sprite.clone();
    added.rect[0] += sprite.rect[2] * base_fraction;
    added.rect[2] = sprite.rect[2] * span;
    added.uv[0] += sprite.uv[2] * base_fraction;
    added.uv[2] = sprite.uv[2] * span;
    added.gradient = Some([at(base_fraction), extra.color]);
    (base, Some(added))
}

/// Appends the slot's `hud-text` and `hud-icon` items to `frame`, anchored in the
/// `grid` the layout is authored in. The title's safe area then scales the whole frame,
/// so an item at an edge anchor stays inside it, and a split screen places each view's
/// own frame, so an item shows only in the view of the slot that named it.
pub fn append(frame: &mut Frame, cx: &Context<'_>, outputs: &Outputs, grid: (f32, f32)) {
    for item in &outputs.items {
        let (col, row): (u8, u8) = match item.anchor {
            Anchor::TopLeft => (0, 0),
            Anchor::Top => (1, 0),
            Anchor::TopRight => (2, 0),
            Anchor::Left => (0, 1),
            Anchor::Center => (1, 1),
            Anchor::Right => (2, 1),
            Anchor::BottomLeft => (0, 2),
            Anchor::Bottom => (1, 2),
            Anchor::BottomRight => (2, 2),
        };
        let ax = item.x + grid.0 * 0.5 * f32::from(col);
        let ay = item.y + grid.1 * 0.5 * f32::from(row);
        match &item.what {
            What::Text(text) => {
                let height = cx.hud_line_height * item.scale;
                let top = match [VertAlign::Top, VertAlign::Middle, VertAlign::Bottom]
                    [usize::from(row)]
                {
                    VertAlign::Top => ay,
                    VertAlign::Middle => ay - height / 2.0,
                    VertAlign::Bottom => ay - height,
                };
                let draw = Draw::Text {
                    x: ax,
                    y: top,
                    scale: item.scale,
                    color: lit_colour(cx, item.color),
                    border: Some(cx.default_border),
                    align: [Align::Left, Align::Centre, Align::Right][usize::from(col)],
                    text: text.clone(),
                    wrap_width: None,
                };
                frame.hud_text.extend(text_shadow(cx, &draw));
                frame.hud_text.push(draw);
            }
            What::ModIcon(src) => {
                let Some(placed) = cx.sheet.get(src) else {
                    continue;
                };
                let (w, h) = (
                    placed.width as f32 * item.scale,
                    placed.height as f32 * item.scale,
                );
                frame.sprites.push(Draw::Sprite {
                    rect: [
                        ax - w * 0.5 * f32::from(col),
                        ay - h * 0.5 * f32::from(row),
                        w,
                        h,
                    ],
                    uv: [
                        placed.x as f32,
                        placed.y as f32,
                        placed.width as f32,
                        placed.height as f32,
                    ],
                    color: item.color,
                });
            }
            What::Icon(name) => {
                let Some(sprite) = cx.layout.sprite(name) else {
                    continue;
                };
                let (w, h) = (sprite.rect[2] * item.scale, sprite.rect[3] * item.scale);
                let mut placed = sprite.clone();
                placed.rect = [
                    ax - w * 0.5 * f32::from(col),
                    ay - h * 0.5 * f32::from(row),
                    w,
                    h,
                ];
                placed.color = item.color;
                placed.gradient = None;
                frame.sprites.extend(sprite_draw(&placed, cx.sheet));
            }
        }
    }
}
