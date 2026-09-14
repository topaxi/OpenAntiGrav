//! The Fury menu backdrop's model: which path is playing, which cloud it flies
//! past, where the camera is this frame, and the numbers the sprites are
//! drawn with.
//!
//! Wipeout HD's Fury style draws its menus over a ship's hull sampled to a
//! point cloud (`oag_rcs::points2`), flown past along an authored camera path
//! (`oag_tables::fury_backdrop`), each point a camera-facing sprite whose size,
//! fade, fog and colour ramp `RadioHead2_Update` computes once a frame and the
//! vertex program applies per point. This module is that per-frame
//! computation, GPU-free: it hands `oag_game::render` a [`Frame`] and the
//! renderer applies it. Everything here is read off
//! [menu-backdrop.md](../../../docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md);
//! the formulas are that page's observations, written out again rather than
//! copied from the decompiler.
//!
//! # What plays, and what does not yet
//!
//! `BackgroundAnimFury_PickPath` rolls one of three path kinds - static, morph,
//! dynamic - and the morph and dynamic kinds draw with vertex programs this
//! project has not read (modes `5`, `9`, `10`, `11`). **Only static paths
//! play**: the roll is kept, and a morph or dynamic pick is re-rolled, which
//! keeps the eight static paths' relative frequency and the recent-eight queue
//! exactly. The loader report says so. A static path never asks for a new
//! cloud - `PointCloud_ModeNeedsNewCloud(2)` is false - so the one cloud the
//! boot picked stays until a mode that does is implemented.
//!
//! # No music
//!
//! Three equaliser bands feed the original's music pulse, its feedback gain and
//! its `musicMultiplier`. This build has no equaliser tap, so the bands sit at
//! `Music Pulse Base` (`musicPulse = 1`), the blend gain at its base level,
//! and the multiplier at the `0` `Render` writes anyway.

use oag_core::rng::Rng;
use oag_tables::fexml::Node;
use oag_tables::fury_backdrop::{FuryBackdrop, Path, STATIC_PATHS};

/// The original counts clip frames at this rate (`PickPath` writes `60.0`
/// into the clip), and the menu stage ticks at the same fixed step.
pub const FRAMES_PER_SECOND: f32 = 60.0;

/// How many recently played paths the picker refuses to repeat.
const RECENT: usize = 8;

/// How many rolls the picker makes before giving up and keeping the current
/// path for another clip.
const TRIES: usize = 64;

/// The projection's near and far planes, `Render`'s own arguments.
pub const NEAR: f32 = 0.005;
pub const FAR: f32 = 50.0;

/// The distances the depth fade runs between: `C+0x28 = 1.0` to `C+0x2c = 3.0`.
const DEPTH_FADE_NEAR: f32 = 1.0;
const DEPTH_FADE_FAR: f32 = 3.0;

/// `FuryStaticPath_Ramp`'s three hard-coded values: the ramp's period, speed
/// and per-point jitter.
const RAMP_PERIOD: f32 = 10.0;
const RAMP_SPEED: f32 = 0.2;
const RAMP_JITTER: f32 = 0.5;

/// The brightness the particle colour rests at, between `OnEnable`'s pulses.
const BRIGHTNESS_AT_REST: f32 = 0.7;

/// The `Wave` pass's constants, hard-coded in `FurySettings_Construct`.
pub const WAVE_COLOUR_SCALE: [f32; 3] = [0.94, 0.94, 0.955];
pub const WAVE_COLOUR_BIAS: f32 = -0.0003;
pub const WAVE_BIAS_FACTOR: f32 = 8.0;

/// The `Blend` pass's cap on the fresh particles, at the equaliser's base
/// level: `clamp(0.016 * 24 * eq, floor, 0.15)` with `eq` sitting at `Music
/// Pulse Base` for want of a tap.
const BLEND_GAIN: f32 = 0.016;
const BLEND_GAIN_SCALE: f32 = 24.0;
const BLEND_FLOOR: [f32; 3] = [0.02, 0.02, 0.023];
const BLEND_CEILING: f32 = 0.15;

/// One clip: a static path, and how far into it the clock is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Clip {
    path: usize,
    frame: u32,
    frames: u32,
}

/// The backdrop's state between frames.
#[derive(Debug, Clone)]
pub struct Fury {
    settings: FuryBackdrop,
    rng: Rng,
    /// Which of the boot's loaded clouds is drawn. A static path never asks
    /// for another, so this is the boot's pick until a mode that does is
    /// read.
    cloud: usize,
    recent: Vec<usize>,
    clip: Clip,
    /// How many rolls landed on a morph or dynamic path and were re-rolled,
    /// for the report.
    rerolled: u32,
}

/// What the renderer draws one frame with.
///
/// Matrices are column-major `[column][row]`, the layout a WGSL `mat4x4<f32>`
/// takes straight from the buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Which loaded cloud.
    pub cloud: usize,
    /// World to view: the camera's look-at, the eye at the origin looking
    /// down `-z`.
    pub world_view: [[f32; 4]; 4],
    /// View to clip.
    pub proj: [[f32; 4]; 4],
    /// The sprite's half-size in world units before the depth-of-field growth.
    pub sprite_size: f32,
    /// The point's displacement along its normal; `0` without music.
    pub music_multiplier: f32,
    /// The base colour every sprite starts from.
    pub particle_colour: [f32; 3],
    /// What the ramp adds at its peak.
    pub colour_ramp: [f32; 3],
    /// `(offset, scale, jitter, 0)`: `t = ((p.z - offset) + rand * jitter) * scale`.
    pub colour_ramp_factors: [f32; 4],
    /// `(4, -3, 6, 0)`: `ramp = max(0, 4 * (1 - frac(t)) * 0.4 * d - 3) ^ 6`.
    pub colour_ramp_factors2: [f32; 4],
    /// `(a, b)`: `fade = d * a + b`.
    pub depth_fade_factors: [f32; 2],
    /// `(-start, -1/strength, 24, 1/factor)`.
    pub dof_factors: [f32; 4],
    /// `(-start, -1/length, -start-length, exponent)`.
    pub fog_factors: [f32; 4],
    /// The trail's survival per frame.
    pub feedback: [f32; 3],
    /// The blend pass's cap on the fresh particles.
    pub source_max: [f32; 3],
    /// The screen's tint, the constant blend colour the composite is scaled by.
    pub tint: [f32; 4],
}

impl Fury {
    /// A backdrop drawing the boot's cloud `cloud`, with the picker seeded by
    /// `seed`.
    ///
    /// The first path is picked at once, so the first frame is already on a
    /// path rather than a frame of nothing; `None` when the file authors no
    /// static path at all, which the disc's does not do.
    #[must_use]
    pub fn new(settings: FuryBackdrop, cloud: usize, seed: u64) -> Option<Self> {
        let mut out = Self {
            settings,
            rng: Rng::new(seed),
            cloud,
            recent: Vec::with_capacity(RECENT),
            clip: Clip {
                path: 0,
                frame: 0,
                frames: 0,
            },
            rerolled: 0,
        };
        out.settings.authored_static_paths().next()?;
        out.pick();
        Some(out)
    }

    /// The settings this plays.
    #[must_use]
    pub fn settings(&self) -> &FuryBackdrop {
        &self.settings
    }

    /// Which cloud is drawn.
    #[must_use]
    pub fn cloud(&self) -> usize {
        self.cloud
    }

    /// Which static path is playing.
    #[must_use]
    pub fn path(&self) -> usize {
        self.clip.path
    }

    /// How many picker rolls landed on a mode this build does not draw.
    #[must_use]
    pub fn rerolled(&self) -> u32 {
        self.rerolled
    }

    /// Plays static path `path` from its start, whatever the picker rolled -
    /// the original's debug `Force Path` (`g_FurySettings+0x414`), which
    /// `PickPath` honours before rolling. `None` when the path is not
    /// authored, and the clip is left as it was.
    pub fn force_path(&mut self, path: usize) -> Option<()> {
        if !self
            .settings
            .static_paths
            .get(path)
            .is_some_and(Path::is_authored)
        {
            return None;
        }
        let duration = self.settings.static_paths[path].duration;
        self.clip = Clip {
            path,
            frame: 0,
            frames: (duration * FRAMES_PER_SECOND).max(1.0) as u32,
        };
        Some(())
    }

    /// Seconds into the clip.
    #[must_use]
    pub fn seconds(&self) -> f32 {
        self.clip.frame as f32 / FRAMES_PER_SECOND
    }

    /// Advances the clock by one frame, picking the next path when this one
    /// has run its length. `Render` counts the clip up once per frame while
    /// the widget is enabled and the game is not paused.
    pub fn tick(&mut self) {
        self.clip.frame += 1;
        if self.clip.frame >= self.clip.frames {
            self.pick();
        }
    }

    /// `BackgroundAnimFury_PickPath`, for the static kind alone.
    ///
    /// `rand() % 3` picks the kind and `rand() % 8` the index; a path with
    /// no duration or one in the recent queue is rejected and the roll
    /// repeated, up to sixty-four times. A morph or dynamic kind is what this
    /// build re-rolls, and counts.
    ///
    /// **The queue is shorter here than the original's eight, and has to
    /// be.** The original's queue holds paths of all three kinds, twenty in
    /// all, so eight recent ones always leave a static path to pick. With
    /// the eight static paths alone in play an eight-deep queue fills after
    /// eight clips and every roll after that is rejected - so this build
    /// keeps the queue one short of the authored static count, which is the
    /// longest "do not repeat" that still leaves a path to pick.
    fn pick(&mut self) {
        let authored = self.settings.authored_static_paths().count();
        let queue = RECENT.min(authored.saturating_sub(1));
        let mut picked = None;
        for _ in 0..TRIES {
            let kind = self.rng.below(3);
            let index = self.rng.below(STATIC_PATHS as u32) as usize;
            if kind != 0 {
                self.rerolled += 1;
                continue;
            }
            if !self.settings.static_paths[index].is_authored() || self.recent.contains(&index) {
                continue;
            }
            picked = Some(index);
            break;
        }
        // Sixty-four failed rolls: the original keeps its current path. With
        // one static path in twenty-four rolls left to land on, that happens
        // one clip in fifteen here, and repeating the clip just watched is
        // the more visible wrong - so the path the queue leaves free is
        // taken instead.
        let path = picked
            .or_else(|| {
                self.settings
                    .authored_static_paths()
                    .find(|index| !self.recent.contains(index))
            })
            .unwrap_or(self.clip.path);
        while self.recent.len() >= queue.max(1) {
            self.recent.remove(0);
        }
        self.recent.push(path);
        let duration = self.settings.static_paths[path].duration;
        self.clip = Clip {
            path,
            frame: 0,
            frames: (duration * FRAMES_PER_SECOND).max(1.0) as u32,
        };
    }

    /// This frame's camera and constants, for a picture `height` lines tall
    /// shown at `aspect`, tinted by the screen's `tint`.
    ///
    /// `height` is the *display's*: `RadioHead2_Update` scales the sprite size
    /// and the colours by the frame buffer's line count, both of which are
    /// exactly `1` at 1080 lines.
    #[must_use]
    pub fn frame(&self, height: f32, aspect: f32, tint: [f32; 4]) -> Frame {
        let path = &self.settings.static_paths[self.clip.path];
        let seconds = self.seconds();
        let (world_view, proj) = camera(path, seconds, aspect);
        // `pointSize * clamp(2.26 - 0.0011667 * height, 0.5, 2.5)` and
        // `clamp(0.0015 * height - 0.62, 0, 4)`, both `1.0` at 1080 lines.
        let sprite_scale = (2.26 - 0.001_166_7 * height).clamp(0.5, 2.5);
        let res_scale = (0.0015 * height - 0.62).clamp(0.0, 4.0);
        // `1 + (mean(eq) - base) * factor` over three equaliser bands the
        // sound system feeds. Nothing feeds them here, and a band nobody
        // feeds reads zero - the same `0.23` RPCS3 shows between beats.
        let music_pulse =
            1.0 + (0.0 - self.settings.music_pulse_base) * self.settings.music_pulse_factor;
        let particle_colour = self
            .settings
            .particle_colour
            .map(|c| c * BRIGHTNESS_AT_REST * music_pulse * res_scale);
        let colour_ramp = self.settings.particle_ramp_colour.map(|c| c * res_scale);
        let eq = self.settings.music_pulse_base;
        let source_max = BLEND_FLOOR
            .map(|floor| (BLEND_GAIN * BLEND_GAIN_SCALE * eq).clamp(floor, BLEND_CEILING));
        Frame {
            cloud: self.cloud,
            world_view,
            proj,
            sprite_size: path.point_size * sprite_scale,
            music_multiplier: 0.0,
            particle_colour,
            colour_ramp,
            colour_ramp_factors: [
                RAMP_SPEED * 100.0 - RAMP_SPEED * 200.0 * seconds / RAMP_PERIOD,
                0.08,
                RAMP_JITTER,
                0.0,
            ],
            colour_ramp_factors2: [4.0, -3.0, 6.0, 0.0],
            depth_fade_factors: [
                1.0 / (DEPTH_FADE_FAR - DEPTH_FADE_NEAR),
                DEPTH_FADE_NEAR / (DEPTH_FADE_NEAR - DEPTH_FADE_FAR),
            ],
            dof_factors: [
                -path.dof_start,
                -1.0 / path.dof_strength,
                24.0,
                1.0 / path.dof_factor,
            ],
            fog_factors: [
                -path.fog_start,
                -1.0 / path.fog_length,
                -path.fog_start - path.fog_length,
                path.fog_exponent,
            ],
            feedback: self.settings.feedback,
            source_max,
            tint,
        }
    }
}

/// `FuryStaticPath_Camera` and the projection `Render` builds after it.
///
/// The eye and the target both lerp from their start to their end over the
/// path's duration; the view is a look-at with the world's `+y` up. The
/// projection is `perspective(fovy, aspect, NEAR, FAR)` - clip `z` mapped to
/// `0..1` for wgpu rather than the RSX's `-1..1`, which changes no pixel: the
/// backdrop draws with no depth test, and `x`, `y` and `w` are the same
/// either way.
fn camera(path: &Path, seconds: f32, aspect: f32) -> ([[f32; 4]; 4], [[f32; 4]; 4]) {
    let t = seconds / path.duration.max(1e-4);
    let lerp = |a: [f32; 3], b: [f32; 3]| std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t);
    let eye = lerp(path.start, path.end);
    let target = lerp(path.focus_start, path.focus_end);
    let forward = normalise(sub(target, eye));
    let right = normalise(cross(forward, [0.0, 1.0, 0.0]));
    let up = cross(right, forward);
    let world_view = [
        [right[0], up[0], -forward[0], 0.0],
        [right[1], up[1], -forward[1], 0.0],
        [right[2], up[2], -forward[2], 0.0],
        [-dot(right, eye), -dot(up, eye), dot(forward, eye), 1.0],
    ];
    let f = 1.0 / (path.fovy.to_radians() * 0.5).tan();
    let proj = [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, FAR / (NEAR - FAR), -1.0],
        [0.0, 0.0, NEAR * FAR / (NEAR - FAR), 0.0],
    ];
    (world_view, proj)
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Unit length, or the vector untouched when it has none - the original's
/// reciprocal square root is guarded the same way.
fn normalise(v: [f32; 3]) -> [f32; 3] {
    let length = dot(v, v).sqrt();
    if length > 0.0 {
        v.map(|c| c / length)
    } else {
        v
    }
}

/// The per-screen tints the widget is authored with.
///
/// `<BackgroundAnimFury>`'s `<ScreenSetting name=".." tint="0xAARRGGBB">`
/// rows out of `skin.xml`, which `BackgroundAnimFury_ParseScreenSetting`
/// reads and `Render` hands to the composite as its constant blend colour.
/// `Main Menu` is white; `default` and most others are `0xFF202020`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tints {
    by_screen: Vec<(String, u32)>,
}

impl Tints {
    /// Every `ScreenSetting` under the first `BackgroundAnimFury` in `root`,
    /// or `None` when the file authors no such widget - the HD-style disc's
    /// `skin.xml` does not.
    #[must_use]
    pub fn read(root: &Node) -> Option<Self> {
        let widget = find(root, "BackgroundAnimFury")?;
        let by_screen = widget
            .children_named("ScreenSetting")
            .filter_map(|setting| {
                let name = setting.attr("name")?;
                let tint = crate::screen::parse_argb(setting.attr("tint")?)?;
                Some((name.to_string(), tint))
            })
            .collect();
        Some(Self { by_screen })
    }

    /// The tint for `screen`, falling back to `default`, then to white -
    /// `ParseScreenSetting`'s own default before any row is read.
    #[must_use]
    pub fn for_screen(&self, screen: &str) -> [f32; 4] {
        let argb = self
            .by_screen
            .iter()
            .find(|(name, _)| name == screen)
            .or_else(|| self.by_screen.iter().find(|(name, _)| name == "default"))
            .map_or(0xFFFF_FFFF, |(_, tint)| *tint);
        let channel = |shift: u32| ((argb >> shift) & 0xff) as f32 / 255.0;
        [channel(16), channel(8), channel(0), channel(24)]
    }

    /// The tint for one of this build's pages: its root is the disc's `Main
    /// Menu`, and every other page is drawn as `default` - the tint the
    /// original's `Controls`, `SoundTest`, records and manual screens carry.
    /// This build's pages are not the disc's screens, so the two strips the
    /// original also draws white (`Additional`, `Extras`, `Controls Menu`) have
    /// no page here to claim them.
    #[must_use]
    pub fn for_root(&self, root: bool) -> [f32; 4] {
        self.for_screen(if root { "Main Menu" } else { "default" })
    }

    /// How many screens name a tint.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_screen.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_screen.is_empty()
    }
}

fn find<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name == name {
        return Some(node);
    }
    node.children.iter().find_map(|child| find(child, name))
}

#[cfg(test)]
mod tests;
