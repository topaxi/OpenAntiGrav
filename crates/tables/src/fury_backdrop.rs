//! `Data/fe/fury.envsettings`: the camera paths and colours behind the Fury
//! front end's menus.
//!
//! The one `.envsettings` on the HD disc that is not a circuit's. Its syntax
//! is [`super::envsettings`]' - `"key"=numbers`, one per line - but its keys
//! are the fields of `BackgroundAnimFury_Item.cpp`'s settings object, which
//! `FurySettings_Construct` registers against three path tables and a handful
//! of top-level values, each with a default the file may leave in place. The
//! layout, the defaults and what every field does are read on
//! [menu-backdrop.md](../../../docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md);
//! this module is the typed view of the file that page describes.
//!
//! ```text
//! "Particle Colour"=0.541176 0.023529 0.000000
//! "staticPaths[0].pointSize"=0.005000
//! "staticPaths[0].start"=3.000000 2.000000 0.000000
//! "morphPaths[0].effectMode"=9
//! "dynamicPaths[0].sections[1].duration"=10.000000
//! ```
//!
//! **Only what the file authors is returned, plus the constructor's defaults
//! for what it leaves out** - a path with no `duration` line keeps the
//! constructor's zero and the picker skips it, which is what the original
//! does with the same file.

use crate::envsettings::EnvSettings;

/// How many static paths the settings object holds.
pub const STATIC_PATHS: usize = 8;
/// How many morph paths.
pub const MORPH_PATHS: usize = 8;
/// How many dynamic paths, each of [`DYNAMIC_SECTIONS`] sections.
pub const DYNAMIC_PATHS: usize = 4;
/// The sections a dynamic path is authored in.
pub const DYNAMIC_SECTIONS: usize = 4;

/// The GPU effect mode every static path draws with - `RadioHead2`, the
/// quad-sprite mode. `FuryStaticPath_EffectMode` returns the literal.
pub const STATIC_EFFECT_MODE: u32 = 2;

/// One authored camera move over the hull, and the look it is drawn with.
///
/// Field names are the file's own. A path's `duration` of zero is the
/// constructor's default and marks a slot the file did not fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Path {
    /// The GPU effect mode: `2` for a static path, authored for the others.
    pub effect_mode: u32,
    /// The sprite's world-space half-size at the reference resolution.
    pub point_size: f32,
    /// Vertical field of view, degrees.
    pub fovy: f32,
    /// Seconds the move lasts.
    pub duration: f32,
    /// Distance at which sprites start growing, and how fast they grow past it.
    pub dof_start: f32,
    pub dof_strength: f32,
    pub dof_factor: f32,
    /// Where the fog starts, how far it runs, and its curve.
    pub fog_start: f32,
    pub fog_length: f32,
    pub fog_exponent: f32,
    /// The eye's start and end.
    pub start: [f32; 3],
    pub end: [f32; 3],
    /// The look-at target's start and end.
    pub focus_start: [f32; 3],
    pub focus_end: [f32; 3],
}

impl Default for Path {
    /// The constructor's own defaults, before the file is read: a zero
    /// duration, and the numbers a path draws with if its own lines are
    /// missing.
    fn default() -> Self {
        Self {
            effect_mode: STATIC_EFFECT_MODE,
            point_size: 0.0,
            fovy: 0.0,
            duration: 0.0,
            dof_start: 0.0,
            dof_strength: 0.0,
            dof_factor: 0.0,
            fog_start: 0.0,
            fog_length: 0.0,
            fog_exponent: 0.0,
            start: [0.0; 3],
            end: [0.0; 3],
            focus_start: [0.0; 3],
            focus_end: [0.0; 3],
        }
    }
}

impl Path {
    /// Whether the picker may choose this path: `BackgroundAnimFury_PickPath`
    /// rejects a duration at or under `1e-4`.
    #[must_use]
    pub fn is_authored(&self) -> bool {
        self.duration > 1e-4
    }
}

/// Everything `fury.envsettings` says, typed.
#[derive(Debug, Clone, PartialEq)]
pub struct FuryBackdrop {
    /// The sprite's base colour, before the brightness, music pulse and
    /// resolution scale `RadioHead2_Update` applies.
    pub particle_colour: [f32; 3],
    /// What the travelling ramp adds at its peak.
    pub particle_ramp_colour: [f32; 3],
    /// How much of the previous frame's trail survives into this one.
    pub feedback: [f32; 3],
    /// The blend pass's `uv0ScaleBias` zoom, authored zero.
    pub feedback_zoom: f32,
    /// The equaliser's smoothing and its coupling into the feedback.
    pub equaliser_damp_speed: f32,
    pub equaliser_feedback_multiplier: f32,
    /// The equaliser level the music pulse is centred on, and its gain.
    pub music_pulse_base: f32,
    pub music_pulse_factor: f32,
    /// The three path tables, in the settings object's order.
    pub static_paths: [Path; STATIC_PATHS],
    pub morph_paths: [Path; MORPH_PATHS],
    /// A dynamic path is four sections plus one effect mode; the sections
    /// carry the same fields a static path does.
    pub dynamic_paths: [DynamicPath; DYNAMIC_PATHS],
}

/// One dynamic path: an effect mode and four sections.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicPath {
    /// Authored per path; the constructor's default is `5`.
    pub effect_mode: u32,
    pub sections: [Path; DYNAMIC_SECTIONS],
}

impl Default for DynamicPath {
    fn default() -> Self {
        Self {
            effect_mode: 5,
            sections: [Path::default(); DYNAMIC_SECTIONS],
        }
    }
}

impl DynamicPath {
    /// The picker's test on a dynamic path is its first section's duration.
    #[must_use]
    pub fn is_authored(&self) -> bool {
        self.sections[0].is_authored()
    }
}

impl Default for FuryBackdrop {
    /// `FurySettings_Construct`'s defaults, before the file is read.
    fn default() -> Self {
        Self {
            particle_colour: [0.045; 3],
            particle_ramp_colour: [1.0, 1.0, 7.0],
            feedback: [0.65; 3],
            feedback_zoom: 0.0,
            equaliser_damp_speed: 0.02,
            equaliser_feedback_multiplier: 0.0,
            music_pulse_base: 0.6,
            music_pulse_factor: 1.2,
            static_paths: [Path::default(); STATIC_PATHS],
            morph_paths: [Path {
                effect_mode: 9,
                ..Path::default()
            }; MORPH_PATHS],
            dynamic_paths: [DynamicPath::default(); DYNAMIC_PATHS],
        }
    }
}

impl FuryBackdrop {
    /// Reads the file's values over the constructor's defaults.
    ///
    /// Never fails: a key the file omits keeps its default, exactly as the
    /// original's registrar leaves a field it finds no line for. A key with
    /// the wrong arity is treated as omitted rather than half-read.
    #[must_use]
    pub fn read(settings: &EnvSettings) -> Self {
        let mut out = Self::default();
        let scalar = |key: &str, into: &mut f32| {
            if let Some(v) = settings.scalar(key) {
                *into = v;
            }
        };
        let vec3 = |key: &str, into: &mut [f32; 3]| {
            if let Some(v) = settings.vec3(key) {
                *into = v;
            }
        };
        vec3("Particle Colour", &mut out.particle_colour);
        vec3("Particle Ramp Colour", &mut out.particle_ramp_colour);
        vec3("Feedback", &mut out.feedback);
        scalar("Feedback Zoom", &mut out.feedback_zoom);
        scalar("Equaliser Damp Speed", &mut out.equaliser_damp_speed);
        scalar(
            "Equaliser/Feedback multiplier",
            &mut out.equaliser_feedback_multiplier,
        );
        scalar("Music Pulse Base", &mut out.music_pulse_base);
        scalar("Music Pulse factor", &mut out.music_pulse_factor);
        for (i, path) in out.static_paths.iter_mut().enumerate() {
            read_path(settings, &format!("staticPaths[{i}]"), path);
        }
        for (i, path) in out.morph_paths.iter_mut().enumerate() {
            let prefix = format!("morphPaths[{i}]");
            read_path(settings, &prefix, path);
            if let Some(mode) = settings.scalar(&format!("{prefix}.effectMode")) {
                path.effect_mode = mode as u32;
            }
        }
        for (i, path) in out.dynamic_paths.iter_mut().enumerate() {
            let prefix = format!("dynamicPaths[{i}]");
            if let Some(mode) = settings.scalar(&format!("{prefix}.effectMode")) {
                path.effect_mode = mode as u32;
            }
            for (s, section) in path.sections.iter_mut().enumerate() {
                read_path(settings, &format!("{prefix}.sections[{s}]"), section);
                section.effect_mode = path.effect_mode;
            }
        }
        out
    }

    /// The static paths the picker may choose, by index.
    pub fn authored_static_paths(&self) -> impl Iterator<Item = usize> + '_ {
        self.static_paths
            .iter()
            .enumerate()
            .filter(|(_, path)| path.is_authored())
            .map(|(i, _)| i)
    }
}

/// The thirteen fields every path and section registers, under `prefix`.
fn read_path(settings: &EnvSettings, prefix: &str, path: &mut Path) {
    let scalar = |name: &str, into: &mut f32| {
        if let Some(v) = settings.scalar(&format!("{prefix}.{name}")) {
            *into = v;
        }
    };
    scalar("pointSize", &mut path.point_size);
    scalar("fovy", &mut path.fovy);
    scalar("duration", &mut path.duration);
    scalar("dofStart", &mut path.dof_start);
    scalar("dofStrength", &mut path.dof_strength);
    scalar("dofFactor", &mut path.dof_factor);
    scalar("fogStart", &mut path.fog_start);
    scalar("fogLength", &mut path.fog_length);
    scalar("fogExponent", &mut path.fog_exponent);
    let vec3 = |name: &str, into: &mut [f32; 3]| {
        if let Some(v) = settings.vec3(&format!("{prefix}.{name}")) {
            *into = v;
        }
    };
    vec3("start", &mut path.start);
    vec3("end", &mut path.end);
    vec3("focusStart", &mut path.focus_start);
    vec3("focusEnd", &mut path.focus_end);
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE: &str = r#""Particle Colour"=0.5 0.1 0.0
"Feedback"=0.6 0.6 0.6
"Music Pulse factor"=1.1
"staticPaths[0].pointSize"=0.005
"staticPaths[0].fovy"=40.0
"staticPaths[0].duration"=8.0
"staticPaths[0].start"=3.0 2.0 0.0
"staticPaths[0].focusEnd"=0.0 0.5 -3.0
"morphPaths[1].effectMode"=10
"morphPaths[1].duration"=6.0
"dynamicPaths[2].effectMode"=9
"dynamicPaths[2].sections[1].duration"=10.0
"#;

    fn read() -> FuryBackdrop {
        FuryBackdrop::read(&EnvSettings::parse(FILE).expect("parses"))
    }

    #[test]
    fn authored_values_land_and_omitted_ones_keep_the_constructors_defaults() {
        let read = read();
        assert_eq!(read.particle_colour, [0.5, 0.1, 0.0]);
        assert_eq!(read.feedback, [0.6; 3]);
        assert_eq!(read.music_pulse_factor, 1.1);
        assert_eq!(read.music_pulse_base, 0.6, "the default");
        assert_eq!(read.particle_ramp_colour, [1.0, 1.0, 7.0], "the default");
        let first = read.static_paths[0];
        assert_eq!(first.point_size, 0.005);
        assert_eq!(first.fovy, 40.0);
        assert_eq!(first.start, [3.0, 2.0, 0.0]);
        assert_eq!(first.focus_end, [0.0, 0.5, -3.0]);
        assert_eq!(first.end, [0.0; 3], "not authored, so zero");
        assert_eq!(first.effect_mode, STATIC_EFFECT_MODE);
    }

    #[test]
    fn only_a_path_with_a_duration_is_authored() {
        let read = read();
        assert_eq!(read.authored_static_paths().collect::<Vec<_>>(), vec![0]);
        assert!(read.morph_paths[1].is_authored());
        assert!(!read.morph_paths[0].is_authored());
        assert_eq!(read.morph_paths[1].effect_mode, 10);
        assert_eq!(read.morph_paths[0].effect_mode, 9, "the morph default");
    }

    #[test]
    fn a_dynamic_paths_mode_reaches_its_sections() {
        let read = read();
        let path = read.dynamic_paths[2];
        assert_eq!(path.effect_mode, 9);
        assert_eq!(path.sections[1].effect_mode, 9);
        assert_eq!(path.sections[1].duration, 10.0);
        assert!(!path.is_authored(), "its first section has no duration");
        assert_eq!(read.dynamic_paths[0].effect_mode, 5, "the dynamic default");
    }
}
