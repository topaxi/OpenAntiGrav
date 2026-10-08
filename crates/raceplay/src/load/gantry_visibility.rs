//! The start gantry's own placement and the track's authored visibility
//! partition - two readings off the same track file, built back to back
//! because the gantry's billboard-placeholder strip has to land on
//! `track_model` before the visibility build reads its final geometry.
//!
//! Split out of `load.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::super::*;

/// Which advert pass `title` runs in `mode`, and why none does when it does not.
///
/// A Zone race on a title whose zone effects bind one shared texture in place of
/// the per-slot targets draws no advert: that texture is unread.
pub(super) fn adverts_for(
    title: &oag_title::Title,
    mode: Mode,
) -> (Option<&oag_title::adverts::Adverts>, &'static str) {
    let zone_shared = title
        .adverts
        .is_some_and(|spec| spec.zone_shares_one_texture && mode == Mode::Zone);
    let reason = if zone_shared {
        "a Zone race binds one shared billboard texture to every slot but 8 on this title and \
         that texture is unread"
    } else {
        "this title's advert pass is not measured"
    };
    (title.adverts.filter(|_| !zone_shared), reason)
}

/// The title's advert spec where slot 8 is drawn as a card
/// ([`oag_title::adverts::Adverts::gantry_card`]), whatever the mode does to
/// the other slots.
pub(super) fn gantry_card(title: &oag_title::Title) -> Option<&oag_title::adverts::Adverts> {
    title.adverts.filter(|spec| spec.gantry_card)
}

/// Places the start gantry (see [`gantry::place`]) and builds the track's
/// [`TrackVisibility`] partition, in that order - `track_model` is stripped
/// of its billboard-slot placeholders in between, which is why both live in
/// one function rather than two: a caller pulling them apart would have to
/// reproduce that ordering itself to get the same picture.
///
/// `ps3_geometry` is the `.rcsmodel` beside the `.vex`, and `geometry_name` the
/// name it was read under: the presence decides which path runs, and the bytes
/// are asked for one thing only - a 2048-lineage model's mesh-object count, to
/// check a `.pvs` against.
#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    archives: &mut oag_assets::Archives,
    track: &str,
    track_model: &mut Model,
    track_blob: &[u8],
    ai: &AiTrack,
    ps3_geometry: Option<&[u8]>,
    geometry_name: Option<&str>,
    vex_geometry: bool,
    (adverts_spec, adverts_off): (Option<&oag_title::adverts::Adverts>, &str),
    gantry_card: Option<&oag_title::adverts::Adverts>,
    start_position: Option<&StartPosition>,
    report: &mut Vec<String>,
) -> (
    Option<gantry::Placed>,
    Option<TrackVisibility>,
    Vec<crate::adverts::Card>,
) {
    // The authored PVS. Skipped for a ribbon build, whose geometry is generated
    // from the spline rather than authored, so the section boxes have nothing
    // to say about it.
    // **What the circuit asks to be loaded with it, and what of that this
    // project does.** `trackstartup.xml` names up to eight billboard slots and
    // a sound bank; every model it names is on the disc. **Slots 1-7 are drawn
    // through their own camera into the texture the circuit's `billboardN`
    // quads show** (`crate::adverts`), where the circuit authors such a quad.
    // **Slot 8 is not a hoarding**: it is the start gantry, and the circuit's
    // own track geometry authors the surface it stands on. See
    // `oag_tables::trackstartup` and `docs/rendering/start-gantry.md`.
    //
    // **Not HD-only.** Pulse ships the same file per circuit, `fexml`-shortened
    // rather than plain - `TrackStartup::parse` expands either form - so this
    // also runs on real geometry (`vex_geometry`), whose paths are `\`-joined
    // where HD's are `/`-joined; `rfind('/')` alone found nothing on Pulse.
    let has_ps3_geometry = ps3_geometry.is_some();
    let mut gantry = None;
    let mut adverts = Vec::new();
    // Whether anything named a slot 8 at all, which is what separates "the gantry
    // did not load" (`place` says so itself) from "nothing asked for one", which
    // otherwise reports nothing at all - see the block after this one.
    let mut named_slot_8 = false;
    let mut unserved_why: Vec<(u32, String)> = Vec::new();
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
            "{name}: {} billboard slot(s), {models} naming a model and {} a colour{}",
            manifest.billboards.len(),
            manifest.billboards.len() - models,
            match &manifest.sound_bank {
                Some(bank) => format!("; sound bank {bank}"),
                None => String::new(),
            },
        ));
        // An advert is drawn through its own camera into a texture the track's
        // placeholder quads show (`crate::adverts`), on a title whose pass was
        // measured. Which titles those are, and with what target, is
        // `oag_title::Title::adverts`: not geometry kind, because a 2048-lineage
        // model is PS3-shaped too and nothing of its pass is measured.
        if let Some(spec) = adverts_spec {
            adverts = crate::adverts::load(archives, &manifest, track_model, spec, report);
        }
        // Why each slot that got no advert got none, for the line that counts the
        // draws left undrawn below.
        for billboard in manifest.billboards.iter().filter(|b| b.num != 8) {
            if adverts.iter().any(|card| card.slot == billboard.num) {
                continue;
            }
            let why = match (adverts_spec, billboard.location()) {
                (None, _) => adverts_off.to_string(),
                (Some(spec), None) if !spec.colour_pool => {
                    "its colour fill's pool order is not measured on this title".to_string()
                }
                (Some(_), None) => "no catalogue entry answers its colour".to_string(),
                (Some(_), Some(_)) => "its advert did not load".to_string(),
            };
            unserved_why.push((billboard.num, why));
        }
        // The manifest's own spelling of the model, not a constant here:
        // every Pulse circuit names the same file, and a source that names
        // another gets that one drawn rather than Pulse's substituted for it.
        if let Some(model) = manifest.billboard(8).and_then(|b| b.location()) {
            named_slot_8 = true;
            if let Some(spec) = gantry_card {
                adverts.extend(super::super::gantry::place_card(
                    archives,
                    model,
                    track_model,
                    spec,
                    report,
                ));
            } else {
                // Pulse's clock is measured; the PS3 titles run HD's own
                // race-manager window off their asset's `GO` edge (`gantry::clock`).
                let clock = if has_ps3_geometry {
                    super::super::gantry::ClockRule::HdRaceManager
                } else {
                    super::super::gantry::ClockRule::Measured
                };
                gantry = super::super::gantry::place(
                    archives,
                    model,
                    track_model,
                    start_position,
                    clock,
                    report,
                );
            }
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
    // A slot is never artwork - see `strip_slot_placeholders`'s own doc. Slot 8's
    // quad is the start gantry's, and the slots with a card keep their draws:
    // `Scene::new` points those at the card's target.
    if gantry.is_some() {
        let replaced = oag_render::gantry::strip_slot_placeholder(track_model, 8);
        if replaced > 0 {
            report.push(format!(
                "{replaced} billboard8 placeholder draw(s) replaced by the start gantry"
            ));
        }
    }
    let placeholders = oag_render::gantry::placeholder_texture_slots(track_model);
    let served: Vec<u32> = adverts.iter().map(|card| card.slot).collect();
    if adverts_spec.is_some_and(|spec| spec.flip_v) {
        let moved = oag_render::gantry::flip_served_placeholder_v(track_model, &served);
        if moved > 0 {
            report.push(format!(
                "{moved} billboard vertice(s) sample their advert with V negated, as the original's \
                 draws do (TEXSCALE V = -1)"
            ));
        }
    }
    let stripped = oag_render::gantry::strip_unserved_slot_placeholders(track_model, &served);
    if stripped > 0 {
        // Named by slot with the reason, so the line says what is missing rather than
        // only that something is.
        let mut named: Vec<String> = unserved_why
            .iter()
            .filter(|(num, _)| placeholders.iter().any(|&(_, n)| n == *num))
            .map(|(num, why)| format!("slot {num}: {why}"))
            .collect();
        if named.is_empty() {
            named.push(format!(
                "{adverts_off}, or the circuit's manifest names no advert for them"
            ));
        }
        report.push(format!(
            "{stripped} billboard-slot placeholder draw(s) suppressed: drawn nothing rather than the stub ({})",
            named.join("; ")
        ));
    }
    let visibility = if vex_geometry {
        TrackVisibility::build(track_model, track_blob, ai)
    } else if has_ps3_geometry {
        // A 2048-lineage model's `.pvs` is indexed by its mesh objects, whose
        // count is the check that the file belongs to it; HD's is not asked.
        let model_chunks = ps3_geometry
            .filter(|blob| oag_mesh::mesh::rcs::psp2::is_psp2(blob))
            .and_then(|blob| oag_rcs::rcsmodel::psp2::parse(blob).ok())
            .map(|model| model.scene.meshes.len());
        TrackVisibility::from_hd_pvs(archives, track, geometry_name, model_chunks, report)
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
    (gantry, visibility, adverts)
}
