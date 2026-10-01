//! Which draws of a Pulse circuit pass the authored section mask from a given
//! pose, and what shape each is - the ours-side half of the moving-draw census
//! in `docs/rendering/frame-audit.md` section 3.
//!
//! One JSON array on stdout, one object per draw call of the three lists, so
//! `data/scratch/<lane>/census-join.py` can match them against
//! `scripts/psp-ge-dump.py census` by (vertex count, rest-geometry extents,
//! diameter). The geometry is the draw's rest pose: a moving draw's box moves
//! with its `Anim Transform`, its diameter does not.
//!
//! ```sh
//! cargo run -q -p oag-render --example pvs_moving_census -- \
//!     data/images/pulse-psp-usa.chd 'Data\Environments\03_Track\track.vex' CRAFT_SECTION CAMERA_SECTION
//! ```
//!
//! `ours` is what the running game draws (the craft's mask unioned with the
//! camera's and the padding, `VisibleSet::around`); `craft` is the craft's
//! authored mask alone, which is what the original culls with. The frustum is
//! not applied, and a moving draw never had it applied.

use oag_render::pvs::{DrawSections, SectionPadding, SwapConflicts, VisibleSet};
use oag_vex::pvs::TrackPvs;
use oag_vex::{track, vex};

fn diameter(points: &[[f32; 3]]) -> f32 {
    let Some(&first) = points.first() else {
        return 0.0;
    };
    let dist2 = |a: [f32; 3], b: [f32; 3]| (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f32>();
    let farthest = |from: [f32; 3]| {
        points
            .iter()
            .copied()
            .max_by(|&a, &b| dist2(a, from).total_cmp(&dist2(b, from)))
            .unwrap_or(from)
    };
    let a = farthest(first);
    let b = farthest(a);
    dist2(a, b).sqrt()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: pvs_moving_census IMAGE ENTRY CRAFT_SECTION CAMERA_SECTION";
    let image = args.next().ok_or(usage)?;
    let entry = args.next().ok_or(usage)?;
    let craft: u8 = args.next().ok_or(usage)?.parse()?;
    let camera: u8 = args.next().ok_or(usage)?.parse()?;

    let mut archives = oag_pulse::open(&image)?;
    let blob = archives.read_name(&entry)?;
    let model = oag_render::mesh::build_with_textures(&entry, &blob, None)?;
    let pvs = TrackPvs::parse(&blob)?;
    let nodes = vex::nodes(&blob)?;
    let ai_node = track::find_node(&blob, &nodes).ok_or("no WO Track node")?;
    let ai = track::parse(&blob[ai_node.payload()])?;
    let padding = SectionPadding::from_track(&ai);
    let governing = oag_vex::pvs::governing_sections(&blob, &nodes)?;
    let (sections, _) = DrawSections::place(&model, &governing, &pvs);
    let swaps = SwapConflicts::find(&pvs, &sections, &model);
    let ours = VisibleSet::around(&pvs, &padding, &swaps, craft, camera);
    let craft_mask = pvs.visible_from(craft);

    let lists = [
        ("opaque", &sections.opaque, &model.draws),
        ("cutout", &sections.alpha_tested, &model.alpha_tested_draws),
        (
            "transparent",
            &sections.transparent,
            &model.transparent_draws,
        ),
    ];
    let mut records = Vec::new();
    for (name, masks, draws) in lists {
        for (index, (mask, draw)) in masks.iter().zip(draws.iter()).enumerate() {
            let indices = &model.indices[draw.range.start as usize..draw.range.end as usize];
            let points: Vec<[f32; 3]> = indices
                .iter()
                .map(|&i| model.vertices[i as usize].position)
                .collect();
            let distinct = indices
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            let mut lo = [f32::MAX; 3];
            let mut hi = [f32::MIN; 3];
            for p in &points {
                for k in 0..3 {
                    lo[k] = lo[k].min(p[k]);
                    hi[k] = hi[k].max(p[k]);
                }
            }
            records.push(format!(
                "{{\"list\":\"{name}\",\"i\":{index},\"node\":{},\"moving\":{},\"cnt\":{},\"nv\":{distinct},\
                 \"mn\":[{},{},{}],\"mx\":[{},{},{}],\"diam\":{},\"mask\":\"{mask:x}\",\
                 \"ours\":{},\"craft\":{}}}",
                draw.node.map_or(-1, i64::from),
                draw.moving,
                indices.len(),
                lo[0],
                lo[1],
                lo[2],
                hi[0],
                hi[1],
                hi[2],
                diameter(&points),
                ours.allows(*mask),
                craft_mask & mask != 0,
            ));
        }
    }
    println!("[{}]", records.join(",\n"));
    Ok(())
}
