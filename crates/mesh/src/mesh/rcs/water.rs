//! HD's water family: a program that reads its picture as a normal map.
//!
//! `water_test_2` and `water_noref` (Vineta K's sea) declare a texture,
//! `waves2.gtf`, and read it only as a normal: three taps, each decoded
//! `2 * tap - 1` and summed into a perturbed normal
//! (`docs/formats/rcsmaterial.md`, "Water family"). Their colour is never a
//! picture; it is the vertex colour times the light plus a lookup in the
//! engine's `paraboloidReflectionTex`. Binding `waves2.gtf` as the albedo, as
//! the generic reading did, drew the normal map as a blue sheet.
//!
//! The classifier is by what the resolved program does, not by name: it
//! declares the paraboloid probe, at least one other sampler is decoded as a
//! normal, no declared sampler besides the lightmap is read as a colour, and
//! the baked-lightmap lighting formula is absent.
//!
//! **The sea and Amphiseum's pool** are classified by their own program facts
//! (`docs/rendering/water.md`, "HD/Fury: `reflectplane_dc_seawater`" and
//! "HD/Fury: Vineta K and Amphiseum"): `reflectplane_dc_seawater` declares
//! `screenSpaceReflectionTex`, `DiffuseTexture` and `DiffuseTexture1` and reads
//! neither of the two as a colour; Amphiseum's `water` declares the paraboloid,
//! `NormalTexture2`, `ColourConstant` and `time`.
//!
//! Under a law that draws the taps ([`Reading::Taps`]: HD's `original`, and
//! `hd`), every family gets its program's scrolled normal taps ([`Tap`]) through a
//! [`PROGRAM_WATER`] entry, the normal map bound third, and the chunk's own
//! tangent frame; the sea draws `light * ColourConstant + specular` plus the sky paraboloid
//! at `Reflectivity` in place of the planar reflection (a stand-in), the other three add the
//! sky paraboloid. Under `off` every
//! family keeps the reading it had before the taps were drawn.

use oag_display::display::Water;
use oag_rcs::rcsmaterial::fragment::Program;
use oag_rcs::{rcsmaterial, rcsmodel};

use crate::mesh::{ModelTexture, Texels};

use super::skin::decode_texture;
use super::{Report, TextureSlots, Textures};
use crate::mesh::{Emissive, PROGRAM_WATER, TangentFrame, slots};

use super::emissive::EMISSIVE_LIMIT;

/// `~crc32("paraboloidReflectionTex")`.
pub(super) const PARABOLOID: u32 = 0x9edd_3243;
/// `~crc32("PerMaterialEnvMap")` (exact preimage). 2048's `Environment_Load`
/// scans every material for it; HD's `.rcsmodel` records carry it too, **no HD
/// program declares it and the HD executable does not contain the string**,
/// so it is not read as HD's reflection source (`rcsmaterial.md`, "Water
/// family").
#[allow(dead_code)]
pub(super) const PER_MATERIAL_ENV_MAP: u32 = 0x8365_b1f3;

/// The two parameters the baked-lightmap lighting formula patches
/// (`pow(lightmap.rgb, 0x002c73e8) * 0x8670f0be + ...`,
/// `docs/rendering/pads.md`). A water program is not lit by the baked atlas;
/// a glass or tinted surface that reads a normal map and the paraboloid probe
/// **is** (`ds_booth_glass`, `mesh_colour_constant`: both match every other
/// fact below and both declare these two), so they keep their generic reading.
const LIGHTMAP_FORMULA: [u32; 2] = [0x002c_73e8, 0x8670_f0be];

/// Whether a resolved program is the water shape described above.
pub(super) fn is_water(declared: &rcsmaterial::Declared, program: &Program) -> bool {
    if !declared.samplers.iter().any(|&(h, _)| h == PARABOLOID)
        || LIGHTMAP_FORMULA
            .iter()
            .any(|h| declared.parameters.contains(h))
    {
        return false;
    }
    let others = || {
        declared
            .samplers
            .iter()
            .filter(|&&(h, _)| h != PARABOLOID && h != rcsmaterial::LIGHTMAP_SAMPLER)
            .map(|&(_, unit)| u8::try_from(unit).unwrap_or(u8::MAX))
    };
    others().any(|u| program.samples_as_normal(u))
        && others().all(|u| program.samples_as_normal(u) || !program.samples_colour(u))
}

/// `~crc32("constantAmbientColour")`, which only the lit-vertex-colour variant
/// declares.
const CONSTANT_AMBIENT: u32 = rcsmaterial::CONSTANT_AMBIENT;

/// The `rate` that marks a glow-table entry as `water_test_2`'s rather than the
/// ice pool's: no glow entry scrolls at 2.0 (`mesh::Emissive::rate` is 0 or 1).
pub(crate) const WATER_FLAG: f32 = 2.0;

/// Whether `law` is drawn **on a title other than its own** yet: the WATER row offers a
/// value only while this holds (`docs/rendering/water.md`, "Open"). A law is always drawn on
/// its own title as `original` (HD's taps are, under `original` on HD), so a borrowed value
/// becomes true only once a borrower draws it: `Hd` with the lane that borrows HD's law onto
/// Pulse, Pure, 2048 and Omega. Omega's own law is drawn on Omega as its `original`
/// (`mesh::rcs::psp2::water`), and `Omega` stays false until a title borrows it: that needs the
/// lender's archive opened in the borrower's race load, which nothing does yet
/// (`docs/rendering/water.md`, "Omega (PS4)"). `Enhanced` is drawn by `mesh::vex_water` on the titles that
/// name their water surfaces.
#[must_use]
pub const fn is_drawn(law: Water) -> bool {
    match law {
        Water::Off | Water::Original | Water::Enhanced => true,
        Water::Hd | Water::Vita2048 | Water::Omega => false,
    }
}

/// `program[1]` of a 2048 sea's entry: the shape of `sea_block`, read as 2048's `fc15` program
/// reads it (RGB-decoded normal map, Phong highlight weighted by the map's alpha). HD's
/// families are 1 to 4, `vex_water`'s `enhanced` is 5.
pub const VITA_SEA_KIND: f32 = 6.0;

/// `program[1]` of a 2048 `fc17` water entry: kind 6's frame and light with `Watercolour` for
/// the colour and `SpecularColour` for the highlight, no gloss from the map's alpha
/// (`docs/rendering/water.md`, "2048 (Vita)").
pub const VITA_WATER_KIND: f32 = 7.0;

/// `program[1]` of the blended `fc17` edge: [`VITA_WATER_KIND`]'s colour and the alpha chain
/// `mix(a * vc.x, Opacity, vc.y)`, `a` the map's alpha, `vc` `VertexColour1`; `Opacity` rides the
/// header's `offset`.
pub const VITA_EDGE_KIND: f32 = 8.0;

/// `program[1]` of an Omega reflective sea's entry (`reflectplane_mid` and its siblings,
/// `mesh::rcs::psp2::water`): two `.ag` taps of the wave normal map at `(+-u + K Speed t, v)`,
/// `Watercolour` times the light, a Phong highlight in `SpecularColour`, and the reflection
/// weighted by [`Fresnel`] (`docs/rendering/water.md`, "Omega (PS4)").
pub const OMEGA_SEA_KIND: f32 = 9.0;

/// `program[1]` of Omega's blended sea edge (`reflectplane_edge`): [`OMEGA_SEA_KIND`] with the
/// alpha `VertexColour1.y * Opacity`; `Opacity` rides the header's `offset`.
pub const OMEGA_EDGE_KIND: f32 = 10.0;

/// The weight Omega's sea programs give the reflection:
/// `ReflectivityStrength * (ReflectivityMin + ReflectivityScale * sat(1 - N.V)^ReflectivityPower)`
/// (`reflectplane_mid` pixel `@0x28c`..`@0x30c` and `@0x380`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fresnel {
    pub min: f32,
    pub scale: f32,
    pub power: f32,
    pub strength: f32,
}

impl Fresnel {
    /// The weight at `facing = sat(1 - N.V)`.
    #[must_use]
    pub fn weight(self, facing: f32) -> f32 {
        self.strength * (self.min + self.scale * facing.clamp(0.0, 1.0).powf(self.power))
    }

    /// What the entry carries: the constant term and the facing term's factor, each already
    /// times `ReflectivityStrength` (the program uses only the products), and the power.
    #[must_use]
    pub fn lanes(self) -> [f32; 3] {
        [
            self.strength * self.min,
            self.strength * self.scale,
            self.power,
        ]
    }
}

/// The three glow-table entries of an Omega reflective sea: [`sea_block`]'s layout (`opacity`
/// in the header's `offset`), with [`Fresnel::lanes`] in the two tap entries' `rate` lanes
/// (the constant term, the facing factor) and the second tap entry's last `program` lane (the
/// power).
#[must_use]
pub fn reflective_block(
    kind: f32,
    colour: [f32; 3],
    opacity: f32,
    power: f32,
    specular: [f32; 3],
    taps: [&Tap; 2],
    fresnel: Fresnel,
) -> Vec<Emissive> {
    let mut block = sea_block(kind, colour, opacity, power, specular, taps);
    let [constant, facing, exponent] = fresnel.lanes();
    block[1].rate = constant;
    block[2].rate = facing;
    block[2].program[1] = exponent;
    block
}

/// What a classified water slot is drawn as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reading {
    /// The program's own colour term with no normal map, no scroll and no
    /// reflection: lit vertex colour, or black plus a glint; the sea and
    /// Amphiseum's pool keep the generic reading.
    Constant,
    /// HD's law: the scrolled normal taps, the program's colour, specular and the sky
    /// reflection where it has one; not the planar reflection.
    Taps,
}

/// The reading each law draws a classified water slot as. This module reads HD's
/// programs, so `original` (on HD) and `hd` draw the taps; `off` is the constant
/// reading by definition, and the 2048 and Omega laws are not drawn here yet.
const fn reading(law: Water) -> Reading {
    match law {
        Water::Original | Water::Hd => Reading::Taps,
        Water::Off | Water::Enhanced | Water::Vita2048 | Water::Omega => Reading::Constant,
    }
}

/// The scroll rate of the sea's taps, per unit of the engine `time`: the literal
/// `c[207].z` of `reflectplane_dc_seawater`'s vertex block 3 (`MAD o[TC4].x, 0.05,
/// time, u`), the same literal Amphiseum's fragment block and Vineta K's vertex block
/// multiply `time` by (`docs/rendering/water.md`, "How it is drawn").
pub const SEA_SCROLL: f32 = 0.05;

/// The second sea tap's scale: `MUL R2.xy, R2, 1.15` before the fetch from
/// `DiffuseTexture1` (fragment `@0x09` of the sea's lit block).
pub const SEA_SECOND_SCALE: f32 = 1.15;

/// One fetch of a water program's normal map: `uv * scale + time * scroll`, its
/// decoded normal weighted by `weight` in the sum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tap {
    pub scale: [f32; 2],
    pub scroll: [f32; 2],
    pub weight: f32,
}

impl Tap {
    /// The coordinate this tap fetches at `uv` and engine time `time`, in seconds.
    #[must_use]
    pub fn at(self, uv: [f32; 2], time: f32) -> [f32; 2] {
        [
            uv[0] * self.scale[0] + time * self.scroll[0],
            uv[1] * self.scale[1] + time * self.scroll[1],
        ]
    }

    /// The sea's two taps (and Amphiseum's): `(u + 0.05 t, v)` and
    /// `1.15 (-u + 0.05 t, v)`, summed unweighted.
    #[must_use]
    pub const fn sea() -> [Self; 2] {
        [
            Self {
                scale: [1.0, 1.0],
                scroll: [SEA_SCROLL, 0.0],
                weight: 1.0,
            },
            Self {
                scale: [-SEA_SECOND_SCALE, SEA_SECOND_SCALE],
                scroll: [SEA_SECOND_SCALE * SEA_SCROLL, 0.0],
                weight: 1.0,
            },
        ]
    }

    /// 2048's own sea: one tap, `(u + 0.05 t, v)` (the vertex program adds `0.05 * time` to
    /// `uv1.x`, `docs/rendering/water.md`, "2048 (Vita)"), and a second of weight zero, which
    /// keeps the two-tap entry layout the shader reads.
    #[must_use]
    pub const fn vita_sea() -> [Self; 2] {
        let [first, second] = Self::sea();
        [
            first,
            Self {
                weight: 0.0,
                ..second
            },
        ]
    }

    /// `fc17`'s water: one tap at `(u, v + Speed * t)` (the vertex program adds
    /// `Speed * time` to `uv1.y`), and a second of weight zero.
    #[must_use]
    pub const fn vita_water(speed: f32) -> [Self; 2] {
        let [first, second] = Self::vita_sea();
        [
            Self {
                scroll: [0.0, speed],
                ..first
            },
            second,
        ]
    }

    /// Omega's reflective sea: both taps at `(sign * u + rate * Speed * t, v)`, summed
    /// unweighted. `rate` and the two signs are the pixel program's own
    /// ([`oag_rcs::gcn::SeaTaps`]: `0.05`, `+u` on `NormalMap` and `-u` on `NormalMap1`),
    /// `speed` the material's `Speed`.
    #[must_use]
    pub fn reflective(rate: f32, signs: [f32; 2], speed: f32) -> [Self; 2] {
        signs.map(|sign| Self {
            scale: [sign, 1.0],
            scroll: [rate * speed, 0.0],
            weight: 1.0,
        })
    }

    /// One of Vineta K's three taps, off the material's own `scale` and `rate`:
    /// `uv * scale + 0.05 t * rate` on both axes (vertex block 3, `MAD o[TC5].xy,
    /// 0.05 time, rate, uv * scale`).
    #[must_use]
    pub fn vineta(scale: f32, rate: f32, weight: f32) -> Self {
        Self {
            scale: [scale, scale],
            scroll: [SEA_SCROLL * rate, SEA_SCROLL * rate],
            weight,
        }
    }
}

/// `~crc32("screenSpaceReflectionTex")`.
const SCREEN_REFLECTION: u32 = 0xec2b_3fc2;
/// `~crc32("DiffuseTexture")` and `~crc32("DiffuseTexture1")`: the sea's two taps.
const DIFFUSE_TEXTURE: u32 = 0x11cb_4f74;
const DIFFUSE_TEXTURE1: u32 = 0x06a7_7e50;
/// Amphiseum's two tap samplers: unit 0 (no preimage) and `~crc32("NormalTexture2")`.
const POOL_FIRST: u32 = 0x739a_786e;
const NORMAL_TEXTURE2: u32 = 0x62ae_87a7;
/// `~crc32("ColourConstant")`, `Reflectivity`, `SpecularColour`, `SpecularPower`, `time`.
const COLOUR_CONSTANT: u32 = 0x4d60_d566;
const REFLECTIVITY: u32 = 0x5e5b_1937;
const SPECULAR_COLOUR: u32 = 0x370a_63cb;
const SPECULAR_POWER: u32 = 0x81e0_e773;
const TIME: u32 = 0x906b_67ba;
/// Vineta K's waves sampler (`waves2`, unit 0 of both programs) and `water_noref`'s
/// second map, whose alpha it reads at `uv` plus the third tap.
const WAVES: u32 = 0xb0b3_4798;
const NOREF_SECOND: u32 = 0x65f5_d93c;
/// Vineta K's per-tap scale and rate parameters, in tap order (vertex block 3:
/// `c464`, `c462`, `c463` and `c461`, `c460`, `c459`).
const VINETA_SCALES: [u32; 3] = [0xf4bc_8b2f, 0x6db5_da95, 0x1ab2_ea03];
const VINETA_RATES: [u32; 3] = [0x0256_7724, 0x9b5f_269e, 0xec58_1608];
/// How Vineta K's fragment programs sum the three decoded taps: `t3 + (t1 + t2) / 2`
/// (`MAD /2` twice, `@0x01`..`@0x0d` of both lit blocks), up to the normalise.
const VINETA_WEIGHTS: [f32; 3] = [0.5, 0.5, 1.0];

/// The water families, by program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    /// `reflectplane_dc_seawater`.
    Sea,
    /// Amphiseum's `water`: the sea's taps and the sky reflection.
    Sky,
    /// `water_test_2`: lit vertex colour plus the reflection times it.
    LitColour,
    /// `water_noref`: the reflection and a glint.
    Glint,
}

impl Family {
    /// `program[1]` of the entry, read by `shade.wesl`.
    fn kind(self) -> f32 {
        match self {
            Self::Sea => 1.0,
            Self::Sky => 2.0,
            Self::LitColour => 3.0,
            Self::Glint => 4.0,
        }
    }
}

/// How many chunks of each family a model has, counted whatever the law draws.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WaterChunks {
    pub sea: usize,
    pub sky: usize,
    pub lit_colour: usize,
    pub glint: usize,
}

fn family(declared: &rcsmaterial::Declared, program: &Program) -> Option<Family> {
    let unit = |hash: u32| {
        declared
            .samplers
            .iter()
            .find(|&&(h, _)| h == hash)
            .map(|&(_, u)| u8::try_from(u).unwrap_or(u8::MAX))
    };
    let not_colour = |hash: u32| unit(hash).is_some_and(|u| !program.samples_colour(u));
    let declares = |hash: u32| declared.parameters.contains(&hash);
    if unit(SCREEN_REFLECTION).is_some()
        && not_colour(DIFFUSE_TEXTURE)
        && not_colour(DIFFUSE_TEXTURE1)
        && declares(COLOUR_CONSTANT)
    {
        return Some(Family::Sea);
    }
    if unit(PARABOLOID).is_some()
        && not_colour(NORMAL_TEXTURE2)
        && declares(COLOUR_CONSTANT)
        && declares(TIME)
    {
        return Some(Family::Sky);
    }
    is_water(declared, program).then(|| {
        if declares(CONSTANT_AMBIENT) {
            Family::LitColour
        } else {
            Family::Glint
        }
    })
}

/// The first vec4 of an authored parameter.
fn parameter(material: &rcsmodel::Material, hash: u32) -> Option<[f32; 4]> {
    material
        .parameters
        .iter()
        .find(|p| p.hash == hash)
        .map(|p| p.value)
}

/// The path a material binds at `hash`.
fn path_of(material: &rcsmodel::Material, hash: u32) -> Option<&str> {
    material
        .samplers
        .iter()
        .find(|(h, p)| *h == hash && p.is_some())
        .and_then(|(_, p)| p.as_deref())
}

/// What a family's taps read: the taps, the normal map, and for `water_noref` its
/// second map. `None` where the material does not author what the program reads, or
/// binds two different files where the law needs one.
fn taps_of(
    material: &rcsmodel::Material,
    family: Family,
) -> Option<(Vec<Tap>, String, Option<String>)> {
    match family {
        Family::Sea | Family::Sky => {
            let (a, b) = match family {
                Family::Sea => (DIFFUSE_TEXTURE, DIFFUSE_TEXTURE1),
                _ => (POOL_FIRST, NORMAL_TEXTURE2),
            };
            let map = path_of(material, a)?;
            (path_of(material, b)? == map).then(|| (Tap::sea().to_vec(), map.to_string(), None))
        }
        Family::LitColour | Family::Glint => {
            let mut taps = Vec::with_capacity(3);
            for ((scale, rate), weight) in
                VINETA_SCALES.iter().zip(&VINETA_RATES).zip(VINETA_WEIGHTS)
            {
                taps.push(Tap::vineta(
                    parameter(material, *scale)?[0],
                    parameter(material, *rate)?[0],
                    weight,
                ));
            }
            let second = match family {
                Family::Glint => Some(path_of(material, NOREF_SECOND)?.to_string()),
                _ => None,
            };
            Some((taps, path_of(material, WAVES)?.to_string(), second))
        }
    }
}

/// The three consecutive glow-table entries of one tapped water slot, kept to three because
/// the table is shared with every other family and a circuit's weapon pads copy an entry
/// each after it (`pads::each_node_its_own_glow`).
///
/// - The sea and the pool: the header (`ColourConstant`, the reflection weight, the specular
///   power), then one entry per [`Tap`] (`tint` the `u` and `v` scale and the `u` scroll,
///   `offset` the `v` scroll, `scale` the weight); the sea's `SpecularColour` rides the two
///   tap entries' `program` lanes (`r`, `g`, then `b`).
/// - Vineta K: the header's `tint` is the three tap weights, then the three taps' scales
///   (one per axis pair, the program scales both axes alike) and their scrolls.
///
/// `water_noref`'s reflection weight differs by variant: the lightmapped block (`@0x3870`,
/// declares the lightmap) multiplies by the second map's alpha `a`, the other (`@0x24b0`)
/// by `0.2 a + 0.4`; the header's `offset` 0 and 1 say which.
fn entries(
    material: &rcsmodel::Material,
    family: Family,
    taps: &[Tap],
    lightmapped: bool,
) -> Option<Vec<Emissive>> {
    let rgb = |v: [f32; 4]| [v[0], v[1], v[2]];
    let header = |tint: [f32; 3], offset: f32, scale: f32| Emissive {
        tint,
        offset,
        scale,
        rate: 0.0,
        program: [PROGRAM_WATER, family.kind()],
    };
    match family {
        Family::Sea | Family::Sky => {
            let [first, second] = taps else {
                return None;
            };
            let colour = rgb(parameter(material, COLOUR_CONSTANT)?);
            let (offset, scale, specular) = match family {
                Family::Sea => (
                    parameter(material, REFLECTIVITY).map_or(0.0, |v| v[0]),
                    parameter(material, SPECULAR_POWER)?[0],
                    rgb(parameter(material, SPECULAR_COLOUR)?),
                ),
                _ => (parameter(material, REFLECTIVITY)?[0], 0.0, [0.0; 3]),
            };
            Some(sea_block(
                family.kind(),
                colour,
                offset,
                scale,
                specular,
                [first, second],
            ))
        }
        Family::LitColour | Family::Glint => {
            let [a, b, c] = taps else {
                return None;
            };
            let uniform = |t: &Tap| t.scale[0] == t.scale[1] && t.scroll[0] == t.scroll[1];
            if !(uniform(a) && uniform(b) && uniform(c)) {
                return None;
            }
            let reflection = match family {
                Family::Glint => f32::from(u8::from(!lightmapped)),
                _ => 1.0,
            };
            let row = |tint: [f32; 3]| Emissive {
                tint,
                offset: 0.0,
                scale: 0.0,
                rate: 0.0,
                program: [0.0; 2],
            };
            Some(vec![
                header([a.weight, b.weight, c.weight], reflection, 0.0),
                row([a.scale[0], b.scale[0], c.scale[0]]),
                row([a.scroll[0], b.scroll[0], c.scroll[0]]),
            ])
        }
    }
}

/// The three consecutive glow-table entries of a two-tap water slot (the sea, the pool, and
/// `enhanced` on a title with no water program): the header (`colour`, the reflection weight
/// as `offset`, the specular power as `scale`, `kind` in `program[1]`), then one entry per
/// [`Tap`] (`tint` the `u` and `v` scale and the `u` scroll, `offset` the `v` scroll, `scale`
/// the weight), the specular colour riding the two tap entries' `program` lanes.
pub(crate) fn sea_block(
    kind: f32,
    colour: [f32; 3],
    reflection: f32,
    power: f32,
    specular: [f32; 3],
    taps: [&Tap; 2],
) -> Vec<Emissive> {
    let head = Emissive {
        tint: colour,
        offset: reflection,
        scale: power,
        rate: 0.0,
        program: [PROGRAM_WATER, kind],
    };
    let tap = |tap: &Tap, program: [f32; 2]| Emissive {
        tint: [tap.scale[0], tap.scale[1], tap.scroll[0]],
        offset: tap.scroll[1],
        scale: tap.weight,
        rate: 0.0,
        program,
    };
    vec![
        head,
        tap(taps[0], [specular[0], specular[1]]),
        tap(taps[1], [specular[2], 0.0]),
    ]
}

/// Lays `block` into the table as consecutive entries (sharing an identical block
/// already there) and returns its first index.
pub(crate) fn lay(table: &mut Vec<Emissive>, block: &[Emissive]) -> Option<usize> {
    table
        .windows(block.len())
        .position(|seen| seen == block)
        .or_else(|| {
            (table.len() + block.len() < EMISSIVE_LIMIT).then(|| {
                let at = table.len();
                table.extend_from_slice(block);
                at
            })
        })
}

/// What [`water`] binds for the taps: the normal map third, `water_noref`'s second map
/// fourth.
pub(super) struct Bound {
    pub(super) normal_maps: TextureSlots,
    pub(super) second_maps: TextureSlots,
}

/// Takes the normal map out of every water material's albedo.
///
/// Two programs share the shape and differ in what is left once the
/// reflection (the engine's `paraboloidReflectionTex`, not drawn) is taken
/// out:
///
/// - one that declares `constantAmbientColour` (`water_test_2`) computes
///   `vertexColour * (ambient + sun * N.L)` plus the reflection times the
///   vertex colour, so with no reflection it is **lit vertex colour**: no
///   picture, which the white placeholder gives;
/// - one that does not (`water_noref`) has no colour but the reflection and a
///   sun glint, so with no reflection it is **black plus the glint**: a black
///   picture, the glint left to the generic specular (**chosen, not
///   measured**), and fog on top.
#[allow(clippy::too_many_arguments)]
pub(super) fn water(
    model: &rcsmodel::Model,
    variants: &[Option<rcsmaterial::Variant>],
    textures: Textures<'_>,
    skins: &mut TextureSlots,
    packed: &mut [u32],
    table: &mut Vec<Emissive>,
    material_tangent: &mut [TangentFrame],
    law: Water,
    report: &mut Report,
) -> Bound {
    let mut bound = Bound {
        normal_maps: vec![None; model.materials.len()],
        second_maps: vec![None; model.materials.len()],
    };
    if super::isolate::water_off() {
        return bound;
    }
    report.water_law = law;
    let mut black: Option<std::sync::Arc<ModelTexture>> = None;
    let mut decoded: std::collections::HashMap<String, Option<std::sync::Arc<ModelTexture>>> =
        Default::default();
    for (slot, material) in model.materials.iter().enumerate() {
        let Some(variant) = variants.get(slot).copied().flatten() else {
            continue;
        };
        let Some(blob) = textures(&format!("/{}", material.name)) else {
            continue;
        };
        let (Some(declared), Some(program)) = (
            rcsmaterial::Declared::parse(&blob, variant.fragment.offset),
            Program::parse(&blob, variant.fragment.offset),
        ) else {
            continue;
        };
        let Some(family) = family(&declared, &program) else {
            continue;
        };
        let chunks = model
            .meshes
            .iter()
            .filter(|mesh| usize::try_from(mesh.material).ok() == Some(slot))
            .count();
        let counter = match family {
            Family::Sea => &mut report.water_chunks.sea,
            Family::Sky => &mut report.water_chunks.sky,
            Family::LitColour => &mut report.water_chunks.lit_colour,
            Family::Glint => &mut report.water_chunks.glint,
        };
        *counter += chunks;
        let lit_vertex_colour = declared.parameters.contains(&CONSTANT_AMBIENT);
        // The picture is never the normal map, under either reading: lit vertex colour has
        // none (white), `water_noref` none but its reflection (black).
        if let Some(texture) = skins
            .get_mut(slot)
            .filter(|_| !matches!(family, Family::Sea | Family::Sky))
        {
            *texture = if lit_vertex_colour {
                None
            } else {
                Some(
                    black
                        .get_or_insert_with(|| {
                            std::sync::Arc::new(ModelTexture {
                                label: "water: no reflection, black".to_string(),
                                width: 1,
                                height: 1,
                                texels: Texels::Rgba8(vec![0, 0, 0, 255]),
                                mip_count: None,
                            })
                        })
                        .clone(),
                )
            };
        }
        if reading(law) == Reading::Taps {
            let mut map = |path: &str| {
                decoded
                    .entry(path.to_string())
                    .or_insert_with(|| {
                        textures(&format!("/{path}"))
                            .and_then(|bytes| decode_texture(path, &bytes))
                            .map(std::sync::Arc::new)
                    })
                    .clone()
            };
            let read = taps_of(material, family).and_then(|(taps, normal, second)| {
                let normal = map(&normal)?;
                let second = match second {
                    Some(path) => Some(map(&path)?),
                    None => None,
                };
                let lightmapped = declared
                    .samplers
                    .iter()
                    .any(|&(h, _)| h == rcsmaterial::LIGHTMAP_SAMPLER);
                Some((
                    entries(material, family, &taps, lightmapped)?,
                    normal,
                    second,
                ))
            });
            let Some((block, normal, second)) = read else {
                super::census::claim(slot, "water", super::census::Outcome::NotDrawn);
                report.water_taps_unread += 1;
                continue;
            };
            // A slot the generic reading gave an additive glow (the non-lightmapped sea adds
            // `DiffuseTexture1`, a normal map, `docs/rendering/water.md`, "What this engine
            // draws today") is the program's colour once its taps are drawn: the glow goes.
            // Any other claim keeps the slot.
            let Some(word) = packed.get_mut(slot) else {
                continue;
            };
            let held = slots::material_index(*word) as usize;
            let glow = held != 0
                && table
                    .get(held - 1)
                    .is_some_and(|entry| entry.program[0] == 0.0 && entry.rate <= 1.0);
            let index = (held == 0 || glow).then(|| lay(table, &block)).flatten();
            let Some(index) = index else {
                super::census::claim(slot, "water", super::census::Outcome::NotDrawn);
                report.water_taps_blocked += 1;
                continue;
            };
            if glow {
                *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
                report.water_glow_replaced += 1;
            }
            *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
            let drawn = Some((normal, second));
            if let Some((normal, second)) = drawn {
                bound.normal_maps[slot] = Some(normal);
                bound.second_maps[slot] = second;
                // The programs build their frame from the chunk's own tangent and
                // `B = cross(N, T) * w` (vertex block 3, `MUL o[TC0].xyz, N x T, v[2].w`).
                if let Some(frame) = material_tangent.get_mut(slot)
                    && !frame.carried()
                {
                    *frame = TangentFrame::Raw;
                }
                super::census::claim(slot, "water", super::census::Outcome::Drawn);
                report.water_taps_drawn += 1;
                continue;
            }
        }
        if matches!(family, Family::Sea | Family::Sky) {
            continue;
        }
        if lit_vertex_colour {
            // The program's own light is `ambient + sun * sat(N.L)` times the
            // vertex colour (`rcsmaterial.md`, "Water family"), where the
            // generic reading adds the vertex colour to the light and
            // multiplies a white picture by the sum: a pale sheet in place of
            // the authored teal. The ambient is the scene's own (`0.4` in the
            // capture, the circuit's `Constant ambient colour`): the material
            // authors no `constantAmbientColour`. The shader branch is
            // `slots::ICE` with [`WATER_FLAG`] in the entry's rate.
            if let Some(word) = packed.get_mut(slot) {
                let layer = Emissive {
                    tint: [0.0; 3],
                    offset: 0.0,
                    scale: 0.0,
                    rate: WATER_FLAG,
                    program: [0.0; 2],
                };
                let index = table.iter().position(|seen| *seen == layer).or_else(|| {
                    (table.len() + 1 < EMISSIVE_LIMIT).then(|| {
                        table.push(layer);
                        table.len() - 1
                    })
                });
                if let Some(index) = index {
                    *word &= slots::ROLE_MASK & !slots::ADD_SECOND;
                    *word |= slots::ICE;
                    *word |= u32::try_from(index + 1).unwrap_or(0) << slots::MATERIAL_SHIFT;
                }
            }
            super::census::claim(slot, "water", super::census::Outcome::NotDrawn);
            report.water_lit_colour += 1;
        } else {
            super::census::claim(slot, "water", super::census::Outcome::NotDrawn);
            report.water_glint_only += 1;
        }
    }
    bound
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sea's first tap scrolls `u` by `0.05` per second and leaves `v`: dropping the
    /// rate, or scrolling `v`, moves this point.
    #[test]
    fn the_first_sea_tap_is_u_plus_five_hundredths_of_time() {
        let [first, _] = Tap::sea();
        assert_eq!(first.at([0.25, 0.75], 2.0), [0.35, 0.75]);
        assert_eq!(first.at([0.25, 0.75], 0.0), [0.25, 0.75]);
    }

    /// The second tap is `1.15 (-u + 0.05 t, v)`: the opposite `u`, the same rate, then
    /// the scale on both axes. Each of the three dropped gives another coordinate.
    #[test]
    fn the_second_sea_tap_is_the_opposite_u_scaled_by_one_point_one_five() {
        let [_, second] = Tap::sea();
        let [u, v] = second.at([0.25, 0.75], 2.0);
        let want = [1.15 * (-0.25 + 0.05 * 2.0), 1.15 * 0.75];
        assert!(
            (u - want[0]).abs() < 1e-6 && (v - want[1]).abs() < 1e-6,
            "{u}, {v}"
        );
        // The two taps move the same way along `u` as time passes, the second 1.15 times as fast.
        let du = second.at([0.0, 0.0], 1.0)[0] - second.at([0.0, 0.0], 0.0)[0];
        assert!((du - 0.0575).abs() < 1e-6, "{du}");
        assert!(second.scale[0] < 0.0 && second.scale[1] > 0.0);
    }

    /// Vineta K's taps add `0.05 t * rate` to both axes of `uv * scale`.
    #[test]
    fn a_vineta_tap_scrolls_both_axes_by_its_rate() {
        let tap = Tap::vineta(2.0, 1.2914, 0.5);
        let [u, v] = tap.at([0.5, 0.25], 10.0);
        assert!((u - (1.0 + 0.645_7)).abs() < 1e-5 && (v - (0.5 + 0.645_7)).abs() < 1e-5);
        assert_eq!(tap.weight, 0.5);
    }

    /// `off`, `2048` and `omega` keep the constant reading; `original` and `hd` draw the taps.
    #[test]
    fn only_hd_s_laws_draw_the_taps() {
        for (law, taps) in [
            (Water::Off, false),
            (Water::Original, true),
            (Water::Hd, true),
            (Water::Vita2048, false),
            (Water::Omega, false),
        ] {
            assert_eq!(reading(law) == Reading::Taps, taps, "{law}");
        }
    }

    /// Omega's sea: both taps move `K * Speed` along `u` per second, the second with `u`
    /// negated, neither scaled (HD's second tap is `1.15` times; `reflectplane_mid` pixel
    /// `@0x7c`, `@0x84`), and the reflection's weight is
    /// `Strength * (Min + Scale * facing^Power)`.
    #[test]
    fn the_reflective_sea_is_two_opposite_unscaled_taps_and_a_fresnel_weight() {
        let [first, second] = Tap::reflective(0.05, [1.0, -1.0], 0.5);
        assert_eq!(first.at([0.25, 0.75], 2.0), [0.25 + 0.05, 0.75]);
        assert_eq!(second.at([0.25, 0.75], 2.0), [-0.25 + 0.05, 0.75]);
        assert_eq!((first.weight, second.weight), (1.0, 1.0));
        assert_eq!((first.scale, second.scale), ([1.0, 1.0], [-1.0, 1.0]));
        let fresnel = Fresnel {
            min: 0.15,
            scale: 2.0,
            power: 7.0,
            strength: 0.7,
        };
        assert_eq!(fresnel.weight(0.0), 0.7 * 0.15);
        assert_eq!(fresnel.weight(1.0), 0.7 * (0.15 + 2.0));
        assert_eq!(fresnel.weight(0.5), 0.7 * (0.15 + 2.0 * 0.5_f32.powf(7.0)));
        assert_eq!(fresnel.weight(3.0), fresnel.weight(1.0));
        // The entry: `sea_block`'s lanes untouched, the weight's two products and the power
        // in the lanes the tap entries leave free.
        let block = reflective_block(
            OMEGA_EDGE_KIND,
            [0.1, 0.2, 0.3],
            0.9,
            250.0,
            [0.4, 0.5, 0.6],
            [&first, &second],
            fresnel,
        );
        let plain = sea_block(
            OMEGA_EDGE_KIND,
            [0.1, 0.2, 0.3],
            0.9,
            250.0,
            [0.4, 0.5, 0.6],
            [&first, &second],
        );
        assert_eq!(block[0], plain[0]);
        assert_eq!(block[0].program, [PROGRAM_WATER, 10.0]);
        assert_eq!((block[0].offset, block[0].scale), (0.9, 250.0));
        assert_eq!(block[1].tint, [1.0, 1.0, 0.05 * 0.5]);
        assert_eq!(block[2].tint, [-1.0, 1.0, 0.05 * 0.5]);
        assert_eq!(block[1].rate, 0.7 * 0.15);
        assert_eq!(block[2].rate, 0.7 * 2.0);
        assert_eq!(block[1].program, [0.4, 0.5]);
        assert_eq!(block[2].program, [0.6, 7.0]);
    }

    /// A tap's entry carries its numbers in the order `water.wesl` reads them.
    #[test]
    fn a_tap_entry_is_scale_then_scroll() {
        let material = rcsmodel::Material {
            name: "sea.rcsmaterial".to_string(),
            state: 1,
            src_factor: 0,
            dst_factor: 0,
            alpha_func: 0,
            alpha_ref: 0.0,
            texture: String::new(),
            second_texture: None,
            texture_sampler: 0,
            second_texture_sampler: None,
            samplers: Vec::new(),
            parameters: vec![
                param(COLOUR_CONSTANT, [0.08, 0.12, 0.13, 0.0]),
                param(SPECULAR_COLOUR, [0.5, 0.5, 0.3, 0.0]),
                param(SPECULAR_POWER, [75.0, 0.0, 0.0, 0.0]),
                param(REFLECTIVITY, [0.3, 0.0, 0.0, 0.0]),
            ],
            curve: None,
        };
        let block = entries(&material, Family::Sea, &Tap::sea(), true).expect("authored");
        assert_eq!(block.len(), 3);
        assert_eq!(block[0].program, [PROGRAM_WATER, 1.0]);
        assert_eq!((block[0].tint, block[0].scale), ([0.08, 0.12, 0.13], 75.0));
        assert_eq!(
            block[0].offset, 0.3,
            "the sea weights its reflection by Reflectivity"
        );
        assert_eq!((block[1].program, block[2].program[0]), ([0.5, 0.5], 0.3));
        assert_eq!(
            (block[2].tint, block[2].offset, block[2].scale),
            ([-1.15, 1.15, 1.15 * 0.05], 0.0, 1.0)
        );
        let vineta = [
            Tap::vineta(1.0, 1.0, 0.5),
            Tap::vineta(2.0, 2.0, 0.5),
            Tap::vineta(4.0, 1.2914, 1.0),
        ];
        let block = entries(&material, Family::LitColour, &vineta, false).expect("three taps");
        assert_eq!(block.len(), 3);
        assert_eq!(block[0].tint, [0.5, 0.5, 1.0]);
        assert_eq!(block[1].tint, [1.0, 2.0, 4.0]);
        assert_eq!(block[2].tint, [0.05, 0.1, 0.05 * 1.2914]);
    }

    /// 2048's sea scrolls `u` by `0.05 t` through one tap; a second of weight zero keeps the
    /// layout. Change the literal or the weight and the program's law is no longer drawn.
    #[test]
    fn the_vita_sea_is_one_tap_at_u_plus_a_twentieth_of_time() {
        let [tap, none] = Tap::vita_sea();
        assert_eq!(tap.at([0.25, 0.5], 10.0), [0.25 + 0.5, 0.5]);
        assert_eq!((tap.weight, none.weight), (1.0, 0.0));
        assert_eq!(SEA_SCROLL, 0.05);
    }

    /// `fc17` scrolls `v` by `Speed * t` and leaves `u` alone (primary 34 adds the product to
    /// `uv1`'s second lane); fc15 scrolls `u` by the literal. Swap the axes and the sheet drifts
    /// the wrong way.
    #[test]
    fn the_fc17_water_is_one_tap_at_v_plus_speed_times_time() {
        let [tap, none] = Tap::vita_water(0.05062);
        let at = tap.at([0.25, 0.5], 10.0);
        assert_eq!(at[0], 0.25);
        assert!((at[1] - (0.5 + 0.5062)).abs() < 1e-6, "{at:?}");
        assert_eq!((tap.weight, none.weight), (1.0, 0.0));
    }

    fn param(hash: u32, value: [f32; 4]) -> oag_rcs::rcsmodel::material::Parameter {
        oag_rcs::rcsmodel::material::Parameter {
            hash,
            value,
            quads: 1,
        }
    }
}
