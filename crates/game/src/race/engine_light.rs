//! Wipeout HD's engine light: one SPU vertex light per craft, behind its
//! nozzle, brightening with the boost.
//!
//! **This is `EngineFlare_SubmitSpuLight` (`0x0029ff28` in `ps3-hdfury-eu`),
//! read in full** - `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The
//! captured buffer's producer is found", confidence 85. Once per craft per
//! frame it submits one candidate to `SpuLight_AddCandidate`:
//!
//! ```text
//! anchor   = flare_node.position - flare_node.z_axis * Distance
//! position = anchor + (U(-0.1, 0.1), U(-0.1, 0.1), 0)
//! D        = Radius + U(-0.1, 0.1)
//! w        = 1.0
//! base     = Fury skin ? (40, 10, 4) : (4, 10, 40)
//! colour   = base * (1 + 10 * boost_blend)
//! ```
//!
//! where `Distance`/`Radius` are the ship directory's `EngineLightData.xml`
//! (`crate::livery::engine_light`), the flare node is the `Engine Flare`
//! locator under the craft's own world matrix, the Fury-skin byte is the
//! one `Ship_SetFuryTrailFlag` writes (the same flag that turns the trail
//! red - `RaceView::hd_trail_red`), and `boost_blend` is the sum
//! `EngineFlare_PlaceShapes` scales `EF_Main`/`EF_Boost` by - the boost snap
//! and its `*0.8`-per-substep decay, `oag_render::exhaust::hd::Flame`. The
//! jitter is a `vec4` the original rewrites every `EngineFlare_Update` tick
//! from its uniform RNG (`FUN_0028c660`), `(x, y, 0, D)`, range `0x3dcccccd`
//! = 0.1 either side.
//!
//! Every one of those numbers is corroborated by the live capture in
//! `data/traces/hd-spu-light-companion/`: 40 records, all `w = 1.0`, 21
//! exactly `(40, 10, 4)`, the rest `(40, 10, 4) * (1 + 10 * blend)` with
//! `blend` at the snap or on its decay tail, and `D` in `0.62`-`2.05` =
//! `Radius` 0.7-2.0 plus the jitter.
//!
//! # What is this project's
//!
//! - **The RNG.** All randomness here goes through [`oag_core::Rng`], one
//!   stream per craft seeded off [`ENGINE_LIGHT_SEED`], drawn three times per
//!   60 Hz tick. The original draws per `EngineFlare_Update` call; how many
//!   of those run per frame was not counted, and the jitter is noise either
//!   way. **Chosen, not measured.**
//! - **Not the blue-to-orange transition tint.** `EngineFlare_SubmitSpuLight`
//!   has a second branch, selected by a countdown at `flare+0x240` and a flag
//!   at `+0x2b4`, that fades `(4, 10, 40)` to `(80, 10, 0)`; what arms it is
//!   unread, so it stays unwired rather than fired on a guess. A capture
//!   showed it once, on a different boot.
//!
//! # Who receives it, and why the floor is not it
//!
//! [`Race::hd_engine_lights`] is bound into the `Scene` uniform of the track
//! chunks **and of the craft**. Bound to the track alone it changed zero
//! pixels of an Amphiseum frame, and the numbers say it never can at ride
//! height: a craft rides at `ride_height * 0.75` = 4.1 units (Feisar) above
//! the floor while `D` is 0.7-2.0 and the light sits at or inside the nozzle
//! (`Distance` runs to -0.4). **The original's own records agree**, measured
//! rather than inferred: every one of the 40 live-captured light positions in
//! `data/traces/hd-spu-light-companion/` sits 2.98-4.41 units (mean 4.03)
//! from the nearest collision triangle of the circuit it was captured on,
//! against 2.85-4.53 (mean 4.09) for this project's lights on the same
//! circuit over a 1,200-tick race - `crates/game/examples/hd_engine_light_reach_probe.rs`.
//! Not one record on either side is within its own `D` of the floor. (That
//! circuit is **Talon's Junction**, not the Amphiseum renderer.md's capture
//! entry names: scored against all sixteen circuits' collision soups,
//! `hd_engine_light_which_circuit.rs`, Talon's Junction puts 40 of 40
//! records 3-4.4 units off a surface and Amphiseum 3 of 40 within 5.)
//!
//! So at rest the only surface within `D` is the craft's own engine housing,
//! which is what a per-ship `Distance` reads as being tuned for, and what
//! the hull *can* take: `scripts/ps3-sho.py svc-twins <image>
//! materials/ships` finds 74 ship materials compiled with `SVC1` twins
//! (1,776 pairs, every `RigidBody` class included, all 888 vertex blocks
//! carrying the `(255, 128)` decode and the `0x868f8229` attribute). The
//! picture agrees: the Fury housings take the warm tint the reference
//! capture shows, brighter on boost. What is still not measured is the
//! runtime half - whether `LightCulling` sets a hull chunk's `SVC1` bit -
//! and that is the one RPCS3 read that would settle it. The track floor and
//! walls light up only when a craft is within `D` of them: landings, wall
//! scrapes and banked sections.

use oag_core::Rng;
use oag_core::math::Vec3;
use oag_gameplay::MAX_SHIPS;
use oag_render::mesh_render::SpuLight;

use super::Race;
use super::drawable::model_matrix_of;
use crate::livery::engine_light::EngineLight;

/// Seed for the engine light's jitter draws. Distinct from every other seed
/// in `race.rs` for the same determinism reason those are distinct from
/// each other: three floats a tick that the simulation must never see, and
/// that must not shift `EXHAUST_SEED`'s stream, which captures already pin.
pub const ENGINE_LIGHT_SEED: u64 = 0x5_9a_2b_04;

/// The per-craft seed: [`ENGINE_LIGHT_SEED`] plus the slot, on the same
/// SplitMix64-decorrelated terms as `race::effects::exhaust_seed`.
pub(super) fn engine_light_seed(slot: usize) -> u64 {
    ENGINE_LIGHT_SEED + slot as u64
}

/// The jitter's half-range: `0x3dcccccd` at `0x008c23cc`/`0x008c23d0`.
const JITTER: f32 = 0.1;

/// The colour at rest, by skin: `(40, 10, 4)` on a Fury skin, `(4, 10, 40)`
/// otherwise - `0x0029fff4`-`0x002a0010` / `0x002a00a4`-`0x002a00c0`.
const FURY_BASE: [f32; 3] = [40.0, 10.0, 4.0];
const CLASSIC_BASE: [f32; 3] = [4.0, 10.0, 40.0];

/// `colour = base * (blend * 10.0 + 1.0)` - the `10.0` is `0x4120` at
/// `0x002a01ec`.
const BOOST_GAIN: f32 = 10.0;

/// The falloff exponent every record carries: `0x008b2ef4 = 1.0`.
const EXPONENT: f32 = 1.0;

/// One craft's jitter this tick, in the original's own lane order:
/// `(x, y, 0, D)`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(super) struct Jitter {
    pub x: f32,
    pub y: f32,
    pub d: f32,
}

impl Jitter {
    /// Three draws of `U(-0.1, 0.1)`.
    pub(super) fn draw(rng: &mut Rng) -> Self {
        let mut uniform = || (rng.next_f32() * 2.0 - 1.0) * JITTER;
        Self {
            x: uniform(),
            y: uniform(),
            d: uniform(),
        }
    }
}

/// The record one craft submits, from its authored light, its live pose,
/// its skin and its boost blend - the arithmetic of `EngineFlare_SubmitSpuLight`
/// with nothing added.
///
/// `model` is the craft's world matrix; `nozzle` and `axis` are the flare
/// locator's position and Z axis in the craft's model space, so
/// `model * (nozzle - axis * Distance)` is the original's
/// `node.row3 - node.row2 * dist` on the node's world matrix - the craft's
/// `0.75` scale included, exactly as the row carries it. `Radius` is **not**
/// scaled: the live `D` spans `Radius +- 0.1` unscaled (0.62-2.05 against
/// 0.7-2.0), which is what pins that.
pub(super) fn record(
    model: oag_core::math::Mat4,
    nozzle: Vec3,
    light: EngineLight,
    fury: bool,
    boost_blend: f32,
    jitter: Jitter,
) -> SpuLight {
    let anchor = model.transform_point3(nozzle - light.axis * light.data.distance);
    let position = anchor + Vec3::new(jitter.x, jitter.y, 0.0);
    let base = if fury { FURY_BASE } else { CLASSIC_BASE };
    let gain = boost_blend * BOOST_GAIN + 1.0;
    SpuLight {
        position: [position.x, position.y, position.z, EXPONENT],
        colour: [
            base[0] * gain,
            base[1] * gain,
            base[2] * gain,
            light.data.radius + jitter.d,
        ],
    }
}

impl Race {
    /// Redraws one craft's jitter for this tick - the original's
    /// `EngineFlare_Update` rewrite of `flare+0x250`. Called from the exhaust
    /// tick, HD only.
    pub(super) fn advance_engine_light_jitter(&mut self, slot: usize) {
        self.view.engine_light_jitter[slot] = Jitter::draw(&mut self.view.engine_light_rng[slot]);
    }

    /// This frame's SPU vertex lights: one per active craft with an authored
    /// engine light, or empty off HD, on a circuit that switches them off
    /// (`Setup::spu_vertex_lights`), and for every craft without one.
    #[must_use]
    pub fn hd_engine_lights(&self) -> Vec<SpuLight> {
        if !self.view.hd_trail_active || !self.view.spu_vertex_lights {
            return Vec::new();
        }
        (0..MAX_SHIPS)
            .filter(|&slot| self.ship_active(slot))
            .filter_map(|slot| {
                let light = self.view.engine_lights.get(slot).copied().flatten()?;
                let nozzle = self.nozzle_local_of(slot)?;
                Some(record(
                    model_matrix_of(&self.sim.world.ships[slot]),
                    nozzle,
                    light,
                    self.view.hd_trail_red[slot] > 0.5,
                    self.view.hd_flame[slot].boost_blend(),
                    self.view.engine_light_jitter[slot],
                ))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::Mat4;

    const NO_JITTER: Jitter = Jitter {
        x: 0.0,
        y: 0.0,
        d: 0.0,
    };

    fn light(distance: f32, radius: f32) -> EngineLight {
        EngineLight {
            data: oag_tables::enginelight::EngineLightData { distance, radius },
            axis: Vec3::Z,
        }
    }

    /// A Fury craft at rest is the literal the capture held 21 times.
    #[test]
    fn a_fury_engine_at_rest_is_forty_ten_four() {
        let light = record(
            Mat4::IDENTITY,
            Vec3::ZERO,
            light(0.0, 1.0),
            true,
            0.0,
            NO_JITTER,
        );
        assert_eq!(light.colour, [40.0, 10.0, 4.0, 1.0]);
        assert_eq!(light.position, [0.0, 0.0, 0.0, 1.0]);
    }

    /// The snap: `blend = 1.0` is `(440, 110, 44)`, the capture's `k = 11`.
    #[test]
    fn the_boost_snap_is_eleven_times_the_base() {
        let light = record(
            Mat4::IDENTITY,
            Vec3::ZERO,
            light(0.0, 1.0),
            true,
            1.0,
            NO_JITTER,
        );
        assert_eq!(light.colour, [440.0, 110.0, 44.0, 1.0]);
    }

    /// A classic skin swaps the red and blue lanes.
    #[test]
    fn a_classic_skin_is_blue() {
        let light = record(
            Mat4::IDENTITY,
            Vec3::ZERO,
            light(0.0, 1.0),
            false,
            0.0,
            NO_JITTER,
        );
        assert_eq!(light.colour, [4.0, 10.0, 40.0, 1.0]);
    }

    /// `Distance` slides the anchor back along the locator's own axis, and
    /// the craft's matrix scale rides on that slide but not on `Radius`.
    #[test]
    fn distance_slides_along_the_axis_through_the_craft_matrix_and_radius_does_not() {
        let model = Mat4::from_scale(Vec3::splat(0.75));
        let light = record(
            model,
            Vec3::new(0.0, 0.0, -4.0),
            light(2.0, 1.5),
            true,
            0.0,
            NO_JITTER,
        );
        // (0, 0, -4) - (0, 0, 1) * 2 = (0, 0, -6), scaled by 0.75.
        assert_eq!(light.position, [0.0, 0.0, -4.5, 1.0]);
        assert_eq!(light.colour[3], 1.5);
    }

    /// The jitter lands on `x`, `y` and `D` and never on `z`.
    #[test]
    fn jitter_moves_x_y_and_range_only() {
        let jitter = Jitter {
            x: 0.05,
            y: -0.05,
            d: 0.1,
        };
        let light = record(
            Mat4::IDENTITY,
            Vec3::ZERO,
            light(0.0, 1.0),
            true,
            0.0,
            jitter,
        );
        assert_eq!(light.position, [0.05, -0.05, 0.0, 1.0]);
        assert_eq!(light.colour[3], 1.1);
    }

    /// Every draw stays inside the authored half-range.
    #[test]
    fn draws_stay_within_a_tenth() {
        let mut rng = Rng::new(engine_light_seed(3));
        for _ in 0..1000 {
            let j = Jitter::draw(&mut rng);
            for v in [j.x, j.y, j.d] {
                assert!((-JITTER..=JITTER).contains(&v), "{v}");
            }
        }
    }
}
