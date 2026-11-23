//! The one-shot no-damage spark a craft throws when its box meets a surface with no friction.
//!
//! HD's contact ring walk gives each record to `Ship_SpawnContactFx`, which picks the spark
//! kind from the record's friction: `friction <= 0` is kind 2, `WO_SHIP_COLL_SPARK_NODAMAGE`
//! (`_ZONE` under the mode mask), and any kind becomes 3 when the craft is not racing
//! (`0x000e734c`..`0x000e7434`, `docs/ghidra/functions/ps3-hdfury-eu/collision-cues.md`,
//! "which records carry no friction", confidence 78). The frictionless records are the floor,
//! the mag floor and the reset planes; walls and craft pairs keep friction `0.035` and the
//! attached damage spark of [`crate::contact_fx`].
//!
//! Kind 2 is a new instance per record, a child of the locator nearest the record, with no
//! strength (scale `1.0`) and no carried velocity. The byte that keeps an attached spark alive
//! is set after every kind, so a frame of only frictionless records still holds a running
//! attached spark ([`crate::contact_fx::ContactFx::advance`], [`crate::field_sparks`]).
//!
//! Where the title's weapon-spark locators exist (HD, and 2048 and Omega by inheritance,
//! [`crate::hit_sparks::throws_weapon_spark`]): that list is `Ship_DispatchCollisionFx`'s, the
//! one this dispatch reads. Pulse and Pure keep none, so they throw nothing here.
//!
//! **Chosen, not measured:** the spark stays where it spawned instead of riding its locator
//! (`collision-cues.md` keeps that value open, confidence 60); a record cap of
//! [`MAX_FRICTIONLESS`] per craft per tick; the cockpit-view flag (`ship + 0x5f42`, inferred)
//! that stops every kind is not applied.
//!
//! **Nothing here is simulation state.**

use super::*;
use oag_physics::wall::{FrictionlessRecord, MAX_FRICTIONLESS, WallResponse};

/// What `Ship_SpawnContactFx` throws for a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContactKind {
    /// Kind 0: the attached damage spark.
    Attached,
    /// Kind 2: the one-shot no-damage spark.
    NoDamage,
    /// Kind 3: a craft that is not racing restarts the attached spark whatever the friction.
    NotRacing,
}

/// The kind rule (`0x000e72c4`..`0x000e7434`): `friction <= 0` is 2, then 3 when the craft is
/// not racing. A NaN friction fails the `<=` and keeps kind 0.
pub(super) fn contact_kind(friction: f32, racing: bool) -> ContactKind {
    if !racing {
        ContactKind::NotRacing
    } else if friction <= 0.0 {
        ContactKind::NoDamage
    } else {
        ContactKind::Attached
    }
}

/// Kind 3: the frictionless record of a craft that is not racing, as the craft-to-craft record the
/// attached spark restarts from (kinds 0 and 3 run one body, `ship-collision-fx.md`, "kind 0 and
/// kind 3"). `None` for a racing craft, whose floor records throw the one-shot instead.
pub(super) fn not_racing_record(
    ship: &Ship,
    wall: &WallResponse,
) -> Option<crate::contact_fx::PairContact> {
    let record = wall
        .frictionless
        .first()
        .filter(|_| wall.floor_contacts > 0)?;
    let racing = ship.physics.craft_state == oag_physics::CraftState::Racing;
    (contact_kind(0.0, racing) == ContactKind::NotRacing).then_some(
        crate::contact_fx::PairContact {
            point: record.point,
            impulse: record.impulse,
        },
    )
}

/// One craft's frictionless records of this tick.
#[derive(Debug, Clone, Copy, Default)]
struct Hits {
    records: [FrictionlessRecord; MAX_FRICTIONLESS],
    count: usize,
}

/// The frictionless records every craft made this tick, for [`RaceView::advance_floor_sparks`].
#[derive(Debug, Clone, Default)]
pub(super) struct FloorSparks {
    hits: [Hits; MAX_SHIPS],
    /// How many sparks have started, ever - for tests.
    started: u32,
}

impl FloorSparks {
    /// Stashes `slot`'s records: the wall pass's floor and mag-floor ones, then the reset
    /// plane's point when it touched one (a record with no impulse).
    pub(super) fn note(&mut self, slot: usize, wall: &WallResponse, reset: Option<Vec3>) {
        let hits = &mut self.hits[slot];
        let walls = (wall.floor_contacts as usize).min(MAX_FRICTIONLESS);
        hits.records[..walls].copy_from_slice(&wall.frictionless[..walls]);
        hits.count = walls;
        if let Some(point) = reset.filter(|_| walls < MAX_FRICTIONLESS) {
            hits.records[walls] = FrictionlessRecord {
                point,
                impulse: 0.0,
            };
            hits.count += 1;
        }
    }
}

impl Race {
    /// How many frictionless sparks have ever started, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn floor_sparks_started_for_tests(&self) -> u32 {
        self.view.floor_sparks.started
    }
}

#[cfg(feature = "view")]
impl RaceView {
    /// Throws one no-damage spark per stashed record, from the locator nearest it, then forgets
    /// them. Nothing under a Shield (the ring loop's shield branch precedes the dispatch), none
    /// for a craft no local view sees within 100 units (`Ship_UpdateEntity`'s reach, as
    /// [`crate::field_sparks`]), and none from a craft that is not racing (kind 3 is not this
    /// spark). One spark per record whichever views see it.
    pub(super) fn advance_floor_sparks(&mut self, core: &SimRace) {
        let hits = std::mem::take(&mut self.floor_sparks.hits);
        if hits.iter().all(|hits| hits.count == 0) {
            return;
        }
        let Some(effect) = self.handles.get(Trigger::CollisionSparkNoDamage).cloned() else {
            return;
        };
        let eyes = self.view_eyes(core);
        for (slot, hits) in hits.iter().enumerate() {
            let ship = &core.sim.world.ships[slot];
            let anchors = self.hit_sparks.dispatch_anchors(slot);
            let position = ship.physics.body.position;
            let seen = eyes
                .iter()
                .any(|eye| (position - *eye).length_squared() < crate::field_sparks::REACH_SQUARED);
            if hits.count == 0
                || anchors.is_empty()
                || !ship.in_world()
                || !seen
                || ship.physics.shield_pickup_timer > 0.0
            {
                continue;
            }
            let model = model_matrix_of(ship);
            for record in &hits.records[..hits.count] {
                let racing = ship.physics.craft_state == oag_physics::CraftState::Racing;
                // A record with no friction: `contact_kind(0.0, ..)`.
                if contact_kind(0.0, racing) != ContactKind::NoDamage {
                    continue;
                }
                let Some(at) = crate::hit_sparks::nearest_locator(anchors, model, record.point)
                else {
                    continue;
                };
                if self.stage.play(&effect, at, 1.0).is_some() {
                    self.floor_sparks.started += 1;
                    log::trace!("floor spark: slot {slot} tick {}", core.sim.world.tick);
                }
            }
        }
    }
}
