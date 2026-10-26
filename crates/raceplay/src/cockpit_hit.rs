//! The static a weapon hit throws over the player's cockpit view: `CockpitHitFx`
//! (`oag_fx::cockpit_hit`) armed where the camera shake is.
//!
//! `Ship_Damage` (`0x088439ac`) arms both at the end of its `source == 2`
//! block, on every weapon hit to the human (`+0x368 == 0`), outside game modes 2
//! and 12 - this port races in neither. The node runs in every camera view and
//! is only *drawn* in the internal one (`CockpitHitFx_Enqueue` tests
//! `*(g_race_manager + 0x2c0) + 0x6d`), so a hit in the chase view and a switch
//! to the cockpit inside the next 0.6 s shows the rest of the run.
//!
//! **Chosen, not measured:** the player's view alone wears it, as the screen
//! flash does (`Race::screen_wash`): an extra seat's view is a chase camera and
//! has no internal view to draw it in. The offsets are drawn from a seeded
//! generator of this race's own once a tick, where the original draws them every
//! frame from the particle system's. A title with no
//! [`oag_title::weapons::WeaponModels::cockpit_hit`], or whose noise failed to
//! load, holds no node and draws nothing.

use super::*;

/// Seed for the offset draws. View-side, like every other effect seed.
pub const COCKPIT_HIT_SEED: u64 = 0x5_9a_2b_05;

/// The node, its generator and the noise it is drawn from.
#[derive(Debug)]
pub(super) struct CockpitHit {
    node: oag_fx::cockpit_hit::Node,
    rng: Rng,
    noise: oag_fx::exhaust::FlareTexture,
}

impl CockpitHit {
    pub(super) fn new(noise: oag_fx::exhaust::FlareTexture) -> Self {
        Self {
            node: oag_fx::cockpit_hit::Node::default(),
            rng: Rng::new(COCKPIT_HIT_SEED),
            noise,
        }
    }
}

impl Race {
    /// `CockpitHitFx_Arm` for a weapon hit on `slot`: the human's own craft
    /// only. An arm while a run is going is ignored by the node.
    pub(super) fn arm_cockpit_hit(&mut self, slot: usize) {
        if slot != self.player_slot() {
            return;
        }
        if let Some(hit) = &mut self.view.cockpit_hit {
            hit.node.arm();
        }
    }

    /// `CockpitHitFx_Update`, once a drawn tick.
    pub(super) fn advance_cockpit_hit(&mut self) {
        let dt = self.sim.dt;
        if let Some(hit) = &mut self.view.cockpit_hit {
            hit.node.advance(dt, &mut hit.rng);
        }
    }

    /// The static to draw in the view being drawn, or `None` with none: no
    /// node, no run, an extra seat's view, or any camera but the internal one.
    #[must_use]
    pub fn cockpit_hit_strips(
        &self,
    ) -> Option<[oag_fx::cockpit_hit::Vertex; oag_fx::cockpit_hit::VERTICES]> {
        if self.extra_viewpoint().is_some()
            || self.view.camera_view != oag_display::display::CameraView::Internal
        {
            return None;
        }
        self.view.cockpit_hit.as_ref()?.node.strips()
    }

    /// The noise [`Self::cockpit_hit_strips`] are drawn with: `None` on a title
    /// that draws no static.
    #[must_use]
    pub fn cockpit_hit_noise(&self) -> Option<&oag_fx::exhaust::FlareTexture> {
        Some(&self.view.cockpit_hit.as_ref()?.noise)
    }

    /// Whether a run is going, in any camera view, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn cockpit_hit_running_for_tests(&self) -> bool {
        self.view
            .cockpit_hit
            .as_ref()
            .is_some_and(|hit| hit.node.running())
    }
}
