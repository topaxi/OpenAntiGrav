//! Reconciling with the server, and what the picture does about the jump.
//!
//! [`Race::reconcile`] is the client's correction: put the server's state back and
//! replay the ticks since, through [`Race::tick_sim_only`]. When the client guessed a
//! remote craft's input wrong, the craft ends the replay somewhere else, and drawn
//! as it is that is a pop of however far apart the two timelines had got.
//!
//! **The smoothing is a view-only offset that decays.** At the correction each
//! craft's drawn position is kept where it was by an offset (`before - after`), and
//! the offset shrinks by a fixed fraction every drawn tick, so the craft glides onto
//! the corrected timeline instead of jumping. The offset is added where a craft is
//! drawn ([`Race::drawn_model`], [`Race::drawn_position`]) and nowhere the
//! simulation reads, so the state hash is the same with it on or off
//! (`tests::correction`). It leaves the orientation alone: a position pop is the one
//! a player sees as a rival teleporting, and a yaw error of the size a few ticks of
//! steering make is not.
//!
//! # Chosen, not measured
//!
//! Nothing here is read off an original; the originals have no rollback. The two
//! numbers are judged by eye in `docs/networking/README.md` ("What a correction looks
//! like"):
//!
//! - [`HALF_LIFE_TICKS`]: how long the glide takes.
//! - [`SNAP_DISTANCE`]: past it a correction is drawn where it lands, because a craft
//!   that moved that far was respawned or hit hard, and sliding it across the track
//!   would draw a ghost.

use super::*;

/// Ticks for the offset to halve: 6 ticks, 100 ms at 60 Hz. **Chosen, not measured.**
pub const HALF_LIFE_TICKS: f32 = 6.0;

/// A correction larger than this many world units is drawn where it lands. Chosen,
/// not measured: about a craft and a half in length (13 units).
pub const SNAP_DISTANCE: f32 = 20.0;

/// What one [`Race::reconcile`] did to the picture's craft.
#[derive(Debug, Clone, Copy)]
pub struct Reconciled {
    /// Ticks replayed.
    pub ticks: usize,
    /// How far each slot's craft moved because of the correction, in world units.
    pub pops: [f32; MAX_SHIPS],
}

impl Race {
    /// Puts `authoritative` back and replays `inputs` over it, one tick each, without
    /// the picture or the sound ([`Self::tick_sim_only`]).
    ///
    /// With smoothing on ([`Self::set_correction_smoothing`], the default) each
    /// craft's drawn position is held where it was and glides to the corrected one
    /// over the next ticks; off, the craft is drawn where the replay left it.
    pub fn reconcile(
        &mut self,
        authoritative: &RaceSim,
        inputs: impl IntoIterator<Item = oag_gameplay::PlayerInputs>,
    ) -> Reconciled {
        self.reconcile_observed(authoritative, inputs, |_| {})
    }

    /// [`Self::reconcile`], showing `each` the simulation after every replayed
    /// tick: a network client keeps what it predicted at each tick, to compare with
    /// the server's state when that tick is confirmed.
    pub fn reconcile_observed(
        &mut self,
        authoritative: &RaceSim,
        inputs: impl IntoIterator<Item = oag_gameplay::PlayerInputs>,
        mut each: impl FnMut(&RaceSim),
    ) -> Reconciled {
        let before: [Vec3; MAX_SHIPS] =
            std::array::from_fn(|slot| self.sim.world.ships[slot].physics.body.position);
        self.restore_sim(authoritative);
        let mut ticks = 0;
        for tick in inputs {
            self.tick_sim_only(&tick);
            each(&self.sim);
            ticks += 1;
        }
        #[cfg(feature = "view")]
        self.settle_presentation();
        let mut pops = [0.0; MAX_SHIPS];
        for (slot, pop) in pops.iter_mut().enumerate() {
            let moved = before[slot] - self.sim.world.ships[slot].physics.body.position;
            *pop = moved.length();
            // **Only a remote person's craft glides.** The craft the camera follows
            // is drawn where the camera puts it: offsetting the hull alone would
            // draw it off its own chase camera for the length of the glide.
            // In a network race every person is remote on every peer; this
            // machine's own seat is the one the camera follows.
            let remote = self.sim.world.controllers[slot] == oag_gameplay::Controller::Remote
                && self.view.seat != Some(slot);
            if self.view.correction_smoothing && remote && *pop <= SNAP_DISTANCE {
                self.view.correction[slot] += moved;
            } else {
                self.view.correction[slot] = Vec3::ZERO;
            }
        }
        Reconciled { ticks, pops }
    }

    /// Whether a correction glides ([`HALF_LIFE_TICKS`]) or is drawn where it lands.
    /// Turning it off drops any glide in progress.
    pub fn set_correction_smoothing(&mut self, on: bool) {
        self.view.correction_smoothing = on;
        if !on {
            self.view.correction = [Vec3::ZERO; MAX_SHIPS];
        }
    }

    /// How far `slot`'s craft is drawn from where the simulation has it, in world
    /// units, while a correction glides out.
    #[must_use]
    pub fn correction_offset_of(&self, slot: usize) -> Vec3 {
        self.view.correction[slot]
    }

    /// One drawn tick of the glide: every offset loses its share. Not run by a
    /// replay, which is not drawn.
    pub(super) fn decay_corrections(&mut self) {
        let keep = 0.5f32.powf(1.0 / HALF_LIFE_TICKS);
        for offset in &mut self.view.correction {
            *offset *= keep;
            if offset.length_squared() < 1.0e-6 {
                *offset = Vec3::ZERO;
            }
        }
    }

    /// `slot`'s craft model matrix as drawn at this tick: the simulation's, moved by
    /// the correction glide.
    pub(super) fn drawn_model(&self, slot: usize) -> Mat4 {
        let model = model_matrix_of(&self.sim.world.ships[slot]);
        match self.view.correction[slot] {
            offset if offset == Vec3::ZERO => model,
            offset => Mat4::from_translation(offset) * model,
        }
    }

    /// `slot`'s body position as drawn at this tick.
    pub(super) fn drawn_position(&self, slot: usize) -> Vec3 {
        self.sim.world.ships[slot].physics.body.position + self.view.correction[slot]
    }
}

/// A scripted misprediction for a capture: at tick `at` craft `slot` is handed to a
/// "remote person" who double-taps the airbrake toward the player (a sideshift, a
/// lateral dash), the client keeps guessing it straight on for `window` ticks, and
/// then the server's state arrives and the client corrects. With `contact` the player
/// dashes at it too. A verification aid,
/// there because no capture has two timelines: see `--force-remote-correction`.
///
/// A sideshift because it is the input a last-input guess cannot see coming (a
/// pressed edge, two within a quarter of a second) and the one that moves a craft
/// sideways into its neighbour.
///
/// The server's side is one `Race::tick_sim_only` run ahead of the client's and
/// rewound (`snapshot_sim`/`restore_sim`), so a single [`Race`] is both, the camera
/// and the particles tick once per drawn tick, and a frame at any tick is a function
/// of the tick count alone.
#[derive(Debug, Clone)]
pub struct Scene {
    /// The tick the remote person takes the craft.
    pub at: u32,
    /// Which craft.
    pub slot: usize,
    /// Ticks the guess runs before the correction arrives.
    pub window: u32,
    /// Whether the correction glides ([`Race::set_correction_smoothing`]).
    pub smooth: bool,
    /// Whether the player dashes toward the craft too, so the two touch on the true
    /// timeline and not on the guessed one.
    pub contact: bool,
    /// A weapon the remote person holds and fires on the second tick after `at` instead of
    /// the sideshift, a fire the client's guess carries no edge for: the correction's replay
    /// is where the client first meets it ([`Race::settle_presentation`]).
    pub fire: Option<oag_tables::weapons::Weapon>,
    /// The server's state after tick `at`, and which way the rival dashes (`+1.0`
    /// right), once `at` has come.
    server: std::cell::RefCell<Option<(RaceSim, f32)>>,
}

impl Scene {
    /// A scene on `slot` from tick `at` with `window` ticks of latency.
    #[must_use]
    pub fn new(at: u32, slot: usize, window: u32, smooth: bool, contact: bool) -> Self {
        Self {
            at,
            slot,
            window,
            smooth,
            contact,
            fire: None,
            server: std::cell::RefCell::default(),
        }
    }

    /// The remote person fires `weapon` on the second tick after `at`, and does not sideshift.
    #[must_use]
    pub fn with_fire(mut self, weapon: oag_tables::weapons::Weapon) -> Self {
        self.fire = Some(weapon);
        self
    }

    /// The tick after `at` the remote person presses fire on.
    const FIRE_AT: u32 = 2;

    /// Which way craft `b` lies from craft `a`: `+1.0` to its right.
    fn side(race: &Race, a: usize, b: usize) -> f32 {
        let ships = &race.sim.world.ships;
        let toward = ships[b].physics.body.position - ships[a].physics.body.position;
        if toward.dot(ships[a].physics.body.right()) >= 0.0 {
            1.0
        } else {
            -1.0
        }
    }

    /// What the remote person holds `k` ticks after `at`: Cross throughout, and the
    /// airbrake on `side` pressed at 0 and 2, so the second press lands inside the
    /// tap window. `None` for `k` before `at`.
    fn held(k: Option<u32>, side: f32, fires: bool) -> u32 {
        use oag_gameplay::input::Button;
        let brake = if side > 0.0 { Button::R } else { Button::L }.bit();
        let extra = match (fires, k) {
            (true, Some(Self::FIRE_AT)) => Button::Square.bit(),
            (true, _) => 0,
            (false, Some(0 | 2)) => brake,
            (false, _) => 0,
        };
        Button::Cross.bit() | extra
    }

    /// The remote person's true input on the tick `k` after `at`.
    fn truth(k: u32, side: f32, fires: bool) -> oag_gameplay::InputSnapshot {
        let mut buttons = oag_gameplay::input::Input::new();
        buttons.begin_frame(Self::held(k.checked_sub(1), side, fires));
        buttons.begin_frame(Self::held(Some(k), side, fires));
        let pressed =
            !fires && Self::held(Some(k), side, false) != oag_gameplay::input::Button::Cross.bit();
        oag_gameplay::InputSnapshot {
            buttons,
            airbrake_left: if pressed && side < 0.0 { 1.0 } else { 0.0 },
            airbrake_right: if pressed && side > 0.0 { 1.0 } else { 0.0 },
            ..oag_gameplay::InputSnapshot::default()
        }
    }

    /// The client's guess: Cross held and nothing else, no edge.
    fn guess() -> oag_gameplay::InputSnapshot {
        let mut buttons = oag_gameplay::input::Input::new();
        let held = oag_gameplay::input::Button::Cross.bit();
        buttons.begin_frame(held);
        buttons.begin_frame(held);
        oag_gameplay::InputSnapshot {
            buttons,
            ..oag_gameplay::InputSnapshot::default()
        }
    }

    /// Both craft's inputs on the tick `k` after `at`: the player's own dash toward the
    /// rival (the other way round from the rival's, known on both timelines) and the
    /// rival's `remote`.
    fn inputs(
        &self,
        k: u32,
        side: f32,
        remote: oag_gameplay::InputSnapshot,
    ) -> oag_gameplay::PlayerInputs {
        let own = if self.contact {
            Self::truth(k, -side, false)
        } else {
            Self::guess()
        };
        let mut inputs = oag_gameplay::PlayerInputs::single(own);
        inputs.set(self.slot, remote);
        inputs
    }

    /// One tick of the capture. `player` is slot 0's input as the capture builds it.
    pub fn tick(&self, race: &mut Race, player: &oag_gameplay::InputSnapshot, tick: u32) {
        if tick < self.at {
            race.tick(&oag_gameplay::PlayerInputs::single(*player));
            return;
        }
        let k = tick - self.at;
        if k == 0 {
            race.set_correction_smoothing(self.smooth);
            race.sim.world.controllers[self.slot] = oag_gameplay::Controller::Remote;
            if let Some(weapon) = self.fire {
                race.sim.world.ships[self.slot].pickup.weapon = Some(weapon);
            }
            let base = race.snapshot_sim();
            let side = Self::side(race, self.slot, 0);
            race.tick_sim_only(&self.inputs(0, side, Self::truth(0, side, self.fire.is_some())));
            *self.server.borrow_mut() = Some((race.snapshot_sim(), side));
            race.restore_sim(&base);
        }
        let Some(side) = self.server.borrow().as_ref().map(|(_, side)| *side) else {
            race.tick(&oag_gameplay::PlayerInputs::single(*player));
            return;
        };
        let remote = if k <= self.window {
            Self::guess()
        } else {
            Self::truth(k, side, self.fire.is_some())
        };
        race.tick(&self.inputs(k, side, remote));
        if k == self.window
            && let Some((authoritative, _)) = self.server.borrow().as_ref()
        {
            let replay: Vec<_> = (1..=self.window)
                .map(|j| self.inputs(j, side, Self::truth(j, side, self.fire.is_some())))
                .collect();
            let reconciled = race.reconcile(authoritative, replay);
            println!(
                "remote correction at tick {tick}: slot {} moved {:.2} units (own craft {:.2}), \
                 smoothing {}",
                self.slot,
                reconciled.pops[self.slot],
                reconciled.pops[0],
                if self.smooth { "on" } else { "off" }
            );
            #[cfg(feature = "view")]
            if self.fire.is_some() {
                println!(
                    "late presentation at tick {tick}: {:?}",
                    race.take_late_report()
                );
            }
        }
    }
}
