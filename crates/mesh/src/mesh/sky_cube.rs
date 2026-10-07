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
use oag_texture::gtf;
use oag_vex::vex;

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

/// The `Lighting.Sky colour` byte that leaves the sky texture unchanged:
/// the tint is `byte / SKY_COLOUR_NEUTRAL`, so 128 is neutral and 255 doubles.
///
/// **Fitted, not read.** The sky draw hands the colour to a vertex-colour
/// multiply and the factor of two between the byte and the product is not in
/// any program read (`PrimList*` pass `COL0` through, and their fragment
/// programs are `tex * COL0` unscaled). What measures it: on Sol 2 (255) the
/// matched frame's unclipped sky pixels sit at 1.98 to 2.03 times ours with
/// zero offset (`r` 0.89), while Talon's Junction (128) and Amphiseum (140)
/// already matched at 1.0. Two circuit values, so a linear law is the simplest
/// thing that fits and nothing more is claimed.
pub const SKY_COLOUR_NEUTRAL: f32 = 128.0;

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
/// `tint` is the vertex colour of all 24 corners, multiplied into the texel
/// (HD's own sky draw passes the circuit's `Lighting.Sky colour` as the cube's
/// vertex colour, `FUN_005ecc28`). [`SKY_COLOUR_NEUTRAL`] says what that colour
/// is in these units; `[1.0; 3]` is no tint, which is what Zone's file swap and
/// every caller without an authored colour pass.
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
/// **2026-08-31: an attempt to settle the sign against a reference capture,
/// and why it did not.** The experiment is sound and worth repeating with a
/// better frame rather than re-derived from scratch:
///
/// - `data/reference/hd-capture/racebox/00.png` is Vineta K from the authored
///   `Start Position` node, and this engine's *full-grid* path still starts
///   there, so a `--mode single_race` render is a genuinely matched pair -
///   same arch, same `harimau` sign, same crowd line, no camera fitting
///   needed. That is the cheap way to compare against these captures, and it
///   is worth knowing before anyone builds pose-matching tooling for it.
/// - The test **is** sensitive: negating `rotation_degrees` and re-rendering
///   visibly moves the sky, so a wrong sign would not hide.
/// - It still did not settle it. Vineta K's sky is largely a gradient with a
///   warm glow rather than a localisable feature, and **the original's
///   exposure blows its sky far brighter than ours**, which shifts apparent
///   hue everywhere the comparison depends on. The reading leaned toward the
///   negated sign and was nowhere near strong enough to act on, so nothing
///   was changed.
///
/// What would settle it: a reference frame on a circuit whose sky carries a
/// *localisable* feature - a sun disc or a distinct cloud mass - with open sky
/// in view, rather than one seen through an arch. The tone difference above is
/// the separate exposure/bloom gap, not this.
///
/// **2026-08-31: every in-race frame in `data/reference/hd-capture/` has now
/// been looked at, and none of them is that frame.** Recorded so the next
/// contributor does not re-open the same seven pictures: `anulpha/00..02` are
/// inside Anulpha Pass's enclosed tube with no sky at all; `talons/00..01`,
/// `talons-heap/00..01` and `talons-rm/00` show Talons Junction's sky, but as
/// a blown-out white haze behind trackside structure - the same "no feature to
/// localise, and brighter than ours" failure Vineta K's produced, on a second
/// circuit. So this is **not** a matter of picking a better frame from what is
/// on disk; it needs a new capture, and the ask is specific: a **racing
/// circuit** (a Zone frame cannot serve - HD's Zone sky is a 64x64 cubemap,
/// see `docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md`), open sky filling a
/// good part of the frame, and one feature whose position can be measured.
///
/// **2026-10-07: the "original's exposure blows its sky far brighter than ours"
/// seen above was in part the circuit's `Sky colour` tint** ([`SKY_COLOUR_NEUTRAL`]),
/// applied now; Vineta K's 134 is only 1.05x, so its own gap is not that.
///
/// **Do not "fix" the sign to make a screenshot match** without that: under an
/// exposure mismatch a hue can be moved either way by eye.
///
/// # Errors
///
/// A `.gtf` that does not parse, is not a cubemap, or whose faces do not
/// decode. The caller reports and draws no sky, per the usual rule: an honest
/// absence rather than an invention.
pub fn build(label: &str, blob: &[u8], rotation_degrees: f32, tint: [f32; 3]) -> Result<Model> {
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
        textures.push(Some(std::sync::Arc::new(ModelTexture::rgba8(
            format!("{label}[{face}]"),
            width,
            height,
            rgba.into_iter().flatten().collect(),
            None,
        ))));

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
                colour: [tint[0], tint[1], tint[2], 1.0],
                texcoord: [(sc + 1.0) / 2.0, (tc + 1.0) / 2.0],
                lit: 0.0,
                anim: 0,
                lightmap_texcoord: [0.0, 0.0],
                xform: 0,
                sun_mask: 1.0,
                slots: crate::mesh::slots::DEFAULT,
                specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
                glow: 0.0,
                texcoord2: [0.0, 0.0],
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
            // Synthetic geometry: no batch to read one from.
            alpha_test_ref: None,
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
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),

        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,

        flame: None,
        absorb_shell: false,
        alpha_test_ref: None,
        centre: [0.0; 3],
        radius: HALF_EXTENT * 3.0f32.sqrt(),
        mesh_count: 6,
        airbrakes: [None, None],
        anim_tracks: Vec::new(),
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
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
