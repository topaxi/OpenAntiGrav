//! [`map`]: what the content layer read, as the plain data a race consumes.

use std::collections::BTreeMap;

use anyhow::{Context, bail};
use oag_ballisticng::track::{ColliderKind, Placement, Section, Tile, TileKind, Track};
use oag_ballisticng::wtrk::Wtrk;
use oag_core::math::Vec3;
use oag_race::Course;
use oag_trackfile::Track as OagTrack;
use oag_vex::collision::{
    CollisionGeometry, CollisionMesh, CollisionNode, HEADER_WORD, SurfaceKind,
};
use oag_vex::pads::PadVolume;
use oag_vex::track::{AiTrack, Junction, Path, SHORT_POINT_VERSION, SplinePoint, StartPosition};

use super::choice::{
    AI_CORRIDOR_FRACTION, AI_CORRIDOR_MARGIN, JUMP_FACTOR, PAD_ABOVE, PAD_BELOW, WORLD_SCALE,
};
use super::frame::{Frame, mirror, world};

/// Triangles per collision mesh: a collision mesh indexes its vertices with
/// `u16` and a foreign triangle list is not indexed, so three vertices each.
const TRIANGLES_PER_MESH: usize = 21_845;

pub(super) fn map(wtrk: &Wtrk, track: &Track) -> anyhow::Result<OagTrack> {
    let mut report = Vec::new();
    report.push(format!(
        "foreign track {:?} (Workshop item {}), tags {}",
        wtrk.title,
        wtrk.workshop_id,
        wtrk.tags.join(", ")
    ));
    report.push(format!(
        "mapped by choice, not measurement: handedness Z negated, world scale x{WORLD_SCALE} \
         on top of the track's own placement, AI corridor {AI_CORRIDOR_FRACTION} of the floor \
         half-width"
    ));

    let lap: Vec<usize> = track.chains.lap_loop().to_vec();
    if lap.len() < 8 {
        bail!("the track has no lap loop to run (its main chain does not close or is too short)");
    }
    let placement = track
        .floor_placement()
        .context("the track marks no single floor object to place its geometry by")?;
    if track.wall_placement() != Some(placement) {
        bail!("the floor and wall objects are placed differently");
    }
    let scale = placement
        .uniform_scale()
        .context("the floor object's placement is not a plain uniform scale")?;
    report.push(format!(
        "the track's own placement scale {scale} is applied to its collision geometry (section \
         and floor-tile positions are already placed)"
    ));

    let frames = frames(&track.sections)?;
    let by_section = tiles_by_section(track);
    let (ai, spacing_note) = spline(track, &lap, &frames, &by_section, &placement, &mut report)?;
    report.extend(spacing_note);
    let start = start_slot(&ai.paths[0].points, track, &lap, &mut report)?;
    let nodes = collision(track, &placement, &mut report);
    let speedup_pads = pads(track, &placement, &frames, TileKind::Speed);
    let weapon_pads = pads(track, &placement, &frames, TileKind::Weapon);
    report.push(format!(
        "{} speed pad volume(s) and {} weapon pad volume(s), one per section the pad tiles cover \
         (box from the tiles' own vertices, vertical reach as Pulse's pad bind)",
        speedup_pads.len(),
        weapon_pads.len()
    ));
    unmapped(track, &mut report);

    Ok(OagTrack {
        id: wtrk.workshop_id.to_string(),
        title: wtrk.title.clone(),
        ai,
        start,
        nodes,
        speedup_pads,
        weapon_pads,
        notes: report,
    })
}

fn frames(sections: &[Section]) -> anyhow::Result<Vec<Frame>> {
    sections
        .iter()
        .map(|s| {
            Frame::of(s.position, s.rotation, s.normal)
                .with_context(|| format!("section {} has a degenerate frame", s.index))
        })
        .collect()
}

fn tiles_by_section(track: &Track) -> Vec<Vec<usize>> {
    let mut out = vec![Vec::new(); track.sections.len()];
    for (i, tile) in track.floor_tiles.iter().enumerate() {
        if let Some(s) = tile.section {
            out[s].push(i);
        }
    }
    out
}

fn tile_corners(track: &Track, tile: &Tile, placement: &Placement) -> Vec<Vec3> {
    tile.indices
        .iter()
        .map(|&i| world(placement.apply(track.floor_vertices[i])))
        .collect()
}

fn spline(
    track: &Track,
    lap: &[usize],
    frames: &[Frame],
    by_section: &[Vec<usize>],
    placement: &Placement,
    report: &mut Vec<String>,
) -> anyhow::Result<(AiTrack, Option<String>)> {
    let line = track.racing_lines.first();
    if line.is_none() {
        report.push(
            "the track authors no AI racing line: the centre line stands in for it".to_string(),
        );
    }
    let mut points = Vec::with_capacity(lap.len());
    let (mut without_floor, mut last_widths) = (0usize, (1.0f32, 1.0f32));
    for &s in lap {
        let f = frames[s];
        let mut extent: Option<(f32, f32)> = None;
        for &t in &by_section[s] {
            for corner in tile_corners(track, &track.floor_tiles[t], placement) {
                let across = (corner - f.pos).dot(f.right);
                extent = Some(extent.map_or((across, across), |(lo, hi)| {
                    (lo.min(across), hi.max(across))
                }));
            }
        }
        let (left, right) = match extent {
            Some((lo, hi)) => (-lo.min(0.0), hi.max(0.0)),
            None => {
                without_floor += 1;
                last_widths
            }
        };
        last_widths = (left, right);
        let racing_line = line.map_or(0.0, |l| l[s].offset * WORLD_SCALE);
        let bound_left = (-left * AI_CORRIDOR_FRACTION).min(racing_line - AI_CORRIDOR_MARGIN);
        let bound_right = (right * AI_CORRIDOR_FRACTION).max(racing_line + AI_CORRIDOR_MARGIN);
        points.push(SplinePoint {
            pos: f.pos.to_array(),
            tangent: f.forward.to_array(),
            down: (-f.up).to_array(),
            lateral: f.right.to_array(),
            progress: 0.0,
            half_width_left: left,
            half_width_right: right,
            ai_bound_left: bound_left,
            ai_bound_right: bound_right,
            racing_line,
            section_id: 0xff,
            flags: 0,
            light_scale: [255; 4],
            unk_0x5c: None,
            unk_0x5d: None,
            unk_0x66: None,
            cage: [0; 8],
        });
    }
    if without_floor > 0 {
        report.push(format!(
            "{without_floor} lap section(s) have no floor tile: the previous section's width stands in"
        ));
    }

    let gaps: Vec<f32> = (0..points.len())
        .map(|i| {
            let a = Vec3::from_array(points[i].pos);
            let b = Vec3::from_array(points[(i + 1) % points.len()].pos);
            a.distance(b)
        })
        .collect();
    let total: f32 = gaps.iter().sum();
    let mut sorted = gaps.clone();
    sorted.sort_by(f32::total_cmp);
    let median = sorted[sorted.len() / 2];
    let mut run = 0.0;
    for (p, g) in points.iter_mut().zip(&gaps) {
        p.progress = run / total;
        run += g;
    }
    let jumps: Vec<String> = gaps
        .iter()
        .enumerate()
        .filter(|(_, g)| **g > median * JUMP_FACTOR)
        .map(|(i, g)| format!("{g:.0} units after section {}", lap[i]))
        .collect();
    let note = (!jumps.is_empty()).then(|| {
        format!(
            "the lap loop has {} jump(s) (spacing over {JUMP_FACTOR}x the median {median:.1}): {}; \
             the original crosses them with a jump pad and this engine's physics does not model \
             that",
            jumps.len(),
            jumps.join(", ")
        )
    });
    report.push(format!(
        "spline: {} control points over {total:.0} units, median spacing {median:.1}",
        points.len()
    ));
    let max_spacing = gaps.iter().copied().fold(0.0f32, f32::max);
    Ok((
        AiTrack {
            version: SHORT_POINT_VERSION,
            paths: vec![Path {
                points,
                max_spacing,
                entry: Some(0),
                exit: Some(0),
            }],
            junctions: vec![Junction {
                prev: [Some(0), None],
                next: [Some(0), None],
            }],
        },
        note,
    ))
}

/// The slot the engine's own start-line rule lands on the authored line from:
/// `Course::START_LINE_ADVANCE` behind it along the ring.
fn start_slot(
    points: &[SplinePoint],
    track: &Track,
    lap: &[usize],
    report: &mut Vec<String>,
) -> anyhow::Result<StartPosition> {
    let line = lap
        .iter()
        .position(|&s| s == track.start_line)
        .context("the start line is not on the lap loop")?;
    let n = points.len();
    let at = |i: usize| Vec3::from_array(points[i % n].pos);
    let mut remaining = Course::START_LINE_ADVANCE;
    let mut i = line;
    let mut position = at(line);
    for _ in 0..n {
        let prev = (i + n - 1) % n;
        let step = at(prev).distance(at(i));
        if step >= remaining {
            position = at(i).lerp(at(prev), remaining / step.max(f32::EPSILON));
            break;
        }
        remaining -= step;
        i = prev;
        position = at(i);
    }
    let tangent = Vec3::from_array(points[i % n].tangent);
    let forward = Vec3::new(tangent.x, 0.0, tangent.z)
        .try_normalize()
        .context("the start section runs vertically")?;
    let up = Vec3::Y;
    report.push(format!(
        "start: the lap line is section {} and the slot sits {} units behind it on the ring, so \
         the engine's own start-line rule lands on it; the authored 8 grid slots (spacing {:.0} \
         units here) are not used, the engine lays its own grid",
        track.start_line,
        Course::START_LINE_ADVANCE,
        grid_spacing(track),
    ));
    Ok(StartPosition {
        position: position.to_array(),
        left: up.cross(forward).to_array(),
        up: up.to_array(),
        forward: forward.to_array(),
    })
}

fn grid_spacing(track: &Track) -> f32 {
    let mut slots: Vec<Vec3> = track
        .floor_tiles
        .iter()
        .filter(|t| t.kind == TileKind::Grid)
        .map(|t| world(t.position))
        .collect();
    slots.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.z.total_cmp(&b.z)));
    let mut gaps: Vec<f32> = slots.windows(2).map(|w| w[0].distance(w[1])).collect();
    gaps.sort_by(f32::total_cmp);
    gaps.get(gaps.len() / 2).copied().unwrap_or(0.0)
}

fn collision(track: &Track, placement: &Placement, report: &mut Vec<String>) -> Vec<CollisionNode> {
    let mut degenerate = 0;
    let mut out = Vec::new();
    for (kind, name, v, n) in [
        (
            SurfaceKind::Floor,
            "foreign floor",
            &track.floor_vertices,
            &track.floor_normals,
        ),
        (
            SurfaceKind::Wall,
            "foreign wall",
            &track.wall_vertices,
            &track.wall_normals,
        ),
    ] {
        let (node, flipped) = soup(kind, name, v, n, placement, &mut degenerate, out.len());
        report.push(format!(
            "{name}: {} triangle(s) in {} mesh(es); {flipped} wound the other way round to face \
             as authored (handedness flips winding)",
            node.geometry
                .meshes
                .iter()
                .map(|m| m.triangles.len())
                .sum::<usize>(),
            node.geometry.meshes.len()
        ));
        out.push(node);
    }
    if degenerate > 0 {
        report.push(format!(
            "{degenerate} zero-area collision triangle(s) dropped"
        ));
    }
    out
}

/// A triangle list as collision geometry facing as authored: each triangle's
/// winding normal is made to agree with the mean of its three authored vertex
/// normals (mirrored), so the handedness flip cannot leave one inside out.
fn soup(
    kind: SurfaceKind,
    name: &str,
    vertices: &[[f32; 3]],
    normals: &[[f32; 3]],
    placement: &Placement,
    degenerate: &mut usize,
    node_index: usize,
) -> (CollisionNode, usize) {
    let mut meshes = Vec::new();
    let (mut flipped, mut verts, mut tris) = (0, Vec::new(), Vec::new());
    let mut flush = |verts: &mut Vec<[f32; 3]>, tris: &mut Vec<[u16; 3]>| {
        if !tris.is_empty() {
            meshes.push(CollisionMesh {
                vertex_scalars: vec![1.0; verts.len()],
                vertices: std::mem::take(verts),
                triangles: std::mem::take(tris),
                chunks: Vec::new(),
            });
        }
    };
    for (t, tn) in vertices
        .as_chunks::<3>()
        .0
        .iter()
        .zip(normals.as_chunks::<3>().0)
    {
        let [a, mut b, mut c] = t.map(|v| world(placement.apply(v)));
        let authored: Vec3 = tn.iter().map(|n| mirror(*n)).sum();
        let winding = (b - a).cross(c - a);
        if winding.length_squared() <= f32::EPSILON {
            *degenerate += 1;
            continue;
        }
        if winding.dot(authored) < 0.0 {
            std::mem::swap(&mut b, &mut c);
            flipped += 1;
        }
        if tris.len() == TRIANGLES_PER_MESH {
            flush(&mut verts, &mut tris);
        }
        let base = verts.len() as u16;
        verts.extend([a, b, c].map(|v| v.to_array()));
        tris.push([base, base + 1, base + 2]);
    }
    flush(&mut verts, &mut tris);
    (
        CollisionNode {
            kind,
            node_index,
            name: Some(name.to_string()),
            geometry: CollisionGeometry {
                version: HEADER_WORD,
                meshes,
            },
        },
        flipped,
    )
}

fn pads(track: &Track, placement: &Placement, frames: &[Frame], kind: TileKind) -> Vec<PadVolume> {
    let mut groups: BTreeMap<usize, Vec<Vec3>> = BTreeMap::new();
    for tile in track.floor_tiles.iter().filter(|t| t.kind == kind) {
        if let Some(s) = tile.section {
            groups
                .entry(s)
                .or_default()
                .extend(tile_corners(track, tile, placement));
        }
    }
    groups
        .into_iter()
        .map(|(s, corners)| {
            let f = frames[s];
            let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for c in corners {
                let d = c - f.pos;
                for (axis, v) in [d.dot(f.right), d.dot(f.up), d.dot(f.forward)]
                    .into_iter()
                    .enumerate()
                {
                    lo[axis] = lo[axis].min(v);
                    hi[axis] = hi[axis].max(v);
                }
            }
            lo[1] -= PAD_BELOW;
            hi[1] += PAD_ABOVE;
            let row = |v: Vec3, w: f32| [v.x, v.y, v.z, w];
            let rows = [
                row(f.right, 0.0),
                row(f.up, 0.0),
                row(f.forward, 0.0),
                row(f.pos, 1.0),
            ];
            let mut to_world = [0.0; 16];
            for (i, r) in rows.iter().enumerate() {
                to_world[i * 4..i * 4 + 4].copy_from_slice(r);
            }
            PadVolume {
                to_world,
                min: lo,
                max: hi,
                disabled: 0.0,
            }
        })
        .collect()
}

/// Names what the content authors that this mapping has no counterpart for.
fn unmapped(track: &Track, report: &mut Vec<String>) {
    let count = |k: TileKind| track.floor_tiles.iter().filter(|t| t.kind == k).count();
    report.push(format!(
        "not mapped: {} recharge strip tile(s) (no counterpart here), {} authored grid slot(s) \
         (see start)",
        count(TileKind::Recharge),
        count(TileKind::Grid)
    ));
    let pit = track.chains.branches.iter().filter(|b| b.pit).count();
    report.push(format!(
        "not mapped: {} branch chain(s) of which {pit} pit lane(s); the ring is the lap loop only",
        track.chains.branches.len()
    ));
    if !track.chains.unreached.is_empty() {
        report.push(format!(
            "not mapped: {} section(s) no link reaches ({}..{})",
            track.chains.unreached.len(),
            track.chains.unreached[0],
            track.chains.unreached[track.chains.unreached.len() - 1]
        ));
    }
    let mut special: BTreeMap<&str, usize> = BTreeMap::new();
    for s in &track.sections {
        for f in &s.special {
            *special.entry(f).or_default() += 1;
        }
    }
    if !special.is_empty() {
        report.push(format!(
            "ignored section flags (read, not acted on): {}",
            special
                .iter()
                .map(|(k, v)| format!("{k} x{v}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let mut scenery = (0, 0);
    for c in &track.colliders {
        match c.kind {
            ColliderKind::Scenery => scenery.0 += 1,
            ColliderKind::Unknown(_) => scenery.1 += 1,
            _ => {}
        }
    }
    report.push(format!(
        "scenery colliders ignored: {} scenery, {} of unknown kind (whether the original treats \
         them as solid is not known)",
        scenery.0, scenery.1
    ));
    report.push(
        "not drawn: sky, fog, lights, scenery meshes and the track atlas (no counterpart mapped \
         yet); the floor and wall are drawn flat from their collision triangles"
            .to_string(),
    );
}
