//! Photo mode's accumulated shot: the frames a motion blur or a depth of field is made of, the
//! moment of the race each is drawn at and the sample of the lens it is drawn through.
//!
//! The counts and the shutter are the originals' (`docs/ui/photo-mode.md`, "Accumulated shots
//! and rewind"; `oag_title::photo::Accumulation`). **Chosen, not measured:** everything else,
//! because how the original maps its frames to time and to the lens was never read.
//!
//! - **Motion blur.** The frames are spread evenly over the shutter, from `shutter` seconds
//!   before the shot's tick to the tick itself, and each is drawn from the race at that moment
//!   (the race is stood at the tick after it and drawn that far between the two ticks,
//!   [`crate::between_ticks`]). A track blur lets the camera ride the craft, so the scenery
//!   smears and the craft stays sharp; a ship blur holds the camera where it is at the shot's
//!   tick, so the craft smears and the scenery stays sharp. At most 15 ticks, the longest shutter.
//! - **Depth of field.** The same tick drawn again with the eye moved across the lens, on a
//!   centre sample and a ring of the rest, through an off-axis frustum that holds the focus
//!   plane ([`LensShift`]). Every frame is the single-frame pass at the full aperture, the
//!   picture the preview shows, and the ring only refines it: it is sized so that the ends of
//!   HD's sharp band move at most [`BAND_DRIFT`] of the picture's height and nothing moves more
//!   than [`RING_SHARE`] of the blur the pass draws at its depth. A thin lens has one sharp
//!   distance and no widest blur, so a wider ring would blur the band the preview keeps sharp
//!   and draw a near surface as copies far wider than the preview's blur.
//! - **Both**: the frames of the blur, the lens samples taken in turn.

use oag_post::photo_fx::{Dof, Edges};
use oag_title::photo::Accumulation;

use super::effects::Blur;
use super::rewind::Rewind;
use super::*;
use crate::timeline::Keeper;

/// The fixed tick rate the shutter's seconds are counted in (ADR-0007).
const TICKS_PER_SECOND: f32 = 60.0;

/// A shutter shorter than this many ticks moves nothing worth a frame.
const LEAST_SHUTTER_TICKS: f32 = 0.01;

/// The most a lens sample moves the ends of the sharp band, as a fraction of the picture's
/// height: a quarter of a pixel at 1080 lines, half of one at 2160.
pub const BAND_DRIFT: f32 = 1.0 / 4320.0;

/// The most a lens sample moves a surface outside the sharp band, as a share of the blur
/// radius the single-frame pass draws at its depth.
pub const RING_SHARE: f32 = 0.25;

/// What a shot is made of.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// The tick the shot is taken at.
    tick: u64,
    frames: u32,
    blur: Option<(Blur, f32)>,
    aperture: Option<Aperture>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Aperture {
    radius: f32,
    focus: f32,
    samples: u32,
}

/// One frame of a shot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// The tick the race is stood at.
    pub stand: u64,
    /// How far between the tick before and `stand` the frame is drawn, `0..=1`.
    pub alpha: f32,
    /// The sample of the lens, when the depth of field accumulates.
    pub lens: Option<LensShift>,
}

impl Plan {
    /// The shot `photo` asks for at `tick`, or `None` when it is the one frame on screen.
    /// `resimulates` is whether the race can be played between keyframes: without it there is no
    /// motion blur to draw.
    #[must_use]
    pub fn new(photo: &Photo, tick: u64, resimulates: bool) -> Option<Self> {
        let acc = photo.accumulation()?;
        let effects = photo.effects();
        let blur = effects.blur().filter(|_| resimulates).and_then(|kind| {
            let scale = match kind {
                Blur::Track => 1.0,
                Blur::Ship => acc.ship_shutter,
            };
            let seconds = Accumulation::shutter_seconds(effects.shutter()) * scale;
            let ticks =
                (seconds * TICKS_PER_SECOND).min(Accumulation::LONGEST_SHUTTER * TICKS_PER_SECOND);
            (ticks >= LEAST_SHUTTER_TICKS).then_some((kind, ticks))
        });
        let aperture = effects.depth_of_field().map(|(aperture, focus)| Aperture {
            radius: ring_radius(aperture, focus, photo.fov()),
            focus: Edges::new(focus).distance,
            samples: acc.dof_frames.max(1),
        });
        let frames = match (blur, aperture) {
            (Some(_), _) => acc.blur_frames,
            (None, Some(aperture)) => aperture.samples,
            (None, None) => return None,
        };
        Some(Self {
            tick,
            frames: frames.max(1),
            blur,
            aperture,
        })
    }

    /// How many frames the shot is made of.
    #[must_use]
    pub fn frames(&self) -> u32 {
        self.frames
    }

    /// What smears, with the shutter in ticks.
    #[must_use]
    pub fn blur(&self) -> Option<(Blur, f32)> {
        self.blur
    }

    /// Whether the frames are drawn through different samples of the lens.
    #[must_use]
    pub fn has_lens(&self) -> bool {
        self.aperture.is_some()
    }

    /// Frame `index` of the shot.
    #[must_use]
    pub fn frame(&self, index: u32) -> Frame {
        let (stand, alpha) = match self.blur {
            Some((_, shutter)) if self.frames > 1 => {
                let back = shutter * (1.0 - index as f32 / (self.frames - 1) as f32);
                let whole = back.floor();
                let stand = self.tick.saturating_sub(whole as u64);
                let alpha = if whole == back || stand == 0 {
                    1.0
                } else {
                    1.0 - (back - whole)
                };
                (stand, alpha)
            }
            _ => (self.tick, 1.0),
        };
        Frame {
            stand,
            alpha,
            lens: self.aperture.map(|aperture| aperture.sample(index)),
        }
    }
}

/// The radius of the lens ring, in world units, for the aperture and focus sliders (`0..=1`)
/// and a vertical field of `fov` degrees: the widest ring that keeps the band's ends within
/// [`BAND_DRIFT`] and every other depth within [`RING_SHARE`] of the pass's blur there.
///
/// A ring sample of radius `s` moves a surface at distance `z` by `s * |1/z - 1/d|` of the
/// picture's tangent, `d` the focus distance. The pass's blur ramps from nothing at the band's
/// ends to its widest at the edges and stays there; the depths checked are the near plane,
/// both ramps in steps and infinity, which bound every other.
fn ring_radius(aperture: f32, focus: f32, fov: f32) -> f32 {
    const STEPS: u32 = 16;
    let e = Edges::new(focus);
    let height = 2.0 * (fov.to_radians() * 0.5).tan();
    let widest = aperture.clamp(0.0, 1.0) * Dof::MAX_RADIUS;
    let drift = BAND_DRIFT * height;
    let inverse_d = 1.0 / e.distance;
    // The widest ring that moves a surface at `inverse_z` (1/distance) no more than `allowed`.
    let ring = |inverse_z: f32, allowed: f32| {
        let per_unit = (inverse_z - inverse_d).abs();
        if per_unit > 0.0 {
            allowed / per_unit
        } else {
            f32::INFINITY
        }
    };
    let allowed = |blur: f32| drift.max(RING_SHARE * widest * blur * height);
    let near = crate::camera::NEAR_PLANE;
    let mut radius = ring(1.0 / e.near_focus.max(near), drift)
        .min(ring(1.0 / e.far_focus, drift))
        .min(ring(1.0 / near, allowed(1.0)))
        .min(ring(0.0, allowed(1.0)));
    let near_start = e.near_edge.max(near);
    for step in 0..=STEPS {
        let t = step as f32 / STEPS as f32;
        if e.near_focus > near_start {
            let z = near_start + (e.near_focus - near_start) * t;
            let blur = (e.near_focus - z) / (e.near_focus - e.near_edge);
            radius = radius.min(ring(1.0 / z, allowed(blur)));
        }
        let z = e.far_focus + (e.far_edge - e.far_focus) * t;
        let blur = (z - e.far_focus) / (e.far_edge - e.far_focus);
        radius = radius.min(ring(1.0 / z, allowed(blur)));
    }
    if widest > 0.0 { radius } else { 0.0 }
}

impl Aperture {
    /// Sample `index`: the centre, then a ring of the others, taken in turn.
    fn sample(self, index: u32) -> LensShift {
        let j = index % self.samples;
        let (dx, dy) = if j == 0 || self.samples < 2 {
            (0.0, 0.0)
        } else {
            let turn = std::f32::consts::TAU * (j - 1) as f32 / (self.samples - 1) as f32;
            (self.radius * turn.cos(), self.radius * turn.sin())
        };
        LensShift {
            dx,
            dy,
            focus: self.focus,
        }
    }
}

/// What moves the race to another tick: the rewind and the keyframes behind it.
#[derive(Debug)]
pub struct Mover<'a> {
    /// The race on screen's move through its past.
    pub rewind: &'a mut Rewind,
    /// The keyframes it seeks in.
    pub keeper: &'a mut Keeper,
}

/// A shot under way: the race is moved to each frame's moment and lens sample in turn.
#[derive(Debug)]
pub struct Shot {
    plan: Plan,
    next: u32,
}

impl Shot {
    /// Begins `plan` on the race `rewind` is moving: stands it at the shot's tick, and for a ship
    /// blur takes the camera there to hold.
    pub fn start(plan: Plan, mover: Option<Mover<'_>>, race: &mut Race) -> Self {
        if let Some(Mover { rewind, keeper }) = mover
            && plan.blur.is_some()
        {
            rewind.follow = true;
            rewind.seek(keeper, race, plan.tick);
            if matches!(plan.blur, Some((Blur::Ship, _)))
                && let Some(pose) = race.photo_camera()
            {
                rewind.follow = false;
                race.photo_hold(pose);
                // Again, with the camera held: the poses a frame blends from were taken under
                // the camera that followed the craft.
                rewind.seek(keeper, race, plan.tick);
            }
        }
        Self { plan, next: 0 }
    }

    /// The plan the shot follows.
    #[must_use]
    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// The frame the next [`Self::prepare`] sets up.
    #[must_use]
    pub fn next(&self) -> u32 {
        self.next
    }

    /// Whether every frame has been drawn.
    #[must_use]
    pub fn done(&self) -> bool {
        self.next >= self.plan.frames
    }

    /// Moves the race and the lens to the next frame and returns the alpha to draw it at.
    pub fn prepare(&mut self, mover: Option<Mover<'_>>, race: &mut Race) -> f32 {
        let frame = self.plan.frame(self.next);
        if let Some(Mover { rewind, keeper }) = mover
            && self.plan.blur.is_some()
        {
            if rewind.at() > frame.stand {
                rewind.seek(keeper, race, frame.stand);
            }
            while rewind.at() < frame.stand && rewind.step(race) {}
        }
        race.photo_set_shift(frame.lens);
        // Each lens sample is a different picture, and the upscaler keeps no history across
        // them. A blur's frames are one motion and the camera blend is keyed on the cut count,
        // so it is left alone there.
        if self.plan.blur.is_none()
            && let Some(photo) = race.view.photo.as_mut()
        {
            photo.cuts = photo.cuts.wrapping_add(1);
        }
        frame.alpha
    }

    /// The frame just drawn is taken in.
    pub fn advance(&mut self) {
        self.next += 1;
    }

    /// Ends the shot: the race back at the shot's tick, the lens as it was, the camera free.
    pub fn finish(self, mover: Option<Mover<'_>>, race: &mut Race) {
        if let Some(Mover { rewind, keeper }) = mover
            && self.plan.blur.is_some()
        {
            rewind.follow = true;
            rewind.seek(keeper, race, self.plan.tick);
        }
        race.photo_set_shift(None);
        race.photo_repose();
    }
}
