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

use oag_formats::ByteOrder;
use oag_physics::collide::{Ray, Raycaster, Surface};
use oag_render::shadow::{self, Placement};
use oag_vex::shadow_occluder::Occluder;
use oag_vex::vex;

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
        let mut out = Vec::with_capacity(self.sim.world.ship_count as usize);
        for slot in 0..self.sim.world.ship_count as usize {
            let ship = &self.sim.world.ships[slot];
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
                ship.physics.slowdown_timer,
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
            let found = self.sim.collision.raycast_all(ray, None, false, &mut hits);
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

    /// How far the circuit's own nearest hoverable floor stands above `at`,
    /// measured along `normal` - negative when it lies below.
    ///
    /// Looks a unit above and two below, which is what bounds [`oag_render::shadow::conform_to_floor`]. `None` where no floor is in that
    /// span. The question a shadow polygon asks of the road it is laid over:
    /// anything positive is the road in front of it.
    #[must_use]
    pub fn floor_above(
        &self,
        at: oag_core::math::Vec3,
        normal: oag_core::math::Vec3,
    ) -> Option<f32> {
        let n = normal.normalize_or_zero();
        let ray = Ray::new(at + n, -n, 3.0);
        let mut hits = [None; MAX_HITS];
        let found = self.sim.collision.raycast_all(ray, None, false, &mut hits);
        hits.iter()
            .flatten()
            .take(found)
            .filter(|hit| hit.surface.is_hoverable())
            .min_by(|a, b| a.distance.total_cmp(&b.distance))
            .map(|hit| (hit.point - at).dot(n))
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
    let mut fallbacks = 0;
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
                fallbacks += 1;
            }
        }
    }
    // The per-slot lines above are the report's, and the log carries them at
    // `debug`: Pulse, Pure and 2048 author no silhouette at all, so eight
    // identical warnings every race would say nothing the first does not.
    if fallbacks > 0 {
        log::warn!(
            "blob shadow: {fallbacks} of {} slot(s) use a generated falloff, which is this \
             project's and not the disc's",
            teams.len()
        );
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
    let gtf = oag_texture::gtf::Gtf::parse(blob).ok()?;
    let texture = gtf.only()?;
    let rgba = texture.to_rgba(blob).ok()?;
    let (width, height) = texture.level_size(0);
    Some(shadow::Silhouette {
        width,
        height,
        rgba: rgba.into_iter().flatten().collect(),
    })
}

/// One craft's authored shadow hull per grid slot, where its model carries one.
///
/// **The `original` tier's geometry**, read out of the same `.vex` the hull
/// mesh comes from: a `Dynamic Shadow Occluder` `0x3c3` node, which 119 of the
/// Pulse disc's 129 are. A model with none contributes `None` and the report
/// says so - a craft with no authored hull casts no `original` shadow rather
/// than borrowing another team's.
///
/// **The first one in the file, and the file order is the tie-break.** A model
/// can author more than one (`shadowShape` beside `shadow_lodShape`, the LOD
/// variant), and which of them the original picks per frame is *unread* - so
/// this takes the first and says how many it passed over, rather than choosing
/// a rule nothing supports.
pub fn hulls(
    archives: &mut oag_assets::Archives,
    teams: &[String],
    race: &oag_title::RaceDefaults,
    mode: oag_race::Mode,
    report: &mut Vec<String>,
) -> Vec<Option<Occluder>> {
    let mut out = Vec::with_capacity(teams.len());
    for (slot, team) in teams.iter().enumerate() {
        // The baseline hull's own occluder regardless of a Normal/Concept
        // pick: `oag_title::race::HullVariant` swaps the drawn model, not
        // its shadow geometry, and threading the player's choice through
        // here would only matter the day the two shapes are shown to
        // disagree.
        let name = oag_livery::entry::ship_entry_name(race.ships_for(team), team, mode, None);
        let found = archives.read_name(&name).ok().and_then(|blob| {
            let tree = vex::nodes(&blob).ok()?;
            let found: Vec<usize> = tree
                .iter()
                .enumerate()
                .filter(|(_, node)| node.class_id == CLASS_OCCLUDER)
                .map(|(index, _)| index)
                .collect();
            let first = *found.first()?;
            let hull = Occluder::parse(&blob[tree[first].payload()], ByteOrder::Little)?;
            // **Placed, not raw.** The payload's vertices are in the space of
            // whatever `Transform` nodes enclose the occluder, exactly as a
            // mesh node's are; `world_transforms` composes that chain, and the
            // occluder's own class contributes the identity to it. Skipping
            // this draws the right hull in the wrong frame - which still looks
            // like a shadow, and is how it went unnoticed until someone said
            // the shape was off.
            let placement = vex::world_transforms(&blob, &tree);
            let hull = hull.placed(placement.get(first)?);
            Some((
                hull,
                found.len(),
                tree[first].name.clone().unwrap_or_default(),
            ))
        });
        match found {
            Some((hull, count, node)) => {
                report.push(format!(
                    "slot {slot}: {name} authors {count} shadow hull(s); {node} is the one \
                     drawn - {} face(s), {} vertex slot(s)",
                    hull.faces.len(),
                    hull.vertices.len()
                ));
                out.push(Some(hull));
            }
            None => {
                report.push(format!(
                    "slot {slot}: {name} authors no Dynamic Shadow Occluder - this craft casts \
                     no `original` shadow, and nothing stands in for it"
                ));
                out.push(None);
            }
        }
    }
    out
}

/// Class id of `Dynamic Shadow Occluder`, from the class-ID table at
/// `0x08ab2370`.
const CLASS_OCCLUDER: u32 = 0x3c3;

/// Both shadow tiers' per-slot assets, in one call.
///
/// One call rather than two at the load site, which is at the 1,000-line rule.
/// They belong together anyway: read off the same models in the same slot
/// order, and loaded whatever `graphics.shadows` says, because the setting
/// applies live and a race must not have to reload to honour it: `blob`'s silhouettes ([`silhouettes`]) and `original`'s
/// hulls ([`hulls`]).
pub fn assets(
    archives: &mut oag_assets::Archives,
    teams: &[String],
    race: &oag_title::RaceDefaults,
    mode: oag_race::Mode,
    report: &mut Vec<String>,
) -> (Vec<shadow::Silhouette>, Vec<Option<Occluder>>) {
    let silhouettes = silhouettes(archives, teams, race.ship_dir, report);
    let hulls = hulls(archives, teams, race, mode, report);
    (silhouettes, hulls)
}
