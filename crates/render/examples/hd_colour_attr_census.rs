//! Disc-wide count of every four-normalised-byte vertex attribute, by hash -
//! the population `VertexDecl::vertex_colour()`'s doc comment measured only
//! over Talon's Junction, before `VertexColour1` (`0x7493d450`) turned up on
//! ship files. Walks every `.rcsmodel` reachable from every archive's own
//! manifest.
use oag_assets::psarc;
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

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut counts: std::collections::BTreeMap<u32, usize> = Default::default();
    let mut sample: std::collections::HashMap<u32, rcsmodel::Attribute> = Default::default();
    let mut vertex_colour1_files: std::collections::BTreeSet<String> = Default::default();
    let mut files = 0usize;
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
        for path in paths {
            let Ok(data) = psarc.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&data) else {
                continue;
            };
            files += 1;
            for mesh in &model.meshes {
                let Some(decl) = mesh.decl.as_ref() else {
                    continue;
                };
                for a in &decl.attributes {
                    if a.components == 4 && a.rsx_type == rcsmodel::vertex_decl::RSX_UBYTE_NORM {
                        *counts.entry(a.name_hash).or_default() += 1;
                        sample.entry(a.name_hash).or_insert(*a);
                        if a.name_hash == 0x7493_d450 {
                            vertex_colour1_files.insert(path.clone());
                        }
                    }
                }
            }
        }
    }
    println!("{files} .rcsmodel file(s) parsed");
    for (hash, count) in &counts {
        let name = sample.get(hash).and_then(rcsmodel::Attribute::name);
        println!("  {hash:#010x} ({name:?}): {count} chunk attribute(s)");
    }
    println!(
        "VertexColour1 (0x7493d450) appears in {} file(s):",
        vertex_colour1_files.len()
    );
    for path in &vertex_colour1_files {
        println!("  {path}");
    }
    Ok(())
}
