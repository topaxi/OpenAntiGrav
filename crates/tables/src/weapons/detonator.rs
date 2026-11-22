//! The attributes only `weaponstats_detonator.xml` authors: the Global stage
//! recharge, the Mine's scoring and slowdown, the Cannon's reload and rebounds,
//! the Bomb's shot count and points, and the whole `EMP` block.
//!
//! ```xml
//! <Weapon type="Global"><Stats slowdown_limit energy_recharge_per_stage/></Weapon>
//! <Weapon type="Mine">  <Stats ... points_per_metre max_distance velocity_reduction/></Weapon>
//! <Weapon type="Cannon"><Stats ... num_rebounds round_recharge_time recharge_pause/></Weapon>
//! <Weapon type="EMP">   <Stats wave_time speed minNumPadsToCollectEMP .../></Weapon>
//! <Weapon type="Bomb">  <Stats ... number_of_shots_to_destroy energy_recharge_per_shot
//!                               points_for_cannon_bullet_hit points_for_destroying/></Weapon>
//! ```
//!
//! **Decoded, meaning unread.** Field names follow the attribute; no unit or
//! law is claimed (`docs/formats/weapon-stats.md`, "Detonator's attributes").
//! Every field is an `Option`: Fury's `DATA05` copy of the file authors none of
//! them, and a table that mentions none of them yields no [`DetonatorStats`] at all.
//!
//! This is a second pass over the document, separate from [`super::parse`], so
//! the ordinary tables decode exactly as before.

use super::{Node, Result, optional};

/// The Mine block's Detonator-only attributes.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DetonatorMine {
    /// `points_per_metre`.
    pub points_per_metre: Option<f32>,
    /// `max_distance`.
    pub max_distance: Option<f32>,
    /// `velocity_reduction`.
    pub velocity_reduction: Option<f32>,
}

/// The Cannon block's Detonator-only attributes.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DetonatorCannon {
    /// `num_rebounds`.
    pub num_rebounds: Option<f32>,
    /// `round_recharge_time`.
    pub round_recharge_time: Option<f32>,
    /// `recharge_pause`.
    pub recharge_pause: Option<f32>,
}

/// The Bomb block's Detonator-only attributes.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DetonatorBomb {
    /// `number_of_shots_to_destroy`.
    pub number_of_shots_to_destroy: Option<f32>,
    /// `energy_recharge_per_shot`.
    pub energy_recharge_per_shot: Option<f32>,
    /// `points_for_cannon_bullet_hit`.
    pub points_for_cannon_bullet_hit: Option<f32>,
    /// `points_for_destroying`.
    pub points_for_destroying: Option<f32>,
}

/// The `<Weapon type="EMP">` block. It authors no `absorb`, so it is not a
/// [`super::Weapon`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EmpStats {
    /// `wave_time`.
    pub wave_time: Option<f32>,
    /// `speed`.
    pub speed: Option<f32>,
    /// `minNumPadsToCollectEMP`.
    pub min_num_pads_to_collect: Option<f32>,
    /// `maxNumPadsToCollectEMP`.
    pub max_num_pads_to_collect: Option<f32>,
    /// `minDistance`.
    pub min_distance: Option<f32>,
    /// `maxDistance`.
    pub max_distance: Option<f32>,
}

/// Every Detonator-only attribute of one `weaponstats_detonator.xml`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DetonatorStats {
    /// The Global block's `energy_recharge_per_stage`.
    pub energy_recharge_per_stage: Option<f32>,
    /// The Mine block's.
    pub mine: DetonatorMine,
    /// The Cannon block's.
    pub cannon: DetonatorCannon,
    /// The Bomb block's.
    pub bomb: DetonatorBomb,
    /// The `EMP` block, or `None` when the file authors none.
    pub emp: Option<EmpStats>,
}

/// The `<Stats>` of the `<Weapon type="...">` named, if there is one.
fn block<'a>(root: &'a Node, kind: &str) -> Option<&'a Node> {
    root.children_named("Weapon")
        .find(|w| w.value("type") == Some(kind))
        .and_then(|w| w.children_named("Stats").next())
}

/// Reads the Detonator-only attributes, or `None` when the document authors
/// none of them.
pub(super) fn parse(root: &Node) -> Result<Option<DetonatorStats>> {
    let read = |kind: &str, attribute: &'static str| -> Result<Option<f32>> {
        match block(root, kind) {
            Some(stats) => optional(stats, "Stats", attribute),
            None => Ok(None),
        }
    };
    let emp = match block(root, "EMP") {
        Some(stats) => Some(EmpStats {
            wave_time: optional(stats, "Stats", "wave_time")?,
            speed: optional(stats, "Stats", "speed")?,
            min_num_pads_to_collect: optional(stats, "Stats", "minNumPadsToCollectEMP")?,
            max_num_pads_to_collect: optional(stats, "Stats", "maxNumPadsToCollectEMP")?,
            min_distance: optional(stats, "Stats", "minDistance")?,
            max_distance: optional(stats, "Stats", "maxDistance")?,
        }),
        None => None,
    };
    let parsed = DetonatorStats {
        energy_recharge_per_stage: read("Global", "energy_recharge_per_stage")?,
        mine: DetonatorMine {
            points_per_metre: read("Mine", "points_per_metre")?,
            max_distance: read("Mine", "max_distance")?,
            velocity_reduction: read("Mine", "velocity_reduction")?,
        },
        cannon: DetonatorCannon {
            num_rebounds: read("Cannon", "num_rebounds")?,
            round_recharge_time: read("Cannon", "round_recharge_time")?,
            recharge_pause: read("Cannon", "recharge_pause")?,
        },
        bomb: DetonatorBomb {
            number_of_shots_to_destroy: read("Bomb", "number_of_shots_to_destroy")?,
            energy_recharge_per_shot: read("Bomb", "energy_recharge_per_shot")?,
            points_for_cannon_bullet_hit: read("Bomb", "points_for_cannon_bullet_hit")?,
            points_for_destroying: read("Bomb", "points_for_destroying")?,
        },
        emp,
    };
    Ok((parsed != DetonatorStats::default()).then_some(parsed))
}
