//! The front-end root's own footer fixtures: the `NavigationController`'s
//! `Confirm`/`Back` prompts and the scrolling tip ticker, both authored once
//! on the shared `FE Screen` (`Data\Plugins\PI001\GUI\Skin.xml` on Pulse,
//! `Data\Plugins\Frontend\Gui\Skin.xml` on Wipeout HD/Fury and Omega -
//! `Top FE Screen -> FE Screen -> BodgeScreenContainingNavigationController`
//! on the latter two, that literal wrapper name being the disc's own)
//! rather than on any individual navigable screen's own definition file.
//! Neither shape is one [`oag_ui::screen::Screens::collect_widgets`]
//! recognises at all - `NavigationController` and `TextInfo` fall through
//! its match to the catch-all arm, children and all.
//!
//! Read directly off the raw parsed tree (`oag_tables::fexml::parse`'s
//! `Node`) instead of through that model, because the model has no way to
//! express either idiom even if the tag were recognised: a
//! `NavigationController` picks *per screen* which prompts to show, which
//! is not a fact any named `Screen` carries; and the ticker's own `TextInfo
//! type="bar"` is two alternating text buffers sharing one clip viewport,
//! not a single positioned widget with a string.
//!
//! **Where this build shows `Confirm`/`Back`, and on what evidence.**
//! `Cell Selection` (both Pulse's own screen and Wipeout HD/Fury's and
//! Omega's) shows both unconditionally - the one screen a PPSSPP capture
//! measured wanting them (`docs/ui/campaign-screens.md`'s 2026-09-14
//! pass), and neither title's own campaign screen file authors a
//! per-screen gate to read instead. Every ordinary menu page
//! (`crate::main::menu_stage::MenuStage::render` in `oag-game`) shows
//! `Confirm` always and `Back` only past this build's own tree root
//! (`Menu::depth() > 1`) - **chosen** for this build's own tree, which no
//! disc screen names, but corroborated rather than picked for symmetry:
//! see `docs/architecture/menus.md`'s "A mouse and a finger" section for
//! the `NavigationButtons` census and the two real RPCS3 captures that
//! measure the identical root/non-root split on the real title. HD/Fury's
//! own `EndRace Results`/`Menu`/`Rewards` each author their own *local*
//! `NavigationController` instead of using the shared one, with no `Back`
//! `Text` at all - read the same way, through
//! `oag_ui_screens::endrace::hd::hd_results_draw_list` and its siblings, not
//! through this module.
//!
//! **HD/Fury's own icon glyph draws, 2026-09-25, through its own atlas**
//! (Omega very likely too - unverified this pass, see
//! `oag_ui::language::roles::BUTTONS`'s own doc). Their
//! `ControlTextConfirmButton`/`BackButton` author
//! `font="buttons"` (`ps_buttons.fnt`/`PS_BUTTONS.fnt`) - a role this build
//! now loads through `oag_ui::language::roles::BUTTONS`
//! (`oag_game::boot::fonts::load_buttons_font`) the same way `Default` is,
//! into its own third GPU-side slot (`crate::render::Renderer::set_buttons_atlas`)
//! rather than the one `face_atlas_slot` already spends on `Title`/`Default`.
//! Used to be excluded here entirely, on the reasoning that a Greek-letter
//! fallback through the *wrong* atlas is worse than nothing per `CLAUDE.md`'s
//! "never invent" rule - which still holds, and is exactly why this reads a
//! dedicated atlas rather than routing through `Default`/`Title`. See
//! [`face_role`]'s own doc for the verification this rests on.
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

use crate::picker::FaceScales;
use oag_ui::frontend::{Align, Draw};
use oag_ui::screen::{argb_to_rgba, parse_argb};

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
/// and `Buttons`, so far.
///
/// **`Default`**: it is the one Pulse face this build ever loads a second,
/// real atlas for (see `crates/game/src/boot/fonts.rs`'s `face_atlas_slot`),
/// because it is the one Pulse face with real lowercase glyph art -
/// `Pulse_20.fnt`/`Pulse_14.fnt` (`menu`/`small`) give every lowercase
/// codepoint the identical box its uppercase twin has, measured off both
/// pressings. Routing a `"small"`-labelled prompt the same way would change
/// nothing it draws (`Pulse_14` has no lowercase to gain) and would be a
/// role this crate picked rather than one this build has verified drawing
/// through - see `docs/ui/menus-original.md`'s "Two faces, not one swapped
/// for the other" section.
///
/// **`Buttons`**: Wipeout HD/Fury's own `font="buttons"`,
/// `ps_buttons.fnt`/`PS_BUTTONS.fnt` (Omega very likely, unverified this
/// pass - see `oag_ui::language::roles::BUTTONS`'s own doc), a genuine second atlas
/// `oag_game::boot::fonts::load_buttons_font` loads through
/// `oag_ui::language::roles::BUTTONS` the same way `Default` is, into a
/// *third* GPU-side face slot (`crate::render::Renderer::set_buttons_atlas`)
/// rather than the one `face_atlas_slot` already spends on `Title`/`Default`,
/// because the two are needed in the same frame (a screen title and this
/// screen's own footer) and one slot cannot serve both. Verified against the
/// disc: `oag-tools --example hd_buttons_font_probe` decodes real
/// circled-glyph art at the codepoints `FE_CONFIRM_BUTTON`/`FE_BACK_BUTTON`/
/// `DifficultyButtonIcon` resolve to (`ε`/`γ`/`δ`), a cross, a circle and a
/// square, matching `data/scratch/drive-2026-09-25/hd-footer-glyphs/atlas-confirm-back-difficulty.png`
/// against the RPCS3 captures shape for shape (compared by eye, not a pixel
/// diff). **Not Pulse**: Pulse's own
/// `ControlTextConfirmButton`/`BackButton` author `font="small"`, not
/// `"buttons"` (`docs/ui/campaign-screens.md`'s "measured on RPCS3"
/// sections), so this never changes what Pulse draws.
///
/// `pub(super)`, not private: [`super::draw::text_draw`] routes `Grid
/// Selection`/`Cell Selection`'s own body text through the identical check,
/// once lane 7's own `Default`-role atlas made that mean something (see
/// `docs/ui/campaign-screens.md`'s "Every label on this screen still
/// renders upper-case" note, closed by that atlas landing) - which is also
/// what makes `Cell Selection`'s own `DifficultyButtonIcon` (`font="buttons"`,
/// a plain nested `Text` under `DifficultyButton`, no part of
/// `NavigationController` at all) draw through this same mapping with no
/// change to `draw.rs` itself.
pub(super) fn face_role(font: &str) -> Option<&'static str> {
    if font.eq_ignore_ascii_case("default") {
        return Some(oag_ui::language::roles::DEFAULT);
    }
    if font.eq_ignore_ascii_case("buttons") {
        return Some(oag_ui::language::roles::BUTTONS);
    }
    None
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

/// The first `<MiniText>` under `node` whose `idstring` is `id`.
fn find_minitext<'a>(node: &'a Node, id: &str) -> Option<&'a Node> {
    if node.name.eq_ignore_ascii_case("MiniText") && node.value("idstring") == Some(id) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_minitext(child, id))
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

/// Which half of a `NavigationController` a [`Prompt`] belongs to - the
/// axis [`NavigationLegend::draw_gated`] filters on. Not carried by every
/// caller: [`NavigationLegend::draw`] draws every prompt regardless, the
/// same unconditional behaviour this module has always had.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PromptKind {
    Confirm,
    Back,
    /// HD's `NAVIGATION` entry: the d-pad glyph and its word, authored beside
    /// the controller rather than in it.
    Navigation,
}

/// How far above the legend's own row [`NavigationLegend::notice`] sits, in
/// the screen's own grid units. Chosen, not measured.
const NOTICE_LIFT: f32 = 56.0;

/// One `NavigationController` prompt - an icon glyph and its label, both
/// resolved through the string table already.
#[derive(Debug, Clone)]
struct Prompt {
    text: String,
    font: String,
    x: f32,
    y: f32,
    color: [f32; 4],
    kind: PromptKind,
    /// The widget's own authored `scale`, `1.0` when it names none - Pulse's
    /// four `Text`s never do, but Wipeout HD/Fury's own word half
    /// (`ControlTextConfirm`/`Back`) authors `scale="0.8"` on both the
    /// shared Bodge controller and every `EndRace_Definition.xml` copy,
    /// which was previously read and silently dropped: [`face_scale`] only
    /// ever answered from the widget's `font`, never its own `scale`.
    /// Multiplied into [`NavigationLegend::draw_prompt`]'s own scale rather
    /// than replacing it, so Pulse's font-keyed answer is unchanged (`1.0`
    /// there, a no-op) and HD's own authored shrink applies on top of it.
    scale: f32,
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

/// `Confirm`/`Back`, off a `<NavigationController name="NavigationController">`
/// anywhere in `root`: two `Text` children carrying either half,
/// `ControlTextConfirmButton`/`ControlTextConfirm` or
/// `ControlTextBackButton`/`ControlTextBack`, each an idstring -
/// `FE_CONFIRM_BUTTON`/`FE_BACK_BUTTON` resolve to `"ε"`/`"γ"`, the
/// small-font glyphs for the cross/circle buttons, the same private-glyph
/// idiom `Cell Selection`'s own `HELP`/`CHANGE DIFFICULTY` row already draws
/// (there with a literal `β`/`δ` baked into the XML instead of an idstring -
/// the disc is not consistent about which). `FE_CONFIRM`/`FE_BACK` resolve
/// to the plain words `"Confirm"`/`"Back"`.
///
/// **Not Pulse-only.** Originally read only off Pulse's own `Skin.xml`
/// (`Data\Plugins\PI001\GUI\Skin.xml`'s `FE Screen`), this reads the
/// identical shape off Wipeout HD/Fury's own shared front-end root
/// (`Data\Plugins\Frontend\Gui\Skin.xml`) too - `Top FE Screen -> FE Screen
/// -> BodgeScreenContainingNavigationController` (that literal name is the
/// disc's own), six `Text` children rather than four: the same
/// `ControlTextConfirmButton`/`Confirm`/`BackButton`/`Back` plus
/// `ControlTextInviteButton`/`ControlTextInvite`, an online multiplayer
/// invite prompt (`StartEnabled="false"` on the disc, and irrelevant here
/// regardless - multiplayer is out of scope). [`Self::read`] filters by the
/// widget's own `name` attribute to exactly the four `ControlText*` names
/// above, not "any `Text` with an idstring" - without that filter, HD's
/// `EndRace Results` screen would also pull in its own
/// `RecordsCycleButton`/`RecordsCycle` (a leaderboard-cycle button, same
/// online-only shape, sitting in that screen's own *local*
/// `NavigationController` beside its Confirm prompt - see
/// `oag_ui_screens::endrace::hd::hd_results_draw_list`'s own doc for where that
/// pair is excluded a second time, by name, in the ordinary text-widget
/// path once `oag_ui::screen::Screens::collect_widgets` started walking
/// `NavigationController` at all).
#[derive(Debug, Clone, Default)]
pub struct NavigationLegend {
    prompts: Vec<Prompt>,
    /// The root's own `FE_SCREEN_TITLE` caption over the page title, where the
    /// file authors one - HD's. Drawn the way `Team Selection`'s headings are.
    caption: Option<crate::picker::hd::MiniText>,
}

impl NavigationLegend {
    /// `root` is `oag_tables::fexml::parse`'s return over the front-end
    /// root's own *expanded* text (`oag_tables::fexml::text` on Pulse;
    /// plain UTF-8 on Wipeout HD/Fury and Omega, the same divergence
    /// `crate::campaign::load_hd`'s own doc gives for every other screen
    /// file) - not `oag_ui::screen::Screens::from_xml`, which never keeps the
    /// raw tree this needs. `globals` is that same parse's `FEGlobals`
    /// declarations - `oag_ui::screen::Screens::from_xml(xml).globals` reads
    /// them without needing this module to duplicate that collection pass.
    /// `None` when the file authors no `NavigationController` at all, or
    /// authors one with neither half this reads.
    #[must_use]
    pub fn read(
        root: &Node,
        globals: &HashMap<String, String>,
        strings: &oag_ui::language::StringTable,
    ) -> Option<Self> {
        let controller = find_first(root, "NavigationController")?;
        let mut prompts: Vec<(String, Prompt)> = Vec::new();
        for text in controller.children_named("Text") {
            let Some(idstring) = text.value("idstring") else {
                continue;
            };
            // The four known `Confirm`/`Back` widgets, by their own `name`
            // attribute - not every `Text` with an idstring. See
            // [`NavigationLegend`]'s own doc for the widget this excludes
            // that would otherwise leak through (`RecordsCycle`), and the
            // one `StartEnabled="false"` on disc regardless (`Invite`).
            let Some(kind) = text.attr("name").and_then(|name| match name {
                "ControlTextConfirmButton" | "ControlTextConfirm" => Some(PromptKind::Confirm),
                "ControlTextBackButton" | "ControlTextBack" => Some(PromptKind::Back),
                _ => None,
            }) else {
                continue;
            };
            // **Wipeout HD/Fury's own icon half draws now, through its own
            // atlas.** `ControlTextConfirmButton`/`BackButton` author
            // `font="buttons"` there (`ps_buttons.fnt`, confirmed present in
            // `DATA02` - `crates/game/src/boot/fonts.rs`'s own module doc);
            // [`face_role`] maps that to [`oag_ui::language::roles::BUTTONS`],
            // a real third atlas `oag_game::boot::fonts::load_buttons_font`
            // loads (see [`face_role`]'s own doc for the `oag-tools --example
            // hd_buttons_font_probe` verification this rests on: the
            // codepoints these two widgets and `Cell Selection`'s own
            // `DifficultyButtonIcon` resolve to - `"ε"`/`"γ"`/`"δ"` - decode
            // to real circled cross/circle/square glyph art, not empty boxes).
            // Used to be excluded here entirely, on the reasoning that a
            // Greek-letter fallback through the *wrong* atlas is worse than
            // nothing per `CLAUDE.md`'s "never invent" rule - which still
            // holds, and is exactly why this reads a dedicated atlas rather
            // than routing through `Default`/`Title`.
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
                    kind,
                    scale: number(globals, text, "scale").unwrap_or(1.0),
                    align_right_to: None,
                    left_bound: None,
                },
            ));
        }
        // The d-pad glyph and `FE_NAVIGATION`, two `Text`s the root authors as
        // siblings of the controller (`ControlTextNavigationButton`,
        // `ControlTextNavigation`), not children: RPCS3 draws them first in
        // the footer on every screen (`NAVIGATION` at authored `x=260`). Only a
        // file that authors them gets them, which is HD's.
        let navigation = [
            ("ControlTextNavigationButton", "FE_NAVIGATION_BUTTON"),
            ("ControlTextNavigation", "FE_NAVIGATION"),
        ];
        if !prompts.is_empty() {
            for (name, key) in navigation {
                let Some(text) = find_named(root, "Text", name) else {
                    continue;
                };
                let word = match text.value("idstring") {
                    Some(id) => strings.get_or_id(id).to_string(),
                    None => text.value("string").unwrap_or_default().to_string(),
                };
                prompts.push((
                    key.to_string(),
                    Prompt {
                        text: word,
                        font: text.value("font").unwrap_or("default").to_string(),
                        x: number(globals, text, "x").unwrap_or(0.0),
                        y: number(globals, text, "y").unwrap_or(0.0),
                        color: color_of(globals, text),
                        kind: PromptKind::Navigation,
                        scale: number(globals, text, "scale").unwrap_or(1.0),
                        align_right_to: None,
                        left_bound: None,
                    },
                ));
            }
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
        //
        // **Gated on the word's own authored scale, not on the title.**
        // Wipeout HD/Fury's own `ControlTextConfirm` authors `scale="0.8"`
        // (`Skin.xml`'s `<Values idstring="FE_CONFIRM" x="534" ... scale="0.8">`),
        // unlike Pulse's, which authors none (`1.0` by default, see
        // [`Prompt::scale`]'s own doc) - a disc-authored fact, not a title
        // check this crate would otherwise have to invent one of. RPCS3
        // draws HD's own `CONFIRM` left-aligned at that authored `x="534"`,
        // 186 native pixels short of `BACK`'s own `x="720"`
        // (`data/scratch/drive-2026-09-25/hd-footer-glyphs/difficulty-icon-zoom.png`
        // and the clean `02-square.png`/`02-triangle.png` captures) - plenty
        // of room at `scale="0.8"`, so applying Pulse's own shrink-to-fit
        // hack there would move `CONFIRM` to a position and alignment RPCS3
        // never shows, not fix an overlap that does not exist on this title.
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
            && confirm.scale > 0.99
        {
            confirm.align_right_to = Some(back_button_x - GAP);
            // The confirm-button glyph is one character - `measure` is not
            // reachable at parse time (no font atlas exists yet), so this
            // is a flat native-pixel estimate of one glyph's own width at
            // its `small`-role scale, not a measurement of `"ε"` itself.
            const GLYPH_WIDTH_ESTIMATE: f32 = 20.0;
            confirm.left_bound = Some(confirm_button_x + GLYPH_WIDTH_ESTIMATE + GAP);
        }
        // The `<MiniText idstring="FE_SCREEN_TITLE">` the root authors at
        // `(160, 48)`: RPCS3 draws the string itself, `SCREEN TITLE`, a
        // placeholder the shipped game never replaced, over every page's
        // title, so it is drawn as authored.
        let caption =
            find_minitext(root, "FE_SCREEN_TITLE").map(|text| crate::picker::hd::MiniText {
                x: number(globals, text, "x").unwrap_or(0.0),
                y: number(globals, text, "y").unwrap_or(0.0),
                text: strings.get_or_id("FE_SCREEN_TITLE").to_string(),
                color: text
                    .value("color")
                    .and_then(|value| resolve(globals, value))
                    .and_then(parse_argb)
                    .unwrap_or(0xFFFF_FFFF),
            });
        Some(Self {
            prompts: prompts.into_iter().map(|(_, prompt)| prompt).collect(),
            caption,
        })
    }

    /// This legend's own draw list, at its disc-authored position -
    /// unconditional, not gated on anything; see the module doc for why.
    /// `measure` is a face's own text-width function at scale `1.0` - the
    /// same indirection `crate::marquee::apply`'s own `measure` argument
    /// is, so this module never has to know what a font atlas is either.
    #[must_use]
    pub fn draw(&self, faces: &FaceScales, measure: &dyn Fn(&str) -> f32) -> Vec<Draw> {
        let mut out: Vec<Draw> = self
            .prompts
            .iter()
            .map(|prompt| Self::draw_prompt(prompt, faces, measure))
            .collect();
        self.draw_caption(&mut out);
        out
    }

    fn draw_caption(&self, out: &mut Vec<Draw>) {
        if let Some(caption) = &self.caption {
            crate::picker::hd::draw_labels(std::slice::from_ref(caption), out);
        }
    }

    /// [`Self::draw`], with [`PromptKind::Back`] left out when `show_back`
    /// is `false`. [`PromptKind::Confirm`] is never gated - every menu page
    /// this build draws has something a `Confirm` press can act on, and
    /// nothing on either title's own disc suggests otherwise (Pulse's
    /// `Skin.xml` carries no per-screen visibility fact at all; Wipeout
    /// HD/Fury's own `NavigationButtons` bitmask, read off `Main Menu` and
    /// its siblings, never clears the `Confirm` bit on a screen this build
    /// would call a menu page - see `docs/architecture/menus.md`'s "A mouse
    /// and a finger" section for the full census and where this rule is
    /// used).
    #[must_use]
    pub fn draw_gated(
        &self,
        faces: &FaceScales,
        measure: &dyn Fn(&str) -> f32,
        show_back: bool,
    ) -> Vec<Draw> {
        let mut out: Vec<Draw> = self
            .prompts
            .iter()
            .filter(|prompt| show_back || prompt.kind != PromptKind::Back)
            .map(|prompt| Self::draw_prompt(prompt, faces, measure))
            .collect();
        self.draw_caption(&mut out);
        out
    }

    fn draw_prompt(prompt: &Prompt, faces: &FaceScales, measure: &dyn Fn(&str) -> f32) -> Draw {
        let mut scale = face_scale(faces, &prompt.font) * prompt.scale;
        let x = match prompt.align_right_to {
            Some(right_to) => {
                // Shrink just enough that the right-aligned text's own left
                // edge does not cross `left_bound` - see
                // `NavigationLegend::read`'s own doc for why this exists at
                // all.
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
    }

    /// `text` as one line in the legend's own word face (not its button glyphs) and colour, a row above the
    /// prompts and starting where the leftmost of them does - what
    /// `Cell Selection` says when a confirmed cell cannot launch. `None` when
    /// the legend has no prompt to take its face from.
    ///
    /// **The row and the left edge are chosen, not measured**: the disc authors
    /// no line for this.
    #[must_use]
    pub fn notice(&self, text: &str, faces: &FaceScales) -> Option<Draw> {
        let base = self.prompts.iter().find(|prompt| {
            prompt.kind == PromptKind::Confirm && prompt.font.eq_ignore_ascii_case("default")
        })?;
        let left = self
            .prompts
            .iter()
            .filter(|prompt| prompt.kind != PromptKind::Navigation)
            .map(|prompt| prompt.x)
            .fold(base.x, f32::min);
        let prompt = Prompt {
            text: text.to_string(),
            x: left,
            y: base.y - NOTICE_LIFT,
            align_right_to: None,
            left_bound: None,
            ..base.clone()
        };
        Some(Self::draw_prompt(&prompt, faces, &|_| 0.0))
    }

    /// One line saying which halves this legend actually has to draw - for
    /// a boot report, the same idiom `oag_ui::menu::Frame::describe` and
    /// `oag_ui::backdrop::Fury::describe` already use, so a source whose
    /// icon glyph was left out (see [`Self::read`]'s own doc) says so
    /// without a screenshot.
    #[must_use]
    pub fn describe(&self) -> String {
        let has = |kind: PromptKind| self.prompts.iter().any(|prompt| prompt.kind == kind);
        match (has(PromptKind::Confirm), has(PromptKind::Back)) {
            (true, true) => "Confirm+Back".to_string(),
            (true, false) => "Confirm only".to_string(),
            (false, true) => "Back only".to_string(),
            // Unreachable in practice - `Self::read` returns `None` rather
            // than a legend with no prompts at all - kept exhaustive rather
            // than a `_` so a future prompt kind cannot silently fall here.
            (false, false) => "no prompts".to_string(),
        }
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
            // Neither `TextInfo` nor either `Text` child states a `font`.
            // **Chosen, corrected 2026-10-08 against a PPSSPP capture**
            // (`docs/ui/menus-original.md`, "What still differs, ranked"):
            // the ticker's tip is real lowercase in the same face as `Help`/
            // `Confirm`/`AI difficulty` beside it, which only `Default`
            // (`pulse_text.fnt`) has - the earlier `"small"` drew it in
            // `Pulse_14`-scaled capitals. Not a traced binding either.
            font: "default".to_string(),
            color,
        })
    }
}

/// How fast the ticker travels, in pixels a second. **Chosen, not
/// measured**: no PPSSPP capture in this pass isolated two frames a known
/// tick count apart with the ticker actually moving - see
/// `docs/ui/campaign-screens.md`'s own `[Open]` entry. Reuses
/// [`oag_ui::anim::MARQUEE_SPEED`], this project's own existing scrolling-
/// text rate, as the nearest available precedent rather than a number
/// invented from nothing.
pub const TICKER_SPEED: f32 = oag_ui::anim::MARQUEE_SPEED;

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
