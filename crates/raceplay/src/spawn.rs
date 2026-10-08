//! Putting a craft on the start line: the authored `Start Position` node, the
//! grid built from it, and the ground probe each slot is dropped onto.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/spawn.rs`.

use super::*;

/// Says so when a model's texture slots did not all fill, and why.
///
/// **Keyed on empty slots, not on an empty list, and the difference is the whole
/// point.** A model that declares no texture slots at all wanted none - the
/// driveable ribbon `oag_render::track::build_model` generates is one, on either
/// disc - and saying "untextured" about it would blame a decode for geometry that
/// was never textured. What is worth reporting is a model that asked for `n`
/// textures and got fewer, because that is a picture missing something it was
/// authored with.
///
/// **PS2 ships are textured now; PS2 track art is the one place this still
/// fires, and it is reported rather than papered over.** A PS2 model's
/// textures are separate archive entries gathered into a nested WAD of
/// Graphics Synthesizer upload packets, and a `.vex` declares no texture
/// pixels of its own to say which. For ships, `ps2_texture_set` below finds
/// it anyway: the entry directly before a model in the archive's own
/// directory, checked independently against every team on the roster - see
/// `docs/formats/ps2-texture.md`. Track models are a separate, harder case:
/// several typically share one texture set instead of one each, and that
/// grouping is not verified against a known-correct picture the way a ship
/// is, so it is not attempted and `16_Track\track.vex` still draws
/// untextured on PS2.
///
/// The model still draws: `Drawable::draw` binds the white fallback for a draw
/// with no texture slot. So an unresolved model is a correctly-shaped
/// untextured one, and nothing here guesses at a texture set to avoid saying
/// that.
///
/// Deliberately keyed on the model rather than on the platform. A PSP model with a
/// slot that will not decode deserves the same line, and the PS2 disc carries
/// PSP-format batches too - so "which disc is this" is never the right question to
/// ask about one mesh.
/// Where a ship starts a race: the track's authored slot, or the spline.
///
/// The authored [`StartPosition`] wins whenever the track has one, because it is
/// a value recovered off the disc and `spline.start()` is an artifact of how this
/// crate resamples - sample 0 of path 0, which is wherever the exporter happened
/// to begin writing control points. On `16_Track` the two are **188.8 units
/// apart** and the authored slot is the one on the grid.
///
/// What this is **not** is pole. Every track ships one slot and a race grids
/// eight, so this places a ship on the one slot that was authored and says
/// nothing about the other seven. On `16_Track` a time trial in the original
/// starts about 138 units *ahead* of this, on the other side of the centreline -
/// which is what makes the remaining slots worth recovering rather than
/// deriving. See `docs/formats/track.md#start-position`.
///
/// The height comes off the collision geometry rather than out of the slot: see
/// [`Pose::from_start_position`] for the measurement that says the authored `y`
/// is not a ride height. `collision` is cast straight down from well above the
/// slot, and a slot over a hole in the mesh falls back to the authored value.
///
/// `None` only when a track has no authored slot *and* an empty spline, which no
/// real track is.
#[must_use]
pub(super) fn spawn_pose(
    spline: &Spline,
    start_position: Option<&StartPosition>,
    collision: &CollisionWorld,
    handling: &Handling,
    grid_cap: Option<f32>,
) -> Option<Pose> {
    let height = capped_spawn_height(handling, grid_cap);
    if let Some(slot) = start_position {
        let pose = Pose::from_start_position(slot, ground_under(collision, slot), height);
        return Some(face_the_way_the_track_runs(pose, slot, spline));
    }
    spline
        .start()
        .map(|sample| Pose::from_sample(sample, sample.racing_line, height))
}

/// How far the authored heading may be off the spline's own before the spline
/// is believed instead: `cos 60 degrees`.
///
/// **A threshold across an empty band rather than a tuned number.** All 92
/// authored slots measured fall at one end or the other: the ones that agree
/// run `+0.920` to `+1.000` and the ones that do not are `0.000` and below, so
/// nothing sits anywhere near this value. See [`face_the_way_the_track_runs`]
/// for the census.
const SLOT_AGREES_WITH_THE_SPLINE: f32 = 0.5;

/// The same pose, turned to the spline's own direction when the authored slot
/// disagrees with it.
///
/// # The slot is usually right, and on one title it is sometimes stale
///
/// A `Start Position` node carries a heading and it is the value to prefer:
/// where a ship points on the grid is authored deliberately, and a spline
/// tangent is this crate's resampling of a curve. So this changes nothing
/// wherever the two agree, which is almost everywhere.
///
/// They do not always agree. Every shipped circuit file on all four discs in
/// hand was measured - the authored forward against the tangent of the nearest
/// resampled point to the slot:
///
/// | source | files | agree | stale |
/// | --- | ---: | ---: | ---: |
/// | `pulse-psp-eu.chd` | 24 | 24 (`+0.925` to `+1.000`) | 0 |
/// | `pure-psp-eu.chd` | 8 | 8 (`+0.988` to `+1.000`) | 0 |
/// | `pulse-ps2-eu.chd` | 32 | 31 (`+0.920` to `+1.000`) | **1** (`-0.441`) |
/// | `hdfury-ps3-eu-dec.iso` | 28 | 19 (`+0.999` to `+1.000`) | **9** (`-1.000`, one `0.000`) |
///
/// HD's nine are `01_vineta_k`, `04_chenghou_project`, `05_ubermall`,
/// `10_sebenco_climb`, `12_sol_2` and `15_anulpha_pass` reversed,
/// `modesto_heights` reversed, `tech_de_ra` reversed, and `zone_3`
/// **forward** - so it is stale authoring rather than a rule about reversed
/// circuits. On eight of them the rotation rows are byte-identical to the
/// forward file's and only the position row moved: the slot was dragged to the
/// other end of the track and never turned round. `tech_de_ra` reversed is the
/// ninth and carries a bare identity matrix, which is a slot nobody authored at
/// all.
///
/// # The PS2's one stale slot is what corroborates the correction
///
/// `09_Track` is the same circuit on the PSP and PS2 pressings and its slot has
/// a **byte-identical position** on both, `(-506.0834, 2.1623526, 242.45824)`.
/// The rotation is not identical: the PSP's forward row runs straight down the
/// track and the PS2's is 116 degrees off it. One slot, exported twice, one
/// rotation maintained and one not - which is HD's failure mode appearing on a
/// disc from four years earlier.
///
/// So one case exists where the right answer is on another pressing, and it can
/// be checked: the heading this substitutes on the PS2 file agrees with the PSP
/// file's **authored** one to within **0.08 degrees**. Nothing about that
/// comparison went into deriving the rule.
///
/// A ship spawned on one of those faces backwards down its own circuit, and so
/// does the whole grid: [`grid_poses`] walks the spline in `base`'s forward
/// direction, so a reversed base lays the field out ahead of pole instead of
/// behind it.
///
/// # Why the tangent and not the negated slot
///
/// Negating a heading that disagrees would fix the eight and leave the ninth
/// pointing across the track, and it would be a correction with nothing behind
/// it - the slot is not "backwards", it is *unmaintained*, and there is no
/// reason the next unmaintained one is exactly 180 degrees out. The spline is a
/// real measurement of which way the circuit runs at that point, so it is what
/// is used. Where the slot is right, the two are the same direction to three
/// decimal places anyway.
///
/// # The replacement is levelled, because the bind levels the authored one
///
/// `oag_vex::track::start_position` drops the authored forward's `y` before
/// normalising it - the bind handler at `0x08926ae8` forces the up row to world
/// `(0, 1, 0)` and re-orthonormalises around it, so **every** slot on **every**
/// track yields an exactly horizontal heading. A spline tangent is not
/// horizontal: on a climbing circuit like `10_sebenco_climb` it carries real
/// pitch. Handing it over raw would spawn those nine craft in a frame the
/// original's bind cannot produce, and leave the other 43 track files in a
/// different convention from them.
///
/// So the tangent is flattened the same way the authored value is, against the
/// same world up. That is reproducing the bind rather than smoothing anything:
/// the pitch is not being discarded because it looks wrong, it is being
/// discarded because the handler this is standing in for discards it.
///
/// **Position, height and the grid's own layout are untouched**, and so is
/// every slot that agrees - which is every track file on both PSP discs, so no
/// Pulse trace, hash or capture comparison moves by this. The one PS2 file that
/// does move was spawning its craft 116 degrees across its own track.
fn face_the_way_the_track_runs(pose: Pose, slot: &StartPosition, spline: &Spline) -> Pose {
    let Some((_, sample, _)) = spline.nearest(pose.position) else {
        return pose;
    };
    let Some(tangent) = Vec3::from_array(sample.tangent).try_normalize() else {
        return pose;
    };
    if Vec3::from_array(slot.forward).dot(tangent) > SLOT_AGREES_WITH_THE_SPLINE {
        return pose;
    }
    // Levelled, as the bind levels the authored row - see above. A tangent that
    // is purely vertical has nothing left once `y` is dropped, and no shipped
    // track has one; the slot is kept rather than replaced by nothing.
    let Some(levelled) = Vec3::new(tangent.x, 0.0, tangent.z).try_normalize() else {
        return pose;
    };
    pose.facing(levelled)
}

/// Every grid slot's pose, front to back, re-dropped onto the track under each.
///
/// `base` is slot 8 - the authored `Start Position` node, which
/// [`oag_gameplay::grid_pose`] establishes is the *back* of the grid. The other
/// seven are offsets from it, and each is dropped onto the collision surface
/// under its own footprint rather than inheriting slot 8's height: a grid is 138
/// units long and no track is flat over that, so sharing one `y` would leave the
/// front of the field buried or floating.
///
/// **Walked along `spline` rather than extrapolated in a straight line from
/// `base`'s own frame, and this is a fix rather than a new measurement.**
/// [`oag_gameplay::grid_pose`]'s straight-line offset is what grid.md's own
/// residual table already flagged - "the original's grid follows the track's
/// curve; ours does not" - and on the one circuit that residual was ever
/// measured against (`16_Track`/Talon's Junction), the grid's own start straight
/// happens to be close enough to flat that a straight line stays on the track. It
/// is not close enough on most others: a sweep of every Wipeout HD circuit's
/// grid (`crates/game/tests/hd_trackwall_ground_truth.rs`, both directions,
/// opponents on) found 9 of 12 *reversed* grids putting one or more slots off
/// the collision mesh entirely - up to 145 units from the driveable line on
/// `tech_de_ra`'s reversed grid - against only one flagged forward grid (`zone_3`,
/// one slot, barely over the envelope). Reversed grids sit on whatever piece of
/// track the exporter put their own `Start Position` node on, which is far more
/// often a curve than the authored front straight forward grids sit on, and the
/// straight-line formula extrapolates *off* that curve over the grid's 138-unit
/// span.
///
/// So each slot's raw position now comes from walking `spline` itself, in
/// `base`'s own forward direction, [`oag_gameplay::GRID_ROW_PITCH`] units per
/// step back from `base`'s nearest sample - the same measured constant, applied
/// along the track instead of along a straight line.
///
/// **Laterally, every slot sits `GRID_COLUMN_OFFSET / 2` from the AI corridor's
/// midpoint at its own sample** (2026-09-29, `Race_ComputeGridLayout`): the even
/// slots on the side of the midpoint `base` is on, the odd slots on the other.
/// This replaced carrying `base`'s own lateral offset down the grid and adding
/// the stagger on one fixed side, which put the odd column 30 units off the
/// track on `01_Track` and `17_Track`, whose node is on the left. Slot 8 keeps
/// `base`'s place along the track and is moved laterally onto the same rule -
/// the original does not use the raw node either, and it is 1.68 units nearer
/// the original's eighth craft on `16_Track` for it.
/// **Pulse PSP's grid is the original's own walk** (`walked`, 2026-10-02,
/// `oag_gameplay::grid_walk`): position and heading both, read from
/// `Race_ComputeGridLayout` and `FUN_0882663c` and reproduced against the original's
/// craft to 0.001 units on `01_Track`. The rest of this comment describes the **fallback**
/// that every other title takes, and Pulse PSP takes where the walk refuses a node.
///
/// **Heading** in the fallback is the node's own for every slot unless
/// `frame_from_sample`: there each slot takes the sample tangent's frame (0.05
/// degrees out of the original on `16_Track`, where the node's is 0.35 out). The
/// original's is the edge chords', which is what `walked` carries. `grid.md`'s
/// "heading, any slot against slot 1: 1.0000 to four decimal places" was a dot
/// product, which cannot see 0.3 degrees. See `docs/physics/grid-state.md`.
///
/// Falls back to [`oag_gameplay::grid_pose`]'s straight line, per slot, when
/// `spline` has no sample near `base`, when `base`'s own tangent gives no
/// direction to walk, or when the walk to that slot runs off the end of the
/// path before covering the full distance - a spline too short for a whole
/// grid's worth of walking is the same situation [`oag_gameplay::grid_pose`]
/// was written for, and it is what this project's own short synthetic test
/// tracks are.
///
/// A slot with no collision surface under it keeps the walked height, which is
/// the same fallback [`spawn_pose`] already takes when a track authors no slot.
pub(super) fn grid_poses(
    base: Pose,
    spline: &Spline,
    collision: &CollisionWorld,
    height: f32,
    frame_from_sample: bool,
    walked: Option<&[Pose; GRID_SLOTS as usize]>,
) -> [Pose; GRID_SLOTS as usize] {
    // Pulse PSP builds each slot's matrix from the track's frame at the slot
    // (`oag_gameplay::orientation_on_sample`); every other title keeps the
    // node's own orientation, which is all that has been measured for them.
    let orientation_at = |sample: &oag_vex::track::Sample| {
        if frame_from_sample {
            oag_gameplay::orientation_on_sample(sample)
        } else {
            base.orientation
        }
    };
    let forward = base.orientation * Vec3::NEG_Z;
    let anchor = spline.nearest(base.position).map(|(index, _, _)| index);
    let walk = anchor.and_then(|index| walk_direction(spline, index, forward));
    // `base` sits off the spline's own centreline - every authored `Start
    // Position` does, by 3.2 to 20.5 units (`docs/formats/track.md`). What the
    // original keeps of that is only *which side of the AI corridor's midpoint*
    // the node is on (`Race_ComputeGridLayout`, `grid.md`): every slot is then
    // laid `GRID_COLUMN_OFFSET / 2` from the midpoint at its own sample, the
    // node's side first and alternating per slot. Carrying the node's own
    // lateral offset down the grid and adding the stagger on one fixed side, as
    // this did until 2026-09-29, put the odd column 30 units off the track on
    // `01_Track` and `17_Track`, whose node is on the *left* of the midpoint.
    let anchor_sample = anchor.and_then(|index| spline.sample(index));
    let midpoint =
        |sample: &oag_vex::track::Sample| 0.5 * (sample.ai_bound_left + sample.ai_bound_right);
    let node_lateral = anchor_sample.map(|sample| {
        let lateral = Vec3::from_array(sample.lateral).normalize_or_zero();
        (base.position - Vec3::from_array(sample.pos)).dot(lateral)
    });
    // `+1` when the node is nearer the right edge of the corridor, `-1` when
    // nearer the left: the sign of the first `10.0` in the original.
    let side = match (anchor_sample, node_lateral) {
        (Some(sample), Some(lateral)) if lateral < midpoint(sample) => -1.0,
        _ => 1.0,
    };
    let half_column = 0.5 * oag_gameplay::GRID_COLUMN_OFFSET;
    let lateral_target = |sample: &oag_vex::track::Sample, slot: u8| {
        let sign = if slot.is_multiple_of(2) { side } else { -side };
        midpoint(sample) + sign * half_column
    };

    core::array::from_fn(|index| {
        let slot = u8::try_from(index + 1).unwrap_or(GRID_SLOTS);
        let back = GRID_SLOTS - slot;
        let mut pose = match (walked, anchor, walk, anchor_sample, node_lateral) {
            // Pulse PSP: the original's own walk, position and heading both.
            (Some(walked), ..) => walked[index],
            // `back == 0` is slot 8, `base` itself: it keeps its own place along
            // the track and only its lateral position is re-derived, from the
            // corridor midpoint rather than from where the node was authored.
            (_, Some(_), Some(_), Some(sample), Some(node_lateral)) if back == 0 => {
                let lateral = Vec3::from_array(sample.lateral).normalize_or_zero();
                Pose {
                    position: base.position
                        + lateral * (lateral_target(sample, slot) - node_lateral),
                    orientation: orientation_at(sample),
                }
            }
            (_, Some(anchor), Some(direction), Some(_), Some(_)) => {
                let target = f32::from(back) * oag_gameplay::GRID_ROW_PITCH;
                let (walked, sample_pos, sample_lateral, ran_off_the_end) =
                    walk_along(spline, anchor, direction, target);
                match spline.sample(walked) {
                    // The walk hit the end of the path (or a path boundary)
                    // before covering the full distance - a track shorter than
                    // one grid's worth of spline, which no shipped circuit is
                    // but a synthetic test fixture can be. The straight-line
                    // formula does not care how long the spline is, so it is
                    // the honest fallback here rather than a slot left standing
                    // wherever the walk ran out.
                    //
                    // **A flag, not a distance check against `target`.** A walk
                    // that covers the full distance still stops short of it by
                    // up to one sample's own spacing (see `walk_along`), and
                    // samples run wider than a couple of units apart on some
                    // circuits, so a fixed tolerance read that quantisation as
                    // "ran off the end" and undid the fix on exactly the
                    // reversed grids it exists for.
                    _ if ran_off_the_end => oag_gameplay::grid_pose(base, slot),
                    Some(sample) => Pose {
                        position: sample_pos + sample_lateral * lateral_target(sample, slot),
                        orientation: orientation_at(sample),
                    },
                    None => oag_gameplay::grid_pose(base, slot),
                }
            }
            _ => oag_gameplay::grid_pose(base, slot),
        };
        let up = pose.orientation * Vec3::Y;
        // `base` already carries `height` along its own up axis, so the drop has
        // to take it off before re-applying it under this slot - otherwise every
        // craft gains a ride height per slot.
        let foot = pose.position - up * height;
        let origin = foot + Vec3::Y * SPAWN_PROBE_RISE;
        let ray = oag_physics::Ray::new(origin, Vec3::NEG_Y, SPAWN_PROBE_REACH);
        if let Some(hit) = oag_physics::Raycaster::raycast(collision, ray, None, false) {
            pose.position = Vec3::new(foot.x, hit.point.y, foot.z) + up * height;
        }
        pose
    })
}

/// Which way `spline`'s own sample index walks toward `forward`, from `anchor`.
///
/// `Some(1)` when stepping to a higher index moves toward `forward`, `Some(-1)`
/// when a lower index does, `None` only when `anchor` is out of range or its
/// sample's `tangent` is exactly zero - a degenerate sample [`walk_along`]
/// could not usefully walk from anyway.
///
/// Reads `anchor`'s own authored `tangent` rather than probing a neighbouring
/// sample's position: a `Start Position` node sitting one sample from a path
/// boundary has a neighbour on one side only, and a first version of this that
/// probed neighbours gave up on exactly the circuits where that happened - most
/// of the reversed grids this exists to fix, since a reverse route more often
/// starts near where its path joins the forward one. `tangent` needs no
/// neighbour: it is already the direction of increasing index at this sample,
/// by construction of how [`Spline::from_track`] samples a path with increasing
/// `t`, so its sign against `forward` alone says which way to step.
#[must_use]
fn walk_direction(spline: &Spline, anchor: usize, forward: Vec3) -> Option<i32> {
    let tangent = Vec3::from_array(spline.sample(anchor)?.tangent);
    let dot = tangent.dot(forward);
    if dot > 0.0 {
        Some(1)
    } else if dot < 0.0 {
        Some(-1)
    } else {
        None
    }
}

/// Walks `distance` units along `spline` from `anchor`, stepping `direction`
/// (`1` or `-1`) one sample at a time and accumulating the real distance between
/// consecutive sample positions.
///
/// **Never crosses a path boundary.** [`Spline`]'s own table concatenates every
/// path in *file* order, and consecutive indices are only spatially adjacent
/// within one path - `ai_order`'s doc comment already records a track (`05_Track`
/// on Pulse) where the next path in file order is "a kilometre away, pointing
/// the wrong way". A first version of this walk did not check that and jumped
/// grid slots onto an unrelated branch on some HD circuits, making a handful of
/// forward grids worse rather than fixing the reversed ones. So a step whose
/// target sample is not [`Spline::path_of`] the same as `anchor`'s is treated
/// exactly like running off the end of the table: the walk stops one sample
/// short, on the last sample that is still part of the same path.
///
/// Also stops, short of `distance`, if the walk runs off either end of the
/// table: a grid at the very start or end of a track's samples is clamped to
/// what the track actually has, the same way [`oag_gameplay::grid_pose`]'s slot
/// argument clamps rather than panics.
///
/// Stops at the last sample reached without *exceeding* `distance`, rather than
/// interpolating past it - sub-sample precision is a fraction of a control-point
/// interval (`docs/formats/track.md`), far finer than the grid measurement this
/// is walking.
///
/// Returns the sample's own index, position, (unit) `lateral` axis, and
/// whether the walk ran off the end of the path or the table before covering
/// `distance` - the caller's signal to fall back rather than trust a short
/// walk.
///
/// **A flag, not a distance check against `distance`.** A walk that covers the
/// full distance still stops short of it by up to one sample's own spacing,
/// because it never interpolates past the last sample it can still afford -
/// see above. Samples run wider than a couple of units apart on some circuits,
/// so a caller comparing the returned distance against `distance` with a fixed
/// tolerance would read that ordinary quantisation as "ran off the end" - measured
/// on `tech_de_ra`'s reversed grid, which is exactly what a first version of this
/// function's caller did, and it undid the curve-following fix on slots that had
/// plenty of path left.
#[must_use]
fn walk_along(
    spline: &Spline,
    anchor: usize,
    direction: i32,
    distance: f32,
) -> (usize, Vec3, Vec3, bool) {
    let path = spline.path_of(anchor);
    let mut index = anchor;
    let mut pos = Vec3::from_array(spline.sample(index).map_or([0.0; 3], |s| s.pos));
    let mut travelled = 0.0f32;
    let mut ran_off_the_end = false;
    loop {
        let Some(next_index) = index
            .checked_add_signed(direction as isize)
            .filter(|&i| spline.path_of(i) == path)
        else {
            ran_off_the_end = travelled < distance;
            break;
        };
        let Some(next_sample) = spline.sample(next_index) else {
            ran_off_the_end = travelled < distance;
            break;
        };
        let next_pos = Vec3::from_array(next_sample.pos);
        let step = (next_pos - pos).length();
        if travelled + step > distance {
            break;
        }
        travelled += step;
        pos = next_pos;
        index = next_index;
    }
    let lateral = spline
        .sample(index)
        .map(|s| Vec3::from_array(s.lateral).normalize_or_zero())
        .unwrap_or(Vec3::X);
    (index, pos, lateral, ran_off_the_end)
}

/// How far up the track's own drop for a spawn probe starts, and how far it
/// reaches.
///
/// Generous either way on purpose: this is a one-off query at load, and a slot
/// authored a few units under an overhanging piece of track should still find the
/// floor rather than silently falling back.
const SPAWN_PROBE_RISE: f32 = 20.0;
const SPAWN_PROBE_REACH: f32 = 80.0;

/// The world `y` of the collision surface under an authored slot, if there is one.
#[must_use]
fn ground_under(collision: &CollisionWorld, slot: &StartPosition) -> Option<f32> {
    let origin = Vec3::from_array(slot.position) + Vec3::Y * SPAWN_PROBE_RISE;
    let ray = oag_physics::Ray::new(origin, Vec3::NEG_Y, SPAWN_PROBE_REACH);
    oag_physics::Raycaster::raycast(collision, ray, None, false).map(|hit| hit.point.y)
}

/// Reads a track `.vex`'s authored grid slot, if it has one.
///
/// `None` covers three cases a caller treats the same way and none of which is an
/// error: a blob that is not a `.vex` at all, one with no `Start Position` node,
/// and one whose node payload is not the 64 bytes a transform needs. A track with
/// no authored slot is a track a ship goes on the spline of.
#[must_use]
pub(super) fn start_position_of(blob: &[u8]) -> Option<StartPosition> {
    let nodes = vex::nodes(blob).ok()?;
    // Version-keyed: unrecovered on version 4, and `None` there puts the ship on
    // the spline instead - which `load`'s own report line already describes.
    let class = vex::classes_of(blob).ok()?.start_position?;
    let node = nodes.iter().find(|node| node.class_id == class)?;
    oag_vex::track::start_position(blob.get(node.payload())?, vex::byte_order(blob))
}
