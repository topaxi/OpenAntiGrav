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
use crate::drawable::model_matrix_of;
use oag_physics::wall::{MAX_FRICTIONLESS, WallResponse};

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

/// Whether the craft is in the original's state 1: racing and off the grid (the countdown is
/// state 0).
fn is_racing(ship: &Ship) -> bool {
    ship.physics.craft_state == oag_physics::CraftState::Racing && !ship.physics.on_grid
}

/// Kind 3: the frictionless record of a craft that is not racing, as the craft-to-craft record the
/// attached spark restarts from (kinds 0 and 3 run one body, `ship-collision-fx.md`, "kind 0 and
/// kind 3"). `None` for a racing craft, whose floor records throw the one-shot instead, and for a
/// craft in state 6, whose contact walk the original skips.
pub(super) fn not_racing_record(
    ship: &Ship,
    wall: &WallResponse,
) -> Option<crate::contact_fx::PairContact> {
    let record = wall
        .frictionless
        .first()
        .filter(|_| wall.floor_contacts > 0)?;
    let wreck = oag_physics::damage::in_state_6(&ship.physics, false);
    (!wreck && contact_kind(0.0, is_racing(ship)) == ContactKind::NotRacing).then_some(
        crate::contact_fx::PairContact {
            point: record.point,
            impulse: record.impulse,
        },
    )
}

/// One craft's frictionless records of this tick, already placed: where each spark spawns (the
/// locator nearest the record, at the pose the record was made in) and where the craft was, for
/// the reach test. Placed when noted because a reset plane's record teleports the craft the same
/// tick.
#[derive(Debug, Clone, Copy, Default)]
struct Hits {
    at: [Vec3; MAX_FRICTIONLESS],
    count: usize,
    position: Vec3,
}

/// The frictionless records every craft made this tick, for [`RaceView::advance_floor_sparks`].
#[derive(Debug, Clone, Default)]
pub(super) struct FloorSparks {
    hits: [Hits; MAX_SHIPS],
    /// How many sparks have started, ever - for tests.
    started: u32,
    /// How many records were noted for a kind 2 throw, ever - for tests.
    noted: u32,
}

impl FloorSparks {
    /// Stashes `slot`'s kind 2 records: the wall pass's floor and mag-floor ones, then the reset
    /// plane's point when it touched one (a record with no impulse), each placed on the locator of
    /// `anchors` nearest it under the craft's pose now. Nothing for a title with no locators or a
    /// craft that is not racing.
    pub(super) fn note(
        &mut self,
        slot: usize,
        ship: &Ship,
        wall: &WallResponse,
        reset: Option<Vec3>,
        anchors: &[oag_livery::SparkAnchor],
    ) {
        if anchors.is_empty() || contact_kind(0.0, is_racing(ship)) != ContactKind::NoDamage {
            return;
        }
        let model = model_matrix_of(ship);
        let walls = (wall.floor_contacts as usize).min(MAX_FRICTIONLESS);
        let points = wall.frictionless[..walls]
            .iter()
            .map(|record| record.point)
            .chain(reset)
            .take(MAX_FRICTIONLESS);
        let hits = &mut self.hits[slot];
        hits.count = 0;
        hits.position = ship.physics.body.position;
        for point in points {
            if let Some(at) = crate::hit_sparks::nearest_locator(anchors, model, point) {
                hits.at[hits.count] = at;
                hits.count += 1;
            }
        }
        self.noted += hits.count as u32;
    }
}

impl Race {
    /// How many records were ever noted for a kind 2 throw, whether or not a view saw them, for
    /// tests.
    #[doc(hidden)]
    #[must_use]
    pub fn floor_records_noted_for_tests(&self) -> u32 {
        self.view.floor_sparks.noted
    }

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
            let seen = eyes.iter().any(|eye| {
                (hits.position - *eye).length_squared() < crate::field_sparks::REACH_SQUARED
            });
            if hits.count == 0 || !seen || ship.physics.shield_pickup_timer > 0.0 {
                continue;
            }
            for &at in &hits.at[..hits.count] {
                if self.stage.play(&effect, at, 1.0).is_some() {
                    self.floor_sparks.started += 1;
                    log::trace!("floor spark: slot {slot} tick {}", core.sim.world.tick);
                }
            }
        }
    }
}
