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
/// **Excludes both pad classes' chunks on a Wipeout HD source.**
/// `oag_render::mesh::rcs::build_scene`'s own doc comment explains why - a
/// caller that wants a circuit's `Weapon Pad`/`Speedup Pad` geometry reaches
/// it through `pads::ps3_pad_models` instead, which is what gives
/// `race::load` a tintable, gateable model rather than one baked
/// unremovably into this one.
pub(super) fn track_model(
    archives: &mut oag_assets::Archives,
    track: &str,
    track_blob: &[u8],
    geometry: &Option<Vec<u8>>,
    model_name: Option<&str>,
    report: &mut Vec<String>,
) -> Option<Model> {
    match geometry {
        None => None,
        // **Two containers wear this extension.** Wipeout HD's needs the `.vex`
        // beside it - a chunk is addressed by hash and its stride comes from the
        // node's box - and Wipeout 2048's needs neither, so it takes a builder
        // of its own rather than a branch inside HD's. See
        // `oag_render::mesh::rcs::psp2`.
        Some(geometry) if mesh::rcs::psp2::is_psp2(geometry) => {
            let animation = psp2_animation(archives, track, model_name, report);
            match mesh::rcs::psp2::build(track, geometry, animation.as_ref(), &mut |path| {
                archives.read_name(path).ok()
            }) {
                Ok((model, built)) => {
                    report.push(format!("{track}: {}", built.describe()));
                    Some(model)
                }
                Err(error) => {
                    report.push(format!(
                        "{track}: the .rcsmodel beside it will not decode ({error:#}) - \
                         drawing the derived ribbon instead, as --ribbon does"
                    ));
                    None
                }
            }
        }
        Some(geometry) => match mesh::rcs::build_scene(track, track_blob, geometry, &mut |path| {
            archives.read_name(path).ok()
        }) {
            Ok((model, built)) => {
                report.push(format!("{track}: {}", built.describe()));
                Some(model)
            }
            Err(error) => {
                report.push(format!(
                    "{track}: the .rcsmodel beside it will not decode ({error:#}) - \
                     drawing the derived ribbon instead, as --ribbon does"
                ));
                None
            }
        },
    }
}

/// The `.rcsskeleton` and `.rcsanimclip` beside a Wipeout 2048 circuit, or
/// `None` with a report line saying which was missing or would not decode.
///
/// **A circuit with a skeleton and no clip still gets its skeleton**: the
/// nodes place the geometry even when nothing moves it, so the clip's
/// absence costs the motion and not the placement. A circuit with no
/// skeleton at all draws its node-bound meshes through the model's own bind
/// matrices - see `oag_render::mesh::rcs::psp2::placement`.
fn psp2_animation(
    archives: &mut oag_assets::Archives,
    track: &str,
    model_name: Option<&str>,
    report: &mut Vec<String>,
) -> Option<mesh::rcs::psp2::Animation> {
    // Beside the `.rcsmodel` that was actually found, so the Omega Collection's
    // `track.final.rcsmodel` reads `track.final.rcsskeleton` and not a
    // `track.rcsskeleton` that does not exist.
    let (skeleton_name, clip_name) = match model_name {
        Some(model) => mesh::rcs::psp2::animation_names_beside(model)?,
        None => mesh::rcs::psp2::animation_names(track)?,
    };
    let Ok(skeleton) = archives.read_name(&skeleton_name) else {
        report.push(format!(
            "{track}: no .rcsskeleton beside it - node-bound scenery is placed by the \
             model's own bind matrices and nothing moves"
        ));
        return None;
    };
    let clip = archives.read_name(&clip_name).ok();
    if clip.is_none() {
        report.push(format!(
            "{track}: no .rcsanimclip beside it - the scenery is placed and nothing moves"
        ));
    }
    match mesh::rcs::psp2::Animation::parse(&skeleton, clip.as_deref()) {
        Ok(animation) => {
            report.push(format!(
                "{track}: {} skeleton node(s), {} animated over {} track(s), {:.1} s loop",
                animation.skeleton.nodes.len(),
                animation.clip.as_ref().map_or(0, |c| c.tracks.len()),
                animation.clip.as_ref().map_or(0, |c| c.tracks.len()),
                animation.clip.as_ref().map_or(0.0, |c| c.duration),
            ));
            Some(animation)
        }
        Err(error) => {
            report.push(format!(
                "{track}: the .rcsskeleton/.rcsanimclip beside it will not decode ({error}) - \
                 node-bound scenery is placed by the model's own bind matrices and nothing moves"
            ));
            None
        }
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
    weapons_on: bool,
    report: &mut Vec<String>,
) -> Option<Model> {
    match &pads {
        Some(pads) => report.push(format!(
            "{} the track's weapon pads: {} triangle(s), {} material(s)",
            if weapons_on {
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
