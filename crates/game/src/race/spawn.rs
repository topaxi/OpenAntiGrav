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
) -> Option<Pose> {
    let height = spawn_height(handling);
    if let Some(slot) = start_position {
        return Some(Pose::from_start_position(
            slot,
            ground_under(collision, slot),
            height,
        ));
    }
    spline
        .start()
        .map(|sample| Pose::from_sample(sample, sample.racing_line, height))
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
/// A slot with no surface under it keeps the offset height it was given, which is
/// the same fallback [`spawn_pose`] already takes when a track authors no slot.
pub(super) fn grid_poses(
    base: Pose,
    collision: &CollisionWorld,
    height: f32,
) -> [Pose; GRID_SLOTS as usize] {
    core::array::from_fn(|index| {
        let slot = u8::try_from(index + 1).unwrap_or(GRID_SLOTS);
        let mut pose = oag_gameplay::grid_pose(base, slot);
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
    oag_formats::track::start_position(blob.get(node.payload())?, vex::byte_order(blob))
}
