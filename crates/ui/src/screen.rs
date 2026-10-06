//! The front end read out of its own XML.
//!
//! Pulse's front end is data, not code. `Data\Plugins\PI001\GUI\Skin.xml` in
//! `Data.wad` holds the boot screens, the `FEGlobals` colour and layout
//! variables, and a `LoadXML` list that pulls in every menu. This module turns
//! the expanded XML into a model the boot sequence can drive.
//!
//! Two conventions from the format matter here:
//!
//! - **`Values` carries its parent's attributes.** `<Movie><Values src="..."/>`
//!   means the `Movie` has a `src`. Attributes also appear directly on the
//!   element, so both are merged, with the element's own winning.
//! - **`FEGlobals->Name` is an indirection** into the `<Variable global="Name">`
//!   list. [`Screens::resolve`] follows it.
//!
//! See `docs/formats/fexml.md` and `docs/architecture/frontend-boot.md`.

use std::collections::HashMap;

// The XML tree parser itself is a format concern and lives in `oag-formats`
// next to the dictionary expander, so the front end and the handling-stats
// decoder share one copy rather than growing two. Re-exported because this
// module is where the rest of the crate reaches for it.
use oag_core::buttons::Button;
pub use oag_tables::fexml::{Node, parse};

mod color;
mod fade;
mod movie;
mod read;
#[cfg(test)]
use movie::has_movie_extension;
mod block;
mod mode3d;
mod reveal;
mod settings;
mod tag_input;
mod touch;
mod widgets;
pub use block::BlockWidget;
pub use color::{argb_to_rgba, parse_argb};
pub use fade::{MEASURED_HIDDEN_WIDGET_FADE_IN_SECONDS, resolve_fade_in};
pub use mode3d::{Mode3dModel, Model};
pub use movie::{DEFAULT_REGION, MOVIE_EXTENSIONS, Movie};
pub use reveal::{RevealKey, interpolate_reveal};
pub use settings::{TouchList, TouchListEntry, TouchSlider};
pub use tag_input::TagInput;
pub use touch::{Include, TouchButton};

/// One `Text` widget.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Text {
    /// Widget name, when it has one.
    pub name: Option<String>,
    /// String-table id to look up.
    pub idstring: Option<String>,
    /// A literal string, used instead of `idstring` by a few screens.
    pub string: Option<String>,
    /// Font name: `Default`, `Small`, `Title`, `Menu`, `HUD`.
    pub font: String,
    /// Position in the PSP's 480x272 screen space.
    pub x: f32,
    /// Position in the PSP's 480x272 screen space.
    pub y: f32,
    /// Scale multiplier.
    pub scale: f32,
    /// ARGB, as the XML writes it.
    pub color: u32,
    /// `left`, `right` or `centre`.
    pub align: String,
    /// `vertalign="middle"`: [`Self::y`] is the line's middle, not its top.
    /// Wipeout 2048's `BOOT_PRESS_ANY` and its save-check text author it;
    /// none of the PSP or PS3 screens this crate's tests parse does. `false`
    /// when absent, which anchors the pen at `y` exactly as before this
    /// field existed - and it only moves a widget once the sequence knows
    /// its face's height (`frontend::Frontend::set_face_scales`).
    pub middle: bool,
    /// Whether the widget starts hidden.
    pub start_enabled: bool,
    /// Whether the widget throbs once visible, rather than sitting still.
    ///
    /// The one measured case is `BOOT_PRESS_START` on `Show Logo`: its period
    /// and depth are read off a real capture in
    /// `docs/architecture/frontend-boot.md` (confidence 75) and hard-coded in
    /// `frontend::draw`'s `PULSE_PERIOD`/`PULSE_FLOOR` rather than re-derived
    /// per widget, since no other `pulse="true"` widget has been captured yet
    /// to say whether they share the same numbers.
    pub pulse: bool,
    /// Seconds after the widget's screen appears before it starts drawing at
    /// all, from the `delay` attribute. Zero when absent, which draws at once
    /// exactly as before this field existed.
    ///
    /// Every widget carries a `delay` in the XML this crate has seen (Pure's
    /// too), not only pulsing ones - see [`Self::pulse`]'s sibling gap noted
    /// on [`Skin::collect_widgets`]'s own doc comment, `delay`/`transition`
    /// together. Only a `pulse` widget acts on it here; a non-pulsing widget's
    /// `delay` is parsed and otherwise unused, the same trade `Image::auto_load`
    /// already makes for a flag this crate reads but does not yet act on.
    pub delay: f32,
    /// Seconds the widget takes to fade in when its enclosing `LeftLayer`
    /// first shows, from that container's own `transition` attribute -
    /// zero for a widget with no such ancestor, which fades over no time at
    /// all (drawn at once, exactly as before this field existed). See
    /// `oag_ui_screens::picker::body`, the one reader: `Data\Plugins\PI001\GUI\
    /// Selection_Definition.xml`'s `<LeftLayer transition="0.5">` groups
    /// are `Track Creation`'s and `Team Selection`'s info panel, and a live
    /// PPSSPP capture (`docs/ui/selection-screens.md`) shows exactly that
    /// panel fading in over about half a second on screen entry while the
    /// title bar's own `<LeftLayer transition="0">` group is solid from the
    /// first frame - the two data points this field is read against.
    pub transition: f32,
    /// The pixel width to wrap at, when `widthlimited="true"` is set.
    ///
    /// Taken from the nearest enclosing `Viewport`'s own `width` - the only
    /// number the XML gives a `widthlimited` text to wrap against. `None`
    /// when `widthlimited` is absent or false, and also when it is true but
    /// nothing wraps the widget in a `Viewport` (nothing to wrap against).
    /// `Show Logo`'s `USLegalText`/`BOOT_LEGAL` is the one measured case:
    /// `Viewport` gives `width="400"`, `x="40"` matching the text's own `x`,
    /// so viewport-width and width-minus-inset agree and cannot be told
    /// apart from this one sample. Confidence 60 on "viewport width, not
    /// inset" as the general rule; 95 that it is right for this screen. See
    /// `docs/architecture/frontend-boot.md`.
    pub wrap_width: Option<f32>,
}

/// One `Redirect`: a button, and the screen it goes to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Redirect {
    /// Widget name, when it has one. `AutoRedirect` is fired by a finishing
    /// movie; `LanguageAutoRedirect` by the language menu.
    pub name: Option<String>,
    /// Abstract button index that fires it forward, if any.
    pub forward: Option<Button>,
    /// Abstract button index that fires it backward, if any.
    pub backward: Option<Button>,
    /// Target screen name from the `Default` child's `goto`.
    pub goto: Option<String>,
    /// Seconds after the screen appears before this redirect fires on its
    /// own, from the `delay` attribute - `<Redirect name="Redirect"
    /// delay="4.0">` on Wipeout 2048's `Boot Studio Logo` is the one
    /// authored case (`docs/formats/2048-frontend.md`). `None` when the
    /// widget carries none, which is every redirect on every other title:
    /// a redirect with no button and no delay is fired by code, and what
    /// fires it is a question `frontend::updates` leaves open rather than
    /// answers with a timer.
    pub delay: Option<f32>,
}

/// An `Image` widget that names a texture.
///
/// The solid-colour kind, which has a `color` and no `src`, is a [`Fill`] and
/// goes to [`Screen::fills`] instead.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// Widget name, when it has one.
    pub name: Option<String>,
    /// The texture's archive entry name, with the extension the XML gives -
    /// or, for a widget the XML leaves `src`-less entirely, a
    /// `hash:`-prefixed spec out of a title's own `fallback_images` table.
    /// See [`Screens::from_xml_with_fallbacks`].
    pub src: String,
    /// Left edge. Absent in the XML means zero, as it does for `Text`.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Authored width, when the XML gives one. Absent means the texture's own.
    pub width: Option<f32>,
    /// Authored height, likewise.
    pub height: Option<f32>,
    /// `Centred="true"`: [`Self::x`]/[`Self::y`] name the widget's centre
    /// rather than its top-left corner. Wipeout 2048's boot screens author
    /// their logos this way (`Title_Screen.gtf` at `x="480" y="220"`,
    /// centred on a 960-wide grid); none of the PSP or PS3 screens this
    /// crate's tests parse carries the attribute. Resolved at draw time rather than here because the
    /// half-size to subtract is the texture's own when the widget authors
    /// no `width`/`height`, and that is not known until the sheet is built.
    pub centred: bool,
    /// Modulating colour as ARGB. White when unstated, which leaves the
    /// texture's own colours alone.
    pub color: u32,
    /// The `U` attribute: left edge of the sub-rect this widget samples out
    /// of `src`, in the texture's own pixels. `None` when the XML gives none,
    /// which means the whole texture.
    ///
    /// **Several widgets share one texture this way on the real discs.**
    /// `ArrowSelect` samples `FETextures.mip` at `U="53" V="0"`; Pure's
    /// `Title Screen->Press start button` samples `FETextures_startscreen.mip`
    /// at `U="0" V="15"`, and two more widgets on that same screen sample
    /// different rects of the same file (`U="0" V="0"` and `U="26" V="15"`) -
    /// without this, every one of them would draw the file's own top-left
    /// corner instead of its own patch.
    pub u: Option<f32>,
    /// The `V` attribute, likewise.
    pub v: Option<f32>,
    /// The `TxtrWidth` attribute: width of the sampled sub-rect. `None` falls
    /// back to the placed texture's own width at draw time - every `U`/`V`
    /// measured on the real discs is paired with a `TxtrWidth`/`TxtrHeight`
    /// beside it, so nothing has been observed to need a different default.
    pub texture_width: Option<f32>,
    /// The `TxtrHeight` attribute, likewise.
    pub texture_height: Option<f32>,
    /// The `AutoLoad` attribute. Recorded but not acted on: this build loads
    /// every referenced texture up front, so there is nothing yet for a
    /// load-on-demand flag to change.
    pub auto_load: bool,
    /// The wrapping `<Animation>`'s own `<Key>` timeline - empty for a widget
    /// the XML does not wrap this way. See [`RevealKey`].
    pub reveal: Vec<RevealKey>,
    /// Seconds this widget takes to fade in - see [`Text::transition`],
    /// which this mirrors. Resolved by [`fade::resolve_fade_in`]: authored
    /// `EnableTransition`/`Transition` wins, else a hidden widget
    /// ([`Self::start_enabled`] false) gets the measured class-wide
    /// default rather than `0.0` - see that function's own doc.
    pub transition: f32,
    /// Whether the widget starts hidden - see [`Text::start_enabled`].
    /// `Image` gained this for `Title Screen->TitleFrame`
    /// (`StartEnabled="false"`), which gates [`Self::transition`]'s own
    /// default - `docs/ghidra/functions/psp-pure-eu/title-screen.md`'s
    /// `Element_UpdateFade` section.
    pub start_enabled: bool,
}

/// A solid-colour `Image` widget: no `src`, so nothing to sample.
///
/// **Carries its own rect since 2026-08-25** - it used to be a bare ARGB
/// value on [`Screen::fills`], because every one measured until then was a
/// genuine full-screen backdrop (`Title Screen`'s and `Show Logo`'s own
/// white/black background, both authored at exactly `480x272`), so
/// `draw_backdrops` drawing every fill across the whole screen cost nothing.
/// That stopped being free the moment `<Animation>` started being recursed
/// into: `Demo_Definition.xml`, shared verbatim by both Pulse's and Pure's
/// disc, wraps two colour-only `Image`s at `width="347" height="1"` and
/// `width="151" height="1"` - a decorative underline, not a backdrop.
#[derive(Debug, Clone, PartialEq)]
pub struct Fill {
    /// Widget name, when it has one - `Infogradient` on the selection
    /// screens' info panel, which is how a reader finds the panel's rect.
    pub name: Option<String>,
    /// Left edge. Absent in the XML means zero, as it does for [`Image`].
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Authored width, when the XML gives one. Absent means the whole
    /// screen, since every colour-only `Image` that omits it is a
    /// full-screen backdrop on the discs measured - the same reading
    /// [`Image::width`] gives a `src` widget with none.
    pub width: Option<f32>,
    /// Authored height, likewise.
    pub height: Option<f32>,
    /// ARGB, resolved through `FEGlobals->` the same as every other colour
    /// this format authors.
    ///
    /// For a gradient widget - see [`Self::gradient`] - this is `Color1`, so
    /// a reader that knows nothing about gradients still draws the left edge's
    /// own colour rather than nothing.
    pub color: u32,
    /// The four corner colours of a `Color1`..`Color4` gradient widget, ARGB,
    /// in that order: **`Color1`/`Color2` are the left edge, `Color3`/`Color4`
    /// the right.** `None` for a plain `color` fill.
    ///
    /// The reading is off `Selection_Definition.xml`'s own rules: every
    /// horizontal rule on `Track Creation` and `Team Selection` is authored
    /// as two 85-wide halves, the left one `Color1`=`Color2`=`0x00ffffff` and
    /// `Color3`=`Color4`=`0xffffffff`, the right one the mirror image - a line
    /// that fades in from the left and back out to the right, which is what
    /// the capture shows. Which of the pair is top and which bottom is
    /// unmeasured: no authored widget gives them different values.
    pub gradient: Option<[u32; 4]>,
    /// The wrapping `<Animation>`'s own `<Key>` timeline - empty for a widget
    /// the XML does not wrap this way. See [`RevealKey`].
    pub reveal: Vec<RevealKey>,
    /// Seconds this fill takes to fade in - see [`Text::transition`].
    pub transition: f32,
}

/// A `Menu` widget: a named, selectable list, and where/how its rows draw.
///
/// The picker's `Menu` has no per-row positions of its own - `DisplayLanguages`
/// populates it with one row per language the disc offers, and every row shares
/// this one widget's `x`, `scale`, `color` and `align`, stepping down from `y`
/// by however tall a row is. Nothing in the XML states a row height; see
/// `docs/architecture/frontend-boot.md` for how that gap is closed.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    /// The `name` attribute, `"Language"` on the picker.
    pub name: String,
    /// Left edge of every row.
    pub x: f32,
    /// Top edge of the first row.
    pub y: f32,
    /// Scale multiplier, shared by every row.
    pub scale: f32,
    /// ARGB, as the XML writes it, shared by every row.
    pub color: u32,
    /// `left`, `right` or `centre`.
    pub align: String,
    /// Font name, e.g. `Default`.
    pub font: String,
}

/// One screen, flattened out of the tree.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Screen {
    /// The `name` attribute.
    pub name: String,
    /// The `type` attribute, when present. `Language Selection` has one.
    pub kind: Option<String>,
    /// Full `Parent->Child` path, matching the state machine's naming.
    pub path: String,
    /// Solid-colour `Image` widgets, each with its own rect.
    pub fills: Vec<Fill>,
    /// `Image` widgets that name a texture, in document order.
    pub images: Vec<Image>,
    /// `Movie` widgets in document order.
    ///
    /// **A `Vec`, because one screen really does declare several.** Pure's
    /// `Intro Screen` carries four - `IntroMovie1`, `FMV Movie`, `ProfileMovie1`
    /// and `ProfileMovie2` - and a single slot kept whichever came last, so the
    /// three before it were invisible. That silently broke the rule that
    /// `crate::boot` applies to a movie's sound: a widget carrying
    /// `sound="false"` could not be found and so could not mute anything.
    pub movies: Vec<Movie>,
    /// `Text` widgets in document order.
    pub texts: Vec<Text>,
    /// The box half of every `<Block>` (label in [`Self::texts`]).
    pub blocks: Vec<BlockWidget>,
    /// Every `<Mode3D>`'s `<Model>`s, in document order.
    pub models: Vec<Mode3dModel>,
    /// `Redirect` widgets in document order.
    pub redirects: Vec<Redirect>,
    /// `TouchButton` widgets in document order - Wipeout 2048's icon tiles,
    /// see [`TouchButton`]. Empty on every other title's screens.
    pub touch_buttons: Vec<TouchButton>,
    /// `Touchlist` widgets in document order - Wipeout 2048's settings and
    /// skin pickers, see [`TouchList`]. Empty on every other title's screens.
    pub touch_lists: Vec<TouchList>,
    /// `TouchSlider` widgets in document order, see [`TouchSlider`]. Empty on
    /// every other title's screens.
    pub touch_sliders: Vec<TouchSlider>,
    /// Whether the screen has a `DisplayLanguages` widget, which is what makes
    /// it the language picker.
    pub display_languages: bool,
    /// The screen's `Menu` widget, if it has one.
    pub menu: Option<Menu>,
    /// `TagInput` widgets in document order. Empty for every screen whose
    /// `TagInput` sits under an anonymous `Screen` - see [`TagInput`]'s own
    /// doc and `docs/formats/fexml.md`.
    pub tag_inputs: Vec<TagInput>,
    /// What the screen's `<ScreenClear>` fills the frame with, as ARGB.
    ///
    /// **Not [`Self::fills`]**, which is the `<Image>`-with-a-colour-and-no-`src`
    /// spelling. The two say the same kind of thing and are kept apart because
    /// only one of them is a *clear*: a `ScreenClear` is the frame the screen
    /// starts from, and every widget on the screen - and on the screens nested
    /// inside it - is drawn over it.
    ///
    /// Wipeout HD's `FE Screen` is what this exists for:
    /// `<ScreenClear><Values Colour="FEGlobals->HD_BG">`, white, and every menu
    /// screen on that disc is a descendant of it. Read here rather than in the
    /// menus so the value comes off the disc rather than out of a constant.
    pub clear: Option<u32>,
}

impl Screen {
    /// The redirect with this widget name.
    #[must_use]
    pub fn redirect_named(&self, name: &str) -> Option<&Redirect> {
        self.redirects
            .iter()
            .find(|r| r.name.as_deref() == Some(name))
    }

    /// Where pressing `button` goes, if anywhere.
    #[must_use]
    pub fn redirect_for(&self, button: Button) -> Option<&Redirect> {
        self.redirects
            .iter()
            .find(|r| r.forward == Some(button) || r.backward == Some(button))
    }
}

/// Every screen in a front-end XML file, plus its globals.
#[derive(Debug, Clone, Default)]
pub struct Screens {
    /// Screens in document order.
    pub screens: Vec<Screen>,
    /// `FEGlobals` variables, by name.
    pub globals: HashMap<String, String>,
    /// `LoadXML` sources, in the order the root lists them.
    pub load_xml: Vec<String>,
    /// The same list with each include's `SrcRel`/`localised` shape kept -
    /// what a loader that follows them needs. See [`Include`].
    pub includes: Vec<Include>,
    /// Whether a colour-only `Image`'s own `OffsetX`/`OffsetY` is folded into
    /// its [`Fill`]'s position, as an `Image` with a `src` already does.
    /// `false` by default: Pulse's `EndRace Results` authors its `tablebg{n}`
    /// rows as fills with their own `OffsetY` and `oag_ui_screens::endrace::table`
    /// adds that back by name, so folding it for them would move a picture
    /// that is measured. Wipeout 2048's `EndRace_Definition.xml` authors
    /// `MessageBox`, `ResultBox` and the rest the same way and places them
    /// by the offset alone, so it reads with [`Self::from_xml_folding_fill_offsets`].
    pub fold_fill_offsets: bool,
}

impl Screens {
    /// `screen` is the innermost named `Screen` enclosing `node`, which is
    /// what a `DirectEmbed` include's widgets belong to - see [`Include`].
    fn collect_globals(&mut self, node: &Node, screen: Option<&str>) {
        if node.name.eq_ignore_ascii_case("Variable")
            && let Some(name) = node.attr("global")
            && let Some(value) = node.value("String")
        {
            self.globals.insert(name.to_string(), value.to_string());
        }
        if node.name.eq_ignore_ascii_case("LoadXML") {
            if let Some(src) = node.value("src") {
                self.load_xml.push(src.to_string());
            }
            if let Some(include) = Include::from_node(node, screen) {
                self.includes.push(include);
            }
        }
        let screen = if node.name.eq_ignore_ascii_case("Screen") {
            node.attr("name").or(screen)
        } else {
            screen
        };
        for child in &node.children {
            self.collect_globals(child, screen);
        }
    }

    fn collect(&mut self, node: &Node, parent: Option<&str>, fallback_images: &[(&str, &str)]) {
        if !node.name.eq_ignore_ascii_case("Screen") {
            for child in &node.children {
                self.collect(child, parent, fallback_images);
            }
            return;
        }

        // Anonymous `Screen` elements are grouping containers, not navigable
        // screens. They still hold children, so they are walked through.
        let Some(name) = node.attr("name") else {
            for child in &node.children {
                self.collect(child, parent, fallback_images);
            }
            return;
        };

        let path = match parent {
            Some(p) => format!("{p}{}{name}", crate::state_machine::SEPARATOR),
            None => name.to_string(),
        };

        let mut screen = Screen {
            name: name.to_string(),
            kind: node.attr("type").map(str::to_string),
            path: path.clone(),
            ..Screen::default()
        };

        for child in &node.children {
            self.collect_widgets(&mut screen, child, None, (0.0, 0.0), 0.0, fallback_images);
        }

        self.screens.push(screen);

        for child in &node.children {
            self.collect(child, Some(&path), fallback_images);
        }
    }

    /// A screen's own widgets, one node at a time - and, unlike [`Self::collect`],
    /// recursing straight through a `Viewport` or an `Animation` rather than
    /// stopping at either.
    ///
    /// **No offset applied.** A `Viewport` carries its own `x`/`y`/`width`/
    /// `height` (see `Values` on the element itself), which on the one measured
    /// case - Pulse's `Show Logo->Viewport`, `x="40" y="0" height="480"
    /// width="400"` on a 480x272 screen - do not read as plausible screen-space
    /// clip or offset numbers at all (480 exceeds the screen's own height).
    /// Nothing here has worked out what they mean, so a nested widget keeps the
    /// coordinates it was authored with, exactly as if the `Viewport` were not
    /// there. That is a **no-op** for `Data\Plugins\PI001\GUI\Skin.xml`'s
    /// `Title Screen` on `pure-psp-usa.chd`, whose own `Viewport` is `x="0"
    /// y="0" width="480" height="272"` - the whole screen, verbatim - so this
    /// gap costs nothing on the one case this build actually draws through it.
    /// Confidence **60** on "no offset" as the right answer generally; **95**
    /// that it is a correct no-op for that specific screen.
    ///
    /// **`width` is not a no-op**, though: it is the one number a
    /// `widthlimited="true"` text inside the `Viewport` needs to wrap
    /// against, so `viewport_width` carries the nearest enclosing
    /// `Viewport`'s own `width` down for [`Self::text_from_node`] to read.
    /// `None` outside any `Viewport`. See [`Text::wrap_width`].
    ///
    /// **`Animation`'s own `Key` timeline is read and stamped onto whatever
    /// widget it wraps** - a texture-space width delta (not the `x`/`y`
    /// travel `hud.rs`'s own `<Animation><Key>` reading covers), confirmed
    /// live in `docs/ghidra/functions/psp-pure-eu/title-screen.md`. See the
    /// `"animation"` arm's own doc and [`RevealKey`]; `<Key>` has no case of
    /// its own here since that arm consumes it directly.
    ///
    /// **A colour-only `Image` inside an `Animation` is collected exactly
    /// like a top-level one.** It was not always: recursing into `Animation`
    /// used to risk turning `Demo_Definition.xml`'s two underlines into
    /// full-screen washes, back when [`Fill`] was a bare ARGB value with no
    /// rect of its own. Now that it carries one, an `Animation`-wrapped
    /// colour-only `Image` draws at its own authored position the same as any
    /// other widget - which is the point: `Title Screen`'s own frame lines are
    /// authored exactly this way.
    /// `offset` is the sum of every enclosing container's `OffsetX`/`OffsetY`,
    /// added to each widget's own `x`/`y` so the positions recorded are
    /// screen positions. The selection screens are where this matters:
    /// `Selection_Definition.xml` authors its whole info panel under
    /// `<Screen OffsetX="290" OffsetY="45">`, and each stat row under a
    /// further `<Screen OffsetY="59">`, so a widget's own `y="3"` is
    /// meaningless until the containers are summed. A container with no
    /// offset contributes zero, which is every container the boot screens
    /// use, so nothing already measured moves.
    fn collect_widgets(
        &self,
        screen: &mut Screen,
        child: &Node,
        viewport_width: Option<f32>,
        offset: (f32, f32),
        transition: f32,
        fallback_images: &[(&str, &str)],
    ) {
        let inner = (
            offset.0 + self.number(child.value("OffsetX")).unwrap_or(0.0),
            offset.1 + self.number(child.value("OffsetY")).unwrap_or(0.0),
        );
        match child.name.to_ascii_lowercase().as_str() {
            "image" => {
                match child
                    .value("src")
                    .or_else(|| {
                        // A widget whose XML gives no `src` at all is assigned one
                        // programmatically on the original - matched here by its own
                        // `name` attribute against a title's own measured table. A
                        // widget this table does not name, and that carries no
                        // `color` either, falls through to `fill_from_node` below and
                        // is dropped exactly as it always was.
                        //
                        // The table's own value takes a `FEGlobals->Name` spelling on
                        // `BackgroundImage`/`BackgroundTopRightImage`: Pure's active
                        // style skin (`Data\Skins\Default\Skin.xml`) declares both as
                        // real `<Variable global="...">`s - `BackgroundTexture` (a real
                        // string global, not a colour) resolves through `Screens::resolve`
                        // the same as `FrameLineColor` already does, rather than being a
                        // hard-coded hash. See `oag_pure::frontend::FALLBACK_IMAGES`.
                        let name = child.attr("name")?;
                        fallback_images
                            .iter()
                            .find(|(widget, _)| *widget == name)
                            .map(|(_, src)| *src)
                    })
                    // A resolved `FEGlobals->` global can itself be the empty
                    // string - `BackgroundTexture` is authored `""` on `Main
                    // Menu`'s own skin, meaning genuinely no texture there,
                    // not a lookup failure - so this is filtered the same as
                    // "no src at all" below rather than passed to a WAD lookup.
                    .and_then(|src| self.resolve(src))
                    .filter(|src| !src.is_empty())
                {
                    // A `src` names a texture; a bare colour is a [`Fill`].
                    // Positioned at `inner` (its own `OffsetX`/`OffsetY`
                    // folded in), not `offset` - the same choice `"text"`
                    // below makes and for the same reason: HD's own detail
                    // column (`Data\Plugins\Frontend\Gui\CellMode_Definition.xml`'s
                    // `<Image name="Event Emblem" OffsetX="630" OffsetY="340">`)
                    // is an `Image` used as a positioned *container* for a
                    // `Text`/`Image` group of its own, the same idiom
                    // `Item`/`LeftLayer` already carry, and an `Image` with
                    // no `OffsetX`/`OffsetY` of its own (every one measured
                    // before this) resolves `inner == offset`, so this is a
                    // no-op everywhere that shape is absent.
                    Some(src) => {
                        let mut image = self.image_from_node(child, src, inner);
                        // The widget's own `EnableTransition`/`Transition`,
                        // read directly off this node rather than only
                        // inherited - Pulse's `GameShareBackdrop` authors
                        // its own literal `transition="0"`. See
                        // [`fade::resolve_fade_in`].
                        let own = self
                            .number(child.value("EnableTransition"))
                            .or_else(|| self.number(child.value("Transition")));
                        image.transition =
                            fade::resolve_fade_in(own, transition, image.start_enabled);
                        screen.images.push(image);
                    }
                    None => {
                        let at = if self.fold_fill_offsets {
                            inner
                        } else {
                            offset
                        };
                        if let Some(mut fill) = self.fill_from_node(child, at) {
                            fill.transition = transition;
                            screen.fills.push(fill);
                        }
                    }
                }
                // An `Image` can hold widgets of its own - HD's detail
                // column nests a `Text` label, a bullet `Image` and a
                // `Text` value under each of its four `*Emblem` images, and
                // `CellMode_Definition.xml`'s own `unlockbox`/`flyerlogo`
                // nest a `Text`/`Image` pair the same way. Before this arm
                // recursed, every one of those was silently dropped - the
                // same gap `"text"`'s own doc records having had for its
                // child widgets, now closed for `Image` too.
                for grandchild in &child.children {
                    self.collect_widgets(
                        screen,
                        grandchild,
                        viewport_width,
                        inner,
                        transition,
                        fallback_images,
                    );
                }
            }
            "screenclear" => {
                // `Colour`, the British spelling, which is what every
                // `ScreenClear` on Wipeout HD's disc uses - and `color` beside
                // it because the same file spells the `Image` attribute the
                // other way, so which one an element takes is not a thing to
                // remember.
                if let Some(value) = child.value("Colour").or_else(|| child.value("color"))
                    && let Some(argb) = self.resolve(value).and_then(parse_argb)
                {
                    screen.clear = Some(argb);
                }
            }
            "movie" => screen.movies.push(Movie::from_node(child)),
            "text" => {
                // Positioned at `inner`, not `offset` - HD's own
                // `Data\Plugins\Frontend\Gui\CellMode_Definition.xml`
                // authors a handful of labels (`Medals Title`/`Points
                // Title`/`EPoints Title`) with their own `OffsetX`/`OffsetY`
                // directly on the `<Text>` tag and no `x`/`y`, no wrapping
                // `<Item>` - every *other* positioned label on either of its
                // two screens sits inside one instead. `inner` already
                // folds a `Text`'s own `OffsetX`/`OffsetY` into what its
                // children see; using it for the text's own position too is
                // a no-op wherever that pair is absent (`inner == offset`
                // then, since `self.number(None).unwrap_or(0.0)` is `0.0`),
                // which is every widget measured before this file.
                let mut text = self.text_from_node(child, viewport_width, inner);
                text.transition = transition;
                screen.texts.push(text);
                // A `Text` can hold widgets of its own: `Team Selection`'s
                // `skin` label carries its two livery arrows as child
                // `Image`s. Its `Values` child is its own attributes, not a
                // widget, and `collect_widgets` has no arm for it anyway.
                for grandchild in &child.children {
                    self.collect_widgets(
                        screen,
                        grandchild,
                        viewport_width,
                        inner,
                        transition,
                        fallback_images,
                    );
                }
            }
            // A `Block` is HD's own menu/table box - `Block_Item.cpp`
            // (`docs/ghidra/functions/ps3-hdfury-eu/menu-blocks.md`), a
            // bordered, filled rect the engine draws off a shared nine-patch
            // and colours/positions per widget. The box half lands in
            // [`Screen::blocks`] ([`BlockWidget`], for a screen that draws
            // it through `crate::menu::block`); the label half is
            // collected as a [`Text`] the
            // same idstring-resolution pass every other text already gets
            // (`oag_ui_screens::campaign::Layout::read_authored`), so a caller reading
            // `screen.texts` finds `GridHead1`'s `IG_HUD_POS` or `EndRace
            // Menu`'s own `race_again` beside every other label rather than
            // needing a third widget vocabulary. Confirmed to appear nowhere
            // on `CellMode_Definition.xml`'s own two screens (`grep -c
            // "<Block" desktop-read, zero), so this is additive: no screen
            // this build already draws gains a new widget from it.
            "block" => {
                let mut text = self.block_from_node(child, viewport_width, inner);
                text.transition = transition;
                screen.texts.push(text);
                screen
                    .blocks
                    .push(self.block_widget_from_node(child, inner));
            }
            "mode3d" => screen.models.extend(self.mode3d_models_from_node(child)),
            "redirect" => screen.redirects.push(widgets::redirect_from_node(child)),
            "touchbutton" => screen
                .touch_buttons
                .push(self.touch_button_from_node(child, inner)),
            "touchlist" => screen
                .touch_lists
                .push(self.touch_list_from_node(child, inner)),
            "touchslider" => screen
                .touch_sliders
                .push(self.touch_slider_from_node(child, inner)),
            "displaylanguages" => screen.display_languages = true,
            "menu" => screen.menu = Some(self.menu_from_node(child, offset)),
            "taginput" => screen.tag_inputs.push(self.tag_input_from_node(child)),
            "viewport" => {
                let width = self.number(child.value("width"));
                for grandchild in &child.children {
                    self.collect_widgets(
                        screen,
                        grandchild,
                        width,
                        inner,
                        transition,
                        fallback_images,
                    );
                }
            }
            // None carries a `width` of its own to pass down - whatever the
            // enclosing `Viewport` gave keeps applying inside. `BackgroundController`
            // is Pure's own case: its `Skin.xml` wraps `FE Screen`'s background art
            // in one (`<BackgroundController><Image name="BackgroundImage">...`),
            // and before this arm existed nothing walked through it either - the
            // same silent drop `Screen` below has, just one container earlier.
            // `Item` is the same idiom again - the selection screens' info
            // panel groups every stat row as `<Item OffsetY="59">` - and
            // carries the offsets that make its children's positions mean
            // anything.
            // `GridController` is the Race Campaign's own container -
            // `CellMode_Definition.xml`'s `Grid Selection`/`Cell Selection`
            // hex layouts, each `Medal_x_y`/`Outline_x_y`/`Lock_x_y` nested
            // under it exactly the way an ordinary info panel nests under
            // `Item`. **A `startenabled="false"` controller is skipped
            // outright, not just left unfocused**: `Grid Selection` authors
            // two, `Grid` and `Grid1`, both naming `Medal_0_0`..`Medal_3_0` -
            // the same widget names - so collecting both would silently
            // overwrite one tier's hexes with the other's in `screen.images`,
            // a flat `Vec` with no notion of which controller a name came
            // from. See `docs/ui/campaign-screens.md`.
            "gridcontroller" if child.attr("startenabled") != Some("false") => {
                for grandchild in &child.children {
                    self.collect_widgets(
                        screen,
                        grandchild,
                        viewport_width,
                        inner,
                        transition,
                        fallback_images,
                    );
                }
            }
            // `NavigationController` is Wipeout HD/Fury's own per-screen
            // `Confirm`/`Back` prompt group - `EndRace_Definition.xml`
            // authors one nested directly inside `EndRace Results`/`EndRace
            // Menu`/`EndRace Rewards`, each with its own `Text` children
            // (`ControlTextConfirmButton`/`ControlTextConfirm`, and on
            // `Results` two more for an online-only "cycle records" button
            // this build already excludes by name - see
            // `oag_ui_screens::endrace::hd::hd_results_draw_list`'s own doc). Before
            // this arm existed the whole container was silently dropped, the
            // same gap `docs/formats/hd-endrace-screens.md`'s own widget
            // table names for all three screens - "that container is not
            // walked". Walking it here, straight through to `screen.texts`
            // like `"text"` above, is additive for every screen this build
            // actually draws in play: confirmed by direct read that no
            // *navigable* screen on either title's own disc
            // (`Data\Plugins\PI001\GUI\*.xml` on Pulse,
            // `Data\Plugins\Frontend\Gui\*.xml` on Wipeout HD/Fury, outside
            // `EndRace_Definition.xml` itself) authors this tag.
            //
            // **One inert exception, worth naming rather than hiding.**
            // Pulse's own shared `Skin.xml` nests its single
            // `NavigationController` inside `FE Screen` itself (an anonymous
            // wrapper `<Screen>` in between, which this same match arm
            // already walks through) - so `FE Screen`'s own `screen.texts`
            // now carries these four widgets too, alongside `Frame::read`'s
            // `<Image>`/`<ScreenClear>` reading, which this arm does not
            // touch. That is harmless in play: `FE_SCREEN` is used only as
            // `oag_title::FrontEnd::menu_frame` (`crates/pulse/src/lib.rs`,
            // `crates/hd/src/frontend.rs`), which never reaches
            // `screen.texts` at all (see `oag_ui::menu::frame`'s own doc for
            // why), and the boot-chain state machine
            // (`crate::frontend::Frontend`) never names `"FE Screen"` as a
            // step, so `Frontend::draw_screen_at` - the one function that
            // does draw a named screen's own `.texts` generically - is never
            // called with it during a live boot. It is reachable only
            // through the `--screen "FE Screen"` debug path, where drawing
            // widgets `FE Screen` genuinely authors is not a wrong picture.
            // The one shared, title-wide copy this build actually *draws*
            // from - Pulse's and Wipeout HD/Fury's own `Skin.xml`, via
            // `oag_ui_screens::campaign::footer` - is read directly off the raw
            // parse tree instead, unconditionally rather than gated on this
            // arm, so nothing here duplicates that read.
            // `LeftLayer` is also the one container that authors a
            // `transition`: how many seconds its own children fade in over
            // when the screen first shows them, read here and inherited by
            // every widget nested under it - `Text::transition`'s own doc has
            // the mechanism and the capture it is read against. A `LeftLayer`
            // with no `transition` of its own inherits whatever enclosed it,
            // the same fallback the executable's own attribute reader uses
            // (`FUN_088920fc`, `docs/ghidra/functions/psp-pulse-usa/
            // race-box-screens.md`) - `EnableTransition`/`DisableTransition`
            // exist there too, as per-direction overrides, but neither is
            // authored on either selection screen, so only the plain
            // `transition` name is read here.
            "leftlayer" => {
                let transition = self.number(child.value("transition")).unwrap_or(transition);
                for grandchild in &child.children {
                    self.collect_widgets(
                        screen,
                        grandchild,
                        viewport_width,
                        inner,
                        transition,
                        fallback_images,
                    );
                }
            }
            "navigationcontroller" | "backgroundcontroller" | "item" => {
                for grandchild in &child.children {
                    self.collect_widgets(
                        screen,
                        grandchild,
                        viewport_width,
                        inner,
                        transition,
                        fallback_images,
                    );
                }
            }
            // `<Animation>` wraps exactly one real widget - its own `<Key>`
            // children are collected here and stamped onto whatever that
            // widget turns out to be. See [`RevealKey`].
            "animation" => {
                let reveal: Vec<RevealKey> = child
                    .children
                    .iter()
                    .filter(|key| key.name.eq_ignore_ascii_case("key"))
                    .filter_map(|key| {
                        Some(RevealKey {
                            time: self.number(key.value("Time"))?,
                            texture_width: self.number(key.value("TextureWidth")).unwrap_or(0.0),
                        })
                    })
                    .collect();
                let (images_from, fills_from) = (screen.images.len(), screen.fills.len());
                for grandchild in &child.children {
                    if grandchild.name.eq_ignore_ascii_case("key") {
                        continue;
                    }
                    self.collect_widgets(
                        screen,
                        grandchild,
                        viewport_width,
                        inner,
                        transition,
                        fallback_images,
                    );
                }
                if !reveal.is_empty() {
                    for image in &mut screen.images[images_from..] {
                        image.reveal = reveal.clone();
                    }
                    for fill in &mut screen.fills[fills_from..] {
                        fill.reveal = reveal.clone();
                    }
                }
            }
            // An anonymous `Screen` is a grouping container, not a navigable
            // screen - [`Self::collect`] already says so and walks straight
            // through one to find the named screens nested inside. This is
            // that same rule applied to widgets: an anonymous `Screen`'s own
            // `Image`/`Text`/etc. children belong to whichever named screen
            // encloses it, exactly as if the wrapper were not there. Before
            // this arm existed they were silently dropped - neither an error
            // nor a picture, just gone - because nothing else in this match
            // recurses into a `Screen` tag. Pulse's own top bar is the case
            // that found it: its `<Image>` sits three anonymous `Screen`
            // levels under `Top FE Screen->FE Screen`. A *named* child is
            // left alone here - it collects its own widgets separately, the
            // next time [`Self::collect`] reaches it, and counting them twice
            // would draw a menu's own chrome on its parent's frame too.
            //
            // **A named `Screen` that carries an `OffsetX`/`OffsetY` is a
            // positioned group, not a destination**, and is walked through
            // the same way: `Track Creation`'s three stat rules are
            // `<Screen name="line bg1" OffsetY="142">` and so on, each
            // holding two gradient halves and nothing a state machine could
            // land on. [`Self::collect`] still lists it as a screen of its
            // own, unchanged; what this adds is its widgets on the screen
            // that encloses it, at the offset it authors.
            "screen" if child.attr("name").is_none() || inner != offset => {
                for grandchild in &child.children {
                    self.collect_widgets(
                        screen,
                        grandchild,
                        viewport_width,
                        inner,
                        transition,
                        fallback_images,
                    );
                }
            }
            _ => {}
        }
    }

    /// Follows a `FEGlobals->Name` indirection, or returns the value as-is.
    #[must_use]
    pub fn resolve<'a>(&'a self, value: &'a str) -> Option<&'a str> {
        if value.is_empty() {
            return None;
        }
        match value.strip_prefix("FEGlobals->") {
            // The real XML has newlines inside these values, from being written
            // across two lines in the source file.
            Some(key) => self.globals.get(key.trim()).map(String::as_str),
            None => Some(value),
        }
    }

    /// `pub(crate)` for `oag_ui_screens::tag_entry` - see
    /// [`Screens::fill_from_node`]'s own note.
    pub fn number(&self, value: Option<&str>) -> Option<f32> {
        self.resolve(value?)?.trim().parse().ok()
    }

    /// A screen by its `name`.
    #[must_use]
    pub fn by_name(&self, name: &str) -> Option<&Screen> {
        self.screens.iter().find(|s| s.name == name)
    }

    /// The screen whose `type` is `Language Selection`, or failing that the one
    /// with a `DisplayLanguages` widget.
    ///
    /// Two ways of finding the same screen, because either alone would be a
    /// guess and agreeing makes it a determination.
    #[must_use]
    pub fn language_selection(&self) -> Option<&Screen> {
        self.screens
            .iter()
            .find(|s| s.kind.as_deref() == Some("Language Selection"))
            .or_else(|| self.screens.iter().find(|s| s.display_languages))
    }

    /// Every screen that plays a movie, in document order.
    pub fn with_movies(&self) -> impl Iterator<Item = &Screen> {
        self.screens.iter().filter(|s| !s.movies.is_empty())
    }
}

#[cfg(test)]
mod tests;
