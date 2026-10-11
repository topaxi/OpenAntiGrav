//! The track-derived inputs of a race: which circuit, its spline, its start slot,
//! its collision and its pads - from a disc's `.vex`, or from a foreign track
//! (`crate::foreign`) mapped into the same shapes.
//!
//! Split out of `load`, which is at this project's 1,000-line ceiling; the disc
//! half is moved as it was, the foreign half is one early return.

use super::*;
use oag_assets::Archives;
use oag_vex::collision::CollisionNode;
use oag_vex::pads::PadVolume;
use oag_vex::vex::classes::Classes;

/// What the rest of [`load`](super::load) needs of the circuit.
pub(super) struct Inputs {
    pub track: String,
    pub ai: AiTrack,
    pub label: String,
    pub track_blob: Vec<u8>,
    pub track_classes: Classes,
    pub start_position: Option<StartPosition>,
    pub nodes: Vec<CollisionNode>,
    pub collision: CollisionWorld,
    pub speedup_pads: Vec<PadVolume>,
    pub weapon_pads: Vec<PadVolume>,
    /// The foreign track's flat-shaded floor and wall, drawn in place of the
    /// art meshes it has no mapping for. `None` for a disc circuit.
    pub drawn: Option<Model>,
}

pub(super) fn resolve(
    options: &Options,
    archives: &mut Archives,
    title: &'static oag_title::Title,
    report: &mut Vec<String>,
) -> Result<Inputs> {
    if let Some(path) = &options.foreign_track {
        return foreign(path, options, report);
    }
    // **The circuit this source actually offers**, resolved here because this is
    // the first point the title is known - see [`Options::track`]. Reported
    // either way, so a run that took a default says which title's default it was
    // rather than leaving a reader to work it out from the path.
    let track = match &options.track {
        Some(asked) => asked.clone(),
        None => {
            report.push(format!(
                "no track named: {}'s own default, {}",
                title.name, title.race.track
            ));
            title.race.track.to_string()
        }
    };

    // **Zone flies a different environment, and the disc authors it.** A Zone
    // circuit ships its own meshes, lights, `fogCube` and one-material
    // `Skycube`, so the mode's whole look is a file to open rather than a filter
    // to apply - `CLAUDE.md`'s rule about not inventing what the assets already
    // carry, and the reason nothing here tints or desaturates anything.
    //
    // Which file, though, is a title fact: Pulse keeps it beside the race
    // circuit under a prefixed name, Pure and HD keep whole circuits of their
    // own. `ZoneCircuit` is that axis, measured on all three.
    let track = if options.mode == Mode::Zone {
        let zone = match &options.track {
            Some(_) => title.race.zone.variant_of(&track),
            None => title.race.zone.default_track(title.race.track),
        };
        if zone == track {
            report.push(format!("zone: racing {track} as named"));
        } else {
            report.push(format!("zone: {track} -> {zone}"));
        }
        zone
    } else {
        track
    };

    log::info!("racing on {}: {track}", title.name);

    let spec = archives
        .locate(&track)
        .ok_or_else(|| assets::zone_circuit_miss(&track, options.mode, title, archives))?
        .to_string();

    let (ai, label) = track_render::load(&spec, &track)?;
    report.push(format!(
        "{label}: {} path(s), {} junction(s), {} control point(s)",
        ai.paths.len(),
        ai.junctions.len(),
        ai.point_count()
    ));

    // Read again for the collision nodes: `oag_render::track::load` takes an
    // archive rather than bytes, so this blob is decompressed twice (see
    // `docs/tools/oag-game.md`); that costs most on the PS2, where 5,861 of
    // `WADS2.WAD`'s 7,200 entries are LZSS.
    let track_blob = archives
        .read_name(&track)
        .with_context(|| format!("reading {track} out of {}", archives.layout.describe()))?;
    stages::reach(stages::Stage::TrackRead);
    // **This track file's own class numbering.** Read once, off its version word,
    // and consulted everywhere below that used to spell a `vex::CLASS_*`
    // constant - all of which are version 6's. A version-4 track (Pure's are)
    // uses different numbers entirely, so the constants would find nothing here
    // or, worse, find the wrong node type. An id this project has not recovered
    // for this version comes back `None`, and each site below says what it does
    // with that rather than substituting version 6's answer. See
    // `oag_vex::vex::classes`.
    let track_classes =
        oag_vex::vex::classes_of(&track_blob).map_err(|e| anyhow::anyhow!("{}: {e}", track))?;
    let speedup_pad_class = track_classes.speedup_pad;
    let weapon_pad_class = track_classes.weapon_pad;
    let start_position = start_position_of(&track_blob);
    match start_position {
        Some(slot) => {
            report.push(format!(
                "Start Position: {:?} facing {:?}",
                slot.position, slot.forward
            ));
            // Slot 8 (this node) is measured for a full grid; a solo mode
            // instead uses this project's own reading of slot 1 - see
            // `Race::start`.
            if !(options.mode.has_opponents() || options.opponents) {
                report.push(
                    "solo mode: player on grid slot 1 (front), this project's own reading, \
                     not a separate measurement"
                        .to_string(),
                );
            }
        }
        None => report.push("no Start Position node: spawning on the spline instead".to_string()),
    }
    let nodes = surfaces::of_track(archives, &track, &track_blob, report)?;
    let collision = collision_world(&nodes);
    report.push(format!(
        "{} collision node(s) -> {} collider(s), {} triangle(s)",
        nodes.len(),
        collision.colliders().len(),
        collision
            .colliders()
            .iter()
            .map(oag_physics::TriangleSoup::triangle_count)
            .sum::<usize>()
    ));

    // The pads' trigger volumes, from the same blob the geometry comes from.
    // Reported with the distance from the grid to the nearest one, because that
    // is the number anyone testing a pad needs and there is nowhere else to get
    // it: a pad is a plate on the track surface with nothing to distinguish it
    // in a screenshot.
    let speedup_pads = oag_vex::vex::nodes(&track_blob)
        .map(|nodes| {
            speedup_pad_class
                .map(|class| oag_vex::pads::volumes(&track_blob, &nodes, class))
                .unwrap_or_default()
        })
        .unwrap_or_default();
    if speedup_pads.is_empty() {
        report.push("the track authors no Speedup Pad trigger volumes".to_string());
    } else {
        let nearest = start_position.map(|slot| {
            speedup_pads
                .iter()
                .map(|pad| Vec3::from_array(slot.position).distance(Vec3::from_array(pad.centre())))
                .fold(f32::INFINITY, f32::min)
        });
        report.push(match nearest {
            Some(d) => format!(
                "{} speedup pad trigger volume(s), nearest {d:.0} units from the grid",
                speedup_pads.len()
            ),
            None => format!("{} speedup pad trigger volume(s)", speedup_pads.len()),
        });
    }
    // The pickup pads, decoded the same way and **unconditionally**, unlike the
    // mesh below: nothing triggers them yet, so this is asset data rather than
    // gameplay, and `the_weapon_pads_are_drawn_where_they_trigger` checks the
    // decode against the drawn geometry regardless of mode. The original
    // decodes them unconditionally too - `World_CollectNodeLists` always walks
    // the tree - it only suppresses what a *weapons-off* mode does with the
    // result afterward, which is what `weapon_pad_model`'s own report line
    // below says plainly rather than repeating here.
    let weapon_pads = oag_vex::vex::nodes(&track_blob)
        .map(|nodes| {
            weapon_pad_class
                .map(|class| oag_vex::pads::volumes(&track_blob, &nodes, class))
                .unwrap_or_default()
        })
        .unwrap_or_default();
    report.push(if weapon_pads.is_empty() {
        "the track authors no Weapon Pad trigger volumes".to_string()
    } else {
        format!("{} weapon pad trigger volume(s)", weapon_pads.len())
    });
    Ok(Inputs {
        track,
        ai,
        label,
        track_blob,
        track_classes,
        start_position,
        nodes,
        collision,
        speedup_pads,
        weapon_pads,
        drawn: None,
    })
}

/// A foreign track in the shapes a disc circuit is read into. Everything the
/// content does not author is named in the report by `crate::foreign`, and left
/// out here rather than borrowed from the host: no `.vex` blob, so no sky, fog,
/// clouds, art meshes, track emitters or PVS.
fn foreign(path: &std::path::Path, options: &Options, report: &mut Vec<String>) -> Result<Inputs> {
    if options.mode == Mode::Zone {
        anyhow::bail!("Zone is defined by a title's own Zone circuits; a foreign track has none");
    }
    let f = crate::foreign::read(path)?;
    report.extend(f.report.iter().cloned());
    let collision = collision_world(&f.nodes);
    report.push(format!(
        "{} collision node(s) -> {} collider(s), {} triangle(s)",
        f.nodes.len(),
        collision.colliders().len(),
        collision
            .colliders()
            .iter()
            .map(oag_physics::TriangleSoup::triangle_count)
            .sum::<usize>()
    ));
    let drawn =
        render_collision::build_model(&f.title, &f.nodes, render_collision::Style::Solid, false);
    Ok(Inputs {
        track: format!("foreign:{}", f.id),
        ai: f.ai,
        label: f.title,
        track_blob: Vec::new(),
        track_classes: oag_vex::vex::classes::for_version(6)
            .expect("version 6 is a recovered class table"),
        start_position: Some(f.start),
        nodes: f.nodes,
        collision,
        speedup_pads: f.speedup_pads,
        weapon_pads: f.weapon_pads,
        drawn: Some(drawn),
    })
}
