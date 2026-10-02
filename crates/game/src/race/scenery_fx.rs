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

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_render::psys;
use oag_vex::placed_psys::Placed;

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
    placed: Vec<Placed>,
    playing: Vec<Option<psys::Playing>>,
    stage: psys::Stage,
    rng: Rng,
}

impl Default for SceneryFx {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl SceneryFx {
    /// Nothing playing yet; the first [`Self::advance`] starts every effect
    /// the library holds.
    #[must_use]
    pub fn new(placed: Vec<Placed>) -> Self {
        Self {
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
