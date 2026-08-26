//! The widget model: the four things a layout is made of, and the two enums
//! that qualify them.
//!
//! Data only. Nothing here decides what is drawn, which is
//! [`super::draw_list`]'s job, and nothing here reads a file, which is
//! [`super::Layout::from_tree`]'s. Split out of [`super`] so the model, the
//! parser and the draw list are three files rather than one.

use crate::frontend::Align;

/// Which of the disc's fonts a widget draws in.
///
/// The role names are the language plugin's, and the mapping to `.fnt` files is
/// in [`crate::frontend`]'s line-height table and
/// [`oag_pulse::names::fonts`]: `HUD` is `PulseHud.fnt` at 25 px,
/// `HUDSmall` is `small.fnt` at 10 px, `Default` is `pulse_text.fnt` at 13 px.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Font {
    /// `font="HUD"`. Every value the HUD displays - speed, lap, times.
    #[default]
    Hud,
    /// `font="HUDSmall"`. Every label beside those values.
    Small,
    /// `font="Default"`.
    ///
    /// Used by exactly eight widgets across all five layouts, `PosTag0` to
    /// `PosTag7` - the floating opponent name tags. Nothing draws those yet,
    /// since a race has one ship, but the variant exists so the parser does not
    /// silently fold them into a font they are not.
    Default,
}

impl Font {
    pub(super) fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "hudsmall" => Self::Small,
            "default" => Self::Default,
            _ => Self::Hud,
        }
    }
}

/// Vertical alignment of a text widget about its `y`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VertAlign {
    /// No `vertalign`: `y` is the top edge, which is what
    /// [`crate::render::Renderer`] draws text at natively.
    #[default]
    Top,
    /// `vertalign="middle"` or `vertalign="centre"`.
    ///
    /// **Both spellings ship**, 37 and 16 times respectively, and they mean the
    /// same thing - unlike `align`, where `centre` is the only spelling used.
    Middle,
    /// `vertalign="bottom"`, the commonest at 41 uses: `y` is the baseline-ish
    /// bottom edge, so a value grows upward from a fixed line.
    Bottom,
}

impl VertAlign {
    pub(super) fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "middle" | "centre" | "center" => Self::Middle,
            "bottom" => Self::Bottom,
            _ => Self::Top,
        }
    }
}

/// One `<Image>`: a rectangle cut out of the layout's own atlas, drawn somewhere.
///
/// # This carries the atlas's *name* and not a handle, and that is the next
/// thing to change
///
/// [`Self::src`] is a string, and every consumer resolves it the same way:
/// through [`Layout::atlas`], which returns **the** texture a layout samples.
/// That is true of Pulse's five layouts and Pure's four - measured, and pinned
/// by `the_layouts_name_at_most_one_texture` - and false of HD/Fury, which names
/// **twelve across eighteen layouts, up to six in one**. See
/// `docs/formats/hd-hud.md`.
///
/// So today an HD sprite's `src` is read, checked against the real texture's
/// dimensions by `crates/game/tests/hd_hud_ground_truth.rs`, and then thrown
/// away at draw time, because [`Assets::atlas_origin`] and [`sprite_draw`] both
/// take **one** origin for the whole frame. Five of the six textures a layout
/// names cannot be reached.
///
/// **What that needs is a field here**, not a change in the renderer: a sprite
/// has to carry which sheet entry it samples - an index into
/// [`crate::sprite::Sheet`], resolved once at load - so `sprite_draw` can offset
/// by that entry's placement rather than by a single per-frame origin. The
/// packer and the shader already handle several images in one sheet; nothing
/// about them is in the way. `oag_render` also has no `.gtf` upload path, which
/// is the second problem and not the first: [`oag_formats::gtf`] decodes all
/// twelve as of 2026-08-17.
///
/// Deliberately **not** done speculatively. A `texture: usize` with one caller
/// and no second atlas to point it at is a field that means nothing, and the
/// change is worth making against a real HD draw path rather than ahead of one.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    /// The `name` attribute, which is how [`draw_list`] finds a widget.
    pub name: String,
    /// Destination in screen space, `[x, y, width, height]`, absolute - the
    /// enclosing `<Item>`'s offset and any `Centred="true"` are already applied.
    pub rect: [f32; 4],
    /// Source rectangle in atlas pixels, `[U, V, TxtrWidth, TxtrHeight]`.
    ///
    /// Separate from [`Self::rect`] because they genuinely differ: `TimeDiffIcon`
    /// samples a 28x23 patch and draws it at 14x12, so the sprite is
    /// half-size rather than 1:1.
    pub uv: [f32; 4],
    /// Modulating colour, already resolved through the constant table.
    pub color: [f32; 4],
    /// The `Src` entry name. See [`Layout::atlas`], which is what resolves it.
    pub src: String,
}

/// One `<Text>`: a string, a font, and where it goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// The `name` attribute.
    pub name: String,
    /// Anchor x in screen space, absolute; what it anchors depends on
    /// [`Self::align`].
    pub x: f32,
    /// Anchor y in screen space, absolute; what it anchors depends on
    /// [`Self::vertalign`].
    pub y: f32,
    /// Which disc font.
    pub font: Font,
    /// Scale multiplier on the font's own cell. Ranges 0.5 to 1.5 in the shipped
    /// layouts.
    pub scale: f32,
    /// Text colour, resolved.
    pub color: [f32; 4],
    /// The `BorderColor`, when the widget has one.
    ///
    /// An outline colour, and 54 widgets carry one. **Not drawn yet** - the
    /// renderer has no outline pass, and faking one with four offset copies
    /// would quadruple the glyph count for something nobody has compared against
    /// the original. Parsed rather than dropped because the attribute is real.
    pub border: Option<[f32; 4]>,
    /// Horizontal alignment about [`Self::x`].
    pub align: Align,
    /// Vertical alignment about [`Self::y`].
    pub vertalign: VertAlign,
    /// A localisation key, resolved against the language plugin's string table.
    pub idstring: Option<String>,
    /// A literal string, used instead of `idstring` by 11 widgets - `"0 kmh"`,
    /// `"+0"`, `"/"`.
    pub string: Option<String>,
}

/// One `<Image>` with **no** `Src`: a solid rectangle rather than a cut-out.
///
/// The same convention the front end's screens use, where
/// [`crate::screen::Screen::fills`] holds the colour-only kind. Exactly one HUD
/// widget is of this kind, `HeadToHeadBar` in the arcade and eliminator layouts,
/// and it is authored `width="5" height="0"` - a zero-height bar, so its length
/// is supplied at runtime and the authored height is a floor rather than a size.
/// That is recorded rather than acted on: nothing here drives it.
#[derive(Debug, Clone, PartialEq)]
pub struct Fill {
    /// The `name` attribute.
    pub name: String,
    /// Destination in screen space, `[x, y, width, height]`, absolute.
    pub rect: [f32; 4],
    /// Colour, resolved.
    pub color: [f32; 4],
}

/// One `<Mode3D><Model>`: a `.vex` model drawn in the 3D overlay layer.
///
/// Recorded here, and drawn by whoever wants one. The countdown
/// (`Pulse_Ready_Go`, `Cockpit_321GO`) still needs a second pass with a
/// projection of its own; the weapon sights do not, and are drawn. See
/// `docs/ui/hud.md`.
///
/// **The sights are nine widgets over three models.** The three model names this
/// comment used to list are right; what it did not say is that four widgets
/// instance each of two of them. `Arcade_HUD.xml` authors `missile_sight_1` …
/// `missile_sight_4` all off `missile_sight_outer.vex`, `missile_sight_inner`
/// off its own, and `leachbeam_sight_1` … `leachbeam_sight_4` off
/// `leachbeam_sight.vex` - nine widgets, all at the same placeholder position,
/// which the runtime overwrites every frame. Four brackets around a centre is a
/// lock-on reticle anchored on the *locked craft*. Drawn as of 2026-08-26; see
/// `docs/ghidra/functions/psp-pulse-usa/lock-sight.md` for the placement law.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// The `name` attribute.
    pub name: String,
    /// The `.vex` entry name.
    pub src: String,
    /// Position in the 3D overlay's own space.
    pub position: [f32; 3],
}
