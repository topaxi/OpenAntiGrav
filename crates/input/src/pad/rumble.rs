//! The `gilrs` motor backend: [`MotorOutput`] for [`Pad`].
//!
//! Each force-feedback pad gets two looping effects, one `Strong` (the large
//! motor) and one `Weak`, started once and then only re-gained
//! (`Effect::set_gain`) when a level changes. `gilrs` runs its effects on a
//! server thread that ticks every 50 ms, so a level lands at 20 Hz at best
//! (`docs/gameplay/rumble.md`, "Device output"). On a platform `gilrs` cannot
//! drive (macOS, the web) no pad reports force feedback and every call is a
//! no-op: the backend has no motors rather than failing.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::{Mutex, PoisonError};

use gilrs::GamepadId;
use gilrs::ff::{BaseEffect, BaseEffectType, Effect, EffectBuilder, Repeat};
use log::{debug, warn};

use super::{Pad, is_real_pad};
use crate::rumble::{MotorOutput, Motors};

/// Every live effect, for [`silence_registered`], which the panic hook calls
/// with no `Gilrs` in reach.
static REGISTRY: Mutex<Vec<Effect>> = Mutex::new(Vec::new());

/// One pad's pair of effects and the levels last sent.
pub(super) struct PadMotors {
    strong: Effect,
    weak: Effect,
    sent: Motors,
    /// The effects run only while a motor is on: a running effect makes the
    /// server re-send it to the device every 50 ms, level or not.
    running: bool,
}

/// The motors of every pad that has been given a level.
#[derive(Default)]
pub(super) struct Effects {
    pads: HashMap<GamepadId, PadMotors>,
}

impl Effects {
    /// A pad went away: its effects go with it.
    pub(super) fn forget(&mut self, id: GamepadId) {
        if let Some(motors) = self.pads.remove(&id) {
            let mut registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
            registry.retain(|e| *e != motors.strong && *e != motors.weak);
        }
    }
}

/// Zeroes every registered effect. Callable from a panic hook.
pub fn silence_registered() {
    let registry = REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    for effect in registry.iter() {
        let _ = effect.set_gain(0.0);
        let _ = effect.stop();
    }
}

/// Chains a panic hook that calls [`silence_registered`] first, so a crash
/// does not leave a motor running.
pub fn silence_on_panic() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        silence_registered();
        previous(info);
    }));
}

fn looping(kind: BaseEffectType, gilrs: &mut gilrs::Gilrs, id: GamepadId) -> Option<Effect> {
    let effect = EffectBuilder::new()
        .add_effect(BaseEffect {
            kind,
            ..Default::default()
        })
        .repeat(Repeat::Infinitely)
        .gamepads(&[id])
        .gain(0.0)
        .finish(gilrs)
        .map_err(|e| warn!("rumble: no effect on pad {id}: {e}"))
        .ok()?;
    Some(effect)
}

/// A pad's pair of effects, registered for the panic hook.
fn open(gilrs: &mut gilrs::Gilrs, id: GamepadId) -> Option<PadMotors> {
    let strong = looping(
        BaseEffectType::Strong {
            magnitude: u16::MAX,
        },
        gilrs,
        id,
    )?;
    let weak = looping(
        BaseEffectType::Weak {
            magnitude: u16::MAX,
        },
        gilrs,
        id,
    )?;
    debug!("rumble: motors opened on pad {id}");
    REGISTRY
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .extend([strong.clone(), weak.clone()]);
    Some(PadMotors {
        strong,
        weak,
        sent: Motors::OFF,
        running: false,
    })
}

impl Pad {
    /// Pads whose backend reports force feedback, with their names.
    #[must_use]
    pub fn motor_pads(&self) -> Vec<String> {
        let Some(gilrs) = &self.gilrs else {
            return Vec::new();
        };
        gilrs
            .gamepads()
            .filter(|(_, pad)| is_real_pad(pad) && pad.is_ff_supported())
            .map(|(_, pad)| pad.name().to_string())
            .collect()
    }

    /// How many force-feedback effects are alive, for a test of the wiring.
    #[must_use]
    pub fn live_effects(&self) -> usize {
        self.rumble.pads.len() * 2
    }
}

impl MotorOutput for Pad {
    fn set(&mut self, seat: usize, motors: Motors) {
        let Some(gilrs) = &mut self.gilrs else {
            return;
        };
        let driven: Vec<GamepadId> = gilrs
            .gamepads()
            .filter(|(id, pad)| {
                pad.is_connected()
                    && is_real_pad(pad)
                    && pad.is_ff_supported()
                    && self.assignment.slot_of(*id) == Some(seat)
            })
            .map(|(id, _)| id)
            .collect();
        for id in driven {
            let pad = match self.rumble.pads.entry(id) {
                Entry::Occupied(pad) => pad.into_mut(),
                Entry::Vacant(slot) => {
                    if motors == Motors::OFF {
                        continue;
                    }
                    let Some(opened) = open(gilrs, id) else {
                        continue;
                    };
                    slot.insert(opened)
                }
            };
            if pad.sent == motors {
                continue;
            }
            let _ = pad.strong.set_gain(f32::from(motors.large) / 255.0);
            let _ = pad.weak.set_gain(f32::from(motors.small) / 255.0);
            if motors == Motors::OFF && pad.running {
                let _ = pad.strong.stop();
                let _ = pad.weak.stop();
                pad.running = false;
            } else if motors != Motors::OFF && !pad.running {
                let _ = pad.strong.play();
                let _ = pad.weak.play();
                pad.running = true;
            }
            pad.sent = motors;
        }
    }

    fn silence(&mut self) {
        for pad in self.rumble.pads.values_mut() {
            let _ = pad.strong.set_gain(0.0);
            let _ = pad.weak.set_gain(0.0);
            let _ = pad.strong.stop();
            let _ = pad.weak.stop();
            pad.running = false;
            pad.sent = Motors::OFF;
        }
    }
}
