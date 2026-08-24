//! Persisted settings.
//!
//! Read from `<config dir>/oag/settings.toml` on startup and rewritten with
//! every recognised key filled in, so the file on disk always documents what
//! it can hold rather than starting empty or silently dropping a key an older
//! version never wrote. A CLI flag still wins over whatever is on disk for
//! that one run; see `--anisotropy` in `main.rs`.
//!
//! # Display against graphics
//!
//! Two tables, matching the two pages the menus put them on:
//!
//! - **`[display]`** is the picture's container and how it reaches a screen:
//!   which monitor, what kind of window, how big, what shape, how a finished
//!   frame is presented and what happens to it on the way out.
//! - **`[graphics]`** is how the picture is drawn: how many pixels, how the
//!   textures are filtered, how much of the world is in frame.
//!
//! The line between them is *whether the renderer would notice*. Turning off
//! vsync changes nothing about the frame that is drawn, only about when it is
//! shown; halving the render scale changes the frame itself. Brightness and
//! gamma sit on the display side under that rule even though they are a shader:
//! they are a monitor calibration, applied after the game has finished drawing.
//!
//! Everything was in `[graphics]` until this split, so [`load`] migrates a file
//! that still is - see [`migrate`].

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use oag_render::mesh::Lod;
use oag_render::mesh_render::Anisotropy;

mod controls;

pub use controls::{Controls, TriggerSensitivity};

/// Mirrors [`Anisotropy`] for serde, which cannot derive on a type this crate
/// does not own. Named the same as `Display`/`FromStr` already spell it in
/// `oag-render`, so the two stay one vocabulary.
#[derive(Serialize, Deserialize)]
#[serde(remote = "Anisotropy")]
enum AnisotropyDef {
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "2x")]
    X2,
    #[serde(rename = "4x")]
    X4,
    #[serde(rename = "8x")]
    X8,
    #[serde(rename = "16x")]
    X16,
}

/// Mirrors [`Lod`] for serde, the same way [`AnisotropyDef`] mirrors
/// `Anisotropy` - a type this crate does not own, so it cannot derive here.
#[derive(Serialize, Deserialize)]
#[serde(remote = "Lod")]
enum LodDef {
    #[serde(rename = "both")]
    Both,
    #[serde(rename = "single")]
    Single,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub display: Display,
    #[serde(default)]
    pub graphics: Graphics,
    #[serde(default)]
    pub audio: Audio,
    #[serde(default)]
    pub source: Source,
    #[serde(default)]
    pub race: Race,
    #[serde(default)]
    pub controls: Controls,
    #[serde(default)]
    pub ai: Ai,
    /// The language picked last time, by the XML's own English name
    /// (`English`, `French`).
    ///
    /// `None` means the picker has not been through yet, and is what makes the
    /// first run ask and every run after it not. Naming a language the source
    /// does not carry falls back to asking, with a note - a settings file
    /// written against the EU disc opened against the USA one, which ships
    /// English only. `--pick-language` shows the picker regardless.
    #[serde(default)]
    pub language: Option<String>,
}

/// What the Race page of the menus last chose.
///
/// Persisted for the same reason `source.image` is: a player who always races
/// one team on one circuit should not have to say so twice. These are archive
/// path components and speed-class names, **not display names** - the words the
/// original shows a player for a circuit are content this project does not ship.
/// See `docs/architecture/adr/0006-no-copyrighted-content.md`.
///
/// The command line still wins for the run it is given on, and `--race` skips
/// the menus entirely, so these are what the menus set rather than a second
/// place to configure a race from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Race {
    /// Which mode's rules run: `time_trial`, `speed_lap` or `zone`.
    ///
    /// One of `oag_race::Mode::ALL`'s tokens. A file carrying anything else
    /// leaves the mode at the default rather than failing the boot, the same way
    /// an unknown speed class does.
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Speed class: `venom`, `flash`, `rapier` or `phantom`.
    #[serde(default = "default_class")]
    pub class: String,
    /// Team directory under `Data\Ships\`, one of `handling::TEAMS`.
    #[serde(default = "default_team")]
    pub team: String,
    /// Which circuit, by the id its own plugin definition gives it.
    ///
    /// **A race, not a directory.** `16_Track` and `32_Track` are two entries
    /// and one folder, the second being the first driven the other way, so this
    /// is not a path component and must not be turned into one here - only the
    /// source can say which `.vex` an id loads. See [`crate::catalogue`].
    #[serde(default = "default_track")]
    pub track: String,
}

/// Time trial: the mode the RACE page opens on, and the one the reference
/// captures under `data/traces/` were taken in.
fn default_mode() -> String {
    oag_race::Mode::TimeTrial.name().to_string()
}
fn default_class() -> String {
    "venom".to_string()
}
fn default_team() -> String {
    crate::race::DEFAULT_TEAM.to_string()
}
/// The circuit the reference scenario is on, so the default run is the one
/// every capture under `data/traces/` was taken against.
fn default_track() -> String {
    "16_Track".to_string()
}

impl Default for Race {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            class: default_class(),
            team: default_team(),
            track: default_track(),
        }
    }
}

/// What the opponents are like.
///
/// The section `docs/gameplay/ai.md` specifies. Only `difficulty` is built; the
/// page's other three keys - `rubberbanding`, `adaptive`, `mistakes` - are
/// design that has not been written yet, and are deliberately absent rather
/// than present and ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ai {
    /// How good the opponents are: `novice`, `skilled`, `elite` or `ace`.
    ///
    /// A **token**, not a typed enum, for the reason [`Race::mode`] is: a bad
    /// value here must not fail the boot, so a profile written by a build with
    /// a level this one does not have still races. The fallback is reported and
    /// ignored, in `main.rs`.
    #[serde(default = "default_difficulty")]
    pub difficulty: String,
}

fn default_difficulty() -> String {
    oag_ai::Difficulty::default().name().to_string()
}

impl Default for Ai {
    fn default() -> Self {
        Self {
            difficulty: default_difficulty(),
        }
    }
}

/// How loud each bus is.
///
/// One row today, and the buses are `oag_audio::Bus`'s own - so this section
/// grows an entry per bus rather than per sound. A typed percentage rather than
/// a bare `u32` for the reason [`crate::display::Brightness`] is one: a value
/// out of range is a file that fails to load with a message, not a gain of 4000
/// discovered by ear.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Audio {
    /// How loud the music bus is, as a percentage. Defaults to 100.
    ///
    /// **Full by default on purpose.** At 100 the mixer's gain is exactly 1.0,
    /// so a `--dump-audio` capture of the PS2 archive round-trips the disc's own
    /// PCM sample-for-sample and can be compared against it byte for byte; any
    /// other default would make the only evidence this project can gather about
    /// its own audio approximate. See [`crate::audio::DUMP_SAMPLE_RATE`].
    #[serde(default)]
    pub music_volume: crate::audio::Volume,
    /// How loud the effects bus is, as a percentage. Defaults to 100.
    ///
    /// The original's options menu has exactly two volumes and this is the
    /// other one: `"Music Volume"` at `0x08a78658` and `"SFX Volume"` at
    /// `0x08a78668`, with nothing else. See
    /// `docs/architecture/adr/0018-audio-mixer-architecture.md`.
    #[serde(default)]
    pub sfx_volume: crate::audio::Volume,
    /// Which release's encode of the soundtrack plays: `auto`, `psp` or `ps2`.
    ///
    /// **`auto`, the booted disc, by default.** Only the sixteen soundtrack
    /// tracks have a proven counterpart on the other release - see
    /// `docs/formats/ps2-audio.md` - so this moves those and nothing else.
    /// Voice, every sound bank, and **the PSP front end's own music** stay
    /// where the game booted from; that last one has no PS2 counterpart at all,
    /// so a PSP boot sounds the same at every value of this key. See
    /// [`crate::audio::MusicSource`], which spells out why.
    ///
    /// The menu row is offered only on a machine that has both discs, but the
    /// key is always in this file, because a settings file is carried between
    /// machines and a value it cannot honour falls back to the booted disc
    /// rather than to silence.
    #[serde(default)]
    pub music_source: crate::audio::MusicSource,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Source {
    /// A disc image, or a directory extracted with `oag-unpack`, opened when
    /// neither the command line nor `$OAG_IMAGE` name one.
    ///
    /// Persisted so a player who always plays off one disc - the PS2 release,
    /// say - does not have to spell it out on every run. The command line
    /// still wins over this for that one run; see `source::resolve`.
    #[serde(default)]
    pub image: Option<String>,
    /// Directories holding downloadable content, searched when neither the
    /// command line nor `$OAG_DLC` name any.
    ///
    /// A list rather than one path, because packs are bought one at a time and
    /// end up wherever the player put each of them. Empty means "look in the
    /// usual places"; see [`crate::source::resolve_dlc`].
    #[serde(default)]
    pub dlc: Vec<String>,
}

/// Which screen the game opens on, what kind of window it opens, and how a
/// finished frame gets from the renderer onto it.
///
/// Nothing in here changes what is drawn - see the module documentation for
/// where the line against [`Graphics`] falls and why.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Display {
    /// Which screen to open on: `default`, or a monitor by name. See
    /// [`crate::display::Monitor`].
    ///
    /// A name this machine does not have falls back to the default with a note
    /// rather than failing, because the usual way to get one is to unplug a
    /// screen.
    #[serde(default)]
    pub monitor: crate::display::Monitor,
    /// `windowed` or `borderless`. See [`crate::display::WindowMode`].
    #[serde(default)]
    pub window_mode: crate::display::WindowMode,
    /// How big a *windowed* window is, spelled `1440x816`.
    ///
    /// Means nothing in borderless, where the display decides.
    #[serde(default)]
    pub window_size: crate::display::Size,
    /// The shape the game is drawn at inside its window: `psp`, `ps2` or
    /// `free`. See [`crate::display::Aspect`].
    #[serde(default)]
    pub aspect: crate::display::Aspect,
    /// How a finished frame reaches the display: `off`, `on` or `smooth`.
    ///
    /// **`off` by default**, because this is a racing game and vsync's cost is
    /// a frame of latency. `smooth` is what "triple buffering" means on a
    /// modern API - no tearing and no half-rate cliff, at the price of the
    /// frames it discards. See [`crate::perf::Vsync`].
    ///
    /// A file that still says `vsync = true` or `false` is read, and rewritten
    /// as a name.
    #[serde(default)]
    pub vsync: crate::perf::Vsync,
    /// How many frames a second the loop may produce: `unlimited`, or a rate up
    /// to 1000. Defaults to 240.
    ///
    /// **Ignored under `vsync = "on"`**, where the display decides - and only
    /// there. Under `smooth` the loop is not blocked, so without this it
    /// renders frames the compositor throws away. See
    /// [`crate::perf::FrameLimit`].
    #[serde(default)]
    pub frame_limit: crate::perf::FrameLimit,
    /// What the finished picture is multiplied by, as a percentage. See
    /// [`crate::display::Brightness`].
    #[serde(default)]
    pub brightness: crate::display::Brightness,
    /// The midtone curve applied to the finished picture, as a percentage of
    /// 1.0. See [`crate::display::Gamma`].
    #[serde(default)]
    pub gamma: crate::display::Gamma,
}

fn default_frustum_culling() -> bool {
    true
}

/// See [`Graphics::pvs_culling`]: on, having cleared the screenshot comparison.
fn default_pvs_culling() -> bool {
    true
}

/// How the picture itself is drawn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graphics {
    /// Which adapter the game draws with: `default`, or one by name.
    ///
    /// **Read once, at boot.** The device is made from it and never remade, so
    /// changing this lands on the next launch. See [`crate::adapter`], which is
    /// also where the answer to "can it render on the CPU" lives: only if the
    /// system has a software driver installed, and then it is on this list like
    /// any other adapter.
    #[serde(default)]
    pub renderer: crate::display::Renderer,
    /// What percentage of the displayed size the game is rendered at.
    ///
    /// Below 100 is the usual internal-resolution knob; above it is
    /// supersampling. Measured against the aspect rectangle rather than the
    /// window, so it means the same thing whatever `display.aspect` is. See
    /// [`crate::display::Scale`].
    #[serde(default)]
    pub render_scale: crate::display::Scale,
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
    /// fresh every frame by `upscale::Framebuffer::resolve`, the same as
    /// `upscaler` is, and moving among those three takes effect the frame
    /// they were chosen on. Moving to or from `msaa4x` takes effect the next
    /// time a race is launched, because that is what rebuilds the pipelines
    /// it is a property of. See [`crate::display::AntiAliasing`] and
    /// `docs/architecture/adr/0013-anti-aliasing-architecture.md`.
    #[serde(default)]
    pub anti_aliasing: crate::display::AntiAliasing,
    /// Anisotropic filtering level for track and ship textures: `off`, `2x`,
    /// `4x`, `8x` or `16x`.
    ///
    /// Defaults to the highest level. Anisotropy is the one texture-filtering
    /// setting worth exposing at all - see `mesh_render::mip_chain` - and a
    /// user turns it down for performance, never up, so defaulting low would
    /// leave most of that work unused out of the box.
    #[serde(with = "AnisotropyDef", default)]
    pub anisotropy: Anisotropy,
    /// How wide the camera's field of view is, as a percentage of the one the
    /// disc's own data authored. See [`crate::display::Fov`].
    #[serde(default)]
    pub fov: crate::display::Fov,
    /// The performance overlay: `off`, `fps`, `pacing` or `dev`.
    ///
    /// Off by default, because it is a diagnostic and not decoration. See
    /// [`crate::perf`] for what each live mode shows and why `pacing` exists at
    /// all - an average frame rate cannot show uneven frames, which is the
    /// thing a player actually sees.
    ///
    /// In this table rather than `[display]` because it is drawn *into* the
    /// frame, at the render scale, over whatever stage is running - measuring a
    /// frame nobody is presenting is the one way to get it wrong.
    #[serde(default)]
    pub perf_overlay: crate::perf::Overlay,
    /// Whether the track's draw calls are tested against the camera's view
    /// frustum before being submitted, skipping the ones entirely outside it.
    ///
    /// **On by default.** The test itself has a real, unconditional cost -
    /// measured on `16_Track`, about 59 microseconds a frame to check all
    /// ~2,000 draw calls - and whether the GPU-submission saving is worth more
    /// than that depends on the GPU: a pixel-identical screenshot with the
    /// setting on versus off confirms it changes nothing about what is drawn,
    /// but a discrete GPU with headroom to spare may not see the saving,
    /// while a weaker or integrated one measurably does. Defaulting on is the
    /// bet that the second case is the more common one to default for; no
    /// menu row, since turning it back off is a one-line settings-file edit
    /// for whoever's hardware disagrees. See the frustum-culling entry in
    /// `docs/overview/roadmap.md`.
    #[serde(default = "default_frustum_culling")]
    pub frustum_culling: bool,
    /// Whether the track's authored potentially-visible set culls draw calls
    /// before the frustum test sees them.
    ///
    /// **On by default**, for a reason [`Self::frustum_culling`] cannot claim:
    /// the visibility itself is not this project's guess. Every track ships a
    /// 64-bit mask per section saying what is visible from it, authored by the
    /// people who built the circuit, and this reads that rather than deriving
    /// anything. Measured over both discs, a section sees a mean of 7.6 of 64.
    ///
    /// What *is* this project's own is which section a render mesh belongs to -
    /// sections attach to spline control points, not to meshes, and no read of
    /// either binary has recovered the association. `oag_render::pvs` therefore
    /// decides it, by intersecting each draw call's bounding sphere with the
    /// authored boxes and keeping every section it reaches.
    ///
    /// So the bar this had to clear before defaulting on was the one frustum
    /// culling cleared: **a screenshot comparison showing it changes nothing
    /// about what is drawn.** Thirty captures - three tracks, several tick
    /// counts, every combination of the two tiers - come out byte-identical to
    /// culling nothing. An earlier and cheaper association rule, placing a draw
    /// call by the single section containing its centre, did **not**: it lost
    /// scenery wider than a section, which is recorded in `oag_render::pvs` so
    /// the cheaper rule is not reintroduced as an optimisation.
    ///
    /// **The saving is modest and track-dependent.** Measured across all 40
    /// PSP track files, the first tier removes 22% to 66% of the frustum test's
    /// input - about half on a median track - and in the worst *section* of
    /// nearly every track it removes nothing. The limit is this renderer's own
    /// batching, a `DrawCall` being one material run spanning 6 to 14 sections,
    /// not the authored data. See
    /// `docs/architecture/adr/0011-authored-pvs-before-frustum-culling.md`.
    #[serde(default = "default_pvs_culling")]
    pub pvs_culling: bool,
    /// Whether a track draws every child of an authored `LodGroup`, or only
    /// the higher-detail first one. Not a quality tier and not distance-based,
    /// see [`Lod`] for why. `both` matches the original, duplicate geometry
    /// included; `single` is a load-time choice that removes it.
    ///
    /// Defaults to `both`: this changes what is drawn, unlike a texture filter
    /// or an overlay, so the out-of-the-box behaviour matches the original
    /// rather than opting a player into a divergence they did not ask for.
    #[serde(with = "LodDef", default)]
    pub lod: Lod,
    /// Whether trackside surfaces animate, or stay frozen at their authored
    /// texture coordinates.
    ///
    /// **On by default, because it is now a reproduction rather than a
    /// guess.** Each animated material carries its own keyframed
    /// `TEXSCALE`/`TEXOFFSET` track in the `.vex` file: key times in 60 Hz
    /// frames, values in 1/256 units, an authored loop period and a step flag.
    /// The renderer replays those, the way `TexAnim_UpdateTransform`
    /// (`0x08927204`) does. See
    /// `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`.
    ///
    /// It was off while the animation was inferred instead: surfaces picked
    /// out by banded texture rows and narrow-V-band geometry, scrolled at
    /// chosen rates. That reading was not merely unproven, it was wrong on the
    /// axis - `col_display7_GLOW`, on all twelve circuits, authors a **U**
    /// scroll where the table drove it in V.
    ///
    /// What it still does, off, is freeze every animated surface at the start
    /// of its track, which is what a still-frame comparison against a capture
    /// of the original wants.
    ///
    /// **It freezes the scenery that *moves*, not only the scenery that
    /// scrolls, and the name no longer says so.** Since the `Anim Transform`
    /// class was ported (2026-08-18) this switch gates both mechanisms off one
    /// clock, because a still-frame comparison wants the whole circuit held
    /// still and not half of it. The name is kept for now rather than migrated,
    /// since it is a persisted key in everyone's settings file - but a `false`
    /// here is now a much bigger hammer than it was when it was written, and a
    /// file written before 2026-08-11 carries `false` from back when that was
    /// the default. That is the first thing to check if a circuit looks static.
    ///
    /// The ships are deliberately not covered by this switch: their blink is
    /// measured in a frame-accurate capture, so it animates either way.
    /// Whether the recovered bloom post-process runs.
    ///
    /// **Defaults off, and the reason is honesty rather than taste.** The
    /// passes are a faithful port - every constant is read out of `BOOT.BIN`,
    /// see `oag_render::post::bloom` - but the *magnitude* reaching the picture
    /// is not yet calibrated: measured on one boost frame it quadruples the
    /// blown-out area (4,381 to 17,578 fully white pixels) and erases the
    /// craft, where the original's shows the hull plainly with the plume as two
    /// distinct wing spikes.
    ///
    /// The suspect is how much the glow mask accumulates rather than the bloom
    /// arithmetic: `oag_render::exhaust::BLEND` weights alpha `SrcAlpha`/`One`,
    /// so overlapping plume fins build the mask to saturation, while the
    /// original's `GU_FIX 0xffffff` destination factor carries **no alpha**,
    /// which would make it a replace. That is a PSP blend-semantics question
    /// this project has not settled. Until it is, defaulting this on would ship
    /// a known-wrong picture to make a recovered subsystem visible.
    #[serde(default = "default_bloom")]
    pub bloom: bool,
    /// How much crossing a speed pad widens the field of view for a moment.
    ///
    /// **[`crate::display::BoostFovKick::DEFAULT`] by default, and an authored
    /// effect rather than a recovered one.** The force it rides on *is*
    /// recovered; this exists so the player can feel it, and it is not being
    /// fitted to the original. It is a magnitude rather than a constant
    /// precisely because it is invented - somebody comparing against a capture
    /// of the original wants [`crate::display::BoostFovKick::OFF`], and
    /// somebody who finds the default too subtle wants a stronger tier.
    ///
    /// The original's own field of view is *not* static: it carries
    /// `authored + 0.075 * dot(fwd, vel)`, additive degrees driven by **forward
    /// speed** - not the "shake" this comment used to call it, which was a
    /// separate `Hud_Update` term the two measurements had been conflating.
    /// That widen is recovered and **ported**, as `crate::race::SPEED_FOV_GAIN_DEG`.
    ///
    /// This setting is the other thing: a boost-gated tangent multiplier that is
    /// ours by choice, composed on top. [`crate::display::BoostFovKick::OFF`] is
    /// still the setting for a comparison against the original, because it takes
    /// *our* effect out while leaving the recovered one in.
    #[serde(default = "default_boost_fov_kick")]
    pub boost_fov_kick: crate::display::BoostFovKick,

    /// Which of the original's three in-race camera perspectives to fly with.
    ///
    /// **The one setting on this page a race also writes.** SELECT cycles the
    /// view mid-race exactly as the original's does, and the handler saves the
    /// new value straight back here, so a player who found their preferred view
    /// with the button keeps it next launch without ever opening a menu. That is
    /// the original's own behaviour: it sets its profile's dirty flag on every
    /// cycle - see `docs/ghidra/functions/psp-pulse-usa/camera.md`.
    ///
    /// The order the button cycles is recovered at confidence 88; **which view a
    /// fresh install starts on is not**, and
    /// [`crate::display::CameraView::default`] documents that choice.
    #[serde(default = "default_camera_view")]
    pub camera_view: crate::display::CameraView,
}

/// See [`Graphics::boost_fov_kick`]: **[`crate::display::BoostFovKick::DEFAULT`]**,
/// so the boost is felt. Off is for comparing against a capture of the
/// original, which does not have *this* effect.
fn default_boost_fov_kick() -> crate::display::BoostFovKick {
    crate::display::BoostFovKick::DEFAULT
}

/// See [`Graphics::camera_view`]: the further of the two chase views, which is
/// what the game rendered before the other two existed and what every capture
/// under `data/traces/` was taken with. A choice, not a reading - see
/// [`crate::display::CameraView`].
fn default_camera_view() -> crate::display::CameraView {
    crate::display::CameraView::default()
}

/// See [`Graphics::bloom`]: **off** until its magnitude is calibrated.
fn default_bloom() -> bool {
    false
}

impl Default for Graphics {
    fn default() -> Self {
        Self {
            renderer: crate::display::Renderer::default(),
            render_scale: crate::display::Scale::default(),
            upscaler: crate::display::Upscaler::default(),
            upscale_sharpness: crate::display::Sharpness::default(),
            anti_aliasing: crate::display::AntiAliasing::default(),
            anisotropy: Anisotropy::default(),
            fov: crate::display::Fov::default(),
            perf_overlay: crate::perf::Overlay::default(),
            frustum_culling: default_frustum_culling(),
            pvs_culling: default_pvs_culling(),
            lod: Lod::default(),
            bloom: default_bloom(),
            boost_fov_kick: default_boost_fov_kick(),
            camera_view: default_camera_view(),
        }
    }
}

/// Where the settings file lives: `<config dir>/oag/settings.toml`.
///
/// `None` on a platform `dirs` cannot place a config directory on, in which
/// case [`load`] falls back to defaults with nothing to persist them to.
pub fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("oag").join("settings.toml"))
}

const HEADER: &str = "\
# OpenAntiGrav settings.
#
# Rewritten on every run with every recognised key present, so this file is
# always a complete reference for what can go in it. A value you set is kept;
# a key that's missing (an older version, a fresh install, a typo'd table
# name) is added back with its default. Comments other than this header are
# not preserved across a rewrite.

";

/// The keys that used to live in `[graphics]` and now live in `[display]`.
///
/// Kept as a list rather than folded into [`migrate`] so the one thing a reader
/// needs from that function - *which* keys moved - is a line they can read.
const MOVED_TO_DISPLAY: [&str; 5] = [
    "window_mode",
    "window_size",
    "aspect",
    "vsync",
    "frame_limit",
];

/// Moves any of [`MOVED_TO_DISPLAY`] a file still holds in `[graphics]` over to
/// `[display]`.
///
/// Every one of these was a `[graphics]` key before the split, and serde
/// ignores a table field it does not know: without this, a player who had set
/// borderless and 120 Hz would open the game windowed at 240 with no message
/// and nothing wrong in the file. The next [`save`] writes the file back in the
/// new shape, so this runs once per install in practice.
///
/// **`[display]` wins where both have a key.** That is the case where the file
/// has already been migrated and something - a hand edit, a copy from an older
/// machine - put the old key back; the new spelling is the one the player last
/// saw on the menu.
///
/// Anything not on that list is left alone, so a `[graphics]` that still has
/// `anisotropy` or `render_scale` is not touched: those did not move.
fn migrate(table: &mut toml::Table) {
    // Checked *before* anything is taken out of `[graphics]`, so a file this
    // gives up on is left exactly as it was rather than half-moved. A
    // `display` that is not a table is given up on rather than replaced:
    // deserialising is about to report it with the key name, which is more use
    // than a silent overwrite.
    if table.get("display").is_some_and(|value| !value.is_table()) {
        return;
    }
    let Some(graphics) = table
        .get_mut("graphics")
        .and_then(toml::Value::as_table_mut)
    else {
        return;
    };
    let moved: Vec<(String, toml::Value)> = MOVED_TO_DISPLAY
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
    let display = table
        .entry("display")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .expect("checked on the way in, and only ever inserted as a table");
    for (key, value) in moved {
        display.entry(key).or_insert(value);
    }
}

/// Loads settings from [`path`], creating the file - or filling in whatever
/// keys it is missing - with defaults.
///
/// A malformed value is an error rather than a silently discarded default: a
/// typo should be visible, not swallowed. Absence is not malformed, so a
/// missing file or config directory is created rather than reported.
pub fn load() -> Result<Settings> {
    let Some(path) = path() else {
        return Ok(Settings::default());
    };

    let on_disk = match std::fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
    };

    let settings: Settings = match &on_disk {
        Some(text) => {
            let mut table: toml::Table = text
                .parse()
                .with_context(|| format!("parsing {}", path.display()))?;
            migrate(&mut table);
            table
                .try_into()
                .with_context(|| format!("parsing {}", path.display()))?
        }
        None => Settings::default(),
    };

    let canonical = format!(
        "{HEADER}{}",
        toml::to_string_pretty(&settings).context("serialising settings")?
    );
    if on_disk.as_deref() != Some(canonical.as_str()) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        std::fs::write(&path, &canonical).with_context(|| format!("writing {}", path.display()))?;
    }

    Ok(settings)
}

/// What every menu row that edits a setting should currently read.
///
/// One list, used by the live menus and by `--menu-page`'s debugging view, so
/// the picture the flag draws is the picture a player would see rather than
/// whatever each row's list happened to start on. Keeping it here rather than in
/// `menu.rs` is the same seam as everywhere else: the menus never read settings,
/// and this is the settings side handing them over.
///
/// `anisotropy` is passed rather than read off `settings` because the command
/// line can override it for one run, and the menus should show what is in
/// effect.
#[must_use]
pub fn menu_seeds(
    settings: &Settings,
    anisotropy: Anisotropy,
) -> Vec<(&'static str, crate::menu::Value)> {
    let text = |value: &str| crate::menu::Value::Text(value.to_string());
    let mut out = vec![
        (
            "display.monitor",
            text(&settings.display.monitor.to_string()),
        ),
        (
            "display.window_mode",
            text(&settings.display.window_mode.to_string()),
        ),
        (
            "display.window_size",
            text(&settings.display.window_size.to_string()),
        ),
        ("controls.scheme", text(&settings.controls.scheme)),
        ("controls.triggers", text(&settings.controls.triggers)),
        (
            "controls.trigger_sensitivity",
            text(&settings.controls.trigger_sensitivity.to_string()),
        ),
        ("display.aspect", text(&settings.display.aspect.to_string())),
        ("display.vsync", text(&settings.display.vsync.to_string())),
        (
            "display.frame_limit",
            text(&settings.display.frame_limit.to_string()),
        ),
        (
            "display.brightness",
            text(&settings.display.brightness.to_string()),
        ),
        ("display.gamma", text(&settings.display.gamma.to_string())),
        (
            "graphics.renderer",
            text(&settings.graphics.renderer.to_string()),
        ),
        (
            "graphics.render_scale",
            text(&settings.graphics.render_scale.to_string()),
        ),
        (
            "graphics.upscaler",
            text(&settings.graphics.upscaler.to_string()),
        ),
        (
            "graphics.upscale_sharpness",
            text(&settings.graphics.upscale_sharpness.to_string()),
        ),
        (
            "graphics.anti_aliasing",
            text(&settings.graphics.anti_aliasing.to_string()),
        ),
        ("graphics.anisotropy", text(&anisotropy.to_string())),
        ("graphics.fov", text(&settings.graphics.fov.to_string())),
        (
            "graphics.perf_overlay",
            text(&settings.graphics.perf_overlay.to_string()),
        ),
        ("graphics.lod", text(&settings.graphics.lod.to_string())),
        (
            "graphics.boost_fov_kick",
            text(&settings.graphics.boost_fov_kick.to_string()),
        ),
        (
            "graphics.camera_view",
            text(&settings.graphics.camera_view.to_string()),
        ),
        (
            "audio.music_volume",
            text(&settings.audio.music_volume.to_string()),
        ),
        (
            "audio.music_source",
            text(&settings.audio.music_source.to_string()),
        ),
        ("race.mode", text(&settings.race.mode)),
        ("race.class", text(&settings.race.class)),
        ("race.team", text(&settings.race.team)),
        ("race.track", text(&settings.race.track)),
        ("ai.difficulty", text(&settings.ai.difficulty)),
    ];
    if let Some(language) = &settings.language {
        out.push(("language", text(language)));
    }
    out
}

/// Writes `settings` back to [`path`], creating the directory if it is missing.
///
/// Called when a menu changes something, so a setting survives the run it was
/// changed in. Silently does nothing on a platform with no config directory,
/// which is the same thing [`load`] does there and for the same reason: there
/// is nowhere to put it, and that is not the player's problem to be told about
/// on every keypress.
///
/// # Errors
///
/// Propagates a directory that cannot be created and a file that cannot be
/// written.
pub fn save(settings: &Settings) -> Result<()> {
    let Some(path) = path() else {
        return Ok(());
    };
    let text = format!(
        "{HEADER}{}",
        toml::to_string_pretty(settings).context("serialising settings")?
    );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests;
