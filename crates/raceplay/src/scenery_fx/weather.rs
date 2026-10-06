//! Pulse's rain and snow: what `Weather_Update` (`0x088f1e58`) does with the
//! effect `Weather_Construct` (`0x088f184c`) spawns off a circuit's
//! `TrackStartup` `<Weather>` element.
//!
//! **Where the weather is.** Out in the open it rides the camera: the effect's
//! box is `depth` units in front of the lens and holds still in the world as
//! the craft drives through it, so rain streaks away from where the camera is
//! heading. In a *covered* section - one with a `weatherPos` anchor, see
//! [`oag_vex::weather`] - the same effect instead stays where the anchor is,
//! in the world: rain falls through the gap the artist put the anchor at and
//! nowhere else.
//!
//! **The switch is on a section change only.** `Weather_Update` compares the
//! section the camera published this frame with last frame's and does
//! nothing while they match, so the mode is held state, entered on the frame a
//! section edge is crossed. A section below zero, which is what both start as,
//! counts as covered; the first frame therefore leaves "covered" for an open
//! section and enters it for a covered one.
//!
//! **The lens** `ScreenPsys` (`WO_RAIN_LENS`, Fort Gale) is [`super::lens`].
//!
//! **The mist overlay** `WeatherMist_Construct` (`0x088fa0a0`) builds from
//! `Tex` and its sizes is [`super::mist`], drawn by `oag_fx::mist`.
//!
//! **Not played, and said so:**
//! - The manager's clock behind the noise: its units are taken as seconds, the
//!   race clock, and the noise table is a seeded one - see [`super::noise`].
//! - `Weather_Update` is skipped in Zone, and here the plan is not built there.

use std::sync::Arc;

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_fx::psys::field::{Anchor, FieldSpec, Frame};
use oag_fx::psys::{self, Effect, System};
use oag_mesh::mesh::GpuVertex;
use oag_tables::trackstartup::{Weather as Config, effect_name};
use oag_vex::weather::{self as covered, Anchor as CoveredAnchor};

use super::lens::{Edge, Lens};
use super::mist::Mist;
use super::wind::Wind;

/// Seed for the weather's own spawn draws, wind table and phases. Any
/// constant: render state, never hashed.
const SEED: u64 = 0x7a1e_b01d;

/// What a circuit asks for and where it is covered, read at load.
#[derive(Debug, Clone, PartialEq)]
pub struct Setup {
    /// The circuit's `<Weather>` element.
    pub config: Config,
    /// Its `weatherPos` anchors, one per covered section.
    pub anchors: Vec<CoveredAnchor>,
    /// `Tex` decoded, the mist overlay's texture; `None` when the circuit
    /// names none or it did not decode, and then the mist draws nothing.
    pub mist_texture: Option<oag_fx::exhaust::FlareTexture>,
}

/// The weather of a race: inert unless the circuit authors one.
#[derive(Debug, Clone)]
pub struct Weather {
    setup: Option<Setup>,
    mask: u64,
    effect: Option<Arc<Effect>>,
    resolved: bool,
    pool: System,
    /// The sections published last frame and this frame, `DAT_08ab10a4` and
    /// `DAT_08ab10a0`.
    sections: (i32, i32),
    /// Whether the effect sits at its anchor rather than on the camera: the
    /// original's instance flag `0x200000`.
    anchored: bool,
    anchor: Frame,
    previous: Option<Frame>,
    wind: Wind,
    lens: Lens,
    mist: Option<Mist>,
    clock: f32,
    rng: Rng,
}

impl Default for Weather {
    fn default() -> Self {
        Self::new(None)
    }
}

impl Weather {
    /// Nothing plays until the first [`Self::advance`].
    #[must_use]
    pub fn new(setup: Option<Setup>) -> Self {
        let mut rng = Rng::new(SEED);
        let config = setup.as_ref().map(|s| &s.config);
        let wind = Wind::new(
            config.map_or(0.0, |c| c.wind_base),
            config.map_or(0.0, |c| c.wind_range),
            config.map_or(0.0, |c| c.drift_y),
            &mut rng,
        );
        let mask = setup
            .as_ref()
            .map_or(0, |s| covered::covered_mask(&s.anchors));
        let mist = setup
            .as_ref()
            .filter(|s| s.mist_texture.is_some())
            .and_then(|s| Mist::new(&s.config));
        Self {
            setup,
            mask,
            effect: None,
            resolved: false,
            pool: System::new(),
            sections: (-1, -1),
            anchored: false,
            anchor: Frame::IDENTITY,
            previous: None,
            wind,
            lens: Lens::new(None),
            mist,
            clock: 0.0,
            rng,
        }
    }

    /// Whether the circuit authors weather.
    #[must_use]
    pub fn is_authored(&self) -> bool {
        self.setup.is_some()
    }

    /// Whether the env effect loaded and is playing.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.effect.is_some() && self.pool.is_running()
    }

    /// The section last published, or `-1` before the first.
    #[must_use]
    pub fn section(&self) -> i32 {
        self.sections.1
    }

    /// Whether it sits at its anchor, a covered section's.
    #[must_use]
    pub fn is_anchored(&self) -> bool {
        self.anchored
    }

    /// The raindrops on the glass.
    #[must_use]
    pub fn lens(&self) -> &Lens {
        &self.lens
    }

    /// The mist overlay, when the circuit authors one and its texture decoded.
    #[must_use]
    pub fn mist(&self) -> Option<&Mist> {
        self.mist.as_ref()
    }

    /// The mist overlay's decoded texture, for the renderer.
    #[must_use]
    pub fn mist_texture(&self) -> Option<&oag_fx::exhaust::FlareTexture> {
        self.setup.as_ref()?.mist_texture.as_ref()
    }

    /// The pool, for tests.
    #[must_use]
    pub fn pool(&self) -> &System {
        &self.pool
    }

    /// One frame: `camera` is where the lens is, `section` the one the craft is in.
    pub fn advance(&mut self, effects: &psys::Library, dt: f32, camera: Frame, section: i32) {
        let Some(setup) = &self.setup else {
            return;
        };
        if !self.resolved {
            self.resolved = true;
            self.effect = setup
                .config
                .env_psys
                .as_deref()
                .and_then(|path| effects.get(effect_name(path)))
                .cloned();
            if let Some(effect) = &self.effect {
                self.pool.ignite(effect, Vec3::ZERO, 1.0);
            }
            self.lens = Lens::new(
                setup
                    .config
                    .screen_psys
                    .as_deref()
                    .and_then(|path| effects.get(effect_name(path))),
            );
        }
        self.clock += dt;
        self.sections = (self.sections.1, section);
        if self.sections.0 != self.sections.1 {
            let was = covered::covered(self.mask, self.sections.0);
            let now = covered::covered(self.mask, self.sections.1);
            if now {
                self.anchored = true;
                let at = setup
                    .anchors
                    .iter()
                    .find(|a| i32::from(a.section) == section);
                if let Some(anchor) = at {
                    self.anchor = frame_of(&anchor.world);
                }
            } else if was {
                self.anchored = false;
            }
            let edge = match (was, now) {
                (false, true) => Some(Edge::Entered),
                (true, true) => Some(Edge::Held),
                (true, false) => Some(Edge::Left),
                (false, false) => None,
            };
            if let Some(edge) = edge {
                self.lens.edge(edge, self.clock);
                if let Some(mist) = &mut self.mist {
                    mist.edge(edge);
                }
            }
        }
        self.wind.advance(dt, self.clock);
        if let Some(mist) = &mut self.mist {
            let wind = self.wind.modifier();
            mist.advance(dt, self.previous, camera, wind);
        }
        self.lens
            .advance(dt, self.clock, camera, self.wind.vector());
        if let Some(effect) = self.effect.clone()
            && let Some(field) = effect.field
        {
            let spec = FieldSpec {
                half_extent: field.half_extent,
                depth: effect.view_depth,
                wind: self.wind.modifier(),
            };
            let anchor = self.anchor_for(camera);
            self.pool
                .advance_field(&effect, dt, &anchor, &spec, &mut self.rng);
        }
        self.previous = Some(camera);
    }

    fn anchor_for(&self, camera: Frame) -> Anchor {
        if self.anchored {
            Anchor::World(self.anchor)
        } else {
            Anchor::Camera {
                previous: self.previous.unwrap_or(camera),
                current: camera,
            }
        }
    }

    /// Appends this frame's geometry, with the camera as it is drawn.
    pub fn extend_vertices(
        &self,
        additive: &mut Vec<GpuVertex>,
        alpha_over: &mut Vec<GpuVertex>,
        camera: Frame,
        right: Vec3,
        up: Vec3,
    ) {
        self.lens
            .extend_vertices(additive, alpha_over, camera, right, up);
        let Some(effect) = &self.effect else {
            return;
        };
        if effect.field.is_none() {
            return;
        }
        let anchor = if self.anchored {
            Anchor::World(self.anchor)
        } else {
            Anchor::Camera {
                previous: camera,
                current: camera,
            }
        };
        self.pool
            .in_world(&anchor, effect.view_depth)
            .extend_vertices(additive, alpha_over, effect, right, up, None);
    }
}

/// A row-major matrix's rows as a [`Frame`].
fn frame_of(world: &[f32; 16]) -> Frame {
    let row = |at: usize| Vec3::new(world[at], world[at + 1], world[at + 2]);
    Frame {
        position: row(12),
        right: row(0),
        up: row(4),
        back: row(8),
    }
}
