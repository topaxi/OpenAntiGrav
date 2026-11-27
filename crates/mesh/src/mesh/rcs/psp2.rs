//! Drawing Wipeout 2048's `.rcsmodel`.
//!
//! [`super`] builds Wipeout HD's container, which needs the `.vex` beside it -
//! a chunk is addressed by hash and its vertex stride comes from the node's
//! authored bounding box. **2048's container needs neither**: every submesh
//! carries its own counts and its own buffer pointers, and the stride falls out
//! of the buffer packing. So this builder takes the model blob alone.
//!
//! **A node-bound submesh is placed, and moved, by its node.** The model's
//! own table names a node for 130 of `altima`'s 1,153 meshes (a
//! median 1,086 units from where they are authored, as-is), and the
//! `.rcsskeleton`/`.rcsanimclip` beside the circuit animate those nodes -
//! see [`placement`] for how a node either bakes through its static world
//! matrix or takes a slot of the shader's node table, and
//! `docs/formats/2048-animation.md` for the format and its evidence.
//!
//! What it draws is positions, triangles, normals, diffuse texture
//! coordinates - the file's own where a submesh's stride carries one, computed
//! off the triangles where normals do not (see [`super::face_normals`]) - and
//! **each submesh's own material's texture**, through the per-submesh material
//! index [`psp2::SubMesh::material`] carries.
//!
//! **That binding is per-submesh, not per-model**, which is what makes a
//! multi-material model - the craft (6 materials) and the circuit (527) both
//! are - paint more than one texture. Each distinct material resolves to at
//! most one decoded [`ModelTexture`], shared by every draw that names it, so a
//! 2,800-submesh circuit uploads a few hundred textures rather than 2,800
//! copies of them. A submesh whose material has no index, no texture path, no
//! archive entry or a texture that will not decode gets **no texture at all**
//! rather than a neighbour's - see [`Report::describe`], which counts each of
//! those separately instead of leaving a silent difference.

use anyhow::Result;
use oag_display::display::Water;

use circuit_pads::HidePads;
use oag_core::math::{Mat4, Vec3};
use oag_rcs::rcsmodel::psp2;

use super::Textures;
use crate::mesh::{Bounds, DrawCall, Flap, GnfForm, GpuVertex, Model, ModelTexture};
use oag_rcs::gxp::sampler_use;
use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmodel::psp2::lighting_only;

mod alpha_emissive;
mod alpha_product;
mod cannon_bolt;
mod chevron;
pub mod circuit_pads;
mod crowd;
mod crowd_rigs;
mod electricity;
mod emissive_add;
mod flat_constant;
mod glow;
mod hull_colour;
mod lit_alpha;
mod normal_map;
mod panel_map;
mod picture_memo;
pub mod placement;
pub mod refraction;
pub mod refraction_only;
mod report;
mod shield_lightning;
mod sky_reflection;
mod snow;
mod static_hologram;
mod tarmac;
mod transparency;
mod twin_blend;
mod uv_flip;
mod uv_keys;
mod vertex_blend;
pub mod water;

pub use chevron::PLASMA_COMBINE;
pub use hull_colour::Census as HullColourCensus;
pub use normal_map::Census as NormalMapCensus;
pub use twin_blend::INHERITED_RATE as TWIN_BLEND_RATE;

pub use placement::Animation;
pub use report::Report;

/// The vertex colour of a `fc01_dummy` submesh before the loader knows the
/// fog: negative components no authored colour has. `oag_raceplay` replaces it
/// with the circuit's `fogColour`; one left in place is drawn black.
pub const FOG_FLAT: [f32; 4] = [-1.0, -1.0, -1.0, 1.0];

/// A `Zone_Colour<k>` vertex carries `colour.a = ZONE_COLOUR_ALPHA + k`
/// (`k` in `1..=8`, so `2.0..=9.0`): the shader reads the showing stage's
/// colour for slot `k` from `Zone::stage_colours` and keeps the vertex's own
/// rgb, the material's authored colour, where the stage table gives none.
pub const ZONE_COLOUR_ALPHA: f32 = 1.0;

/// Whether a mesh object's Maya shape name says it is track surface: `wohdtrackShape`,
/// `TRACK_0Shape`, `TRACK_Shape`, any shape with `track` in its name.
///
/// **Chosen, not measured.** HD authors "this chunk is track surface" as a bit of
/// the chunk's render-block record (`oag_rcs::rcsmodel::Mesh::is_track`), which a
/// Zone race reads to publish the `Track` colour group and `zoneModeTrack<n>`
/// instead of the `Scene` ones. 2048's and Omega's container has no such record
/// that has been read (the mesh object's `+0x0a` flags are `0x0101`/`0x0201` on
/// the road and on everything else alike), so the shape name stands in for it
/// where an HD-lineage circuit borrows HD's Zone recolour
/// (`docs/rendering/hd-zone-recolour.md`, "On 2048's container").
#[must_use]
pub fn is_track_shape(name: &str) -> bool {
    name.to_ascii_lowercase().contains("track")
}

/// Whether a blob is 2048's container rather than Wipeout HD's.
///
/// Cheap and exact: the two containers disagree on their very first word, and
/// neither value is a plausible reading of the other.
#[must_use]
pub fn is_psp2(model_blob: &[u8]) -> bool {
    model_blob
        .get(0..4)
        .is_some_and(|b| u32::from_le_bytes(b.try_into().expect("four bytes")) == psp2::MAGIC)
}

/// Decodes a material's diffuse texture in whichever container its path names.
///
/// **Offered to the texture sink** ([`crate::mesh_render::TextureSinkScope`]),
/// so a load that has a device uploads each texture as it is decoded.
fn decode_material_texture(path: &str, blob: &[u8], report: &mut Report) -> Option<ModelTexture> {
    let (texture, form) = decode_texture_form(path, blob)?;
    if let Some(form) = form {
        report.gnf.record(form);
    }
    Some(crate::mesh_render::offer_to_texture_sink(texture))
}

/// [`decode_material_texture`]'s decode alone: the texture, and the form a
/// `.gnf` took for the report's counts.
fn decode_texture_form(path: &str, blob: &[u8]) -> Option<(ModelTexture, Option<GnfForm>)> {
    match oag_texture::decode::Container::of_name(path) {
        Some(oag_texture::decode::Container::Gnf) => {
            ModelTexture::from_gnf_form(path, blob).map(|(texture, form)| (texture, Some(form)))
        }
        _ => ModelTexture::from_gxt(path, blob).map(|texture| (texture, None)),
    }
}

/// A diffuse decoded once per path, and the form to count again on each
/// material that shares it, so the report reads as it did per material.
type Decoded = (std::sync::Arc<ModelTexture>, Option<GnfForm>);

/// Every picture the build decoded, by path: materials share a `.gxt` between them (Altima's
/// track resolves 523 material slots to 164 distinct files), and a second picture
/// (`vertex_blend`) is often another material's first.
type Pictures = std::collections::HashMap<String, Result<Decoded, Unbound>>;

/// One picture read and decoded, or why not.
fn decode_picture(path: &str, textures: Textures<'_>) -> Result<Decoded, Unbound> {
    let blob = textures(path).ok_or(Unbound::ArchiveMiss)?;
    let (texture, form) = decode_texture_form(path, &blob).ok_or(Unbound::DecodeRefused)?;
    let texture = crate::mesh_render::offer_to_texture_sink(texture);
    Ok((std::sync::Arc::new(texture), form))
}

/// Builds every submesh of a 2048 `.rcsmodel` into one model.
///
/// **A submesh with no node is in the space its caller draws in** - a
/// circuit's in world coordinates, a craft's about its own origin - and
/// nothing is composed onto it. **A node-bound one is in its node's space**
/// and is placed by [`placement::plan`]: baked through the node's static
/// world matrix, or left where it is and moved by the shader's node table
/// when `animation` names keys for it. See [`psp2::Model::positions`] and
/// [`psp2::nodes`].
///
/// `textures` is called once per **distinct material** that names a `.gxt`,
/// never once per draw - see `bind_textures` below.
///
/// # Errors
///
/// Propagates [`psp2::parse`].
pub fn build(
    label: &str,
    model_blob: &[u8],
    animation: Option<&Animation>,
    textures: Textures<'_>,
) -> Result<(Model, Report)> {
    build_circuit(
        label,
        model_blob,
        animation,
        textures,
        Water::Off,
        HidePads::None,
    )
}

/// [`build`] for a race's circuit, under the player's water setting. 2048's own seas
/// (`psp2::water`) draw their scrolled normal map under `original` and stay the flat colour
/// under any other value.
///
/// `hide_pads` says which of the circuit's own pad submeshes are left out
/// ([`circuit_pads::HidePads`]).
///
/// # Errors
///
/// Propagates [`psp2::parse`].
pub fn build_circuit(
    label: &str,
    model_blob: &[u8],
    animation: Option<&Animation>,
    textures: Textures<'_>,
    water: Water,
    hide_pads: HidePads,
) -> Result<(Model, Report)> {
    build_planned(
        label,
        model_blob,
        &|scene| placement::plan(scene, animation),
        None,
        textures,
        (water, hide_pads),
    )
}

/// [`build`], with the motion taken from the `.vex` the model was exported
/// beside - see [`placement::plan_from_vex`]. What Omega's front-end scene
/// needs: it ships no `.rcsskeleton` or `.rcsanimclip`.
///
/// # Errors
///
/// Propagates [`psp2::parse`].
pub fn build_with_vex(
    label: &str,
    model_blob: &[u8],
    vex_blob: &[u8],
    textures: Textures<'_>,
) -> Result<(Model, Report)> {
    build_planned(
        label,
        model_blob,
        &|scene| placement::plan_from_vex(scene, vex_blob),
        None,
        textures,
        (Water::Off, HidePads::None),
    )
}

/// [`build_with_vex`] for a billboard: its meshes also ride the UV keys their
/// `.vex` nodes author, which is how 2048 and Omega play what Wipeout HD
/// authors as a material curve - see [`uv_keys`].
///
/// A billboard build alone, because the engine points a material's
/// `uvOffset`/`uvScale` at the keyed instance in its billboard loader.
///
/// # Errors
///
/// Propagates [`psp2::parse`].
pub fn build_billboard(
    label: &str,
    model_blob: &[u8],
    vex_blob: &[u8],
    textures: Textures<'_>,
) -> Result<(Model, Report)> {
    build_planned(
        label,
        model_blob,
        &|scene| placement::plan_from_vex(scene, vex_blob),
        Some(vex_blob),
        textures,
        (Water::Off, HidePads::None),
    )
}

/// Which side an airbrake mesh is, by the artists' own name: the last
/// component of its Maya path, `Airbrake_Left` or `Airbrake_Right` (the shape
/// node under it, `Airbrake_LeftShape`, is the same mesh).
fn flap_side(path: &str) -> Option<usize> {
    let leaf = path.rsplit(['|', '/']).next().unwrap_or(path);
    let leaf = leaf.trim_suffix("Shape");
    if leaf.eq_ignore_ascii_case("Airbrake_Left") {
        Some(0)
    } else if leaf.eq_ignore_ascii_case("Airbrake_Right") {
        Some(1)
    } else {
        None
    }
}

fn build_planned(
    label: &str,
    model_blob: &[u8],
    planner: &dyn Fn(&psp2::nodes::Scene) -> placement::Plan,
    billboard_vex: Option<&[u8]>,
    textures: Textures<'_>,
    (water, hide_pads): (Water, HidePads),
) -> Result<(Model, Report)> {
    // The planners below each read a PS4 material file's programs; decode each file once.
    // And they share their pictures: a texture two planners bind is read and decoded once.
    oag_rcs::gcn::memoised(|| {
        picture_memo::scoped(|| {
            build_scanned(
                label,
                model_blob,
                planner,
                billboard_vex,
                textures,
                (water, hide_pads),
            )
        })
    })
}

fn build_scanned(
    label: &str,
    model_blob: &[u8],
    planner: &dyn Fn(&psp2::nodes::Scene) -> placement::Plan,
    billboard_vex: Option<&[u8]>,
    textures: Textures<'_>,
    (water, hide_pads): (Water, HidePads),
) -> Result<(Model, Report)> {
    let mut decoded = psp2::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("{label}: the .rcsmodel beside it: {e}"))?;
    let plan = planner(&decoded.scene);
    let ps4 = psp2::is_ps4(model_blob);

    let mut model = Model::none(label);
    model.sky_reflection = sky_reflection::load(label, ps4, &mut *textures);
    let mut report = Report {
        unpaired: decoded.unpaired_pointers,
        materials: decoded.materials.len(),
        anim_nodes: plan.anim_nodes.len(),
        unmatched_nodes: plan.unmatched,
        frozen_nodes: plan.frozen,
        ..Report::default()
    };
    model.anim_nodes = plan.anim_nodes;
    // Omega's crowd-entity shapes are replaced by rig instances (`psp2::crowd_rigs`).
    let rig_chunks = crowd_rigs::instance(&mut decoded, ps4, &mut *textures, &mut report);
    let decoded = decoded;
    report.panel.sky_reflection = model.sky_reflection.is_some();
    // Each material's own scroll, resolved before any vertex is written; the
    // glow layer's emissive texture is decoded here too, so a layer exists
    // only where its picture does.
    let mut glow = glow::plan(&decoded.materials, &mut *textures, &mut report);
    // `nr_twinblend` takes one more entry of the same table.
    let mut twin = twin_blend::plan(&decoded, &mut glow.emissive, &mut *textures, &mut report);
    // `dc_hologramwithstatic` rides the same bindings and glow table.
    static_hologram::plan(
        &decoded,
        (&mut twin, &mut glow.emissive),
        &mut *textures,
        &mut report,
    );
    // The Detonator mine's visualiser ring (`electricity.rcsmaterial`) rides them too.
    electricity::plan(
        &decoded,
        (&mut twin, &mut glow.emissive),
        &mut *textures,
        &mut report,
    );
    // The Detonator cannon's bolt rides them too.
    cannon_bolt::plan(
        &decoded,
        ps4,
        (&mut twin, &mut glow.emissive),
        &mut *textures,
        &mut report,
    );
    // `diffuse_normal_specular_emmissive_alpha` adds `Texture2Alpha^2 * Constant1` after the light.
    let mut emissive_add = emissive_add::plan(
        &decoded,
        ps4,
        &mut glow.emissive,
        &mut *textures,
        &mut report,
    );
    // `2048_shipshield_old`'s Lightning layer rides the same third binding.
    shield_lightning::plan(
        &decoded,
        ps4,
        &mut emissive_add,
        &mut glow.emissive,
        &mut *textures,
        &mut report,
    );
    // The `alpha_emissive` families add the picture's alpha times `glowtinrt` after the light.
    alpha_emissive::plan(
        &decoded,
        &mut glow,
        &mut emissive_add,
        &mut *textures,
        &mut report,
    );
    // `mt_tunnelrefraction` and `cl_tunnelrefraction` read the refraction target.
    let glass = refraction::plan(&decoded.materials, &mut glow.emissive, &mut report);
    // Materials whose vertex programs write `1 - v` into the diffuse coordinate.
    let uv_flip = uv_flip::plan(&decoded, ps4, &mut *textures, &mut report);
    // A PS4 submesh with a tangent reads a normal map, bound third.
    let mut normals = normal_map::plan(&decoded, ps4, &twin, &mut *textures, &mut report);
    // Omega's `RT_snow1` binds its drift normal third and two detail normals after it.
    let snow = snow::plan(
        &decoded,
        ps4,
        (&mut normals, &mut glow.emissive),
        &mut *textures,
        &mut report,
    );
    // 2048's own seas scroll a normal map, bound third as the PS4's are.
    let sea = water::plan(
        &decoded,
        ps4,
        water == Water::Original,
        &mut glow,
        (&twin, &mut normals),
        &mut *textures,
        &mut report,
    );
    // Omega's hull programs multiply `VertexColour1` into the light (`psp2::hull_colour`).
    let hull_colour = hull_colour::plan(&decoded, ps4, &mut *textures, &mut report.hull_colour);
    // Omega's `tarmac_markings_*` roads lay a dirt splat and a decal atlas over the diffuse.
    let mut tarmac = tarmac::plan(&decoded, &mut glow.emissive, &mut *textures, &mut report);
    // Omega's panel-map roads march a panel map and reflect the sky cube (`psp2::panel_map`).
    let sky = (ps4, &normals);
    panel_map::plan(
        &decoded,
        sky,
        &mut tarmac,
        &mut glow.emissive,
        textures,
        &mut report,
    );
    // Omega's crowd sprites carry `Shadowed` and `Ambient` in the lit sum (`psp2::crowd`).
    let crowd = crowd::plan(
        &decoded,
        ps4,
        &mut glow.emissive,
        &mut *textures,
        &mut report,
    );
    // Anulpha Pass's chevrons add their emissive mask and wave after the light.
    let chevron = chevron::plan(
        &decoded,
        &mut glow.emissive,
        &mut glow.tracks,
        &mut *textures,
        &mut report,
    );
    // `fc07_blend_vertred_lambert_2diff` mixes a second picture in by the vertex colour's red.
    let mut pictures = Pictures::new();
    let blend = {
        let taken: Vec<bool> = (0..decoded.materials.len())
            .map(|m| {
                let m = Some(m);
                glow.role(m) != 0
                    || twin.of(m).is_some()
                    || emissive_add.of(m).is_some()
                    || normals.of(m).is_some()
                    || tarmac.of(m).is_some()
                    || chevron.of(m).is_some()
                    || (crowd.role(m) | glass.role(m) | sea.role(m)) != 0
            })
            .collect();
        vertex_blend::plan(
            &decoded,
            ps4,
            &taken,
            &mut glow.emissive,
            &mut pictures,
            &mut *textures,
            &mut report,
        )
    };
    // Weapon pads' bars take an entry per class, so each can be recoloured on its own.
    let pad_bars =
        circuit_pads::PadBars::plan(&decoded, &emissive_add, &mut glow.emissive, &mut report);
    model.emissive = glow.emissive.clone();
    model.anim_tracks = glow.tracks.clone();
    let uv_keys = billboard_vex.map_or_else(uv_keys::Plan::default, |vex_blob| {
        uv_keys::plan(&decoded, vex_blob, &mut model.anim_tracks, &mut report)
    });
    // Every draw is pushed in submesh order below, and `bind_textures`
    // zips on that; a hidden submesh still gets its draw, empty, so the
    // zip stays a binding.
    for submesh in &decoded.submeshes {
        let mut place = submesh
            .node
            .and_then(|n| plan.placements.get(n).copied())
            .unwrap_or(placement::Placement::STATIC);
        // The diagnostic filter (`OAG_SKIP_MATERIAL` and friends) hides a submesh as a hidden
        // node is, keeping its draw empty so `bind_textures` still zips.
        let filtered = {
            let material = submesh.material.and_then(|m| decoded.materials.get(m));
            let paths: Vec<&str> = material
                .into_iter()
                .flat_map(|m| {
                    std::iter::once(m.name.as_str()).chain(m.textures.iter().map(String::as_str))
                })
                .collect();
            super::isolate::excludes_psp2(submesh.material, None, &paths)
        };
        // Omega's track loader swaps every crowd-entity shape for rig instances and clears its
        // draw bits; the rigs are not loaded, so the placeholder's flat `Colour` is not drawn
        // either (`oag_rcs::rcsmodel::psp2::nodes::CROWD_ENTITY_FLAGS`).
        let crowd_entity = ps4
            && submesh
                .mesh
                .and_then(|m| decoded.scene.meshes.get(m))
                .is_some_and(|m| m.is_crowd_entity());
        let circuit_pad = hide_pads.hides(&decoded, submesh);
        place.hidden |= filtered || crowd_entity || circuit_pad;
        report.node_bound += usize::from(submesh.node.is_some());
        report.moving += usize::from(place.xform != 0);
        report.crowd_entities += usize::from(crowd_entity && !filtered);
        report.hidden += usize::from(place.hidden && !place.unplaced && !crowd_entity);
        // `fc01_dummy`'s fragment program samples nothing and declares one
        // uniform, `fogColour`; its three instructions leave the circuit's fog
        // colour as the pixel (docs/formats/2048-material-params.md, "What
        // `fc01_dummy` outputs"). The colour is the scene's, not the model's,
        // so these vertices carry [`FOG_FLAT`] and the loader that knows the
        // fog writes it in.
        let placeholder = submesh
            .material
            .and_then(|m| decoded.materials.get(m))
            .is_some_and(|m| m.is_placeholder());
        report.placeholder_submeshes += usize::from(placeholder && !place.hidden);
        report.unplaced += usize::from(place.unplaced);
        if let (Some(m), false) = (submesh.material, place.hidden) {
            super::census::shown(m);
        }
        // The second texture is this material's lightmap, where its submesh's
        // own declaration carries a `lightmapUV` - which is the same set of
        // submeshes, exactly (`oag_rcs`'s declaration ground truth).
        let role = if submesh.lightmap_texcoords.is_empty() {
            0
        } else {
            crate::mesh::slots::SECOND_IS_LIGHTMAP
        };
        // The diffuse map's own alpha is the surface's alpha, gloss and cut-out on Omega's programs
        // (`docs/formats/omega-status.md`, "The alpha lane"). Vita keeps the red lane for every
        // material but the alpha-lit ones below, whose programs are read to take `T.www`.
        let role = if ps4 {
            role | crate::mesh::slots::DEFAULT
        } else {
            role
        };
        let on_track = submesh
            .mesh
            .and_then(|m| decoded.scene.meshes.get(m))
            .is_some_and(|m| is_track_shape(&m.name));
        report.track_shape_submeshes += usize::from(on_track && !place.hidden);
        let role = if on_track {
            role | crate::mesh::slots::ZONE_TRACK
        } else {
            role
        };
        // A Zone race draws a pad on its own program, not the Zone surface, as HD's render
        // flags make it (`docs/ghidra/functions/ps3-hdfury-eu/zone-plain-chunks.md`, "2048 and
        // Omega": this container has no such flag, so the pad is the shape or material).
        let role = if circuit_pads::is_circuit_pad(&decoded, submesh) {
            role | crate::mesh::slots::ZONE_PLAIN
        } else {
            role
        };
        // A Zone material's surface is `Zone_Colour * (ambient + sun * max(N.L, 0) +
        // Emissive)` under fog (its unlightmapped fragment programs,
        // `docs/formats/2048-material-params.md`, "The Zone surface"), with the
        // stage's own `Colour <k>` and `Emissive` where the Zone table authors
        // them. The vertex keeps the material's authored colour as the fallback
        // and names its slot in `colour.a` ([`ZONE_COLOUR_ALPHA`]); the shader
        // does the rest.
        let zone_slot = submesh
            .material
            .and_then(|m| decoded.materials.get(m))
            .and_then(|m| m.zone_colour_slot());
        let zone = zone_slot.map(|(_, colour)| colour);
        report.zone_colour_submeshes += usize::from(zone.is_some());
        let emissive = (submesh.material.and_then(|m| decoded.materials.get(m)))
            .filter(|_| ps4)
            .and_then(flat_constant::emissive_colour);
        report.emissive_colour_submeshes += usize::from(emissive.is_some());
        let zone = zone.or(emissive);
        if let (Some(m), true) = (submesh.material, emissive.is_some()) {
            super::census::claim(m, "flat_emissive", super::census::Outcome::Drawn);
        }
        // HD's `nr_twinblend`: unlit, three coordinate pairs, one glow-table entry.
        let twin_here = twin.of(submesh.material);
        let (colour, lit, role) = match zone {
            None if twin_here.is_some() => (
                [1.0; 4],
                0.0,
                crate::mesh::slots::NO_AMBIENT
                    | crate::mesh::slots::NO_SUN
                    | twin_here.map_or(0, |t| t.slot << crate::mesh::slots::MATERIAL_SHIFT),
            ),
            None if placeholder => (
                FOG_FLAT,
                0.0,
                role | crate::mesh::slots::NO_AMBIENT | crate::mesh::slots::NO_SUN,
            ),
            Some([r, g, b]) => (
                [
                    r,
                    g,
                    b,
                    match zone_slot {
                        Some((k, _)) if !ps4 => ZONE_COLOUR_ALPHA + f32::from(k),
                        _ => 1.0,
                    },
                ],
                0.0,
                role | crate::mesh::slots::NO_AMBIENT | crate::mesh::slots::NO_SUN,
            ),
            None => ([1.0; 4], 1.0, role),
        };
        // Anulpha Pass's alpha-lit families, read on each title's own programs (`lit_alpha`).
        let alpha_lit = lit > 0.5
            && submesh
                .material
                .and_then(|m| decoded.materials.get(m))
                .is_some_and(lit_alpha::admits);
        if let (Some(m), true) = (submesh.material, alpha_lit) {
            super::census::claim(m, "lit_alpha", super::census::Outcome::Drawn);
        }
        let role = if alpha_lit {
            role | crate::mesh::slots::LIT_ALPHA | crate::mesh::slots::DEFAULT
        } else {
            role
        };
        // A reflective window gates its reflection by the diffuse alpha (`emissive_add`).
        let role = if emissive_add.alpha_gated(submesh.material) {
            role | crate::mesh::slots::DEFAULT
        } else {
            role
        };
        report.lit_alpha_submeshes += usize::from(alpha_lit && !place.hidden);
        // **A vertex-lit surface**: a submesh whose declaration names `NOVAColor` and no
        // lightmap. Its vertex program lays `scale * rgb + bias` on the baked colour and passes
        // `a` on as the sun's mask, and its fragment program adds the two to the sun and
        // multiplies the albedo - **no constant ambient** (docs/formats/2048-material-params.md,
        // "Vertex-lit and lightmapped combines"). The marker rides `sun_mask`, which
        // `shade.wesl` reads as `2 + mask`.
        let nova_vertex = !ps4
            && lit > 0.5
            && twin_here.is_none()
            && !submesh.colours.is_empty()
            && submesh.lightmap_texcoords.is_empty();
        let role = if nova_vertex {
            role | crate::mesh::slots::NO_AMBIENT
        } else {
            role
        };
        report.nova_vertex_submeshes += usize::from(nova_vertex && !place.hidden);
        let hull_factor = hull_colour.of(submesh.material)
            && lit > 0.5
            && twin_here.is_none()
            && !submesh.colours.is_empty()
            && submesh.lightmap_texcoords.is_empty()
            && tarmac.of(submesh.material).is_none()
            && !glass.is_ice(submesh.material);
        if hull_factor && !place.hidden {
            let name = submesh
                .material
                .and_then(|m| decoded.materials.get(m))
                .map_or("", |m| m.name.as_str());
            *report
                .hull_colour
                .submeshes
                .entry(name.to_string())
                .or_default() += 1;
        }
        let to_world = Mat4::from_cols_array(&place.to_world);
        let at_zero = Mat4::from_cols_array(&place.world_at_zero);
        // Normals through the bake's inverse transpose: 166 skeleton nodes
        // across the corpus scale non-uniformly (88 of them carry meshes),
        // and a normal turned by the plain matrix skews under one. A moving
        // node's normals go through the shader's node matrix instead, which
        // takes no inverse transpose - `mesh.wesl`'s own caveat, on lit
        // geometry here rather than Pulse's prelit.
        let normal_to_world = to_world.inverse().transpose();
        let base = u32::try_from(model.vertices.len()).unwrap_or(u32::MAX);
        let first = u32::try_from(model.indices.len()).unwrap_or(u32::MAX);
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for (i, position) in submesh.positions.iter().enumerate() {
            if place.hidden {
                break;
            }
            // Baked through the node's static matrix, or left in the node's
            // own space for the table to move; the bounds are world-space
            // either way, which for a moving node means time zero.
            let position = to_world
                .transform_point3(Vec3::from_array(*position))
                .to_array();
            let placed = if place.xform == 0 {
                position
            } else {
                at_zero
                    .transform_point3(Vec3::from_array(position))
                    .to_array()
            };
            for a in 0..3 {
                lo[a] = lo[a].min(placed[a]);
                hi[a] = hi[a].max(placed[a]);
            }
            // The file's own normal where this submesh's stride carries one
            // (`psp2::SubMesh::normals`) - a zero otherwise, which is
            // `super::face_normals`'s signal to derive one from the
            // triangles, exactly as it already does for an HD model. Turned
            // with the bake and renormalised.
            let normal = submesh.normals.get(i).copied().unwrap_or([0.0; 3]);
            let normal = if normal == [0.0; 3] {
                normal
            } else {
                normal_to_world
                    .transform_vector3(Vec3::from_array(normal))
                    .normalize_or_zero()
                    .to_array()
            };
            report.authored_normals += usize::from(normal != [0.0; 3]);
            // Counted for `Report::decoded_tangents`; drawn only where `normals` bound a map.
            report.decoded_tangents += usize::from(submesh.tangents.get(i).is_some());
            // The file's own diffuse coordinate where the submesh's
            // declaration names one (`psp2::SubMesh::texcoords`), the
            // vertex's origin otherwise - the same fallback shape `normal`
            // above uses, and equally honest: an untextured vertex painted
            // through `(0, 0)` is indistinguishable from one with no texture
            // bound at all, which is what `report.diffuse_texture` being
            // `None` already says.
            let texcoord = uv_flip.apply(
                submesh.material,
                submesh.texcoords.get(i).copied().unwrap_or([0.0, 0.0]),
            );
            // The atlas coordinate a lightmapped submesh's own declaration
            // names, and nothing where it names none - so a Vita model, which
            // never has one, is unchanged to the bit.
            let lightmap_texcoord = submesh
                .lightmap_texcoords
                .get(i)
                .copied()
                .unwrap_or([0.0, 0.0]);
            // The wipe's pair rides the lightmap coordinate and the bars' the second set, as
            // HD's `nr_twinblend` binds them.
            let pair = twin_here.and_then(|_| submesh.bars_blend_texcoords.get(i));
            let lightmap_texcoord = pair.map_or(lightmap_texcoord, |p| [p[2], p[3]]);
            let texcoord2 = pair.map_or([0.0, 0.0], |p| [p[0], p[1]]);
            // `2048_shipshield_old` reads both pairs of its packed `Uv1_Uv2`: the first is the
            // diffuse's, the second the Lightning layer's.
            let (texcoord, texcoord2) = match submesh.packed_texcoords.get(i) {
                Some(&[a, b, c, d]) if emissive_add.mask(submesh.material).is_some() => {
                    ([a, b], [c, d])
                }
                _ => (texcoord, texcoord2),
            };
            // The road's decal pair rides `texcoord2` and its dirt pair `colour.xy`
            // (`psp2::tarmac`); the fragment stage reads the vertex colour as white there.
            let (texcoord2, colour) = match (
                tarmac.of(submesh.material),
                submesh.markings_texcoords.get(i),
                submesh.dirt_texcoords.get(i),
            ) {
                (Some(_), Some(&markings), Some(&[u, v])) => (markings, [u, v, 1.0, 1.0]),
                _ => (texcoord2, colour),
            };
            // The blended water edge's alpha chain reads `VertexColour1.x` and `.y`
            // (`psp2::water`); they ride the second coordinate pair, which that material's
            // vertices use for nothing else.
            // Omega's edge reads the same attribute's `y` (`VertexColour1.y * Opacity`), which
            // the PS4 loader carries as the submesh's colours.
            let edge_colours = if ps4 {
                &submesh.colours
            } else {
                &submesh.vita_vertex_colour1
            };
            let texcoord2 = match edge_colours.get(i) {
                Some(&[x, y, ..]) if sea.edge(submesh.material) => {
                    [f32::from(x) / 255.0, f32::from(y) / 255.0]
                }
                _ => texcoord2,
            };
            // The blend family's weight, `VertexColour1`'s red, rides the same pair's `x`.
            let texcoord2 = blend
                .texcoord2(submesh.material, submesh.vita_vertex_colour1.get(i))
                .unwrap_or(texcoord2);
            let (colour, sun_mask) = match submesh.colours.get(i) {
                Some(&bytes) if hull_factor => hull_colour::vertex(bytes),
                Some(&[r, g, b, a]) if nova_vertex => (
                    [
                        f32::from(r) / 255.0,
                        f32::from(g) / 255.0,
                        f32::from(b) / 255.0,
                        1.0,
                    ],
                    2.0 + f32::from(a) / 255.0,
                ),
                _ => (colour, 1.0),
            };
            // Omega's snow: `NOVAColor` in the colour and `sun_mask`, `VertexColour2` and the drift
            // mask in the two coordinate pairs (`psp2::snow`).
            let snow_vertex = snow.vertex(
                submesh.material,
                (
                    submesh.ps4_nova_colour.get(i),
                    submesh.ps4_vertex_colour2.get(i),
                    submesh.colours.get(i),
                ),
            );
            let (colour, sun_mask, lightmap_texcoord, texcoord2) = match snow_vertex {
                Some(v) => (v.colour, v.sun_mask, v.lightmap_texcoord, v.texcoord2),
                None => (colour, sun_mask, lightmap_texcoord, texcoord2),
            };
            // The visualiser ring's vertices carry `Uv3`, `uv1`, `uv2` and `VertexColour1.w`.
            let ring = electricity::vertex(twin_here, &model.emissive, submesh, i)
                .or_else(|| cannon_bolt::vertex(twin_here, &model.emissive, submesh, i));
            let (texcoord, lightmap_texcoord, texcoord2, sun_mask) =
                ring.unwrap_or((texcoord, lightmap_texcoord, texcoord2, sun_mask));
            // `rt_icetunnel` weights its grab by the baked colour's red (`refraction::is_ice`).
            let colour = match submesh.colours.get(i) {
                Some(&[r, ..]) if glass.is_ice(submesh.material) => {
                    [1.0, 1.0, 1.0, f32::from(r) / 255.0]
                }
                _ => colour,
            };
            model.vertices.push(GpuVertex {
                quake: 0.0,
                position,
                normal,
                colour,
                texcoord,
                lit,
                lightmap_texcoord,
                anim: uv_keys
                    .anim(
                        submesh.mesh,
                        submesh.material.and_then(|m| decoded.materials.get(m)),
                    )
                    .unwrap_or_else(|| glow.anim(submesh.material)),
                slots: (if submesh.lightmap_texcoords.is_empty() && twin_here.is_none() {
                    role | glow.role(submesh.material)
                } else {
                    role
                }) | pad_bars
                    .role(&decoded, submesh)
                    .unwrap_or_else(|| emissive_add.role(submesh.material))
                    | tarmac.role(submesh.material)
                    | crowd.role(submesh.material)
                    | chevron.role(submesh.material)
                    | glass.role(submesh.material)
                    | sea.role(submesh.material)
                    | blend.role(submesh.material)
                    | snow.role(submesh.material),
                xform: place.xform,
                sun_mask,
                specular_exponent: crate::mesh::DEFAULT_SPECULAR_EXPONENT,
                glow: 0.0,
                texcoord2,
                tangent: normals.tangent(submesh.material, submesh.tangents.get(i), &to_world),
                depth_bias: 0.0,
            });
        }
        if !place.hidden {
            for &index in &submesh.indices {
                model.indices.push(base + u32::from(index));
            }
            // An airbrake is a node-bound mesh named `Airbrake_Left` or
            // `Airbrake_Right` whose vertices are about the flap's own hinge,
            // which is the node's matrix the bake above has just applied: the
            // same hinge-frame arrangement a Pulse flap has, so the same
            // `Flap` swings it. See `Flap::deflect`.
            let side = submesh
                .mesh
                .and_then(|m| decoded.scene.meshes.get(m))
                .and_then(|m| flap_side(&m.name));
            if let (Some(side), true) = (side, place.xform == 0) {
                let end = u32::try_from(model.vertices.len()).unwrap_or(u32::MAX);
                Flap::record(&mut model.airbrakes[side], base..end, to_world);
            }
        }
        let centre: [f32; 3] = if place.hidden {
            [0.0; 3]
        } else {
            std::array::from_fn(|a| (lo[a] + hi[a]) * 0.5)
        };
        let radius = if place.hidden {
            0.0
        } else {
            (0..3)
                .map(|a| (hi[a] - lo[a]) * 0.5)
                .fold(0.0f32, |acc, half| acc + half * half)
                .sqrt()
        };
        model.draws.push(DrawCall {
            writes_depth: false,
            range: first..u32::try_from(model.indices.len()).unwrap_or(u32::MAX),
            texture: None,
            bounds: Bounds { centre, radius },
            moving: place.xform != 0,
            culled: false,
            blend: None,
            blend_state: None,
            depth_point: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            node: None,
            // The mesh object this submesh belongs to: the join key for the
            // circuit's `.pvs`, whose bitmaps are indexed by exactly this
            // number (`oag_rcs::hd_pvs`, the `Psp2` dialect). `None` when the
            // node table did not read, which the PVS test always allows.
            chunk: submesh
                .mesh
                .and_then(|mesh| u32::try_from(mesh).ok())
                .or_else(|| rig_chunks.get(&submesh.record).copied()),
            // A Vita material authors no PSP-style reference.
            alpha_test_ref: None,
        });
        report.submeshes += 1;
        if !place.hidden {
            report.triangles += submesh.triangle_count();
        }
        report.non_finite_texcoords += submesh.non_finite_texcoords;
    }
    model.mesh_count = report.submeshes;
    // Fills in only the vertices left at a zero normal above - a submesh
    // whose stride was too small to carry one (not observed on any of the
    // 993 shipped files). See `super::face_normals`, whose docs make the
    // same fallback argument for the HD container.
    super::face_normals(&mut model);
    finish_bounds(&mut model);

    bind_textures(
        &decoded,
        (
            &glow,
            &twin,
            &normals,
            &emissive_add,
            &tarmac,
            &chevron,
            &blend,
            &snow,
        ),
        pictures,
        &mut model,
        &mut report,
        textures,
        ps4,
    );
    transparency::route(&decoded, &mut model, &mut report);
    Ok((model, report))
}

/// The readings whose bindings ride beside a draw's picture.
type Plans<'a> = (
    &'a glow::Plan,
    &'a twin_blend::Plan,
    &'a normal_map::Plan,
    &'a emissive_add::Plan,
    &'a tarmac::Plan,
    &'a chevron::Plan,
    &'a vertex_blend::Plan,
    &'a snow::Plan,
);

/// Binds each draw to its own submesh's material's texture.
///
/// **One decode per distinct material, not per draw.** Altima's circuit has
/// 2,800 submeshes over 527 materials and its materials share `.gxt` files
/// between them, so decoding per draw would decode the same texture hundreds
/// of times; the cache is keyed by material index and every miss is decoded
/// once. A material whose texture does not resolve caches the miss too, so a
/// missing archive entry costs one lookup rather than one per submesh.
fn bind_textures(
    decoded: &psp2::Model,
    (glow, twin, normals, emissive_add, tarmac, chevron, blend, snow): Plans<'_>,
    mut diffuses: Pictures,
    model: &mut Model,
    report: &mut Report,
    textures: Textures<'_>,
    ps4: bool,
) {
    // `build` pushes exactly one draw per submesh, in order, which is what
    // makes this zip a binding rather than an off-by-one waiting to happen -
    // and a misbound draw would paint a submesh with its neighbour's texture,
    // the one outcome `psp2::parse` already refuses to allow.
    debug_assert_eq!(model.draws.len(), decoded.submeshes.len());
    let mut cache: Vec<Option<Result<usize, Unbound>>> = vec![None; decoded.materials.len()];
    // A material file's programs say which of its samplers feed only the
    // lighting; read once per distinct file.
    let mut rules: std::collections::HashMap<&str, Option<LightingRule>> =
        std::collections::HashMap::new();
    // A lightmap is shared by however many materials name the same file, and a
    // 2,048-square atlas is 16 MiB decoded, so decode each path once.
    let mut atlases: std::collections::HashMap<&str, Option<std::sync::Arc<ModelTexture>>> =
        std::collections::HashMap::new();
    // `diffuses` decodes each `.gxt` once however many materials name it. Each
    // slot still gets its own entry, so a lightmap stays positionally beside
    // its diffuse.
    // One alpha-times-constant picture per (map, constant).
    type AlphaKey<'p> = (&'p str, [u32; 3]);
    let mut alpha_pictures: std::collections::HashMap<
        AlphaKey<'_>,
        Option<std::sync::Arc<ModelTexture>>,
    > = std::collections::HashMap::new();
    for (draw, submesh) in model.draws.iter_mut().zip(&decoded.submeshes) {
        let Some(index) = submesh.material else {
            continue;
        };
        let Some(slot) = cache.get_mut(index) else {
            continue;
        };
        let resolved = *slot.get_or_insert_with(|| {
            let layer = glow.layers.get(index).and_then(Option::as_ref);
            let twin_here = twin.of(Some(index));
            let material = &decoded.materials[index];
            let rule = lighting_rule(material, ps4, &mut rules, &mut *textures);
            let refused: Vec<u32> = rule.as_ref().map_or_else(Vec::new, |r| r.refused.clone());
            let path = match layer {
                _ if twin_here.is_some() => twin_here.map(|t| t.first.as_str()),
                _ if refraction::is_pictureless(material) => None,
                Some(layer) => Some(layer.diffuse.as_str()),
                None => emissive_add
                    .colour(Some(index))
                    .or_else(|| alpha_emissive::specular_picture(material).map(|s| s.picture))
                    .or_else(|| {
                        let mut not_pictures = refused.clone();
                        if super::skin::NITRO_FAMILIES
                            .contains(&lighting_only::family(material).as_str())
                        {
                            not_pictures.extend(super::skin::NITRO_LOOKUPS);
                        }
                        material.diffuse_texture_without(&not_pictures)
                    }),
            };
            let refused_here = layer.is_none()
                && material
                    .samplers
                    .iter()
                    .any(|(hash, _)| refused.contains(hash));
            report.lighting_only_materials += usize::from(refused_here && path.is_none());
            let alpha = (layer.is_none() && twin_here.is_none())
                .then(|| alpha_product::source(material))
                .flatten()
                .and_then(|(map, colour)| {
                    alpha_pictures
                        .entry((map, colour.map(f32::to_bits)))
                        .or_insert_with(|| {
                            let (texture, _) = decode_texture_form(map, &textures(map)?)?;
                            let picture = alpha_product::picture(&texture, colour)?;
                            Some(std::sync::Arc::new(
                                crate::mesh_render::offer_to_texture_sink(picture),
                            ))
                        })
                        .clone()
                });
            report.alpha_product_materials += usize::from(alpha.is_some());
            if alpha.is_some() {
                super::census::claim(index, "alpha_product", super::census::Outcome::Drawn);
            }
            let (texture, form) = match (alpha, path) {
                (Some(picture), _) => (picture, None),
                (None, Some(path)) => {
                    let entry = diffuses
                        .entry(path.to_string())
                        .or_insert_with(|| decode_picture(path, &mut *textures));
                    match entry.clone() {
                        Ok(found) => found,
                        Err(why) => {
                            log::trace!(
                                "psp2 material {index} ({}) unbound ({why:?}): {path}",
                                material.name
                            );
                            return Err(why);
                        }
                    }
                }
                (None, None) => {
                    let lighting_colour = refused_here
                        .then(|| constant_colour(material, rule.as_ref()?.colour?))
                        .flatten();
                    if let Some(colour) = lighting_colour {
                        report.constant_colour_materials += 1;
                        report.inherited_lighting_only +=
                            usize::from(rule.as_ref().is_some_and(|r| r.inherited));
                        (std::sync::Arc::new(colour), None)
                    } else if let Some(colour) =
                        ps4.then(|| flat_constant::diffuse(material)).flatten()
                    {
                        report.constant_diffuse_materials += 1;
                        (std::sync::Arc::new(colour), None)
                    } else {
                        let why = unbound_without_picture(material, refused_here);
                        log::trace!("psp2 material {index} unbound ({why:?}): {}", material.name);
                        return Err(why);
                    }
                }
            };
            let path = path.unwrap_or("constant colour");
            if let Some(form) = form {
                report.gnf.record(form);
            }
            let at = model.textures.len();
            model.textures.push(Some(texture));
            // The material's render-state word, positionally beside its
            // texture as `Model::material_state` is defined (bit 4 culls).
            debug_assert_eq!(model.material_state.len(), at);
            model
                .material_state
                .push(material.state.map_or(0, u32::from));
            // Positionally beside the diffuse, as `Model::lightmaps` is
            // defined; left empty for a model no material of which names one,
            // which is every Vita model.
            if twin.any() || normals.any() || emissive_add.any() || blend.any() {
                debug_assert_eq!(model.pad_masks.len(), at);
                model.pad_masks.push(
                    twin_here
                        .map(|t| t.wipe.clone())
                        .or_else(|| emissive_add.mask(Some(index)).cloned())
                        .or_else(|| normals.of(Some(index)).cloned())
                        .or_else(|| blend.second(Some(index)).cloned()),
                );
            }
            if tarmac.any() || chevron.any() || snow.any() {
                let layers = tarmac.of(Some(index));
                let pulse = chevron.of(Some(index));
                debug_assert_eq!(model.fifth_maps.len(), at);
                model.fifth_maps.push(
                    layers
                        .and_then(|l| l.fifth.clone())
                        .or_else(|| pulse.map(|p| p.emask.clone()))
                        .or_else(|| snow.large(Some(index))),
                );
                debug_assert_eq!(model.sixth_maps.len(), at);
                model.sixth_maps.push(
                    layers
                        .and_then(|l| l.sixth.clone())
                        .or_else(|| pulse.map(|p| p.wave.clone())),
                );
            }
            if twin.any() || tarmac.any() || snow.any() {
                debug_assert_eq!(model.wave_maps.len(), at);
                model.wave_maps.push(
                    twin_here
                        .and_then(|t| t.bars.clone())
                        .or_else(|| tarmac.of(Some(index)).and_then(|l| l.emissive.clone()))
                        .or_else(|| snow.small(Some(index))),
                );
            }
            if glow.has_layers()
                || twin.any()
                || decoded.materials.iter().any(|m| m.lightmap.is_some())
            {
                debug_assert_eq!(model.lightmaps.len(), at);
                let atlas = twin_here
                    .map(|t| t.second.clone())
                    .or_else(|| layer.map(|l| l.texture.clone()))
                    .or_else(|| {
                        decoded.materials[index]
                            .lightmap
                            .as_deref()
                            .and_then(|lmap| {
                                let entry = atlases.entry(lmap).or_insert_with(|| {
                                    let atlas = picture_memo::picture(lmap, textures, report);
                                    report.lightmaps += usize::from(atlas.is_some());
                                    report.lightmap_misses += usize::from(atlas.is_none());
                                    atlas
                                });
                                entry.clone()
                            })
                    });
                model.lightmaps.push(atlas);
            }
            if report.diffuse_texture.is_none() {
                report.diffuse_texture = Some(path.to_string());
            }
            report.textures += 1;
            Ok(at)
        });
        match resolved {
            Ok(at) => {
                draw.texture = Some(at);
                report.textured_draws += 1;
            }
            Err(why) => {
                report.unresolved_draws += 1;
                report.unbound.count(why);
            }
        }
    }
}

/// Why `bind_textures` left a material's draws without a picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unbound {
    /// A `fc01_dummy` placeholder: its program outputs the fog colour.
    Placeholder,
    /// A `Zone_ColourN` material: drawn unlit in that colour.
    ZoneColour,
    /// The material names no texture of its own (a lightmap aside): flat by design.
    NamesNoTexture,
    /// The material names a texture of its own, and none of them is a diffuse sampler.
    NoDiffusePath,
    /// Tunnel glass whose program takes the lit term untextured.
    PicturelessGlass,
    /// A lighting-only material whose unlit program names no colour value.
    NoConstantColour,
    /// The named `.gxt` is not in the archive set the loader reads.
    ArchiveMiss,
    /// The `.gxt` is there and its format would not decode.
    DecodeRefused,
}

/// Draws left without a picture, by the exit of `bind_textures` that left them;
/// together they are [`Report::unresolved_draws`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnboundDraws {
    /// On a `fc01_dummy` placeholder.
    pub placeholder: usize,
    /// On a `Zone_ColourN` material.
    pub zone_colour: usize,
    /// On a material that names no texture of its own.
    pub names_no_texture: usize,
    /// On a material that names a texture of its own, none of them a diffuse sampler.
    pub no_diffuse_path: usize,
    /// Tunnel glass drawn without a picture.
    pub pictureless_glass: usize,
    /// Lighting-only material with no constant colour.
    pub no_constant_colour: usize,
    /// Named `.gxt` absent from the archive set.
    pub archive_miss: usize,
    /// Named `.gxt` present but not decodable.
    pub decode_refused: usize,
}

impl UnboundDraws {
    /// Adds another model's counts.
    pub fn add(&mut self, other: &Self) {
        self.placeholder += other.placeholder;
        self.zone_colour += other.zone_colour;
        self.names_no_texture += other.names_no_texture;
        self.no_diffuse_path += other.no_diffuse_path;
        self.pictureless_glass += other.pictureless_glass;
        self.no_constant_colour += other.no_constant_colour;
        self.archive_miss += other.archive_miss;
        self.decode_refused += other.decode_refused;
    }

    /// The non-zero exits, as ` (n no diffuse path, ...)`; empty when there are none.
    #[must_use]
    pub fn describe(&self) -> String {
        let parts: Vec<String> = [
            (self.placeholder, "placeholder"),
            (self.zone_colour, "Zone colour"),
            (self.names_no_texture, "name no texture"),
            (self.no_diffuse_path, "name a texture, none diffuse"),
            (self.pictureless_glass, "pictureless glass"),
            (self.no_constant_colour, "lighting-only without a colour"),
            (self.archive_miss, "texture not in the archive set"),
            (self.decode_refused, "texture would not decode"),
        ]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, what)| format!("{n} {what}"))
        .collect();
        if parts.is_empty() {
            String::new()
        } else {
            format!(" ({})", parts.join(", "))
        }
    }

    fn count(&mut self, why: Unbound) {
        match why {
            Unbound::Placeholder => self.placeholder += 1,
            Unbound::ZoneColour => self.zone_colour += 1,
            Unbound::NamesNoTexture => self.names_no_texture += 1,
            Unbound::NoDiffusePath => self.no_diffuse_path += 1,
            Unbound::PicturelessGlass => self.pictureless_glass += 1,
            Unbound::NoConstantColour => self.no_constant_colour += 1,
            Unbound::ArchiveMiss => self.archive_miss += 1,
            Unbound::DecodeRefused => self.decode_refused += 1,
        }
    }
}

/// Which of the picture-less exits a material that bound no picture and no colour took.
fn unbound_without_picture(material: &psp2::material::Material, lighting_only: bool) -> Unbound {
    if lighting_only {
        Unbound::NoConstantColour
    } else if refraction::is_pictureless(material) {
        Unbound::PicturelessGlass
    } else if material.is_placeholder() {
        Unbound::Placeholder
    } else if material.zone_colour().is_some() {
        Unbound::ZoneColour
    } else if material
        .textures
        .iter()
        .chain(material.samplers.iter().map(|(_, path)| path))
        .all(|t| Some(t) == material.lightmap.as_ref())
    {
        Unbound::NamesNoTexture
    } else {
        Unbound::NoDiffusePath
    }
}

/// What decides a material's lighting-only samplers: their name hashes and the
/// colour parameter's, from its own programs or from 2048's answer.
#[derive(Debug, Clone)]
struct LightingRule {
    refused: Vec<u32>,
    colour: Option<u32>,
    inherited: bool,
}

/// The material's rule, read once per `.rcsmaterial` file. A Vita material
/// reads its own programs; a PS4 one, whose programs are unread, takes 2048's
/// answer where its instance record matches it.
fn lighting_rule<'m>(
    material: &'m psp2::material::Material,
    ps4: bool,
    cache: &mut std::collections::HashMap<&'m str, Option<LightingRule>>,
    textures: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) -> Option<LightingRule> {
    if ps4 {
        // Judged per instance: two materials may share a `.rcsmaterial` path
        // and differ in their record, and the answer is the record's.
        let row = lighting_only::inherited(material)?;
        return Some(LightingRule {
            refused: row.lighting_only.to_vec(),
            colour: Some(row.colour),
            inherited: true,
        });
    }
    cache
        .entry(material.name.as_str())
        .or_insert_with(|| {
            let uses = sampler_use::sampler_use(&textures(&material.name)?)?;
            // Only where the ambient-only program samples nothing: the case
            // `psp2_sampler_use_ground_truth` pins. A lit-only sampler beside
            // a picture the program does sample is left as it was.
            uses.colourless().then(|| LightingRule {
                refused: uses.lighting_only.iter().map(|n| name_hash(n)).collect(),
                colour: uses.colour_uniform.as_deref().map(name_hash),
                inherited: false,
            })
        })
        .clone()
}

/// The constant a lighting-only material takes its colour from: the one
/// uniform its ambient-only program declares besides the ambient and fog
/// colours, read from the material's own value, as a one-texel picture.
///
/// Unorm bytes, no sRGB step, the way `skin::constant_diffuse` binds HD's.
/// `None` where the program names no single colour or the material carries no
/// value for it, which leaves the draw unresolved and counted as such.
fn constant_colour(material: &psp2::material::Material, colour: u32) -> Option<ModelTexture> {
    let name = format!("{colour:08x}");
    let value = material.param(colour)?;
    let byte = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    let channel = |i: usize| byte(value.get(i).copied().unwrap_or(1.0));
    Some(crate::mesh_render::offer_to_texture_sink(
        ModelTexture::rgba8(
            format!("constant colour {name}"),
            1,
            1,
            vec![channel(0), channel(1), channel(2), 255],
            None,
        ),
    ))
}

/// The model's own centre and radius, from every vertex it ended up with.
fn finish_bounds(model: &mut Model) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for vertex in &model.vertices {
        for a in 0..3 {
            lo[a] = lo[a].min(vertex.position[a]);
            hi[a] = hi[a].max(vertex.position[a]);
        }
    }
    if model.vertices.is_empty() {
        return;
    }
    model.centre = std::array::from_fn(|a| (lo[a] + hi[a]) * 0.5);
    model.radius = model
        .vertices
        .iter()
        .map(|v| {
            (0..3)
                .map(|a| (v.position[a] - model.centre[a]).powi(2))
                .sum::<f32>()
        })
        .fold(0.0f32, f32::max)
        .sqrt();
}
