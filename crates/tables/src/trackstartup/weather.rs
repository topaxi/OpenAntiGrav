//! `<LevelFx><Weather .../>`: the rain or snow a circuit asks for.
//!
//! Two of Pulse's circuits author one, Outpost 7 (`07_Track`, snow) and
//! Fort Gale (`14_Track`, rain and its screen lens). `Weather_Construct`
//! (`0x088f184c`) loads `EnvPsys` and `ScreenPsys` by name and
//! `Weather_Update` (`0x088f1e58`) runs them; the other attributes feed the
//! wind and the mist overlay. See
//! `docs/ghidra/functions/psp-pulse-usa/weather.md`.

use crate::fexml;

/// A circuit's `<Weather>` element, every attribute as authored.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Weather {
    /// `Tex`: the mist overlay's texture, `Data\Tex\ScreenFX\Mist.mip`.
    pub tex: Option<String>,
    /// `Alpha`: the mist overlay's opacity outdoors.
    pub alpha: f32,
    /// `DisplayScale`.
    pub display_scale: f32,
    /// `TexScale`.
    pub tex_scale: f32,
    /// `AspectRatio`.
    pub aspect_ratio: f32,
    /// `EnvPsys`: the effect that rains or snows, `Data\Psys\<name>.POB`.
    pub env_psys: Option<String>,
    /// `ScreenPsys`: the lens droplets, drawn on the screen rather than in the
    /// world. Fort Gale only.
    pub screen_psys: Option<String>,
    /// `DriftY`: the fall, world units per tick, before the wind's 0.3 factor.
    pub drift_y: f32,
    /// `DriftMistMult`: the mist overlay's drift, `-(0.3 * wind)` times this.
    pub drift_mist_mult: f32,
    /// `MistInside`, an integer flag in the original (`Xml_AttributeAsInt`, so
    /// Outpost 7's authored `0.2` reads `0`, confirmed live 2026-10-04):
    /// non-zero keeps the mist overlay's opacity where it is on a cover edge,
    /// zero fades it to `0` under cover and back to `Alpha` in the open.
    pub mist_inside: f32,
    /// `WindBase`: the wind's steady strength.
    pub wind_base: f32,
    /// `WindRange`: how far the wind's noise swings it.
    pub wind_range: f32,
}

impl Weather {
    /// Reads one `<Weather>` element. An attribute that is absent reads as
    /// zero (or `None` for a name), as the original's config block is
    /// zero-filled before it parses.
    #[must_use]
    pub fn from_element(element: &fexml::Node) -> Self {
        let float = |name: &str| {
            element
                .attr(name)
                .and_then(|text| text.trim().parse().ok())
                .unwrap_or(0.0)
        };
        let text = |name: &str| {
            element
                .attr(name)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        };
        Self {
            tex: text("tex"),
            alpha: float("alpha"),
            display_scale: float("displayscale"),
            tex_scale: float("texscale"),
            aspect_ratio: float("aspectratio"),
            env_psys: text("envpsys"),
            screen_psys: text("screenpsys"),
            drift_y: float("drifty"),
            drift_mist_mult: float("driftmistmult"),
            mist_inside: float("mistinside"),
            wind_base: float("windbase"),
            wind_range: float("windrange"),
        }
    }
}

/// The name of a `Data\Psys\<name>.POB` path, as `Library` keys it.
///
/// `EnvPsys` is authored as `data\psys\WO_RAIN.POB`; the library's name is
/// `WO_RAIN`.
#[must_use]
pub fn effect_name(path: &str) -> &str {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.strip_suffix(".POB")
        .or_else(|| file.strip_suffix(".pob"))
        .unwrap_or(file)
}
