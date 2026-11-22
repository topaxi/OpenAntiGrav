//! HD's damage smoke: what a craft trails after a weapon hit leaves it hurt.
//!
//! # Recovered (HD, PS3)
//!
//! `ShipDamageFx_Update_q` (`0x002a17e8`) is called from the weapon branch of
//! HD's damage function (`0x000e7760`, `source == 2`, the shield still above
//! zero, not a LeachBeam) with the shield left after the hit, in the same
//! absolute units as the class's own pool (`+0x90`). It picks a state:
//!
//! - above **70**: `WO_DAMAGE_MILD`;
//! - above **40**, up to 70: `WO_DAMAGE_MODERATE`;
//! - 40 or under: `WO_DAMAGE_CRITICAL` **and** `WO_DAMAGE_MODERATE`.
//!
//! A state is an attached one-shot (200 ticks, none of the three loops), and
//! writing one sets a `3.3` s hold. While the hold runs a hit that lands in
//! the same state does nothing; a hit that changes the state plays the new one
//! straight away; once it has run out any hit plays the current state again.
//! The smoke is therefore per hit and not a level: nothing replays it while the
//! craft simply flies, and a recharge draws nothing. A state change ends the previous state's
//! smoke outright, particles included (`0x002a1994`), and every smoke carries the craft's
//! velocity (`0x002a1c18`; `docs/ghidra/functions/ps3-hdfury-eu/particle-triggers.md`, "the
//! damage smoke re-read").
//!
//! # Chosen, not measured
//!
//! - **The hold counts down by the tick.** Its decrement site was not read; the
//!   `3.3` s equals the effects' own 200 ticks, which is what a hold exists to
//!   cover.
//! - **The anchor's sign along the node's Z is read off a frame**, not off the code: the
//!   original adds `1.5` of the flare node's third row (`0x002a0268`), and which way that row
//!   points along the hull is settled on `docs/ghidra/functions/ps3-hdfury-eu/particle-triggers.md`
//!   ("The smoke's anchor against a frame").
//! - **The death flag is a spent shield.** `object + 0x2b4`, which selects the `2.0` offset,
//!   is not read to its writer; a craft with no shield left stands for it.
//! - **A craft with no `Engine Flare` locator** anchors at its origin.
//! - **Every craft smokes.** The original's per-viewport mask test
//!   (`craft + 0x7a3c`) was not read, as for [`super::hit_sparks`].
//! - **The Zone's `WO_DAMAGE_ELECTRIC`** is not played: it needs the mode map.
//!
//! The effect owner sounds a damage cue alongside (`+0x1a4..+0x1b0` are sound
//! handles, not effect handles); that is left to the audio lane.

use super::*;
use oag_title::Trigger;

/// Shield above this: [`State::Mild`] (`DAT_008b2fb4`).
pub const MILD_ABOVE: f32 = 70.0;

/// Shield above this and not above [`MILD_ABOVE`]: [`State::Moderate`]
/// (`DAT_008b2fb8`).
pub const MODERATE_ABOVE: f32 = 40.0;

/// The hold a state change writes, seconds (`DAT_008b2fb0`).
pub const HOLD: f32 = 3.3;

/// How far along the flare node's Z row the smoke leaves its origin (`0xbfc00000` at
/// `0x002a0308`, subtracted).
pub const ANCHOR_ALONG_Z: f32 = 1.5;

/// The same once the death flag is set.
pub const ANCHOR_ALONG_Z_DEAD: f32 = 2.0;

/// The smoke's anchor in the craft's model space: the flare node's origin plus
/// [`ANCHOR_ALONG_Z`] ([`ANCHOR_ALONG_Z_DEAD`] when `dead`) of its Z row. `axis` carries the
/// hull's `0.75` scale as the original's row does.
#[must_use]
pub fn anchor(nozzle: Vec3, axis: Vec3, dead: bool) -> Vec3 {
    let along = if dead {
        ANCHOR_ALONG_Z_DEAD
    } else {
        ANCHOR_ALONG_Z
    };
    nozzle + axis * along
}

/// A craft's smoke state, `craft + 0xe0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Shield above [`MILD_ABOVE`].
    Mild,
    /// Shield above [`MODERATE_ABOVE`] and up to [`MILD_ABOVE`].
    Moderate,
    /// Shield at or under [`MODERATE_ABOVE`].
    Critical,
}

impl State {
    /// The state a shield of `shield` is in.
    #[must_use]
    pub fn of(shield: f32) -> Self {
        if shield > MILD_ABOVE {
            Self::Mild
        } else if shield > MODERATE_ABOVE {
            Self::Moderate
        } else {
            Self::Critical
        }
    }

    /// The effects this state plays, in the order the original spawns them.
    fn triggers(self) -> &'static [Trigger] {
        match self {
            Self::Mild => &[Trigger::DamageMild],
            Self::Moderate => &[Trigger::DamageModerate],
            Self::Critical => &[Trigger::DamageCritical, Trigger::DamageModerate],
        }
    }
}

/// What the hold lets through for a hit that finds the craft in `state`
/// while it last wrote `last` and `hold` seconds are left.
#[must_use]
pub fn plays(state: State, last: State, hold: f32) -> bool {
    hold <= 0.0 || state != last
}

/// One smoke riding its craft.
#[derive(Debug, Clone, Copy)]
struct Riding {
    slot: usize,
    playing: psys::Playing,
    /// A damage state's smoke, which the next state kills; not a LeachBeam spark.
    state_smoke: bool,
}

/// The damage smoke's own state: per-craft state and hold, and the instances
/// riding hulls.
#[derive(Debug, Clone)]
pub(super) struct DamageFx {
    last: [State; MAX_SHIPS],
    hold: [f32; MAX_SHIPS],
    riding: Vec<Riding>,
    /// How many have started, ever - for tests.
    started: u32,
}

impl Default for DamageFx {
    fn default() -> Self {
        Self {
            last: [State::Mild; MAX_SHIPS],
            hold: [0.0; MAX_SHIPS],
            riding: Vec::new(),
            started: 0,
        }
    }
}

impl DamageFx {
    /// How many are still riding their craft.
    #[cfg(test)]
    pub(super) fn riding_len(&self) -> usize {
        self.riding.len()
    }
}

impl Race {
    /// Where every smoke riding a craft sits now, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn damage_smoke_anchors_for_tests(&self) -> Vec<Vec3> {
        self.view
            .damage_fx
            .riding
            .iter()
            .filter_map(|smoke| self.view.stage.anchor_of(smoke.playing))
            .collect()
    }

    /// How many smokes have ever started, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn damage_smokes_started_for_tests(&self) -> u32 {
        self.view.damage_fx.started
    }
}

#[cfg(feature = "view")]
impl RaceView {
    /// Where `slot`'s smoke leaves it now, world space: [`anchor`] through the pose the hull is
    /// drawn at, or the craft's origin when its hull has no `Engine Flare` locator.
    fn smoke_anchor(&self, core: &SimRace, slot: usize) -> Vec3 {
        let ship = &core.sim.world.ships[slot];
        match (
            self.nozzles.get(slot).copied().flatten(),
            self.nozzle_axes.get(slot).copied().flatten(),
        ) {
            (Some(nozzle), Some(axis)) => self.drawn_model(core, slot).transform_point3(anchor(
                nozzle,
                axis,
                ship.physics.shield <= 0.0,
            )),
            _ => self.drawn_position(core, slot),
        }
    }

    /// A weapon hit landed on `slot`: plays the smoke its remaining shield
    /// calls for. Nothing on a title that answers no [`Trigger::DamageMild`],
    /// on a craft with no shield left, or while the hold rules it out.
    pub(super) fn throw_damage_smoke(&mut self, core: &SimRace, slot: usize) {
        let ship = &core.sim.world.ships[slot];
        let shield = ship.physics.shield;
        if shield <= 0.0 || !ship.in_world() {
            return;
        }
        let state = State::of(shield);
        let fx = &self.damage_fx;
        if !plays(state, fx.last[slot], fx.hold[slot]) {
            return;
        }
        let at = self.smoke_anchor(core, slot);
        let velocity = ship.physics.body.linear_velocity;
        let changed = state != self.damage_fx.last[slot];
        let mut any = false;
        for &trigger in state.triggers() {
            let Some(effect) = self.handles.get(trigger).cloned() else {
                continue;
            };
            if changed && !any {
                self.kill_state_smoke(slot);
            }
            if let Some(playing) = self.stage.play_riding(&effect, at, 1.0) {
                self.stage.carry(playing, &effect, velocity);
                self.damage_fx.riding.push(Riding {
                    slot,
                    playing,
                    state_smoke: true,
                });
                self.damage_fx.started += 1;
                any = true;
            }
        }
        if any {
            self.damage_fx.last[slot] = state;
            self.damage_fx.hold[slot] = HOLD;
        }
    }

    /// A landed LeachBeam hit on `slot`, on a title whose hit spark is not
    /// thrown from the hull's locators (HD): one burst of
    /// [`Trigger::LeachHitSpark`] attached at the craft's damage-effect point.
    ///
    /// `0x000e7760` sets a flag (`object + 0x260`) on a weapon kind 7 hit and
    /// `0x002a06e0`, running every tick, clears it and spawns
    /// `WO_SHIP_SPARK_DAMAGE_LEACHBEAM` attached at `+0x1c0`, the point
    /// [`State`]'s smoke uses. No state, no hold: every hit throws one.
    pub(super) fn throw_attached_leach_spark(&mut self, core: &SimRace, slot: usize) {
        let ship = &core.sim.world.ships[slot];
        if !ship.in_world() {
            return;
        }
        let at = self.smoke_anchor(core, slot);
        let Some(effect) = self.handles.get(Trigger::LeachHitSpark).cloned() else {
            return;
        };
        if let Some(playing) = self.stage.play_riding(&effect, at, 1.0) {
            self.damage_fx.riding.push(Riding {
                slot,
                playing,
                state_smoke: false,
            });
            self.damage_fx.started += 1;
        }
    }

    /// Ends `slot`'s previous damage state outright, particles included (`0x002a1994`, which
    /// calls `0x0027f7b0(.., 1)` and `0x00285868`; `particle-triggers.md`, "the damage smoke
    /// re-read").
    fn kill_state_smoke(&mut self, slot: usize) {
        let stage = &mut self.stage;
        self.damage_fx.riding.retain(|smoke| {
            let ends = smoke.slot == slot && smoke.state_smoke;
            if ends {
                stage.kill(smoke.playing);
            }
            !ends
        });
    }

    /// Counts the holds down and keeps every emitting smoke on its craft.
    /// Before `Stage::advance`, like the hit sparks.
    pub(super) fn advance_damage_smoke(&mut self, core: &SimRace) {
        let dt = core.sim.dt;
        for left in &mut self.damage_fx.hold {
            *left = (*left - dt).max(0.0);
        }
        let mut riding = std::mem::take(&mut self.damage_fx.riding);
        riding.retain(|smoke| {
            let ship = &core.sim.world.ships[smoke.slot];
            if !ship.in_world() || !self.stage.is_emitting(smoke.playing) {
                self.stage.detach(smoke.playing);
                return false;
            }
            self.stage
                .follow(smoke.playing, self.smoke_anchor(core, smoke.slot));
            true
        });
        self.damage_fx.riding = riding;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thresholds_are_the_executables() {
        assert_eq!(State::of(100.0), State::Mild);
        assert_eq!(State::of(70.1), State::Mild);
        assert_eq!(State::of(70.0), State::Moderate);
        assert_eq!(State::of(40.1), State::Moderate);
        assert_eq!(State::of(40.0), State::Critical);
        assert_eq!(State::of(0.5), State::Critical);
    }

    #[test]
    fn the_hold_lets_a_change_through_and_silences_a_repeat() {
        assert!(!plays(State::Mild, State::Mild, 1.0));
        assert!(plays(State::Moderate, State::Mild, 1.0));
        assert!(plays(State::Mild, State::Mild, 0.0));
    }

    #[test]
    fn critical_plays_both_of_its_effects() {
        assert_eq!(
            State::Critical.triggers(),
            &[Trigger::DamageCritical, Trigger::DamageModerate]
        );
    }
}
