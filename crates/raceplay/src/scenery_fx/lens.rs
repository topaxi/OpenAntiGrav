//! The screen lens: `ScreenPsys`, Fort Gale's `WO_RAIN_LENS` - raindrops on the
//! glass.
//!
//! `Weather_Construct` (`0x088f184c`) spawns it as a second instance beside the
//! rain, and `Weather_Update` (`0x088f1e58`) steers it. **Its emitter sits on
//! the lens:** a rectangle, shape 2, of `10 x 5.625` - the screen's 16:9 -
//! `6` units in front of the camera, droplets stay where they were born on the
//! glass and run down it, tilted by the wind as the screen sees it. What the
//! weather does to it, all of it read off `Weather_Update`:
//!
//! - **In the open it emits** a count that is eased, a fortieth a tick, toward
//!   `4` per tick; **under cover** the target is `0`. The count is the integer
//!   part, written into the resource's per-emission range.
//! - **On entering a covered section** the drops already on the glass are
//!   given `80` and `40` ticks of life where they had `16` and `4`, their drag
//!   goes from `0.9` to `0.97` and their speed to `0.3` of what it was, and
//!   after `0.3` s the emitter is switched off. Leaving a covered section
//!   restores the drag and switches the emitter on, **but not the lifetime
//!   or the speed** - the original puts the lifetime back only when the
//!   `0.3` s timer runs out and the speed factor never. Both are ported as
//!   read, so a lens that left cover inside the `0.3` s keeps its long lives.
//! - **The azimuth** the droplets are aimed along (`res+0x54`) is
//!   `atan(-wx / wy)` of the wind in the camera's frame: down the glass,
//!   leaning with the wind.
//!
//! Not read: the lens's draw state beyond its placement (the `+0x98` depth,
//! `6`, and the instance matrix, read live). The mist overlay is
//! [`super::mist`].

use std::sync::Arc;

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_fx::psys::field::{Anchor, Frame};
use oag_fx::psys::{Effect, System};
use oag_mesh::mesh::GpuVertex;

/// Seed for the lens's own draws. Any constant: render state, never hashed.
const SEED: u64 = 0x1e45_d20b;

/// How far the per-emission count is eased a tick (`node+0x464`).
const SMOOTHING: f32 = 0.025;

/// The count the open sections ease toward (`node+0x460`).
const OPEN_COUNT: f32 = 4.0;

/// Seconds the emitter runs on after entering cover (`0.3`).
const LINGER: f32 = 0.3;

/// Lifetimes a drop is given on entering cover, in ticks (`0x50`, `0x28`).
const COVER_LIFETIME: (f32, f32) = (80.0, 40.0);

/// The drag every axis takes on entering cover (`0x3f7851ec`).
const COVER_DRAG: f32 = 0.97;

/// The speed factor entering cover sets (`0x3e99999a`).
const COVER_SPEED: f32 = 0.3;

/// What the weather's section edge did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// An open section into a covered one.
    Entered,
    /// A covered section into another covered one.
    Held,
    /// A covered section into an open one.
    Left,
}

/// The raindrops on the glass.
#[derive(Debug, Clone)]
pub struct Lens {
    effect: Option<Effect>,
    base: Option<Base>,
    pool: System,
    rng: Rng,
    /// The smoothed per-emission count (`node+0x45c`) and its target.
    count: f32,
    target: f32,
    /// When the emitter stops, in clock seconds; `0` for no timer (`node+0x428`).
    timer: f32,
    /// The lifetimes saved on entering cover (`node+0x42c`).
    saved_lifetime: (f32, f32),
    emitting: bool,
    azimuth: f32,
}

/// What the effect authors, to put back.
#[derive(Debug, Clone, Copy)]
struct Base {
    drag: Vec3,
    speed: (f32, f32),
}

impl Lens {
    /// A lens that plays `effect`, or nothing where it did not load.
    #[must_use]
    pub fn new(effect: Option<&Arc<Effect>>) -> Self {
        let effect = effect.map(|e| (**e).clone());
        let base = effect
            .as_ref()
            .and_then(|e| e.emitters.first())
            .map(|spec| Base {
                drag: spec.drag_per_tick,
                speed: spec.speed_per_tick,
            });
        let mut pool = System::new();
        if let Some(effect) = &effect {
            pool.ignite(effect, Vec3::ZERO, 1.0);
        }
        Self {
            effect,
            base,
            pool,
            rng: Rng::new(SEED),
            count: 0.0,
            target: OPEN_COUNT,
            timer: 0.0,
            saved_lifetime: (0.0, 0.0),
            emitting: true,
            azimuth: 0.0,
        }
    }

    /// Whether the lens loaded.
    #[must_use]
    pub fn is_loaded(&self) -> bool {
        self.effect.is_some()
    }

    /// Droplets alive on the glass.
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.pool.alive_count()
    }

    /// Whether the emitter is on.
    #[must_use]
    pub fn is_emitting(&self) -> bool {
        self.emitting
    }

    /// The count the emitter is easing toward.
    #[must_use]
    pub fn target(&self) -> f32 {
        self.target
    }

    /// The integer count each emission makes now.
    #[must_use]
    pub fn per_emission(&self) -> u32 {
        self.count as u32
    }

    /// The azimuth the droplets are aimed along, radians.
    #[must_use]
    pub fn azimuth(&self) -> f32 {
        self.azimuth
    }

    /// The section edge of this frame, if there was one.
    pub fn edge(&mut self, edge: Edge, clock: f32) {
        let (Some(effect), Some(base)) = (&mut self.effect, self.base) else {
            return;
        };
        let Some(spec) = effect.emitters.first_mut() else {
            return;
        };
        match edge {
            Edge::Entered => {
                self.timer = clock + LINGER;
                self.saved_lifetime = spec.lifetime_ticks;
                spec.lifetime_ticks = COVER_LIFETIME;
                spec.drag_per_tick = Vec3::splat(COVER_DRAG);
                spec.speed_per_tick = (base.speed.0 * COVER_SPEED, base.speed.1 * COVER_SPEED);
                self.target = 0.0;
            }
            Edge::Left => {
                self.target = OPEN_COUNT;
                self.timer = 0.0;
                spec.drag_per_tick = base.drag;
                if !self.emitting {
                    self.emitting = true;
                    self.pool.ignite(effect, Vec3::ZERO, 1.0);
                }
            }
            Edge::Held => {}
        }
    }

    /// One frame. `wind` is the world wind of `node+0x410`, `camera` the lens.
    pub fn advance(&mut self, dt: f32, clock: f32, camera: Frame, wind: Vec3) {
        let Some(effect) = &mut self.effect else {
            return;
        };
        for _ in 0..(dt * 60.0) as u32 {
            self.count += (self.target - self.count) * SMOOTHING;
        }
        let per_emission = self.count as u32;
        let seen = camera.direction_to_local(wind);
        self.azimuth = (-seen.x / seen.y).atan();
        if let Some(spec) = effect.emitters.first_mut() {
            spec.per_emission = (per_emission, per_emission);
            if self.timer > 0.0 && self.timer < clock {
                self.timer = 0.0;
                self.emitting = false;
                self.pool.stop();
                spec.lifetime_ticks = self.saved_lifetime;
            }
        }
        self.pool.set_azimuth(self.azimuth);
        self.pool
            .advance(effect, dt, Vec3::ZERO, Vec3::Y, &mut self.rng);
    }

    /// Appends this frame's geometry, on the glass of `camera`.
    pub fn extend_vertices(
        &self,
        additive: &mut Vec<GpuVertex>,
        alpha_over: &mut Vec<GpuVertex>,
        camera: Frame,
        right: Vec3,
        up: Vec3,
    ) {
        let Some(effect) = &self.effect else {
            return;
        };
        self.pool
            .in_world(&Anchor::Lens(camera), effect.view_depth)
            .extend_vertices(additive, alpha_over, effect, right, up, None);
    }
}
