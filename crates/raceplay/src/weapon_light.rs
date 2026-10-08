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
//! # The Rocket (`0x00123de8`, its draw; read live)
//!
//! One light per rocket per frame while it is drawn: `D = 50.0`
//! (`0x008aa124`), `w = 1.0` (`0x008aa128`), colour `(7, 5, 1)` (seeded once
//! from `0x40e00000`, `0x40a00000`, `0x3f800000`), position the lanes of the
//! rocket's own row at `this + 0x90`. **Read live (RPCS3, Talon's Junction,
//! one volley)**: while rockets flew, the visible buffer held the player's
//! engine light `(4, 10, 40)` and three `(7, 5, 1)` records of `D = 50`,
//! `w = 1` at the three rockets' positions, which then ran off down the
//! track; `scratch/hd-weapon-lights/live1/snaps.json`.
//!
//! **Not this**: `Rocket_Update`'s own `0x006778c8` call at `0x001246cc`
//! (`D = 100`, colour `(14, 10, 2)`, where `r15 = 2.0` from `lis r15, 0x4000`
//! at `0x00124540`). It sits in the block that allocates a `0x180`-byte object
//! after the `0x0007be58` trace. Six snapshots across the first second of
//! flight held no `D = 100` record, but polling the candidate list live
//! (`hd-blast-fill`, two volleys) finds it: one to three frames, three times a
//! shot from about 0.9 s, 12 to 25 units beside a flying rocket's path (one
//! boot), so it is a contact-style flash, not a launch light and not a steady flight light. It is
//! left unwired: what `0x0007be58` traces, and why every half second, is
//! unread.
//!
//! # The Bomb blast (`0x001512f8`, its draw; confidence 75)
//!
//! Two lights at the blast's centre three units up its axis, both
//! `w = 7 (1 - x) + 1.5` with `x` the blast's `+0x2dc`, and a scale `s` that
//! is the tunables table's `[1]` (`0x008c1aa4 + 4`, written by the update's
//! last phase):
//!
//! ```text
//! light 1: colour (500, 100 + 100 x, 50), D = 40 s
//! light 2: colour (20, 5, 0.5),           D = 100 s
//! ```
//!
//! `x` and `s` are not closed-form in the disassembly (three writers and a
//! `powf`), so they are **read off the original's own update run in the
//! emulator** (`scratch/hd-weapon-blasts/emu.py` stepping `0x001503d8` at
//! 60 Hz and calling `0x001512f8`; `scratch/hd-weapon-lights/bomblight.py`),
//! which every sample below reproduces to four digits:
//!
//! | age (s) | `x` | `s` |
//! | --- | --- | --- |
//! | 0 - 0.1 | 1 | 1 |
//! | 0.1 - 0.8 | `1 - (t - 0.1) / 0.7` | 1 |
//! | 0.8 - 1.5 | `((t - 0.8) / 0.7)^0.25` | 1 |
//! | 1.5 - 2.0 | `(1 - (t - 1.5) / 0.5)^2` | `x` |
//! | 2.0 - 3.0 | 0 | 0 |
//!
//! The four breakpoints and the `0.25` exponent (`0x008ab168`)
//! are the emulated run's; the closed forms are fitted to its samples.
//!
//! # What is chosen, not measured
//!
//! - **The Rocket's position** is the projectile's own position; the original
//!   reads the row at `+0x90` of the rocket's object, which this engine's
//!   `Projectile::position` stands in for.
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

const ROCKET_COLOUR: [f32; 3] = [7.0, 5.0, 1.0];
const ROCKET_RANGE: f32 = 50.0;
const ROCKET_EXPONENT: f32 = 1.0;

const BOMB_LIGHT_ONE: [f32; 3] = [500.0, 100.0, 50.0];
const BOMB_LIGHT_TWO: [f32; 3] = [20.0, 5.0, 0.5];
const BOMB_RANGE_ONE: f32 = 40.0;
const BOMB_RANGE_TWO: f32 = 100.0;
const BOMB_UP: f32 = 3.0;

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

/// The Bomb blast's `(x, s)` at `age` seconds; see the module doc's table.
pub(crate) fn bomb_envelope(age: f32) -> (f32, f32) {
    if age < 0.1 {
        (1.0, 1.0)
    } else if age < 0.8 {
        (1.0 - (age - 0.1) / 0.7, 1.0)
    } else if age < 1.5 {
        (((age - 0.8) / 0.7).powf(0.25), 1.0)
    } else if age < 2.0 {
        let x = (1.0 - (age - 1.5) / 0.5).powi(2);
        (x, x)
    } else {
        (0.0, 0.0)
    }
}

/// The Bomb blast's two lights at `age`.
pub(crate) fn bomb_records(position: Vec3, up: Vec3, age: f32) -> [SpuLight; 2] {
    let (x, s) = bomb_envelope(age);
    let at = position + up * BOMB_UP;
    let w = 7.0 * (1.0 - x) + 1.5;
    let one = [
        BOMB_LIGHT_ONE[0],
        BOMB_LIGHT_ONE[1] + 100.0 * x,
        BOMB_LIGHT_ONE[2],
    ];
    [
        record(at, one, BOMB_RANGE_ONE * s, w),
        record(at, BOMB_LIGHT_TWO, BOMB_RANGE_TWO * s, w),
    ]
}

/// A flying Rocket's light.
pub(crate) fn rocket_record(position: Vec3) -> SpuLight {
    record(position, ROCKET_COLOUR, ROCKET_RANGE, ROCKET_EXPONENT)
}

impl Race {
    /// Switches the weapons' point lights on or off; a verification aid
    /// (`--no-weapon-lights`). On by default.
    pub fn set_weapon_lights(&mut self, on: bool) {
        self.view.hd_weapon_lights = on;
    }

    /// Every SPU vertex light this frame: the engines', then the weapons'.
    /// Empty off HD and on a circuit that switches the lights off.
    #[must_use]
    pub fn hd_spu_lights(&self) -> Vec<SpuLight> {
        let mut lights = self.hd_engine_lights();
        if !self.view.hd_trail_active || !self.view.spu_vertex_lights || !self.view.hd_weapon_lights
        {
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
        let bombs = self
            .view
            .hd_bomb_blasts
            .iter()
            .flatten()
            .flat_map(|blast| bomb_records(blast.position, blast.up, blast.age));
        missiles.chain(rockets).chain(bombs).collect()
    }
}

#[cfg(test)]
mod tests;
