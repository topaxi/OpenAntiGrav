//! Scratch: list every variant a `.rcsmaterial` ships, with the permutation word each feature hash decodes to.
//!
//! ```sh
//! cargo run -p oag-render --example hd_material_variants -- data/images/hdfury-ps3-eu-dec.iso /data/materials/ships/diffuse_vcol.rcsmaterial
//! ```
//!
//! One line per variant: class, feature name, fragment block offset and length. A live
//! RPCS3 draw's program is matched to a block offset by opcode stream
//! (`docs/reverse-engineering/rpcs3-capture.md`), and this names the feature set that block serves.
use oag_mesh::mesh;
use oag_rcs::rcsmaterial::{self, Features, PASS_WORD_BITS};

const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (image, path) = (&args[0], &args[1]);
    let blob = ARCHIVES
        .iter()
        .find_map(|a| mesh::read_blob(&format!("{image}:{a}"), path).ok())
        .expect("material not found");
    let parsed = rcsmaterial::RcsMaterial::parse(&blob).expect("parse");
    let names: std::collections::BTreeMap<u32, (u32, String)> = (0..(1u32 << PASS_WORD_BITS))
        .map(|w| {
            let f = Features::from_pass_word(w);
            (f.hash(), (w, f.name()))
        })
        .collect();
    for (i, v) in parsed.variants.iter().enumerate() {
        let (word, name) = names
            .get(&v.feature_hash)
            .map(|(w, n)| (format!("{w:#05x}"), n.as_str()))
            .unwrap_or_else(|| ("?".into(), "(unknown feature hash)"));
        println!(
            "{i:3} {:?} word {word} {name}  vertex@{:#x} fragment@{:#x}+{:#x}",
            v.class, v.vertex.offset, v.fragment.offset, v.fragment.len
        );
    }
}
