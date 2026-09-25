//! The start gantry's own placement and the track's authored visibility
//! partition - two readings off the same track file, built back to back
//! because the gantry's billboard-placeholder strip has to land on
//! `track_model` before the visibility build reads its final geometry.
//!
//! Split out of `load.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::super::*;

/// Places the start gantry (see [`gantry::place`]) and builds the track's
/// [`TrackVisibility`] partition, in that order - `track_model` is stripped
/// of its billboard-slot placeholders in between, which is why both live in
/// one function rather than two: a caller pulling them apart would have to
/// reproduce that ordering itself to get the same picture.
///
/// `has_ps3_geometry` is `ps3_geometry.is_some()` at the call site - only
/// presence is asked here, never the bytes themselves.
#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    archives: &mut oag_assets::Archives,
    track: &str,
    track_model: &mut Model,
    track_blob: &[u8],
    ai: &AiTrack,
    has_ps3_geometry: bool,
    vex_geometry: bool,
    start_position: Option<&StartPosition>,
    report: &mut Vec<String>,
) -> (Option<gantry::Placed>, Option<TrackVisibility>) {
    // The authored PVS. Skipped for a ribbon build, whose geometry is generated
    // from the spline rather than authored, so the section boxes have nothing
    // to say about it.
    // **What the circuit asks to be loaded with it, and what of that this
    // project does.** `trackstartup.xml` names up to eight billboard slots and
    // a sound bank; every model it names is on the disc. Seven of the eight are
    // still unplaced - where a hoarding attaches is unrecovered, so they stay
    // unwired rather than put somewhere plausible. **Slot 8 is not a hoarding
    // and is no longer among them**: it is the start gantry, and the circuit's
    // own track geometry authors the surface it stands on. See
    // `oag_tables::trackstartup` and `docs/rendering/start-gantry.md`.
    //
    // **Not HD-only.** Pulse ships the same file per circuit, `fexml`-shortened
    // rather than plain - `TrackStartup::parse` expands either form - so this
    // also runs on real geometry (`vex_geometry`), whose paths are `\`-joined
    // where HD's are `/`-joined; `rfind('/')` alone found nothing on Pulse.
    let mut gantry = None;
    // Whether anything named a slot 8 at all, which is what separates "the gantry
    // did not load" (`place` says so itself) from "nothing asked for one", which
    // otherwise reports nothing at all - see the block after this one.
    let mut named_slot_8 = false;
    if (has_ps3_geometry || vex_geometry)
        && let Some(name) = track
            .rfind(['/', '\\'])
            .map(|at| format!("{}/trackstartup.xml", &track[..at]))
        && let Ok(blob) = archives.read_name(&name)
    {
        let manifest =
            oag_tables::trackstartup::TrackStartup::parse(&String::from_utf8_lossy(&blob));
        let models = manifest
            .billboards
            .iter()
            .filter(|b| b.location().is_some())
            .count();
        report.push(format!(
            "{name}: {} billboard slot(s), {models} naming a model and {} a colour - \
             slots 1-7 unplaced, because what a hoarding attaches to is unrecovered{}",
            manifest.billboards.len(),
            manifest.billboards.len() - models,
            match &manifest.sound_bank {
                Some(bank) => format!("; sound bank {bank}"),
                None => String::new(),
            },
        ));
        // The manifest's own spelling of the model, not a constant here:
        // every Pulse circuit names the same file, and a source that names
        // another gets that one drawn rather than Pulse's substituted for it.
        if let Some(model) = manifest.billboard(8).and_then(|b| b.location()) {
            named_slot_8 = true;
            gantry =
                super::super::gantry::place(archives, model, track_model, start_position, report);
        }
    }
    // **Draw nothing and say so**, on the one route into `place` that reports
    // nothing itself: a circuit whose manifest is missing, unreadable, or
    // names no slot 8 never calls it. That circuit can still have a measurable
    // mount, so the silence would read exactly like a circuit that has none.
    if !named_slot_8 && let Some(mount) = oag_render::gantry::mount(track_model) {
        report.push(format!(
            "no start gantry: nothing named slot 8 - no manifest, or one that names no \
             model for it - although this circuit does author a mount for one, a \
             {:.1} x {:.1} panel at {:?}",
            mount.width,
            mount.height,
            mount.centre.to_array().map(|v| (v * 10.0).round() / 10.0),
        ));
    }
    // The billboard slots are never artwork - see `strip_slot_placeholders`'s own doc.
    let stripped = oag_render::gantry::strip_slot_placeholders(track_model);
    if stripped > 0 {
        report.push(format!("{stripped} billboard-slot placeholder draw(s) suppressed: drawn nothing rather than the stub"));
    }
    let visibility = if vex_geometry {
        TrackVisibility::build(track_model, track_blob, ai)
    } else if has_ps3_geometry {
        TrackVisibility::from_hd_pvs(archives, track, report)
    } else {
        None
    };
    // A PS3 circuit reports its own line from `from_hd_pvs`, so only the
    // section partition and its absence are described here.
    match &visibility {
        Some(v) if v.chunks().is_none() => report.push(format!(
            "{} authored visibility section(s); {} of {} draw call(s) governed by one \
             ({:.1}%), the rest always drawn; {} LOD-swap pair(s)",
            v.pvs.len(),
            v.placement.placed,
            v.placement.total(),
            v.placement.placed_fraction() * 100.0,
            v.swap_pairs(),
        )),
        None if vex_geometry => report.push(
            "no authored visibility sections: PVS culling is unavailable on this track".to_string(),
        ),
        _ => {}
    }
    (gantry, visibility)
}
