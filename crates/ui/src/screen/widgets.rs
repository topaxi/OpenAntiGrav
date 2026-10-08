//! The small per-widget node-to-struct builders `Screens::collect_widgets`
//! calls - split out under the 1,000-line rule
//! (`scripts/check-file-size.py`), the same shape `touch.rs`/`settings.rs`
//! already split their own `touch_button_from_node`/`touch_list_from_node`/
//! `touch_slider_from_node` into. A move, not a behaviour change.

use super::{Fill, Image, Menu, Node, Redirect, Screens, Text, argb_to_rgba, parse_argb};
use crate::frontend::Draw;
use oag_core::buttons::button_from_name;

impl Fill {
    /// This fill as something to draw: a [`Draw::GradientFill`] when it
    /// carries `Color1`..`Color4`, a plain [`Draw::Fill`] otherwise.
    ///
    /// A gradient is the mean of each edge's two corners, which is every
    /// shape a shipped widget authors: the top and bottom of an edge never
    /// differ (see [`Draw::GradientFill`]). The one conversion the screens
    /// that draw a fill share.
    #[must_use]
    pub fn draw(&self) -> Draw {
        let rect = [
            self.x,
            self.y,
            self.width.unwrap_or(0.0),
            self.height.unwrap_or(0.0),
        ];
        match self.gradient {
            Some([c1, c2, c3, c4]) => Draw::GradientFill {
                rect,
                left: mean(argb_to_rgba(c1), argb_to_rgba(c2)),
                right: mean(argb_to_rgba(c3), argb_to_rgba(c4)),
            },
            None => Draw::Fill {
                rect,
                color: argb_to_rgba(self.color),
            },
        }
    }
}

fn mean(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
        (a[3] + b[3]) * 0.5,
    ]
}

impl Screens {
    /// A colour-only `Image`: a plain `color` fill, or a `Color1`..`Color4`
    /// gradient. A widget with neither is not a fill and is dropped.
    ///
    /// `pub(crate)` for `oag_ui_screens::tag_entry`, the same reason
    /// [`Self::image_from_node`] already is: it walks a screen's own
    /// `TagInput` siblings (the cell backgrounds and confirm bars) directly,
    /// off a screen `collect_widgets` never registers - see that module's
    /// own doc.
    pub fn fill_from_node(&self, node: &Node, offset: (f32, f32)) -> Option<Fill> {
        let argb = |attr: &str| {
            self.resolve(node.value(attr).unwrap_or_default())
                .and_then(parse_argb)
        };
        let gradient = match (
            argb("Color1"),
            argb("Color2"),
            argb("Color3"),
            argb("Color4"),
        ) {
            (Some(c1), Some(c2), Some(c3), Some(c4)) => Some([c1, c2, c3, c4]),
            _ => None,
        };
        let color = argb("color").or_else(|| gradient.map(|corners| corners[0]))?;
        Some(Fill {
            name: node.attr("name").map(str::to_string),
            x: self.number(node.value("x")).unwrap_or(0.0) + offset.0,
            y: self.number(node.value("y")).unwrap_or(0.0) + offset.1,
            width: self.number(node.value("width")),
            height: self.number(node.value("height")),
            color,
            gradient,
            reveal: Vec::new(),
            transition: 0.0,
        })
    }

    /// An `Image` widget off its node, `offset` added to its position.
    /// `pub(crate)` for `oag_ui_screens::picker::slideshow`, which walks a screen
    /// tree of its own and wants its images read the same way.
    pub fn image_from_node(&self, node: &Node, src: &str, offset: (f32, f32)) -> Image {
        Image {
            name: node.attr("name").map(str::to_string),
            src: src.to_string(),
            x: self.number(node.value("x")).unwrap_or(0.0) + offset.0,
            y: self.number(node.value("y")).unwrap_or(0.0) + offset.1,
            width: self.number(node.value("width")),
            height: self.number(node.value("height")),
            centred: node.flag("Centred").unwrap_or(false),
            color: self
                .resolve(node.value("color").unwrap_or_default())
                .and_then(parse_argb)
                .unwrap_or(0xffff_ffff),
            u: self.number(node.value("U")),
            v: self.number(node.value("V")),
            texture_width: self.number(node.value("TxtrWidth")),
            texture_height: self.number(node.value("TxtrHeight")),
            auto_load: node.flag("AutoLoad").unwrap_or(false),
            reveal: Vec::new(),
            transition: 0.0,
            start_enabled: node.flag("StartEnabled").unwrap_or(true),
        }
    }

    pub(super) fn menu_from_node(&self, node: &Node, offset: (f32, f32)) -> Menu {
        Menu {
            name: node.attr("name").unwrap_or("Menu").to_string(),
            x: self.number(node.value("x")).unwrap_or(0.0) + offset.0,
            y: self.number(node.value("y")).unwrap_or(0.0) + offset.1,
            scale: self.number(node.value("scale")).unwrap_or(1.0),
            color: self
                .resolve(node.value("color").unwrap_or_default())
                .and_then(parse_argb)
                .unwrap_or(0xffff_ffff),
            align: node.value("align").unwrap_or("left").to_string(),
            font: node.value("font").unwrap_or("Default").to_string(),
        }
    }

    /// A `<Block>` widget, read as a [`Text`] - see `collect_widgets`'s own
    /// `"block"` arm for why. The one attribute a `Block` spells differently
    /// from a `Text` is its own label colour: `TextColor`, not `color`
    /// (which a `Block` uses for its fill instead) - so this calls
    /// [`Self::text_from_node`] and then overrides the colour where
    /// `TextColor` is present, rather than duplicating every other field.
    pub(super) fn block_from_node(
        &self,
        node: &Node,
        viewport_width: Option<f32>,
        offset: (f32, f32),
    ) -> Text {
        let mut text = self.text_from_node(node, viewport_width, offset);
        if let Some(argb) = self
            .resolve(node.value("TextColor").unwrap_or_default())
            .and_then(parse_argb)
        {
            text.color = argb;
        }
        text
    }

    /// `pub(crate)` for `oag_ui_screens::tag_entry` - see [`Self::fill_from_node`]'s
    /// own note.
    pub fn text_from_node(
        &self,
        node: &Node,
        viewport_width: Option<f32>,
        offset: (f32, f32),
    ) -> Text {
        Text {
            name: node.attr("name").map(str::to_string),
            idstring: node.value("idstring").map(str::to_string),
            string: node.value("String").map(str::to_string),
            font: node.value("font").unwrap_or("Default").to_string(),
            x: self.number(node.value("x")).unwrap_or(0.0) + offset.0,
            y: self.number(node.value("y")).unwrap_or(0.0) + offset.1,
            scale: self.number(node.value("scale")).unwrap_or(1.0),
            color: self
                .resolve(node.value("color").unwrap_or_default())
                .and_then(parse_argb)
                .unwrap_or(0xffff_ffff),
            align: node.value("align").unwrap_or("left").to_string(),
            middle: node
                .value("vertalign")
                .is_some_and(|v| v.eq_ignore_ascii_case("middle")),
            start_enabled: node.flag("StartEnabled").unwrap_or(true),
            pulse: node.flag("pulse").unwrap_or(false),
            delay: self.number(node.value("delay")).unwrap_or(0.0),
            wrap_width: node
                .flag("widthlimited")
                .unwrap_or(false)
                .then_some(viewport_width)
                .flatten(),
            transition: 0.0,
        }
    }
}

pub(super) fn redirect_from_node(node: &Node) -> Redirect {
    Redirect {
        name: node.attr("name").map(str::to_string),
        forward: node.value("forward").and_then(button_from_name),
        backward: node.value("backward").and_then(button_from_name),
        goto: node
            .children_named("Default")
            .find_map(|d| d.attr("goto"))
            .map(str::to_string),
        delay: node.value("delay").and_then(|d| d.trim().parse().ok()),
    }
}
