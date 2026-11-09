//! The wall-contact and landing selectors (`docs/gameplay/rumble.md`,
//! "Contact and landing"): which of seven held collision patterns and four
//! landing one-shots a craft's contact measure and touchdown speed call for,
//! and the timers that decide when one is replaced or stops.
//!
//! The thresholds and times are the original's (one function, same constants
//! in HD and Omega). What feeds them is ours and **chosen, not measured**: the
//! original's contact measure and the sign and unit of its landing speed are
//! unread, so the caller passes the wall response's impulse and the downward
//! speed at touchdown.

use oag_tables::rumble::Library;

use super::{Handle, Player};

/// Wall patterns, by band then side: heavy, then right and left of
/// medium, light, very light.
pub const WALL: [&str; 7] = [
    "collision_heavy_looping",
    "collision_right_med_looping",
    "collision_right_light_looping",
    "collision_right_very_light_looping",
    "collision_left_med_looping",
    "collision_left_light_looping",
    "collision_left_very_light_looping",
];

/// Landing one-shots, heaviest first.
pub const LANDING: [&str; 4] = [
    "landing_heavy",
    "landing_med",
    "landing_light",
    "landing_very_light",
];

const NO_LANDING: usize = 4;
/// Seconds a landing stays armed against a weaker one.
const LANDING_REARM: f32 = 1.0;
/// Seconds without contact before a new contact replaces the voice.
const NEW_CONTACT_GAP: f32 = 0.3;
/// Seconds of continuous contact before the voice stops.
const CONTACT_LIMIT: f32 = 1.5;
/// Seconds at zero contact before the voice stops.
const RELEASE_AFTER: f32 = 0.1;
/// Seconds at zero contact before the contact timer clears.
const CLEAR_AFTER: f32 = 0.4;

/// The wall pattern (index into [`WALL`]) a contact of size `m` on `side`
/// calls for; `None` at 2 or less.
#[must_use]
pub fn wall_band(m: f32, side: f32) -> Option<usize> {
    let m = m.abs();
    let level = if m > 55.0 {
        return Some(0);
    } else if m >= 30.0 {
        1
    } else if m >= 10.0 {
        2
    } else if m > 2.0 {
        3
    } else {
        return None;
    };
    Some(if side < 0.0 { level + 3 } else { level })
}

/// The landing pattern (index into [`LANDING`]) a touchdown at `v` calls for,
/// or `LANDING.len()` for none.
#[must_use]
pub fn landing_band(v: f32) -> usize {
    if v > 6.0 {
        0
    } else if v >= 4.75 {
        1
    } else if v >= 3.5 {
        2
    } else if v > 3.0 {
        3
    } else {
        NO_LANDING
    }
}

/// One craft's selector state.
#[derive(Debug, Clone)]
pub struct Selector {
    wall: [Option<usize>; 7],
    landing: [Option<usize>; 4],
    voice: Option<(Handle, usize)>,
    gap: f32,
    contact_time: f32,
    spent: bool,
    armed: usize,
    armed_for: f32,
}

impl Selector {
    /// A selector over `library`'s patterns.
    #[must_use]
    pub fn new(library: &Library) -> Self {
        Self {
            wall: WALL.map(|n| library.index_of(n)),
            landing: LANDING.map(|n| library.index_of(n)),
            voice: None,
            gap: NEW_CONTACT_GAP + 1.0,
            contact_time: 0.0,
            spent: false,
            armed: NO_LANDING,
            armed_for: 0.0,
        }
    }

    /// One update with the wall contact `(size, side)`, or `None`. Returns the
    /// pattern index newly held, if any.
    pub fn contact(
        &mut self,
        player: &mut Player,
        reading: Option<(f32, f32)>,
        dt: f32,
    ) -> Option<usize> {
        let band = reading.and_then(|(m, side)| wall_band(m, side));
        let Some(band) = band else {
            self.gap += dt;
            if self.gap >= RELEASE_AFTER {
                self.stop_voice(player);
            }
            if self.gap >= CLEAR_AFTER {
                self.contact_time = 0.0;
                self.spent = false;
            }
            return None;
        };
        let new_contact = self.gap > NEW_CONTACT_GAP;
        self.gap = 0.0;
        self.contact_time += dt;
        if self.contact_time > CONTACT_LIMIT {
            self.stop_voice(player);
            self.spent = true;
            return None;
        }
        let changed = self.voice.is_none_or(|(_, held)| held != band);
        if self.spent || !(changed || new_contact) {
            return None;
        }
        self.stop_voice(player);
        let id = self.wall[band]?;
        self.voice = player.hold(id).map(|h| (h, band));
        self.voice.map(|_| band)
    }

    fn stop_voice(&mut self, player: &mut Player) {
        if let Some((handle, _)) = self.voice.take() {
            player.stop(handle);
        }
    }

    /// One update with the touchdown speed `v` (0 when not touching down).
    /// Returns the landing index newly played, if any.
    pub fn landing(&mut self, player: &mut Player, v: f32, dt: f32) -> Option<usize> {
        if self.armed < NO_LANDING {
            self.armed_for += dt;
            if self.armed_for >= LANDING_REARM {
                self.armed = NO_LANDING;
            }
        }
        let band = landing_band(v);
        if band >= self.armed {
            return None;
        }
        // A stronger landing "finishes" the voice before it; a landing is a
        // one-shot, whose release flag is set already, so that is a no-op.
        self.armed = band;
        self.armed_for = 0.0;
        let id = self.landing[band]?;
        player.one_shot(id, None).then_some(band)
    }

    /// Forgets every voice.
    pub fn clear(&mut self) {
        self.voice = None;
        self.gap = NEW_CONTACT_GAP + 1.0;
        self.contact_time = 0.0;
        self.spent = false;
        self.armed = NO_LANDING;
    }
}

#[cfg(test)]
mod tests;
