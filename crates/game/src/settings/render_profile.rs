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
///
/// **A list of titles, not of rows** - see [`known_profiles`] for the rows a
/// fresh file actually writes. A title contributes one row per *platform* it
/// ships an archive candidate for, and Pulse ships two (PSP and PS2); this
/// list itself does not carry that, deliberately, so it stays the same list
/// [`crate::title::open_source`] tries titles from.
pub(super) const KNOWN_TITLES: &[&oag_title::Title] = &[
    oag_pulse::TITLE,
    oag_pure::TITLE,
    oag_hd::TITLE,
    oag_2048::TITLE,
    oag_omega::TITLE,
];

/// The `[render_profiles.<key>]` table a `(title, platform)` pair reads and
/// writes.
///
/// **Chosen: `"<Title::name> (<Platform>)"`, e.g. `"Wipeout Pulse (PSP)"` or
/// `"Wipeout Pulse (PS2)"`.** This keeps the existing convention rather than
/// inventing a second one beside it: a `render_profiles` key already *is*
/// `Title::name`, quoted in the TOML file for every title whose name has a
/// space in it - `[render_profiles."Wipeout HD"]` is what a file on disk
/// looks like today - and this only extends that string, rather than a
/// disjoint slug (`pulse-ps2`) or a nested `[render_profiles.<title>.<platform>]`
/// table. Nesting would need [`Settings::render_profiles`] to become a
/// `BTreeMap<String, BTreeMap<String, RenderProfile>>` and touch every one of
/// the call sites - `menu_seeds`, `Session::render_profile`,
/// `Session::render_profile_mut`, the headless and capture routes - that key
/// it with one plain string today, for a shape the file gains nothing from:
/// nothing reads "every platform of this title" as a group.
///
/// `Platform`'s own [`std::fmt::Display`] supplies the second half, so
/// nothing here hand-spells "PSP" or "PS2" a second time - see
/// `oag_disc::platform` for where that string comes from.
#[must_use]
pub fn profile_key(title: &oag_title::Title, platform: oag_disc::Platform) -> String {
    format!("{} ({platform})", title.name)
}

/// The platforms `title` ships an archive candidate for, off its own
/// `archives.data` list rather than hand-spelled - see [`known_profiles`].
///
/// Pulse's own `DATA_CANDIDATES` names both `Platform::Psp` and
/// `Platform::Ps2`, in that order, which is why Pulse alone contributes two
/// rows below; every other title in [`KNOWN_TITLES`] today names one.
fn title_platforms(title: &'static oag_title::Title) -> impl Iterator<Item = oag_disc::Platform> {
    title.archives.data.iter().map(|&(_, platform)| platform)
}

/// Every `(title, platform)` pair this build knows how to boot - one row per
/// platform a [`KNOWN_TITLES`] entry ships an archive candidate for.
///
/// Derived from each title's own [`oag_title::Title::archives`] rather than
/// six hand-written pairs, so the known list cannot drift from the one
/// `crate::title::open_source` actually opens against - a title that grew a
/// third platform's candidate would grow a third row here for free.
pub(super) fn known_profiles()
-> impl Iterator<Item = (&'static oag_title::Title, oag_disc::Platform)> {
    KNOWN_TITLES
        .iter()
        .flat_map(|&title| title_platforms(title).map(move |platform| (title, platform)))
}

/// See [`RenderProfile::minimum_resolution`]: **50 %**.
///
/// Its own function rather than `Default::default()`, which for a
/// [`oag_display::display::Scale`] is 100 - a floor equal to the ceiling, which is
/// the one value that means "the controller has nowhere to go" and would fire
/// the menu's own warning out of the box.
fn default_minimum_resolution() -> oag_display::display::Scale {
    oag_display::display::Scale::OFFERED[0]
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
    /// [`oag_display::display::Scale`].
    #[serde(default)]
    pub render_scale: oag_display::display::Scale,
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
    pub target_fps: crate::drs::Target,
    /// The lowest render scale the controller may fall to.
    ///
    /// A percentage of the aspect rectangle, exactly as `render_scale` is, so
    /// the two are directly comparable and a floor at or above the ceiling
    /// means the controller has nowhere to go. Ignored while
    /// `target_fps` is `off`. See [`oag_display::display::Scale`].
    ///
    /// **Defaults to 50 %**, the lowest value the render-scale row itself
    /// offers, so the floor and the ceiling read against the same list.
    #[serde(default = "default_minimum_resolution")]
    pub minimum_resolution: oag_display::display::Scale,
    /// What resolves the frame onto the surface: `off`, `fxaa`, `smaa`,
    /// `fsr1` or `fsr3`.
    ///
    /// **One axis since
    /// [ADR-0041](../../../../docs/architecture/adr/0041-one-row-for-what-resolves-the-frame.md)**,
    /// where an anti-aliasing key and an upscaler key used to sit side by side
    /// and need five menu warnings to describe how they interact. Every value
    /// answers the same question, so at most one can be doing it.
    ///
    /// Defaults to `off`, the blit's own bilinear tap. Read fresh every frame
    /// by `upscale::Framebuffer::resolve_scene`, so moving this row takes
    /// effect the frame it was chosen on - unlike [`Self::msaa`].
    ///
    /// **`fsr1` only has an effect below 100 % `render_scale`.** FSR 1 is a
    /// magnifier; asked to minify it undoes the supersampling it was handed.
    /// See `crate::upscale::magnifies`. `fsr3` has something to do at every
    /// scale. See [`oag_display::display::Reconstruction`].
    #[serde(default)]
    pub reconstruction: oag_display::display::Reconstruction,
    /// How hard FSR 1's RCAS pass sharpens, in stops: 0 is maximum and each
    /// whole step halves it. Ignored unless `upscaler` is `fsr1`.
    #[serde(default)]
    pub upscale_sharpness: oag_display::display::Sharpness,
    /// How many samples the rasterizer takes: `off` or `4x`.
    ///
    /// **Its own axis since ADR-0041**, because it is a property of the
    /// rasterizer rather than a choice about what reads the resolved result -
    /// it composes with every [`Self::reconstruction`] value except `fsr3`,
    /// which anti-aliases the same frame temporally and greys this row.
    ///
    /// **Baked into the scene's pipelines when a race starts**, so moving it
    /// takes effect the next time a race is launched rather than the frame it
    /// was chosen on. See [`oag_display::display::Msaa`] and
    /// `docs/architecture/adr/0013-anti-aliasing-architecture.md`.
    #[serde(default)]
    pub msaa: oag_display::display::Msaa,
    /// How hard the finished frame is smeared along the camera's own motion:
    /// `off`, `low`, `medium` or `high`.
    ///
    /// **An enhancement of this project's, off by default.** Neither PSP
    /// build renders motion blur, so on is a divergence a player opts into -
    /// the same footing as FSR 1 and SMAA, unlike the recovered bloom
    /// (`Graphics::bloom`), on by default because the original runs it. It is also the
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
    pub motion_blur: oag_display::display::MotionBlur,
    /// What casts a shadow, and what draws it: `off` or `blob`.
    ///
    /// **In this per-title table rather than flat in `[graphics]`** for both
    /// of the reasons this table exists. It is render-cost-sensitive the way
    /// [`Self::msaa`] is - a quad per craft on a grid of eight is eight extra
    /// blended draws - and, unlike a shutter fraction, *what a value means
    /// depends on the title*: the `original` tier still to come is 129
    /// authored occluder hulls on Pulse, a shadow-map pipeline on HD/Fury,
    /// a track-proximity projection on 2048, and honest absence on Pure. One
    /// shared value could not carry that.
    ///
    /// Defaults to `off` on every title and stays there until `original`
    /// exists - see [`oag_display::display::Shadows::Off`] for why a generated
    /// falloff is not a default this project may take. Read fresh every
    /// frame, so the setting applies live.
    #[serde(default)]
    pub shadows: oag_display::display::Shadows,
    /// Which screen filter draws the finished frame: `off`, or the id of a
    /// preset - a built-in such as `psp-3000` or `crt-interlaced`, or the
    /// stem of a `.wgsl` file in the player's own `shaders/` directory. See
    /// [`crate::screen`].
    ///
    /// **Per title, and that is the whole reason it is here rather than in
    /// `[display]` beside brightness and gamma**, where "applied after the
    /// game has finished drawing" would otherwise have put it. What the right
    /// filter *is* depends on which machine the title ran on: a PSP disc wants
    /// its LCD, the PS2 port wants an interlaced television, and Wipeout HD
    /// wants nothing at all - so one shared value would be wrong on two of
    /// the three titles a player switches between. Not render-cost-sensitive
    /// in the way `render_scale` is, but the table's other reason - *what a
    /// value means depends on the title* - applies exactly as it does to
    /// [`Self::shadows`].
    ///
    /// A `String` rather than an enum for the reason `display.front_end_style`
    /// is one: which values exist is a property of files on disk, and a file
    /// naming a preset that is no longer there draws unfiltered with a note
    /// rather than failing to load. Defaults to `off`.
    #[serde(default = "default_screen_filter")]
    pub screen_filter: String,
    /// How much of that filter shows, as a percentage: 100 is the preset as
    /// authored. See [`oag_display::display::FilterStrength`].
    #[serde(default)]
    pub screen_filter_strength: oag_display::display::FilterStrength,
    /// How far out an authored `LodGroup` switches to its coarser tiers:
    /// `original`, `high` or `maximum`.
    ///
    /// **`original` is the recovered rule and the default** -
    /// `LodGroup_SelectChild`'s per-frame switch at the distances the disc
    /// authors (`docs/formats/vex.md`, "the switch is found"). `high` doubles
    /// every distance and `maximum` never switches; both are this project's
    /// own, chosen rather than measured. One multiplier on the authored
    /// distances, so the rule itself stays the one source of truth - see
    /// [`oag_render::mesh::ModelDetail`].
    ///
    /// Per title because it is render-cost-sensitive the way
    /// [`Self::msaa`] is: the coarse tier is where a full grid's triangle
    /// count goes down. Read fresh every frame, so the row applies live.
    #[serde(default)]
    pub model_detail: oag_render::mesh::ModelDetail,
}

/// See [`RenderProfile::screen_filter`]: `off`.
fn default_screen_filter() -> String {
    SCREEN_FILTER_OFF.to_string()
}

/// The value of [`RenderProfile::screen_filter`] that runs no pass at all.
pub const SCREEN_FILTER_OFF: &str = "off";

impl Default for RenderProfile {
    fn default() -> Self {
        Self {
            render_scale: oag_display::display::Scale::default(),
            target_fps: crate::drs::Target::default(),
            // The one field whose default is not its type's - see
            // `default_minimum_resolution`, and the reason this impl is
            // written out rather than derived.
            minimum_resolution: default_minimum_resolution(),
            reconstruction: oag_display::display::Reconstruction::default(),
            upscale_sharpness: oag_display::display::Sharpness::default(),
            msaa: oag_display::display::Msaa::default(),
            motion_blur: oag_display::display::MotionBlur::default(),
            shadows: oag_display::display::Shadows::default(),
            screen_filter: default_screen_filter(),
            screen_filter_strength: oag_display::display::FilterStrength::default(),
            model_detail: oag_render::mesh::ModelDetail::default(),
        }
    }
}

/// Inserts a default [`RenderProfile`] for every `(title, platform)` row
/// [`known_profiles`] names that [`Settings::render_profiles`] does not
/// already hold, so the file stays a complete reference for what it can
/// hold - the same property `HEADER` (in `settings.rs`) documents for every
/// other table.
pub(super) fn ensure_known_titles(settings: &mut Settings) {
    for (title, platform) in known_profiles() {
        settings
            .render_profiles
            .entry(profile_key(title, platform))
            .or_default();
    }
}

/// Every key a `[render_profiles.<title>]` table holds.
///
/// A **superset** of [`MOVED_TO_RENDER_PROFILES`], and the distinction is not
/// pedantry: a setting added to the profile *after* the split never lived flat
/// in `[graphics]`, so there is nothing to migrate for it and listing it as
/// migratable would claim a history it does not have. `target_fps` and
/// its floor are the first two of those - they landed straight here, beside
/// the `render_scale` ceiling they are compared against.
///
/// Test-only: nothing in the running game asks "which keys does a profile
/// hold", because the struct answers that. What needs it is the sweep that
/// checks every menu seed lands in a table the settings file actually writes.
#[cfg(test)]
pub(super) const PROFILE_KEYS: [&str; 11] = [
    "render_scale",
    "target_fps",
    "minimum_resolution",
    "reconstruction",
    "upscale_sharpness",
    "msaa",
    "motion_blur",
    "shadows",
    "screen_filter",
    "screen_filter_strength",
    "model_detail",
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

/// Folds a profile's pre-[ADR-0041] `upscaler` and `anti_aliasing` keys into
/// `reconstruction` and `msaa`.
///
/// **Both old keys are consumed, and one of them can lose information.** The
/// two used to be independent, so a file can hold `anti_aliasing = "fxaa"`
/// beside `upscaler = "fsr1"` - a pairing the new axis cannot express, and one
/// `menu.toml` already warned about as `FIGHTS THE UPSCALER'S EDGE-ADAPTIVE
/// RESAMPLE`. The upscaler wins, because it is the value that was actually
/// carrying the frame onto the surface; the spatial pass is dropped.
///
/// `anti_aliasing = "msaa4x"` becomes `msaa = "4x"` and leaves the
/// reconstruction to the old `upscaler`, which is the case that loses nothing:
/// the two really were orthogonal.
///
/// Runs on a table rather than on a [`RenderProfile`] for the reason
/// [`super::migrate_render_profiles`] does - serde has not seen the value yet,
/// so an unknown key would be dropped silently before any typed code could
/// look at it.
fn migrate_reconstruction(profile: &mut toml::Table) {
    let upscaler = profile
        .remove("upscaler")
        .and_then(|value| value.as_str().map(str::to_owned));
    let anti_aliasing = profile
        .remove("anti_aliasing")
        .and_then(|value| value.as_str().map(str::to_owned));
    // Nothing to fold: a file already written on this side of the split, or a
    // fresh one. Left alone rather than defaulted, so serde's own `default`
    // stays the single answer to "what does a missing key mean".
    if upscaler.is_none() && anti_aliasing.is_none() {
        return;
    }
    let msaa = matches!(anti_aliasing.as_deref(), Some("msaa4x"));
    let reconstruction = match upscaler.as_deref() {
        // An upscaler that was doing something carries over as-is, whatever
        // the anti-aliasing row said beside it.
        Some(name @ ("fsr1" | "fsr3")) => name.to_owned(),
        // No upscaler, so whatever the anti-aliasing row held is what was
        // resolving the frame - unless it was MSAA, which is not on this axis.
        _ => match anti_aliasing.as_deref() {
            Some(name @ ("fxaa" | "smaa")) => name.to_owned(),
            _ => "off".to_owned(),
        },
    };
    profile.insert("reconstruction".to_owned(), reconstruction.into());
    profile.insert("msaa".to_owned(), if msaa { "4x" } else { "off" }.into());
}

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
    for &title in KNOWN_TITLES {
        let profile = render_profiles
            .entry(title.name.to_string())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()))
            .as_table_mut()
            .expect("only ever inserted as a table, immediately above");
        for (key, value) in &moved {
            profile.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
}

/// Splits every bare `[render_profiles.<title>]` table - the shape
/// [`migrate_render_profiles`] just populated, or a hand-migrated file, or an
/// older file's own - into one `[render_profiles.<key>]` row per platform
/// that title ships, per [`known_profiles`]. See the parent module's
/// "Display against graphics against render profiles" doc for why a
/// PS2-sourced Pulse race and a PSP-sourced one read different rows from here
/// on: different scenes, different geometry payloads, the PS2's own engine
/// flare.
///
/// **Every platform seeded from the one bare value**, not just one - the
/// same "absence is not evidence of wrong" rule [`migrate_render_profiles`]'s
/// own doc names: a bare key has no way to say which platform a saved value
/// was tuned against, so guessing one would invent evidence the file does not
/// have. Pulse's PSP and PS2 rows start identical and only diverge once a
/// player changes one of them on purpose.
///
/// **A platform-specific row already present wins**, the same rule
/// `settings::migrate` applies for `[display]`: a hand edit or a copy from an
/// older machine putting the bare key back must not clobber a value already
/// split out per platform.
///
/// **The bare key is removed once processed, whether or not it seeded
/// anything new.** Left in place it is a key nothing types away -
/// `Settings::render_profiles` is a `BTreeMap<String, RenderProfile>`, and any
/// string parses as one - so it would round-trip forever as a row nothing
/// reads or writes.
///
/// Run after [`migrate_reconstruction_keys`], not before: that one folds
/// ADR-0041's `upscaler`/`anti_aliasing` keys wherever `render_profiles`
/// already has entries, and every file old enough to still carry those two
/// keys is also old enough to still be bare-title-keyed, so running this
/// split first would leave the platform rows it just created still holding
/// the pre-fold spelling.
pub(super) fn migrate_platform_split(table: &mut toml::Table) {
    if table
        .get("render_profiles")
        .is_some_and(|value| !value.is_table())
    {
        return;
    }
    let Some(profiles) = table
        .get_mut("render_profiles")
        .and_then(toml::Value::as_table_mut)
    else {
        return;
    };
    for &title in KNOWN_TITLES {
        let Some(bare) = profiles.remove(title.name) else {
            continue;
        };
        for platform in title_platforms(title) {
            profiles
                .entry(profile_key(title, platform))
                .or_insert_with(|| bare.clone());
        }
    }
}

/// Folds every profile's pre-[ADR-0041] `upscaler` and `anti_aliasing` keys.
///
/// **After [`migrate_render_profiles`], never before.** That one moves the two
/// old keys out of a flat `[graphics]` table and into each profile; running
/// this first would fold the profiles that already existed and leave the ones
/// it was about to create still holding the old spelling.
pub(super) fn migrate_reconstruction_keys(table: &mut toml::Table) {
    let Some(profiles) = table
        .get_mut("render_profiles")
        .and_then(toml::Value::as_table_mut)
    else {
        return;
    };
    for (_, profile) in profiles.iter_mut() {
        if let Some(profile) = profile.as_table_mut() {
            migrate_reconstruction(profile);
        }
    }
}
