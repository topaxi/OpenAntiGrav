//! The second glyph texture `Draw::FacedText` samples, split out of
//! [`super`] under the 1,000-line rule in `scripts/check-file-size.py` - a
//! new seam rather than a moved one, since nothing here existed before this
//! title font landed.
//!
//! # Why a second texture rather than a second draw call
//!
//! A menu screen can need both the body face and a named role's face - HD's
//! bold chrome title over an otherwise `Default`-drawn frame - in the same
//! frame. [`super::Renderer`] already binds a second texture alongside the
//! glyph atlas for exactly this shape, the sprite sheet, sampled by
//! [`super::quad::MODE_SPRITE`] without ever splitting the pass into two draw
//! calls; the face atlas is a third texture in the same bind group, selected
//! by [`super::quad::MODE_FACE_ATLAS`] the same way.

use super::resources::upload_rg8;
use oag_ui::font::Atlas;

/// Uploads `atlas`'s two planes as the face texture, or a 1x1 transparent
/// placeholder when `atlas` is `None` - a title whose chrome names no role,
/// or whose role failed to load. Nothing samples the placeholder: a
/// `Draw::FacedText` only ever reaches [`super::Renderer::push_text`]'s
/// `face` path when [`super::Renderer::face_atlas`] is `Some`, so a 1x1
/// blank is never asked to stand in for real glyphs.
pub(super) fn upload_face(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    atlas: Option<&Atlas>,
) -> wgpu::TextureView {
    let texture = match atlas {
        Some(atlas) => upload_rg8(
            device,
            queue,
            "face atlas",
            atlas.width,
            atlas.height,
            &atlas.luma,
            &atlas.coverage,
        ),
        None => upload_rg8(device, queue, "face atlas (placeholder)", 1, 1, &[0], &[0]),
    };
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// A sampler for the face texture. Always linear: every role this build
/// has ever loaded through it - HD's `helvb.fnt`, Pulse's own
/// `pulse_text.fnt` - is a real antialiased font like every other menu
/// face, and the placeholder is never sampled - see [`upload_face`]. Mirrors
/// `sprite_sampler`, which is unconditionally linear for the same reason.
pub(super) fn face_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("face"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}

/// The face atlas's size, in pixels, for `Uniforms::face_atlas` -
/// `(1.0, 1.0)` for the placeholder, which no quad's `uv` ever divides by
/// since none samples it.
pub(super) fn face_atlas_size(atlas: Option<&Atlas>) -> [f32; 2] {
    atlas.map_or([1.0, 1.0], |atlas| {
        [atlas.width as f32, atlas.height as f32]
    })
}

impl super::Renderer {
    /// Loads (or clears) the face atlas `Draw::FacedText` samples, for a
    /// title whose screen title draws in a named role - or, since
    /// 2026-09-21, for Pulse's own `Default`-role body face, loaded beside
    /// its unchanged `menu`-role primary so the two roles' widgets can draw
    /// in their own, disc-measured faces at once. See `docs/ui/menus-original.md`'s
    /// "Two faces, not one swapped for the other" section for why the
    /// primary atlas itself never changes for this.
    ///
    /// `role` is the name a `Draw::FacedText::role` has to match
    /// (case-insensitively, [`super::Renderer::push_text`]'s `atlas_for`) to
    /// draw from this slot rather than falling back to the primary atlas -
    /// `None` alongside `atlas: None` for a title that names none.
    ///
    /// Mirrors [`super::Renderer::set_sprites`]'s own shape: a fresh texture
    /// and view, the bind-group layout untouched, one bind group rebuilt
    /// around it. `None` restores the placeholder [`super::Renderer::new`]
    /// built, which is what a title with no [`oag_title::MenuSkin::title_font`]
    /// and no `Default`-role atlas of its own (or one whose role failed to
    /// read) keeps throughout.
    pub fn set_face_atlas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        atlas: Option<Atlas>,
        role: Option<&'static str>,
    ) {
        self.face_view = upload_face(device, queue, atlas.as_ref());
        self.ui_bind_group = super::resources::ui_bind_group(
            device,
            &self.ui_layout,
            &self.uniform_buffer,
            &self.atlas_view,
            &self.atlas_sampler,
            &self.sprite_view,
            &self.sprite_sampler,
            &self.face_view,
            &self.face_sampler,
            &self.buttons_view,
        );
        self.face_atlas = atlas;
        self.face_role = role;
    }

    /// Loads (or clears) the *third* glyph texture `Draw::FacedText { role:
    /// "Buttons", .. }` samples - Wipeout HD/Fury's own
    /// `ps_buttons.fnt`/`PS_BUTTONS.fnt` (Omega very likely, unverified this
    /// pass - see `crate::language::roles::BUTTONS`'s own doc), resolved
    /// through
    /// `crate::language::roles::BUTTONS`
    /// (`oag_game::boot::fonts::load_buttons_font`) the same way
    /// [`Self::set_face_atlas`]'s own atlas is, but into a slot of its own:
    /// HD's screen title (`Title` role, [`Self::set_face_atlas`]'s own slot)
    /// and this screen's footer button glyphs are on screen in the same
    /// frame, so they cannot share one.
    ///
    /// **The slot answers to `role`, `Buttons` unless a title's body face
    /// holds [`Self::set_face_atlas`]'s slot** - Pulse's PSP pressings, which
    /// have no button font and put their `Title` face here instead
    /// (`oag_title::MenuSkin::body_font`). A `Buttons` draw never falls back
    /// to the primary atlas the way [`Self::face_atlas`]'s roles do; see
    /// `super::text::GlyphSlot::Buttons`'s own doc for why not.
    pub fn set_buttons_atlas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        atlas: Option<Atlas>,
        role: &'static str,
    ) {
        self.buttons_view = upload_face(device, queue, atlas.as_ref());
        self.ui_bind_group = super::resources::ui_bind_group(
            device,
            &self.ui_layout,
            &self.uniform_buffer,
            &self.atlas_view,
            &self.atlas_sampler,
            &self.sprite_view,
            &self.sprite_sampler,
            &self.face_view,
            &self.face_sampler,
            &self.buttons_view,
        );
        self.buttons_atlas = atlas;
        self.buttons_role = role;
    }
}
