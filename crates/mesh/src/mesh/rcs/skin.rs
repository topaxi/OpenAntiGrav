//! What each material slot contributes: its textures, and the shader variant
//! the lit-race key resolves to.
//!
//! Split out of `mesh/rcs.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

mod lightmapped;

use std::sync::Arc;

use oag_rcs::{rcsmaterial, rcsmodel};

use super::super::{ModelTexture, TextureSlots};
use super::{Report, Textures, glass_sheen};

/// Decodes one material's texture, or says which way it could not be.
///
/// **A `.gtf` that will not decode draws nothing rather than something.**
/// `Texture::to_rgba` refuses the RSX's Morton-swizzled layouts and cubemaps -
/// 53 of the disc's 7,333 files - and a refusal here leaves the slot `None`,
/// which `mesh_render::build` binds its white 1x1 for. That is the same white
/// sheet this module has been removing, so it is counted in
/// [`Report::untextured`] rather than left to be discovered in a screenshot.
///
/// **A `.gtf` that is already in DXT blocks keeps them**, with the mip chain
/// the file authors, and [`blocks`] says exactly when. Decoding those to RGBA8
/// was costing 4.5x the memory for the same picture.
pub(super) fn decode_texture(label: &str, blob: &[u8]) -> Option<ModelTexture> {
    ModelTexture::from_gtf(label, blob).map(crate::mesh_render::offer_to_texture_sink)
}

/// One texture slot per material, in material-table order.
///
/// Positional and never compacted, because a chunk names its material by
/// ordinal - dropping the ones that fail to decode would re-skin the model.
///
/// Returns the first texture and the **second**, whatever its role. Until
/// 2026-08-20 the second was loaded and dropped unless it was the circuit's
/// lightmap, on the grounds that its role was unread; [`roles`] reads it now,
/// so the texture has to be there for the role to mean anything.
pub(super) fn skin(
    model: &rcsmodel::Model,
    picks: &[Pick],
    textures: Textures<'_>,
    report: &mut Report,
) -> (TextureSlots, TextureSlots) {
    let mut cache: std::collections::HashMap<String, Option<Arc<ModelTexture>>> =
        Default::default();
    let mut load = |path: &str, textures: Textures<'_>| {
        // A circuit's 442 materials name far fewer distinct textures, and
        // decoding a 2048x2048 DXT5 twice is the cost this avoids. The two
        // slots share the cache because a lightmap atlas is named by dozens of
        // materials at once.
        //
        // **What comes back is the same texture, not a copy of it.** Cloning
        // the decoded texels per slot is what made one circuit retain 1,858 MiB
        // of the 277 MiB it had actually decoded; `mesh_render::build` then
        // dedupes its uploads on this very `Arc`'s identity, so the sharing has
        // to start here for it to mean anything there.
        cache
            .entry(path.to_string())
            .or_insert_with(|| {
                textures(path)
                    .as_deref()
                    .and_then(|blob| decode_texture(path, blob))
                    .map(Arc::new)
            })
            .clone()
    };
    let mut skins = Vec::with_capacity(model.materials.len());
    let mut seconds = Vec::with_capacity(model.materials.len());
    for (slot, material) in model.materials.iter().enumerate() {
        let pick = picks.get(slot).copied().unwrap_or_default();
        let entry =
            |index: Option<usize>| -> Option<&str> { material.samplers.get(index?)?.1.as_deref() };
        if super::isolate::tinting() {
            skins.push(Some(Arc::new(ModelTexture::rgba8(
                format!("tint:{slot}"),
                1,
                1,
                super::isolate::tint(slot).to_vec(),
                None,
            ))));
            seconds.push(None);
            continue;
        }
        match entry(Some(pick.albedo)) {
            None => {
                report.untextured += 1;
                skins.push(None);
            }
            Some(path) => {
                let decoded = load(path, textures);
                if decoded.is_none() {
                    report.untextured += 1;
                }
                skins.push(decoded);
            }
        }
        // The lightmap is still counted apart from every other use of the
        // slot, because the two are answered by different evidence: the
        // lightmap by `Material::lightmap`'s four-signal path reading, and
        // everything else by the material's own microcode. See
        // `oag_rcs::rcsmodel::Material::lightmap` and [`roles`].
        let second = entry(pick.aux).map(|path| load(path, textures));
        match (material.lightmap_entry().is_some(), &second) {
            (true, Some(Some(_))) => report.lightmapped += 1,
            // Named and did not decode: counted apart, because a lightmap that
            // silently fails to load leaves the surface at full brightness,
            // which is what an unlit surface looks like anyway.
            (true, _) => report.lightmap_undecoded += 1,
            (false, Some(Some(_))) => report.second_texture_loaded += 1,
            (false, Some(None)) => report.second_texture_unread += 1,
            (false, None) => {}
        }
        seconds.push(second.flatten());
    }
    (skins, seconds)
}

/// Sampler name hashes whose texture is **not a picture** - a lookup a shader
/// addresses with something other than the surface's own coordinate.
///
/// # Why a list of hashes and not a guess about file names
///
/// Each is identified by what it binds across every HD circuit
/// (`crates/render/examples/hd_sampler_bind.rs`, `OAG_RAMPS=1`), and the
/// counts are the evidence:
///
/// | Hash | Uses | What it binds | Confidence |
/// | --- | ---: | --- | ---: |
/// | `0x35281c78` | 37 | `blue_metal_facing_ramp.gtf`, and nothing else | 88 |
/// | `0x994bbcf1` | 27 | `blue_metal_facing_ramp.gtf`, `tunnel_fx_facingramp.gtf` | 88 |
/// | `0x94b2b285` | 1 | `dc_iridescent_gradient.gtf` | 85 |
/// | `0x739a786e` | 20 | `ds_floor_n_rh`, `ds_pit_box_n`, `ds_wall_n` | 88 |
/// | `0x20c3e476` | 64 | `blue_metal_spec`, `tunnel_fx_spec`, `tunnel_fx_lights_spec` | 85 |
/// | `0x48f37f5a` (`NormalMap`) | 5 distinct (family, path) | `ds_solarwall_n`, `tracknormal`, `aadc_build_g_window_n`, `jd_chenghou_windowtrans_01_n` - only ever a `*_n*.gtf` (`hd_sampler_hash_binds.rs`, all 28 circuits and the ships) | 85 |
/// | `0xfe9bd1f3`, `0xeddf202a` | 2 each | `and_ice_norm`, `256norm4` and `cf_ice2_norm`, `and_snow_norm` - only the ice materials' normal maps | 85 |
/// | `0xb1f2a176` (`EmissiveTexture`) | 71 distinct path(s) | `*_emissive.gtf`, `*_e.gtf`, `advert_*.gtf`, `*glow*.gtf`, `dc_grad*.gtf` - never a plain diffuse | 90 |
///
/// Every use of the first three is a texture 32 texels or less in one
/// dimension - a ramp - and `0x94b2b285` is the one whose *coordinate* is
/// traced outright: `etched_glass_tech`'s block #7 samples it at
/// `dot(V, N)` (`DP3 R2.w, -R1, R2` then `TEX H2.xyz, -R2.wwww unit2`), the
/// view-facing scalar. The last two never bind anything but a `*_n*.gtf` and
/// a `*spec*.gtf` respectively. `EmissiveTexture` is named rather than
/// binding-inferred - it is one of the 38 preimages
/// `docs/formats/rcsmaterial.md`'s "The sampler names, by preimage" table
/// carries, grouped there under "Light" beside `lightmap` and
/// `shadowMapTex` - and the binding census agrees with the name: swept
/// disc-wide (`crates/render/examples/hd_emissive_texture_bind_census.rs`),
/// every one of its 71 distinct bound paths reads as a glow, an advert or a
/// gradient by name, never a plain diffuse.
///
/// # What this list is for, and what it is not
///
/// **It only stops a fallback, never drives one.** Where a material's colour
/// lane does not resolve, [`picks`] used to bind entry 0 whatever it was -
/// and on `etched_glass_tech` entry 0 is the iridescence ramp, so the glass
/// floor painted a hue ramp at the road's own UV and read as rainbow bands
/// welded to the surface. A ramp is not a picture; binding the entry that is
/// one is closer to the file than binding the first entry blindly.
///
/// **`EmissiveTexture` hit the same fallback from the opposite direction.**
/// Amphiseum's `track_wall.rcsmaterial` (and, disc-wide, 25 lightmapped
/// slots, all named `track_wall.rcsmaterial`, all on this one circuit's own
/// forward and reversed `.rcsmodel` -
/// `crates/render/examples/hd_emissive_first_census.rs`) name
/// `EmissiveTexture` at entry 0 and the real diffuse - `Texture1`, per the
/// same preimage table - at entry 1. [`picks`]'s lightmap branch does not
/// read the microcode at all,
/// so it took "first entry with a path" literally and bound the near-black
/// glow decal as the wall's entire picture, leaving a trackside panel
/// rendering solid black under a fully-resolved, correctly-lit variant - see
/// `docs/formats/rcsmaterial.md`, "A trackside wall panel drew solid black
/// because its picture was its glow decal". Excluding the hash here does not
/// change anything for a material where `EmissiveTexture` is the *only*
/// populated entry: `position` finds nothing past it, and `unwrap_or`
/// answers entry 0 exactly as before.
///
/// **Nothing here samples these textures in their own role yet.** The facing
/// ramp wants `dot(V, N)`, the normal map wants a tangent frame, the specular
/// map wants the exponent chain, `EmissiveTexture` wants the additive glow
/// term `mesh::rcs::emissive` already reads separately - none of which this
/// list wires, and `CLAUDE.md`'s rule is that a role with no recovered
/// shading stays unwired rather than guessed.
const NOT_A_PICTURE: &[u32] = &[
    rcsmaterial::LIGHTMAP_SAMPLER,
    0x3528_1c78,
    0x994b_bcf1,
    0x94b2_b285,
    0x739a_786e,
    0x20c3_e476,
    0x48f3_7f5a,
    0xfe9b_d1f3,
    0xeddf_202a,
    0xb1f2_a176,
];

/// Which of a material's sampler entries this renderer binds, out of however
/// many it has.
///
/// **The renderer used to bind entries 0 and 1 and call them "the texture" and
/// "the second texture".** A material has as many entries as it likes - 1,291
/// across Talon's Junction's 442 materials, up to seven on one - so those two
/// names were a position, not a role. This is the role: the entry whose
/// declared sampler the program's colour lane reaches, and the entry its alpha
/// lane reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Pick {
    /// The entry bound as the surface's picture.
    pub albedo: usize,
    /// The entry bound beside it - the coverage source, or the lightmap.
    pub aux: Option<usize>,
}

impl Default for Pick {
    /// Entries 0 and 1, which is what this renderer always bound and what a
    /// material with no resolved variant still gets.
    fn default() -> Self {
        Self {
            albedo: 0,
            aux: Some(1),
        }
    }
}

/// Which entry each material's two bindings come from.
///
/// **Conservative by construction, and the measurement says how much that
/// costs.** Of Talon's Junction's 302 drawn materials, the colour lane never
/// resolves past entry 0 and the alpha lane resolves past entry 1 on **nine**;
/// everything else either lands on an entry this renderer already bound or
/// does not resolve at all (`crates/render/examples/hd_role_census.rs`). So
/// this is plumbing rather than a new picture: it removes "first and second
/// slot" as a concept from the renderer, and the roles that would change a
/// frame - the specular map, the emissives, the facing ramp, all named and
/// supplied in the same table - need their shading read out of the microcode
/// before anything can bind them usefully.
///
/// **A lightmapped material keeps entry 1 regardless.** The lightmap is the
/// one role of that binding this project has identified, `mesh.wesl` reads it
/// as the prelit term and the sun mask, and `Material::lightmap`'s four-signal
/// path reading is what finds it - so an alpha trace never displaces it.
pub(super) fn picks(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
) -> Vec<Pick> {
    use rcsmaterial::fragment::{Program, Texel};

    let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
    model
        .materials
        .iter()
        .enumerate()
        .map(|(slot, material)| {
            let default = Pick::default();
            // **The lightmap wins the second binding, wherever it sits.** It
            // is the one role of that binding this project has identified,
            // `mesh.wesl` reads it as the prelit term and the sun mask, and
            // the file names it outright - see `Material::lightmap_entry`.
            // Binding it only when it happened to be entry 1 left a third of
            // Talon's Junction's baked lighting unread and its track walls
            // rendering near black.
            if material.lightmap_entry().is_some() {
                let at = material
                    .samplers
                    .iter()
                    .position(|(hash, path)| {
                        *hash == rcsmaterial::LIGHTMAP_SAMPLER && path.is_some()
                    })
                    .unwrap_or(1);
                return Pick {
                    albedo: lightmapped::albedo(
                        material,
                        variants.get(slot).copied().flatten(),
                        |name| {
                            cache
                                .entry(name.to_string())
                                .or_insert_with(|| textures(&format!("/{name}")))
                                .clone()
                        },
                    )
                    .unwrap_or(default.albedo),
                    aux: Some(at),
                };
            }
            let Some(variant) = variants.get(slot).copied().flatten() else {
                return default;
            };
            let Some(blob) = cache
                .entry(material.name.clone())
                .or_insert_with(|| textures(&format!("/{}", material.name)))
                .clone()
            else {
                return default;
            };
            let (Some(declared), Some(program)) = (
                rcsmaterial::Declared::parse(&blob, variant.fragment.offset),
                Program::parse(&blob, variant.fragment.offset),
            ) else {
                return default;
            };
            // **The glass family's facing-ramp combine wins both bindings,
            // ahead of the generic picture/coverage reading below.** Its
            // colour lane is `Texel::Mixed` - more than one unit reaches it -
            // so the generic reading below falls to `NOT_A_PICTURE`'s
            // albedo fallback and its alpha lane resolves to the same entry,
            // which collapses `aux` onto `albedo` and leaves the material's
            // own ramp (`Material::texture`) bound nowhere. See
            // `glass_sheen`'s own doc for the full read and the routing
            // question it settles.
            if let Some(sheen) = glass_sheen::classify(material, &declared, &program) {
                return Pick {
                    albedo: sheen.ramp_entry,
                    aux: Some(sheen.grid_entry),
                };
            }
            // An entry only counts when it supplies a `.gtf`: an entry the
            // material declares and leaves empty is the engine's to bind, and
            // pointing a lane at it would sample `mesh_render::build`'s
            // placeholder.
            let index_of = |unit: u32| -> Option<usize> {
                material.samplers.iter().position(|(hash, path)| {
                    path.is_some()
                        && declared
                            .samplers
                            .iter()
                            .any(|&(h, u)| h == *hash && u == unit)
                })
            };
            // The first entry that supplies a `.gtf` and is not a lookup - see
            // [`NOT_A_PICTURE`]. Entry 0 where no entry qualifies, which is
            // what this bound before any of it was read.
            let picture = material
                .samplers
                .iter()
                .position(|(hash, path)| path.is_some() && !NOT_A_PICTURE.contains(hash))
                .unwrap_or(default.albedo);
            let texels = program.output_texels();
            let colour = texels[0].merge(texels[1]).merge(texels[2]);
            let albedo = colour
                .unit()
                .and_then(|u| index_of(u32::from(u)))
                .unwrap_or(picture);
            let aux = match texels[3] {
                Texel::Unit { unit, .. } => index_of(u32::from(unit)),
                _ => None,
            }
            .filter(|&i| i != albedo)
            .or(default.aux);
            Pick { albedo, aux }
        })
        .collect()
}

/// What each material slot's own microcode says its two texture units are for,
/// packed as [`crate::mesh::slots`], in material-table order.
///
/// **The reading this project spent a year without.** A `.rcsmaterial` is a
/// table of shader variants; [`variants`] already resolves which one an
/// ordinary lit race draws through, and that variant's fragment program states
/// outright which unit it samples for the picture and which unit and channel it
/// writes to the output alpha.
/// `oag_rcs::rcsmaterial::fragment::Program::output_texels` traces it, and
/// this turns the answer into the four bits `mesh.wesl` decodes.
///
/// **Only a positive reading is acted on.** A lane the taint could not follow
/// answers [`rcsmaterial::fragment::Texel::Untraced`] - a constant, an
/// interpolator, or an opcode this decoder does not model - and `Untraced` is
/// not the same claim as "opaque". Those slots keep
/// [`crate::mesh::slots::DEFAULT`], which is exactly what this renderer did
/// before any of this existed, so the reading can only add correct surfaces
/// and never take a working one away. The same rule covers a unit above 1: a
/// `.rcsmodel` material names two textures and some programs sample four, so
/// an alpha traced to unit 2 is recorded by the census and not acted on here.
/// Which material slots write `1 - v` into the varying their fragment
/// program samples with, read out of each resolved **vertex** block.
///
/// **Read per material, because the disc does not agree with itself.** One of
/// Talon's Junction's 283 resolved variants flips - `track_wall` - and its 22
/// chunks are exactly the surfaces that addressed a blank third of their own
/// atlas and drew as flat black bands. Flipping every coordinate instead would
/// trade those for every surface that is already right; see
/// `oag_rcs::rcsmaterial::vertex`.
///
/// The attribute asked about is the one the chunk's own declaration calls the
/// diffuse coordinate, so a layout that names it something other than `Uv1`
/// still answers.
pub(super) fn flips(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
) -> Vec<bool> {
    let mut decl_of: std::collections::HashMap<u32, Option<&rcsmodel::VertexDecl>> =
        Default::default();
    // **Every surface, not just each chunk's first.** `Mesh::surfaces` is the
    // chunk plus its `extra_surfaces` - a second material painted over the
    // same geometry, the shape Talon's Junction's magstrip floor uses
    // (`mag_effect_loop_opaque` under `mageffectloop`'s blended glow). A
    // material that is only ever an *extra* surface never appears as any
    // chunk's own `.material`, so counting `model.meshes` alone reads it as
    // zero chunks and [`variants`] below skips resolving it entirely - see
    // that function's own "declared and never drawn" branch, which this
    // walk is what keeps honest.
    for mesh in model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces) {
        decl_of.entry(mesh.material).or_insert(mesh.decl.as_ref());
    }
    let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
    let mut out = Vec::with_capacity(model.materials.len());
    for (slot, material) in model.materials.iter().enumerate() {
        let ordinal = u32::try_from(slot).unwrap_or(u32::MAX);
        let hash = decl_of
            .get(&ordinal)
            .copied()
            .flatten()
            .and_then(rcsmodel::VertexDecl::diffuse_texcoord)
            .map(|a| a.name_hash);
        let flipped = variants
            .get(slot)
            .copied()
            .flatten()
            .zip(hash)
            .and_then(|(variant, hash)| {
                let blob = cache
                    .entry(material.name.clone())
                    .or_insert_with(|| textures(&format!("/{}", material.name)))
                    .clone()?;
                let program = rcsmaterial::vertex::Program::of(&blob, variant.vertex)?;
                Some(program.flips(hash))
            })
            .unwrap_or(false);
        out.push(flipped);
    }
    out
}

pub(super) fn roles(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    picks: &[Pick],
    seconds: &[Option<Arc<ModelTexture>>],
    textures: Textures<'_>,
    report: &mut Report,
) -> Roles {
    use crate::mesh::slots;
    use rcsmaterial::fragment::{Program, Texel};

    let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
    let mut out = Vec::with_capacity(model.materials.len());
    let mut specular_exponent = Vec::with_capacity(model.materials.len());
    for (slot, material) in model.materials.iter().enumerate() {
        let mut packed = slots::DEFAULT;
        if material.lightmap_entry().is_some() {
            packed |= slots::SECOND_IS_LIGHTMAP;
        }
        let variant = variants.get(slot).copied().flatten();
        let blob = variant.and_then(|_| {
            cache
                .entry(material.name.clone())
                .or_insert_with(|| textures(&format!("/{}", material.name)))
                .clone()
        });
        let program = variant
            .zip(blob.as_deref())
            .and_then(|(variant, blob)| Program::parse(blob, variant.fragment.offset));
        // **A resolved `0.0` is checked against the model's own patch table
        // before it is discarded.** `Program::specular_exponent`'s own doc
        // comment carries the disc-wide evidence that `0.0` is the strongest
        // candidate for `SpecularPower` patched at draw time rather than a
        // real value baked in the file - `pow(x, 0) = 1` is not a plausible
        // authored shininess. That is now checked rather than assumed:
        // `Program::patches` says whether the exponent's own code slot is one
        // `SpecularPower` overwrites, and where it is, the material's own
        // parameter table (the same table `Flame::from_material` reads) has
        // the value the engine actually puts there. Verified disc-wide before
        // being wired - `crates/render/examples/hd_specular_patch_census.rs` -
        // every one of 62 materials across 16 circuits whose `0.0` chain is
        // patched this way also authors a non-zero `SpecularPower`, at values
        // (30 to 100, non-round) the shared-literal population never carries.
        // A `0.0` that is *not* patched, or a material with no authored value
        // for it, still falls back to `DEFAULT_SPECULAR_EXPONENT` - every
        // other resolved value is wired exactly as read.
        let resolved = program.as_ref().and_then(|program| {
            let value = program.specular_exponent()?;
            if value != 0.0 {
                return Some(value);
            }
            let slot = program.specular_exponent_slot()?;
            program
                .patches(rcsmaterial::SPECULAR_POWER)
                .any(|patched| patched == slot)
                .then(|| {
                    material
                        .parameters
                        .iter()
                        .find(|p| p.hash == rcsmaterial::SPECULAR_POWER)
                        .map(|p| p.value[0])
                })
                .flatten()
                .filter(|value| *value != 0.0)
        });
        if resolved.is_none() {
            report.specular_exponent_unresolved += 1;
        }
        specular_exponent.push(resolved.unwrap_or(crate::mesh::DEFAULT_SPECULAR_EXPONENT));
        let declared = variant.zip(blob.as_deref()).and_then(|(variant, blob)| {
            rcsmaterial::Declared::parse(blob, variant.fragment.offset)
        });
        // **The constant ambient is a declared input, not a global.** A
        // program that is not fed `constantAmbientColour` must not receive it;
        // see `slots::NO_AMBIENT` for what applying it to everything cost.
        if declared
            .as_ref()
            .is_some_and(|d| !d.takes_constant_ambient())
        {
            packed |= slots::NO_AMBIENT;
        }
        // And the sun is one too. The pair is the three-way lighting key, and
        // a program fed neither is emissive - see `slots::NO_SUN`.
        if declared
            .as_ref()
            .is_some_and(|d| !d.takes_directional_light())
        {
            packed |= slots::NO_SUN;
        }
        // **The glass family's facing-ramp combine, off the same fact
        // `picks` already routed by.** See `glass_sheen`'s own doc for the
        // shape this catches. `units` below still runs unconditionally: with
        // `picks` already pointing `albedo`/`aux` at the ramp and the grid,
        // its ordinary unit lookup answers correctly on its own, which is
        // how the alpha-from-second and channel bits below fall out for
        // this material without a second special case.
        if let Some((d, p)) = declared.as_ref().zip(program.as_ref())
            && glass_sheen::classify(material, d, p).is_some()
        {
            packed |= slots::FACING_RAMP_SHEEN;
        }
        // **The two rim-shaded weapon glows**, off a fingerprint of the
        // resolved program itself - see `rim_glow`.
        if let Some((d, p)) = declared.as_ref().zip(program.as_ref()) {
            let alpha = material
                .parameters
                .iter()
                .find(|p| p.hash == super::rim_glow::RIM_EDGE_ALPHA)
                .map(|p| p.value[0]);
            packed |= super::rim_glow::classify(d, p, alpha);
        }
        let (first_unit, second_unit) = units(
            declared.as_ref(),
            material,
            picks.get(slot).copied().unwrap_or_default(),
        );
        // **Only where there is a second texture to point at.** A program's
        // unit 1 and a `.rcsmodel` material's second slot are not the same
        // thing: 40 of the disc's materials sample two units while naming one
        // texture, and pointing the albedo at a unit nothing is bound to
        // samples `mesh_render::build`'s black placeholder - which is a
        // black surface, the loudest possible way to be wrong. Caught by the
        // picture on the first run of this reading.
        let bound = seconds.get(slot).is_some_and(Option::is_some);
        if let (Some(program), true) = (program, bound) {
            let texels = program.output_texels();
            let colour = texels[0].merge(texels[1]).merge(texels[2]);
            // **Never the lightmap.** When the second slot is the circuit's
            // baked atlas, its alpha is the sun-occlusion mask - `mesh.wesl`
            // reads it as exactly that, `mask = baked.a * in.sun_mask` - and
            // its RGB is a light term, not a picture. Pointing coverage or
            // albedo at it paints a shadow map as a stencil. Measured on the
            // frame: without this guard nine of Talon's Junction's
            // lightmapped slots took `ALPHA_FROM_SECOND`, and the circuit's
            // perforated trackside barrier beside the blimp lost its holes
            // and washed out at tick 295.
            let lightmapped = packed & slots::SECOND_IS_LIGHTMAP != 0;
            let is_second = |unit: u32| !lightmapped && unit == second_unit;
            if colour.unit().map(u32::from).is_some_and(is_second) {
                packed |= slots::ALBEDO_FROM_SECOND;
            }
            if let Texel::Unit {
                unit,
                channel: Some(channel),
            } = texels[3]
                && (u32::from(unit) == first_unit || is_second(u32::from(unit)))
            {
                packed &= !slots::alpha_channel(3);
                packed |= slots::alpha_channel(u32::from(channel));
                if is_second(u32::from(unit)) {
                    packed |= slots::ALPHA_FROM_SECOND;
                }
            }
        }
        out.push(packed);
    }
    Roles {
        packed: out,
        specular_exponent,
    }
}

/// What [`roles`] resolves per material slot, in material-table order:
/// [`roles`]'s own bit-packed word, and the specular exponent read off the
/// same resolved fragment program - kept together because both come out of
/// one pass over one parsed [`rcsmaterial::fragment::Program`] per material,
/// and a second pass to re-decode it for the exponent alone would parse
/// every material's microcode twice for no reason.
pub(super) struct Roles {
    pub(super) packed: Vec<u32>,
    pub(super) specular_exponent: Vec<f32>,
}

/// Which texture unit each of a material's two `.gtf` slots reaches, read from
/// the **sampler name hash** the slot carries beside its path
/// (`rcsmodel::Material::texture_sampler`) against the resolved variant's own
/// sampler declaration.
///
/// # Why this is not the slot's ordinal
///
/// **Confidence 80**, and the number that earns it: of the disc's 13,933
/// populated texture slots, 8,021 carry a hash the material's own lit-race
/// variant declares as a sampler - and **only 776 of those land on the slot's
/// own ordinal**. The first slot reaches unit 1 on 4,751 of them. Reading the
/// hash is therefore closer to the file than counting slots is, on nine slots
/// out of ten. `docs/formats/rcsmaterial.md`, "A texture slot names its
/// sampler, and the slot's ordinal is not its unit", carries the sweep and the
/// three preimages (`Texture1`, `diffuse`, `lightmap`) that identify the field.
///
/// # Why the caller accepts this *and* the ordinal
///
/// **The hash adds identifications; it never removes them**, and that is a
/// measurement rather than caution. Two things break if it replaces the
/// ordinal outright. Talon's Junction's cloud plate is the first: its second
/// slot's hash is `lightmap`, its resolved variant declares no `lightmap` at
/// all, and its microcode plainly samples `cloud mask.gtf` at unit 1 - the
/// reading `docs/formats/rcsmaterial.md` traced end to end and this renderer
/// already draws correctly. The second showed up in a frame: replacing the
/// ordinal moved 27 of Talon's Junction's 442 slots off
/// `ALPHA_FROM_SECOND`, and the circuit's perforated trackside barrier - the
/// one beside the blimp at tick 295 - lost its holes and washed out. That
/// reading is retired: the pairing it rested on was off by one entry, and with
/// the entries paired correctly the hash answers on its own.
///
/// Where the variant declares no such sampler at all, this answers the
/// ordinal, which is what the caller would have used anyway.
fn units(
    declared: Option<&rcsmaterial::Declared>,
    material: &rcsmodel::Material,
    pick: Pick,
) -> (u32, u32) {
    // **The entries this renderer actually bound**, which [`picks`] chose and
    // which are not necessarily 0 and 1 - asking the hashes of entries 0 and 1
    // here would answer for textures no draw samples.
    let unit_of = |index: Option<usize>| {
        let (declared, (hash, path)) = (declared?, material.samplers.get(index?)?);
        path.as_ref()?;
        declared
            .samplers
            .iter()
            .find(|(h, _)| h == hash)
            .map(|&(_, unit)| unit)
    };
    let first = unit_of(Some(pick.albedo));
    let second = unit_of(pick.aux);
    // A material whose two slots resolve to the same unit is a reading that
    // cannot be right, and taking it would paint the first texture through the
    // second's role. The ordinal answers for the second slot there.
    match (first.unwrap_or(0), second) {
        (f, Some(s)) if s != f => (f, s),
        (f, _) => (f, u32::from(f != 1)),
    }
}

/// Which shader variant each material slot resolves to, in material-table
/// order.
///
/// **The first use of the variant key this project can read.** A material is a
/// table of up to 68 shader variants and the original picks one by a two-part
/// key - see `oag_rcs::rcsmaterial`. Half of it a chunk decides, from its
/// own vertex declaration; half a render pass decides, and for an ordinary lit
/// race that half is [`rcsmaterial::LIT_RACE_PASS`].
///
/// Resolving it **per material slot** rather than per draw is sound because a
/// slot never has to be two things at once: measured over all 123 `.rcsmodel`
/// on the disc and the 3,566 slots they use, zero serve chunks whose
/// chunk-determined keys differ (`a_material_slot_never_needs_two_different_variants`).
/// The first chunk naming a slot therefore answers for all of them.
///
/// Nothing shades differently for this yet. It is carried so the shader step is
/// a shader step, and counted so the loader report says how much of a circuit
/// the reading actually reaches.
pub(super) fn variants(
    model: &rcsmodel::Model,
    textures: Textures<'_>,
    report: &mut Report,
) -> Vec<Option<rcsmaterial::Variant>> {
    // One representative chunk per slot, which the invariant above licenses,
    // and how many chunks each slot serves - because the slot is not the
    // meaningful unit. Talon's Junction declares 442 materials and only 302 are
    // named by any chunk, so counting over all of them dilutes the answer with
    // slots nothing draws.
    let mut decl_of: std::collections::HashMap<u32, Option<&rcsmodel::VertexDecl>> =
        Default::default();
    let mut chunks_of: std::collections::HashMap<u32, usize> = Default::default();
    // **Every surface of every chunk, not each chunk's first alone.** See
    // `flips`'s identical walk for why: a material painted only as a second
    // (or later) surface over another chunk's geometry - `Mesh::surfaces`,
    // the shape Talon's Junction's magstrip floor uses - never appears as any
    // chunk's own `.material`, so it would otherwise read as zero chunks
    // below and fall into "declared and never drawn" without this ever
    // resolving a variant for it, even though `super::build`'s own emit loop
    // draws it through exactly this same `Mesh::surfaces` walk.
    for mesh in model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces) {
        decl_of.entry(mesh.material).or_insert(mesh.decl.as_ref());
        *chunks_of.entry(mesh.material).or_default() += 1;
    }

    let mut cache: std::collections::HashMap<String, Option<rcsmaterial::RcsMaterial>> =
        Default::default();
    let mut out = Vec::with_capacity(model.materials.len());
    for (slot, material) in model.materials.iter().enumerate() {
        let ordinal = u32::try_from(slot).unwrap_or(u32::MAX);
        let chunks = chunks_of.get(&ordinal).copied().unwrap_or(0);
        if chunks == 0 {
            // Declared and never drawn: not a gap in this reading, so it is not
            // counted as one.
            out.push(None);
            continue;
        }
        // The material's own name is its path in the archive.
        let parsed = cache
            .entry(material.name.clone())
            .or_insert_with(|| {
                textures(&format!("/{}", material.name))
                    .as_deref()
                    .and_then(|blob| rcsmaterial::RcsMaterial::parse(blob).ok())
            })
            .clone();
        let Some(parsed) = parsed else {
            report.materials_unread += 1;
            out.push(None);
            continue;
        };
        let decl = decl_of.get(&ordinal).copied().flatten();
        // Numerically, not by union: bits 1-2 are a four-way field and
        // `Features::with` would leave both `Ambient` and `IleLightmap` set,
        // naming a permutation nothing ships. See `Features::chunk_word`.
        let word = rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, decl);
        let key = rcsmaterial::Features::from_pass_word(word);
        // `Static` first, since it is what every world chunk uses and what
        // most materials ship a row for. Falling through the rest of
        // `Class::ALL` - `StaticQuake` (`Static`'s own programs with the
        // Quake weapon's displacement in front, nothing here fires that
        // weapon), `RigidBody` (a moving body's own class, ships and
        // weapons) and `StaticUncompressed` - covers a material whose table
        // ships only one of those, such as
        // `data/materials/frontendscene/basic_vertexemissive.rcsmaterial`
        // (`RigidBody` only) or a ship hull material for which `Static`'s
        // key happens to miss.
        //
        // **This is a fallback across classes, not a search over the key**:
        // `(class, features)` is still looked up exactly, one class at a
        // time, and never relaxed. It is sound because the **fragment**
        // program - the only half this renderer reads - is class-independent
        // wherever both exist: `crates/render/examples/
        // hd_class_fragment_share_check.rs` checked every `.rcsmaterial` on
        // the disc for a same-feature-hash pair across classes and found
        // **zero** of 36,257 with a different fragment block, only the
        // vertex program (which encodes the vertex-class-specific position
        // transform this project's own generic decoder does not run) ever
        // differs. So whichever class's row happens to exist answers the
        // same shading.
        //
        // **Measured before wiring**: this fallback moves the disc's `Static`
        // misses by exactly one material
        // (`frontendscene_hd_atg.vex`'s `basic_vertexemissive.rcsmaterial`,
        // `RigidBody`-only) - not the ship hull's 78, which
        // `crates/rcs/src/rcsmodel/vertex_decl.rs`'s `vertex_colour()` fix
        // resolves instead (see `hd_ship_class_census.rs`'s own doc comment
        // for the disc-wide count that settled which bug this actually was).
        let variant = rcsmaterial::Class::ALL
            .into_iter()
            .find_map(|class| parsed.variant(class, key));
        match variant {
            Some(v) => {
                report.variants_resolved += 1;
                report.variant_chunks += chunks;
                out.push(Some(*v));
            }
            None => {
                report.variants_unshipped += 1;
                report.variant_chunks_missed += chunks;
                out.push(None);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    // Only the tests still name these: the block/RGBA choice they assert moved
    // to `ModelTexture::from_gtf`, which this module now delegates to.
    use super::super::super::{BlockFormat, Texels};

    fn declared(samplers: &[(u32, u32)]) -> rcsmaterial::Declared {
        rcsmaterial::Declared {
            parameters: Vec::new(),
            samplers: samplers.to_vec(),
            parameter_patches: Vec::new(),
        }
    }

    fn material(first: u32, second: Option<u32>) -> rcsmodel::Material {
        let samplers = std::iter::once((first, Some("a.gtf".to_string())))
            .chain(second.map(|h| (h, Some("b.gtf".to_string()))))
            .collect();
        rcsmodel::Material {
            name: "m.rcsmaterial".to_string(),
            state: 1,
            src_factor: 0,
            dst_factor: 0,
            alpha_func: 0,
            alpha_ref: 0.0,
            texture: "a.gtf".to_string(),
            second_texture: second.map(|_| "b.gtf".to_string()),
            texture_sampler: first,
            second_texture_sampler: second,
            samplers,
            parameters: Vec::new(),
            curve: None,
        }
    }

    /// A minimal `.gtf`: one `DXT1` texture, `width` x `height`, with a chain
    /// of `mip_levels` if the caller asks for one.
    ///
    /// Twelve-byte header, one 36-byte descriptor, then the blocks. Every field
    /// the parser checks is filled from the arguments, so a wrong one fails
    /// `Gtf::parse` here rather than being silently ignored.
    fn dxt1(width: u16, height: u16, mip_levels: u8) -> Vec<u8> {
        let mut blob = vec![0u8; 48];
        blob[0..4].copy_from_slice(&0x0105_0000u32.to_be_bytes());
        blob[8..12].copy_from_slice(&1u32.to_be_bytes());
        let length: usize = (0..mip_levels)
            .map(|level| {
                let w = (u32::from(width) >> level).max(1).div_ceil(4) as usize;
                let h = (u32::from(height) >> level).max(1).div_ceil(4) as usize;
                w * h * 8
            })
            .sum();
        blob[16..20].copy_from_slice(&48u32.to_be_bytes());
        blob[20..24].copy_from_slice(&(length as u32).to_be_bytes());
        blob[24] = 0x06;
        blob[25] = mip_levels;
        blob[26] = 2;
        blob[32..34].copy_from_slice(&width.to_be_bytes());
        blob[34..36].copy_from_slice(&height.to_be_bytes());
        blob[36..38].copy_from_slice(&1u16.to_be_bytes());
        blob.resize(48 + length, 0);
        blob
    }

    /// A texture with a chain keeps the disc's blocks and the disc's levels.
    ///
    /// The 4.5x that makes an HD race fit in half a gigabyte, and the levels
    /// are the ones the original minified with rather than a box filter of the
    /// base - see `docs/formats/gtf.md`.
    #[test]
    fn a_chained_dxt1_keeps_its_blocks_and_its_own_mip_levels() {
        let decoded = decode_texture("a.gtf", &dxt1(8, 8, 4)).expect("decodes");
        let Texels::Blocks { format, levels } = &decoded.texels else {
            panic!("a chained DXT1 should keep its blocks");
        };
        assert_eq!(*format, BlockFormat::Bc1);
        assert_eq!(
            levels.iter().map(Vec::len).collect::<Vec<_>>(),
            [32, 8, 8, 8],
            "8x8, 4x4, 2x2 and 1x1, each a whole number of 8-byte blocks"
        );
    }

    /// With no chain in the file there is nothing to bind, so it decodes and
    /// the renderer box-filters its own - 649 of the disc's 7,131.
    #[test]
    fn a_single_level_dxt1_decodes_instead() {
        let decoded = decode_texture("a.gtf", &dxt1(8, 8, 1)).expect("decodes");
        assert!(
            matches!(decoded.texels, Texels::Rgba8(ref rgba) if rgba.len() == 8 * 8 * 4),
            "a single-level file takes the RGBA path"
        );
    }

    /// **One decoded texture, however many slots name it.**
    ///
    /// The slots are positional and a circuit's lightmap atlas is named by 275
    /// of Talon's Junction's 442 at once, so a copy per slot retained 1,858 MiB
    /// where 277 MiB had been decoded - and `mesh_render::build` keys its
    /// uploads on this very identity, so losing the sharing here costs it there
    /// too. Nothing about the pictures would look wrong, which is why it needs
    /// a test.
    #[test]
    fn every_slot_naming_one_texture_shares_it_rather_than_copying() {
        let blob = dxt1(8, 8, 4);
        let model = rcsmodel::Model {
            meshes: Vec::new(),
            materials: (0..8).map(|_| material(0, None)).collect(),
        };
        let picks = vec![Pick::default(); model.materials.len()];
        let mut loads = 0;
        let mut load = |path: &str| {
            loads += 1;
            (path == "a.gtf").then(|| blob.clone())
        };
        let mut report = Report::default();
        let (skins, _) = skin(&model, &picks, &mut load, &mut report);

        assert_eq!(loads, 1, "one read of the archive, not one per slot");
        let first = skins[0].as_ref().expect("slot 0 is textured");
        assert_eq!(skins.len(), 8);
        for (slot, texture) in skins.iter().enumerate() {
            let texture = texture.as_ref().expect("every slot names a.gtf");
            assert!(
                Arc::ptr_eq(first, texture),
                "slot {slot} holds a copy rather than the shared texture"
            );
        }
        assert_eq!(Arc::strong_count(first), 8, "eight slots, one allocation");
    }

    /// The declared binding wins over the ordinal, which is the whole point:
    /// Talon's Junction's `track_surface` puts its first texture at unit 1 and
    /// its second at unit 2.
    #[test]
    fn a_slots_sampler_hash_names_its_unit_and_the_ordinal_does_not() {
        let d = declared(&[(0x11cb_4f74, 0), (0x739a_786e, 1), (0x37b5_db58, 2)]);
        assert_eq!(
            units(
                Some(&d),
                &material(0x739a_786e, Some(0x37b5_db58)),
                Pick::default()
            ),
            (1, 2)
        );
    }

    /// No declaration to read - no variant resolved, or a block this reader
    /// cannot frame - answers exactly what the caller would have assumed.
    #[test]
    fn with_nothing_declared_the_ordinal_answers() {
        assert_eq!(
            units(
                None,
                &material(0x3bdc_0403, Some(0x37b5_db58)),
                Pick::default()
            ),
            (0, 1)
        );
    }

    /// The cloud plate's shape: the first slot's hash is declared, the second
    /// slot's is not, and the second falls back to the ordinal rather than
    /// going unknown - which is what keeps that surface drawing.
    #[test]
    fn an_undeclared_second_slot_keeps_the_ordinal() {
        let d = declared(&[(0x515e_298e, 0), (0xfd66_9142, 1)]);
        assert_eq!(
            units(
                Some(&d),
                &material(0x515e_298e, Some(0x37b5_db58)),
                Pick::default()
            ),
            (0, 1)
        );
    }

    /// Both slots resolving to one unit cannot be right, and taking it would
    /// paint the first texture through the second's role.
    #[test]
    fn two_slots_on_one_unit_is_refused() {
        let d = declared(&[(0xaaaa_aaaa, 1)]);
        assert_eq!(
            units(
                Some(&d),
                &material(0xaaaa_aaaa, Some(0xaaaa_aaaa)),
                Pick::default()
            ),
            (1, 0)
        );
    }
}
