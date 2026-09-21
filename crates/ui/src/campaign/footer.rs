//! The front-end root's own footer fixtures, shared by every menu screen
//! but authored once: the `NavigationController`'s `Confirm`/`Back` prompts
//! and the scrolling tip ticker, both on `FE Screen`
//! (`Data\Plugins\PI001\GUI\Skin.xml`) rather than on `Grid Selection`/`Cell
//! Selection`'s own `CellMode_Definition.xml`. Neither shape is one
//! [`crate::screen::Screens::collect_widgets`] recognises at all - `
//! NavigationController` and `TextInfo` fall through its match to the
//! catch-all arm, children and all - which is the actual gap
//! `docs/ui/campaign-screens.md`'s "Open" section names - the main menu's
//! own copy of this same footer is a sibling gap, tracked in its own
//! handover thread rather than linked from here.
//!
//! Read directly off the raw parsed tree (`oag_tables::fexml::parse`'s
//! `Node`) instead of through that model, because the model has no way to
//! express either idiom even if the tag were recognised: a
//! `NavigationController` picks *per screen* which prompts to show, which
//! is not a fact any named `Screen` carries; and the ticker's own `TextInfo
//! type="bar"` is two alternating text buffers sharing one clip viewport,
//! not a single positioned widget with a string. This build shows the
//! `NavigationController`'s `Confirm`/`Back` prompts unconditionally
//! wherever they are asked for - `Cell Selection` alone today, since that
//! is the one screen `docs/ui/campaign-screens.md`'s 2026-09-14 PPSSPP pass
//! measured wanting them - rather than modelling the original's own
//! per-screen gate, which was not traced.
//!
//! # The ticker's own content is not authored at all
//!
//! `NewsItemBarText1`/`NewsItemBarText2` (the two alternating buffers) and
//! `NewsItemTagText` (a third, smaller label to their left) carry no
//! `idstring` and no literal `string` anywhere in `Skin.xml` - they are
//! filled at runtime, by a mechanism this build does not have. Per
//! `CLAUDE.md`'s "never invent what the assets already author": nothing
//! here invents a value for `NewsItemTagText` at all (it draws nothing, see
//! [`TickerLayout`]'s own doc), and [`ticker_draw`] only ever rotates
//! through disc strings the caller resolved honestly - see
//! `crate::main::campaign_stage::CampaignStage::ticker_tips` in `oag-game`
//! for the rule.

use std::collections::HashMap;

use oag_tables::fexml::Node;

use crate::frontend::{Align, Draw};
use crate::picker::FaceScales;
use crate::screen::{argb_to_rgba, parse_argb};

fn resolve<'a>(globals: &'a HashMap<String, String>, value: &'a str) -> Option<&'a str> {
    if value.is_empty() {
        return None;
    }
    match value.strip_prefix("FEGlobals->") {
        Some(key) => globals.get(key.trim()).map(String::as_str),
        None => Some(value),
    }
}

fn color_of(globals: &HashMap<String, String>, node: &Node) -> [f32; 4] {
    node.value("color")
        .and_then(|value| resolve(globals, value))
        .and_then(parse_argb)
        .map(argb_to_rgba)
        .unwrap_or([1.0, 1.0, 1.0, 1.0])
}

fn number(globals: &HashMap<String, String>, node: &Node, name: &str) -> Option<f32> {
    resolve(globals, node.value(name)?)?.trim().parse().ok()
}

fn face_scale(faces: &FaceScales, font: &str) -> f32 {
    match font.to_ascii_lowercase().as_str() {
        "default" => faces.default,
        "small" => faces.small,
        _ => 1.0,
    }
}

/// Depth-first search for the first descendant (or `node` itself) named
/// `tag`, case-insensitively - `Node` carries no parent pointer and no
/// indexed lookup, so this is the plain way to reach a tag
/// `collect_widgets` never visits.
fn find_first<'a>(node: &'a Node, tag: &str) -> Option<&'a Node> {
    if node.name.eq_ignore_ascii_case(tag) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_first(child, tag))
}

/// [`find_first`], also requiring the node's own `name` attribute - for
/// `Viewport name="TextInfoIsAlwaysLast"`, the one of several `Viewport`s on
/// `FE Screen` that clips the ticker's bar text.
fn find_named<'a>(node: &'a Node, tag: &str, name: &str) -> Option<&'a Node> {
    if node.name.eq_ignore_ascii_case(tag) && node.attr("name") == Some(name) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_named(child, tag, name))
}

/// One `NavigationController` prompt - an icon glyph and its label, both
/// resolved through the string table already.
#[derive(Debug, Clone)]
struct Prompt {
    text: String,
    font: String,
    x: f32,
    y: f32,
    color: [f32; 4],
}

/// `Confirm`/`Back`, off `Skin.xml`'s own `<NavigationController
/// name="NavigationController">`: four `Text` children,
/// `ControlTextConfirmButton`/`ControlTextConfirm`/`ControlTextBackButton`/
/// `ControlTextBack`, each an idstring - `FE_CONFIRM_BUTTON`/`FE_BACK_BUTTON`
/// resolve to `"ε"`/`"γ"`, the small-font glyphs for the cross/circle
/// buttons, the same private-glyph idiom `Cell Selection`'s own
/// `HELP`/`CHANGE DIFFICULTY` row already draws (there with a literal
/// `β`/`δ` baked into the XML instead of an idstring - the disc is not
/// consistent about which). `FE_CONFIRM`/`FE_BACK` resolve to the plain
/// words `"Confirm"`/`"Back"`.
#[derive(Debug, Clone, Default)]
pub struct NavigationLegend {
    prompts: Vec<Prompt>,
}

impl NavigationLegend {
    /// `root` is `oag_tables::fexml::parse`'s return over `Skin.xml`'s own
    /// *expanded* text (`oag_tables::fexml::text`, not
    /// `crate::screen::Screens::from_xml` - that type never keeps the raw
    /// tree this needs). `globals` is that same parse's `FEGlobals`
    /// declarations - `crate::screen::Screens::from_xml(xml).globals` reads
    /// them without needing this module to duplicate that collection pass.
    /// `None` when the file authors no `NavigationController` at all.
    #[must_use]
    pub fn read(
        root: &Node,
        globals: &HashMap<String, String>,
        strings: &crate::language::StringTable,
    ) -> Option<Self> {
        let controller = find_first(root, "NavigationController")?;
        let mut prompts = Vec::new();
        for text in controller.children_named("Text") {
            let Some(idstring) = text.value("idstring") else {
                continue;
            };
            prompts.push(Prompt {
                text: strings.get_or_id(idstring).to_string(),
                font: text.value("font").unwrap_or("Default").to_string(),
                x: number(globals, text, "x").unwrap_or(0.0),
                y: number(globals, text, "y").unwrap_or(0.0),
                color: color_of(globals, text),
            });
        }
        if prompts.is_empty() {
            return None;
        }
        Some(Self { prompts })
    }

    /// This legend's own draw list, at its disc-authored position -
    /// unconditional, not gated on anything; see the module doc for why.
    #[must_use]
    pub fn draw(&self, faces: &FaceScales) -> Vec<Draw> {
        self.prompts
            .iter()
            .map(|prompt| Draw::Text {
                x: prompt.x,
                y: prompt.y,
                scale: face_scale(faces, &prompt.font),
                color: prompt.color,
                border: None,
                align: Align::Left,
                text: prompt.text.clone(),
                wrap_width: None,
            })
            .collect()
    }
}

/// The scrolling tip ticker's own clip rect and style, off `Skin.xml`'s
/// `<TextInfo type="bar">` inside `<Viewport name="TextInfoIsAlwaysLast">`.
///
/// **Carries no content.** The sibling `TextInfo type="tag"`
/// (`NewsItemTagText`, drawn to the ticker's own left at a fixed `(30,
/// 235)`) is not represented here at all - it authors no `idstring` and no
/// literal `string` either, so drawing it would mean inventing whatever
/// short label the original fills in at runtime. Left undrawn, honestly,
/// rather than guessed.
#[derive(Debug, Clone)]
pub struct TickerLayout {
    /// `[x, y, width, height]`, PSP-native pixels - not enforced as a hard
    /// clip by [`ticker_draw`] (see its own doc), just where the text lands.
    pub viewport: [f32; 4],
    font: String,
    color: [f32; 4],
}

impl TickerLayout {
    /// `root`/`globals` are [`NavigationLegend::read`]'s own. `None` when
    /// the file authors no `TextInfoIsAlwaysLast` viewport - every title
    /// but Pulse today.
    #[must_use]
    pub fn read(root: &Node, globals: &HashMap<String, String>) -> Option<Self> {
        let viewport = find_named(root, "Viewport", "TextInfoIsAlwaysLast")?;
        let x = number(globals, viewport, "OffsetX").unwrap_or(0.0);
        let y = number(globals, viewport, "OffsetY").unwrap_or(0.0);
        let width = number(globals, viewport, "width").unwrap_or(0.0);
        let height = number(globals, viewport, "height").unwrap_or(0.0);
        // The `type="bar"` `TextInfo` nested inside this viewport - the
        // sibling `type="tag"` one sits outside it (a `LeftLayer`
        // ancestor's own child), so this search cannot cross into it.
        let bar = find_first(viewport, "TextInfo");
        let color = bar.map_or([1.0, 1.0, 1.0, 1.0], |bar| color_of(globals, bar));
        Some(Self {
            viewport: [x, y, width, height],
            // Neither `TextInfo` nor either `Text` child states a `font` -
            // **chosen, not measured**: `small` is picked as the plausible
            // read for a footer-height (13px-tall bar rows either side of
            // it) ticker line, not a traced binding.
            font: "small".to_string(),
            color,
        })
    }
}

/// How fast the ticker travels, in pixels a second. **Chosen, not
/// measured**: no PPSSPP capture in this pass isolated two frames a known
/// tick count apart with the ticker actually moving - see
/// `docs/ui/campaign-screens.md`'s own `[Open]` entry. Reuses
/// [`crate::anim::MARQUEE_SPEED`], this project's own existing scrolling-
/// text rate, as the nearest available precedent rather than a number
/// invented from nothing.
pub const TICKER_SPEED: f32 = crate::anim::MARQUEE_SPEED;

/// Gap between one tip and the next, in pixels - **chosen**, purely
/// cosmetic breathing room, not a measured value.
const GAP: f32 = 40.0;

/// The ticker's own draw list at `elapsed` seconds since the campaign
/// screen opened, cycling through `tips` (already-resolved disc strings -
/// see the module doc) at [`TICKER_SPEED`].
///
/// **Not glyph-clipped.** [`TickerLayout::viewport`] says where the strip
/// starts and how wide it is, but a tip's own text may draw a few pixels
/// past either edge for part of its cycle - the same simplification
/// `crate::marquee`'s row-value scroll avoids only because `Renderer`
/// trims that one row specially; wiring the identical per-glyph trim
/// through for a second, unrelated widget was judged out of this pass's
/// scope. Each visible tip is drawn up to twice (once at its own place in
/// the loop, once a full loop-width further along) so the strip has no
/// visible seam where it wraps.
#[must_use]
pub fn ticker_draw(
    layout: &TickerLayout,
    elapsed: f32,
    tips: &[String],
    faces: &FaceScales,
    measure: &dyn Fn(&str) -> f32,
) -> Vec<Draw> {
    if tips.is_empty() || layout.viewport[2] <= 0.0 {
        return Vec::new();
    }
    let scale = face_scale(faces, &layout.font);
    let widths: Vec<f32> = tips.iter().map(|tip| measure(tip) * scale).collect();
    let total: f32 = widths.iter().sum::<f32>() + GAP * tips.len() as f32;
    if total <= 0.0 {
        return Vec::new();
    }
    let travelled = (elapsed.max(0.0) * TICKER_SPEED) % total;
    let [vx, vy, vw, _] = layout.viewport;
    let mut out = Vec::new();
    let mut cursor = 0.0;
    for (tip, width) in tips.iter().zip(&widths) {
        for lap in [0.0, total] {
            let x = vx + cursor - travelled + lap;
            if x + width < vx || x > vx + vw {
                continue;
            }
            out.push(Draw::Text {
                x,
                y: vy,
                scale,
                color: layout.color,
                border: None,
                align: Align::Left,
                text: tip.clone(),
                wrap_width: None,
            });
        }
        cursor += width + GAP;
    }
    out
}

#[cfg(test)]
mod tests;
