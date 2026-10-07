//! Building a drawable [`Model`] from a PS3 `.vex` and the `.rcsmodel` beside
//! it.
//!
//! The PS3 counterpart of [`super::build_class`], and deliberately a separate
//! function rather than a branch inside it. The two share the *scene* - node
//! tree, class table, world transforms, all read by `oag_vex::vex` exactly
//! as before - and share nothing at all below that: a PSP mesh's geometry is a
//! batch list in its own payload, and a PS3 mesh's is a chunk in another file,
//! addressed by hash and quantised through a bias and scale. Threading that
//! through the 300-line PSP builder would have put a `if ps3` at every step.
//!
//! # What it draws, and what it does not
//!
//! **Textured, lit, and blended where the material says so**, all of it out of
//! the disc: positions, triangle indices and vertex normals from the
//! `.rcsmodel`, the texture coordinate from the last four bytes of each vertex,
//! and the `.gtf` each material names through [`oag_texture::gtf`]. Nothing is
//! substituted - a made-up normal or an invented alpha would light a model
//! wrongly rather than visibly failing, which is the failure mode `CLAUDE.md`
//! names.
//!
//! What is **not** drawn, and is counted rather than hidden: a mesh whose chunk
//! cannot be found or whose vertex stride cannot be recovered, and the second
//! texture a material may name at `+0x78`. That second slot is a *mask* on
//! Talon's Junction's cloud plate (`cloud mask.gtf` beside `clouds_new.gtf`), a
//! lightmap on its road, an emissive map on its tunnels and a normal map on a
//! craft - one field with at least four uses, selected by a shader nothing here
//! reads. So a surface whose coverage lives in that second texture still paints
//! solid; see [`surface`].

use anyhow::{Context, Result, bail};
use oag_core::math::{Mat4, Vec3};
use oag_rcs::rcsmodel;
use oag_vex::vex;

use super::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, TextureSlots, anim_node, slots};

/// How far a dequantised point may miss the authored box face by, in world
/// units, before a stride is rejected.
///
/// Two quantisation steps at the `1/128` scale every measured file uses. Read
/// from each mesh's own scale rather than hardcoded, so a file that quantises
/// differently is judged on its own terms.
const TOLERANCE_STEPS: f32 = 2.0;

mod report;

pub use report::Report;

/// The `.rcsmodel` entry name beside a `.vex` one.
///
/// A path rewrite rather than a lookup, because that is how the disc pairs
/// them: `/data/ships/assegai/ship.vex` sits beside
/// `/data/ships/assegai/ship.rcsmodel`, and every pair on the disc is spelled
/// that way. Returns `None` for a name that does not end in `.vex`, since there
/// is then no pairing rule to apply.
#[must_use]
pub fn sibling_name(vex_name: &str) -> Option<String> {
    let stem = vex_name.strip_suffix(".vex").or_else(|| {
        // The game's own spelling is case-insensitive in both containers, and a
        // PSP-shaped name reaches here through `oag_hd`'s tables.
        vex_name
            .len()
            .checked_sub(4)
            .filter(|&at| vex_name[at..].eq_ignore_ascii_case(".vex"))
            .map(|at| &vex_name[..at])
    })?;
    Some(format!("{stem}.rcsmodel"))
}

/// The Omega Collection's spelling of [`sibling_name`]: `<stem>.final.rcsmodel`.
///
/// **Its circuits ship their baked outputs under a `.final` infix** -
/// `Data\environments\tech_de_ra\track.vex` sits beside
/// `track.final.rcsmodel`, `track.final.rcsskeleton`, `track.final.rcsanimclip`
/// and `track.final.pvs`, and `track.rcsmodel` does not exist. Measured over
/// the whole PS4 package pair: all 22 of its own and 2048-derived circuits
/// (and every `track_reversed` one) use the `.final` spelling, and the four
/// `zone_N` circuits use the plain one - never both in one directory. So a
/// caller that tries [`sibling_name`] first and this second can never read the
/// wrong file, and every other title, none of which ships a `.final` name, is
/// untouched.
#[must_use]
pub fn sibling_name_cooked(vex_name: &str) -> Option<String> {
    let plain = sibling_name(vex_name)?;
    let stem = plain.strip_suffix(".rcsmodel")?;
    Some(format!("{stem}.final.rcsmodel"))
}

/// The `.rcsmodel` beside `name`, when this `.vex` needs one and the archive
/// carries it.
///
/// `None` covers three different things and deliberately does not tell them
/// apart, because every caller does the same thing with all three - draws
/// nothing and says so:
///
/// - a PSP or PS2 `.vex`, which has its geometry inside it and needs no sibling;
/// - a name with no `.vex` extension to rewrite;
/// - a sibling the archive does not have.
///
/// A caller that wants the distinction has [`super::geometry_is_external`] for the
/// first of them.
#[must_use]
pub fn sibling_geometry(spec: &str, name: &str, data: &[u8]) -> Option<Vec<u8>> {
    if !super::geometry_is_external(data) {
        return None;
    }
    super::read_blob(spec, &sibling_name(name)?).ok()
}

/// The whole of a PS3 model, read out of the archive `spec` names, or `None`
/// when `name` is not one.
///
/// **`None` is the answer for every PSP and PS2 source**, so a caller keeps its
/// existing path for those unchanged and adds one match arm rather than a
/// branch at every step.
///
/// The one call a caller needs: it decides whether this `.vex` has its geometry
/// elsewhere, fetches the sibling and builds the pair. Everything a PSP or PS2
/// source does is unchanged, because [`sibling_geometry`] answers `None` for
/// them and the caller keeps its existing path.
///
/// # Errors
///
/// As [`build_scene`].
pub fn scene_from(spec: &str, name: &str, data: &[u8]) -> Result<Option<(Model, Report)>> {
    let Some(geometry) = sibling_geometry(spec, name, data) else {
        return Ok(None);
    };
    let (model, report) = build_scene(name, data, &geometry, &mut |path| {
        super::read_blob(spec, path).ok()
    })?;
    Ok(Some((model, report)))
}

/// How a build fetches a material's `.gtf` out of whatever archive the caller
/// holds.
///
/// A closure rather than an archive spec because the two callers hold different
/// things: the viewer has a `<image>:<path>` string and the game has an
/// `oag_assets::Archives` spanning seven of them. `None` for a texture the
/// archive does not have, which [`Report::untextured`] counts.
pub type Textures<'a> = &'a mut dyn FnMut(&str) -> Option<Vec<u8>>;

/// The [`super::slots::RIM_GLOW`]/[`super::slots::RIM_EDGE`] bit a resolved
/// fragment program earns, or `0` - `rim_glow`'s own classifier, public so a
/// disc-wide census (`crates/render/examples/hd_unlit_census.rs`) asks the
/// same question the loader does rather than a copy of it.
#[must_use]
pub fn rim_glow_bit(
    declared: &oag_rcs::rcsmaterial::Declared,
    program: &oag_rcs::rcsmaterial::fragment::Program,
    authored_alpha: Option<f32>,
) -> u32 {
    rim_glow::classify(declared, program, authored_alpha)
}

/// The Plasma head's per-material alpha parameter hash - see
/// [`rim_glow_bit`].
pub const RIM_EDGE_ALPHA: u32 = rim_glow::RIM_EDGE_ALPHA;

/// A loader that finds nothing, for a caller with no archive in hand.
///
/// **Not a convenience** - it is the honest way to build geometry when the
/// textures cannot be reached, and it produces the untextured model this module
/// produced before `.gtf` was read, with every slot counted as missing rather
/// than silently white.
pub fn no_textures(_: &str) -> Option<Vec<u8>> {
    None
}

mod cutout;
mod isolate;
mod light_cone;
mod mag_wave;
mod pad_ne;
mod pads;
mod refraction;
pub use pads::{build_pads, build_weapon_pads};
mod bounds;
mod glass_sheen;
mod ice;
pub mod psp2;
mod rim_glow;
mod skin;
mod vertex_scroll;
mod water;
use bounds::bounding_sphere;

mod emissive;
pub use emissive::EMISSIVE_LIMIT;

mod place;
use place::{is_world_baked, node_geometry, referenced};

pub mod curve_track;

mod setup;
use setup::{MaterialSetup, material_setup};

mod scene;
use scene::Setup;
pub use scene::{BehindGlass, View, build_scene, build_scene_views};

/// Whether a chunk's declaration names no texture coordinate.
///
/// `false` for a chunk with no declaration at all - an inline one, where
/// `rcsmodel::Mesh::texcoords` still reads the last four bytes and the count
/// would be about this module rather than about the data.
fn declares_no_texcoord(mesh: &rcsmodel::Mesh) -> bool {
    mesh.decl
        .as_ref()
        .is_some_and(|decl| decl.diffuse_texcoord().is_none())
}

/// How one chunk is drawn: which texture slot, and blended or not.
///
/// # `.gtf` is what turned the see-through chunks back on
///
/// **The alpha a blend needs is in the texture and nowhere else**, and until
/// `oag_texture::gtf` was read this module had none - so a see-through chunk was
/// *left out* rather than blended, because alpha-over at `alpha = 1.0` paints
/// exactly the opaque pixels while dropping depth write and additive blows a
/// glass panel to white. That stopgap is gone: `Texture::to_rgba` returns RGBA,
/// the shader's `fs_main_blend` multiplies the texel's alpha into its output,
/// and these surfaces are drawn with the equation the material asks for.
///
/// The stakes are the same as they were: on Talon's Junction the largest surface
/// in the whole file is a 63-triangle `clouds` plate spanning 2,011 x 2,195
/// world units, and drawn opaque it covers the circuit - seen from above the
/// track was one white blob. 4.3 % to 35.9 % of chunks are see-through across
/// all 16 circuits (`crates/formats/tests/rcsmodel_material_ground_truth.rs`).
///
/// # A texture that will not paint is *not* a reason to blend
///
/// A material whose `.gtf` is missing or refused has no alpha either, so
/// blending it would put the white 1x1 through the transparent pass and paint
/// the same sheet with depth write off - strictly worse than before. Such a
/// chunk stays in the opaque pass and is counted in [`Report::untextured`].
fn surface(
    model: &rcsmodel::Model,
    mesh: &rcsmodel::Mesh,
    skin: &[Option<std::sync::Arc<ModelTexture>>],
    material_slots: &[u32],
    material_specular_exponent: &[f32],
    material_anim: &[u32],
) -> Surface {
    let slot = mesh.material as usize;
    let texture = skin.get(slot).and_then(Option::as_ref).map(|_| slot);
    // Which of `Model::anim_tracks` this material's own curve (if any) drives
    // - see `curve_track::material_anim_tracks`. `0` (no track) for a
    // material with nothing authored to animate, the same "index plus one"
    // shape `GpuVertex::anim` already carries for a Pulse/Pure `TEXOFFSET`
    // block.
    let anim = material_anim.get(slot).copied().unwrap_or(0);
    // `DEFAULT` for a slot with no reading, which is what every title but HD
    // has and what an HD material whose microcode did not trace answers.
    let mut roles = material_slots.get(slot).copied().unwrap_or(slots::DEFAULT);
    // The one per-chunk bit in the word: the chunk's own render-block flags
    // say whether it is track surface, and a Zone race binds the `Track` or
    // the `Scene` parameter set on that - see `slots::ZONE_TRACK`. Every
    // surface of a chunk carries the chunk's flags, so this holds for an
    // extra surface too.
    if mesh.is_track() {
        roles |= slots::ZONE_TRACK;
    }
    // Same fallback rule, same reason - see `mesh::vertex::GpuVertex::specular_exponent`.
    let specular_exponent = material_specular_exponent
        .get(slot)
        .copied()
        .unwrap_or(crate::mesh::DEFAULT_SPECULAR_EXPONENT);
    let material = model.material_of(mesh);
    let blend = match material.map(rcsmodel::Material::blend) {
        // Only a painted surface can be blended - see above.
        Some(rcsmodel::Blend::Factors { src, dst }) if texture.is_some() => {
            Some(blend_state(src, dst))
        }
        _ => None,
    };
    // Same rule, same reason: the alpha a cutout tests is the texture's, so an
    // unpainted one has nothing to test and stays in the opaque pass rather
    // than discarding against the white 1x1's alpha of 1.
    let cutout = texture.is_some() && material.is_some_and(|m| cutout::of(m).is_some());
    Surface {
        texture,
        blend,
        cutout,
        roles,
        specular_exponent,
        anim,
    }
}

/// One authored factor pair as the state a pipeline is built with.
///
/// **A translation, not a decision.** `oag_rcs::rcsmodel::Factor` is the
/// disc's own four values under names, and each has exactly one counterpart in
/// `wgpu`; the operation is `Add` because the RSX's blend equation register is a
/// separate field this reading has not touched and `GL_FUNC_ADD` is what it
/// holds at reset.
///
/// **Alpha follows colour rather than splitting**, for the reason
/// [`crate::mesh_render::ADDITIVE_BLEND`] gives: a target later read as
/// premultiplied should not disagree with its own colour channels. The disc says
/// nothing about the alpha channel either way - a `CellGcmBlendFunc` pair is
/// programmed for both and only the colour half is what these materials vary.
pub fn blend_state(src: rcsmodel::Factor, dst: rcsmodel::Factor) -> wgpu::BlendState {
    fn factor(f: rcsmodel::Factor) -> wgpu::BlendFactor {
        match f {
            rcsmodel::Factor::One => wgpu::BlendFactor::One,
            rcsmodel::Factor::SrcColour => wgpu::BlendFactor::Src,
            rcsmodel::Factor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
            rcsmodel::Factor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
        }
    }
    let component = wgpu::BlendComponent {
        src_factor: factor(src),
        dst_factor: factor(dst),
        operation: wgpu::BlendOperation::Add,
    };
    wgpu::BlendState {
        color: component,
        alpha: component,
    }
}

/// One submesh's decoded arrays, as [`emit`] takes them.
///
/// A struct rather than four parameters because they are one thing - the same
/// submesh read four ways, all of them the same length - and because the two
/// call sites would otherwise differ only in an argument's position.
#[derive(Debug, Clone, Copy)]
struct Geometry<'a> {
    points: &'a [[f32; 3]],
    /// `None` leaves every vertex normal zero, which is [`face_normals`]'
    /// signal to derive one.
    normals: Option<&'a [[f32; 3]]>,
    /// `None` leaves every coordinate at the origin of the texture.
    texcoords: Option<&'a [[f32; 2]]>,
    /// Where the circuit's lightmap atlas is sampled, for a chunk that declares
    /// a `lightmapUV`. `None` leaves it at the origin, which a white lightmap
    /// makes harmless.
    lightmap_texcoords: Option<&'a [[f32; 2]]>,
    /// The second diffuse set, read only for [`slots::ICE`] surfaces.
    texcoords2: Option<&'a [[f32; 2]]>,
    /// HD's **baked per-vertex light and sun-occlusion mask**, `[r, g, b,
    /// mask]`, from `oag_rcs::rcsmodel::Mesh::vertex_light`. `None` on 632
    /// of Talon's Junction's 983 chunks - not a gap but the other half of the
    /// split: a chunk bakes into the lightmap atlas **or** its vertices, never
    /// both.
    vertex_light: Option<&'a [[f32; 4]]>,
    indices: &'a [u16],
    /// Which chunk of the `.rcsmodel` this geometry is, in file order.
    ///
    /// The join key for `track.pvs`, whose per-cell bitmaps are indexed by
    /// exactly this number - see [`oag_rcs::hd_pvs`]. `None` for geometry
    /// that is not a chunk of the model the PVS was authored against.
    chunk: Option<u32>,
}

/// What [`surface`] decided, carried into [`emit`].
#[derive(Debug, Clone, Copy, Default)]
struct Surface {
    /// Index into `Model::textures`, or `None` for a material with no painted
    /// texture.
    texture: Option<usize>,
    /// What the material's own microcode says its two texture units are for,
    /// packed as [`slots`] and handed to every vertex this surface emits.
    roles: u32,
    /// This material's specular exponent, handed to every vertex this
    /// surface emits - see `mesh::vertex::GpuVertex::specular_exponent`.
    specular_exponent: f32,
    /// The blend equation the material authors, or `None` for the opaque pass.
    ///
    /// The state itself rather than a `vex::BlendClass`, because a PS3 material
    /// authors a factor *pair* and Pulse's three classes have no member for
    /// most of them - see `mesh::DrawCall::blend_state`.
    blend: Option<wgpu::BlendState>,
    /// Whether this surface is an alpha-test cutout - `Transparency::Mode2`,
    /// see [`cutout`]. Never set together with [`Self::blend`]: mode 2 turns
    /// the alpha test on and blending off, and they are separate registers.
    cutout: bool,
    /// Which of `Model::anim_tracks` this surface's material drives, plus
    /// one; `0` (identity) for a material with no curve. See
    /// [`curve_track::material_anim_tracks`].
    anim: u32,
}

/// Zero for a coordinate that is not a finite number.
///
/// **Almost nothing reaches this any more, and it stays.** 1.68 % of a
/// circuit's vertices used to decode to an infinity or a NaN here, and every
/// one of them was a `tangent` or a colour set read as a coordinate because the
/// reader took the last four bytes of a vertex; reading where
/// `rcsmodel::VertexDecl` says leaves Talon's Junction with **9** of 600,280
/// and Assegai with 6 of 25,144, which are a different and still-unexplained
/// thing. An inline chunk has no declaration at all, so the guard is also what
/// those go through. A NaN in a vertex buffer is not a visible failure, it is a
/// hole in the rasteriser's output, so it is pinned to zero here.
fn finite(v: f32) -> f32 {
    if v.is_finite() { v } else { 0.0 }
}

/// How many of a submesh's decoded normals the file actually authored.
///
/// **A zero one did not come from the file**, it came from a vertex record that
/// is zero from the position onward - padding at the end of a buffer, and
/// `cockpit_screenShape` is one that carries some. `emit` derives a normal for
/// those from the triangles, so counting them here would make the load report
/// claim the disc's data where the fallback ran.
fn authored(normals: &[[f32; 3]]) -> usize {
    normals
        .iter()
        .filter(|n| Vec3::from_array(**n).length_squared() > 1e-12)
        .count()
}

/// Appends one submesh's geometry to a model, as its own draw call.
///
/// `normals` are the file's own, already decoded; `None` leaves the vertex
/// normal zero, which is [`face_normals`]'s signal to derive one.
///
/// `place` is what an `Anim Transform` above this node changes: the vertices
/// are baked in that node's space rather than in world space, they carry its
/// slot, and the draw call's bounding sphere is lifted back out by its
/// time-zero matrix. See `mesh::anim_node::placement`, which decides all three
/// together so they cannot get out of step.
fn emit(
    out: &mut Model,
    mesh: Geometry<'_>,
    place: &anim_node::Placement,
    node: Option<u32>,
    surface: Surface,
) {
    let to_world = Mat4::from_cols_array(&place.to_world);
    let Geometry {
        points,
        normals,
        texcoords,
        lightmap_texcoords,
        texcoords2,
        vertex_light,
        indices,
        chunk,
    } = mesh;
    let flip_v = surface.roles & slots::FLIP_V != 0;
    let first_vertex = u32::try_from(out.vertices.len()).unwrap_or(u32::MAX);
    let first_index = u32::try_from(out.indices.len()).unwrap_or(u32::MAX);
    let mut centre = Vec3::ZERO;
    for (k, point) in points.iter().enumerate() {
        let p = to_world.transform_point3(Vec3::from_array(*point));
        centre += p;
        // The node transforms in these files are rigid, so rotating the
        // direction and renormalising is the whole of it - an inverse transpose
        // would be needed only under non-uniform scale.
        let normal = normals
            .and_then(|n| n.get(k))
            .map(|n| {
                to_world
                    .transform_vector3(Vec3::from_array(*n))
                    .normalize_or_zero()
                    .to_array()
            })
            .unwrap_or([0.0, 0.0, 0.0]);
        let light = vertex_light.and_then(|c| c.get(k));
        out.vertices.push(GpuVertex {
            position: p.to_array(),
            // Zero means the file gave none, and `face_normals` derives it.
            normal,
            // HD's `f[TC1]`: the per-vertex light the fragment program
            // **adds** to the lightmap term before multiplying the albedo, so
            // `mesh.wesl` folds it into the authored sum, not into a tint.
            // Alpha stays the texture's own multiplier - `colour.a` is not
            // where the fourth byte goes, see `sun_mask` below.
            // **Zero for a chunk with no colour set is read**: the blocks
            // without it write `MOV o[TC1].xyz, c[K].xxxx`, and `c[K]` against
            // each program's local-constant table is 0.0 in 11,180 of 11,184.
            colour: light
                .map(|&[r, g, b, _]| [finite(r), finite(g), finite(b), 1.0])
                .unwrap_or([0.0, 0.0, 0.0, 1.0]),
            // A non-finite half is a vertex whose coordinate this reading does
            // not explain - 1.68 % of a circuit's stride-18 ones. Zero rather
            // than a NaN travelling into the vertex buffer.
            lightmap_texcoord: lightmap_texcoords
                .and_then(|t| t.get(k))
                .map(|&[u, v]| [finite(u), finite(v)])
                .unwrap_or([0.0, 0.0]),
            // **`1 - v` where the material's own vertex program writes it**,
            // which one of Talon's Junction's 283 resolved variants does. See
            // `oag_rcs::rcsmaterial::vertex` for the microcode and
            // `skin::flips` for why this is per material rather than global.
            texcoord: texcoords
                .and_then(|t| t.get(k))
                .map(|&[u, v]| {
                    let v = finite(v);
                    [finite(u), if flip_v { 1.0 - v } else { v }]
                })
                .unwrap_or([0.0, 0.0]),
            // **Unlit under the tint diagnostic**, so the flat colour reaches
            // the frame as itself: `lit` 0.0 takes `mesh.wesl`'s stand-in
            // path, whose light and tint are both 1.0 for an HD model, and
            // the palette entry can be matched exactly rather than by hue
            // through a coloured light rig.
            lit: if isolate::tinting() || isolate::unlit() {
                0.0
            } else {
                1.0
            },
            anim: surface.anim,
            slots: surface.roles,
            xform: place.xform,
            // The colour set's fourth byte - see
            // `oag_rcs::rcsmodel::Mesh::vertex_light` and
            // `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "The sun is
            // real and it is masked". `1.0` (unmasked) for a chunk with no
            // colour set, which is what a lightmapped chunk uses instead -
            // `mesh.wesl` multiplies this by the lightmap's own alpha, so
            // `1.0` here leaves that gate untouched.
            sun_mask: light.map(|&[.., m]| finite(m)).unwrap_or(1.0),
            specular_exponent: surface.specular_exponent,
            glow: 0.0,
            texcoord2: texcoords2
                .and_then(|t| t.get(k))
                .map_or([0.0, 0.0], |&[u, v]| [finite(u), finite(v)]),
        });
    }
    let centre = centre / points.len() as f32;
    let radius = out.vertices[first_vertex as usize..]
        .iter()
        .map(|v| (Vec3::from_array(v.position) - centre).length())
        .fold(0.0f32, f32::max);
    // The vertices are in the anchor's space when this batch moves, so the
    // sphere is too; the anchor's own time-zero matrix makes it a world-space
    // statement about one instant, which is why `moving` turns the frustum
    // test off rather than trusting it.
    let centre = place.bounds_matrix.map_or(centre, |m| {
        Vec3::from_array(vex::transform_point(&m, centre.to_array()))
    });

    out.indices
        .extend(indices.iter().map(|&i| first_vertex + u32::from(i)));
    // Three lists, and the cutout is the middle one: it writes depth like the
    // opaque pass and is drawn before the blended pass, which is exactly what
    // an alpha test is - see `cutout` and `mesh::Model::alpha_tested_draws`.
    let list = match (surface.blend, surface.cutout) {
        (Some(_), _) => &mut out.transparent_draws,
        (None, true) => &mut out.alpha_tested_draws,
        (None, false) => &mut out.draws,
    };
    list.push(DrawCall {
        moving: place.xform != 0,
        range: first_index..u32::try_from(out.indices.len()).unwrap_or(u32::MAX),
        texture: surface.texture,
        bounds: Bounds {
            centre: centre.to_array(),
            radius,
        },
        culled: false,
        // **`None`, and that is not an omission.** `DrawCall::blend` holds the
        // class a *Pulse* batch's `pass_mask` names, and a PS3 material names
        // none - it authors a factor pair, which `blend_state` carries. Putting
        // the nearest member there instead would be the same fold this module
        // just stopped doing: 144 of the disc's see-through materials are
        // `ONE`/`ONE`, and calling them `AlphaOver` in a field defined as a
        // recovered class is a fabricated value even where nothing reads it.
        // Which list a draw is in, above, is what says it is transparent.
        blend: None,
        blend_state: surface.blend,
        // A PS3 chunk has no `.vex` mesh payload and so no derived layer; the
        // uniform value leaves `Model::sort_by_layer` holding chunk order.
        layer: vex::LAYER_DEFAULT,
        node,
        chunk,
        // A PS3 material's own reference is per *material* and reaches the
        // shader through `Model::alpha_test_ref` instead - see
        // `mesh::rcs::cutout`. `None` here leaves that path alone.
        alpha_test_ref: None,
    });
}

/// Flattens every node of `class` into one buffer pair, taking its geometry
/// from `model_blob`.
///
/// # Errors
///
/// A `.vex` that will not walk, a `.rcsmodel` that will not parse, and a class
/// id this file's version does not number. **A mesh that cannot be drawn is not
/// an error** - it is counted in the returned [`Report`], because on a circuit
/// that is the ordinary case and failing the load over it would draw nothing at
/// all.
///
/// **Never skips a world-baked node reference on its own** - see
/// [`build_with_options`], which this calls with `world_space_fallback:
/// false`. That is what every caller outside this module wants: a craft's
/// `mesh::rcs::build` call (`oag_game::livery`, `livery::flare`) has no
/// second pass to catch a wrongly-skipped part, so skipping here would draw
/// nothing for it rather than draw it through the (harmless, if wrong-transform)
/// node path - an invisible ship part being strictly worse than a
/// coincidentally-placed one. Only [`build_scene`] - which *does* have a
/// second, world-space pass right below it - is allowed to ask for the skip.
pub fn build(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    textures: Textures<'_>,
    pick: fn(vex::classes::Classes) -> Option<u32>,
) -> Result<(Model, Report)> {
    build_with_options(
        label,
        data,
        model_blob,
        Setup::Decode(textures),
        pick,
        false,
        View::Main,
    )
}

/// [`build`], with the world-bake skip [`is_world_baked`] documents - on only
/// when the caller has a world-space pass ready to draw the skipped chunk
/// instead, which is why this is not `pub`: [`build_scene`] is the one caller
/// that qualifies, and every other caller goes through [`build`] instead.
///
/// `setup` is where the materials come from, and `view` which chunks it draws -
/// see [`scene::View`].
fn build_with_options(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    setup: Setup<'_>,
    pick: fn(vex::classes::Classes) -> Option<u32>,
    world_space_fallback: bool,
    view: View,
) -> Result<(Model, Report)> {
    if !vex::has_magic(data) {
        bail!("{label} is not a .vex file (no VEXX magic)");
    }
    let classes = vex::classes_of(data).with_context(|| format!("{label}: class table"))?;
    let Some(class_id) = pick(classes) else {
        bail!(
            "{label} is .vex version {}, and the class id for this node type has not \
             been recovered for it",
            classes.version
        );
    };
    let model = rcsmodel::Model::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("{label}: the .rcsmodel beside it: {e}"))?;

    let nodes = vex::nodes(data).context("walking the node tree")?;
    // Composed exactly as the PSP path composes them: a mesh's positions are in
    // the space of whichever transform encloses it, nested up to 25 deep on a
    // circuit. Nothing about that moved to the PS3.
    let world = vex::world_transforms(data, &nodes);
    let order = vex::byte_order(data);
    // **Anchored, not absolute, for anything under an `Anim Transform`.** The
    // vertices are baked once at load and the node's matrix changes every
    // frame, so the node has to stay out of the bake - exactly as
    // `super::build_class` does it, and on the same data: HD authors 5,518 of
    // these nodes and 2,321 of a circuit's `Mesh` nodes hang under one. `world`
    // above is kept alongside, because [`is_world_baked`] asks a world-space
    // question and gets a world-space answer.
    let anchors = vex::anim_anchors(data, &nodes);
    let (anim_nodes, anim_slot) = anim_node::collect(data, &nodes, &anchors, classes);
    let anchor_world = vex::anchor_world(data, &nodes, 0.0);

    let mut out = Model::none(label);
    out.anim_nodes = anim_nodes;
    // The same per-frame switch the PSP builder tags for, keyed by the same
    // node indices `emit` stamps on each draw - chosen for HD, not measured:
    // its own switch code is unread. It hides nothing on the HD disc, whose
    // groups hold at most one child each (`hd_lod_ground_truth.rs`). See
    // `super::LodGroups`.
    out.lod_groups = super::LodGroups::collect(data, &nodes, classes, &anchors, &anchor_world);
    let mut report = Report::default();
    let MaterialSetup {
        textures: skins,
        lightmaps: seconds,
        material_slots,
        material_specular_exponent,
        material_variants,
        emissive,
        alpha_test_ref,
        mag_emissive,
        wave_maps,
        material_anim,
        anim_tracks,
    } = match setup {
        Setup::Decode(textures) => material_setup(&model, model_blob, textures, &mut report),
        Setup::Copy(from) => MaterialSetup::of(from),
    };
    out.pad_masks = mag_emissive;
    out.wave_maps = wave_maps;
    out.emissive = emissive;
    out.textures = skins;
    out.lightmaps = seconds;
    // Every vertex this module writes carries HD's baked per-vertex light in
    // `colour`, which the fragment programs **add** rather than multiply - see
    // `emit`. The flag is what stops the stand-in shading path from tinting by
    // it, and it is a property of the model rather than of the target because
    // the capture and viewer paths draw these same models into a gamma one.
    out.vertex_colour_is_light = true;
    // The disc's own alpha-test reference, for the cutout draws below - see
    // `cutout`, which reports a comparison this shader cannot reproduce
    // rather than drawing one wrongly.
    out.alpha_test_ref = alpha_test_ref;
    out.material_variants = material_variants;
    out.material_slots = material_slots;
    out.material_specular_exponent = material_specular_exponent;
    out.material_anim = material_anim;
    out.anim_tracks = anim_tracks;

    for (index, node) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.class_id == class_id)
    {
        report.nodes += 1;
        let Some((hash, min, max)) = node_geometry(&data[node.payload()], order) else {
            continue;
        };
        let (Some(chunk_index), Some(chunk)) = (model.mesh_index(hash), model.mesh(hash)) else {
            continue;
        };
        let mesh = chunk;
        // **Skip a chunk this node names but does not actually place.** See
        // `is_world_baked`: drawing it through `to_world` below would test
        // its already-world-space positions against this node's un-transformed
        // local box, which cannot pass, and letting it fall through to
        // `report.no_stride`/`strays` invites `referenced`'s exclusion (which
        // hands the same hash to the world-space pass) to draw it twice
        // wherever the loose `submesh_fits` tolerance happens to pass anyway.
        // The world-space question, asked of the world-space matrix: `place`
        // below may bake this node's vertices in its anchor's space instead,
        // and testing the chunk's own bias against a matrix that is not world
        // space would answer a different question.
        let to_world = Mat4::from_cols_array(&world[index]);
        if world_space_fallback && is_world_baked(mesh, min, max, to_world) {
            report.world_baked += 1;
            continue;
        }
        if isolate::excludes(&model, mesh) {
            report.isolated += 1;
            continue;
        }
        if !view.draws(mesh) {
            report.behind_glass += usize::from(mesh.is_behind_glass());
            continue;
        }
        report.addressed += 1;
        let place = anim_node::placement(&anchors, &anchor_world, &anim_slot, index);
        let mut emitted = false;
        let first_vertex = u32::try_from(out.vertices.len()).unwrap_or(u32::MAX);
        // Every surface, as the world-space pass does - `Mesh::surfaces`.
        for mesh in chunk.surfaces() {
            let tolerance = mesh.scale.iter().fold(0.0f32, |a, &b| a.max(b)) * TOLERANCE_STEPS;
            // **What the chunk declares, first.** The searches below fit a stride
            // to the authored box and to buffer layout, and they exist because this
            // field had not been read; see `rcsmodel::vertex_decl`. They stay for
            // an inline chunk, which declares nothing.
            let Some(stride) = mesh
                .declared_stride()
                .or_else(|| mesh.solve_stride(model_blob, (min, max), tolerance))
                .or_else(|| mesh.solve_stride_by_layout())
                .or_else(|| mesh.solve_stride_by_normals(model_blob))
            else {
                report.no_stride += 1;
                continue;
            };

            let surface = surface(
                &model,
                mesh,
                &out.textures,
                &out.material_slots,
                &out.material_specular_exponent,
                &out.material_anim,
            );
            report.see_through += usize::from(surface.blend.is_some());
            report.cutout += usize::from(surface.cutout);
            report.no_texcoord += usize::from(declares_no_texcoord(mesh));
            report.track_surface += usize::from(std::ptr::eq(mesh, chunk) && mesh.is_track());
            for submesh in &mesh.submeshes {
                if submesh.vertex_count == 0 || submesh.index_count == 0 {
                    continue;
                }
                // **The stride is the mesh's, and one submesh may not share it.**
                // `solve_stride` tolerates that; drawing must not, or the odd
                // submesh's attribute bytes are read as positions and scatter over
                // the world. See `rcsmodel::Mesh::submesh_fits`.
                //
                // **The node's box describes the chunk, so only the chunk's own
                // surface is judged against it.** A later surface has its own bias
                // and is not inside that box in the first place; testing it there
                // would drop it as a stray for being exactly where it belongs.
                if std::ptr::eq(mesh, chunk)
                    && !mesh.submesh_fits(model_blob, submesh, stride, (min, max))
                {
                    report.strays += 1;
                    continue;
                }
                let (Ok(points), Ok(indices)) = (
                    mesh.positions(model_blob, submesh, stride),
                    mesh.indices(model_blob, submesh),
                ) else {
                    continue;
                };

                let normals = mesh.normals(model_blob, submesh, stride).ok();
                let texcoords = mesh.texcoords(model_blob, submesh, stride).ok();
                let lightmap_texcoords = mesh.lightmap_texcoords(model_blob, submesh, stride).ok();
                let vertex_light = mesh.vertex_light(model_blob, submesh, stride).ok();
                let texcoords2 = ice::second_uv(surface.roles, mesh, model_blob, submesh, stride);
                report.authored_normals += normals.as_deref().map_or(0, authored);
                emit(
                    &mut out,
                    Geometry {
                        points: &points,
                        normals: normals.as_deref(),
                        texcoords: texcoords.as_deref(),
                        lightmap_texcoords: lightmap_texcoords.as_deref(),
                        texcoords2: texcoords2.as_deref(),
                        vertex_light: vertex_light.as_deref(),
                        indices: &indices,
                        chunk: u32::try_from(chunk_index).ok(),
                    },
                    &place,
                    u32::try_from(index).ok(),
                    surface,
                );
                report.triangles += indices.len() / 3;
                emitted = true;
            }
        }
        if emitted {
            report.drawn += 1;
            out.mesh_count += 1;
            // The same flap the `.vex`-geometry builder records: HD's
            // `Ship.vex` authors the `Airbrake` tree and the `.rcsmodel` only
            // supplies the triangles, which this node has just appended.
            super::Flap::collect(
                &mut out.airbrakes,
                &nodes,
                classes,
                &anchors,
                node,
                first_vertex..u32::try_from(out.vertices.len()).unwrap_or(u32::MAX),
            );
        }
    }

    face_normals(&mut out);
    let (centre, radius) = bounding_sphere(&out.vertices);
    out.centre = centre;
    out.radius = radius;
    Ok((out, report))
}

/// Gives every vertex the average of the faces meeting at it.
///
/// # This is a derivation, not a substitute
///
/// **The `.rcsmodel` does carry authored normals and they are not read** - they
/// are in the 8 to 16 attribute bytes after each position, whose layout is
/// unrecovered. What is computed here comes out of the triangles this module
/// already decoded, so it is a fact about the geometry rather than a stand-in
/// for data nobody has: an unlit model is a white silhouette, and a silhouette
/// hides exactly the decoding mistakes this is meant to expose.
///
/// It will differ from the authored normals wherever the artists split or
/// smoothed them by hand, so a model lit this way is a shape check and not a
/// match against the original. Reading the real ones supersedes it.
pub(crate) fn face_normals(model: &mut Model) {
    // **Only where the file gave none.** A zero normal is `emit`'s signal that
    // `rcsmodel::Mesh::normals` had nothing for that vertex; an authored one is
    // better than anything derivable here, because it carries the hard edges the
    // exporter split vertices for and a smooth average by construction cannot.
    let mut derived = vec![Vec3::ZERO; model.vertices.len()];
    for triangle in model.indices.as_chunks::<3>().0 {
        let [a, b, c] = [triangle[0], triangle[1], triangle[2]].map(|i| i as usize);
        let (pa, pb, pc) = (
            Vec3::from_array(model.vertices[a].position),
            Vec3::from_array(model.vertices[b].position),
            Vec3::from_array(model.vertices[c].position),
        );
        let face = (pb - pa).cross(pc - pa);
        for index in [a, b, c] {
            derived[index] += face;
        }
    }
    for (vertex, derived) in model.vertices.iter_mut().zip(derived) {
        if Vec3::from_array(vertex.normal).length_squared() > 1e-12 {
            continue;
        }
        // A vertex on no triangle, or on exactly cancelling ones, keeps a
        // usable up rather than a zero the shader would normalise to NaN.
        vertex.normal = if derived.length_squared() > 1e-12 {
            derived.normalize().to_array()
        } else {
            [0.0, 1.0, 0.0]
        };
    }
}

#[cfg(test)]
mod tests;
