//! Persisted graphics settings.
//!
//! Read from `<config dir>/oag/settings.toml` on startup and rewritten with
//! every recognised key filled in, so the file on disk always documents what
//! it can hold rather than starting empty or silently dropping a key an older
//! version never wrote. A CLI flag still wins over whatever is on disk for
//! that one run; see `--anisotropy` in `main.rs`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use oag_render::mesh_render::Anisotropy;

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
    pub graphics: Graphics,
    #[serde(default)]
    pub source: Source,
    #[serde(default)]
    pub race: Race,
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
            class: default_class(),
            team: default_team(),
            track: default_track(),
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
}

/// Vsync's default, spelled once so [`Graphics::default`] and serde's
/// `default` cannot disagree about it.
///
/// **Off**, with [`crate::perf::FrameLimit::DEFAULT`] doing the bounding
/// instead. This is a racing game: vsync's cost is a frame of latency, which is
/// the one thing a player steering at 60 Hz feels directly, and the limiter
/// gives back most of what vsync was there for without it. A player who wants
/// the tear-free presentation turns it on, and the limit row greys out because
/// the display is then deciding.
fn default_vsync() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graphics {
    /// Anisotropic filtering level for track and ship textures: `off`, `2x`,
    /// `4x`, `8x` or `16x`.
    ///
    /// Defaults to the highest level. Anisotropy is the one texture-filtering
    /// setting worth exposing at all - see `mesh_render::mip_chain` - and a
    /// user turns it down for performance, never up, so defaulting low would
    /// leave most of that work unused out of the box.
    #[serde(with = "AnisotropyDef", default)]
    pub anisotropy: Anisotropy,
    /// The shape the game is drawn at inside its window: `psp`, `ps2` or
    /// `free`. See [`crate::display::Aspect`].
    #[serde(default)]
    pub aspect: crate::display::Aspect,
    /// `windowed` or `borderless`. See [`crate::display::WindowMode`].
    #[serde(default)]
    pub window_mode: crate::display::WindowMode,
    /// What percentage of the displayed size the game is rendered at.
    ///
    /// Below 100 is the usual internal-resolution knob; above it is
    /// supersampling. Measured against the aspect rectangle rather than the
    /// window, so it means the same thing whatever `aspect` is. See
    /// [`crate::display::Scale`].
    #[serde(default)]
    pub render_scale: crate::display::Scale,
    /// How big a *windowed* window is, spelled `1440x816`.
    ///
    /// A window property, not a graphics one in the sense the others are:
    /// borderless ignores it entirely, because the display decides. It sits in
    /// this table anyway because that is where a player looks for it.
    #[serde(default)]
    pub window_size: crate::display::Size,
    /// The performance overlay: `off`, `fps` or `pacing`.
    ///
    /// Off by default, because it is a diagnostic and not decoration. See
    /// [`crate::perf`] for what the two live modes show and why the second one
    /// exists at all - an average frame rate cannot show uneven frames, which
    /// is the thing a player actually sees.
    #[serde(default)]
    pub perf_overlay: crate::perf::Overlay,
    /// Whether the surface waits for the display's refresh.
    ///
    /// **Off by default**, because this is a racing game and vsync's cost is a
    /// frame of latency. Turning it on is what a player does about tearing, and
    /// doing so hands the pacing to the display - which is why it greys
    /// [`Graphics::frame_limit`] out rather than stacking with it.
    #[serde(default = "default_vsync")]
    pub vsync: bool,
    /// How many frames a second the loop may produce when vsync is off:
    /// `unlimited`, or a rate up to 1000. Defaults to 240.
    ///
    /// **Ignored entirely while `vsync` is on**, where the display decides. See
    /// [`crate::perf::FrameLimit`].
    #[serde(default)]
    pub frame_limit: crate::perf::FrameLimit,
}

impl Default for Graphics {
    /// Written out rather than derived, because vsync's default is `true` and
    /// a derived one would be `false` - silently, and only for the field where
    /// it matters most.
    fn default() -> Self {
        Self {
            anisotropy: Anisotropy::default(),
            aspect: crate::display::Aspect::default(),
            window_mode: crate::display::WindowMode::default(),
            render_scale: crate::display::Scale::default(),
            window_size: crate::display::Size::default(),
            perf_overlay: crate::perf::Overlay::default(),
            vsync: default_vsync(),
            frame_limit: crate::perf::FrameLimit::default(),
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
            toml::from_str(text).with_context(|| format!("parsing {}", path.display()))?
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
        ("graphics.anisotropy", text(&anisotropy.to_string())),
        (
            "graphics.aspect",
            text(&settings.graphics.aspect.to_string()),
        ),
        (
            "graphics.render_scale",
            text(&settings.graphics.render_scale.to_string()),
        ),
        (
            "graphics.window_mode",
            text(&settings.graphics.window_mode.to_string()),
        ),
        (
            "graphics.window_size",
            text(&settings.graphics.window_size.to_string()),
        ),
        (
            "graphics.perf_overlay",
            text(&settings.graphics.perf_overlay.to_string()),
        ),
        (
            "graphics.vsync",
            crate::menu::Value::Flag(settings.graphics.vsync),
        ),
        (
            "graphics.frame_limit",
            text(&settings.graphics.frame_limit.to_string()),
        ),
        ("race.class", text(&settings.race.class)),
        ("race.team", text(&settings.race.team)),
        ("race.track", text(&settings.race.track)),
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
mod tests {
    use super::*;

    /// The default circuit has to be the one every capture was taken on, or a
    /// fresh install races something a trace comparison cannot be run against.
    ///
    /// Note what is *not* asserted: that the id composes into a path. It does
    /// not - only the source can say which `.vex` an id loads, which is
    /// `catalogue`'s job and is why `Race` carries no layout field.
    #[test]
    fn the_default_circuit_is_the_reference_scenarios_own() {
        let race = Race::default();
        assert_eq!(race.track, "16_Track");
        assert!(
            crate::race::DEFAULT_TRACK.contains(&race.track),
            "{} is not the circuit {} names",
            race.track,
            crate::race::DEFAULT_TRACK
        );
    }

    /// A settings file from a build that predates the race table must still
    /// load, with the table filled in rather than the parse failing.
    #[test]
    fn an_older_settings_file_gains_the_race_table() {
        let settings: Settings = toml::from_str("[graphics]\nanisotropy = \"4x\"").expect("parse");
        assert_eq!(settings.race.track, "16_Track");
        assert_eq!(settings.race.class, "venom");
    }

    /// The one field whose default is not its type's own, asserted from both
    /// directions.
    ///
    /// `Graphics` has a hand-written `Default` precisely because a derived one
    /// would make `vsync` false by accident rather than on purpose - which
    /// happens to be the same value, and that is exactly why it needs pinning:
    /// the day the default flips, a derive would silently disagree with
    /// `default_vsync` and only the file-loading path would notice.
    #[test]
    fn the_two_ways_of_getting_a_default_agree() {
        let fresh = Graphics::default();
        let loaded: Settings = toml::from_str("").expect("parse");
        assert_eq!(fresh.vsync, default_vsync());
        assert_eq!(loaded.graphics.vsync, fresh.vsync);
        assert_eq!(loaded.graphics.frame_limit, fresh.frame_limit);
        assert_eq!(
            fresh.frame_limit,
            crate::perf::FrameLimit::DEFAULT,
            "a fresh install should be limited, not unlimited"
        );
    }
}
