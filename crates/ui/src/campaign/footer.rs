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
        // `1.0`, not `faces.default` (a ratio against the `menu` role,
        // 13/22): a `"default"`-labelled prompt now draws through its own
        // `Default`-role atlas at that face's own native size, the same
        // fix `oag_title::HelpText::scale`'s own doc gives for the
        // identical shape one widget over - see [`face_role`].
        "default" => 1.0,
        "small" => faces.small,
        _ => 1.0,
    }
}

/// The role a `font` value routes a [`Draw::in_role`] call to - `Default`
/// alone, for now: it is the one Pulse face this build ever loads a second,
/// real atlas for (see `crates/game/src/boot/fonts.rs`'s `face_atlas_slot`),
/// because it is the one Pulse face with real lowercase glyph art -
/// `Pulse_20.fnt`/`Pulse_14.fnt` (`menu`/`small`) give every lowercase
/// codepoint the identical box its uppercase twin has, measured off both
/// pressings. Routing a `"small"`-labelled prompt the same way would change
/// nothing it draws (`Pulse_14` has no lowercase to gain) and would be a
/// role this crate picked rather than one this build has verified drawing
/// through - see `docs/ui/menus-original.md`'s "Two faces, not one swapped
/// for the other" section.
fn face_role(font: &str) -> Option<&'static str> {
    font.eq_ignore_ascii_case("default")
        .then_some(crate::language::roles::DEFAULT)
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
    /// `Some` overrides [`Self::x`]/left alignment: draw right-aligned so
    /// the text *ends* here instead of starting at `x`. See
    /// [`NavigationLegend::read`]'s own doc for the one prompt this is
    /// built for.
    align_right_to: Option<f32>,
    /// `Some` alongside [`Self::align_right_to`]: the leftmost native pixel
    /// this prompt's own text may start at before [`NavigationLegend::draw`]
    /// shrinks its scale to fit, rather than let it run past this bound the
    /// way it otherwise would into whatever sits to its own left.
    left_bound: Option<f32>,
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
        let mut prompts: Vec<(String, Prompt)> = Vec::new();
        for text in controller.children_named("Text") {
            let Some(idstring) = text.value("idstring") else {
                continue;
            };
            prompts.push((
                idstring.to_string(),
                Prompt {
                    text: strings.get_or_id(idstring).to_string(),
                    // `ControlTextConfirmButton`/`BackButton` author
                    // `font="small"` themselves; `ControlTextConfirm`/`Back`
                    // (the words) author no `font` at all. `"default"` is
                    // the smaller of this build's two available scales
                    // (`docs/ui/menus-original.md`: `default` 13px, `small`
                    // 17px against a 22px face) - picked over `"small"`
                    // because it measurably reduces the overlap below, not
                    // because it is authored anywhere.
                    font: text.value("font").unwrap_or("default").to_string(),
                    x: number(globals, text, "x").unwrap_or(0.0),
                    y: number(globals, text, "y").unwrap_or(0.0),
                    color: color_of(globals, text),
                    align_right_to: None,
                    left_bound: None,
                },
            ));
        }
        if prompts.is_empty() {
            return None;
        }
        // `FE_CONFIRM`'s own word left-aligned at its authored `x="368"`
        // runs into `FE_BACK_BUTTON`'s own glyph at `x="415"` even at this
        // build's smallest available role scale - measured directly,
        // `data/scratch/lane-pulse/shots/crop-legend2-zoom.png`: "CONFIRM"
        // draws straight through the back button's own circle. This
        // build's only loaded menu-face atlas (`Pulse_20.fnt`, the `menu`
        // role every campaign-screen `Draw::Text` scales rather than
        // switches) is simply wider per glyph than whatever compact face
        // authored these 47 native pixels for - not a role this crate can
        // pick its way out of, see the module doc's own "not reachable"
        // note. Right-aligning to end just short of the back glyph fixed
        // that edge, but then ran the word into its *own* confirm-button
        // glyph instead (`crop-legend3-zoom.png`) - the word is simply
        // wider than the gap at any of this build's two available scales,
        // whichever side it is anchored from. [`NavigationLegend::draw`]
        // shrinks it to fit between both glyphs as a last resort, once
        // `measure` is in hand. **Chosen, not measured**: nothing on disc
        // says where the word should end or how far it may shrink, only
        // that the original's own (narrower) rendering fits without either.
        const GAP: f32 = 6.0;
        if let Some(back_button_x) = prompts
            .iter()
            .find(|(id, _)| id == "FE_BACK_BUTTON")
            .map(|(_, p)| p.x)
            && let Some(confirm_button_x) = prompts
                .iter()
                .find(|(id, _)| id == "FE_CONFIRM_BUTTON")
                .map(|(_, p)| p.x)
            && let Some((_, confirm)) = prompts.iter_mut().find(|(id, _)| id == "FE_CONFIRM")
        {
            confirm.align_right_to = Some(back_button_x - GAP);
            // The confirm-button glyph is one character - `measure` is not
            // reachable at parse time (no font atlas exists yet), so this
            // is a flat native-pixel estimate of one glyph's own width at
            // its `small`-role scale, not a measurement of `"ε"` itself.
            const GLYPH_WIDTH_ESTIMATE: f32 = 20.0;
            confirm.left_bound = Some(confirm_button_x + GLYPH_WIDTH_ESTIMATE + GAP);
        }
        Some(Self {
            prompts: prompts.into_iter().map(|(_, prompt)| prompt).collect(),
        })
    }

    /// This legend's own draw list, at its disc-authored position -
    /// unconditional, not gated on anything; see the module doc for why.
    /// `measure` is a face's own text-width function at scale `1.0` - the
    /// same indirection `crate::marquee::apply`'s own `measure` argument
    /// is, so this module never has to know what a font atlas is either.
    #[must_use]
    pub fn draw(&self, faces: &FaceScales, measure: &dyn Fn(&str) -> f32) -> Vec<Draw> {
        self.prompts
            .iter()
            .map(|prompt| {
                let mut scale = face_scale(faces, &prompt.font);
                let x = match prompt.align_right_to {
                    Some(right_to) => {
                        // Shrink just enough that the right-aligned text's
                        // own left edge does not cross `left_bound` - see
                        // `NavigationLegend::read`'s own doc for why this
                        // exists at all.
                        if let Some(left_bound) = prompt.left_bound {
                            let width = measure(&prompt.text) * scale;
                            let available = right_to - left_bound;
                            if width > available && width > 0.0 {
                                scale *= available.max(0.0) / width;
                            }
                        }
                        right_to
                    }
                    None => prompt.x,
                };
                Draw::in_role(
                    face_role(&prompt.font),
                    x,
                    prompt.y,
                    scale,
                    prompt.color,
                    if prompt.align_right_to.is_some() {
                        Align::Right
                    } else {
                        Align::Left
                    },
                    prompt.text.clone(),
                )
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
            // **chosen**, not a traced binding, but corroborated against
            // `data/reference/psp-campaign-screens/cellselect-grid0_3_2.png`:
            // the ticker's own text there sits at the same small size as
            // `Help`/`Confirm`/`AI difficulty`, not the panel's bigger face.
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

/// The ticker's own draw at `elapsed` seconds since the campaign screen
/// opened, cycling through `tips` (already-resolved disc strings - see the
/// module doc) at [`TICKER_SPEED`]. `None` when there is nothing to show -
/// no tips, no viewport, or (briefly, once a cycle) the gap between one tip
/// leaving and the next entering.
///
/// **Exactly one `Draw::Text` at a time, on purpose.** An earlier version of
/// this function drew a tip up to twice (once at its own place in the loop,
/// once a full loop-width further along) so the strip had no visible seam
/// at the wrap - but that only works unclipped, and the caller now clips
/// this single draw to [`TickerLayout::viewport`] the same way
/// `crate::marquee`'s row-value scroll clips itself (`Renderer::render_with`'s
/// `clip: Option<(usize, f32, f32)>`, keyed on the draw's own index in the
/// flattened list). That mechanism clips one index, not several, so a
/// caller wanting the render-level clip needs one draw to point it at - see
/// `crate::main::campaign_stage::CampaignStage::ticker_draw` in `oag-game`
/// for where the index is found. The trade is an honest half-second of
/// nothing shown at each wrap rather than a seam this build cannot clip
/// away, and a real player never sees it: the eye follows leaving/entering
/// text, not the gap between two texts *neither* on screen. Reproduces
/// `cellselect-grid0_3_2.png`'s own clean cut at both edges of the ticker's
/// own band.
#[must_use]
pub fn ticker_draw(
    layout: &TickerLayout,
    elapsed: f32,
    tips: &[String],
    faces: &FaceScales,
    measure: &dyn Fn(&str) -> f32,
) -> Option<Draw> {
    if tips.is_empty() || layout.viewport[2] <= 0.0 {
        return None;
    }
    let scale = face_scale(faces, &layout.font);
    let widths: Vec<f32> = tips.iter().map(|tip| measure(tip) * scale).collect();
    let total: f32 = widths.iter().sum::<f32>() + GAP * tips.len() as f32;
    if total <= 0.0 {
        return None;
    }
    let travelled = (elapsed.max(0.0) * TICKER_SPEED) % total;
    let [vx, vy, vw, _] = layout.viewport;
    let mut cursor = 0.0;
    for (tip, width) in tips.iter().zip(&widths) {
        let x = vx + cursor - travelled;
        if x + width >= vx && x <= vx + vw {
            return Some(Draw::in_role(
                face_role(&layout.font),
                x,
                vy,
                scale,
                layout.color,
                Align::Left,
                tip.clone(),
            ));
        }
        cursor += width + GAP;
    }
    None
}

#[cfg(test)]
mod tests;
