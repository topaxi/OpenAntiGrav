//! A mod's own particle effect: a parametric TOML file that plays through the same
//! [`System`] a disc `.pob` does (`schemas/v1/effect.tosd`).
//!
//! The file mirrors [`EmitterSpec`] in its units (ticks, world units per tick,
//! degrees), so a disc effect could be written in it, and builds an [`Effect`] with
//! every emitter a root: no child emitters, no spawn shapes, no weather box. What a
//! file cannot say it cannot get wrong; what it says wrongly is refused by name and
//! the effect then draws nothing, never a guess.
//!
//! Chosen, not measured: the format is ours. The laws it plays (emission, ageing,
//! curves, blend) are the disc effects' own.

use serde::Deserialize;

use oag_core::math::Vec3;
use oag_pob::{Channel, ChannelMode};

use super::{
    Atlas, Blend, ColourMode, ColourScale, Direction, Effect, EmitterSpec, FrameAdvance,
    MAX_EMITTER_STATES, Render, Spawn, Sprite, StreakDraw, playback,
};

/// `[toml-schema]` kind and major.
pub const SCHEMA_KIND: &str = "effect";
/// See [`SCHEMA_KIND`].
pub const SCHEMA_MAJOR: u32 = 1;

/// The ceilings a file is held to: a mod's effect must stay cheap to play.
const MAX_LIVE_CAP: usize = 512;
const MAX_PER_EMISSION: u32 = 64;
const MAX_LIFETIME_TICKS: f32 = 600.0;
const MAX_DURATION_TICKS: f32 = 600.0;
const MAX_KEYS: usize = 32;

/// Which way an effect's cone axis points when a host fires it at a craft's nozzle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Axis {
    /// Away from the craft's nose.
    #[default]
    Rear,
    /// Up out of the craft.
    Up,
}

/// A parsed effect file: the effect and how to aim it.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    /// The playable effect.
    pub effect: Effect,
    /// Where its cone axis points on a craft.
    pub axis: Axis,
}

/// Why a file is not an effect.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError(String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ParseError {}

fn bad<T>(what: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError(what.into()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(rename = "toml-schema")]
    header: Option<oag_schema::Header>,
    #[serde(default)]
    axis: Axis,
    #[serde(default)]
    emitter: Vec<Emitter>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Emitter {
    name: Option<String>,
    /// Ticks the emitter emits for.
    duration: f64,
    #[serde(default)]
    looping: bool,
    /// Ticks between emissions, `[min, max]` or one number.
    #[serde(default = "one_pair")]
    interval: Pair,
    /// Particles per emission, `[min, max]` or one number.
    #[serde(default = "one_pair", rename = "per-emission")]
    per_emission: Pair,
    /// Lifetime in ticks, `[centre, spread]`.
    lifetime: [f64; 2],
    /// Ejection speed per tick, `[centre, spread]`.
    #[serde(default)]
    speed: [f64; 2],
    /// The cone's half angle in degrees.
    #[serde(default)]
    spread: f64,
    /// Per-tick velocity decay, `1` for none.
    #[serde(default = "one")]
    drag: f64,
    /// Downward acceleration, world units per tick squared.
    #[serde(default)]
    gravity: f64,
    #[serde(default = "default_cap", rename = "live-cap")]
    live_cap: usize,
    #[serde(default)]
    blend: BlendName,
    /// The half-size over a particle's life, in world units.
    size: Curve,
    /// The opacity over a particle's life, `0..=1`.
    alpha: Curve,
    /// `"#rrggbb"`, or `[[age, "#rrggbb"], ...]` over a particle's life.
    colour: Colour,
    /// A PNG of the mod (a path), or `{ entry = "<circuit .vex>", name = "<texture>" }`,
    /// a texture of a circuit on the player's disc; drawn on every particle. None draws
    /// the soft round default.
    texture: Option<TextureName>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum TextureName {
    Mod(String),
    Host { entry: String, name: String },
}

/// Which picture an emitter wants, for the caller to find.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureRef<'a> {
    /// A PNG of the mod, by its path in the mod.
    Mod(&'a str),
    /// A `Texture` node of a circuit of the player's disc: the lookup a track file's
    /// host references use.
    Host {
        /// The circuit's entry name.
        entry: &'a str,
        /// The texture's name.
        name: &'a str,
    },
}

#[derive(Deserialize, Clone, Copy)]
#[serde(untagged)]
enum Pair {
    One(u32),
    Two([u32; 2]),
}

#[derive(Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum BlendName {
    #[default]
    Additive,
    AlphaOver,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Curve {
    Constant(f64),
    Keys(Vec<[f64; 2]>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Colour {
    One(String),
    Stops(Vec<(f64, String)>),
}

fn one() -> f64 {
    1.0
}

fn one_pair() -> Pair {
    Pair::One(1)
}

fn default_cap() -> usize {
    64
}

impl Pair {
    fn range(self) -> (u32, u32) {
        match self {
            Self::One(n) => (n, n),
            Self::Two([lo, hi]) => (lo, hi),
        }
    }
}

/// Parses `text`. `texture` finds a picture an emitter names, or says why it cannot.
///
/// # Errors
///
/// A [`ParseError`] naming the first thing wrong; the caller draws nothing.
pub fn parse(
    text: &str,
    texture: &mut dyn FnMut(&TextureRef<'_>) -> Result<Sprite, String>,
) -> Result<Parsed, ParseError> {
    let file: File = toml::from_str(text).map_err(|e| ParseError(e.to_string()))?;
    oag_schema::check(file.header.as_ref(), SCHEMA_KIND, SCHEMA_MAJOR)
        .map_err(|e| ParseError(e.to_string()))?;
    if file.emitter.is_empty() {
        return bad("no [[emitter]]");
    }
    if file.emitter.len() > MAX_EMITTER_STATES {
        return bad(format!(
            "{} emitters, at most {MAX_EMITTER_STATES}",
            file.emitter.len()
        ));
    }
    let mut emitters = Vec::new();
    for (index, emitter) in file.emitter.iter().enumerate() {
        let label = emitter
            .name
            .clone()
            .unwrap_or_else(|| format!("emitter {index}"));
        emitters.push(
            build(emitter, &label, texture).map_err(|e| ParseError(format!("{label}: {e}")))?,
        );
    }
    let roots = Effect::roots_of(&emitters);
    Ok(Parsed {
        effect: Effect {
            name: "mod effect".to_string(),
            emitters,
            field: None,
            view_depth: 0.0,
            roots,
            skipped_templates: 0,
        },
        axis: file.axis,
    })
}

fn finite(value: f64, what: &str) -> Result<f32, ParseError> {
    if value.is_finite() {
        Ok(value as f32)
    } else {
        bad(format!("{what} is not a finite number"))
    }
}

fn build(
    e: &Emitter,
    label: &str,
    texture: &mut dyn FnMut(&TextureRef<'_>) -> Result<Sprite, String>,
) -> Result<EmitterSpec, ParseError> {
    let duration = finite(e.duration, "duration")?;
    if !(0.0..=MAX_DURATION_TICKS).contains(&duration) || (!e.looping && duration < 1.0) {
        return bad(format!(
            "duration {duration} is not 1 to {MAX_DURATION_TICKS} ticks"
        ));
    }
    let interval = e.interval.range();
    let per_emission = e.per_emission.range();
    if interval.0 < 1 || interval.1 < interval.0 {
        return bad(format!("interval {interval:?}: at least 1, min <= max"));
    }
    if per_emission.1 < per_emission.0 || per_emission.1 > MAX_PER_EMISSION {
        return bad(format!(
            "per-emission {per_emission:?}: min <= max <= {MAX_PER_EMISSION}"
        ));
    }
    let lifetime = (
        finite(e.lifetime[0], "lifetime")?,
        finite(e.lifetime[1], "lifetime")?,
    );
    if lifetime.0 < 1.0
        || lifetime.1 < 0.0
        || lifetime.1 > lifetime.0
        || lifetime.0 + lifetime.1 > MAX_LIFETIME_TICKS
    {
        return bad(format!(
            "lifetime {lifetime:?}: centre at least 1, 0 <= spread <= centre, at most {MAX_LIFETIME_TICKS}"
        ));
    }
    let speed = (finite(e.speed[0], "speed")?, finite(e.speed[1], "speed")?);
    let spread = finite(e.spread, "spread")?;
    if !(0.0..=180.0).contains(&spread) {
        return bad(format!("spread {spread} is not 0 to 180 degrees"));
    }
    let drag = finite(e.drag, "drag")?;
    if !(0.0..=1.0).contains(&drag) {
        return bad(format!("drag {drag} is not 0 to 1"));
    }
    if e.live_cap == 0 || e.live_cap > MAX_LIVE_CAP {
        return bad(format!(
            "live-cap {} is not 1 to {MAX_LIVE_CAP}",
            e.live_cap
        ));
    }
    let size = curve(&e.size, "size", 1.0)?;
    let alpha = curve(&e.alpha, "alpha", 255.0)?;
    let sprite = match &e.texture {
        None => None,
        Some(name) => {
            let wanted = match name {
                TextureName::Mod(path) => TextureRef::Mod(path),
                TextureName::Host { entry, name } => TextureRef::Host { entry, name },
            };
            match texture(&wanted) {
                Ok(sprite) => Some(sprite),
                Err(why) => return bad(format!("texture {wanted:?}: {why}")),
            }
        }
    };
    Ok(EmitterSpec {
        name: label.to_string(),
        duration_ticks: duration,
        looping: e.looping,
        interval_ticks: interval,
        per_emission,
        lifetime_ticks: lifetime,
        speed_per_tick: speed,
        direction: Direction::Cone {
            half_angle: spread.to_radians(),
        },
        drag_per_tick: Vec3::splat(drag),
        gravity_per_tick2: finite(e.gravity, "gravity")?,
        live_cap: e.live_cap,
        size,
        alpha,
        colour_divisor: ColourScale::Full.divisor(),
        palette: palette(&e.colour)?,
        colour_mode: ColourMode::OverLife,
        render: Render::Billboard,
        streak: StreakDraw::Procedural,
        blend: match e.blend {
            BlendName::Additive => Blend::Additive,
            BlendName::AlphaOver => Blend::AlphaOver,
        },
        distort_strength: 0.0,
        particle_child: None,
        death_child: None,
        spawn_probability: 1.0,
        velocity_inherit: 0.0,
        spawn: Spawn::Point,
        emission_scale: constant(1.0),
        extent_animation: None,
        playback: playback::Playback::default(),
        sprite,
        atlas: Atlas::SINGLE,
        frames: FrameAdvance::Still,
        sheet_rect: None,
        short_run: false,
        world_space: false,
        template: false,
        rotation: None,
    })
}

fn constant(value: f32) -> Channel {
    Channel {
        period: 0.0,
        mode: ChannelMode::Constant,
        lo: 0.0,
        hi: value,
        keys: Vec::new(),
    }
}

/// A curve as a channel, its values times `unit` (the alpha channel is in bytes).
fn curve(curve: &Curve, what: &str, unit: f32) -> Result<Channel, ParseError> {
    match curve {
        Curve::Constant(value) => {
            let value = finite(*value, what)?;
            if value < 0.0 {
                return bad(format!("{what} {value} is negative"));
            }
            Ok(constant(value * unit))
        }
        Curve::Keys(keys) => {
            if keys.is_empty() || keys.len() > MAX_KEYS {
                return bad(format!("{what}: 1 to {MAX_KEYS} keys"));
            }
            let mut last = -1.0_f32;
            let mut out = Vec::new();
            for key in keys {
                let (age, value) = (finite(key[0], what)?, finite(key[1], what)?);
                if !(0.0..=1.0).contains(&age) || age <= last {
                    return bad(format!(
                        "{what}: key ages ascend strictly within 0 to 1 (at {age})"
                    ));
                }
                if value < 0.0 {
                    return bad(format!("{what}: value {value} is negative"));
                }
                last = age;
                out.push((age, value));
            }
            Ok(Channel {
                period: 0.0,
                mode: ChannelMode::Keyframed,
                lo: 0.0,
                hi: unit,
                keys: out,
            })
        }
    }
}

fn hex(text: &str) -> Result<[f32; 3], ParseError> {
    let digits = text.strip_prefix('#').filter(|d| d.len() == 6);
    let Some(digits) = digits else {
        return bad(format!("colour {text:?} is not \"#rrggbb\""));
    };
    let byte = |at: usize| {
        u8::from_str_radix(&digits[at..at + 2], 16)
            .map(|b| f32::from(b) / 255.0)
            .or_else(|_| bad(format!("colour {text:?} is not \"#rrggbb\"")))
    };
    Ok([byte(0)?, byte(2)?, byte(4)?])
}

/// The gradient baked to the 256 entries a particle walks over its life.
fn palette(colour: &Colour) -> Result<Box<[[f32; 4]; 256]>, ParseError> {
    let stops: Vec<(f32, [f32; 3])> = match colour {
        Colour::One(text) => vec![(0.0, hex(text)?)],
        Colour::Stops(stops) => {
            if stops.is_empty() || stops.len() > MAX_KEYS {
                return bad(format!("colour: 1 to {MAX_KEYS} stops"));
            }
            let mut last = -1.0_f32;
            let mut out = Vec::new();
            for (age, text) in stops {
                let age = finite(*age, "colour")?;
                if !(0.0..=1.0).contains(&age) || age <= last {
                    return bad(format!(
                        "colour: stop ages ascend strictly within 0 to 1 (at {age})"
                    ));
                }
                last = age;
                out.push((age, hex(text)?));
            }
            out
        }
    };
    let mut table = Box::new([[0.0; 4]; 256]);
    for (index, entry) in table.iter_mut().enumerate() {
        let age = index as f32 / 255.0;
        let rgb = match stops.iter().position(|(stop, _)| *stop >= age) {
            None => stops[stops.len() - 1].1,
            Some(0) => stops[0].1,
            Some(next) => {
                let ((t0, c0), (t1, c1)) = (stops[next - 1], stops[next]);
                let mix = (age - t0) / (t1 - t0);
                std::array::from_fn(|i| c0[i] + (c1[i] - c0[i]) * mix)
            }
        };
        *entry = [rgb[0], rgb[1], rgb[2], 1.0];
    }
    Ok(table)
}

#[cfg(test)]
mod tests;
