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
    /// The layout's own atlas, packed into a sheet with its placement known.
    ///
    /// Empty for a layout that names none - Pure's HUDs are `<Model>` geometry
    /// and name no texture at all. See [`Layout::atlas`].
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
}

impl Assets {
    /// Where this layout's own atlas sits in [`Self::sheet`], in sheet pixels.
    ///
    /// `(0, 0)` when there is no layout, when the layout names no texture, or
    /// when the named one is not in the sheet. All three pair with nothing being
    /// drawn, so the value never reaches a shader - see [`Layout::atlas`].
    #[must_use]
    pub fn atlas_origin(&self) -> (f32, f32) {
        self.layout
            .as_ref()
            .and_then(Layout::atlas)
            .and_then(|atlas| self.sheet.get(atlas))
            .map_or((0.0, 0.0), |placed| (placed.x as f32, placed.y as f32))
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
    /// **One name rather than a set**, and that is measured rather than assumed:
    /// every one of the nine layouts shipped across the two titles names at most
    /// one distinct `.mip`, checked 2026-08-12. `the_layouts_name_at_most_one_texture`
    /// in `crates/game/tests/hud_layout_ground_truth.rs` fails the day one does
    /// not, rather than silently drawing eight sprites from the wrong sheet.
    ///
    /// # And it is false for HD/Fury
    ///
    /// That invariant is Pulse's and Pure's, not the dialect's. HD's eighteen
    /// composed layouts name **twelve** distinct textures between them, **up to
    /// six in one layout** - see `docs/formats/hd-hud.md`. So this returns the
    /// first of six there, and everything downstream that assumes one sheet -
    /// [`Assets::atlas_origin`], the single `atlas_origin` [`sprite_draw`]
    /// takes, [`Overlay`] - assumes it wrongly.
    ///
    /// **That is the blocker on drawing an HD HUD**, not the missing `.gtf`
    /// upload: `oag_formats::gtf` decodes all twelve as of 2026-08-17. What is
    /// needed is a sprite carrying *which* texture it samples through to the
    /// draw call. Nothing asserts anything about HD's count today, because the
    /// test above walks Pulse's five.
    #[must_use]
    pub fn atlas(&self) -> Option<&str> {
        self.sprites
            .iter()
            .map(|sprite| sprite.src.as_str())
            .find(|src| !src.is_empty())
    }
}
