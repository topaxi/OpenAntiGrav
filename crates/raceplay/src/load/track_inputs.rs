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
    /// A foreign track's drawn model: its art meshes (ADR-0076), or its
    /// flat-shaded floor and wall when it has none. `None` for a disc circuit.
    #[cfg(feature = "view")]
    pub drawn: Option<Model>,
    /// What of a foreign track's art is drawn beside [`Self::drawn`]: its sky,
    /// its flare, its sun's shadow direction. Default for a disc circuit.
    #[cfg(feature = "view")]
    pub art: crate::foreign::art::Extras,
}

pub(super) fn resolve(
    options: &Options,
    archives: &mut Archives,
    title: &'static oag_title::Title,
    report: &mut Vec<String>,
) -> Result<Inputs> {
    if let Some(path) = &options.foreign_track {
        return foreign(path, options, archives, report);
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

    // `oag_render::track::load`'s read, without the renderer: the same
    // container open and the same parse.
    let data = oag_assets::Container::open(&spec)?.read_entry(&track)?;
    let ai = oag_vex::track::from_vex(&data, &track).map_err(anyhow::Error::msg)?;
    let label = track.clone();
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
            if !(options.mode.has_opponents()
                || options.opponents
                || options
                    .mode
                    .is_multiplayer_time_trial(usize::from(options.people_flying())))
            {
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
        #[cfg(feature = "view")]
        drawn: None,
        #[cfg(feature = "view")]
        art: Default::default(),
    })
}

/// A foreign track in the shapes a disc circuit is read into. Everything the
/// content does not author is named in the report by `crate::foreign`, and left
/// out here rather than borrowed from the host: no `.vex` blob, so no fog,
/// clouds, track emitters or PVS. Its art, where the file has some, is drawn
/// by `crate::foreign::art`.
#[cfg_attr(not(feature = "view"), allow(unused_variables))]
fn foreign(
    path: &std::path::Path,
    options: &Options,
    archives: &mut Archives,
    report: &mut Vec<String>,
) -> Result<Inputs> {
    if options.mode == Mode::Zone {
        anyhow::bail!("Zone is defined by a title's own Zone circuits; a foreign track has none");
    }
    #[cfg_attr(not(feature = "view"), allow(unused_mut))]
    let (mut f, lines) = crate::foreign::read_any(path)?;
    report.extend(lines);
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
    #[cfg(feature = "view")]
    let flat =
        render_collision::build_model(&f.title, &f.nodes, render_collision::Style::Solid, false);
    #[cfg(feature = "view")]
    let (drawn, art) = match f.art.take() {
        Some(art) => {
            use crate::foreign::art;
            let dir = path.parent().unwrap_or_else(|| std::path::Path::new(""));
            let textures = art::resolve_textures(&art, dir, archives, report);
            let extras = art::extras(&art, &textures, report);
            let mut model = art::model(&f.title, &art, textures, report);
            if model.indices.is_empty() {
                report.push("art: no mesh drawn; the collision is drawn flat-shaded".into());
                model = flat;
            } else if model.radius < flat.radius {
                // The far plane and the framing read the track model's sphere: a
                // track whose art is a few decals still spans its whole collision.
                (model.centre, model.radius) = (flat.centre, flat.radius);
            }
            (model, extras)
        }
        None => {
            report.push(
                "the track has no art section: its collision is drawn flat-shaded, with no \
                 sky, sun or lens flare"
                    .into(),
            );
            (flat, Default::default())
        }
    };
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
        #[cfg(feature = "view")]
        drawn: Some(drawn),
        #[cfg(feature = "view")]
        art,
    })
}

/// A disc circuit as a track in this project's own format: the same inputs
/// [`resolve`] reads for a race, moved into [`oag_trackfile::Track`]. **Local
/// use only** - what it returns is content from a disc and must never be
/// committed (`just audit-leakage`).
///
/// A Pulse or Pure circuit (one whose `.vex` carries its own geometry) exports
/// its art too ([`circuit_art`]); what a track file has no field for is
/// dropped, and the track's notes say so (`oag_trackfile::ABSENT` is the list).
pub(super) fn export(
    options: &Options,
    archives: &mut Archives,
    title: &'static oag_title::Title,
) -> Result<oag_trackfile::Track> {
    let mut report = Vec::new();
    let inputs = resolve(options, archives, title, &mut report)?;
    if inputs.track.starts_with("foreign:") {
        anyhow::bail!(
            "{}: already a foreign track, nothing to export from a disc",
            inputs.track
        );
    }
    let start = inputs
        .start_position
        .context("the circuit authors no Start Position node; a track file needs one")?;
    let mut notes = vec![format!(
        "exported from {}'s circuit {} by this project's local exporter (not for redistribution)",
        title.name, inputs.track
    )];
    #[cfg(feature = "view")]
    let art = circuit_art(
        archives,
        title,
        &inputs.track,
        &inputs.track_blob,
        &mut report,
    )?;
    #[cfg(not(feature = "view"))]
    let art = None;
    notes.extend(report);
    Ok(oag_trackfile::Track {
        id: inputs.track,
        title: inputs.label,
        ai: inputs.ai,
        start,
        nodes: inputs.nodes,
        speedup_pads: inputs.speedup_pads,
        weapon_pads: inputs.weapon_pads,
        art,
        notes,
    })
}

/// A track file's sky: the `Skycube` of the host circuit its art names, built
/// the way a disc circuit's own is. `None`, with the reason in `report`, when
/// the entry does not read or authors no sky.
#[cfg(feature = "view")]
pub(super) fn foreign_sky(
    archives: &mut Archives,
    entry: &str,
    report: &mut Vec<String>,
) -> Option<Model> {
    let blob = match archives.read_name(entry) {
        Ok(blob) => blob,
        Err(e) => {
            report.push(format!(
                "art sky: {entry} does not read ({e}); the sky stays black"
            ));
            return None;
        }
    };
    // The PS2's texture set only when the file embeds none, as the track model's
    // own build decides it: a preceding entry that happens to parse must not
    // re-skin a PSP sky.
    let built = mesh::build_sky(entry, &blob, None).and_then(|sky| {
        if !sky.textures.is_empty()
            && sky.textures.iter().all(Option::is_none)
            && let Some(set) = ps2_texture_set(archives, entry)
        {
            mesh::build_sky(entry, &blob, Some(&set))
        } else {
            Ok(sky)
        }
    });
    let mut sky = match built {
        Ok(sky) if !sky.indices.is_empty() => sky,
        Ok(_) => {
            report.push(format!(
                "art sky: {entry} authors no Skycube; the sky stays black"
            ));
            return None;
        }
        Err(e) => {
            report.push(format!("art sky: {entry}: {e}; the sky stays black"));
            return None;
        }
    };
    super::environment::unlit_sky(&mut sky, report);
    report.push(format!(
        "art sky: {entry}'s Skycube, {} triangle(s)",
        sky.indices.len() / 3
    ));
    Some(sky)
}

/// A Pulse or Pure circuit's art: the track model its `.vex` builds with only
/// the finest tier of each `LodGroup` (a track file has no level of detail),
/// its textures as references to its own `Texture` nodes, its sky, and on a
/// title whose flare law applies, its `lensflare` node as the sun. `None`, and
/// a line in `report`, for a circuit whose geometry is not in its `.vex` (HD,
/// Omega, 2048: checked, applies, not wired).
#[cfg(feature = "view")]
pub(super) fn circuit_art(
    archives: &mut Archives,
    title: &'static oag_title::Title,
    track: &str,
    blob: &[u8],
    report: &mut Vec<String>,
) -> Result<Option<oag_trackfile::art::Art>> {
    use crate::foreign::art;
    if oag_vex::vex::geometry_is_external(blob) {
        report.push(
            "art not exported: this circuit's geometry is in its .rcsmodel, which the exporter \
             does not write yet"
                .into(),
        );
        return Ok(None);
    }
    let model = circuit_model(archives, track, blob)?;
    let names = art::texture_names(blob)?;
    let mut out = art::from_model(&model, track, &names);
    art::lights_from_track(&mut out, blob, &oag_vex::vex::nodes(blob)?);
    let moving = model.vertices.iter().filter(|v| v.xform != 0).count();
    let scrolling = model.vertices.iter().filter(|v| v.anim != 0).count();
    report.push(format!(
        "art exported: {} mesh(es), {} texture reference(s); not carried: {moving} vertices under \
         an Anim Transform (placed at time zero), {scrolling} with a texture animation, {} \
         extra-pass draw(s), each draw's depth point and glow",
        out.meshes.len(),
        out.textures.len(),
        model.shine_draws.len(),
    ));
    if mesh::build_sky(track, blob, None).is_ok_and(|sky| !sky.indices.is_empty())
        || ps2_texture_set(archives, track)
            .and_then(|set| mesh::build_sky(track, blob, Some(&set)).ok())
            .is_some_and(|sky| !sky.indices.is_empty())
    {
        out.sky = Some(track.to_string());
    }
    if title.looks.lens_flare.applies(archives.layout.platform) {
        let mut quiet = Vec::new();
        if let Some(flare) = super::lens_flare::read(blob, &mut quiet) {
            art::flare_as_sun(&mut out, &flare, &names);
        }
    }
    Ok(Some(out))
}

/// A circuit's track model as the race loader builds it (the PS2's texture set
/// beside it included), with each `LodGroup`'s finest tier only.
#[cfg(feature = "view")]
pub fn circuit_model(archives: &mut Archives, track: &str, blob: &[u8]) -> Result<Model> {
    let mut model = mesh::build_with_textures(track, blob, None)?;
    if !model.textures.is_empty()
        && model.textures.iter().all(Option::is_none)
        && let Some(external) = ps2_texture_set(archives, track)
    {
        model = mesh::build_with_textures(track, blob, Some(&external))?;
    }
    model.keep_nearest();
    Ok(model)
}
