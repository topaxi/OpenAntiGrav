//! The wgpu-free half of [`super::Pipeline`]: the Cannon's two hand-built
//! quads' vertex geometry, and nothing that touches a device or a queue.
//!
//! Recovered 2026-09-17 from `Cannon_DrawRound` (`0x0886545c`),
//! `Cannon_BuildBoltList`/`Cannon_BuildMuzzleFlashList` (`0x08864cd0`/
//! `0x08864dc4`), `Cannon_UpdateRound` (`0x0886593c`) and `Cannon_Construct`
//! (`0x088651d8`) on `psp-pulse-usa`. See
//! `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, the
//! 2026-09-17 section, for the instruction-level read this module ports.
//!
//! **The original builds these in a hand-transformed clip-space-like frame**:
//! it transforms the round's previous and current position through the
//! current top-of-stack matrix, resets that matrix to identity, and then adds
//! a constant width straight to the transformed X/Y before the GU rasterises -
//! a common fixed-function trick that keeps a billboard's on-screen size
//! correct under perspective without a per-vertex basis. This project's
//! renderer does not have that hand-transform stage, so the offsets are built
//! from the camera's own `right`/`up` instead - the same port
//! [`crate::exhaust::sprite`] already makes for its own camera-facing quad.
//! The two are equivalent for a camera-facing billboard: adding a constant to
//! clip-space X/Y before the perspective divide and adding `right`/`up` in
//! world space both describe a quad that keeps a fixed *apparent* size formed
//! in the camera's own screen-aligned axes.

use bytemuck::Zeroable;
use oag_core::math::Vec3;

use crate::mesh::GpuVertex;

/// Half-width of the bolt streak's two crossed ribbons, world units.
///
/// `Cannon_Construct` seeds `+0xd4` to this and `Cannon_UpdateRound`
/// re-seeds it every tick (`DAT_08ab1060` = `0x3eb33333` = `0.35`); nothing
/// else ever writes it, so it is a flat constant, not a per-round roll.
/// Confidence 90 (a single literal, read at both writers).
pub const BOLT_HALF_WIDTH: f32 = 0.35;

/// How far back toward the current position the streak's *near* end sits,
/// as a fraction of the previous-to-current segment measured from the
/// current end.
///
/// `Cannon_DrawRound` computes `near = curr + FRACTION * (prev - curr)`
/// (`DAT_08ab1064` = `0x3e4ccccd` = `0.2`) and draws the streak from `near`
/// to `prev` - the closest fifth of the inter-tick segment is left
/// uncovered, which reads as leaving room for the round's own dart mesh at
/// its nose. Confidence 85: the arithmetic is read at instruction level: the
/// *reason* for the gap is inferred, not measured.
pub const BOLT_NEAR_FRACTION: f32 = 0.2;

/// Seconds since spawn the muzzle flash quad draws for.
///
/// `Cannon_DrawRound`'s own gate: `*(round + 0xc8) < 0.1`. Confidence 88 -
/// see "Which display list is the bolt and which the flash" on the evidence
/// page.
pub const FLASH_WINDOW_SECONDS: f32 = 0.1;

/// The flash's random half-size range, world units, **before**
/// [`FLASH_SIZE_SCALE`].
///
/// `Cannon_UpdateRound` rerolls `+0xcc` every tick inside the flash window
/// with `Psys_RandFloatRange(DAT_08ab105c, DAT_08ab1058)` =
/// `Psys_RandFloatRange(0.65, 1.3)`. Confidence 85 (both literals read
/// directly; the PRNG stream itself is not reproduced - see
/// [`super::random`]).
pub const FLASH_SIZE_RANGE: (f32, f32) = (0.65, 1.3);

/// `Cannon_DrawRound` multiplies the rolled half-size by this before using
/// it: `fVar10 = *(round + 0xcc) * 3.0`. Confidence 88 (the literal at the
/// draw site itself).
pub const FLASH_SIZE_SCALE: f32 = 3.0;

/// The flash's random alpha range, as the 8-bit channel value
/// `Psys_RandIntRange(0x96, 0xff)` returns in `Cannon_DrawRound`, rerolled
/// every frame the flash draws. The colour channels are **not** randomised -
/// `iVar9 * 0x1000000 + 0xffffff` sets RGB to `0xffffff` (opaque white) and
/// only the top byte (alpha) to the rolled value, so "randomly coloured" in
/// the handover thread's framing is corrected here to "randomly *faded*, a
/// fixed white". Confidence 88.
pub const FLASH_ALPHA_RANGE: (u8, u8) = (0x96, 0xff);

/// The bolt's fixed vertex colour: opaque white, `+0xd8 = 0xffffffff`,
/// written once in `Cannon_Construct` and never touched again. Confidence 90.
pub const BOLT_COLOUR: [f32; 4] = [1.0, 1.0, 1.0, 1.0];

/// One camera-facing ribbon between `near` and `far`, `width` wide along
/// `width_vec`, in the same six-vertex triangle-list shape
/// [`crate::exhaust::sprite::quad`] uses.
///
/// `width_vec` carries both the axis and the half-width, so the two crossed
/// quads [`bolt_vertices`] builds pass `(right - up) * `[`BOLT_HALF_WIDTH`]
/// and `(right + up) * `[`BOLT_HALF_WIDTH`] - the two diagonals
/// `Cannon_BuildBoltList`'s pair of quads reads as, at 90 degrees to each
/// other, forming a camera-facing cross section rather than a flat ribbon.
fn ribbon_quad(near: Vec3, far: Vec3, width_vec: Vec3, colour: [f32; 4]) -> [GpuVertex; 6] {
    let corner = |point: Vec3, sign: f32, u: f32, v: f32| GpuVertex {
        position: (point + width_vec * sign).to_array(),
        normal: [0.0, 0.0, 1.0],
        colour,
        texcoord: [u, v],
        lit: 0.0,
        ..bytemuck::Zeroable::zeroed()
    };
    // UVs match the recovered vertex seeds exactly: near/-width = (0,1),
    // far/-width = (1,1), near/+width = (0,0), far/+width = (1,0).
    let near_minus = corner(near, -1.0, 0.0, 1.0);
    let far_minus = corner(far, -1.0, 1.0, 1.0);
    let near_plus = corner(near, 1.0, 0.0, 0.0);
    let far_plus = corner(far, 1.0, 1.0, 0.0);
    [
        near_minus, far_minus, near_plus, far_minus, far_plus, near_plus,
    ]
}

/// The bolt's two crossed ribbons for one live round, drawn every tick it is
/// alive. `Cannon_BuildBoltList` draws two 4-vertex strips rather than one;
/// see this module's own doc comment for why.
///
/// `prev`/`curr` are the round's position last tick and this tick; `right`/
/// `up` are the camera's own basis vectors, read the same way
/// [`crate::exhaust::sprite`]'s are. Twelve vertices: two six-vertex quads
/// back to back.
#[must_use]
pub fn bolt_vertices(prev: Vec3, curr: Vec3, right: Vec3, up: Vec3) -> [GpuVertex; 12] {
    let near = curr + (prev - curr) * BOLT_NEAR_FRACTION;
    let far = prev;
    let diagonal_a = (right - up) * BOLT_HALF_WIDTH;
    let diagonal_b = (right + up) * BOLT_HALF_WIDTH;
    let mut out = [GpuVertex::zeroed(); 12];
    out[..6].copy_from_slice(&ribbon_quad(near, far, diagonal_a, BOLT_COLOUR));
    out[6..].copy_from_slice(&ribbon_quad(near, far, diagonal_b, BOLT_COLOUR));
    out
}

/// The muzzle flash's one quad, drawn only while the round's age is under
/// [`FLASH_WINDOW_SECONDS`].
///
/// `center` is the round's own position; `half_size` and `alpha` are
/// [`super::random::flash_roll`]'s output for this round this tick;
/// `rotation` is in radians. **Rotating the basis rather than the built
/// corners** is the equivalent of `FUN_08864ea0`'s per-corner rotate-about-
/// centre, since a square built from a rotated `right`/`up` pair is exactly
/// a square whose corners were each rotated about its own centre.
#[must_use]
pub fn flash_vertices(
    center: Vec3,
    right: Vec3,
    up: Vec3,
    half_size: f32,
    rotation: f32,
    alpha: f32,
) -> [GpuVertex; 6] {
    let (sin, cos) = rotation.sin_cos();
    let rotated_right = right * cos + up * sin;
    let rotated_up = up * cos - right * sin;
    let colour = [1.0, 1.0, 1.0, alpha];
    let corner = |sx: f32, sy: f32, u: f32, v: f32| GpuVertex {
        position: (center + rotated_right * (sx * half_size) + rotated_up * (sy * half_size))
            .to_array(),
        normal: [0.0, 0.0, 1.0],
        colour,
        texcoord: [u, v],
        lit: 0.0,
        ..bytemuck::Zeroable::zeroed()
    };
    // Same strip-to-list shape as `ribbon_quad`/`exhaust::sprite::quad`:
    // bl=(0,1), br=(1,1), tl=(0,0), tr=(1,0).
    let bl = corner(-1.0, -1.0, 0.0, 1.0);
    let br = corner(1.0, -1.0, 1.0, 1.0);
    let tl = corner(-1.0, 1.0, 0.0, 0.0);
    let tr = corner(1.0, 1.0, 1.0, 0.0);
    [bl, br, tl, br, tr, tl]
}
