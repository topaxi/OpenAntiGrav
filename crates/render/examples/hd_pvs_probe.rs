//! Scratch probe: what a circuit's `track.pvs` hides at one world position.
//!
//! The question it exists to answer is "did the visibility set take that
//! wall?". A chunk's *bounding sphere* is a poor judge of that on a circuit
//! whose chunks are hundreds of units wide, so this reports the distance to
//! each chunk's axis-aligned box, which for a slab is the distance to the slab.
//!
//! `cargo run --release -p oag-render --example hd_pvs_probe -- \
//!     <image> <archive> <track.vex> [x y z]`
//!
//! With no position it **sweeps every cell** and reports the closest thing
//! each one hides - which is how "a wall disappears as I approach it" becomes
//! a number rather than an impression.

use oag_rcs::{hd_pvs, rcsmodel};
use oag_render::mesh;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let image = args
        .next()
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let archive = args.next().unwrap_or_else(|| "DATA02".into());
    let name = args
        .next()
        .unwrap_or_else(|| "/data/environments/15_anulpha_pass/track.vex".into());
    let point: [f32; 3] =
        std::array::from_fn(|_| args.next().and_then(|s| s.parse().ok()).unwrap_or(f32::NAN));

    let spec = format!("{image}:PS3_GAME/USRDIR/{archive}.PSARC");
    let model_blob = mesh::read_blob(&spec, &mesh::rcs::sibling_name(&name).unwrap())?;
    let model = rcsmodel::Model::parse(&model_blob).map_err(|e| anyhow::anyhow!("{e}"))?;
    let pvs_blob = mesh::read_blob(&spec, &name.replace(".vex", ".pvs"))?;
    let pvs = hd_pvs::Pvs::parse(&pvs_blob).map_err(|e| anyhow::anyhow!("{e}"))?;

    let boxes: Vec<Option<([f32; 3], [f32; 3])>> = model
        .meshes
        .iter()
        .map(|chunk| {
            let stride = chunk
                .declared_stride()
                .or_else(|| chunk.solve_stride_without_a_box(&model_blob))?;
            let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
            for submesh in &chunk.submeshes {
                if let Ok(positions) = chunk.positions(&model_blob, submesh, stride) {
                    for p in positions {
                        for i in 0..3 {
                            min[i] = min[i].min(p[i]);
                            max[i] = max[i].max(p[i]);
                        }
                    }
                }
            }
            (min[0] <= max[0]).then_some((min, max))
        })
        .collect();
    let gap_to = |b: &([f32; 3], [f32; 3]), p: [f32; 3]| -> f32 {
        (0..3)
            .map(|i| (b.0[i] - p[i]).max(p[i] - b.1[i]).max(0.0).powi(2))
            .sum::<f32>()
            .sqrt()
    };

    if point.iter().any(|v| v.is_nan()) {
        println!(
            "{} cell(s) over {} chunk(s); the closest thing each cell hides:",
            pvs.cells(),
            pvs.chunks()
        );
        let mut worst: Vec<(f32, usize, usize)> = Vec::new();
        for cell in 0..pvs.cells() {
            let at = pvs.position(cell).unwrap();
            let mut best = (f32::INFINITY, usize::MAX);
            for (index, b) in boxes.iter().enumerate() {
                let Some(b) = b else { continue };
                if pvs.visible(cell, index) {
                    continue;
                }
                let gap = gap_to(b, at);
                if gap < best.0 {
                    best = (gap, index);
                }
            }
            worst.push((best.0, cell, best.1));
        }
        worst.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let mut gaps: Vec<f32> = worst.iter().map(|w| w.0).collect();
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        println!(
            "  min {:.2}  p10 {:.2}  median {:.2}  max {:.2}",
            gaps[0],
            gaps[gaps.len() / 10],
            gaps[gaps.len() / 2],
            gaps[gaps.len() - 1],
        );
        println!("  the 15 cells that hide something closest:");
        for (gap, cell, index) in worst.iter().take(15) {
            let at = pvs.position(*cell).unwrap();
            let material = model
                .meshes
                .get(*index)
                .and_then(|c| model.material_of(c))
                .map(|m| {
                    m.name
                        .rsplit('/')
                        .next()
                        .unwrap_or("?")
                        .trim_end_matches(".rcsmaterial")
                        .to_string()
                })
                .unwrap_or_else(|| "?".into());
            println!(
                "    cell {cell:4} at [{:8.1} {:8.1} {:8.1}] hides chunk {index:5} at {gap:7.2}  {material}",
                at[0], at[1], at[2],
            );
        }
        return Ok(());
    }

    let cell = pvs.nearest_cell(point).expect("a non-empty partition");
    let at = pvs.position(cell).unwrap();
    println!(
        "{} cell(s) over {} chunk(s), {} byte(s) per bitmap; nearest cell {cell} at [{:.1} {:.1} {:.1}], {:.1} units away",
        pvs.cells(),
        pvs.chunks(),
        pvs.bitmap_bytes(),
        at[0],
        at[1],
        at[2],
        ((at[0] - point[0]).powi(2) + (at[1] - point[1]).powi(2) + (at[2] - point[2]).powi(2))
            .sqrt(),
    );
    println!(
        "that cell draws {} of {} chunk(s) ({:.1}%)",
        pvs.visible_count(cell),
        pvs.chunks(),
        f64::from(pvs.visible_count(cell)) / pvs.chunks() as f64 * 100.0,
    );

    let mut hidden = Vec::new();
    for (index, chunk) in model.meshes.iter().enumerate() {
        if pvs.visible(cell, index) {
            continue;
        }
        let Some(stride) = chunk
            .declared_stride()
            .or_else(|| chunk.solve_stride_without_a_box(&model_blob))
        else {
            continue;
        };
        let (mut min, mut max) = ([f32::MAX; 3], [f32::MIN; 3]);
        for submesh in &chunk.submeshes {
            if let Ok(positions) = chunk.positions(&model_blob, submesh, stride) {
                for p in positions {
                    for i in 0..3 {
                        min[i] = min[i].min(p[i]);
                        max[i] = max[i].max(p[i]);
                    }
                }
            }
        }
        if min[0] > max[0] {
            continue;
        }
        // Distance to the box, zero when the point is inside it.
        let gap: f32 = (0..3)
            .map(|i| (min[i] - point[i]).max(point[i] - max[i]).max(0.0).powi(2))
            .sum::<f32>()
            .sqrt();
        let material = model
            .material_of(chunk)
            .map(|m| {
                m.name
                    .rsplit('/')
                    .next()
                    .unwrap_or("?")
                    .trim_end_matches(".rcsmaterial")
                    .to_string()
            })
            .unwrap_or_else(|| "?".into());
        let triangles: usize = chunk.submeshes.iter().map(|s| s.index_count).sum::<usize>() / 3;
        hidden.push((
            gap,
            index,
            material,
            triangles,
            [max[0] - min[0], max[1] - min[1], max[2] - min[2]],
        ));
    }
    hidden.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    println!(
        "{} chunk(s) hidden here; the nearest 20 by box distance:",
        hidden.len()
    );
    for (gap, index, material, triangles, size) in hidden.iter().take(20) {
        println!(
            "  gap {gap:8.2}  chunk {index:5}  {triangles:6} tri  size [{:7.1} {:7.1} {:7.1}]  {material}",
            size[0], size[1], size[2],
        );
    }
    Ok(())
}
