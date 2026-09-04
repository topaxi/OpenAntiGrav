//! Where each craft's `blob` shadow goes this frame.
//!
//! The half of [`oag_render::shadow`] that needs the collision world, and so
//! cannot live in a crate that depends on nothing but `oag-core` and `wgpu`.
//! It casts, and the render crate draws.
//!
//! **Presentation, not simulation.** Nothing here writes `World`, nothing here
//! is hashed, and a shadow that placed itself differently on two machines
//! would move no state - which is why the query runs at draw time from the
//! pose the tick left behind, rather than being recorded during the step. See
//! `docs/architecture/determinism.md`.

use oag_physics::collide::{Ray, Raycaster, Surface};
use oag_render::shadow::{self, Placement};

use super::Race;

/// How many hits one downward cast may return.
///
/// The cast has to see past a non-hoverable surface rather than stopping at
/// it: a craft beside a wall gets the *wall* from a nearest-hit query, and its
/// shadow would stand up the wall's face. Four is the same order
/// `oag_physics::wall`'s own per-probe buffer uses, and one cast down through
/// the road never approaches it.
const MAX_HITS: usize = 4;

impl Race {
    /// One [`Placement`] per craft that is over a floor, in slot order.
    ///
    /// Slot order and slot-indexed silhouettes: [`Placement::silhouette`] is
    /// the grid slot, so a caller pairs it with its own per-slot texture
    /// without a second mapping to keep in step.
    ///
    /// A craft with no floor under it inside its own probe reach - airborne
    /// off a jump, or over a gap - contributes **nothing** rather than a
    /// shadow at its last known height.
    #[must_use]
    pub fn shadow_placements(&self) -> Vec<Placement> {
        let mut out = Vec::with_capacity(self.world.ship_count as usize);
        for slot in 0..self.world.ship_count as usize {
            let ship = &self.world.ships[slot];
            if !ship.active {
                continue;
            }
            let body = &ship.physics.body;
            let up = body.up();
            // The craft's own hover target - the height its spring holds it
            // at, and so the height its shadow is full strength at. The disc's
            // number by way of `oag_physics::hover`; what is ours is the span
            // past it the shadow fades over.
            let ride = oag_physics::hover::target_height(
                &ship.handling,
                ship.physics.mag_lock_blend,
                ship.physics.leap_timer,
            );
            // Cast past the hover reach, because the shadow outlives contact:
            // a craft off a jump is well above anything the spring still
            // finds, and a cast that stopped there would cut the shadow off at
            // full strength.
            let reach = ride * shadow::FADE_REACH;
            // Cast along the craft's **own** down axis rather than world
            // gravity, the same choice `hover::probe` documents: it is what
            // makes a magstrip and an inverted section work at all.
            let ray = Ray::new(body.position, -up, reach);
            // `None` for the skip: no craft is a collider in this world - the
            // ships are stepped against the track's own geometry and each
            // other through `oag_physics::pair`, not through the raycaster -
            // so there is no self-hit to avoid. See `race::tick`, which leaves
            // `Environment::self_collider` at its default for the same reason.
            let mut hits = [None; MAX_HITS];
            let found = self.collision.raycast_all(ray, None, false, &mut hits);
            // Nearest *hoverable* hit, not nearest hit. `raycast_all` returns
            // storage order rather than a distance sort - deliberately, see
            // its own docs - so the minimum is taken here.
            let Some(hit) = hits
                .iter()
                .flatten()
                .take(found)
                .filter(|hit| hit.surface.is_hoverable())
                .min_by(|a, b| a.distance.total_cmp(&b.distance))
            else {
                continue;
            };
            debug_assert!(matches!(hit.surface, Surface::Floor | Surface::MagFloor));
            let height = (body.position - hit.point).dot(up);
            let strength = shadow::fade(height, ride);
            if strength <= 0.0 {
                continue;
            }
            // The hull's own footprint, straight off `<Misc>` and **not**
            // through the `0.75` global `oag_physics::wall::hull_extent`
            // applies: that scale builds the collision box, and what casts a
            // shadow is the model a player sees. The two numbers agree on the
            // craft's size - Feisar's `<Misc>` length is 13.0 against a drawn
            // bounding radius of 6.45 - so the unscaled one is the one that
            // matches the hull on screen.
            out.push(Placement {
                contact: hit.point,
                normal: hit.normal,
                forward: body.forward(),
                half_length: ship.handling.dimensions.length * 0.5,
                half_width: ship.handling.dimensions.width * 0.5,
                strength,
                silhouette: slot,
            });
        }
        out
    }
}

/// The entry a team's authored shadow silhouette sits at, in the shared
/// `Data\Ships\<team>\` spelling every title's roster uses.
///
/// **Measured, not guessed**: all nine of Wipeout HD's single-channel `B8`
/// textures are `/data/ships/<team>/textures/ambient_shadow.gtf`, listed one
/// by one in `crates/formats/tests/gtf_ground_truth.rs`. Written with
/// backslashes because that is the spelling a WAD name hash needs, and the
/// PSARC reader folds them - see `oag_assets::psarc::normalise`.
fn silhouette_entry(dir: &str, team: &str) -> String {
    format!(r"{dir}\{team}\textures\ambient_shadow.gtf")
}

/// One `blob` silhouette per grid slot, in `teams`' own order.
///
/// The disc's own image where the source ships one and a generated falloff
/// where it does not, and the report says which happened for every slot -
/// the difference between "this is the artists' silhouette" and "this is a
/// circle we drew" is exactly the kind that disappears silently otherwise.
///
/// **Tried on every title rather than gated on one.** The entry name is
/// Wipeout HD's measured path in the shared `Data\Ships` spelling, so a title
/// that ships one is found by looking and a title that does not misses
/// cleanly. Nothing about the attempt claims another title *should* have one:
/// `blob` `0x3e0` and `textureBlob` `0x3df` are authored zero times on the
/// Pulse disc, which is why the fallback exists at all.
pub fn silhouettes(
    archives: &mut oag_assets::Archives,
    teams: &[String],
    dir: &str,
    report: &mut Vec<String>,
) -> Vec<shadow::Silhouette> {
    let mut out = Vec::with_capacity(teams.len());
    for (slot, team) in teams.iter().enumerate() {
        let name = silhouette_entry(dir, team);
        match archives
            .read_name(&name)
            .ok()
            .and_then(|blob| decode_silhouette(&blob))
        {
            Some(image) => {
                report.push(format!(
                    "slot {slot}: {name}, {}x{} - the disc's own shadow silhouette",
                    image.width, image.height
                ));
                out.push(image);
            }
            None => {
                // Reported per slot, not once: a set where seven teams decode
                // and one does not is a decode bug, and a summary line would
                // hide it.
                report.push(format!(
                    "slot {slot}: no {name} - the blob shadow falls back to a generated \
                     falloff, which is this project's and not the disc's"
                ));
                out.push(shadow::Silhouette::falloff(FALLOFF_SIZE));
            }
        }
    }
    out
}

/// The generated falloff's resolution.
///
/// 64x64. Ours, like the falloff itself: it is a smooth radial ramp with no
/// detail to lose, and this is the size at which a shadow filling a good part
/// of the screen still has no visible steps.
const FALLOFF_SIZE: u32 = 64;

/// One `.gtf` as pixels the shadow pipeline can bind.
///
/// The single-channel `B8` path in particular: `to_rgba` applies the
/// descriptor's own `remap`, which broadcasts the stored byte across rgb and
/// forces alpha opaque, so the coverage arrives in the red channel exactly as
/// `shadow.wgsl` reads it. See `docs/formats/gtf.md`.
fn decode_silhouette(blob: &[u8]) -> Option<shadow::Silhouette> {
    let gtf = oag_formats::gtf::Gtf::parse(blob).ok()?;
    let texture = gtf.only()?;
    let rgba = texture.to_rgba(blob).ok()?;
    let (width, height) = texture.level_size(0);
    Some(shadow::Silhouette {
        width,
        height,
        rgba: rgba.into_iter().flatten().collect(),
    })
}
