//! The footer legend's flow layout (`oag_title::legend::LegendFlow`).
//!
//! HD's and Omega's `NavigationController` ignores the x each prompt authors and lays them out
//! left to right from the `NAVIGATION` word: a prompt's glyph at the cursor, its word beside it,
//! the next cursor a gap past the word's measured ink
//! (`docs/ghidra/functions/ps3-hdfury-eu/navigation-legend.md`, "The law"). Widths are measured
//! once, when the legend is given its flow; the positions are worked out at draw time, because
//! which prompts a page shows decides where the others sit.

use oag_title::legend::{LegendFlow, WordAfter};

use super::{NavigationLegend, Prompt, PromptKind};

/// A text's measured width: `(font, string, scale, snap)`, in the screen's authored units. `font`
/// is the widget's own `font` value; `snap` is [`LegendFlow::snap_pen`].
pub type Ink<'a> = &'a dyn Fn(&str, &str, f32, bool) -> f32;

/// A legend's flow and the cursor it starts from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Flow {
    law: LegendFlow,
    /// Where the first prompt's glyph goes: `(int)(nav.x + gap + w(nav))`, `NAVIGATION` being
    /// the one word that keeps its authored x.
    anchor: f32,
}

impl Flow {
    /// Where `glyph`'s word goes, from the glyph's x.
    fn word_x(self, glyph_x: f32, glyph_ink: f32) -> f32 {
        match self.law.word_after {
            WordAfter::Fixed(distance) => glyph_x + distance,
            WordAfter::GlyphWidthPlus(extra) => glyph_x + glyph_ink.trunc() + extra,
        }
    }
}

/// The prompts a flow places, in the order `NavigationController_Layout` places them. The
/// registered extras and Invite follow, which a screen supplies itself
/// ([`NavigationLegend::extra_prompt_x`]).
const FLOWED: [PromptKind; 2] = [PromptKind::Confirm, PromptKind::Back];

impl NavigationLegend {
    /// This legend laid out by `law`, its words measured through `ink`. A legend with no
    /// `NAVIGATION` word has nothing to start from and keeps its authored x.
    #[must_use]
    pub fn with_flow(mut self, law: LegendFlow, ink: Ink) -> Self {
        for prompt in &mut self.prompts {
            prompt.ink = ink(&prompt.font, &prompt.text, prompt.scale, law.snap_pen);
        }
        self.flow = self
            .prompts
            .iter()
            .find(|prompt| prompt.kind == PromptKind::Navigation && !prompt.glyph)
            .map(|nav| Flow {
                law,
                anchor: (nav.x + law.gap + nav.ink).trunc(),
            });
        self
    }

    /// Whether [`Self::with_flow`] gave this legend a flow to place its prompts by.
    #[must_use]
    pub fn is_flowed(&self) -> bool {
        self.flow.is_some()
    }

    /// The prompts at the x the page lays them out at, and the cursor after the last one placed.
    /// `Back` is placed only when `show_back`: a hidden prompt takes no room.
    pub(super) fn laid_out(&self, show_back: bool) -> (Vec<Prompt>, Option<f32>) {
        let mut prompts = self.prompts.clone();
        let Some(flow) = self.flow else {
            return (prompts, None);
        };
        let mut cursor = flow.anchor;
        for kind in FLOWED {
            if kind == PromptKind::Back && !show_back {
                continue;
            }
            let at = |glyph: bool| {
                prompts
                    .iter()
                    .position(|prompt| prompt.kind == kind && prompt.glyph == glyph)
            };
            let (Some(button), Some(word)) = (at(true), at(false)) else {
                continue;
            };
            let word_x = flow.word_x(cursor, prompts[button].ink);
            prompts[button].x = cursor;
            prompts[word].x = word_x;
            cursor = prompts[word].ink.trunc() + word_x + flow.law.gap;
        }
        (prompts, Some(cursor))
    }

    /// Where a screen's own extra prompts go, in the order it registers them (Cell Selection's
    /// `Records` and `Difficulty`): `[glyph x, word x]` for each, flowing on from the prompts the
    /// page shows. `extras` is each one's measured `[glyph width, word width]`. `None` for a legend
    /// with no flow.
    #[must_use]
    pub fn extra_prompts_x(&self, show_back: bool, extras: &[[f32; 2]]) -> Option<Vec<[f32; 2]>> {
        let flow = self.flow?;
        let mut cursor = self.laid_out(show_back).1?;
        let mut placed = Vec::with_capacity(extras.len());
        for [glyph_ink, word_ink] in extras {
            let word_x = flow.word_x(cursor, *glyph_ink);
            placed.push([cursor, word_x]);
            cursor = word_ink.trunc() + word_x + flow.law.gap;
        }
        Some(placed)
    }
}
