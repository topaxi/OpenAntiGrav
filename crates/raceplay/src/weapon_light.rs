//! Wipeout HD's weapon point lights: the same `SpuLight_AddCandidate`
//! (`0x0040d990`, reached through the `0x006778c8` thunk) the engines submit
//! to, fed by the weapons' own draw functions.
//!
//! # The call
//!
//! `0x006778c8(v2 = position, v3 = colour, f1 = D, f2 = w)`: the candidate
//! store writes `f2` to the record's quad-1 `w` (the falloff exponent) and
//! `f1` to quad-2 `w` (the range), the colour vector's lanes to `r, g, b`
//! (`0x0040da38`-`0x0040dab0`, read in full). There is no light slot. The SPU
//! sums `max(0, 1 - |d| / D)^w * max(0, N.L) * colour` per vertex, which is
//! `oag_mesh::mesh_render::SpuLights` and `spu_light_sum`; the weapons only
//! add records to that list.
//!
//! # The Missile explosion (`0x00155420`, vtable slot 5; confidence 85)
//!
//! With `f = +0x17c = (1 - p)^2` over the entry's 1.0 s age `p`:
//!
//! ```text
//! position = row 3 of the matrix at this + 0xf0 + viewport * 0x40
//! D        = f * 150.0        (0x008ab258)
//! w        = f * 7.0 + 1.5    (0x008ab25c, 0x008ab260)
//! colour   = (500, 200, 50)   (0x43fa0000, 0x43480000, 0x42480000; seeded once)
//! ```
//!
//! # The Rocket (`Rocket_Update`, `0x00123fb0`, call at `0x001246cc`)
//!
//! `D = 100.0` (`0x008aa178`), `w = 1.0` (`0x008aa128`), colour
//! `(14, 10, 2)`: `0x4160` and `0x4120` are the first two lanes, and the
//! third is `r15`, which `lis r15, 0x4000` set to `2.0` at `0x00124540`, on
//! the only path into the store. One light per update while the rocket flies.
//!
//! # What is chosen, not measured
//!
//! - **The Rocket's position** is the projectile's own position. The original
//!   builds it as a vector plus a scaled direction (`0x00124474`-`0x001244c8`),
//!   unresolved.
//! - **No cap or compaction.** The original keeps eight visible records;
//!   `SpuLights` passes every record to the vertex stage.
//! - **One viewport.** The original calls once per viewport that sees the
//!   object.

use oag_core::math::Vec3;
use oag_mesh::mesh_render::SpuLight;
use oag_tables::weapons::Weapon;

use super::Race;
use super::missile_blast::LIFETIME_SECONDS;

const MISSILE_COLOUR: [f32; 3] = [500.0, 200.0, 50.0];
const MISSILE_RANGE: f32 = 150.0;
const MISSILE_EXPONENT_SLOPE: f32 = 7.0;
const MISSILE_EXPONENT_BASE: f32 = 1.5;

const ROCKET_COLOUR: [f32; 3] = [14.0, 10.0, 2.0];
const ROCKET_RANGE: f32 = 100.0;
const ROCKET_EXPONENT: f32 = 1.0;

fn record(position: Vec3, colour: [f32; 3], range: f32, exponent: f32) -> SpuLight {
    SpuLight {
        position: [position.x, position.y, position.z, exponent],
        colour: [colour[0], colour[1], colour[2], range],
    }
}

/// The Missile explosion's light at age `age` seconds, or `None` once the
/// entry is spent.
pub(crate) fn missile_record(position: Vec3, age: f32) -> Option<SpuLight> {
    let p = age / LIFETIME_SECONDS;
    if !(0.0..1.0).contains(&p) {
        return None;
    }
    let f = (1.0 - p) * (1.0 - p);
    Some(record(
        position,
        MISSILE_COLOUR,
        f * MISSILE_RANGE,
        f * MISSILE_EXPONENT_SLOPE + MISSILE_EXPONENT_BASE,
    ))
}

/// A flying Rocket's light.
pub(crate) fn rocket_record(position: Vec3) -> SpuLight {
    record(position, ROCKET_COLOUR, ROCKET_RANGE, ROCKET_EXPONENT)
}

impl Race {
    /// Every SPU vertex light this frame: the engines', then the weapons'.
    /// Empty off HD and on a circuit that switches the lights off.
    #[must_use]
    pub fn hd_spu_lights(&self) -> Vec<SpuLight> {
        let mut lights = self.hd_engine_lights();
        if !self.view.hd_trail_active || !self.view.spu_vertex_lights {
            return lights;
        }
        lights.extend(self.hd_weapon_lights());
        lights
    }

    fn hd_weapon_lights(&self) -> Vec<SpuLight> {
        let missiles = self.hd_missile_blast_lights();
        let rockets = self
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .filter(|projectile| projectile.kind == Some(Weapon::Rocket))
            .map(|projectile| rocket_record(projectile.position));
        missiles.chain(rockets).collect()
    }
}

#[cfg(test)]
mod tests;
