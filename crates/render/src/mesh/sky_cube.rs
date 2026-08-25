//! Building a drawable [`Model`] from a Wipeout HD `sky.gtf` cubemap.
//!
//! Wipeout HD authors no `Skycube` node - class `0x3c6` is in every circuit's
//! class table and zero nodes carry it. Its sky is `sky.gtf` beside the track:
//! a six-face `.gtf` cubemap (see `docs/formats/gtf.md`), which the executable
//! itself ties to the sky - `"sky.gtf"`, `"skycube"` and
//! `"Lighting.Debug.Draw sky"` all appear in `EBOOT.elf`, per
//! `docs/formats/envsettings.md`. So the *picture* is the disc's; what this
//! module supplies is the standard geometry a cubemap defines for itself: one
//! quad per face on a camera-centred cube, each face sampled exactly where the
//! RSX's own cubemap addressing would sample it.
//!
//! The model comes out shaped for the existing sky draw - `race::Scene` already
//! draws a sky model camera-centred with [`crate::mesh_render::Depth::Sky`],
//! which is how Pulse's authored `Skycube` mesh has always been drawn. Nothing
//! downstream tells the two apart.

use anyhow::{Context, Result, bail};
use oag_formats::{gtf, vex};

use super::{Bounds, DrawCall, GpuVertex, Model, ModelTexture};

/// Half-extent of the cube, in world units.
///
/// **Appearance-invariant, and that is measurable from the geometry**: the cube
/// is camera-centred, so scaling it moves every face along its own view ray and
/// the projected picture is identical at any size. The one live constraint is
/// `mesh_render::Depth::Sky`'s: the faces must clear the near plane (1.0) or
/// they clip away entirely. 32 sits inside the 18.4-61.7 radius range Pulse's
/// authored skies ship at, for no reason beyond being comfortably clear of it.
const HALF_EXTENT: f32 = 32.0;

/// One face of the cube: its outward axis and the two texture axes.
///
/// This is the OpenGL cubemap face table - which face a direction selects and
/// where on it `(s, t)` land - inverted to *place* geometry instead of look it
/// up. The RSX inherits GL's addressing, and `docs/formats/gtf.md` pins the
/// face order (`+X, -X, +Y, -Y, +Z, -Z`) and content (face 2 is the zenith,
/// face 3 the nadir) against Talon's Junction's own sky. For corner
/// `(sc, tc) ∈ {-1, +1}²` the world position is
/// `major * H + s_axis * sc * H + t_axis * tc * H` and the texture coordinate
/// is `((sc + 1) / 2, (tc + 1) / 2)` - `t` grows towards the image's last row,
/// which is wgpu's `v` sense too, so no flip appears anywhere.
const FACES: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
    ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, -1.0, 0.0]),
    ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]),
    ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
    ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),
    ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
    ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, -1.0, 0.0]),
];

/// The four `(sc, tc)` corners of a face, in fan order.
const CORNERS: [(f32, f32); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];

/// Builds a camera-centred cube from a `.gtf` cubemap.
///
/// `rotation_degrees` is the circuit's `Lighting.Sky rotation`, applied about
/// the world's vertical. **Degrees is read off the corpus rather than assumed**:
/// the shipped values include 180 and -40, which no radian reading survives.
/// That a yaw is the axis is forced by the data too - it is the only rotation
/// that keeps a cubemap's horizon level and its zenith overhead - but the
/// *sign* has nothing on the disc to check it against and is this project's
/// choice (mathematically positive about `+Y`); the caller's report should say
/// the rotation was applied on those terms.
///
/// # Errors
///
/// A `.gtf` that does not parse, is not a cubemap, or whose faces do not
/// decode. The caller reports and draws no sky, per the usual rule: an honest
/// absence rather than an invention.
pub fn build(label: &str, blob: &[u8], rotation_degrees: f32) -> Result<Model> {
    let parsed = gtf::Gtf::parse(blob).with_context(|| format!("{label}: parsing"))?;
    let Some(texture) = parsed.only() else {
        bail!(
            "{label}: {} textures where a sky has one",
            parsed.textures.len()
        );
    };
    if !texture.cubemap {
        bail!("{label}: not a cubemap");
    }
    let (width, height) = texture.level_size(0);

    let (sin, cos) = rotation_degrees.to_radians().sin_cos();
    let turn = |p: [f32; 3]| [p[0] * cos + p[2] * sin, p[1], p[2] * cos - p[0] * sin];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    let mut draws = Vec::with_capacity(6);
    let mut textures = Vec::with_capacity(6);
    for (face, (major, s_axis, t_axis)) in FACES.iter().enumerate() {
        let rgba = texture
            .face_to_rgba(blob, face)
            .with_context(|| format!("{label}: decoding face {face}"))?;
        textures.push(Some(ModelTexture {
            label: format!("{label}[{face}]"),
            width,
            height,
            rgba: rgba.into_iter().flatten().collect(),
        }));

        let base = vertices.len() as u32;
        for &(sc, tc) in &CORNERS {
            let position = turn([
                (major[0] + s_axis[0] * sc + t_axis[0] * tc) * HALF_EXTENT,
                (major[1] + s_axis[1] * sc + t_axis[1] * tc) * HALF_EXTENT,
                (major[2] + s_axis[2] * sc + t_axis[2] * tc) * HALF_EXTENT,
            ]);
            vertices.push(GpuVertex {
                position,
                // Inward, and unread: `lit` 0.0 keeps the light rig off the
                // sky, the same way Pulse's own sky meshes are authored
                // `_nolight`. A sky is a picture of light, not a surface.
                normal: [-major[0], -major[1], -major[2]],
                colour: [1.0, 1.0, 1.0, 1.0],
                texcoord: [(sc + 1.0) / 2.0, (tc + 1.0) / 2.0],
                lit: 0.0,
                anim: 0,
                lightmap_texcoord: [0.0, 0.0],
                xform: 0,
                sun_mask: 1.0,
                slots: crate::mesh::slots::DEFAULT,
            });
        }
        let start = indices.len() as u32;
        for i in [0u32, 1, 2, 0, 2, 3] {
            indices.push(base + i);
        }
        draws.push(DrawCall {
            range: start..start + 6,
            texture: Some(face),
            bounds: Bounds {
                centre: [0.0, 0.0, 0.0],
                radius: HALF_EXTENT * 3.0f32.sqrt(),
            },
            moving: false,
            // Two-sided: the camera is always inside this cube, and skipping
            // the cull spends nothing on twelve triangles.
            culled: false,
            blend: None,
            blend_state: None,
            layer: vex::LAYER_DEFAULT,
            node: None,
            chunk: None,
        });
    }

    let lightmaps = vec![None; textures.len()];
    Ok(Model {
        label: label.to_string(),
        vertices,
        indices,
        draws,
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures,
        lightmaps,
        material_slots: Vec::new(),
        material_variants: Vec::new(),

        vertex_colour_is_light: false,

        flame: None,
        centre: [0.0; 3],
        radius: HALF_EXTENT * 3.0f32.sqrt(),
        mesh_count: 6,
        airbrakes: [None, None],
        anim_tracks: Vec::new(),
        node_vertex_ranges: Vec::new(),
        anim_nodes: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every face's outward axis must be the cross product of its own `s` and
    /// `t` axes read the GL way round - which is what "the table is the GL
    /// table" means arithmetically, so a transposed row fails here rather than
    /// as a mirrored skyline in a screenshot.
    #[test]
    fn face_axes_are_the_gl_cubemap_basis() {
        for (face, (major, s, t)) in FACES.iter().enumerate() {
            let cross = [
                s[1] * t[2] - s[2] * t[1],
                s[2] * t[0] - s[0] * t[2],
                s[0] * t[1] - s[1] * t[0],
            ];
            // GL's cube faces are left-handed: on every one of the six,
            // s cross t points along the face's own *inward* axis, so the
            // outward axis is t cross s. Uniform across the table, which is
            // the property a transposed or sign-flipped row breaks.
            let inward = [-major[0], -major[1], -major[2]];
            assert_eq!(cross, inward, "face {face}");
        }
    }

    /// The zenith face (+Y, face 2 - the deep-blue one on Talon's Junction per
    /// `docs/formats/gtf.md`) must sit overhead, and the nadir underfoot.
    #[test]
    fn zenith_is_overhead() {
        assert_eq!(FACES[2].0, [0.0, 1.0, 0.0]);
        assert_eq!(FACES[3].0, [0.0, -1.0, 0.0]);
    }

    /// On every side face, the image's first row (`t = 0`) must be the top
    /// edge of the quad - a sky whose horizon renders upside down passes every
    /// other check in this module.
    #[test]
    fn side_face_image_tops_point_up() {
        for &face in &[0usize, 1, 4, 5] {
            let (_, _, t_axis) = FACES[face];
            assert_eq!(t_axis, [0.0, -1.0, 0.0], "face {face}");
        }
    }
}
