//! The camera the player is cut to when their own craft goes out, and the
//! circuit's authored cameras it stands on.
//!
//! # Recovered and measured (Pulse, PSP)
//!
//! `Ship_SetState` case 4 gives the second camera object the wreck as its
//! subject and puts it in mode 5; from then on it is the picture. The camera is
//! one of the circuit's own `Camera` nodes, picked by the nearest *aim point*,
//! and it zooms until the craft fills 35 units. The law, its constants and the
//! numbers it was measured against are in [`oag_render::camera::destroy`].
//!
//! # What this port does
//!
//! The camera takes over on the tick the player's craft becomes
//! `Destroyed` (state 4) and follows the wreck through `Eliminated` (state 5);
//! the focus stops following once the big explosion has gone off (the
//! original's state 6). **Chosen, not measured:** it lets go when the player's
//! craft is racing again, so a respawning mode returns to the chase camera. The
//! original's moving on to another craft after ten seconds, and its re-pick of
//! a station when the craft lies more than 60 units from the current one's
//! aim, are read in `FUN_0887fd3c` and `FUN_08880168` and not ported: a wreck
//! does not move, so neither fires on one.
//!
//! The player's own shakes at the explosions (`Camera_ArmShake(0.3, 0.4)` at
//! state 5 and `(0.8, 0.6)` at state 6) are not ported.
//!
//! # HD, 2048 and Omega
//!
//! The title's [`oag_title::destroyed::DestroyCamera`] says how the same view moves
//! (`docs/ghidra/functions/ps3-hdfury-eu/destroy-camera.md`): substeps and random rate factors on
//! the cut, the hand-back to the chase camera on the camera's own 8 s clock, and on 2048 and
//! Omega the hold view after it. Their stations are the `Camera` nodes of the circuit's
//! `track.vex`, read whether or not the title draws the circuit's effects. A Zone end whose
//! [`oag_title::destroyed::ZoneEnd::orbit_camera`] is set is the orbit round the craft instead,
//! and so is a Detonator end ([`oag_title::destroyed::DetonatorEnd::orbit_camera`]).
//!
//! **Chosen, not measured:**
//!
//! - The 8 s clock starts when the race's camera is built and resets each time it passes 8 s,
//!   as the original's resets in mode 0 (row H1). Its phase at the blast sets how long the
//!   station view lasts, 0 to 8 s; what starts the original's clock is unread (`0x000ab2d8`).
//! - The subject the camera hands to is the player's own wreck: `RaceManager+0xe8`, the list the
//!   original picks another subject from, is unread, and the single-player case leaves the
//!   player's craft as read (row H1).
//! - The eye the hold view freezes is the chase camera's at the hand-back, or at the cut on a
//!   circuit with no `Camera` node. 2048's own circuits author none (a census of `altima`,
//!   `tower` and `sol`), so there the hold view is the whole destroy camera: Omega's fallback
//!   is read (row W8), 2048's pick `0x812199c0` is not, and 2048 takes Omega's.

use super::*;
use oag_render::camera::destroy::{self, Destroy, Ease, Station, orbit::Orbit};

/// The seed for the starting field's draw. Distinct from the other streams.
pub const DESTROY_CAMERA_SEED: u64 = 0x5_9a_2b_04;

/// The destroy camera's own state: the stations, and the camera once it has
/// taken over.
#[derive(Debug, Clone)]
pub(super) struct DestroyCamera {
    stations: Vec<Station>,
    focus_rate: f32,
    rng: Rng,
    active: Option<Destroy>,
    held: bool,
    /// The camera's own free-running clock, seconds (row H1).
    clock: f32,
    /// The 8 s clock has handed the view back for this wreck: it is not cut to again until the
    /// craft has raced.
    released: bool,
    /// The eye the hold view stands at, once it has taken over.
    hold_eye: Option<Vec3>,
    /// HD's Zone end orbit, once it has taken over.
    orbit: Option<Orbit>,
    /// How many times the camera has taken over or let go - see
    /// [`crate::RaceView::camera_cuts`].
    pub(super) cuts: u32,
}

impl Default for DestroyCamera {
    fn default() -> Self {
        Self::new(Vec::new(), destroy::FOCUS_RATE_BY_CLASS[0])
    }
}

impl DestroyCamera {
    /// Forgets the cut, so the next tick the player is out cuts again from where the chase rig is.
    pub(super) fn restart(&mut self) {
        self.active = None;
        self.orbit = None;
        self.hold_eye = None;
        self.released = false;
        self.held = false;
    }

    pub(super) fn new(stations: Vec<Station>, focus_rate: f32) -> Self {
        Self {
            stations,
            focus_rate,
            rng: Rng::new(DESTROY_CAMERA_SEED),
            active: None,
            held: false,
            clock: 0.0,
            released: false,
            hold_eye: None,
            orbit: None,
            cuts: 0,
        }
    }
}

/// Every authored `Camera` in a circuit as a [`Station`], or none off a source whose
/// `circuit_effects` rule is not read (`circuit_fx`, the loader's flag) unless the title's
/// destroy camera reads them anyway ([`oag_title::destroyed::DestroyCamera`]).
pub(super) fn stations(
    circuit_fx: bool,
    law: &oag_title::destroyed::DestroyCamera,
    track_blob: &[u8],
    report: &mut Vec<String>,
) -> Vec<Station> {
    if !circuit_fx && !law.stations_without_circuit_effects {
        return Vec::new();
    }
    let stations: Vec<Station> = oag_vex::camera::cameras(track_blob)
        .iter()
        .map(|camera| Station {
            eye: Vec3::from_array(camera.position()),
            aim: Vec3::from_array(camera.aim),
        })
        .collect();
    report.push(if stations.is_empty() {
        "destroy camera: the circuit authors no Camera node, so the chase camera stays on a wreck"
            .to_string()
    } else {
        format!(
            "destroy camera: {} authored Camera node(s) to cut to when the player's craft is destroyed",
            stations.len()
        )
    });
    stations
}

/// The camera's focus rate for a class name: Venom `0.4`, Flash `0.5`, Rapier
/// and Phantom `0.6` (`FUN_0887f9bc`; only Venom's was seen running).
pub(super) fn focus_rate(class: &str) -> f32 {
    let index = oag_tables::handling::SpeedClass::census(class).map_or(0, |class| class.index());
    destroy::FOCUS_RATE_BY_CLASS[index]
}

#[cfg(feature = "view")]
impl RaceView {
    /// The camera, while it has the picture.
    #[must_use]
    pub(super) fn destroy_camera_now(&self) -> Option<&Destroy> {
        self.destroy_camera.active.as_ref()
    }
}

#[cfg(feature = "view")]
impl RaceView {
    /// Whether the close orbit round the player's wreck has the picture.
    #[must_use]
    pub fn orbiting(&self) -> bool {
        self.destroy_camera.orbit.is_some()
    }

    /// The player's craft is in state 6: the camera stops following it.
    pub(super) fn hold_destroy_camera(&mut self) {
        self.destroy_camera.held = true;
    }
}

#[cfg(feature = "view")]
impl RaceView {
    /// Takes the camera over when the player's craft is destroyed, moves it
    /// while the wreck is out of the race and lets it go when the craft is
    /// racing again. Once a tick, after the craft's state has settled.
    pub(super) fn advance_destroy_camera(&mut self, core: &SimRace) {
        let player = core.player_slot();
        let ship = &core.sim.world.ships[player];
        let state = ship.physics.craft_state;
        let subject = ship.physics.body.position;
        let dt = core.sim.dt;
        let law = core.sim.destroyed.camera;
        let orbits = core.sim.zone_end().is_some_and(|end| end.orbit_camera)
            || core.sim.detonator_end().is_some_and(|end| end.orbit_camera);
        let out = matches!(
            state,
            oag_physics::CraftState::Destroyed | oag_physics::CraftState::Eliminated
        );
        let mut cuts = 0;
        let camera = &mut self.destroy_camera;
        camera.clock += dt;
        let handback_due = law
            .hands_back_after_secs
            .is_some_and(|secs| camera.clock > secs);
        if handback_due {
            camera.clock = 0.0;
        }
        if !out {
            if camera.active.take().is_some() {
                cuts += 1;
            }
            camera.orbit = None;
            camera.hold_eye = None;
            camera.released = false;
            camera.held = false;
            camera.cuts = camera.cuts.wrapping_add(cuts);
            return;
        }
        if orbits {
            let orbit = match &mut camera.orbit {
                Some(orbit) => {
                    orbit.advance(dt, subject);
                    *orbit
                }
                None => {
                    cuts += 1;
                    *camera.orbit.insert(Orbit::start(subject))
                }
            };
            camera.active = Some(orbit.pose(ship.physics.body.orientation));
            camera.cuts = camera.cuts.wrapping_add(cuts);
            return;
        }
        if handback_due && !camera.released && camera.active.is_some() {
            camera.released = true;
            camera.active = None;
            cuts += 1;
            if law.holds_the_wreck {
                let eye = self.tick_view_unshaken(core).inverse().w_axis.truncate();
                self.destroy_camera.hold_eye = Some(eye);
            }
        }
        let camera = &mut self.destroy_camera;
        if let Some(eye) = camera.hold_eye {
            camera.active = Some(Destroy::hold(
                eye,
                subject,
                ship.physics.body.linear_velocity,
            ));
        } else if !camera.released {
            match &mut camera.active {
                Some(active) => active.advance(subject, camera.held, dt),
                None => {
                    let (low, high) = destroy::START_FOV_SPREAD;
                    let offset = low + (high - low) * camera.rng.next_f32();
                    let ease = camera.ease(&law);
                    camera.active = Destroy::start(&camera.stations, subject, ease, offset);
                    if camera.active.is_some() {
                        cuts += 1;
                    } else if law.holds_the_wreck {
                        // No station to stand on: the pick gives mode 7 and Omega goes to the
                        // hold mode 8 (`0x013832f0`) from there (row W8).
                        camera.released = true;
                        cuts += 1;
                        let chase = self.tick_view_unshaken(core).inverse().w_axis.truncate();
                        self.destroy_camera.hold_eye = Some(chase);
                        self.destroy_camera.active = Some(Destroy::hold(
                            chase,
                            subject,
                            ship.physics.body.linear_velocity,
                        ));
                    }
                }
            }
        }
        self.destroy_camera.cuts = self.destroy_camera.cuts.wrapping_add(cuts);
    }
}

impl DestroyCamera {
    /// The rates a cut is made with: the class's focus rate and the field's, each scaled by the
    /// title's random factor where it has one (row D4).
    #[cfg(feature = "view")]
    fn ease(&mut self, law: &oag_title::destroyed::DestroyCamera) -> Ease {
        let mut ease = Ease::per_frame(self.focus_rate);
        ease.substeps_per_second = law.substeps_per_second;
        if let Some(factors) = law.rate_factors {
            let mut draw = |(low, high): (f32, f32)| low + (high - low) * self.rng.next_f32();
            ease.fov_rate *= draw(factors.field);
            ease.focus_rate *= draw(factors.focus);
        }
        ease
    }
}
