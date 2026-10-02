//! The particle effects a circuit places itself: the welders' sparks and the
//! steam vents its `.vex` authors as `ParticleSystem` nodes.
//!
//! **No simulation event, by design.** The original spawns each one when the
//! circuit loads and keeps it on its node for the whole race
//! ([`oag_vex::placed_psys`] for the read), so there is nothing for the
//! simulation to decide: this is scenery, owned by the view and advanced on
//! the same clock the circuit's animated geometry runs on (`tick / 60`, see
//! `race::scene::frame`).
//!
//! **Its own [`psys::Stage`].** Circuit seven places eighteen steam vents,
//! every one attached for the whole race; on the shared stage they would
//! leave weapon detonations a handful of slots. A second pool keeps the two
//! apart. Chosen, not measured: the original's particle manager is one pool.
//!
//! **Not done, and said so:**
//! - The original hands the instance the node's whole world matrix; a
//!   [`psys::System`] takes a position and an `up`, so the emitter frame's
//!   rotation about its own `+Y` is not carried.
//! - Nothing is culled. Whether the original's draw slot (`0x08915fd0`) skips a
//!   node outside the visible sections is unread; every placed effect here is
//!   simulated and drawn wherever the camera is. Chosen, not measured.
//! - Pulse PSP only, the one source the spawn was confirmed live on - see
//!   `race::load::pulse_psp::finish`.
//!
//! **The weather is here too**, in [`weather`]: a circuit's rain or snow is the
//! other thing it places on its own, from its `TrackStartup` rather than its
//! `.vex` nodes, and it rides the same pool of view-side state.

pub mod lens;
mod noise;
pub mod weather;
mod wind;

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_render::psys;
use oag_vex::placed_psys::Placed;

pub use weather::Weather;

/// What a circuit places on its own scenery, as read at load.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    /// The `ParticleSystem` nodes of its `.vex`.
    pub placed: Vec<Placed>,
    /// Its `<Weather>` element and anchors.
    pub weather: Option<weather::Setup>,
}

/// Seed for the scenery effects' own spawn parameters. Any constant: render
/// state, never hashed.
const SEED: u64 = 0x5ce7_e4f5;

/// Severity every placed instance plays at. `PsysNode_Init` writes no
/// severity, so the instance keeps the default every other untuned spawn here
/// plays at too.
const SEVERITY: f32 = 1.0;

/// The circuit's placed effects, and the pool they play in.
#[derive(Debug, Clone)]
pub struct SceneryFx {
    weather: Weather,
    placed: Vec<Placed>,
    playing: Vec<Option<psys::Playing>>,
    stage: psys::Stage,
    rng: Rng,
}

impl Default for SceneryFx {
    fn default() -> Self {
        Self::new(Plan::default())
    }
}

impl SceneryFx {
    /// Nothing playing yet; the first [`Self::advance`] starts every effect
    /// the library holds.
    #[must_use]
    pub fn new(plan: Plan) -> Self {
        let Plan { placed, weather } = plan;
        Self {
            weather: Weather::new(weather),
            playing: vec![None; placed.len()],
            placed,
            stage: psys::Stage::new(),
            rng: Rng::new(SEED),
        }
    }

    /// Moves every instance onto its node at `seconds` and runs the pool one
    /// step. An effect the library did not load is skipped, and stays
    /// undrawn rather than stood in for.
    pub fn advance(&mut self, effects: &psys::Library, dt: f32, seconds: f32) {
        for (placed, playing) in self.placed.iter().zip(&mut self.playing) {
            let world = placed.world_at(seconds);
            let at = Vec3::new(world[12], world[13], world[14]);
            let up = Vec3::new(world[4], world[5], world[6]);
            match playing {
                Some(handle) => {
                    self.stage.follow(*handle, at);
                    self.stage.orient(*handle, up);
                }
                None => {
                    let Some(effect) = effects.get(&placed.name) else {
                        continue;
                    };
                    *playing = self.stage.attach(effect, at, SEVERITY);
                    if let Some(handle) = playing {
                        self.stage.orient(*handle, up);
                    }
                }
            }
        }
        self.stage.advance(dt, &mut self.rng);
    }

    /// The circuit's weather.
    #[must_use]
    pub fn weather(&self) -> &Weather {
        &self.weather
    }

    /// Runs the weather one step - see [`weather`].
    pub fn advance_weather(
        &mut self,
        effects: &psys::Library,
        dt: f32,
        camera: psys::field::Frame,
        section: i32,
    ) {
        self.weather.advance(effects, dt, camera, section);
    }

    /// The pool, for the renderer and for tests.
    #[must_use]
    pub fn stage(&self) -> &psys::Stage {
        &self.stage
    }

    /// The placements this race carries.
    #[must_use]
    pub fn placed(&self) -> &[Placed] {
        &self.placed
    }

    /// How many placements are playing now.
    #[must_use]
    pub fn playing_count(&self) -> usize {
        self.playing.iter().flatten().count()
    }
}

impl super::Race {
    /// The weather's one step: where the camera is and the section the drawn
    /// craft is in, as `cam+0x1e8` publishes it through `FUN_08878644`.
    ///
    /// A craft the spline cannot place keeps the section it last published -
    /// chosen, not measured: the original's field is always a real section.
    pub(super) fn advance_weather(&mut self, dt: f32) {
        let section = self
            .station_camera_section()
            .unwrap_or_else(|| self.section_of_slot(self.player_slot()));
        let section = if section == oag_render::pvs::UNPLACED {
            self.view.scenery_fx.weather().section()
        } else {
            i32::from(section)
        };
        let camera = self.camera_frame();
        self.view
            .scenery_fx
            .advance_weather(&self.view.effects, dt, camera, section);
    }

    /// The lens as a world frame: where it is and where its axes point.
    pub(super) fn camera_frame(&self) -> psys::field::Frame {
        let world = self.view().inverse();
        psys::field::Frame {
            position: world.w_axis.truncate(),
            right: world.x_axis.truncate(),
            up: world.y_axis.truncate(),
            back: world.z_axis.truncate(),
        }
    }
}
