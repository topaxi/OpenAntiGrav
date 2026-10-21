//! Census: which resolved lit-race fragment programs light the albedo's
//! **alpha** (`Program::lit_albedo` is `Alpha`), which light its colour, and
//! which this reading cannot follow - per material family, with chunk counts,
//! over every `.rcsmodel` on the disc. The disc-wide reach of
//! `mesh::slots::LIT_ALPHA` (`docs/rendering/hd-track-tone.md`, "The blue cast").
//!
//! ```sh
//! cargo run -p oag-render --example hd_lit_albedo_census [image]
//! ```
use oag_assets::psarc;
use oag_rcs::rcsmaterial::{self, fragment::LitAlbedo};
use oag_rcs::rcsmodel;
use std::collections::BTreeMap;

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

#[derive(Default)]
struct Tally {
    alpha: usize,
    colour: usize,
    unread: usize,
    /// Albedo textures lit through their alpha, by file stem.
    alpha_textures: BTreeMap<String, usize>,
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let mut families: BTreeMap<String, Tally> = BTreeMap::new();
    let (mut models, mut chunks) = (0usize, 0usize);
    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(mut psarc) = psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        let mut blobs: BTreeMap<String, Option<Vec<u8>>> = BTreeMap::new();
        let mut programs: BTreeMap<(String, usize), Option<LitAlbedo>> = BTreeMap::new();
        for path in paths {
            let Ok(data) = psarc.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&data) else {
                continue;
            };
            models += 1;
            for mesh in model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces) {
                let Some(decl) = mesh.decl.as_ref() else {
                    continue;
                };
                let Some(material) = model.materials.get(mesh.material as usize) else {
                    continue;
                };
                let blob = blobs
                    .entry(material.name.clone())
                    .or_insert_with(|| psarc.read_path(&format!("/{}", material.name)).ok())
                    .clone();
                let Some(blob) = blob else { continue };
                let Ok(parsed) = rcsmaterial::RcsMaterial::parse(&blob) else {
                    continue;
                };
                let word =
                    rcsmaterial::Features::chunk_word(rcsmaterial::LIT_RACE_PASS, Some(decl));
                let key = rcsmaterial::Features::from_pass_word(word);
                let Some(variant) = parsed.variant(rcsmaterial::Class::Static, key) else {
                    continue;
                };
                chunks += 1;
                let family = material
                    .name
                    .rsplit('/')
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(".rcsmaterial")
                    .to_string();
                let read = *programs
                    .entry((material.name.clone(), variant.fragment.offset))
                    .or_insert_with(|| {
                        let program =
                            rcsmaterial::fragment::Program::parse(&blob, variant.fragment.offset)?;
                        // The unit the slot's first texture reaches, by its sampler hash,
                        // as `mesh::rcs::skin::units` reads it; unit 0 where undeclared.
                        let unit = program
                            .declared
                            .samplers
                            .iter()
                            .find(|(hash, _)| *hash == material.texture_sampler)
                            .map_or(0, |(_, unit)| *unit);
                        program.lit_albedo(u8::try_from(unit).ok()?)
                    });
                let tally = families.entry(family).or_default();
                match read {
                    Some(LitAlbedo::Alpha) => {
                        tally.alpha += 1;
                        let stem = material
                            .texture
                            .rsplit('/')
                            .next()
                            .unwrap_or("")
                            .to_string();
                        *tally.alpha_textures.entry(stem).or_default() += 1;
                    }
                    Some(LitAlbedo::Colour) => tally.colour += 1,
                    None => tally.unread += 1,
                }
            }
        }
    }
    let (mut alpha, mut colour, mut unread) = (0, 0, 0);
    println!("{models} model(s), {chunks} resolved chunk(s)");
    for (family, t) in &families {
        alpha += t.alpha;
        colour += t.colour;
        unread += t.unread;
        if t.alpha > 0 {
            println!(
                "ALPHA  {family}: {} chunk(s) alpha, {} colour, {} unread; textures {:?}",
                t.alpha, t.colour, t.unread, t.alpha_textures
            );
        }
    }
    println!(
        "chunks: {alpha} alpha, {colour} colour, {unread} unread; {} families",
        families.len()
    );
    Ok(())
}
