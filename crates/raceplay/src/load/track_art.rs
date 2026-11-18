//! A circuit's art beside its track inputs: a foreign track's drawn model, the skies a
//! track file borrows, and a disc circuit exported into this project's track format.

use super::*;
use oag_assets::Archives;
#[cfg(feature = "view")]
use oag_trackfile::art::PadPlate;

/// A foreign track's drawn model and what is drawn beside it, from the art section
/// [`super::track_inputs::Inputs::art`] carried out: its art meshes (ADR-0076), or its
/// flat-shaded floor and wall when it has none; its sky, its flare, its sun's shadow
/// direction.
#[cfg(feature = "view")]
#[allow(clippy::too_many_arguments)]
pub(super) fn foreign_drawn(
    path: &std::path::Path,
    label: &str,
    nodes: &[oag_vex::collision::CollisionNode],
    art: Option<oag_trackfile::art::Art>,
    speedup_pads: &[oag_vex::pads::PadVolume],
    weapon_pads: &[oag_vex::pads::PadVolume],
    title: &'static oag_title::Title,
    archives: &mut Archives,
    report: &mut Vec<String>,
) -> (Model, crate::foreign::art::Extras) {
    let flat = render_collision::build_model(label, nodes, render_collision::Style::Solid, false);
    match art {
        Some(art) => {
            use crate::foreign::art;
            let dir = path.parent().unwrap_or_else(|| std::path::Path::new(""));
            let textures = art::resolve_textures(&art, dir, archives, report);
            let mut extras = art::extras(&art, &textures, report);
            art::host_pads(
                &mut extras,
                Some(&art),
                speedup_pads,
                weapon_pads,
                title.race.track,
                archives,
                report,
            );
            let mut model = art::model(label, &art, textures, report);
            if art.draw_collision && !model.indices.is_empty() {
                model = art::under_collision(&flat, model, report);
            }
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
            let mut extras = crate::foreign::art::Extras::default();
            crate::foreign::art::host_pads(
                &mut extras,
                None,
                speedup_pads,
                weapon_pads,
                title.race.track,
                archives,
                report,
            );
            (flat, extras)
        }
    }
}

/// A disc circuit as a track in this project's own format: the same inputs
/// [`super::track_inputs::resolve`] reads for a race, moved into [`oag_trackfile::Track`]. **Local
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
    let inputs = super::track_inputs::resolve(options, archives, title, title, &mut report)?;
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
        channels: Vec::new(),
        id: inputs.track,
        title: inputs.label,
        host: Some(oag_source::layers::slug(title)),
        titles: Vec::new(),
        ai: inputs.ai,
        start,
        nodes: inputs.nodes,
        speedup_pads: inputs.speedup_pads,
        weapon_pads: inputs.weapon_pads,
        props: oag_trackfile::props::Props::new(),
        speedup_pad_props: Vec::new(),
        weapon_pad_props: Vec::new(),
        path_props: Vec::new(),
        triggers: Vec::new(),
        art,
        loading: None,
        notes,
    })
}

/// **The host title's Zone sky for a track file that carries no sky of its own**,
/// the one Zone-only part that is a title default rather than a circuit's file
/// (`RaceDefaults::zone_sky`, HD's `ZoneSky.gtf`). `None` for a disc circuit, for
/// a race that is not Zone and for a title with no Zone sky (Pulse keeps its Zone
/// sky in the circuit's Zone file, which a track file has none of).
#[cfg(feature = "view")]
pub(super) fn host_zone_sky(
    archives: &mut Archives,
    track: &str,
    (title, mode, store): (
        &'static oag_title::Title,
        Mode,
        oag_gpu::formats::SceneStore,
    ),
    report: &mut Vec<String>,
) -> Option<Model> {
    if !track.starts_with("foreign:") || title.race.zone_sky.is_none() {
        return None;
    }
    super::environment::hd_sky_model(archives, track, title.race, mode, store, report)
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
    for which in [PadPlate::Speedup, PadPlate::Weapon] {
        let pads = circuit_pads(archives, track, blob, which)?;
        out.meshes.extend(
            art::from_model(&pads, track, &names)
                .meshes
                .into_iter()
                .map(|mut m| {
                    m.name = format!("{} pad {}", which_name(which), m.name);
                    m.pad = Some(which);
                    m
                }),
        );
    }
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

#[cfg(feature = "view")]
fn which_name(which: PadPlate) -> &'static str {
    match which {
        PadPlate::Speedup => "speed",
        PadPlate::Weapon => "weapon",
    }
}

/// A circuit's pad plates of one kind as the race loader builds them
/// (`mesh::build_pads`, `mesh::build_weapon_pads`, the PS2's texture set beside
/// it when the file embeds none). Their texture slots index the same `Texture`
/// nodes the track model's do.
#[cfg(feature = "view")]
pub fn circuit_pads(
    archives: &mut Archives,
    track: &str,
    blob: &[u8],
    which: PadPlate,
) -> Result<Model> {
    let build = match which {
        PadPlate::Speedup => mesh::build_pads,
        PadPlate::Weapon => mesh::build_weapon_pads,
    };
    let mut model = build(track, blob, None)?;
    if !model.textures.is_empty()
        && model.textures.iter().all(Option::is_none)
        && let Some(external) = ps2_texture_set(archives, track)
    {
        model = build(track, blob, Some(&external))?;
    }
    Ok(model)
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
