//! Disc-wide census of every `ADD_SECOND` hit `mesh::rcs::emissive::emissive`
//! produces **today** (before any fix in this lane), plus - for each one -
//! the declared sampler hash of the texture actually loaded as the second
//! one (`Model::lightmaps[slot]`, whose `label` is the path the material's
//! own sampler table names it under) and what that hash's own role is.
//!
//! **Why by the loaded texture's own path, not by "whatever hash the
//! declaration says sits at hardware unit 1".** That was tried first and
//! rejected by measurement: `tunnel_fx_noalpha` declares its *specular* map
//! at hardware unit 1 and its *emissive* map at unit 2, while the second
//! texture this renderer actually loads and adds - `Pick::aux`'s own choice,
//! which `skin::skin` decodes into `Model::lightmaps[slot]` - is the
//! emissive one at ordinal 1, not whatever sits at unit 1. Resolving "unit 1"
//! independent of which texture is actually bound would have refused a real,
//! working glow by inspecting the wrong sampler. Matching on the *loaded*
//! texture's own path against `material.samplers` finds the entry (and
//! therefore the hash) that was actually decoded, whatever unit it declares.
//!
//! This is the falsifier `docs/rendering/hd-ship-materials.md` asks for: a
//! correct fix must keep every circuit glow that is a real glow (a `Light`
//! preimage - `EmissiveTexture`/`Emissive`/`emissive`/`ReflectionMap`/
//! `EnvMap`/`EnvMap1`) and drop every hull surface whose added texture is a
//! normal or specular map (the `Surface maps` preimage group -
//! `NormalTexture`/`NormalTexture2`/`NormalMap`/`Normal`/`SpecularTexture`/
//! `SpecMap`/`Spec` - plus four further, per-material-family hashes that
//! never made the disc-wide preimage sweep, identified here the same way
//! `skin::NOT_A_PICTURE` identifies its own entries: by what they bind,
//! disc-wide, with no counter-example).
//!
//! ```sh
//! cargo run -p oag-render --example hd_emissive_role_census
//! ```
use oag_mesh::mesh::{self, slots};
use oag_rcs::rcsmodel;

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// `NormalTexture` `NormalTexture2` `NormalMap` `Normal` `SpecularTexture`
    /// `SpecMap` `Spec` (named, disc-wide preimages), plus four unnamed
    /// per-family hashes measured here: every one of 73 distinct paths they
    /// bind, disc-wide, across ships and weapons, is a normal map by name -
    /// see this file's own `SHIP_SURFACE_MAP_SAMPLERS` doc comment.
    SurfaceMap,
    /// `EmissiveTexture` `Emissive` `emissive` `ReflectionMap` `EnvMap`
    /// `EnvMap1` - the `Light` group minus `lightmap`/`shadowMapTex`, which
    /// `emissive()` already refuses on a separate, earlier check.
    Glow,
}

/// Named, disc-wide "Surface maps" preimages, **narrowed from all seven** by
/// an independent disc-wide bind census (`hd_zz_surfacemap_bind_census`
/// scratch, folded into this comment): a preimage match alone is not
/// evidence the hash is *never* a picture, and three of the seven have a
/// real counter-example bound to them somewhere on the disc -
///
/// | Hash | Name | Distinct paths | Counter-example |
/// | --- | --- | ---: | --- |
/// | `0x739a786e` | `NormalTexture` | 28 | none (already `skin::NOT_A_PICTURE`) |
/// | `0x62ae87a7` | `NormalTexture2` | 1 | none |
/// | `0xd9d6922d` | `Normal` | 30 | none |
/// | `0x48f37f5a` | `NormalMap` | 10 | `medal_dissolvehexnormal.gtf` - "normal" may name the hex pattern, not a lighting normal |
/// | `0x20c3e476` | `SpecularTexture` | 108 | `and_power_glow.gtf`, `and_station4_diff.gtf`, `biodome_bar_colour.gtf` - a glow, a diffuse and a colour texture, disc-wide |
/// | `0x576c4bf3` | `SpecMap` | 3 | `startdarksideswatch.gtf` - an advert, confirmed live: `scanlinetext` binds it at Talon's Junction and would lose a real glow if this hash refused |
/// | `0x9fc347ff` | `Spec` | 1 | `detonator_ao.gtf` - an ambient-occlusion map, not specular |
///
/// Excluding the bottom four would repeat the regression the doc already
/// measured once (`docs/rendering/hd-ship-materials.md`, "two fixes tried and
/// both failed") - a plausible-sounding preimage name is not itself a
/// disc-measured role. Only the top three carry zero counter-examples and are
/// kept.
const NAMED_SURFACE_MAP: &[u32] = &[
    0x739a_786e, // NormalTexture
    0x62ae_87a7, // NormalTexture2
    0xd9d6_922d, // Normal
];

/// Per-material-family sampler hashes with no preimage, each measured
/// disc-wide (`hd_zz_hash_bind_check` scratch, folded in here) to bind
/// **only** a normal map, with zero exceptions:
///
/// | Hash | Family | Distinct paths | Every one |
/// | --- | --- | ---: | --- |
/// | `0x436d3929` | `diffuse_with_specular_from_alpha_n_vcol`/`_n`/`detonator_*` | 45 | `*_n.gtf`/`*_norm*.gtf`/`*normal*.gtf` |
/// | `0xc8f18561` | `carbonfibre` | 11 | `carbon_n.gtf` or `detonator_n.gtf` |
/// | `0x0617f872` | `nitro_body_new` | 13 | `*_n.gtf`/`*_norm*.gtf`, or `blank_normals.gtf` |
/// | `0xc78c9866` | `detonator_ship_dg_iridescent` | 4 | `detonator_n.gtf`, or a weapon's own `*_n_*.gtf` |
const SHIP_SURFACE_MAP_SAMPLERS: &[u32] = &[0x436d_3929, 0xc8f1_8561, 0x0617_f872, 0xc78c_9866];

/// The same bar on circuit scenery, keyed by (family, hash) - mirrors
/// `emissive.rs`'s `CIRCUIT_SURFACE_MAP_SAMPLERS`, which carries the
/// measured rows.
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

/// Named, disc-wide "Light" preimages minus `lightmap`/`shadowMapTex`.
const NAMED_GLOW: &[u32] = &[
    0xb1f2_a176, // EmissiveTexture
    0xfa79_b1cd, // Emissive
    0x030f_d39b, // emissive
    0xb160_0426, // ReflectionMap
    0x2d5f_c6a6, // EnvMap
    0x6e46_5921, // EnvMap1
];

fn sampler_role(family: &str, hash: u32) -> Option<Role> {
    if NAMED_SURFACE_MAP.contains(&hash)
        || SHIP_SURFACE_MAP_SAMPLERS.contains(&hash)
        || CIRCUIT_SURFACE_MAP_SAMPLERS.contains(&(family, hash))
    {
        Some(Role::SurfaceMap)
    } else if NAMED_GLOW.contains(&hash) {
        Some(Role::Glow)
    } else {
        None
    }
}

/// Every `/data/ships/<team>/ship.vex` and every circuit's `track.vex`/
/// `track_reversed.vex`, found by listing each archive's own manifest.
fn every_vex(image: &str) -> Vec<String> {
    let mut found = std::collections::BTreeSet::new();
    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        for path in psarc.paths() {
            let is_ship = path.starts_with("/data/ships/")
                && path.ends_with("/ship.vex")
                && path.matches('/').count() == 4;
            let is_track = path.starts_with("/data/environments/")
                && (path.ends_with("/track.vex") || path.ends_with("/track_reversed.vex"))
                && path.matches('/').count() == 4;
            if is_ship || is_track {
                found.insert(path.clone());
            }
        }
    }
    found.into_iter().collect()
}

#[derive(Default)]
struct Bucket {
    total: usize,
    surface_map: usize,
    glow: usize,
    unresolved: usize,
    names: std::collections::BTreeMap<String, usize>,
}

impl Bucket {
    fn record(&mut self, family: &str) {
        self.total += 1;
        *self.names.entry(family.to_string()).or_default() += 1;
    }
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let vex_paths = every_vex(&image);

    let mut ships = Bucket::default();
    let mut circuits = Bucket::default();

    for vex_path in &vex_paths {
        let Some((spec, data)) = ARCHIVES.iter().find_map(|archive| {
            let spec = format!("{image}:{archive}");
            mesh::read_blob(&spec, vex_path).ok().map(|d| (spec, d))
        }) else {
            continue;
        };
        let Some(geometry) = mesh::rcs::sibling_geometry(&spec, vex_path, &data) else {
            continue;
        };
        let Ok(geometry_model) = rcsmodel::Model::parse(&geometry) else {
            continue;
        };
        let mut cache: std::collections::HashMap<String, Option<Vec<u8>>> = Default::default();
        let mut textures = |name: &str| -> Option<Vec<u8>> {
            if let Some(hit) = cache.get(name) {
                return hit.clone();
            }
            let found = ARCHIVES.iter().find_map(|archive| {
                let spec = format!("{image}:{archive}");
                mesh::read_blob(&spec, name).ok()
            });
            cache.insert(name.to_string(), found.clone());
            found
        };
        let Ok((model, _report)) = mesh::rcs::build_scene(&spec, &data, &geometry, &mut textures)
        else {
            continue;
        };

        let bucket = if vex_path.starts_with("/data/ships/") {
            &mut ships
        } else {
            &mut circuits
        };

        for (slot, packed) in model.material_slots.iter().enumerate() {
            if packed & slots::ADD_SECOND == 0 {
                continue;
            }
            let Some(material) = geometry_model.materials.get(slot) else {
                continue;
            };
            // The texture actually loaded as the second one - `Pick::aux`'s
            // own choice, decoded by `skin::skin` into this same slot of
            // `Model::lightmaps`. Its `label` is the path it was decoded
            // from, which is how this finds which entry of the material's
            // own sampler table it came from without re-deriving `Pick`.
            let Some(Some(second)) = model.lightmaps.get(slot) else {
                continue;
            };
            let hash = material
                .samplers
                .iter()
                .find(|(_, p)| p.as_deref() == Some(second.label.as_str()))
                .map(|&(h, _)| h);
            let family = material
                .name
                .rsplit('/')
                .next()
                .unwrap_or(&material.name)
                .trim_end_matches(".rcsmaterial");
            bucket.record(family);
            match hash.and_then(|h| sampler_role(family, h)) {
                Some(Role::SurfaceMap) => bucket.surface_map += 1,
                Some(Role::Glow) => bucket.glow += 1,
                None => bucket.unresolved += 1,
            }
            if std::env::var("OAG_VERBOSE").is_ok()
                || bucket.total <= 5
                || hash.and_then(|h| sampler_role(family, h)) == Some(Role::SurfaceMap)
            {
                println!(
                    "  {} [{slot}] second={} hash={} ({})",
                    material.name,
                    second.label,
                    hash.map_or("none".to_string(), |h| format!("{h:#010x}")),
                    hash.and_then(|h| sampler_role(family, h))
                        .map_or("unresolved".to_string(), |r| format!("{r:?}")),
                );
            }
        }
    }

    for (label, bucket) in [("ships", &ships), ("circuits", &circuits)] {
        println!(
            "\n{label}: {} ADD_SECOND slot(s) today - {} resolve to a SurfaceMap sampler, \
             {} resolve to a named Glow sampler, {} unresolved (kept on the current path)",
            bucket.total, bucket.surface_map, bucket.glow, bucket.unresolved
        );
        for (family, count) in &bucket.names {
            println!("  {count:>4}  {family}");
        }
    }
    println!(
        "\ntotal: {} ADD_SECOND slot(s) disc-wide ({} ships + {} circuits)",
        ships.total + circuits.total,
        ships.total,
        circuits.total
    );
    Ok(())
}
