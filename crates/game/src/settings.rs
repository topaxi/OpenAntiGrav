//! Persisted settings.
//!
//! Read from `<config dir>/oag/settings.toml` on startup and rewritten with
//! every recognised key filled in, so the file on disk always documents what
//! it can hold rather than starting empty or silently dropping a key an older
//! version never wrote. A CLI flag still wins over whatever is on disk for
//! that one run; see `--anisotropy` in `main.rs`.
//!
//! # Display against graphics against render profiles
//!
//! Three tables:
//!
//! - **`[display]`** is the picture's container and how it reaches a screen:
//!   which monitor, what kind of window, how big, what shape, how a finished
//!   frame is presented and what happens to it on the way out.
//! - **`[graphics]`** is how the picture is drawn, for whichever title the
//!   value applies to equally: how the textures are filtered, how much of
//!   the world is in frame.
//! - **`[render_profiles.<title> (<platform>)]`** is the subset of "how the
//!   picture is drawn" whose right default trades off against how expensive
//!   that *particular source's* own scene is to render: resolution,
//!   upscaling, anti-aliasing, motion blur. Pure and Pulse's PSP/PS2-era
//!   scenes and HD/Fury/2048's real lighting and higher poly counts are not
//!   the same cost to draw, and this is kept **one profile per (title,
//!   original platform) pair**, not a shared value or a two-way
//!   "classic/modern" grouping: a grouping bakes in a guess about relative
//!   cost that does not hold even within a pair - Pulse authors 129 dynamic
//!   shadow-occluder hulls across 83 WADs, Pure authors none, despite being
//!   "the same era". The platform half of the key exists for the same
//!   reason one level up: Pulse's PSP and PS2 releases are different
//!   scenes - different geometry payloads, different textures, the PS2's
//!   own engine flare - so the title alone is not a fine enough key.
//!   Opening a source switches which profile the menus read and write, with
//!   nothing to explain in the UI, because only one source is ever open at
//!   once - see [`RenderProfile`] and [`profile_key`].
//!
//! The line between `[display]` and `[graphics]` is *whether the renderer
//! would notice*. Turning off vsync changes nothing about the frame that is
//! drawn, only about when it is shown; halving the render scale changes the
//! frame itself. Brightness and gamma sit on the display side under that
//! rule even though they are a shader: they are a monitor calibration,
//! applied after the game has finished drawing. The line between
//! `[graphics]` and `[render_profiles.<title> (<platform>)]` is *whether the
//! right default depends on which source is open*: anisotropy and field of
//! view cost about the same whatever is on screen, so they stay in
//! `[graphics]`.
//!
//! Everything was in `[graphics]` until the display split, so [`load`]
//! migrates a file that still is - see [`migrate`]. Everything
//! render-profile-shaped was still flat in `[graphics]` until the next
//! split, so [`load`] migrates that too - see [`migrate_render_profiles`].
//! And every profile was keyed by title alone until *this* split, so
//! [`load`] migrates that last - see `render_profile::migrate_platform_split`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use log::warn;
use serde::{Deserialize, Serialize};

use oag_mesh::mesh_render::Anisotropy;

mod controls;
mod race;
mod render_profile;
pub mod web;

pub use controls::{Controls, TriggerSensitivity};
pub use race::{Race, Remix};
pub use render_profile::{RenderProfile, SCREEN_FILTER_OFF, profile_key};
use render_profile::{
    ensure_known_titles, migrate_platform_split, migrate_reconstruction_keys,
    migrate_render_profiles,
};

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub display: Display,
    #[serde(default)]
    pub graphics: Graphics,
    /// The render-cost-sensitive slice of [`Graphics`], one entry per
    /// `(title, original platform)` pair, keyed by [`profile_key`]. See the
    /// module doc's "Display against graphics against render profiles"
    /// section for why this is its own table rather than a field on
    /// `Graphics`, and for why the key carries the platform too.
    ///
    /// A `BTreeMap` rather than `HashMap` so the canonical rewrite
    /// [`load`]/[`save`] write is stable across runs - an unordered map's
    /// iteration order is not something this file should depend on.
    ///
    /// [`ensure_known_titles`] fills in every title this build links, so the
    /// file documents all of them the same way every `Graphics` field is
    /// always present regardless of whether a player touched it - a title
    /// with no entry yet reads as [`RenderProfile::default`].
    #[serde(default)]
    pub render_profiles: BTreeMap<String, RenderProfile>,
    #[serde(default)]
    pub audio: oag_sound::settings::Settings,
    #[serde(default)]
    pub source: Source,
    #[serde(default)]
    pub race: Race,
    #[serde(default)]
    pub remix: Remix,
    #[serde(default)]
    pub controls: Controls,
    #[serde(default)]
    pub ai: Ai,
    #[serde(default)]
    pub log: Log,
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

/// Where the game's log file goes and how much goes into it. Settings file
/// only: no menu row, and `--log-file` overrides `file` for one run.
///
/// Both keys are **absent until a player writes them**, and the rewrite
/// [`load`] does leaves them absent: writing the resolved default path back
/// would freeze it, and a later `$XDG_STATE_HOME` would stop applying.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Log {
    /// Absent: `$XDG_STATE_HOME/oag/logs/oag-game.log` (see `oag_log::path`).
    /// A path: that file. **Set but empty: no log file at all**, the one
    /// switch for turning it off. Always appended to; entries older than seven
    /// days are pruned at startup.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// The file's own filter, in `RUST_LOG` syntax. Absent:
    /// [`oag_log::tool::FILE_FILTER`]. The terminal is untouched by it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
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
    /// usual places"; see [`oag_source::source::resolve_dlc`].
    #[serde(default)]
    pub dlc: Vec<String>,
}

/// Which screen the game opens on, what kind of window it opens, and how a
/// finished frame gets from the renderer onto it.
///
/// Nothing in here changes what is drawn - see the module documentation for
/// where the line against [`Graphics`] falls and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Display {
    /// Which screen to open on: `default`, or a monitor by name. See
    /// [`oag_display::display::Monitor`].
    ///
    /// A name this machine does not have falls back to the default with a note
    /// rather than failing, because the usual way to get one is to unplug a
    /// screen.
    #[serde(default)]
    pub monitor: oag_display::display::Monitor,
    /// `windowed` or `borderless`. See [`oag_display::display::WindowMode`].
    #[serde(default)]
    pub window_mode: oag_display::display::WindowMode,
    /// How big a *windowed* window is, spelled `1440x816`.
    ///
    /// Means nothing in borderless, where the display decides.
    #[serde(default)]
    pub window_size: oag_display::display::Size,
    /// How big the web build's canvas is: `fit` (the whole page) or a size in
    /// CSS pixels, `1280x720`. Means nothing on a desktop, where
    /// [`Self::window_size`] is the window; a page has no window, so the web
    /// build offers this row instead of that one. See [`web::CanvasSize`].
    #[serde(
        default = "default_canvas_size",
        skip_serializing_if = "web::not_on_the_web"
    )]
    pub canvas_size: String,
    /// The shape the game is drawn at inside its window: `psp`, `ps2` or
    /// `free`. See [`oag_display::display::Aspect`].
    #[serde(default)]
    pub aspect: oag_display::display::Aspect,
    /// Which of a source's own front-end stylings to draw: `HD` or `FURY` on
    /// Wipeout HD, which is the only title in hand that has two.
    ///
    /// **The disc's own axis, not this build's.** HD's `OPT_FE_STYLE` offers
    /// exactly these two values and the disc ships every loading-screen
    /// illustration twice, white-and-blue and black-and-red - see
    /// `oag_hd::loading::FEATURES`. A `String` rather than an enum for the
    /// reason `race.team` is one: which values exist is a property of the
    /// source, and a value this source does not offer falls back to its first
    /// with a note rather than failing the boot.
    ///
    /// Empty means "whatever the source leads with", which is every title
    /// except HD and is HD's own base styling.
    #[serde(default)]
    pub front_end_style: String,
    /// How a finished frame reaches the display: `off`, `on` or `smooth`.
    ///
    /// **`off` by default**, because this is a racing game and vsync's cost is
    /// a frame of latency. `smooth` is what "triple buffering" means on a
    /// modern API - no tearing and no half-rate cliff, at the price of the
    /// frames it discards. See [`oag_present::perf::Vsync`].
    ///
    /// A file that still says `vsync = true` or `false` is read, and rewritten
    /// as a name.
    #[serde(default)]
    pub vsync: oag_present::perf::Vsync,
    /// How many frames a second the loop may produce: `unlimited`, or a rate up
    /// to 1000. Defaults to 240.
    ///
    /// **Ignored under `vsync = "on"`**, where the display decides - and only
    /// there. Under `smooth` the loop is not blocked, so without this it
    /// renders frames the compositor throws away. See
    /// [`oag_present::perf::FrameLimit`].
    #[serde(default)]
    pub frame_limit: oag_present::perf::FrameLimit,
    /// What the finished picture is multiplied by, as a percentage. See
    /// [`oag_display::display::Brightness`].
    #[serde(default)]
    pub brightness: oag_display::display::Brightness,
    /// The midtone curve applied to the finished picture, as a percentage of
    /// 1.0. See [`oag_display::display::Gamma`].
    #[serde(default)]
    pub gamma: oag_display::display::Gamma,
    /// Whether a race pauses when the window loses focus (`true`) or runs on
    /// under whatever took the focus (`false`).
    ///
    /// **On by default - chosen, not measured.** A race keeps going while the
    /// player is looking at something else, and an alt-tab, the Steam Deck's
    /// overlay or a notification costs a crash and a lost lap; paused, it
    /// costs one press to resume. A player with a second monitor who wants it
    /// live turns it off. Minimising the window and Android's `Suspended`
    /// always pause, whatever this says.
    #[serde(default = "default_pause_on_focus_loss")]
    pub pause_on_focus_loss: bool,
}

fn default_canvas_size() -> String {
    "fit".to_string()
}

fn default_pause_on_focus_loss() -> bool {
    true
}

impl Default for Display {
    fn default() -> Self {
        Self {
            monitor: Default::default(),
            window_mode: Default::default(),
            window_size: Default::default(),
            canvas_size: default_canvas_size(),
            aspect: Default::default(),
            front_end_style: String::new(),
            vsync: Default::default(),
            frame_limit: Default::default(),
            brightness: Default::default(),
            gamma: Default::default(),
            pause_on_focus_loss: default_pause_on_focus_loss(),
        }
    }
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
    pub renderer: oag_display::display::Renderer,
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
    /// disc's own data authored. See [`oag_display::display::Fov`].
    #[serde(default)]
    pub fov: oag_display::display::Fov,
    /// The performance overlay: `off`, `fps`, `pacing` or `dev`.
    ///
    /// Off by default, because it is a diagnostic and not decoration. See
    /// [`oag_present::perf`] for what each live mode shows and why `pacing` exists at
    /// all - an average frame rate cannot show uneven frames, which is the
    /// thing a player actually sees.
    ///
    /// In this table rather than `[display]` because it is drawn *into* the
    /// frame, at the render scale, over whatever stage is running - measuring a
    /// frame nobody is presenting is the one way to get it wrong.
    #[serde(default)]
    pub perf_overlay: oag_present::perf::Overlay,
    /// How a raster HUD is stretched to the screen: `integer`,
    /// `sharp-bilinear`, `nearest` or `linear`.
    ///
    /// **Read only by the titles whose HUD is raster art** (Pulse on the PSP
    /// and PS2, Pure - `oag_title::HudArt::raster`); HD, 2048 and Omega draw
    /// as before whatever it says. Taken when a race loads. No menu row: it is
    /// a one-line settings-file choice, the way [`Self::frustum_culling`] is.
    /// See [`oag_display::display::HudScale`] and `docs/ui/hud-skins.md`.
    #[serde(default)]
    pub hud_scale: oag_display::display::HudScale,
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
    /// How much crossing a speed pad widens the field of view for a moment.
    ///
    /// **[`oag_display::display::BoostFovKick::DEFAULT`] by default, and an authored
    /// effect rather than a recovered one.** The force it rides on *is*
    /// recovered; this exists so the player can feel it, and it is not being
    /// fitted to the original. It is a magnitude rather than a constant
    /// precisely because it is invented - somebody comparing against a capture
    /// of the original wants [`oag_display::display::BoostFovKick::OFF`], and
    /// somebody who finds the default too subtle wants a stronger tier.
    ///
    /// The original's own field of view is *not* static: it carries
    /// `authored + 0.075 * dot(fwd, vel)`, additive degrees driven by **forward
    /// speed** - not the "shake" this comment used to call it, which was a
    /// separate `Hud_Update` term the two measurements had been conflating.
    /// That widen is recovered and **ported**, as `oag_raceplay::SPEED_FOV_GAIN_DEG`.
    ///
    /// This setting is the other thing: a boost-gated tangent multiplier that is
    /// ours by choice, composed on top. [`oag_display::display::BoostFovKick::OFF`] is
    /// still the setting for a comparison against the original, because it takes
    /// *our* effect out while leaving the recovered one in.
    #[serde(default = "default_boost_fov_kick")]
    pub boost_fov_kick: oag_display::display::BoostFovKick,

    /// Which of the original's three in-race camera perspectives to fly with.
    ///
    /// **The one setting on this page a race also writes.** SELECT cycles the
    /// view mid-race exactly as the original's does, and the handler saves the
    /// new value straight back here, so a player who found their preferred view
    /// with the button keeps it next launch without ever opening a menu. That is
    /// the original's own behaviour: it sets its profile's dirty flag on every
    /// cycle - see `docs/ghidra/functions/psp-pulse-usa/camera.md`.
    ///
    /// The order the button cycles is recovered at confidence 88, and a fresh
    /// install starts on the nearer chase view because a fresh profile of the
    /// original does - see [`oag_display::display::CameraView::default`].
    #[serde(default = "default_camera_view")]
    pub camera_view: oag_display::display::CameraView,
}

/// See [`Graphics::boost_fov_kick`]: **[`oag_display::display::BoostFovKick::DEFAULT`]**,
/// so the boost is felt. Off is for comparing against a capture of the
/// original, which does not have *this* effect.
fn default_boost_fov_kick() -> oag_display::display::BoostFovKick {
    oag_display::display::BoostFovKick::DEFAULT
}

/// See [`Graphics::camera_view`]: the nearer chase view, which is what a fresh
/// profile of the original starts on (measured 2026-10-01) - see
/// [`oag_display::display::CameraView`].
fn default_camera_view() -> oag_display::display::CameraView {
    oag_display::display::CameraView::default()
}

impl Default for Graphics {
    fn default() -> Self {
        Self {
            renderer: oag_display::display::Renderer::default(),
            anisotropy: Anisotropy::default(),
            fov: oag_display::display::Fov::default(),
            perf_overlay: oag_present::perf::Overlay::default(),
            hud_scale: oag_display::display::HudScale::default(),
            frustum_culling: default_frustum_culling(),
            pvs_culling: default_pvs_culling(),
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
    crate::profile::config_dir().map(|dir| dir.join("oag").join("settings.toml"))
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
/// `anisotropy` or `fov` is not touched: those never moved out of
/// `[graphics]` at all. `render_scale` and its render-profile siblings did
/// move, but to `[render_profiles.<title>]` rather than `[display]` - see
/// [`migrate_render_profiles`], a separate function because it moves keys to
/// a table per title rather than to one fixed table.
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
///
/// A file or directory the process may not read or write (an Android
/// reinstall can restore the app's files under another owner) is not a reason
/// to refuse to boot: it logs a warning and runs on defaults, writing nothing.
pub fn load() -> Result<Settings> {
    let Some(path) = path() else {
        return Ok(Settings::default());
    };
    load_from(&path)
}

fn load_from(path: &std::path::Path) -> Result<Settings> {
    let on_disk = match crate::profile::read_to_string(path) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            warn!(
                "could not read {}: {e}; running on default settings, nothing is written back",
                path.display()
            );
            return Ok(Settings::default());
        }
    };

    let mut settings: Settings = match &on_disk {
        Some(text) => {
            let mut table: toml::Table = text
                .parse()
                .with_context(|| format!("parsing {}", path.display()))?;
            migrate(&mut table);
            migrate_render_profiles(&mut table);
            // After it, not before - see `migrate_reconstruction_keys`.
            migrate_reconstruction_keys(&mut table);
            // Last: splits whatever `[render_profiles.<title>]` migrate_render_profiles
            // just populated (or an older file already had) into one row per
            // platform - see `migrate_platform_split`'s own doc for why it runs
            // after the reconstruction fold rather than before.
            migrate_platform_split(&mut table);
            table
                .try_into()
                .with_context(|| format!("parsing {}", path.display()))?
        }
        None => Settings::default(),
    };
    ensure_known_titles(&mut settings);
    #[cfg(target_arch = "wasm32")]
    if on_disk.is_none() {
        web::first_run(&mut settings);
    }

    let canonical = format!(
        "{HEADER}{}",
        toml::to_string_pretty(&settings).context("serialising settings")?
    );
    if on_disk.as_deref() != Some(canonical.as_str()) {
        let written = path
            .parent()
            .map_or(Ok(()), crate::profile::create_dir_all)
            .and_then(|()| crate::profile::write(path, &canonical));
        if let Err(e) = written {
            warn!(
                "could not write {}: {e}; running on the settings read, nothing persists",
                path.display()
            );
        }
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
///
/// `title` and `platform` name whose [`RenderProfile`] the five
/// `graphics.*` rows below that moved into `render_profiles` should read -
/// see [`profile_key`]. Passed rather than looked up here because a caller
/// (the live menus) already has the open source's title and platform in
/// hand - there is exactly one source open at a time, so there is nothing
/// for the menus to choose between.
#[must_use]
pub fn menu_seeds(
    settings: &Settings,
    anisotropy: Anisotropy,
    title: &oag_title::Title,
    platform: oag_disc::Platform,
) -> Vec<(&'static str, oag_ui::menu::Value)> {
    let text = |value: &str| oag_ui::menu::Value::Text(value.to_string());
    let profile = settings
        .render_profiles
        .get(&profile_key(title, platform))
        .cloned()
        .unwrap_or_default();
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
            "controls.touch_go_zones",
            text(if settings.controls.touch_go_zones {
                "on"
            } else {
                "off"
            }),
        ),
        (
            "controls.pilot_assist",
            text(settings.controls.pilot_assist.name()),
        ),
        (
            "controls.touch_scheme",
            text(&settings.controls.touch_scheme),
        ),
        (
            "controls.touch_opacity",
            text(&settings.controls.touch_opacity.to_string()),
        ),
        (
            "controls.trigger_sensitivity",
            text(&settings.controls.trigger_sensitivity.to_string()),
        ),
        ("display.aspect", text(&settings.display.aspect.to_string())),
        ("display.vsync", text(&settings.display.vsync.to_string())),
        (
            "display.pause_on_focus_loss",
            text(if settings.display.pause_on_focus_loss {
                "on"
            } else {
                "off"
            }),
        ),
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
            text(&profile.render_scale.to_string()),
        ),
        ("graphics.target_fps", text(&profile.target_fps.to_string())),
        (
            "graphics.minimum_resolution",
            text(&profile.minimum_resolution.to_string()),
        ),
        (
            "graphics.reconstruction",
            text(&profile.reconstruction.to_string()),
        ),
        (
            "graphics.upscale_sharpness",
            text(&profile.upscale_sharpness.to_string()),
        ),
        ("graphics.msaa", text(&profile.msaa.to_string())),
        (
            "graphics.motion_blur",
            text(&profile.motion_blur.to_string()),
        ),
        (
            "graphics.motion_blur_resolution",
            text(&profile.motion_blur_resolution.to_string()),
        ),
        ("graphics.shadows", text(&profile.shadows.to_string())),
        (
            "graphics.model_detail",
            text(&profile.model_detail.to_string()),
        ),
        (
            "graphics.texture_detail",
            text(&profile.texture_detail.to_string()),
        ),
        ("graphics.screen_filter", text(&profile.screen_filter)),
        (
            "graphics.screen_filter_strength",
            text(&profile.screen_filter_strength.to_string()),
        ),
        ("graphics.anisotropy", text(&anisotropy.to_string())),
        ("graphics.fov", text(&settings.graphics.fov.to_string())),
        (
            "graphics.perf_overlay",
            text(&settings.graphics.perf_overlay.to_string()),
        ),
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
        // The second of the original's two volumes, and it was missing here
        // until 2026-08-24 while its row sat on the AUDIO page: an unseeded
        // `choice` opens on its list's *first* option, so SFX VOLUME read `0`
        // whatever the file said, and the first press moved a player at 100
        // straight to 25. `every_settings_row_is_one_the_game_seeds` only swept
        // the `display` and `graphics` pages, which is why nothing caught it;
        // it sweeps every page now.
        (
            "audio.sfx_volume",
            text(&settings.audio.sfx_volume.to_string()),
        ),
        (
            "audio.speech_volume",
            text(&settings.audio.speech_volume.to_string()),
        ),
        (
            "audio.master_volume",
            text(&settings.audio.master_volume.to_string()),
        ),
        (
            "audio.music_source",
            text(&settings.audio.music_source.to_string()),
        ),
        (
            "display.front_end_style",
            text(&settings.display.front_end_style),
        ),
        ("race.mode", text(&settings.race.mode)),
        ("race.class", text(&settings.race.class)),
        ("race.team", text(&settings.race.team)),
        ("race.track", text(&settings.race.track)),
        ("race.variant", text(&settings.race.variant)),
        ("race.kill_target", text(&settings.race.kill_target)),
        ("race.weapons", text(&settings.race.weapons)),
        ("ai.difficulty", text(&settings.ai.difficulty)),
        ("remix.track_title", text(&settings.remix.track_title)),
        ("remix.track", text(&settings.remix.track)),
        ("remix.craft_title", text(&settings.remix.craft_title)),
        ("remix.team", text(&settings.remix.team)),
        ("remix.variant", text(&settings.remix.variant)),
    ];
    // The web's own row; a desktop file has no such key (see `web::not_on_the_web`).
    if cfg!(target_arch = "wasm32") {
        out.push(("display.canvas_size", text(&settings.display.canvas_size)));
    }
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
        crate::profile::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    crate::profile::write(&path, text).with_context(|| format!("writing {}", path.display()))
}

#[cfg(test)]
mod tests;
