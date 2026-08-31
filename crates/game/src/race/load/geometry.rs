//! Which surface a circuit draws: its own art, or the ribbon this build derives.
//!
//! Its own module because `super` is at this project's 1,000-line ceiling, and
//! because the choice is one decision made in one place - a `.rcsmodel` that is
//! absent, one that will not decode, and one in a container this build reads
//! are three outcomes with one fallback between them.

use oag_render::mesh::{self, Model};

/// The circuit's authored geometry, or `None` when the caller should draw the
/// derived ribbon instead.
///
/// **Two containers wear the `.rcsmodel` extension.** Wipeout HD's needs the
/// `.vex` beside it - a chunk is addressed by hash and its vertex stride comes
/// from the node's authored box - and Wipeout 2048's needs neither, so it takes
/// a builder of its own rather than a branch inside HD's. See
/// `oag_render::mesh::rcs::psp2`.
///
/// A sibling that will not decode falls back exactly as a missing one does, and
/// the report says which happened: they need different work to fix.
///
/// # The second element
///
/// **The circuit's `Weapon Pad` chunks, kept apart on purpose.** A PS3
/// circuit's weapon pads are baked into the same world-space chunk pass as
/// the road (`oag_render::mesh::rcs::build_scene`'s own doc comment explains
/// why), so this is the only place that pass ever has a `Mode` to gate them
/// by - the caller wires this straight into the same `weapon_pad_model` slot
/// `Scene::new` already gates by `oag_race::Mode::weapons_enabled` for the
/// PSP-shaped path. `None` for a ribbon build, a Wipeout 2048 circuit (its
/// `.rcsmodel` carries no comparable node tree, so a caller reaches its pads a
/// different way if it ever does), and a `.vex` version with no recovered
/// `weapon_pad` class id.
pub(super) fn track_model(
    archives: &mut oag_assets::Archives,
    track: &str,
    track_blob: &[u8],
    geometry: &Option<Vec<u8>>,
    report: &mut Vec<String>,
) -> (Option<Model>, Option<Model>) {
    match geometry {
        None => (None, None),
        // **Two containers wear this extension.** Wipeout HD's needs the `.vex`
        // beside it - a chunk is addressed by hash and its stride comes from the
        // node's box - and Wipeout 2048's needs neither, so it takes a builder
        // of its own rather than a branch inside HD's. See
        // `oag_render::mesh::rcs::psp2`.
        Some(geometry) if mesh::rcs::psp2::is_psp2(geometry) => {
            match mesh::rcs::psp2::build(track, geometry, &mut |path| archives.read_name(path).ok())
            {
                Ok((model, built)) => {
                    report.push(format!("{track}: {}", built.describe()));
                    (Some(model), None)
                }
                Err(error) => {
                    report.push(format!(
                        "{track}: the .rcsmodel beside it will not decode ({error:#}) - \
                         drawing the derived ribbon instead, as --ribbon does"
                    ));
                    (None, None)
                }
            }
        }
        Some(geometry) => match mesh::rcs::build_scene(track, track_blob, geometry, &mut |path| {
            archives.read_name(path).ok()
        }) {
            Ok((model, weapon_pads, built)) => {
                report.push(format!("{track}: {}", built.describe()));
                (Some(model), weapon_pads)
            }
            Err(error) => {
                report.push(format!(
                    "{track}: the .rcsmodel beside it will not decode ({error:#}) - \
                     drawing the derived ribbon instead, as --ribbon does"
                ));
                (None, None)
            }
        },
    }
}

/// The second element of [`track_model`]'s return, folded into the load
/// report: whether it draws or was decoded and held back is
/// `oag_race::Mode::weapons_enabled`'s call, made at `Scene::new` rather than
/// here - see `race::load`'s own comment on `weapon_pad_model` for why the
/// decode itself stays unconditional.
///
/// **Its own function because the PS3 path used to have no branch at all.**
/// `oag_game::race::load` set `weapon_pad_model` to `None` outright whenever
/// `!vex_geometry`, which is always true on the PS3 path - a circuit's weapon
/// pads there are baked into `rcs_model`'s own world-space chunk pass rather
/// than named by a PSP-shaped `Weapon Pad` node, so [`track_model`] pulls
/// them out as its own second element instead, and this is where that
/// element turns into the same report line the PSP-shaped path already
/// writes. Reported from play: HD drew weapon pads in every mode, because the
/// mode's own gate had nothing to act on.
pub(super) fn weapon_pad_model(
    pads: Option<Model>,
    mode: oag_race::Mode,
    report: &mut Vec<String>,
) -> Option<Model> {
    match &pads {
        Some(pads) => report.push(format!(
            "{} the track's weapon pads: {} triangle(s), {} material(s)",
            if mode.weapons_enabled() {
                "drawing"
            } else {
                "decoded but not drawing (weapons off in this mode)"
            },
            pads.indices.len() / 3,
            pads.draws.len() + pads.alpha_tested_draws.len() + pads.transparent_draws.len(),
        )),
        None => report.push(
            "the track's .rcsmodel authors no Weapon Pad chunk this build could place - \
             no pickup plates are drawn"
                .to_string(),
        ),
    }
    pads
}
