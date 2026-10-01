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
//! Pulse's circuits only, because only Pulse's `track.vex` payloads carry an aim
//! point: another title, or a circuit with no cameras, keeps the chase camera.

use super::*;
use oag_render::camera::destroy::{self, Destroy, Station};

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
}

impl Default for DestroyCamera {
    fn default() -> Self {
        Self::new(Vec::new(), destroy::FOCUS_RATE_BY_CLASS[0])
    }
}

impl DestroyCamera {
    pub(super) fn new(stations: Vec<Station>, focus_rate: f32) -> Self {
        Self {
            stations,
            focus_rate,
            rng: Rng::new(DESTROY_CAMERA_SEED),
            active: None,
            held: false,
        }
    }
}

/// Every authored `Camera` in a circuit as a [`Station`], or none off Pulse.
pub(super) fn stations(
    title: &oag_title::Title,
    track_blob: &[u8],
    report: &mut Vec<String>,
) -> Vec<Station> {
    if title.name != oag_pulse::TITLE.name {
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
    let index = oag_physics::SpeedClass::from_name(class)
        .and_then(|class| {
            oag_physics::SpeedClass::ALL
                .iter()
                .position(|c| *c == class)
        })
        .unwrap_or(0);
    destroy::FOCUS_RATE_BY_CLASS[index]
}

impl Race {
    /// Takes the camera over when the player's craft is destroyed, moves it
    /// while the wreck is out of the race and lets it go when the craft is
    /// racing again. Once a tick, after the craft's state has settled.
    pub(super) fn advance_destroy_camera(&mut self) {
        let player = self.sim.world.primary_slot();
        let ship = &self.sim.world.ships[player];
        let state = ship.physics.craft_state;
        let subject = ship.physics.body.position;
        let out = matches!(
            state,
            oag_physics::CraftState::Destroyed | oag_physics::CraftState::Eliminated
        );
        let camera = &mut self.view.destroy_camera;
        if !out {
            camera.active = None;
            camera.held = false;
            return;
        }
        match &mut camera.active {
            Some(active) => active.advance(subject, camera.held),
            None => {
                let (low, high) = destroy::START_FOV_SPREAD;
                let offset = low + (high - low) * camera.rng.next_f32();
                camera.active =
                    Destroy::start(&camera.stations, subject, camera.focus_rate, offset);
            }
        }
    }

    /// The player's craft is in state 6: the camera stops following it.
    pub(super) fn hold_destroy_camera(&mut self) {
        self.view.destroy_camera.held = true;
    }

    /// The camera, while it has the picture.
    #[must_use]
    pub(super) fn destroy_camera_now(&self) -> Option<&Destroy> {
        self.view.destroy_camera.active.as_ref()
    }
}
