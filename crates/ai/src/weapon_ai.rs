//! When an opponent fires the forward weapon it is holding: the original's own
//! law.
//!
//! `WeaponAi_Update` (`0x08851550`) and `WeaponAi_DecideFireOrAbsorb`
//! (`0x088518b4`) decide this in Wipeout Pulse; this module is that decision,
//! fire and absorb, for the weapons the forward setup (`0x088508d4`) builds.
//! Every constant and branch is read off `BOOT.BIN`, see
//! `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md` for addresses and
//! confidence. What is **chosen** is said so where it happens, with no score.
//!
//! # The law
//!
//! A craft holding a forward weapon rolls once a tick, as the original does:
//! `AiManager_Update` calls every `WeaponAi_Update` once a world step, and its
//! quarter-second gate opens once and stays open. The chance is a rate from
//! a five-entry table, indexed by how close the nearest craft ahead and behind
//! are, times the weapon's authored `useAgainstPlayer` or `useAgainstAI`
//! (`Data\XML\WeaponAIstats.xml`), times five in an Eliminator. Nothing fires
//! before 0.8 s held. An **aimed** weapon (anything leaving the nose as a shot
//! or beam) also needs a craft in its predicted path ([`target_in_path`]); a roll
//! with nothing there arms a three-second window in which a target that appears
//! gets a second, flat chance. The Quake is not aimed: the roll alone.
//!
//! # Not covered
//!
//! - **Absorbing in an Eliminator**: there the original's absorb spends the
//!   weapon on a Shield rather than energy, and was swept and did not move
//!   the time to five kills. `oag_raceplay` keeps the weapon instead.
//! - **The Cannon**: the original's opponent fire byte reaches a bit nothing
//!   dispatches and the Cannon fires off an uninitialised byte
//!   (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`), so
//!   there is no law; `Driver::holds_fire` stays ours.
//! - **Mines, Bomb, Turbo, Shield**: each has its own setup in the original,
//!   and this project's own rule in `oag_raceplay`.
//!
//! # Ticks, not seconds
//!
//! [`WeaponAi`] sits on the craft in the world snapshot beside
//! [`crate::Driver`], so it is integers and stays `Eq`: the original's two `f32`
//! clocks (`+0x2c` held, `+0x54` wait) become tick counts at 60 Hz.

use oag_core::math::Vec3;

/// `g_weapon_ai_rate` (`0x08ab0eac`): the per-decision rate, by fire index.
///
/// Read out of the image, confidence 80. Index 0 is **exactly zero**: outside
/// an Eliminator an opponent with nobody close ahead never fires.
pub const RATE: [f32; 5] = [0.0, 0.0005, 0.002, 0.008, 0.05];

/// `g_weapon_ai_rate_eliminator` (`0x08ab0ec0`), the table modes 8 and `0x12`
/// read instead. Confidence 80.
pub const RATE_ELIMINATOR: [f32; 5] = [0.001, 0.002, 0.01, 0.02, 0.1];

/// How long a weapon is held before the fire roll may succeed: `+0x2c > 0.8`.
/// `+0x2c` gains `dt` per decision, so at 60 Hz that is 48 ticks, strictly: the
/// 49th.
pub const HOLD_TICKS: u32 = 48;

/// The window an aimed weapon arms when its roll finds nothing in the path:
/// `+0x54 = 3.0`, counted down by `dt` at the top of every call.
pub const WAIT_TICKS: u32 = 180;

/// Along-track gap inside which the craft ahead raises the fire index by three
/// and the craft behind lowers it by one (`0x088508d4`, `0x08850d34`).
pub const CLOSE: f32 = 100.0;

/// The player counts as a target for a forward weapon inside this gap ahead,
/// which picks `useAgainstPlayer` over `useAgainstAI`.
pub const PLAYER_AHEAD: f32 = 150.0;

/// The predicted-path test's horizon, in seconds: a closest approach later than
/// this is no target (`0x41a00000` at `0x0885109c`).
pub const PATH_HORIZON: f32 = 20.0;

/// The miss allowed per unit the shot has flown to the closest approach
/// (`0x3e19999a` at `0x08851080`): a cone of about eight and a half degrees.
pub const PATH_SLOPE: f32 = 0.15;

/// And the miss allowed at no distance at all (`0x40800000` at `0x088510ac`).
pub const PATH_FLOOR: f32 = 4.0;

/// The `useAgainst*` multiplier in an Eliminator (`0x08851b10..5c`).
pub const ELIMINATOR_USE: f32 = 5.0;

/// How long a weapon is held before an absorb roll may succeed:
/// `+0x2c > 0.2`, at 60 Hz 12 ticks, strictly.
pub const ABSORB_HOLD_TICKS: u32 = 12;

/// Held longer than this, the absorb index rises by one: `+0x2c > 15.0`
/// (`0x088508d4`).
pub const ITCHY_TICKS: u32 = 15 * 60;

/// No craft ahead inside this gap raises the absorb index by one
/// (`0x088508d4`: `gap_ahead == 0 || gap_ahead > 300`).
pub const NOTHING_AHEAD: f32 = 300.0;

/// `FUN_088509e4`'s shield tiers: the absorb index rises by one for each of
/// these the craft's own shield is at or below (`> 80` none, `< 20` four).
/// Read off Pulse at confidence 80; HD, 2048 and Omega inherit them on their
/// own shield scale, unmeasured there.
pub const SHIELD_TIERS: [f32; 4] = [80.0, 60.0, 40.0, 20.0];

/// What a craft does with its held weapon this tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    /// Keeps holding it.
    Hold,
    /// Fires it (`craft+0x15`).
    Fire,
    /// Absorbs it into its shield (`craft+0x17`).
    Absorb,
}

/// One craft's weapon-AI clocks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WeaponAi {
    /// Ticks the current pickup has been held, `0` with an empty slot (`+0x2c`).
    pub held_ticks: u32,
    /// Ticks left on the window an aimed weapon armed when its roll found no
    /// target (`+0x54`); `0` when none is open.
    pub wait_ticks: u32,
}

/// A weapon's two `useAgainst*` multipliers, out of `WeaponAIstats.xml`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Odds {
    /// While the player is inside [`PLAYER_AHEAD`].
    pub use_against_player: f32,
    /// Otherwise.
    pub use_against_ai: f32,
    /// `absorb`, the absorb roll's multiplier (`stats[0x10]`).
    pub absorb: f32,
}

/// What the decision reads this tick, measured by the caller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Situation {
    /// The weapon is aimed (`+0x58`): it waits for [`target_in_path`].
    pub aimed: bool,
    /// The race is an Eliminator: [`RATE_ELIMINATOR`], a fire index of at least
    /// one, and [`ELIMINATOR_USE`].
    pub eliminator: bool,
    /// Along-track gap to the nearest craft ahead, if any (`+0x3c`).
    pub gap_ahead: Option<f32>,
    /// Along-track gap to the nearest craft behind, positive, if any (`+0x40`).
    pub gap_behind: Option<f32>,
    /// Signed along-track gap to the player, positive ahead (`+0x60`).
    pub player_gap: Option<f32>,
    /// [`target_in_path`] found a craft (`+0x52 == 0`).
    pub target_in_path: bool,
    /// The craft's own shield pool, which `FUN_088509e4` tiers.
    pub shield: f32,
    /// This weapon's row of `WeaponAIstats.xml`.
    pub odds: Odds,
}

impl Situation {
    /// The fire index (`+0x38`) a forward weapon's setup builds, clamped.
    ///
    /// `0x088508d4`: three for a craft inside [`CLOSE`] ahead, less one for one
    /// inside [`CLOSE`] behind. Its other term adds two when nothing is close
    /// ahead and `+0x44` is non-zero; nothing in the image writes `+0x44`
    /// (uninitialised heap), so **taken as zero, chosen, not measured**. The
    /// Eliminator raises a zero to one (`0x08851a8c`).
    #[must_use]
    pub fn fire_index(&self) -> usize {
        let mut index: i32 = 0;
        if self.gap_ahead.is_some_and(|gap| gap > 0.0 && gap < CLOSE) {
            index += 3;
        }
        if self.gap_behind.is_some_and(|gap| gap > 0.0 && gap < CLOSE) {
            index -= 1;
        }
        let index = index.clamp(0, 4);
        if self.eliminator && index == 0 {
            1
        } else {
            index as usize
        }
    }

    /// The absorb index (`+0x34`) a forward weapon's setup builds for a
    /// weapon held `held_ticks`, clamped: one per [`SHIELD_TIERS`] the shield
    /// is at or below, one past [`ITCHY_TICKS`] held, one with nobody inside
    /// [`NOTHING_AHEAD`]. An Eliminator forces it to zero (`0x08851a8c`).
    #[must_use]
    pub fn absorb_index(&self, held_ticks: u32) -> usize {
        if self.eliminator {
            return 0;
        }
        let mut index = SHIELD_TIERS
            .iter()
            .filter(|&&tier| self.shield <= tier)
            .count();
        if held_ticks > ITCHY_TICKS {
            index += 1;
        }
        if self
            .gap_ahead
            .is_none_or(|gap| gap <= 0.0 || gap > NOTHING_AHEAD)
        {
            index += 1;
        }
        index.min(4)
    }

    /// The chance an absorb roll succeeds this tick, the long way as
    /// [`Self::fire_chance`] is.
    #[must_use]
    pub fn absorb_chance(&self, held_ticks: u32) -> f32 {
        let rate = self.rates()[self.absorb_index(held_ticks)];
        if rate == 0.0 || self.odds.absorb == 0.0 {
            return 0.0;
        }
        1.0 / ((1.0 / rate) * (1.0 / self.odds.absorb))
    }

    /// The table this race reads.
    #[must_use]
    pub fn rates(&self) -> &'static [f32; 5] {
        if self.eliminator {
            &RATE_ELIMINATOR
        } else {
            &RATE
        }
    }

    /// The chance a fire roll succeeds this tick.
    ///
    /// Computed the original's long way, `1 / ((1 / rate) * (1 / use))`, which
    /// is not the same function as `rate * use` in `f32`.
    #[must_use]
    pub fn fire_chance(&self) -> f32 {
        let rate = self.rates()[self.fire_index()];
        let player_in_reach = self
            .player_gap
            .is_some_and(|gap| gap > 0.0 && gap < PLAYER_AHEAD);
        let mut odds = if player_in_reach {
            self.odds.use_against_player
        } else {
            self.odds.use_against_ai
        };
        if self.eliminator {
            odds *= ELIMINATOR_USE;
        }
        if rate == 0.0 {
            return 0.0;
        }
        1.0 / ((1.0 / rate) * (1.0 / odds))
    }
}

impl WeaponAi {
    /// Advances both clocks one tick; call it every tick, holding or not. The
    /// wait counts down first, as `+0x54` does at the top of `WeaponAi_Update`;
    /// an empty slot resets the held count.
    pub fn tick(&mut self, holding: bool) {
        self.wait_ticks = self.wait_ticks.saturating_sub(1);
        self.held_ticks = if holding {
            self.held_ticks.saturating_add(1)
        } else {
            0
        };
    }

    /// What to do this tick. Call after [`Self::tick`].
    ///
    /// `early`, `roll` and `absorb` are three independent draws in `0..1`: the
    /// open window's flat [`RATE`]`[4]` chance, the fire roll and the absorb
    /// roll, which is only drawn on when the fire roll fails or the hold is
    /// not met.
    pub fn decide(
        &mut self,
        situation: &Situation,
        early: f32,
        roll: f32,
        absorb: f32,
    ) -> Decision {
        // The window an earlier roll opened: a target now in the path gets the
        // table's top rate, whatever the hold time and the odds.
        if situation.aimed
            && self.wait_ticks != 0
            && situation.target_in_path
            && early < situation.rates()[4]
        {
            self.wait_ticks = 0;
            return Decision::Fire;
        }
        if roll >= situation.fire_chance() || self.held_ticks <= HOLD_TICKS {
            if absorb < situation.absorb_chance(self.held_ticks)
                && self.held_ticks > ABSORB_HOLD_TICKS
            {
                return Decision::Absorb;
            }
            return Decision::Hold;
        }
        if !situation.aimed || situation.target_in_path {
            return Decision::Fire;
        }
        self.wait_ticks = WAIT_TICKS;
        Decision::Hold
    }
}

/// The noise stream [`WeaponAi::decide_for`] draws the window's chance from.
const EARLY_STREAM: u32 = 6;

/// And the fire roll's: a stream of its own, so the two are not one coin.
const FIRE_STREAM: u32 = 7;

/// And the absorb roll's.
const ABSORB_STREAM: u32 = 8;

impl WeaponAi {
    /// [`Self::decide`] with both draws from `driver`'s own noise this tick,
    /// never the world's generator (see `crate::noise::roll`).
    pub fn decide_for(&mut self, driver: &crate::Driver, situation: &Situation) -> Decision {
        let early = crate::noise::roll(driver.seed, driver.phase, EARLY_STREAM);
        let roll = crate::noise::roll(driver.seed, driver.phase, FIRE_STREAM);
        let absorb = crate::noise::roll(driver.seed, driver.phase, ABSORB_STREAM);
        self.decide(situation, early, roll, absorb)
    }
}

/// A craft as the predicted-path test sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mover {
    pub position: Vec3,
    /// World velocity, units a second.
    pub velocity: Vec3,
}

/// Whether a shot fired now along `forward` at `speed` passes close enough to
/// any of `others` to be worth firing: `WeaponAi_FindTargetInPath`
/// (`0x08850edc`).
///
/// Everything is first laid into the shooter's plane (normal `up`), keeping each
/// vector's length. A craft counts when the closest approach between the shot
/// and it, each on a straight line, lies within [`PATH_HORIZON`] seconds and the
/// miss is under [`PATH_SLOPE`] times the shot's travel plus [`PATH_FLOOR`].
///
/// `speed` is the weapon's authored per-class figure **as authored**: the
/// original does not divide it by 3.6 here, where its launchers do. No walls: a
/// craft round a corner is a target if the straight line passes it.
#[must_use]
pub fn target_in_path(
    position: Vec3,
    up: Vec3,
    forward: Vec3,
    speed: f32,
    others: impl IntoIterator<Item = Mover>,
) -> bool {
    let shot = flatten(up, forward * speed);
    for other in others {
        let at = flatten(up, other.position - position) + position;
        let velocity = flatten(up, other.velocity);
        let gap = flatten(up, at - position);
        let closing = flatten(up, velocity - shot);
        let t = -gap.dot(closing) / closing.dot(closing);
        // Both comparisons false on a NaN, as the original's `c.le.s`/`c.lt.s`
        // pair is: a craft moving exactly with the shot is no target.
        if t > 0.0 && t < PATH_HORIZON {
            let miss = (gap + closing * t).length();
            let flown = ((position + shot * t) - position).length();
            if miss < flown * PATH_SLOPE + PATH_FLOOR {
                return true;
            }
        }
    }
    false
}

/// `v` laid into the plane with normal `up`, keeping its length (`0x0885058c`).
fn flatten(up: Vec3, v: Vec3) -> Vec3 {
    let length = v.dot(v).sqrt();
    let laid = v - up * v.dot(up);
    let laid_length = laid.dot(laid).sqrt();
    if laid_length < 1e-8 {
        laid
    } else {
        laid * (length / laid_length)
    }
}

#[cfg(test)]
mod tests;
