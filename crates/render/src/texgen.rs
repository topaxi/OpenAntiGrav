//! Environment ("shade") mapped texture-coordinate generation, the PSP GE's
//! `TEXMAPMODE` uvgen mode 2.
//!
//! # Scope, corrected 2026-08-10
//!
//! **The boost plume no longer uses this module, and the plume-specific
//! rationale below is retained as history. A Pulse hull's extra pass does:
//! see [`crate::shine`], which calls [`environment_map`] with
//! [`ENV_BASIS_0`]/[`ENV_BASIS_1`] - the same fixed pair, because the hull
//! model takes the same load-time branch the plume was first read on
//! (`model+0x1a8 == 1`, read live on a hull 2026-09-30).** The plume's compiled display
//! list was read to be replayed under `TEXMAPMODE` 0 - authored coordinates,
//! sampled through the keyframed `TEXOFFSET` u-scroll authored in
//! `shipboost.vex` itself (see `mesh-draw.md`, "The plume is replayed under
//! `TEXMAPMODE` 0", and `texture-animation.md`, "The values gap is closed").
//! What this module implements stays real for the batches genuinely inside
//! the `Mesh_BeginTransparentPass` bracket - 73 `PRIM`s per frame draw under
//! mode 2 on Talon's Junction - but nothing in the reimplementation draws
//! those yet, so the module currently has no caller.
//!
//! # Why this exists
//!
//! `Mesh_BeginTransparentPass` (`0x0890d904`) sets `Gu_TexMapMode(2, 0, 1)` -
//! uvgen 2 with `TEXSHADELS` LS0 = light 0, LS1 = light 1 - and nothing inside
//! the batch loop re-emits `0xc0` to undo it, so a transparent batch inside
//! that bracket never reads its authored texture coordinates. The GE
//! generates them per vertex from the vertex normal and two light directions
//! instead. See [`mesh-draw.md`].
//!
//! Until this module existed the reimplementation fed those authored
//! coordinates to the sampler, and for `Data\Ships\<Team>\shipboost.vex` -
//! the speed-pad plume - that is not a small error. Its authored UVs are
//! degenerate: `u` spans `[0, 0.0078125]` across all 121 vertices, which is
//! **column 0** of a 64x16 texture, and column 0 of `pulse_boost2_ADD.tga` is
//! a uniform `(250, 248, 250)` down all sixteen rows. So the texture
//! multiplied the fins by a constant white, the whole plume collapsed to flat
//! vertex colour, and it drew as two hard-edged solid orange wedges. The
//! texture the artists actually authored is a streak lookup, and both of its
//! axes carry something:
//!
//! - **`u` is a brightness ramp.** Column 0 is a uniform `(250, 248, 250)`
//!   down every row; the ramp falls through violet (`(184, 122, 207)` at
//!   `u ~ 0.2`, `(143, 72, 218)` at `u ~ 0.45`) to `(10, 4, 18)` at
//!   `u ~ 0.95`.
//! - **`v` is the streak pattern.** At a fixed column the rows alternate hard
//!   between a bright magenta `(241, 110, 253)` and a dark navy
//!   `(20, 5, 122)`, in an irregular, non-repeating sequence. Read together
//!   with the `u` ramp it is diagonal banding.
//!
//! The `v` axis is what breaks a continuous fin into discrete bright bands -
//! the streaked, wispy look. Sampling column 0 throws away *both* axes.
//!
//! That also accounts for a measurement this project already had and could
//! not explain: the original's boost region reads magenta (mean
//! `(223, 164, 224)`) against ours at `(159, 118, 175)`
//! ([`oag_fx::exhaust`], and the table in `race::Scene::new`). With column 0
//! as the only sample there was **no magenta anywhere in our pipeline** for
//! the additive blend to reach.
//!
//! # Why on the CPU
//!
//! uvgen is a **transform-stage** operation on real hardware - the GE
//! generates the coordinate per vertex, before rasterisation - so doing it per
//! vertex here is the faithful placement, not a shortcut. It also costs
//! nothing: the plume is 121 vertices, and `oag_fx::exhaust` already
//! rewrites a whole vertex buffer every frame.
//!
//! The alternative, a fourth bind group carrying the mode and the two light
//! vectors, would have to be threaded through `mesh_render::build`,
//! `capture_from`, `oag-view`'s orbit renderer, `race::Drawable` and the MSAA
//! test - the blast radius `mesh-draw.md` twice refused for this pass - to
//! serve one model.
//!
//! [`mesh-draw.md`]: ../../../docs/ghidra/functions/psp-pulse-usa/mesh-draw.md

use oag_core::math::{Mat4, Vec3};

use oag_mesh::mesh::GpuVertex;

/// The light-0 direction of the fixed environment-map basis - the `u` axis.
/// Named for the plume while it was the only known user; it is
/// `g_envmap_light_basis` column 0, which every model whose `+0x1a8` is set
/// reads, the hull included.
///
/// Recovered, not fitted: column 0 of `g_envmap_light_basis` (`0x08abf510`),
/// which `Vex_LoadModel` builds once at `0x08913078`-`0x08913248` as
/// `Rx(-1.0) * Ry(0.3)` from two read-only literals,
/// `g_envmap_light_basis_angle_x` (`0x08abf500`, `-1.0`) and
/// `g_envmap_light_basis_angle_y` (`0x08abf504`, `0.3`), both in radians.
/// Confidence **85** on the numeric vector - the `vmmul` operand order is
/// derived from PPSSPP's matrix-register semantics rather than measured - and
/// **95** on the two angles and the two rotation builds.
///
/// **The plume takes the load-time, fixed-basis path, not the runtime one.**
/// `Vex_LoadModel` and `Vex_UpdateLightLists_q` both write these lists and
/// `model+0x1a8` picks between them; `ExhaustFlare_Init` (`0x08905444`) sets
/// the boost model's parent link to the craft-class ancestor immediately
/// before loading it, so the ancestor walk matches on its first step and the
/// model takes the fixed basis. The runtime branch reads rows 0 and 1 of the
/// live view matrix and **is** a matcap; this one is not, which is why
/// [`environment_map`] leaves these vectors in world space instead of
/// following the camera.
///
/// If the plume ever reads mirrored against a captured reference, the first
/// thing to try is the transposed `vmmul` reading, which is distinguishable
/// by eye: it gives `dir0.y == 0` exactly where this one gives
/// `dir1.x == 0` exactly.
///
/// See `docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`, "The two light
/// vectors are a fixed world-space pair, built once at load".
pub const ENV_BASIS_0: Vec3 = Vec3::new(0.955_336_5, -0.248_672_2, -0.159_670_4);

/// The light-1 direction - the `v` axis. Column 1 of the same basis; see [`ENV_BASIS_0`].
pub const ENV_BASIS_1: Vec3 = Vec3::new(0.0, 0.540_302_3, -0.841_471);

/// Generates uvgen-2 texture coordinates for `base` into `out`.
///
/// The formula is the GE's, as implemented by PPSSPP's software transform
/// (`GPU/Software/TransformUnit.cpp`, `GE_TEXMAP_ENVIRONMENT_MAP`):
///
/// ```text
/// u = (1 + dot(world_normal, normalize(light[LS0]))) / 2
/// v = (1 + dot(world_normal, normalize(light[LS1]))) / 2
/// ```
///
/// with the normal taken to **world** space - here, through `model`'s upper
/// 3x3 - and renormalised, and the light *positions* normalised to
/// directions. `light0` and `light1` are already in world space; pass them as
/// the pass programs them.
///
/// GE world really is model-to-world for a mesh batch, with no model-view
/// folding: the mesh path sets GE matrix 2 while the view matrix is GE matrix
/// 1, set separately. That fork is what decides whether this is a matcap, and
/// for the boost plume it lands on "not a matcap" - see [`ENV_BASIS_0`].
///
/// `out` is resized to `base`'s length and every field but `texcoord` is
/// copied through unchanged, so the caller can upload it whole.
///
/// # Why it must run per frame
///
/// The dot product is between a world-space light direction and a world-space
/// normal, so the coordinates move as the **model** rotates - not as the
/// camera does. They cannot be baked at load time for anything that turns,
/// and a ship turns constantly.
///
/// # The camera must not reach this
///
/// The sibling GE path (`Vex_UpdateLightLists_q`) takes its two vectors from
/// rows 0 and 1 of the live view matrix, which makes it a matcap. The boost
/// plume does not take that path, and a reimplementation that let the view
/// matrix in here would silently become one - a difference no single frame
/// shows. This function takes no view matrix, which is the structural half of
/// that guarantee; the empirical half is two cameras at one ship pose giving
/// plume means `(197.2, 149.7, 212.2)` and `(198.1, 149.1, 209.6)`, within
/// three counts per channel. Re-run that check after touching this.
pub fn environment_map(
    base: &[GpuVertex],
    out: &mut Vec<GpuVertex>,
    model: Mat4,
    light0: Vec3,
    light1: Vec3,
) {
    let normals = oag_core::math::Mat3::from_mat4(model);
    environment_map_by(base, out, light0, light1, |v| {
        normals * Vec3::from_array(v.normal)
    });
}

/// [`environment_map`], with the world normal - before it is renormalised -
/// taken from `normal` rather than from one matrix for the whole model. For a
/// caller whose vertices each ride their own node, which would otherwise copy
/// the whole vertex list once to place the normals and again to map them.
pub fn environment_map_by(
    base: &[GpuVertex],
    out: &mut Vec<GpuVertex>,
    light0: Vec3,
    light1: Vec3,
    normal: impl Fn(&GpuVertex) -> Vec3,
) {
    let l0 = light0.normalize_or_zero();
    let l1 = light1.normalize_or_zero();

    out.clear();
    out.reserve(base.len());
    out.extend(base.iter().map(|v| {
        let mut out = *v;
        out.texcoord = coordinate(normal(v), l0, l1);
        out
    }));
}

/// [`environment_map_by`], producing the coordinates alone - for a drawable
/// that streams them in their own buffer
/// (`mesh_render::Texcoords::Streamed`) rather than re-uploading every whole
/// vertex to change eight bytes of each. The same arithmetic to the bit.
pub fn environment_texcoords_by(
    base: &[GpuVertex],
    out: &mut Vec<[f32; 2]>,
    light0: Vec3,
    light1: Vec3,
    normal: impl Fn(&GpuVertex) -> Vec3,
) {
    let l0 = light0.normalize_or_zero();
    let l1 = light1.normalize_or_zero();
    out.clear();
    out.reserve(base.len());
    out.extend(base.iter().map(|v| coordinate(normal(v), l0, l1)));
}

/// One vertex's coordinate from its world normal, not yet renormalised.
///
/// `normalize_or_zero` rather than `normalize`: a degenerate normal would
/// produce NaN texture coordinates, and a NaN UV takes the whole batch off
/// screen rather than drawing one bad vertex.
#[inline]
fn coordinate(normal: Vec3, l0: Vec3, l1: Vec3) -> [f32; 2] {
    let n = normal.normalize_or_zero();
    [(1.0 + n.dot(l0)) * 0.5, (1.0 + n.dot(l1)) * 0.5]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vertex(normal: [f32; 3]) -> GpuVertex {
        GpuVertex {
            position: [0.0; 3],
            normal,
            colour: [1.0; 4],
            texcoord: [0.5, 0.5],
            lightmap_texcoord: [0.0, 0.0],
            lit: 0.0,
            anim: 0,
            xform: 0,
            sun_mask: 1.0,
            slots: oag_mesh::mesh::slots::DEFAULT,
            specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
            glow: 0.0,
            texcoord2: [0.0, 0.0],
        }
    }

    /// The two ends of the range and its midpoint, which is the whole formula.
    #[test]
    fn a_normal_along_the_light_maps_to_one_and_against_it_to_zero() {
        let base = [
            vertex([0.0, 0.0, 1.0]),
            vertex([0.0, 0.0, -1.0]),
            vertex([1.0, 0.0, 0.0]),
        ];
        let mut out = Vec::new();
        environment_map(
            &base,
            &mut out,
            Mat4::IDENTITY,
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        assert_eq!(out[0].texcoord[0], 1.0);
        assert_eq!(out[1].texcoord[0], 0.0);
        // Perpendicular to both lights, so both axes sit at the midpoint.
        assert_eq!(out[2].texcoord, [0.5, 0.5]);
    }

    /// The light vectors need no normalisation from the caller.
    #[test]
    fn an_unnormalised_light_gives_the_same_coordinates_as_a_unit_one() {
        let base = [vertex([0.3, -0.6, 0.74])];
        let (mut unit, mut long) = (Vec::new(), Vec::new());
        environment_map(
            &base,
            &mut unit,
            Mat4::IDENTITY,
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        environment_map(
            &base,
            &mut long,
            Mat4::IDENTITY,
            Vec3::new(0.0, 0.0, 90.0),
            Vec3::new(0.0, 7.5, 0.0),
        );
        assert_eq!(unit[0].texcoord, long[0].texcoord);
    }

    /// The coordinates track the **model**'s rotation. Turning the model
    /// halfway round moves a vertex from one end of the `u` ramp to the other,
    /// which is why this cannot be baked at load time.
    #[test]
    fn rotating_the_model_moves_the_coordinates() {
        let base = [vertex([0.0, 0.0, 1.0])];
        let mut turned = Vec::new();
        environment_map(
            &base,
            &mut turned,
            Mat4::from_rotation_y(std::f32::consts::PI),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
        );
        assert!(
            (turned[0].texcoord[0] - 0.0).abs() < 1.0e-6,
            "u {} - a half turn should put this normal at the far end of the ramp",
            turned[0].texcoord[0]
        );
    }

    /// Everything but the coordinate is carried through, so the caller can
    /// upload the result as the whole vertex buffer.
    #[test]
    fn every_other_vertex_field_survives() {
        let mut v = vertex([0.0, 1.0, 0.0]);
        v.position = [1.0, 2.0, 3.0];
        v.colour = [0.25, 0.5, 0.75, 1.0];
        v.lit = 1.0;
        v.anim = 3;

        let mut out = Vec::new();
        environment_map(&[v], &mut out, Mat4::IDENTITY, Vec3::Z, Vec3::Y);

        assert_eq!(out[0].position, v.position);
        assert_eq!(out[0].normal, v.normal);
        assert_eq!(out[0].colour, v.colour);
        assert_eq!(out[0].lit, v.lit);
        assert_eq!(out[0].anim, v.anim);
        assert_ne!(out[0].texcoord, v.texcoord);
    }

    /// A zero normal must not produce a NaN coordinate: one NaN vertex takes
    /// its whole batch off screen, which is a far worse failure than a wrong
    /// texel.
    #[test]
    fn a_degenerate_normal_stays_finite() {
        let mut out = Vec::new();
        environment_map(
            &[vertex([0.0; 3])],
            &mut out,
            Mat4::IDENTITY,
            Vec3::Z,
            Vec3::Y,
        );
        assert_eq!(out[0].texcoord, [0.5, 0.5]);
    }

    /// `out` is reused across frames by the caller, so it must not accumulate.
    #[test]
    fn reusing_the_output_buffer_does_not_accumulate() {
        let base = [vertex([0.0, 0.0, 1.0]), vertex([0.0, 0.0, -1.0])];
        let mut out = Vec::new();
        for _ in 0..3 {
            environment_map(&base, &mut out, Mat4::IDENTITY, Vec3::Z, Vec3::Y);
        }
        assert_eq!(out.len(), base.len());
    }
}
