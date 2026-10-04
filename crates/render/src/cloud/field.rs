//! One `cloudGroup`'s field: the records `FUN_08933c1c` scatters through each
//! cube, the `Overlap` cull, the colour ramp and the per-sprite draws, in the
//! order the original makes them.
//!
//! Every step is read on `docs/ghidra/functions/psp-pulse-usa/clouds.md`, and
//! the whole chain was checked against a running original: fed the seed each
//! group kept, it reproduces all 369 authored records on `05_Track`, the cull
//! (14, 14 and 74 kept), every variant, phase, rate and baked colour.

use oag_vex::cloud::{CloudAttributes, CloudGroup, GroupCube};

use crate::ranrot::Ranrot;

/// `DAT_08a88a90`: a cube authors one record per ten units of its volume.
const RECORDS_PER_VOLUME: f32 = 0.1;

/// `DAT_08a88a94`: a unit cube's edge in the editor's units. It scales both
/// the scatter (`+/- 10 * 0.5` per axis, in the cube's own frame) and the
/// half-size (`scale * 10`).
const CUBE_UNIT: f32 = 10.0;

/// `0x40c90fdb`, the upper bound of the phase draw.
const PHASE_MAX: f32 = 6.283_185_5;

/// `0x3b03126f`, the bound of the rate draw.
const RATE_MAX: f32 = 0.002;

/// `0x3e490fdb`, the bound of `kind == 3`'s phase draw (`PI / 16`).
const KIND3_PHASE_MAX: f32 = 0.196_349_54;

/// One authored record: `FUN_08931c3c`'s output before the cull.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Record {
    /// World position.
    pub position: [f32; 3],
    /// Half-size, world units.
    pub half_size: f32,
    /// Index of the cube that authored it in [`CloudGroup::cubes`], or `None`
    /// once the cull has removed it (the original writes `-1`).
    pub cube: Option<usize>,
}

/// One sprite as `CloudGroup_BuildDisplayList` bakes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Baked {
    /// World position.
    pub position: [f32; 3],
    /// Half-size, world units.
    pub half_size: f32,
    /// The baked vertex colour, `R` in the low byte, as the GE reads it.
    pub colour: u32,
    /// Which cell of the 4x2 texture atlas it draws (`FUN_089322f8`).
    pub cell: u8,
    /// Starting phase, radians.
    pub phase: f32,
    /// Phase added per draw.
    pub rate: f32,
}

/// Builds `group`'s field from `seed`, the way `FUN_08933c1c` does after
/// `PsysRng_Reseed(seed)`.
#[must_use]
pub fn build(group: &CloudGroup, seed: u32) -> Vec<Baked> {
    let mut rng = Ranrot::reseeded(seed);
    let mut records = scatter(&group.cubes, &mut rng);
    cull(&mut records, group.attributes.overlap);
    let ramp = Ramp::new(&group.attributes, &records);
    records
        .iter()
        .filter_map(|record| {
            let cube = record.cube?;
            let colour = ramp.bake(record.position[1], record.half_size);
            let (cell, phase, rate) = draws(group.cubes[cube].kind, &mut rng);
            Some(Baked {
                position: record.position,
                half_size: record.half_size,
                colour,
                cell,
                phase,
                rate,
            })
        })
        .collect()
}

/// Length of each of a matrix's first three rows, as `CloudCube_Init`
/// (`0x08931b28`) measures them with `vdot_t`/`vsqrt_s`.
fn row_lengths(m: &[f32; 16]) -> [f32; 3] {
    std::array::from_fn(|r| {
        let row = &m[r * 4..r * 4 + 3];
        (row[0] * row[0] + row[1] * row[1] + row[2] * row[2]).sqrt()
    })
}

/// How many records a cube authors: `FUN_08931bec`, its volume times
/// [`RECORDS_PER_VOLUME`], truncated.
#[must_use]
pub fn record_count(cube: &GroupCube) -> usize {
    let [sx, sy, sz] = row_lengths(&cube.world);
    let volume = sx * sy * sz;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let n = (volume * RECORDS_PER_VOLUME).max(0.0) as usize;
    n
}

/// `FUN_08931c3c` per cube, in collection order: three axis draws, then the
/// half-size draw, per record.
#[must_use]
pub fn scatter(cubes: &[GroupCube], rng: &mut Ranrot) -> Vec<Record> {
    let mut out = Vec::new();
    for (index, cube) in cubes.iter().enumerate() {
        let lengths = row_lengths(&cube.world);
        let row = |r: usize| {
            [
                cube.world[r * 4],
                cube.world[r * 4 + 1],
                cube.world[r * 4 + 2],
            ]
        };
        let span = cube.sprite_radius * cube.sprite_radius_var;
        let size_scale = cube.scale * CUBE_UNIT;
        for _ in 0..record_count(cube) {
            let offset: [f32; 3] = std::array::from_fn(|axis| {
                let s = lengths[axis];
                // A negative length would empty the range; the original
                // zeroes it, which a square root never needs.
                let (lo, hi) = if s < -s { (0.0, 0.0) } else { (-s, s) };
                rng.range(lo, hi) / s * CUBE_UNIT * 0.5
            });
            let (r0, r1, r2, t) = (row(0), row(1), row(2), row(3));
            let position = std::array::from_fn(|k| {
                r0[k] * offset[0] + r1[k] * offset[1] + r2[k] * offset[2] + t[k]
            });
            let half_size = (cube.sprite_radius + rng.range(-span, span)) * size_scale;
            out.push(Record {
                position,
                half_size,
                cube: Some(index),
            });
        }
    }
    out
}

/// `CloudGroup_CullOverlappingSprites` (`0x08932eac`): every live record
/// removes each later live one that `CloudGroup_SpritesOverlap` (`0x08932f98`)
/// finds closer than `(ha + hb) * (1 - overlap)`.
pub fn cull(records: &mut [Record], overlap: f32) {
    for a in 0..records.len() {
        if records[a].cube.is_none() {
            continue;
        }
        for b in a + 1..records.len() {
            if records[b].cube.is_none() {
                continue;
            }
            let threshold = (records[a].half_size + records[b].half_size) * (1.0 - overlap);
            let d: [f32; 3] =
                std::array::from_fn(|k| records[a].position[k] - records[b].position[k]);
            let distance_sq = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
            if threshold * threshold > distance_sq {
                records[b].cube = None;
            }
        }
    }
}

/// `CloudGroup_SampleColourRamp` (`0x08932ba0`) over the group's height range.
#[derive(Debug, Clone, Copy)]
struct Ramp {
    keys: [[f32; 4]; 3],
    breaks: [f32; 3],
    reciprocals: [f32; 2],
    low: f32,
    high: f32,
}

impl Ramp {
    /// The three keys `CloudGroup_Init` lays out (`Lo`, `Mid` with alpha
    /// `(Hi + Lo) / 2`, `Hi`), and the vertical extent `FUN_08933c1c`
    /// measures: the lowest `y - h` and highest `y + h` over **every** authored
    /// record. Its live-record test reads record 0's flag on every pass
    /// (`0x08933d74`, no index), and record 0 is never culled, so culled
    /// records widen the range too; the live box matched that, not the
    /// survivors'.
    fn new(a: &CloudAttributes, records: &[Record]) -> Self {
        let keys = [
            [a.lo_colour[0], a.lo_colour[1], a.lo_colour[2], a.lo_alpha],
            [
                a.mid_colour[0],
                a.mid_colour[1],
                a.mid_colour[2],
                (a.hi_alpha + a.lo_alpha) * 0.5,
            ],
            [a.hi_colour[0], a.hi_colour[1], a.hi_colour[2], a.hi_alpha],
        ];
        let breaks = [0.0, a.midpoint, 1.0];
        let reciprocal = |span: f32| if span == 0.0 { 0.0 } else { 1.0 / span };
        let low = records
            .iter()
            .map(|r| r.position[1] - r.half_size)
            .fold(1.0e20, f32::min);
        let high = records
            .iter()
            .map(|r| r.position[1] + r.half_size)
            .fold(-1.0e20, f32::max);
        Self {
            keys,
            breaks,
            reciprocals: [
                reciprocal(breaks[1] - breaks[0]),
                reciprocal(breaks[2] - breaks[1]),
            ],
            low,
            high,
        }
    }

    fn sample(&self, y: f32) -> [f32; 4] {
        let t = ((y - self.low) / (self.high - self.low)).clamp(0.0, 1.0);
        for i in 1..3 {
            if t <= self.breaks[i] {
                let k = (t - self.breaks[i - 1]) * self.reciprocals[i - 1];
                let (a, b) = (self.keys[i - 1], self.keys[i]);
                return std::array::from_fn(|c| a[c] + k * (b[c] - a[c]));
            }
        }
        self.keys[2]
    }

    /// `CloudGroup_BuildDisplayList`'s bake: the ramp at the sprite's bottom
    /// and top edges, averaged, each channel times 255 truncated to a byte.
    fn bake(&self, y: f32, half_size: f32) -> u32 {
        let bottom = y - half_size;
        let (a, b) = (self.sample(bottom), self.sample(bottom + half_size * 2.0));
        let byte = |c: usize| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let v = (((a[c] + b[c]) * 0.5) * 255.0) as u32;
            v & 0xff
        };
        byte(3) << 24 | byte(2) << 16 | byte(1) << 8 | byte(0)
    }
}

/// The per-sprite draws, by the cube's `kind`, in the order
/// `CloudGroup_BuildDisplayList` makes them: atlas cell, phase, rate.
///
/// Only `kind == 2` ships. A kind outside `0..=3` makes no draw in the
/// original and reads a zeroed record, so it lands on cell 0, phase 0, rate 0.
fn draws(kind: u32, rng: &mut Ranrot) -> (u8, f32, f32) {
    match kind {
        0 => {
            let cell = [0, 1, 2, 3][index(rng.int_range(0, 3))];
            (
                cell,
                rng.range(0.0, PHASE_MAX),
                rng.range(-RATE_MAX, RATE_MAX),
            )
        }
        1 => {
            let cell = [4, 5][index(rng.int_range(0, 1))];
            (cell, rng.range(0.0, PHASE_MAX), 0.0)
        }
        2 => {
            let cell = [6, 7][index(rng.int_range(0, 1))];
            (
                cell,
                rng.range(0.0, PHASE_MAX),
                rng.range(-RATE_MAX, RATE_MAX),
            )
        }
        3 => {
            let _ = rng.int_range(0, 0);
            (0, rng.range(-KIND3_PHASE_MAX, KIND3_PHASE_MAX), 0.0)
        }
        _ => (0, 0.0, 0.0),
    }
}

fn index(i: i32) -> usize {
    usize::try_from(i).unwrap_or(0)
}

/// The atlas cell `cell` covers, as `(u0, v0, u1, v1)`: `FUN_089322f8`'s 4x2
/// grid of quarter-width, half-height cells over the 128x64 cloud texture.
#[must_use]
pub fn cell_uv(cell: u8) -> (f32, f32, f32, f32) {
    let col = f32::from(cell % 4);
    let row = f32::from(cell / 4);
    let (u0, v0) = (col * 0.25, row * 0.5);
    (u0, v0, u0 + 0.25, v0 + 0.5)
}
