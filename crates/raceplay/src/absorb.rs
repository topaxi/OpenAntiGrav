//! What a craft shows when it absorbs a pickup: the `WO_WEAPON_ABSORB` burst,
//! played at the hull's own authored locators on a stagger.
//!
//! The sound (`Cue::Absorb`) was always raised; this is the picture that goes
//! with it. Both are one call in the original, so [`Race::play_absorb_feedback`]
//! raises both, and both absorb paths call it: `Race::spend_pickup`'s absorb
//! arm and the Eliminator lap refill.
//!
//! # Recovered, per title
//!
//! - **Pulse** (`Ship_PlayAbsorbFeedback`, `0x08840640`): the effect once per
//!   `Ship Collision Fx` node the hull carries, up to ten, node `i` delayed
//!   `i * 0.1` seconds. The node list is `Ship_GatherCollisionFxNodes`
//!   (`0x0883ea50`), a depth-first walk of the live hull model for that one
//!   class; the delay is `DAT_08abf564`, which the particle node spawner
//!   (`FUN_08916200`) moves into the node's `+0xb8` and its update
//!   (`FUN_08915cdc`) counts down before the system first runs. The node is
//!   the system's parent, so a burst rides the hull. See
//!   `docs/ghidra/functions/psp-pulse-usa/shield.md`.
//! - **Pure** (`FUN_08925e20`): the same loop over the same class, eight
//!   nodes instead of ten. See
//!   `docs/ghidra/functions/psp-pure-usa/rocket-and-collision-fx.md`.
//! - **Wipeout HD** (`FUN_000d9398`): a different class and a different
//!   shape. Six `absorb` locators (`oag_vex::vex::CLASS_ABSORB`), fired as
//!   three mirrored pairs - slots 0 and 5, then 1 and 4, then 2 and 3 - at
//!   `0.0`, `0.2` and `0.4` seconds, and only when all six are authored. See
//!   `docs/ghidra/functions/ps3-hdfury-eu/absorb-feedback.md`.
//!
//! # What is ours
//!
//! Severity `1.0`: `ShipCollisionFx_Trigger` never writes its severity field
//! for the absorb kind, so the effect plays as authored. And a burst's
//! emitter frame is world up rather than the locator's live one, which
//! [`psys::Stage`] does for every effect it plays - measured harmless here:
//! `WO_WEAPON_ABSORB`'s two emitters are a full-sphere radial spray and a
//! zero-speed ring, neither of which reads the frame.

use super::*;
use oag_title::Burst;

/// The effect every absorb plays - `Data\Psys\WO_WEAPON_ABSORB.POB` on every
/// title that has one.
pub const ABSORB_EFFECT: &str = "WO_WEAPON_ABSORB";

/// The burst `title` plays on an absorb, off its [`oag_title::Effects`] table:
/// `None` for a title whose absorb path is unread (2048, Omega), which then
/// plays nothing. Keyed on the title the craft comes from - the same footing
/// `Setup::shield_palette` is chosen on.
#[must_use]
pub fn absorb_burst_for(title: &oag_title::Title) -> Option<Burst> {
    title
        .effect_on(oag_title::Trigger::ShieldAbsorb)
        .and_then(|spec| spec.burst)
}

/// Each slot's locators for `burst`, model space: the `Ship Collision Fx` set
/// for a [`Burst::Sequential`] title, the `absorb` set for HD's
/// [`Burst::MirroredPairs`], and nothing for a title with no burst.
pub(super) fn anchors(burst: Option<Burst>, liveries: &[oag_livery::Livery]) -> Vec<Vec<Vec3>> {
    liveries
        .iter()
        .map(|livery| match burst {
            Some(Burst::Sequential { .. }) => {
                livery.collision_fx.iter().map(|a| a.position).collect()
            }
            Some(Burst::MirroredPairs { .. }) => livery.absorb.clone(),
            None => Vec::new(),
        })
        .collect()
}

/// One scheduled `WO_WEAPON_ABSORB` instance.
#[derive(Debug, Clone, Copy)]
pub(super) struct PendingAbsorb {
    /// The absorbing craft.
    slot: usize,
    /// Index into that craft's `RaceView::absorb_anchors`.
    node: usize,
    /// Seconds before it starts - the node's own `+0xb8`.
    delay: f32,
    /// The instance once started.
    playing: Option<psys::Playing>,
}

impl Race {
    /// `Ship_PlayAbsorbFeedback`: the `ABSORB` cue, then the staggered burst.
    ///
    /// The cue is raised unconditionally, as it was before the burst existed;
    /// only the picture depends on the hull carrying locators.
    ///
    /// `pickup` is the absorb handler's own call, which also starts the hull
    /// overlay: `FUN_08844ec4` stores the craft's clock into `+0x878` just
    /// before it calls the feedback (`0x088455ac..b4`), and the Eliminator's
    /// `Ship_RefillLapShield` does not - so a lap refill bursts and never
    /// lights the hull. See `oag_fx::hull_overlay`.
    ///
    /// HD's shell timer is stamped on **every** call, the lap refill included:
    /// `FUN_000d9398` stores `craft+0x7a5c` after all of its branches. It only
    /// draws on a craft whose livery loaded a shell, so the stamp is harmless
    /// on every other title.
    pub(super) fn play_absorb_feedback(&mut self, slot: usize, pickup: bool) {
        if pickup && let Some(elapsed) = self.view.absorb_overlay.get_mut(slot) {
            *elapsed = Some(0.0);
        }
        if let Some(shell) = self.view.absorb_shell.get_mut(slot) {
            shell.stamp();
        }
        self.sim.cues.push(oag_sound::sfx::CueEvent::new(
            oag_sound::sfx::Cue::Absorb,
            slot,
        ));
        let Some(burst) = self.view.absorb_burst else {
            return;
        };
        let nodes = self.view.absorb_anchors.get(slot).map_or(0, Vec::len);
        for (node, delay) in burst.schedule(nodes) {
            self.view.absorb_bursts.push(PendingAbsorb {
                slot,
                node,
                delay,
                playing: None,
            });
        }
    }

    /// Counts every scheduled burst down, starts the ones that are due, and
    /// keeps every started one on its locator.
    ///
    /// Before `Stage::advance`, so a burst started this tick emits this tick
    /// from where its locator is now.
    pub(super) fn advance_absorb_bursts(&mut self) {
        // The craft's own clock, `+0x830 += dt` in `FUN_088418e0`, read
        // against the stamp; past the window there is nothing left to draw.
        for elapsed in &mut self.view.absorb_overlay {
            *elapsed = elapsed
                .map(|seconds| seconds + self.sim.dt)
                .filter(|seconds| *seconds <= oag_fx::hull_overlay::WINDOW);
        }
        for shell in &mut self.view.absorb_shell {
            shell.advance(self.sim.dt);
        }
        if self.view.absorb_bursts.is_empty() {
            return;
        }
        let effect = self.view.effects.get(ABSORB_EFFECT).cloned();
        let dt = self.sim.dt;
        let mut bursts = std::mem::take(&mut self.view.absorb_bursts);
        bursts.retain_mut(|burst| {
            let ship = &self.sim.world.ships[burst.slot];
            let local = self
                .view
                .absorb_anchors
                .get(burst.slot)
                .and_then(|anchors| anchors.get(burst.node));
            let (true, Some(local)) = (ship.active, local) else {
                if let Some(playing) = burst.playing {
                    self.view.stage.detach(playing);
                }
                return false;
            };
            // `FUN_08915cdc`: a positive delay is spent before the system
            // runs at all, one frame's worth at a time.
            if burst.delay > 0.0 {
                burst.delay -= dt;
                return true;
            }
            let at = model_matrix_of(ship).transform_point3(*local);
            match burst.playing {
                // Riding rather than fired: the original parents the system to
                // the locator, so the emitters ride the hull for as long as
                // they run. Only a stage with every slot attached refuses it.
                None => {
                    let Some(effect) = effect.as_ref() else {
                        return false;
                    };
                    burst.playing = self.view.stage.play_riding(effect, at, 1.0);
                    self.view.absorb_started += u32::from(burst.playing.is_some());
                    burst.playing.is_some()
                }
                Some(playing) if self.view.stage.is_emitting(playing) => {
                    self.view.stage.follow(playing, at);
                    true
                }
                // Done emitting: hand the slot back and let the last
                // particles finish where they are.
                Some(playing) => {
                    self.view.stage.detach(playing);
                    false
                }
            }
        });
        self.view.absorb_bursts = bursts;
    }

    /// The hull overlay's pulse on `slot` this tick - see
    /// [`oag_fx::hull_overlay::pulse`] - or `None` when it draws nothing.
    #[must_use]
    pub fn absorb_overlay_pulse(&self, slot: usize) -> Option<f32> {
        self.view
            .absorb_overlay
            .get(slot)
            .copied()
            .flatten()
            .and_then(oag_fx::hull_overlay::pulse)
    }

    /// Whether `slot` is inside its one-second absorb window this tick -
    /// `HullOverlay_AbsorbWindowActive(craft)`, `0.0 <= elapsed <= 1.0` on the
    /// same clock the hull overlay reads.
    ///
    /// Reuses [`Self::absorb_overlay_pulse`]'s own state
    /// (`self.view.absorb_overlay`) rather than a second timer: that `Vec`
    /// already holds `Some(elapsed)` exactly while `0.0 <= elapsed <= WINDOW`
    /// ([`advance_absorb_bursts`](Self::advance_absorb_bursts) filters it),
    /// and `HullOverlay_AbsorbFade`/`HullOverlay_AbsorbWindowActive` were
    /// read as the same window test in a float and a bool shape - see
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`,
    /// "Absorb". `Hud_UpdateEnergyBar` calls the bool form on the player's
    /// own craft every frame; this is that call.
    #[must_use]
    pub fn absorb_window_active(&self, slot: usize) -> bool {
        self.view
            .absorb_overlay
            .get(slot)
            .copied()
            .flatten()
            .is_some()
    }

    /// HD's absorb shell fade on `slot` this tick - the `ShieldColour` its
    /// material reads - or `None` while the original hides the shell. See
    /// [`oag_fx::absorb_shell::AbsorbShell::fader`].
    #[must_use]
    pub fn absorb_shell_fader(&self, slot: usize) -> Option<f32> {
        self.view.absorb_shell.get(slot)?.fader()
    }

    /// How many absorb bursts have ever started, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn absorb_started_for_tests(&self) -> u32 {
        self.view.absorb_started
    }

    /// How many absorb bursts are emitting on the stage right now, for tests.
    #[doc(hidden)]
    #[must_use]
    pub fn absorb_playing_for_tests(&self) -> usize {
        self.view
            .absorb_bursts
            .iter()
            .filter_map(|burst| burst.playing)
            .filter(|playing| self.view.stage.is_emitting(*playing))
            .count()
    }
}
