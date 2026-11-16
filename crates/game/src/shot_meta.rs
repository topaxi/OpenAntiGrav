//! What a screenshot of a race carries so the shot can be made again (`docs/formats/shot-png.md`).
//!
//! Two chunks, written by [`encode`] and read by [`read`]:
//!
//! - an `iTXt` chunk, keyword [`KEYWORD`] (`oag`): [`Meta`] as TOML, for a person (a bug report
//!   reads it with `oag-game --png-info`) and for the flags that draw the frame again;
//! - the private ancillary chunk [`oag_replay::shot::CHUNK`] (`oaGs`): the run's human input
//!   stream, the launch request, the seed and the state hash at the shot's tick
//!   ([`oag_replay::ShotRecord`]), which `oag-game --from-png` re-simulates.
//!
//! **No player name or tag and no filesystem path is written**: the disc is named by its serial,
//! a track by its archive entry, and a mod by its id.

use oag_replay::ShotRecord;
use serde::{Deserialize, Serialize};

/// The `iTXt` keyword.
pub const KEYWORD: &str = "oag";

/// The first line of the text, and the version of its layout.
pub const FORMAT: u32 = 1;

/// A screenshot's description. Every section but `shot`, `build` and `reproduce` is optional: a
/// source that does not know a fact leaves it out rather than guessing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Meta {
    /// The layout version of this text.
    pub format: u32,
    /// The frame.
    pub shot: Shot,
    /// The title, its disc and its platform.
    pub title: Title,
    /// The race.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race: Option<Race>,
    /// The camera the frame was drawn from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub camera: Option<Camera>,
    /// The player's craft in the frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub craft: Option<Craft>,
    /// The render settings in force: the title's render profile, as the settings file spells it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub render: Option<toml::Table>,
    /// The adapter that drew it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu: Option<Gpu>,
    /// The mods in force.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mods: Vec<Mod>,
    /// The build.
    pub build: Build,
    /// How to make the frame again.
    pub reproduce: Reproduce,
}

/// The frame itself.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shot {
    /// The world tick the frame was drawn at.
    pub tick: u64,
    /// The race's state hash at that tick, as `0x` and sixteen hex digits.
    pub state_hash: String,
    /// The picture's size.
    pub width: u32,
    /// See [`Self::width`].
    pub height: u32,
    /// How the frame was taken (`photo mode`, `screenshot`, `drive script`).
    pub source: String,
    /// Whether the `oaGs` chunk carries the input stream; when not, the text says why.
    pub stream: bool,
    /// Why there is no stream, when there is none.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub no_stream_because: String,
}

/// The title, its disc and its platform.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Title {
    /// The title as the engine names it.
    pub name: String,
    /// The disc's serial, which names the pressing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial: Option<String>,
    /// The console the title shipped on.
    pub platform: String,
}

/// The race the frame is of.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Race {
    /// The circuit's archive entry.
    pub track: String,
    /// The race mode.
    pub mode: String,
    /// The speed class.
    pub class: String,
    /// The team whose craft was flown.
    pub team: String,
    /// The world generator's seed, as its bit pattern in hex.
    pub seed: String,
}

/// A camera: where it is, which way it looks, how wide.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Camera {
    /// Which camera: the photo lens, or `race` for the one the race draws from.
    pub lens: String,
    /// The eye, world space.
    pub eye: [f32; 3],
    /// The look direction.
    pub forward: [f32; 3],
    /// Up.
    pub up: [f32; 3],
    /// The vertical field of view in degrees, when the camera states one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fov_deg: Option<f32>,
}

/// The player's craft.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Craft {
    /// The position, world space.
    pub position: [f32; 3],
    /// The orientation as a quaternion `[x, y, z, w]`.
    pub rotation: [f32; 4],
}

/// The adapter that drew the frame.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Gpu {
    /// The adapter's name.
    pub adapter: String,
    /// The graphics API.
    pub backend: String,
    /// The driver, when the adapter names one.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub driver: String,
    /// The driver's own description.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub driver_info: String,
}

/// A mod in force.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Mod {
    /// Its id.
    pub id: String,
    /// Its release.
    pub version: String,
}

/// The build that drew the frame.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Build {
    /// The short commit hash, or `unknown` for a build made outside a checkout.
    pub commit: String,
    /// The crate version and suffix.
    pub version: String,
}

/// How to make the frame again.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reproduce {
    /// Re-simulates the stream and draws the frame: exact, where `shot.stream` is true.
    pub exact: String,
    /// The flags that draw this race and camera without the stream. A frame of a human-flown
    /// race is only as exact as the pose: the craft is put where it was, not driven there.
    pub flags: Vec<String>,
}

impl Build {
    /// This build.
    #[must_use]
    pub fn this() -> Self {
        Self {
            commit: oag_net::GIT_HASH.to_string(),
            version: oag_net::BUILD_VERSION.to_string(),
        }
    }
}

impl Meta {
    /// The text, a `# oag screenshot` line first.
    #[must_use]
    pub fn to_text(&self) -> String {
        let body = toml::to_string_pretty(self).expect("a Meta always serialises");
        format!("# oag screenshot metadata; read with `oag-game --png-info`\n{body}")
    }

    /// Reads a text [`Self::to_text`] wrote.
    ///
    /// # Errors
    ///
    /// Text that is not a [`Meta`].
    pub fn from_text(text: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(text)
    }
}

/// A hash as the text spells it.
#[must_use]
pub fn hex(hash: u64) -> String {
    format!("{hash:#018x}")
}

/// The `oag-game` flags that draw `race` from `camera` with the craft where it was: the shape
/// of a compare-page pair's `ours.flags`. `--pose` puts the craft at its position on the track's
/// own heading, so a craft that was not flying along the circuit is only approximated; the
/// stream is what is exact.
#[must_use]
pub fn flags(
    race: &Race,
    camera: Option<&Camera>,
    craft: Option<&Craft>,
    size: (u32, u32),
) -> Vec<String> {
    let mut flags: Vec<String> = ["--race", "--no-audio", "--track", &race.track]
        .iter()
        .chain(
            [
                "--mode",
                &race.mode,
                "--class",
                &race.class,
                "--team",
                &race.team,
            ]
            .iter(),
        )
        .map(|flag| (*flag).to_string())
        .collect();
    if race.track.is_empty() {
        flags.drain(2..4);
    }
    let triple = |v: [f32; 3]| format!("{},{},{}", v[0], v[1], v[2]);
    if let Some(craft) = craft {
        flags.extend(["--pose".to_string(), triple(craft.position)]);
    }
    if let Some(camera) = camera {
        flags.extend([
            "--camera-pose".to_string(),
            format!(
                "{},{}",
                triple(camera.eye),
                format!("{},{}", triple(camera.forward), triple(camera.up))
            ),
        ]);
        if let Some(fov) = camera.fov_deg {
            flags.extend(["--camera-fov".to_string(), fov.to_string()]);
        }
    }
    flags.extend(["--size".to_string(), format!("{}x{}", size.0, size.1)]);
    flags
}

/// The command line that made a capture, for the text: the arguments as they were given, less
/// `--screenshot` and `--source` and every pair whose value is a filesystem path, so the text
/// names none.
#[must_use]
pub fn own_flags(args: impl IntoIterator<Item = String>) -> Vec<String> {
    let path_like = |arg: &str| {
        arg.starts_with('/')
            || arg.starts_with('~')
            || arg.starts_with("./")
            || arg.starts_with("../")
            || arg.as_bytes().get(1..3) == Some(b":\\")
    };
    let args: Vec<String> = args.into_iter().collect();
    let mut flags = Vec::new();
    let mut at = 0;
    while at < args.len() {
        let arg = &args[at];
        let value = args.get(at + 1).filter(|next| !next.starts_with("--"));
        if matches!(arg.as_str(), "--screenshot" | "--source") {
            at += 1 + usize::from(value.is_some());
        } else if arg.starts_with("--") && value.is_some_and(|value| path_like(value)) {
            at += 2;
        } else if path_like(arg) {
            at += 1;
        } else {
            flags.push(arg.clone());
            at += 1;
        }
    }
    flags
}

/// A camera's description from the pose the renderer draws with.
#[must_use]
pub fn camera_of(lens: &str, pose: &oag_raceplay::CameraOverride) -> Camera {
    let v = |v: oag_core::math::Vec3| [v.x, v.y, v.z];
    Camera {
        lens: lens.to_string(),
        eye: v(pose.eye),
        forward: v(pose.orientation * oag_core::math::Vec3::NEG_Z),
        up: v(pose.orientation * oag_core::math::Vec3::Y),
        fov_deg: pose.fov_deg,
    }
}

/// The frame as a PNG with its metadata: `meta` as `iTXt`, and `record` as `oaGs`.
///
/// `pixels` is RGBA8888 as read back; the alpha is made opaque, as every capture's is.
#[must_use]
pub fn encode(
    width: u32,
    height: u32,
    pixels: &[u8],
    meta: &Meta,
    record: Option<&ShotRecord>,
) -> Vec<u8> {
    let mut pixels = pixels.to_vec();
    oag_mesh::capture::make_opaque(&mut pixels);
    let mut extra = vec![(
        oag_texture::png::ITXT,
        oag_texture::png::itxt(KEYWORD, &meta.to_text()),
    )];
    if let Some(record) = record {
        extra.push((oag_replay::shot::CHUNK, record.to_bytes()));
    }
    oag_texture::png::encode_rgba_with(width, height, &pixels, &extra)
}

/// What a PNG carries of ours.
#[derive(Debug, Default)]
pub struct Found {
    /// The `iTXt` text, as written.
    pub text: Option<String>,
    /// The text parsed.
    pub meta: Option<Meta>,
    /// The input stream and hash.
    pub record: Option<ShotRecord>,
}

/// Why a PNG could not be read.
#[derive(Debug)]
pub enum ReadError {
    /// Not a whole PNG.
    Png(oag_texture::png::ChunkError),
    /// The `iTXt` text is not ours.
    Text(toml::de::Error),
    /// The `oaGs` chunk does not read.
    Record(oag_replay::shot::ShotError),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Png(why) => write!(f, "{why}"),
            Self::Text(why) => write!(f, "the oag text chunk does not parse: {why}"),
            Self::Record(why) => write!(f, "{why}"),
        }
    }
}

impl std::error::Error for ReadError {}

/// Reads our chunks out of `png`. A PNG without them reads as [`Found::default`].
///
/// # Errors
///
/// [`ReadError`] for a damaged file or a chunk of ours that does not parse.
pub fn read(png: &[u8]) -> Result<Found, ReadError> {
    let mut found = Found::default();
    for (kind, data) in oag_texture::png::chunks(png).map_err(ReadError::Png)? {
        if kind == oag_texture::png::ITXT
            && let Some(text) = oag_texture::png::itxt_text(data, KEYWORD)
        {
            found.meta = Some(Meta::from_text(text).map_err(ReadError::Text)?);
            found.text = Some(text.to_string());
        } else if kind == oag_replay::shot::CHUNK {
            found.record = Some(ShotRecord::from_bytes(data).map_err(ReadError::Record)?);
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests;
