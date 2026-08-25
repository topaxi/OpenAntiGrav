//! Scratch probe: every stride the disc's own vertex declarations carry, and
//! whether the search agrees with them where both answer.
//!
//! `rcsmodel::STRIDES` is the candidate list the three stride searches try. A
//! width missing from it is a chunk no search can ever solve, and the
//! declaration is the oracle that says which widths exist.

use oag_formats::rcsmodel;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let mut declared: std::collections::BTreeMap<usize, usize> = Default::default();
    let mut agree: std::collections::BTreeMap<(usize, Option<usize>), usize> = Default::default();
    for archive_index in 0..7 {
        let spec = format!("{image}:PS3_GAME/USRDIR/DATA{archive_index:02}.PSARC");
        let Ok(mut psarc) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = psarc
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = psarc.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            for chunk in &model.meshes {
                for surface in chunk.surfaces() {
                    let Some(truth) = surface.declared_stride() else {
                        continue;
                    };
                    *declared.entry(truth).or_default() += 1;
                    let solved = surface.solve_stride_without_a_box(&blob);
                    *agree.entry((truth, solved)).or_default() += 1;
                }
            }
        }
    }
    println!("strides the disc's declarations carry: {declared:?}");
    println!(
        "candidates the search tries:           {:?}",
        rcsmodel::STRIDES
    );
    let (mut right, mut wrong, mut silent) = (0usize, 0usize, 0usize);
    for ((truth, solved), count) in &agree {
        match solved {
            Some(s) if s == truth => right += count,
            Some(_) => {
                wrong += count;
                println!("  declared {truth}, search says {solved:?}  x{count}");
            }
            None => silent += count,
        }
    }
    println!(
        "against the declaration: {right} agree, {wrong} disagree, {silent} no answer \
         ({:.2}% agreement where the search answers)",
        right as f64 / (right + wrong).max(1) as f64 * 100.0,
    );
    Ok(())
}
