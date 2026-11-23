//! What a destroyed craft sounds like on a title whose
//! [`oag_title::destroyed::DestroyedSound`] is `Ship` (HD/Fury): the blast, the local
//! player's looped `~SHIP_DESTROYED` and the burning craft's `~SHIP_ONFIRE`.
//!
//! `Ship_SetState`'s case 4 plays the blast and opens both loops
//! (`docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, row 3); state 6 stops
//! `~SHIP_DESTROYED` (row 12) and state 8 stops both (row 15). Cues are an output, so what
//! is kept here is the last tick's answer for each craft, outside the state hash.

use super::*;
use oag_gameplay::cue::{Cue, CueEvent};
use oag_title::destroyed::DestroyedSound;

/// What a craft's destruction had started, so a tick raises only the edges.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Heard {
    /// The blast was raised for this wreck.
    blast: bool,
    /// `~SHIP_DESTROYED` is open (the local player's).
    destroyed: bool,
    /// `~SHIP_ONFIRE` is open.
    on_fire: bool,
}

impl RaceSim {
    /// Raises the destroyed craft's cues off the states this tick settled, once a tick.
    ///
    /// The Zone and Detonator ends (`Ship_SetState`'s modes 6 and 14) play `shipdestroy`
    /// instead and none of these (`zone-destroy.md`, row Z5). `~SHIP_ONFIRE` is for an
    /// ordinary race alone: not the Eliminator, whose craft returns by state 8.
    pub(super) fn raise_destroyed_cues(&mut self) {
        if let Some(end) = self.zone_end() {
            self.raise_zone_end_cue(end);
            return;
        }
        if self.destroyed.sound != DestroyedSound::Ship {
            return;
        }
        let mode = self.world.mode();
        if matches!(mode, Mode::Zone | Mode::Detonator) {
            return;
        }
        let eliminator = mode == Mode::Eliminator;
        for slot in 0..self.world.ship_count as usize {
            let ship = &self.world.ships[slot];
            let down = ship.active && ship.physics.craft_state != oag_physics::CraftState::Racing;
            let past_state_6 = oag_physics::damage::in_state_6(&ship.physics, eliminator);
            let local = self.world.person_flown(slot);
            let heard = self.destroyed_heard[slot];
            let now = Heard {
                blast: down,
                destroyed: down && local && !past_state_6,
                on_fire: down && !eliminator,
            };
            self.destroyed_heard[slot] = now;
            if now.blast && !heard.blast {
                let blast = if local {
                    Cue::ShipExplosionPlayer
                } else {
                    Cue::ShipExplosion
                };
                self.cues.push(CueEvent::new(blast, slot));
            }
            for (was, is, start, stop) in [
                (
                    heard.destroyed,
                    now.destroyed,
                    Cue::ShipDestroyed,
                    Cue::ShipDestroyedStop,
                ),
                (
                    heard.on_fire,
                    now.on_fire,
                    Cue::ShipOnFire,
                    Cue::ShipOnFireStop,
                ),
            ] {
                match (was, is) {
                    (false, true) => self.cues.push(CueEvent::new(start, slot)),
                    (true, false) => self.cues.push(CueEvent::new(stop, slot)),
                    _ => {}
                }
            }
        }
    }

    /// A Zone craft's end: `shipdestroy` once, on the blast (`zone-destroy.md`, row Z5). Where
    /// the title's event is not resolved the end is silent (`ZoneEnd::cue`).
    fn raise_zone_end_cue(&mut self, end: oag_title::destroyed::ZoneEnd) {
        let slot = self.world.primary_slot();
        let down = self.world.ships[slot].physics.craft_state != oag_physics::CraftState::Racing;
        let heard = std::mem::replace(
            &mut self.destroyed_heard[slot],
            Heard {
                blast: down,
                ..Heard::default()
            },
        );
        if down && !heard.blast && end.cue.is_some() {
            self.cues.push(CueEvent::new(Cue::ShipDestroy, slot));
        }
    }
}
