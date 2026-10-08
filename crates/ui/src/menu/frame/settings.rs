//! The original's settings screen, read off the disc: the rules between rows,
//! where the label and value columns sit, and what the rows step by.
//!
//! **Read, not transcribed** (`CLAUDE.md`, "Never invent what the assets
//! already author"). Pulse's `Single Player` screen in `RaceBox_Definition.xml`
//! nests twelve `<Image>`s in an `<item OffsetX="50" OffsetY="33">`, the two
//! halves of one fading line per row boundary, each a one-texel quad of
//! `pulse_assets.mip` carrying `Color1`..`Color4`; its `<Text>`s are the labels
//! and its `<List>`s the value column. This reads exactly those, so the numbers
//! 50, 33, 23 and 250 are the screen's and nothing here knows them.
//!
//! What the disc does not author - the step arrows, drawn by the executable's
//! `List` code - is [`oag_title::MenuSettings`]'s, each field marked by where
//! it came from.

use crate::frontend::{Draw, Placed};
use crate::screen::{Screens, parse};

/// One settings screen: columns, pitch, rules and the arrows' placement.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// The title's own numbers this was read with.
    pub spec: oag_title::MenuSettings,
    /// Left edge of the labels. The screen's `FEGlobals->MenuXOffset`.
    pub label_x: f32,
    /// Left edge of the values - the first `<List>`'s `x`.
    pub value_x: f32,
    /// Top of the first row's text: the first `<List>`'s `y`.
    pub first_y: f32,
    /// From one row's text to the next: the step between the `<List>`s' `y`.
    pub pitch: f32,
    /// The screen's rules, positioned with the `<item>`'s own offset folded in.
    pub rules: Vec<Draw>,
    /// The step arrows' sheet placement, `None` when the sheet did not carry
    /// the texture, which draws the rows without arrows rather than with a
    /// stand-in.
    pub arrows: Option<Arrows>,
}

/// The two step arrows and their halos, as sheet rectangles `[x, y, w, h]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Arrows {
    /// Left then right, as drawn.
    pub plain: [[f32; 4]; 2],
    /// The lit pair a focused row draws under them.
    pub glow: [[f32; 4]; 2],
}

impl Layout {
    /// Whether `page` is one this layout draws.
    #[must_use]
    pub fn draws(&self, page: &str) -> bool {
        self.spec.pages.contains(&page)
    }

    /// Reads `spec.screen` out of `document` (already-expanded front-end XML),
    /// resolving `FEGlobals->` through `globals`. `None` when the screen is
    /// not there or authors no `<List>` to place a column by. The numbers stay in
    /// the screen's own grid: only the PSP's 480x272 is wired.
    #[must_use]
    pub fn read(
        document: &str,
        globals: &std::collections::HashMap<String, String>,
        spec: oag_title::MenuSettings,
        sprites: &[(String, Placed)],
    ) -> Option<Self> {
        let root = parse(document);
        let screen = find(&root, spec.screen)?;
        let scratch = Screens {
            globals: globals.clone(),
            ..Screens::default()
        };

        let mut rules = Vec::new();
        for layer in screen.children_named("LeftLayer") {
            for item in layer.children_named("item") {
                let offset = (
                    scratch.number(item.value("OffsetX")).unwrap_or(0.0),
                    scratch.number(item.value("OffsetY")).unwrap_or(0.0),
                );
                for image in item.children_named("Image") {
                    if let Some(fill) = scratch.fill_from_node(image, offset) {
                        rules.push(fill.draw());
                    }
                }
            }
        }

        let mut lists = Vec::new();
        let mut label_x = None;
        for layer in screen.children_named("LeftLayer") {
            for list in layer.children_named("List") {
                lists.push((
                    scratch.number(list.value("x"))?,
                    scratch.number(list.value("y"))?,
                ));
            }
            label_x = label_x.or_else(|| {
                layer
                    .children_named("Text")
                    .find_map(|text| scratch.number(text.value("x")))
            });
        }
        let &(value_x, first_y) = lists.first()?;
        let pitch = lists.get(1).map_or(0.0, |second| second.1 - first_y);
        if pitch <= 0.0 {
            return None;
        }

        let arrows = sprites
            .iter()
            .find(|(src, _)| src.eq_ignore_ascii_case(spec.sheet))
            .map(|(_, placed)| *placed)
            .map(|placed| {
                let at = |rect: [f32; 4]| {
                    [
                        placed.x as f32 + rect[0],
                        placed.y as f32 + rect[1],
                        rect[2],
                        rect[3],
                    ]
                };
                Arrows {
                    plain: spec.arrow.map(at),
                    glow: spec.arrow_glow.map(at),
                }
            });
        Some(Self {
            spec,
            label_x: label_x?,
            value_x,
            first_y,
            pitch,
            rules,
            arrows,
        })
    }

    /// How far above a row's text its band starts: the first rule's `y` against
    /// the first text's, so a pointer's row is the span between two rules.
    /// Zero when the screen authors no rules.
    #[must_use]
    pub fn band_offset(&self) -> f32 {
        self.rules
            .iter()
            .filter_map(|draw| match draw {
                Draw::GradientFill { rect, .. } | Draw::Fill { rect, .. } => Some(rect[1]),
                _ => None,
            })
            .reduce(f32::min)
            .map_or(0.0, |top| top - self.first_y)
    }

    /// The two arrows' rects on the row whose text sits at `y`, left then
    /// right: `[x, y, w, h]` in the grid being drawn in.
    #[must_use]
    pub fn arrow_rects(&self, y: f32) -> [[f32; 4]; 2] {
        let at = |side: usize| {
            let [_, _, w, h] = self.spec.arrow[side];
            let (dx, dy) = self.spec.arrow_offset[side];
            [self.value_x + dx, y + dy, w, h]
        };
        [at(0), at(1)]
    }

    /// One line for the boot report.
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "settings screen {}: labels x={}, values x={}, first row y={}, pitch {}, {} rule(s), arrows {}",
            self.spec.screen,
            self.label_x,
            self.value_x,
            self.first_y,
            self.pitch,
            self.rules.len(),
            if self.arrows.is_some() {
                "from the sheet"
            } else {
                "not in the sheet, none drawn"
            },
        )
    }
}

fn find<'a>(node: &'a crate::screen::Node, name: &str) -> Option<&'a crate::screen::Node> {
    if node.name.eq_ignore_ascii_case("Screen")
        && node
            .attr("name")
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
    {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, name))
}
