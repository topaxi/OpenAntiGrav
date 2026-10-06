//! Wipeout HD's additive glow layer, per material.
//!
//! The one shape that separates HD's emissive family from every other
//! two-texture material: `MAD H0.xyz, H0.wwww, H1, H0` - the albedo plus the
//! diffuse alpha times a tinted, scrolling sample from unit 1. `mesh.wesl`
//! *selects* between its two textures everywhere else, so without this the
//! glow is simply absent rather than wrong.
//!
//! Its own module rather than more of [`super::skin`], because it asks a
//! different question of the same inputs: `skin::roles` reads which unit each
//! texture is for, and this reads what is done with one of them.
//!
//! # What it will not do
//!
//! **Never the lightmap.** A material whose second slot is the circuit's baked
//! atlas is refused outright, whatever its microcode says: adding a light term
//! and a sun-occlusion mask to the albedo paints a shadow map as a glow. That
//! is the same refusal `skin::roles` already makes for albedo and coverage, and
//! it is why 50 of the disc's accumulating slots get no entry here.
//!
//! # Reach
//!
//! **119 material slots across the 16 circuits** declare the engine's `time`
//! *and* accumulate, from 51 on Modesto Heights and 31 on Tech de Ra down to
//! none at all on the four Zone tracks - `crates/render/examples/hd_emissive_reach.rs`.
//! This module is looser by design and gives a layer to any accumulating slot,
//! **83 on Modesto Heights**, because a material that adds its second texture
//! without a clock still has a glow to draw. What it does *not* do is scroll
//! one it has no rate for; see the three populations below.

use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{Emissive, ModelTexture, slots};

use super::{Report, Textures, skin::Pick};

/// The float3 the emissive sample is multiplied by, parameter `0xe8bcd7f5`.
const TINT: u32 = 0xe8bc_d7f5;
/// `a`, added to `v` before the scale: parameter `0x78256a45`.
const OFFSET: u32 = 0x7825_6a45;
/// `b`, what that sum is scaled by: parameter `0x78787596`.
const SCALE: u32 = 0x7878_7596;
// None of the three has a preimage yet, so each is cited by hash. See
// `crates/render/examples/hd_param_names.rs`, which named 64 of 300.

/// What a sampler's own declared name and disc-wide binding say about
/// whether an accumulate through it is a real glow.
///
/// **Established by what the hash binds, disc-wide, with no counter-example**,
/// the same evidentiary bar `skin::NOT_A_PICTURE` sets, not by a preimage
/// name alone. A preimage match is real evidence the compiler symbol was
/// really named that, but it is not by itself evidence the hash is *never* a
/// picture: three of the seven "Surface maps" preimages
/// (`docs/formats/rcsmaterial.md`, "sampler names, by preimage") turned up a
/// live counter-example on inspection and are excluded from
/// [`NAMED_SURFACE_MAP`] here:
///
/// | Hash | Name | Counter-example |
/// | --- | --- | --- |
/// | `0x20c3e476` | `SpecularTexture` | binds `and_power_glow.gtf`, `and_station4_diff.gtf` and `biodome_bar_colour.gtf` somewhere on the disc |
/// | `0x576c4bf3` | `SpecMap` | binds `startdarksideswatch.gtf`, an advert, live on `scanlinetext` at Talon's Junction |
/// | `0x9fc347ff` | `Spec` | binds `detonator_ao.gtf`, an ambient-occlusion map |
/// | `0x48f37f5a` | `NormalMap` | a softer call: its one bind, `medal_dissolvehexnormal.gtf`, may name a hex pattern rather than a lighting normal, and this reading does not settle which |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SamplerRole {
    /// A normal or specular map: never a glow, whatever the microcode's own
    /// accumulate shape says. On a ship hull, `accumulates(1)` is tripped by
    /// ordinary tangent-space lighting arithmetic reusing a register, not a
    /// real additive combine - see `docs/rendering/hd-ship-materials.md`,
    /// Finding 1.
    SurfaceMap,
    /// A named glow/reflection role - the "Light" preimage group minus
    /// `lightmap`/`shadowMapTex`, which this function already refuses on an
    /// earlier, separate check.
    Glow,
}

/// Named, disc-wide "Surface maps" preimages with **zero** counter-examples
/// measured (`crates/render/examples/hd_emissive_role_census.rs`'s own doc
/// comment carries the full seven-hash table and which four were dropped).
const NAMED_SURFACE_MAP: &[u32] = &[
    0x739a_786e, // NormalTexture
    0x62ae_87a7, // NormalTexture2
    0xd9d6_922d, // Normal
];

/// Per-material-family sampler hashes with no preimage, each measured
/// disc-wide to bind **only** a normal map (or, for the fourth, weapon
/// normal maps too), with zero exceptions - see
/// `crates/render/examples/hd_emissive_role_census.rs`'s own doc comment for
/// the full per-hash path table this was measured from.
///
/// | Hash | Family | Paths |
/// | --- | --- | ---: |
/// | `0x436d3929` | `diffuse_with_specular_from_alpha_n_vcol`/`_n`/`detonator_*` | 45 |
/// | `0xc8f18561` | `carbonfibre` | 11 |
/// | `0x0617f872` | `nitro_body_new` | 13 |
/// | `0xc78c9866` | `detonator_ship_dg_iridescent` | 4 |
const SHIP_SURFACE_MAP_SAMPLERS: &[u32] = &[0x436d_3929, 0xc8f1_8561, 0x0617_f872, 0xc78c_9866];

/// The same shape on circuit scenery, keyed by **material family and
/// sampler hash together**, because on a circuit the hash alone is not
/// enough: `Texture2` (`0xa2d555b9`) binds a normal map under
/// `diffuse_normal_specular` and a picture under other families, and a hash
/// that is sometimes a picture cannot be refused by hash. Each pair below was
/// measured across every `.rcsmodel` in all seven archives (643 models) and
/// **every** path it binds decoded and checked: 136 paths, 119 with the
/// canonical tangent-space signature (RGB mean near `(128, 128, 250)`) and 17
/// DXT5 two-channel packings reading as flat `(128, 128, 128)` - all named
/// `*_n`, `*_ne` or `*normal*`, none a picture.
///
/// | Family | Hash | Paths |
/// | --- | --- | ---: |
/// | `tech_de_ra_rocks` | `0x0cddca48` | 1 - `rocks_01_normal_alpha.gtf`, the only path the hash binds at all |
/// | `diffuse_normal_specular` | `0xa2d555b9` | 37 |
/// | `diffuse_normal_specular_emmissive` | `0xa2d555b9` | 47 |
/// | `weapon_pads` | `0xa2d555b9` | 21 - the pads' `_ne` files, whose RGB `docs/rendering/pads.md` already measured as a normal map |
/// | `glass_reflect_opacity_normal` | `0xa2d555b9` | 6 |
/// | `and_glass_normscale` | `0xa2d555b9` | 2 |
/// | `glass_texture_n` | `0xa2d555b9` | 2 - the ship glass, `hd-ship-materials.md`'s own open case |
/// | `pb_diffalphaspecnormal` | `0x3bdc0403` | 4 |
/// | `tracktexture_with_normal` | `0x48f37f5a` | 4 |
/// | `reflectplane_dc_seawater` | `0x11cb4f74` | 4 |
/// | `glassalpha` | `0xfc52b822` | 3 |
/// | `glassalpha_customr` | `0xfc52b822` | 1 |
/// | `dc_windows_a` | `0xfc52b822` | 1 |
/// | `mt_windows_a` | `0xfc52b822` | 1 |
/// | `mt_windows_c_opaque` | `0xfc52b822` | 1 |
/// | `mt_tunnelrefraction` | `0x41d572a2` | 1 |
/// | `cf_icetunnel` | `0xfe9bd1f3` | 1 |
///
/// **How the first row was found**: Tech De Ra's mountains drew cyan/purple.
/// Of the circuit's 22 `tech_de_ra_rocks` slots, slot 343 (58 chunks, the
/// mountain range itself) is the only one whose `lightmap` entry carries no
/// path, so `Pick::aux` fell through to entry 1 - this normal map - and the
/// accumulate added it, untinted, to `rocks_01_sand.gtf`. Every other rock
/// slot names a `-lmap.gtf` there and drew correctly. A brute-force sweep of
/// about a million compound names against `rcsmaterial::name_hash` lands
/// `Normal_Spec` on this hash (and `Dirt` - already a recovered preimage -
/// and `Rock` on the material's other two samplers; three hits for three
/// targets where chance predicts 0.0007), which agrees with the file's own
/// name; the refusal rests on the bind census, not on that reading. The
/// other rows are the same census run over every accumulating slot on the
/// disc whose added texture decodes to a normal map.
///
/// What refusing does *not* do is draw the normal map in its own role: that
/// still wants the tangent frame `mesh.wesl` has no input for, and the pads'
/// own `_ne`-alpha-gated term is a separate open question on
/// `docs/rendering/pads.md`. Nothing added is an honest absence; a normal
/// map added as a glow was not.
const CIRCUIT_SURFACE_MAP_SAMPLERS: &[(&str, u32)] = &[
    ("tech_de_ra_rocks", 0x0cdd_ca48),
    ("diffuse_normal_specular", 0xa2d5_55b9),
    ("diffuse_normal_specular_emmissive", 0xa2d5_55b9),
    ("weapon_pads", 0xa2d5_55b9),
    ("glass_reflect_opacity_normal", 0xa2d5_55b9),
    ("and_glass_normscale", 0xa2d5_55b9),
    ("glass_texture_n", 0xa2d5_55b9),
    ("pb_diffalphaspecnormal", 0x3bdc_0403),
    ("tracktexture_with_normal", 0x48f3_7f5a),
    ("reflectplane_dc_seawater", 0x11cb_4f74),
    ("glassalpha", 0xfc52_b822),
    ("glassalpha_customr", 0xfc52_b822),
    ("dc_windows_a", 0xfc52_b822),
    ("mt_windows_a", 0xfc52_b822),
    ("mt_windows_c_opaque", 0xfc52_b822),
    ("mt_tunnelrefraction", 0x41d5_72a2),
    ("cf_icetunnel", 0xfe9b_d1f3),
];

/// Named, disc-wide "Light" preimages minus `lightmap`/`shadowMapTex`, which
/// [`emissive`] already refuses on a separate, earlier check.
const NAMED_GLOW: &[u32] = &[
    0xb1f2_a176, // EmissiveTexture
    0xfa79_b1cd, // Emissive
    0x030f_d39b, // emissive
    0xb160_0426, // ReflectionMap
    0x2d5f_c6a6, // EnvMap
    0x6e46_5921, // EnvMap1
];

/// The material's leaf file name without its extension - the key the
/// per-family tables are written in.
fn family(material: &rcsmodel::Material) -> &str {
    material
        .name
        .rsplit('/')
        .next()
        .unwrap_or(&material.name)
        .trim_end_matches(".rcsmaterial")
}

fn sampler_role(material: &rcsmodel::Material, hash: u32) -> Option<SamplerRole> {
    if NAMED_SURFACE_MAP.contains(&hash)
        || SHIP_SURFACE_MAP_SAMPLERS.contains(&hash)
        || CIRCUIT_SURFACE_MAP_SAMPLERS.contains(&(family(material), hash))
    {
        Some(SamplerRole::SurfaceMap)
    } else if NAMED_GLOW.contains(&hash) {
        Some(SamplerRole::Glow)
    } else {
        None
    }
}

/// The glow table and each material's index into it, plus one.
///
/// `roles` is read for [`slots::SECOND_IS_LIGHTMAP`] and written with
/// [`slots::ADD_SECOND`] and the index, so it must already carry what
/// `skin::roles` produced.
///
/// Deduplicated by value, like the texture-transform tracks: a circuit's
/// animated materials collapse to a handful of distinct `(tint, rate)` pairs
/// and the shader's table stays small.
pub(super) fn emissive(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    picks: &[Pick],
    seconds: &[Option<std::sync::Arc<ModelTexture>>],
    roles: &mut [u32],
    textures: Textures<'_>,
    report: &mut Report,
) -> Vec<Emissive> {
    let mut table: Vec<Emissive> = Vec::new();
    let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();

    for (slot, material) in model.materials.iter().enumerate() {
        let Some(packed) = roles.get_mut(slot) else {
            continue;
        };
        // The three conditions, in the order that makes the cheapest one
        // first: there has to be a decoded second texture to add, it must not
        // be the baked atlas, and the program must actually add it.
        if !seconds.get(slot).is_some_and(Option::is_some) {
            continue;
        }
        if *packed & slots::SECOND_IS_LIGHTMAP != 0 {
            continue;
        }
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let blob = cache
            .entry(material.name.clone())
            .or_insert_with(|| textures(&format!("/{}", material.name)))
            .clone();
        let Some(blob) = blob else { continue };
        let Some(program) = rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)
        else {
            continue;
        };
        // Unit 1 is where every material this reading has met binds its second
        // texture, and `skin::units` is what says so per material - but it
        // answers for the *`.gtf` slot*, and this asks about the microcode's
        // own unit. They agree on every accumulating material measured.
        if !program.accumulates(1) {
            continue;
        }

        // **The role of the texture actually being added, not of whatever
        // sits at hardware unit 1.** `Pick::aux` is the same ordinal
        // `skin::skin` already decoded into `seconds[slot]` - the texture
        // this function is about to tint and add - so its own declared
        // sampler hash is what a role decision has to be about. Resolving
        // "unit 1" independently (`Declared::samplers` cross-reference,
        // ignoring `Pick::aux`) was tried and measured wrong:
        // `tunnel_fx_noalpha` declares `SpecularTexture` at hardware unit 1
        // and its own *emissive* texture at unit 2, while `Pick::aux` (no
        // lightmap, an untraced alpha lane) resolves to ordinal 1 - the
        // emissive one - exactly as `seconds[slot]` already holds. Asking
        // about "unit 1" there would have refused a real, working glow this
        // project already regressed once (`docs/rendering/hd-ship-materials.md`,
        // "two fixes tried and both failed").
        let aux_hash = picks
            .get(slot)
            .and_then(|pick| pick.aux)
            .and_then(|aux| material.samplers.get(aux))
            .map(|&(hash, _)| hash);
        match aux_hash.map(|hash| sampler_role(material, hash)) {
            Some(Some(SamplerRole::SurfaceMap)) => {
                report.emissive_surface_map_excluded += 1;
                continue;
            }
            Some(Some(SamplerRole::Glow)) => {}
            Some(None) | None => report.emissive_role_unresolved += 1,
        }

        let find = |hash: u32| material.parameters.iter().find(|p| p.hash == hash);
        let tint = find(TINT).map_or([1.0, 1.0, 1.0], |p| [p.value[0], p.value[1], p.value[2]]);

        // **A material that accumulates is not necessarily one that scrolls**,
        // and conflating the two is a defect rather than an approximation. The
        // three populations, over the circuits measured:
        //
        // - it authors `a` and `b` (`mt_uvanim_diffuse_emissive`) - the scroll
        //   is per material and this replays it;
        // - it declares `time` but authors neither
        //   (`nr_billboardholographicscanlines`, 17 slots on one circuit;
        //   `scanlinebillboard`, `scroller_glow_v3`, `pipefx_v2`) - its
        //   coordinate constants are the shader's own **inline literals**,
        //   which live in the code rather than in the material record and
        //   which this reading has not recovered;
        // - it does not declare `time` at all (`glass_texture_customr`,
        //   `and_diffuse_emissive`, `bluemetal`) - a still additive layer.
        //
        // Only the first scrolls here. **A `scale` of 0 is the most
        // destructive value in the range, not a neutral one**: it annihilates
        // the surface's own `v`, so every pixel samples one row of the texture
        // and that row marches down it. Defaulting to `1.0` and gating the
        // clock on the material having authored the pair is the honest shape -
        // the second population draws its glow still rather than sliding
        // through a texture on a rate nobody read.
        let (offset, scale, rate) = match (find(OFFSET), find(SCALE)) {
            (Some(a), Some(b)) => (a.value[0], b.value[0], 1.0),
            _ => (0.0, 1.0, 0.0),
        };
        let layer = Emissive {
            tint,
            offset,
            scale,
            rate,
        };

        let index = table.iter().position(|seen| *seen == layer).or_else(|| {
            // Past the shader's table the surface draws without its glow
            // rather than the build failing - the same graceful direction the
            // texture-transform and node-transform ceilings take.
            (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                table.push(layer);
                table.len() - 1
            })
        });
        let Some(index) = index else { continue };
        *packed |= slots::ADD_SECOND;
        *packed |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
    }
    table
}

/// How many distinct glow layers one model may carry, matching `mesh.wesl`'s
/// `Emissives` array.
///
/// Slot 0 is "no glow", so a model gets `EMISSIVE_LIMIT - 1` real entries.
///
/// **Measured**: the busiest circuit is Modesto Heights at 51 accumulating
/// slots, which deduplicate further, against 64 here - and 64 `vec4` pairs is
/// 2 KiB of uniform, well inside the 64 KiB binding every backend guarantees.
pub const EMISSIVE_LIMIT: usize = 64;
