//! The authored track a subcommand reads: its grid slot, its spline graph, and
//! the `track` and `pads` dumps.

use anyhow::{Context, Result, bail};
use log::debug;
use oag_core::math::Vec3;
use oag_pulse as pulse;
use oag_race::Course;
use oag_vex::{track, vex};

/// Reads a track's authored grid slot out of its `.vex`, for `drive`.
///
/// `Ok(None)` when the file carries no `Start Position` node, which is not an
/// error: a track without one is a track a ship starts on the spline of, the
/// same fallback `oag_raceplay` takes.
pub(crate) fn load_start_position(
    source: &str,
    name: &str,
) -> Result<Option<track::StartPosition>> {
    let mut archives = pulse::open(source)?;
    let blob = archives
        .read_name(name)
        .with_context(|| format!("reading {name} out of {}", archives.layout.describe()))?;
    let nodes = vex::nodes(&blob).context("walking the node tree")?;
    let Some(node) = nodes
        .iter()
        .find(|n| n.class_id == vex::CLASS_START_POSITION)
    else {
        return Ok(None);
    };
    let payload = blob
        .get(node.payload())
        .context("the Start Position payload runs past the end of the file")?;
    Ok(track::start_position(payload, vex::byte_order(&blob)))
}

/// Reads a track's `WO Track` spline graph out of its `.vex`, for `track` and
/// for `drive`.
pub(crate) fn read_track_blob(source: &str, name: &str) -> Result<Vec<u8>> {
    let mut archives = pulse::open(source)?;
    archives
        .read_name(name)
        .with_context(|| format!("reading {name} out of {}", archives.layout.describe()))
}

pub(crate) fn ai_of(blob: &[u8], name: &str) -> Result<track::AiTrack> {
    let nodes = vex::nodes(blob).context("walking the node tree")?;
    // By the class id this file's own version word implies, not by the version-6
    // constant: `CLASS_WO_TRACK` finds nothing in a version-4 `.vex`, which is
    // every track on Pure's disc.
    let node =
        track::find_node(blob, &nodes).with_context(|| format!("{name} has no WO Track node"))?;
    let payload = blob
        .get(node.payload())
        .context("the WO Track payload runs past the end of the file")?;
    let ai = track::parse(payload).map_err(|e| anyhow::anyhow!("{name}: {e}"))?;
    debug!(
        "{name}: {} path(s), {} junction(s), {} control point(s)",
        ai.paths.len(),
        ai.junctions.len(),
        ai.point_count()
    );
    Ok(ai)
}

pub(crate) fn load_ai(source: &str, name: &str) -> Result<track::AiTrack> {
    ai_of(&read_track_blob(source, name)?, name)
}

/// Every path resampled, with the path each sample came from.
///
/// The same `Course::STEPS_PER_SEGMENT` and the same order `oag_raceplay::Spline` and
/// `oag_render::track` use, so a dumped line, a drawn line and the line a
/// scenario run starts on are one set of points.
pub(crate) fn resample(ai: &track::AiTrack, steps: usize) -> Vec<(usize, track::Sample)> {
    let mut samples = Vec::new();
    for (index, path) in ai.paths.iter().enumerate() {
        for segment in 0..path.points.len() {
            for step in 0..steps {
                if let Some(sample) = path.sample(segment, step as f32 / steps as f32) {
                    samples.push((index, sample));
                }
            }
        }
    }
    samples
}

/// Writes a track's resampled spline to stdout as CSV.
///
/// One row per sample, in the order the curve runs, carrying the frame
/// (`tangent`, `lateral`, `down`), both half-widths, the AI corridor and the
/// authored `racing_line` offset. Two derived columns come along because every
/// consumer computes them the same way and one of them is easy to get backwards:
///
/// - `lift_*` is the sample lifted off the surface by
///   [`oag_vex::track::HOVER_LIFT`], which is where ships actually fly.
/// - `line_*` is that point moved across the track by `racing_line` along
///   `lateral`, which is the authored line itself.
///
/// Nothing is renormalised: the interpolated axes are neither unit length nor
/// exactly perpendicular, and that is the original's behaviour rather than a
/// decode error - `oag_gameplay::spawn` says so at length.
pub(crate) fn dump_track(source: &str, name: &str, steps: usize) -> Result<()> {
    if steps == 0 {
        bail!("--steps must be at least 1");
    }
    let ai = load_ai(source, name)?;

    println!(
        "index,path,pos_x,pos_y,pos_z,lift_x,lift_y,lift_z,line_x,line_y,line_z,\
         tan_x,tan_y,tan_z,lat_x,lat_y,lat_z,down_x,down_y,down_z,\
         half_left,half_right,ai_left,ai_right,racing_line,section,flags"
    );
    let samples = resample(&ai, steps);
    for (index, (path_index, s)) in samples.iter().enumerate() {
        let lift = [
            s.pos[0] - track::HOVER_LIFT * s.down[0],
            s.pos[1] - track::HOVER_LIFT * s.down[1],
            s.pos[2] - track::HOVER_LIFT * s.down[2],
        ];
        let line = [
            lift[0] + s.racing_line * s.lateral[0],
            lift[1] + s.racing_line * s.lateral[1],
            lift[2] + s.racing_line * s.lateral[2],
        ];
        let mut row = vec![index.to_string(), path_index.to_string()];
        for v in [s.pos, lift, line, s.tangent, s.lateral, s.down] {
            row.extend(v.iter().map(|c| format!("{c:.7}")));
        }
        for v in [
            s.half_width_left,
            s.half_width_right,
            s.ai_bound_left,
            s.ai_bound_right,
            s.racing_line,
        ] {
            row.push(format!("{v:.7}"));
        }
        row.push(s.section_id.to_string());
        row.push(s.flags.to_string());
        println!("{}", row.join(","));
    }
    debug!("{} sample(s) at {steps} per segment", samples.len());
    Ok(())
}

/// Writes a track's pad trigger volumes to stdout as CSV.
///
/// One row per pad, speedup pads first then weapon pads, each numbered within
/// its class - `pad 3` of class `speedup` is stable across runs because
/// [`vex::nodes`] walks the file in file order. `progress` is
/// arc-length distance from the start line along the course ring, and `offset`
/// is how far the pad's centre sits from its nearest ring point - large means
/// the pad is off the primary ring (a shortcut branch) and its `progress` and
/// approach points should not be trusted.
///
/// Every `--before N` adds three column triples: the ring point N units
/// upstream of the pad lifted to hover height (`beforeN_x/y/z`, where ships
/// fly and where a teleported craft should be placed), the spline tangent
/// there (`beforeN_tan_*`, the direction to face and to align velocity with)
/// and the surface up (`beforeN_up_*`, the negated authored `down`).
pub(crate) fn dump_pads(source: &str, name: &str, before: &[f32]) -> Result<()> {
    let blob = read_track_blob(source, name)?;
    let ai = ai_of(&blob, name)?;
    let nodes = vex::nodes(&blob).context("walking the node tree")?;

    let start = nodes
        .iter()
        .find(|n| n.class_id == vex::CLASS_START_POSITION)
        .and_then(|n| blob.get(n.payload()))
        .and_then(|payload| track::start_position(payload, vex::byte_order(&blob)));
    let course = Course::from_track(&ai, start.map(|s| Vec3::from_array(s.position)))
        .context("the track's primary spline chain does not close into a ring")?;

    // The same resampling the ring itself was built from, so a ring index maps
    // back onto a full sample - tangent and down - by nearest position.
    let samples = resample(&ai, Course::STEPS_PER_SEGMENT);

    let mut header =
        vec!["pad,class,centre_x,centre_y,centre_z,dir_x,dir_y,dir_z,progress,offset".to_owned()];
    for n in before {
        for triple in ["", "_tan", "_up"] {
            for axis in ["x", "y", "z"] {
                header.push(format!("before{n:.0}{triple}_{axis}"));
            }
        }
    }
    println!("{}", header.join(","));

    let classes = [
        ("speedup", vex::CLASS_SPEEDUP_PAD),
        ("weapon", vex::CLASS_WEAPON_PAD),
    ];
    let mut total = 0usize;
    for (label, class_id) in classes {
        let volumes = oag_vex::pads::volumes(&blob, &nodes, class_id);
        for (index, pad) in volumes.iter().enumerate() {
            let centre = Vec3::from_array(pad.centre());
            let direction = pad.direction().unwrap_or([0.0; 3]);
            let located = course
                .locate(centre, None)
                .context("locating a pad on the course ring")?;

            let mut row = vec![index.to_string(), label.to_owned()];
            for v in [centre.to_array(), direction] {
                row.extend(v.iter().map(|c| format!("{c:.7}")));
            }
            row.push(format!("{:.7}", located.progress));
            row.push(format!("{:.7}", located.offset));

            for n in before {
                // "N units before" walks the long way round: forward by
                // `length - N` is backward by N on a closed ring, and stays
                // inside `advance`'s documented contract.
                let upstream = course.advance(located.index, course.length() - n);
                let point = course
                    .position(upstream)
                    .context("an advanced ring index is always in range")?;
                let sample = samples
                    .iter()
                    .map(|(_, s)| s)
                    .min_by(|a, b| {
                        let da = point.distance_squared(Vec3::from_array(a.pos));
                        let db = point.distance_squared(Vec3::from_array(b.pos));
                        da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .context("a course ring always has samples")?;
                let lift = [
                    point.x - track::HOVER_LIFT * sample.down[0],
                    point.y - track::HOVER_LIFT * sample.down[1],
                    point.z - track::HOVER_LIFT * sample.down[2],
                ];
                let up = [-sample.down[0], -sample.down[1], -sample.down[2]];
                for v in [lift, sample.tangent, up] {
                    row.extend(v.iter().map(|c| format!("{c:.7}")));
                }
            }
            println!("{}", row.join(","));
            total += 1;
        }
    }
    debug!(
        "{total} pad(s), course length {:.1}, start at ring index {}",
        course.length(),
        course.start_index()
    );
    Ok(())
}
