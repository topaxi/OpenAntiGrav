//! Preimages for `.rcsmodel` parameter-name hashes.
//!
//! The technique that named `zoneTexInner` and `paraboloidReflectionTex`,
//! pointed at the *parameter* table rather than the sampler one: hash a corpus
//! of candidate identifiers and match against the hashes the disc's material
//! records actually carry. The corpus is whatever the caller points at -
//! `EBOOT.elf`'s own strings when it has been decrypted (see
//! `docs/reverse-engineering/toolchain.md`), and the `.rcsmaterial` files
//! otherwise, which carry the exporter's own token vocabulary.
//!
//! ```sh
//! cargo run -p oag-render --example hd_param_names
//! cargo run -p oag-render --example hd_param_names -- <image> path/to/EBOOT.elf
//! ```

use oag_formats::rcsmaterial::name_hash;

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut names: std::collections::HashMap<u32, String> = std::collections::HashMap::new();
    let mut add = |text: &str| {
        names.entry(name_hash(text)).or_insert_with(|| text.into());
    };
    for corpus in std::env::args().skip(2) {
        let bytes = std::fs::read(&corpus)?;
        let mut found = 0usize;
        for run in bytes.split(|&b| b == 0) {
            if !(3..=64).contains(&run.len())
                || !run
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'.')
            {
                continue;
            }
            if let Ok(text) = std::str::from_utf8(run) {
                add(text);
                found += 1;
            }
        }
        println!("{corpus}: {found} identifier-shaped strings");
    }

    // Every parameter hash the disc's material records carry, with how often it
    // appears and which materials use it.
    let mut uses: std::collections::BTreeMap<u32, (usize, Vec<String>)> =
        std::collections::BTreeMap::new();
    let mut sho_strings = 0usize;
    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel") || p.ends_with(".rcsmaterial"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            if path.ends_with(".rcsmaterial") {
                // The exporter's own token vocabulary, in the clear inside the
                // shader container - the corpus that is on the disc rather than
                // in the executable.
                for run in blob.split(|&b| b == 0) {
                    if (3..=64).contains(&run.len())
                        && run.iter().all(|b| b.is_ascii_alphanumeric() || *b == b'_')
                        && let Ok(text) = std::str::from_utf8(run)
                    {
                        add(text);
                        sho_strings += 1;
                    }
                }
                continue;
            }
            let Ok(model) = oag_formats::rcsmodel::Model::parse(&blob) else {
                continue;
            };
            for material in &model.materials {
                let short = material
                    .name
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(&material.name)
                    .to_string();
                for p in &material.parameters {
                    let row = uses.entry(p.hash).or_default();
                    row.0 += 1;
                    if row.1.len() < 4 && !row.1.contains(&short) {
                        row.1.push(short.clone());
                    }
                }
            }
        }
    }
    println!(
        "{sho_strings} strings in the .rcsmaterial files, {} distinct",
        names.len()
    );

    let mut named = 0usize;
    for (hash, (count, materials)) in &uses {
        let name = names.get(hash);
        named += usize::from(name.is_some());
        println!(
            "{hash:#010x} x{count:<6} {:<32} {materials:?}",
            name.map_or("-", String::as_str)
        );
    }
    println!(
        "\n{named} of {} parameter hashes have a preimage",
        uses.len()
    );
    Ok(())
}
