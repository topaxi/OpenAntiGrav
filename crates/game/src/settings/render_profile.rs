//! The render-cost-sensitive slice of [`super::Graphics`], kept one per
//! title rather than shared - see the parent module's "Display against
//! graphics against render profiles" doc section for the reasoning, and
//! [`RenderProfile`] for the type itself.
//!
//! Its own file rather than staying inline in `settings.rs`: that file is
//! past the 1,000-line rule with this and [`super::migrate_render_profiles`]
//! in it. See `scripts/check-file-size.py`, which is the rule as a gate.

use serde::{Deserialize, Serialize};

use super::Settings;

/// Every title this build links, in the order their [`RenderProfile`]
/// entries are written - what a fresh file's `[render_profiles]` table
/// documents regardless of which discs happen to be on this machine.
///
/// **Not [`crate::main::session::Session::titles`]** - that is which titles
/// *this machine* can currently open a source for; this is which titles
/// *this build* knows how to boot at all, the same distinction
/// `render_profiles` itself exists to respect: a profile is not evidence
/// about hardware someone has, only about a title this game can run.
pub(super) const KNOWN_TITLES: &[&str] = &[
    oag_pulse::TITLE.name,
    oag_pure::TITLE.name,
    oag_hd::TITLE.name,
    oag_2048::TITLE.name,
];

/// See [`RenderProfile::dynamic_resolution_floor`]: **50 %**.
///
/// Its own function rather than `Default::default()`, which for a
/// [`crate::display::Scale`] is 100 - a floor equal to the ceiling, which is
/// the one value that means "the controller has nowhere to go" and would fire
/// the menu's own warning out of the box.
fn default_dynamic_resolution_floor() -> crate::display::Scale {
    crate::display::Scale::OFFERED[0]
}

/// The render-cost-sensitive slice of [`super::Graphics`], persisted one per
/// title rather than shared - see the module doc's "Display against graphics
/// against render profiles" section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderProfile {
    /// What percentage of the displayed size the game is rendered at.
    ///
    /// Below 100 is the usual internal-resolution knob; above it is
    /// supersampling. Measured against the aspect rectangle rather than the
    /// window, so it means the same thing whatever `display.aspect` is. See
    /// [`crate::display::Scale`].
    #[serde(default)]
    pub render_scale: crate::display::Scale,
    /// The frame rate a resolution controller aims for, or `off`.
    ///
    /// The target *is* the enable, so there is no second key that can disagree
    /// with it. While it is on, `render_scale` above becomes the **ceiling**
    /// and keeps its meaning when it is off - the controller reads it and
    /// never writes it, so a loaded machine cannot make a chosen quality drift
    /// downward between sessions. See [`crate::drs`].
    ///
    /// Per title beside the ceiling it pairs with, which is what this whole
    /// table is for: a target that Pulse holds comfortably is not one HD/Fury
    /// holds, and the point of dynamic resolution is that the answer depends
    /// on how expensive the scene is.
    #[serde(default)]
    pub dynamic_resolution: crate::drs::Target,
    /// The lowest render scale the controller may fall to.
    ///
    /// A percentage of the aspect rectangle, exactly as `render_scale` is, so
    /// the two are directly comparable and a floor at or above the ceiling
    /// means the controller has nowhere to go. Ignored while
    /// `dynamic_resolution` is `off`. See [`crate::display::Scale`].
    ///
    /// **Defaults to 50 %**, the lowest value the render-scale row itself
    /// offers, so the floor and the ceiling read against the same list.
    #[serde(default = "default_dynamic_resolution_floor")]
    pub dynamic_resolution_floor: crate::display::Scale,
    /// Which resampler carries the frame onto the surface: `off` or
    /// `fsr1`.
    ///
    /// Defaults to `off`, which is what this always did. FSR 1 costs two
    /// fullscreen passes and is a clear win on photographic art at a low render
    /// scale; whether it is one on this game's hard-edged paletted art is a
    /// screenshot comparison has now been run, and at 50 % on one frame of one
    /// track FSR 1 wins clearly. **The default has not moved on it**, because
    /// one frame of one track is not the sample a default flip is held to
    /// here; see HANDOVER for what would settle it. See
    /// [`crate::display::Upscaler`].
    ///
    /// **Only has an effect below 100 % `render_scale`.** FSR 1 is a magnifier;
    /// asked to minify it undoes the supersampling it was handed. See
    /// `crate::upscale::magnifies`.
    #[serde(default)]
    pub upscaler: crate::display::Upscaler,
    /// How hard FSR 1's RCAS pass sharpens, in stops: 0 is maximum and each
    /// whole step halves it. Ignored unless `upscaler` is `fsr1`.
    #[serde(default)]
    pub upscale_sharpness: crate::display::Sharpness,
    /// Which anti-aliasing the scene draws with: `off`, `fxaa`, `smaa` or
    /// `msaa4x`.
    ///
    /// Defaults to `off`. **Only `msaa4x` is baked into the scene's
    /// pipelines when a race starts** - `off`, `fxaa` and `smaa` are read
    /// fresh every frame by `upscale::Framebuffer::resolve_scene`, the same as
    /// `upscaler` is, and moving among those three takes effect the frame
    /// they were chosen on. Moving to or from `msaa4x` takes effect the next
    /// time a race is launched, because that is what rebuilds the pipelines
    /// it is a property of. See [`crate::display::AntiAliasing`] and
    /// `docs/architecture/adr/0013-anti-aliasing-architecture.md`.
    #[serde(default)]
    pub anti_aliasing: crate::display::AntiAliasing,
    /// How hard the finished frame is smeared along the camera's own motion:
    /// `off`, `low`, `medium` or `high`.
    ///
    /// **An enhancement of this project's, off by default.** Neither PSP
    /// build renders motion blur, so on is a divergence a player opts into -
    /// the same footing as FSR 1 and SMAA, unlike the recovered bloom
    /// (`Graphics::bloom`) whose default is about calibration. It is also the
    /// first consumer of the reprojection infrastructure temporal
    /// anti-aliasing needs, which is most of why it exists - see
    /// `oag_render::post::motion_blur`, `docs/rendering/motion-blur.md` and
    /// ADR-0028.
    ///
    /// A strength rather than a boolean, per that design: the value names a
    /// shutter fraction, and the technique underneath can improve without a
    /// settings migration - it already has once, camera reprojection to the
    /// design's per-object velocity buffer, with nobody's file moving. Read
    /// fresh every frame, so the row applies live, MSAA included: the
    /// blur's prepare stage reads sample 0 of the multisampled attachments.
    #[serde(default)]
    pub motion_blur: crate::display::MotionBlur,
}

impl Default for RenderProfile {
    fn default() -> Self {
        Self {
            render_scale: crate::display::Scale::default(),
            dynamic_resolution: crate::drs::Target::default(),
            // The one field whose default is not its type's - see
            // `default_dynamic_resolution_floor`, and the reason this impl is
            // written out rather than derived.
            dynamic_resolution_floor: default_dynamic_resolution_floor(),
            upscaler: crate::display::Upscaler::default(),
            upscale_sharpness: crate::display::Sharpness::default(),
            anti_aliasing: crate::display::AntiAliasing::default(),
            motion_blur: crate::display::MotionBlur::default(),
        }
    }
}

/// Inserts a default [`RenderProfile`] for every title in [`KNOWN_TITLES`]
/// that [`Settings::render_profiles`] does not already hold, so the file
/// stays a complete reference for what it can hold - the same property
/// `HEADER` (in `settings.rs`) documents for every other table.
pub(super) fn ensure_known_titles(settings: &mut Settings) {
    for title in KNOWN_TITLES {
        settings
            .render_profiles
            .entry((*title).to_string())
            .or_default();
    }
}

/// Every key a `[render_profiles.<title>]` table holds.
///
/// A **superset** of [`MOVED_TO_RENDER_PROFILES`], and the distinction is not
/// pedantry: a setting added to the profile *after* the split never lived flat
/// in `[graphics]`, so there is nothing to migrate for it and listing it as
/// migratable would claim a history it does not have. `dynamic_resolution` and
/// its floor are the first two of those - they landed straight here, beside
/// the `render_scale` ceiling they are compared against.
///
/// Test-only: nothing in the running game asks "which keys does a profile
/// hold", because the struct answers that. What needs it is the sweep that
/// checks every menu seed lands in a table the settings file actually writes.
#[cfg(test)]
pub(super) const PROFILE_KEYS: [&str; 7] = [
    "render_scale",
    "dynamic_resolution",
    "dynamic_resolution_floor",
    "upscaler",
    "upscale_sharpness",
    "anti_aliasing",
    "motion_blur",
];

/// The keys that used to live flat in `[graphics]` and now live in one
/// `[render_profiles.<title>]` table per title. See [`RenderProfile`] for
/// why each is render-cost-sensitive enough to want this.
///
/// **Only the ones with a flat past**, which is what separates this from
/// [`PROFILE_KEYS`].
pub(super) const MOVED_TO_RENDER_PROFILES: [&str; 5] = [
    "render_scale",
    "upscaler",
    "upscale_sharpness",
    "anti_aliasing",
    "motion_blur",
];

/// Moves any of [`MOVED_TO_RENDER_PROFILES`] a file still holds flat in
/// `[graphics]` into every title in [`KNOWN_TITLES`]'s own
/// `[render_profiles.<title>]` table.
///
/// **Every known title's entry is seeded with the same one value**, not just
/// one title's. A pre-split file has no way to say which title a saved value
/// was tuned against - it was one shared value at the time - so guessing
/// which title "owns" it would be inventing evidence the file does not have.
/// Copying it everywhere is the same "absence is not evidence of wrong" rule
/// `Display::front_end_style` and `Ai::difficulty` already follow (both in
/// `settings.rs`). Every title starts identical and only diverges once a
/// player changes one of them on purpose.
///
/// **`[render_profiles.<title>]` wins where both have a key**, the same rule
/// `settings::migrate` applies for `[display]`: a hand edit or a copy from an
/// older machine putting the flat key back must not clobber a value already
/// split out per title.
pub(super) fn migrate_render_profiles(table: &mut toml::Table) {
    if table
        .get("render_profiles")
        .is_some_and(|value| !value.is_table())
    {
        return;
    }
    let Some(graphics) = table
        .get_mut("graphics")
        .and_then(toml::Value::as_table_mut)
    else {
        return;
    };
    let moved: Vec<(String, toml::Value)> = MOVED_TO_RENDER_PROFILES
        .iter()
        .filter_map(|key| {
            graphics
                .remove(*key)
                .map(|value| ((*key).to_string(), value))
        })
        .collect();
    if moved.is_empty() {
        return;
    }
    let render_profiles = table
        .entry("render_profiles")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .expect("checked on the way in, and only ever inserted as a table");
    for title in KNOWN_TITLES {
        let profile = render_profiles
            .entry((*title).to_string())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .expect("only ever inserted as a table, immediately above");
        for (key, value) in &moved {
            profile.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
}
