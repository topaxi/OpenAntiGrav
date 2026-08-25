//! What the HUD needs off the disc, and which texture it samples.
//!
//! Split out of [`super`] so the layout model and the draw list stay free of
//! the sprite sheet, the font atlases and the string table - none of which a
//! headless test of either needs.

use super::Layout;

/// Everything the HUD needs off the disc.
///
/// Grouped rather than spread across [`crate::race::Loaded`] because it is a
/// unit: without the layout none of the rest is usable, and the whole thing
/// degrades together.
#[derive(Debug)]
pub struct Assets {
    /// The mode's parsed layout, or `None` when it could not be read.
    ///
    /// `None` means **no HUD is drawn at all**, and that is the honest outcome:
    /// the geometry is the disc's and there is nothing of ours to stand in for it.
    /// A HUD invented on the spot would be a worse failure than none, because it
    /// would look like a working HUD in the wrong place.
    pub layout: Option<Layout>,
    /// **Every** texture the layout names, packed into one sheet, keyed by the
    /// reference the layout spells rather than by the entry that was read.
    ///
    /// One entry for Pulse, up to six for HD - see [`Layout::atlas`] for the
    /// measurement and for what assuming one cost. Empty for a layout that
    /// names none: Pure's HUDs carry no `<Image>` at all.
    ///
    /// Keyed by the *reference* so [`super::sprite_draw`] can look a sprite's
    /// own `src` up directly. Which entry a reference resolves to is the
    /// loader's business and `oag_title::HudArt::texture_extension`'s rule.
    pub sheet: crate::sprite::Sheet,
    /// The `HUD` font, or the built-in 5x7 set when the disc's is unreadable.
    pub font: crate::font::Atlas,
    /// The `HUDSmall` font, likewise.
    pub small_font: crate::font::Atlas,
    /// The language's string table, for the `IG_HUD_*` captions.
    pub strings: crate::language::StringTable,
    /// The grid this layout's coordinates are in, and what it is shown as.
    ///
    /// **The source's, not the PSP's.** A HUD layout carries bare numbers and
    /// says nothing about the space they are in, so the renderer has to be told:
    /// Pulse and Pure author 480x272, the PS2 pressing 640x448, and HD 1920x1080
    /// ([hd-frontend.md](../../../docs/formats/hd-frontend.md#the-coordinate-space-is-1920x1080),
    /// confidence 90). Left at the default, HD's lap counter drew four times
    /// oversized in the middle of the picture instead of in its panel at the
    /// top-left corner - a yellow slab that reads as scenery rather than as a
    /// digit. See [`crate::frontend::Space`].
    pub space: crate::frontend::Space,
    /// How this title's HUD sprites reach the screen: which of them are up
    /// whenever the HUD is, and the one colour this build substitutes.
    ///
    /// Off [`oag_title::Title::hud_art`] rather than a `const` in this module,
    /// because all three of its rows are per-title measurements that disagree -
    /// see [`oag_title::HudArt`]. The texture row is spent by the time this
    /// exists: [`Self::sheet`] is already built.
    pub art: &'static oag_title::HudArt,
}

impl Assets {
    /// The context [`super::draw_list`] takes, or `None` with no layout to draw.
    ///
    /// The same seven fields [`super::Overlay::context`] assembles, off a
    /// loaded [`Assets`] rather than off a built renderer - so a test can ask
    /// what a real race's HUD draws without a GPU, which is the whole reason
    /// the draw list is separated from the pass in the first place.
    #[must_use]
    pub fn context(&self) -> Option<super::Context<'_>> {
        let layout = self.layout.as_ref()?;
        Some(super::Context {
            default_border: layout.default_border(),
            layout,
            strings: &self.strings,
            sheet: &self.sheet,
            art: self.art,
            hud_line_height: self.font.line_height,
            small_line_height: self.small_font.line_height,
        })
    }
}

impl Layout {
    /// The texture this layout's sprites sample, as the layout itself names it.
    ///
    /// **Read off the file rather than named by this build**, which is
    /// `CLAUDE.md`'s "never invent what the assets already author" rule applied
    /// to one more constant: `PulseHUD.mip` appears ten times inside
    /// `TimeTrial_HUD.xml` and once in this repository, and the one in this
    /// repository was the only copy that could be wrong. It was: Pure's four HUD
    /// layouts name no `.mip` at all - their art is `<Model>` geometry off
    /// `Data\HUD\*.vex` - so a Pure race went looking for a Pulse texture.
    ///
    /// `None` for a layout whose sprites name no texture, which covers both that
    /// case and Pulse's own `MPTag_HUD.xml`.
    ///
    /// **The first of however many**, and no longer the one a frame samples.
    /// Every one of the nine layouts shipped across the two PSP titles names at
    /// most one distinct texture, checked 2026-08-12 and still pinned by
    /// `the_layouts_name_at_most_one_texture` in
    /// `crates/game/tests/hud_layout_ground_truth.rs`; HD's eighteen composed
    /// layouts name **twelve between them, up to six in one**. So this is a
    /// convenience for a caller that wants the layout's own first reference -
    /// a report line, a test - and nothing on the draw path reads it.
    ///
    /// It used to be the draw path. Until 2026-08-25 the sheet held this
    /// texture alone and every sprite was offset by its placement, so on HD the
    /// 45 arcade sprites naming one of the other five sampled `HUD_Components`
    /// at their own texture's coordinates: the pickup icon came out as a grey
    /// box, in the right place, at the right size. See [`Assets::sheet`] and
    /// [`textures`], which is what replaced it.
    ///
    /// [`textures`]: Layout::textures
    #[must_use]
    pub fn atlas(&self) -> Option<&str> {
        self.sprites
            .iter()
            .map(|sprite| sprite.src.as_str())
            .find(|src| !src.is_empty())
    }

    /// Every distinct texture this layout's sprites name, in the order they are
    /// first named.
    ///
    /// The order is the layouts' own document order, so a sheet built from it
    /// is byte-identical from one run to the next - which is what makes a
    /// screenshot comparable. See [`crate::sprite::Sheet`].
    #[must_use]
    pub fn textures(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for src in self
            .sprites
            .iter()
            .map(|sprite| sprite.src.as_str())
            .filter(|src| !src.is_empty())
        {
            if !out.contains(&src) {
                out.push(src);
            }
        }
        out
    }
}
