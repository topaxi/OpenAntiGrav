//! Detonator's own object models: the parts of a laid mine and bomb beyond the one the ordinary
//! weapon pools draw, and the model every speed pad and weapon pad carries.
//!
//! The first part of each (`mine_rita_football`, `detonator_bomb_final`) rides the Mine's and the
//! Bomb's own pools ([`super::scene`]'s `mines`/`bombs`), loaded in place of the ordinary bodies
//! (`oag_title::DetonatorObjects`). These pools carry the rest, each fed the matrices its
//! first part is, so a part is the same object posed the same way. The container rides
//! [`super::bomb_blast::BombBlastModels`] for the Repulser field's reason: a seventh element of
//! every weapon tuple from `load` to `Scene::new` is not worth a new argument.
//!
//! Evidence: `docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`, "The Detonator objects'
//! models". The mine's `Electric_Visualiser` is music-driven (`MasterVolume`, `BassHit`,
//! `Tempo` ...) and is not loaded or drawn.

use super::*;

/// How many mine bodies a pool holds: the larger of the projectile pool and Detonator's field.
pub(crate) const MINE_POOL: usize =
    if oag_weapons::projectile::MAX_PROJECTILES > oag_weapons::detonator::MAX_MINES {
        oag_weapons::projectile::MAX_PROJECTILES
    } else {
        oag_weapons::detonator::MAX_MINES
    };

/// The models [`oag_title::DetonatorObjects`] names beyond the first part of a mine and a bomb.
/// Empty outside a Detonator race.
#[derive(Debug, Clone, Default)]
pub struct DetonatorModels {
    /// `mine_rita_spikes`, on every laid mine.
    pub mine_spikes: Option<Model>,
    /// `detonator_bomb_final_equalisers`, on every laid bomb.
    pub bomb_equalisers: Option<Model>,
    /// `detonator_bomb_glowedges`, on every laid bomb.
    pub bomb_glow_edges: Option<Model>,
    /// `detonator_cannonbolt`, on every pulse cannon round in flight.
    pub cannon_bolt: Option<Model>,
    /// `hd_muzzleflash_detonator`, at a cannon locator while a round is young.
    pub muzzle_flash: Option<Model>,
    /// `Detonator_Pad`, once per pad in [`Self::pads`].
    pub pad: Option<Model>,
    /// Each speed pad's and weapon pad's pad-local to world matrix: where [`Self::pad`] is drawn.
    pub pads: Vec<Mat4>,
}

/// The drawable pools [`Scene`] builds from a [`DetonatorModels`].
#[derive(Debug, Default)]
pub(crate) struct DetonatorDrawables {
    pub(crate) mine_spikes: Vec<Drawable>,
    pub(crate) bomb_equalisers: Vec<Drawable>,
    pub(crate) bomb_glow_edges: Vec<Drawable>,
    pub(crate) cannon_bolt: Vec<Drawable>,
    pub(crate) muzzle_flash: Vec<Drawable>,
    pub(crate) pad: Vec<Drawable>,
    pub(crate) pads: Vec<Mat4>,
    /// Every laid mine's music envelopes: view state, stepped as the frames are drawn.
    pub(crate) envelopes: std::sync::Mutex<crate::detonator_look::MineEnvelopes>,
}

impl DetonatorDrawables {
    /// Builds each pool with `build`: the model and how many slots it needs.
    pub(crate) fn build(
        models: DetonatorModels,
        mut build: impl FnMut(Option<Model>, usize) -> Result<Vec<Drawable>>,
    ) -> Result<Self> {
        let pad_slots = models.pads.len();
        Ok(Self {
            mine_spikes: build(models.mine_spikes, MINE_POOL)?,
            bomb_equalisers: build(
                models.bomb_equalisers,
                oag_weapons::projectile::MAX_PROJECTILES,
            )?,
            bomb_glow_edges: build(
                models.bomb_glow_edges,
                oag_weapons::projectile::MAX_PROJECTILES,
            )?,
            cannon_bolt: build(models.cannon_bolt, oag_weapons::detonator::cannon::POOL)?,
            muzzle_flash: build(models.muzzle_flash, oag_weapons::detonator::cannon::POOL)?,
            pad: if pad_slots == 0 {
                Vec::new()
            } else {
                build(models.pad, pad_slots)?
            },
            pads: models.pads,
            envelopes: Default::default(),
        })
    }

    /// Every pool, for the passes that treat the weapon pools alike.
    pub(crate) fn pools_mut(&mut self) -> [&mut Vec<Drawable>; 6] {
        [
            &mut self.mine_spikes,
            &mut self.bomb_equalisers,
            &mut self.bomb_glow_edges,
            &mut self.cannon_bolt,
            &mut self.muzzle_flash,
            &mut self.pad,
        ]
    }
}

#[cfg(feature = "view")]
impl crate::Loaded {
    /// The craft in `slot`'s `cannon_flash_left` and `cannon_flash_right` locators, `[left, right]`
    /// in hull space: where Detonator's cannon leaves it. Authored by the hull's own
    /// `locators.vex` (`/data/ships/detonator/`, `docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`,
    /// "The disc's Detonator folder"), so the hull and the muzzles come from one folder. Both
    /// `None` for a source that authors none, and for a slot past the grid.
    #[must_use]
    pub fn cannon_muzzles(&self, slot: usize) -> [Option<Mat4>; 2] {
        self.liveries
            .get(slot)
            .map_or([None, None], |livery| livery.cannon_flash)
    }
}
