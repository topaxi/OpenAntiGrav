//! Where was every moving draw at the instant a GE dump was taken?
//!
//! The original's `Anim Transform` clock is one global time that every node
//! reads (`AnimTransform::sample` documents the wrap), so a dump's moving PRIMs,
//! whose world boxes `scripts/psp-ge-dump.py census` records, pin that time:
//! the right `T` is the one at which the most observed boxes coincide with a
//! moving draw of ours of the same vertex count and diameter. With `T` known,
//! the world box of a moving draw the dump does **not** contain is known too,
//! and `scripts/pvs-moving-residue.py` can ask whether the view rejects it.
//!
//! ```sh
//! cargo run -q -p oag-render --example pvs_moving_phase -- \
//!     data/images/pulse-psp-usa.chd 'Data\Environments\03_Track\track.vex' OBS.tsv [MAX_SECONDS]
//! ```
//!
//! `OBS.tsv`: one dump PRIM per line, `nv diam mnx mny mnz mxx mxy mxz`
//! (whitespace separated). Output: one JSON object, `t` (best seconds), `score`
//! (distinct observed boxes matched), `runner_up` (the best score at a time more
//! than half a second away - the ambiguity check), and `boxes`, the world box of
//! every moving draw at `t`.
//!
//! A moving draw's world box here is the box of the draw's extremal vertices
//! along 13 axes, transformed: a lower bound on the exact box, within a unit or
//! two on these meshes, which is why the tolerance below is not zero.

use oag_vex::{track, vex};

/// Absolute tolerance on each of a box's six numbers, in world units.
fn tolerance() -> f32 {
    std::env::var("TOL")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1.5)
}

fn extremal(points: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let dirs: [[f32; 3]; 13] = [
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
        [0.0, 0.0, 1.0],
        [1.0, 1.0, 0.0],
        [1.0, -1.0, 0.0],
        [1.0, 0.0, 1.0],
        [1.0, 0.0, -1.0],
        [0.0, 1.0, 1.0],
        [0.0, 1.0, -1.0],
        [1.0, 1.0, 1.0],
        [1.0, 1.0, -1.0],
        [1.0, -1.0, 1.0],
        [-1.0, 1.0, 1.0],
    ];
    let mut out: Vec<[f32; 3]> = Vec::new();
    for d in dirs {
        let dot = |p: &[f32; 3]| p[0] * d[0] + p[1] * d[1] + p[2] * d[2];
        if let Some(hi) = points.iter().max_by(|a, b| dot(a).total_cmp(&dot(b))) {
            out.push(*hi);
        }
        if let Some(lo) = points.iter().min_by(|a, b| dot(a).total_cmp(&dot(b))) {
            out.push(*lo);
        }
    }
    out
}

fn diameter(points: &[[f32; 3]]) -> f32 {
    let dist2 = |a: [f32; 3], b: [f32; 3]| (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f32>();
    let mut best = 0.0f32;
    if points.len() <= 3000 {
        for (i, &a) in points.iter().enumerate() {
            for &b in &points[i + 1..] {
                best = best.max(dist2(a, b));
            }
        }
        return best.sqrt();
    }
    let farthest = |from: [f32; 3]| {
        points
            .iter()
            .copied()
            .max_by(|&a, &b| dist2(a, from).total_cmp(&dist2(b, from)))
            .unwrap_or(from)
    };
    let a = farthest(points[0]);
    let b = farthest(a);
    dist2(a, b).sqrt()
}

struct Moving {
    list: &'static str,
    index: usize,
    node: i64,
    slot: usize,
    nv: usize,
    diam: f32,
    points: Vec<[f32; 3]>,
    lo: [f32; 3],
    hi: [f32; 3],
}

fn world_box(m: &[f32; 16], points: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for &p in points {
        let w = vex::transform_point(m, p);
        for k in 0..3 {
            lo[k] = lo[k].min(w[k]);
            hi[k] = hi[k].max(w[k]);
        }
    }
    (lo, hi)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: pvs_moving_phase IMAGE ENTRY OBS.tsv [MAX_SECONDS]";
    let image = args.next().ok_or(usage)?;
    let entry = args.next().ok_or(usage)?;
    let obs_path = args.next().ok_or(usage)?;
    let max_seconds: f32 = args.next().map_or(Ok(1800.0), |s| s.parse())?;

    let mut archives = oag_pulse::open(&image)?;
    let blob = archives.read_name(&entry)?;
    let model = oag_mesh::mesh::build_with_textures(&entry, &blob, None)?;
    let nodes = vex::nodes(&blob)?;
    let _ = track::find_node(&blob, &nodes).ok_or("no WO Track node")?;

    let obs: Vec<(usize, f32, [f32; 3], [f32; 3])> = std::fs::read_to_string(&obs_path)?
        .lines()
        .filter_map(|l| {
            let v: Vec<f32> = l
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            (v.len() == 8).then(|| (v[0] as usize, v[1], [v[2], v[3], v[4]], [v[5], v[6], v[7]]))
        })
        .collect();

    let lists = [
        ("opaque", &model.draws),
        ("cutout", &model.alpha_tested_draws),
        ("transparent", &model.transparent_draws),
    ];
    let mut moving: Vec<Moving> = Vec::new();
    for (name, draws) in lists {
        for (index, draw) in draws.iter().enumerate() {
            if !draw.moving {
                continue;
            }
            let indices = &model.indices[draw.range.start as usize..draw.range.end as usize];
            let Some(&first) = indices.first() else {
                continue;
            };
            let slot = model.vertices[first as usize].xform as usize;
            if slot == 0 {
                continue;
            }
            let points: Vec<[f32; 3]> = indices
                .iter()
                .map(|&i| model.vertices[i as usize].position)
                .collect();
            let nv = indices
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len();
            let (lo, hi) = world_box(&vex::IDENTITY, &points);
            moving.push(Moving {
                list: name,
                index,
                node: draw.node.map_or(-1, i64::from),
                slot,
                nv,
                diam: diameter(&points),
                points: extremal(&points),
                lo,
                hi,
            });
        }
    }

    // Per moving draw, the observed boxes it could be: same count, diameter
    // within 3 %.
    let candidates: Vec<Vec<usize>> = moving
        .iter()
        .map(|m| {
            obs.iter()
                .enumerate()
                .filter(|(_, o)| {
                    o.0 == m.nv && (o.1 - m.diam).abs() <= 0.03 * m.diam.max(o.1).max(1.0)
                })
                .map(|(k, _)| k)
                .collect()
        })
        .collect();

    let tol = tolerance();
    let score_at = |seconds: f32| -> usize {
        let matrices = model.sample_anim_nodes(seconds);
        let mut hit = vec![false; obs.len()];
        for (m, cands) in moving.iter().zip(&candidates) {
            if cands.is_empty() {
                continue;
            }
            let Some(matrix) = matrices.get(m.slot - 1) else {
                continue;
            };
            let (lo, hi) = world_box(matrix, &m.points);
            for &k in cands {
                let o = &obs[k];
                if (0..3).all(|a| (lo[a] - o.2[a]).abs() <= tol && (hi[a] - o.3[a]).abs() <= tol) {
                    hit[k] = true;
                }
            }
        }
        hit.iter().filter(|&&h| h).count()
    };

    let steps = (max_seconds * 60.0) as usize;
    let mut scores: Vec<(usize, f32)> = (0..=steps)
        .map(|k| {
            let t = k as f32 / 60.0;
            (score_at(t), t)
        })
        .collect();
    scores.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.total_cmp(&b.1)));
    let (mut best_score, mut best_t) = scores[0];
    // Refine on a 1/480 s grid around the coarse optimum.
    for k in -16..=16 {
        let t = best_t + k as f32 / 480.0;
        if t < 0.0 {
            continue;
        }
        let s = score_at(t);
        if s > best_score {
            best_score = s;
            best_t = t;
        }
    }
    let runner_up = scores
        .iter()
        .find(|(_, t)| (t - best_t).abs() > 0.5)
        .copied()
        .unwrap_or((0, 0.0));
    let mut top: Vec<(usize, f32)> = Vec::new();
    for &(s, t) in &scores {
        if top.iter().all(|&(_, u)| (t - u).abs() > 2.0) {
            top.push((s, t));
        }
        if top.len() == 8 {
            break;
        }
    }
    let top_json: Vec<String> = top.iter().map(|(s, t)| format!("[{s},{t}]")).collect();

    let matrices = model.sample_anim_nodes(best_t);
    let rest = model.sample_anim_nodes(0.0);
    // The mesh's own box is not parsed here; the union of every batch it
    // carries (in the node's space) stands in for it, and what the game tests is
    // its eight corners under the world matrix, so the corners are what is out.
    let mut node_box: std::collections::BTreeMap<i64, ([f32; 3], [f32; 3])> = Default::default();
    for m in &moving {
        let e = node_box.entry(m.node).or_insert((m.lo, m.hi));
        for k in 0..3 {
            e.0[k] = e.0[k].min(m.lo[k]);
            e.1[k] = e.1[k].max(m.hi[k]);
        }
    }
    let boxes: Vec<String> = moving
        .iter()
        .filter_map(|m| {
            let matrix = matrices.get(m.slot - 1)?;
            let (lo, hi) = world_box(matrix, &m.points);
            let (rlo, rhi) = world_box(rest.get(m.slot - 1)?, &m.points);
            let (llo, lhi) = world_box(&vex::IDENTITY, &m.points);
            let (nlo, nhi) = node_box[&m.node];
            let mut corners: Vec<[f32; 3]> = Vec::new();
            for x in [nlo[0], nhi[0]] {
                for y in [nlo[1], nhi[1]] {
                    for z in [nlo[2], nhi[2]] {
                        corners.push(vex::transform_point(matrix, [x, y, z]));
                    }
                }
            }
            // The same mesh box over the previous 14 frames at 1/60 s: the
            // original keeps a mesh it saw in view drawing for 8 + rand % 7
            // frames, so a draw far outside the view now may have been inside it.
            let mut history: Vec<Vec<[f32; 3]>> = Vec::new();
            for back in 1..=14 {
                let earlier = model.sample_anim_nodes((best_t - back as f32 / 60.0).max(0.0));
                let Some(em) = earlier.get(m.slot - 1) else {
                    continue;
                };
                let mut cs = Vec::new();
                for x in [nlo[0], nhi[0]] {
                    for y in [nlo[1], nhi[1]] {
                        for z in [nlo[2], nhi[2]] {
                            cs.push(vex::transform_point(em, [x, y, z]));
                        }
                    }
                }
                history.push(cs);
            }
            Some(format!(
                "{{\"list\":\"{}\",\"i\":{},\"node\":{},\"nv\":{},\"diam\":{},\"mn\":{:?},\"mx\":{:?},\
                 \"rest_mn\":{:?},\"rest_mx\":{:?},\"local_mn\":{:?},\"local_mx\":{:?},\"mesh_corners\":{:?},\"history\":{:?}}}",
                m.list, m.index, m.node, m.nv, m.diam, lo, hi, rlo, rhi, llo, lhi, corners, history
            ))
        })
        .collect();
    println!(
        "{{\"t\":{best_t},\"score\":{best_score},\"top\":[{}],\"observed\":{},\"runner_up\":{{\"score\":{},\"t\":{}}},\"boxes\":[{}]}}",
        top_json.join(","),
        obs.len(),
        runner_up.0,
        runner_up.1,
        boxes.join(",\n")
    );
    Ok(())
}
