//! Scratch probe: every `.pvs` on the disc, against the `.rcsmodel` beside it.
//!
//! Reports the header, whether the file is exactly
//! `16 + cells*16 + cells*ceil(chunks/8)` bytes, and whether every bitmap's
//! last byte carries only the bits `chunks` leaves valid - the two tests that
//! say the layout is read right rather than merely plausible.

use oag_rcs::rcsmodel;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    for archive_index in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC");
        let Ok(mut psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".pvs"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = psarc.read_path(&path) else {
                continue;
            };
            if blob.len() < 16 {
                println!("{path}: {} bytes, too short", blob.len());
                continue;
            }
            let word = |i: usize| u32::from_be_bytes(blob[i * 4..i * 4 + 4].try_into().unwrap());
            let (cells, chunks, third, fourth) = (word(0), word(1), word(2), word(3));
            let model_path = path.replace(".pvs", ".rcsmodel");
            let model_chunks = psarc
                .read_path(&model_path)
                .ok()
                .and_then(|b| rcsmodel::Model::parse(&b).ok())
                .map(|m| m.meshes.len());
            let width = (chunks as usize).div_ceil(8);
            let head = 16 + cells as usize * 16;
            let exact = head + cells as usize * width == blob.len();
            let extra = blob.len() as isize - (head + cells as usize * width) as isize;
            let spare = (chunks as usize) % 8;
            let mask = if spare == 0 {
                0xffu8
            } else {
                (1u8 << spare) - 1
            };
            let padding_clean = (0..cells as usize).all(|c| {
                let at = head + c * width + width - 1;
                at < blob.len() && blob[at] & !mask == 0
            });
            println!(
                "{path}: {} B  cells {cells} chunks {chunks} ({}) third {third} fourth {fourth:#010x}  exact {exact} extra {extra:+}  padding_clean {padding_clean}",
                blob.len(),
                match model_chunks {
                    Some(n) if n == chunks as usize => "matches model".into(),
                    Some(n) => format!("model has {n}"),
                    None => "no model".into(),
                },
            );
        }
    }
    Ok(())
}
