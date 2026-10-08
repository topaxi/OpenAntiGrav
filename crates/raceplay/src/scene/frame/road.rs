//! The road's own per-frame vertex writes: the Quake's ripple through the
//! track and both pad models, then the pads' uniforms and the weapon pads'
//! tint. Split out of `frame.rs` under the 1,000-line rule; the pad half is a
//! move, with no behaviour change.

use super::super::super::*;
use super::super::Scene;
use oag_render::ripple::{Ripple, Wave};

impl Scene {
    /// Hands each model the road spans a Quake ripples through it - see
    /// [`crate::Ripples`]. A model with none, and every model on a race
    /// that is not Pulse off a PSP disc, keeps still.
    pub fn attach_ripples(&mut self, ripples: crate::Ripples) {
        let attach = |drawable: Option<&mut super::super::Drawable>, ripple: Option<Ripple>| {
            if let Some(drawable) = drawable {
                drawable.set_ripple(ripple);
            }
        };
        attach(Some(&mut self.track), ripples.track);
        attach(self.pads.as_mut(), ripples.pads);
        attach(self.weapon_pads.as_mut(), ripples.weapon_pads);
    }

    /// The Quake's ripple, then the pads.
    ///
    /// **The ripple goes first**: a weapon pad's tint rewrites the pad's
    /// whole node from its authored vertices, adding the ripple's
    /// displacement back itself, so ordering it after the tint would flatten
    /// a pad the bump is crossing for one frame in every frame.
    pub(super) fn write_road(
        &self,
        queue: &wgpu::Queue,
        race: &Race,
        seconds: f32,
        (view_projection, prev_vp): (Mat4, Mat4),
        pads_ready: &mut Vec<bool>,
        recoloured: &mut Vec<oag_mesh::mesh::GpuVertex>,
    ) {
        // The simulation's own wave, drawn - `Race::advance_quake` retires it
        // at five seconds, which is also where the bump's envelope reaches
        // nothing, so the road is flat again by the tick it goes.
        let wave = race
            .sim
            .world
            .quake
            .map(|wave| Wave::at(wave.progress, wave.direction, wave.age));
        for drawable in [
            Some(&self.track),
            self.pads.as_ref(),
            self.weapon_pads.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            drawable.write_ripple(queue, wave, recoloured);
        }

        for pads in [self.pads.as_ref(), self.weapon_pads.as_ref()]
            .into_iter()
            .flatten()
        {
            pads.write(queue, view_projection, Mat4::IDENTITY, prev_vp);
        }
        // Ready-to-collect vs cooling down - see `Drawable::tint_weapon_pads`
        // and `oag_render::weapon_pad` for the recovered mechanism this
        // reproduces. Gameplay state, driven off `seconds` like the scenery.
        if let Some(weapon_pads) = &self.weapon_pads {
            pads_ready.extend(
                race.weapon_pad_refresh_left()
                    .iter()
                    .map(|&left| left <= 0.0),
            );
            match race.sim.weapon_pad_glow {
                oag_title::weapon_pad::WeaponPadGlow::Cycle(cycle) => {
                    weapon_pads.glow_weapon_pads(queue, seconds, cycle, pads_ready);
                }
                oag_title::weapon_pad::WeaponPadGlow::Authored => {
                    weapon_pads.tint_weapon_pads(queue, seconds, pads_ready, recoloured);
                }
            }
        }
    }
}
