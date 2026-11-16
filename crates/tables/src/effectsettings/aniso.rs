//! HD's Zone aniso palette: the 17-texel ramp a stage's five rim colours build.
//!
//! `Environment_UpdateStageBlend` rebuilds four of these on each stage commit,
//! through `Environment_BuildAnisoPalette` (`0x003d9970`), into the textures the
//! shader publishes as `zoneAnisoPalette` and `zoneAnisoPaletteOuter`; the Zone
//! arenas' materials read them at `pow(rim, zoneAnisoPower)`. Read statically and
//! confirmed live; see `docs/ghidra/functions/ps3-hdfury-eu/zone-palette-textures.md`,
//! sections 4 and 5.

use super::EffectSettings;

/// The stage keys one group's ramp reads, as the schema spells them after the
/// stage prefix. `Base Colour` and `Base Colour Highlight` are
/// [`super::key::SCENE_BASE_COLOUR`] and its siblings.
pub mod key {
    /// The ramp's first point: texel 0, the head-on colour.
    pub const SCENE_NEAR_COLOUR: &str = "Scene.Near Colour";
    /// The ramp's third point, texel 8. A point of its own, not a midpoint.
    pub const SCENE_BASE_COLOUR_MIDDLE: &str = "Scene.Base Colour Middle";
    /// The ramp's fifth point, texel 16, the silhouette colour.
    pub const SCENE_BASE_COLOUR_HIGHLIGHT_MIDDLE: &str = "Scene.Base Colour Highlight Middle";
    /// The ramp's interior spacing; see [`super::ramp`].
    pub const SCENE_ANISO_CURVE: &str = "Scene.Aniso Curve";
    /// The track group's [`SCENE_NEAR_COLOUR`].
    pub const TRACK_NEAR_COLOUR: &str = "Track.Near Colour";
    /// The track group's [`SCENE_BASE_COLOUR_MIDDLE`].
    pub const TRACK_BASE_COLOUR_MIDDLE: &str = "Track.Base Colour Middle";
    /// The track group's [`SCENE_BASE_COLOUR_HIGHLIGHT_MIDDLE`].
    pub const TRACK_BASE_COLOUR_HIGHLIGHT_MIDDLE: &str = "Track.Base Colour Highlight Middle";
    /// The track group's [`SCENE_ANISO_CURVE`].
    pub const TRACK_ANISO_CURVE: &str = "Track.Aniso Curve";
    /// The exponent of the track group's lookup, `zoneAnisoPower` in the
    /// track publication (`0x00c814e0`).
    pub const TRACK_ANISO_POWER: &str = "Track.Aniso Power";
}

/// Texels in one ramp: four segments of four, plus the last point.
pub const TEXELS: usize = 17;

/// Which publication a ramp belongs to: the `Scene.*` keys or the `Track.*`
/// ones, chosen per chunk by the same bit that picks the stage texture set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// The `Scene.*` keys, `zoneAnisoPalette` beside `zoneMode<n>.gtf`.
    Scene,
    /// The `Track.*` keys, beside `zoneModeTrack<n>.gtf`.
    Track,
}

/// One group's ramp inputs at one stage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnisoGroup {
    /// `Near Colour`, `Base Colour`, `Base Colour Middle`, `Base Colour
    /// Highlight`, `Base Colour Highlight Middle`, in texel order (stage record
    /// `+0x10 +0x40 +0x50 +0x20 +0x30` for Scene, `+0x70 +0xa0 +0xb0 +0x80
    /// +0x90` for Track).
    pub points: [[f32; 3]; 5],
    /// `Aniso Curve`.
    pub curve: f32,
    /// `Aniso Power`, the lookup's exponent.
    pub power: f32,
}

impl AnisoGroup {
    /// The 17 texels this group's ramp holds.
    #[must_use]
    pub fn ramp(&self) -> [[f32; 3]; TEXELS] {
        ramp(self.points, self.curve)
    }
}

/// `Environment_BuildAnisoPalette`'s law.
///
/// For each segment `i` of four: texel `4i` is `P[i]`, texels `4i + 1..=3` are
/// `P[i] + (P[i+1] - P[i]) * w` at `w = curve / 2`, `0.5`, `1 - curve / 2`;
/// texel 16 is `P[4]`. `0.5` and `1.0` are the TOC constants at `0x008b79b8` and
/// `0x008b7974`. The original then stores each lane as FP16.
#[must_use]
pub fn ramp(points: [[f32; 3]; 5], curve: f32) -> [[f32; 3]; TEXELS] {
    let a = curve * 0.5;
    let weights = [0.0, a, 0.5, 1.0 - a];
    let mut out = [[0.0; 3]; TEXELS];
    for segment in 0..4 {
        let from = points[segment];
        let to = points[segment + 1];
        for (step, w) in weights.iter().enumerate() {
            // Two multiplies and an add, not `mul_add`, per
            // `docs/architecture/determinism.md`.
            out[segment * 4 + step] =
                std::array::from_fn(|lane| from[lane] + (to[lane] - from[lane]) * w);
        }
    }
    out[TEXELS - 1] = points[4];
    out
}

impl EffectSettings {
    /// One group's ramp inputs at `stage`, or `None` unless the stage authors
    /// all five points, the curve and the power.
    ///
    /// HD's base `zonemode.effectsettings` authors none of the Middle, Near or
    /// curve keys; its runtime then builds the ramp from whatever the stage
    /// record held, which varied by stage in the live read. Only the four Zone
    /// arenas compile the shader that reads the ramp, and their
    /// `zonemodedlc3.effectsettings` authors every key on every stage, so a
    /// missing key draws nothing rather than a value no file wrote.
    #[must_use]
    pub fn aniso_group(&self, stage: u32, group: Group) -> Option<AnisoGroup> {
        use super::key as base;
        let (near, colour, middle, highlight, highlight_middle, curve, power) = match group {
            Group::Scene => (
                key::SCENE_NEAR_COLOUR,
                base::SCENE_BASE_COLOUR,
                key::SCENE_BASE_COLOUR_MIDDLE,
                base::SCENE_BASE_COLOUR_HIGHLIGHT,
                key::SCENE_BASE_COLOUR_HIGHLIGHT_MIDDLE,
                key::SCENE_ANISO_CURVE,
                base::SCENE_ANISO_POWER,
            ),
            Group::Track => (
                key::TRACK_NEAR_COLOUR,
                base::TRACK_BASE_COLOUR,
                key::TRACK_BASE_COLOUR_MIDDLE,
                base::TRACK_BASE_COLOUR_HIGHLIGHT,
                key::TRACK_BASE_COLOUR_HIGHLIGHT_MIDDLE,
                key::TRACK_ANISO_CURVE,
                key::TRACK_ANISO_POWER,
            ),
        };
        let point = |name| self.stage_rgb(stage, name);
        Some(AnisoGroup {
            points: [
                point(near)?,
                point(colour)?,
                point(middle)?,
                point(highlight)?,
                point(highlight_middle)?,
            ],
            curve: self.stage_scalar(stage, curve)?,
            power: self.stage_scalar(stage, power)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The live read's shape: at curve `0.5` a segment's interior is the even
    /// quarter spacing, and the five points land on texels 0, 4, 8, 12, 16.
    #[test]
    fn the_ramp_puts_the_five_points_on_every_fourth_texel_and_quarters_between() {
        let points = [
            [0.0, 0.0, 0.0],
            [0.619_608, 0.0, 2.0],
            [0.0, 4.0, 1.0],
            [0.0, 1.482_353, 7.0],
            [0.0, 5.0, 5.0],
        ];
        let texels = ramp(points, 0.5);
        for (k, p) in points.iter().enumerate() {
            assert_eq!(texels[4 * k], *p, "point {k}");
        }
        // The live stage-4 Scene ramp read these at FP16: 0.1549, 0.3098, 0.4646.
        let near = |got: [f32; 3], want: [f32; 3]| {
            assert!(
                got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-5),
                "{got:?} against {want:?}"
            );
        };
        near(texels[1], [0.154_902, 0.0, 0.5]);
        near(texels[2], [0.309_804, 0.0, 1.0]);
        near(texels[3], [0.464_706, 0.0, 1.5]);
    }

    /// `Aniso Curve` moves the first and third interior texels and never the
    /// middle one: `curve / 2`, `0.5`, `1 - curve / 2`.
    #[test]
    fn the_curve_moves_the_outer_interior_texels_and_not_the_middle() {
        let points = [[0.0; 3], [1.0; 3], [1.0; 3], [1.0; 3], [1.0; 3]];
        let at_zero = ramp(points, 0.0);
        assert_eq!(
            [at_zero[1][0], at_zero[2][0], at_zero[3][0]],
            [0.0, 0.5, 1.0]
        );
        let at_one = ramp(points, 1.0);
        assert_eq!([at_one[1][0], at_one[2][0], at_one[3][0]], [0.5, 0.5, 0.5]);
    }

    const STAGE: &str = r#""1 Sub Venom.Scene.Near Colour"=0.1 0.2 0.3
"1 Sub Venom.Scene.Base Colour"=1 1 1
"1 Sub Venom.Scene.Base Colour Middle"=2 2 2
"1 Sub Venom.Scene.Base Colour Highlight"=3 3 3
"1 Sub Venom.Scene.Base Colour Highlight Middle"=4 4 4
"1 Sub Venom.Scene.Aniso Curve"=0.5
"1 Sub Venom.Scene.Aniso Power"=0.8
"1 Sub Venom.Track.Base Colour"=1 1 1
"#;

    #[test]
    fn a_group_reads_its_five_points_in_texel_order() {
        let table = EffectSettings::parse(STAGE).expect("parses");
        let scene = table.aniso_group(1, Group::Scene).expect("all authored");
        assert_eq!(scene.points[0], [0.1, 0.2, 0.3]);
        assert_eq!(scene.points[2], [2.0, 2.0, 2.0]);
        assert_eq!(scene.points[3], [3.0, 3.0, 3.0]);
        assert_eq!(scene.points[4], [4.0, 4.0, 4.0]);
        assert_eq!((scene.curve, scene.power), (0.5, 0.8));
    }

    #[test]
    fn a_group_missing_any_key_builds_no_ramp() {
        let table = EffectSettings::parse(STAGE).expect("parses");
        assert_eq!(table.aniso_group(1, Group::Track), None);
    }
}
