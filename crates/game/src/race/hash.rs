//! [`Race::state_hash`]: the race-level half of the determinism hash, over the
//! state that lives outside the world snapshot.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/hash.rs`.

use super::*;

impl Race {
    /// One 64-bit fingerprint of everything this race carries from tick to tick.
    ///
    /// [`oag_gameplay::hash::hash_world`] plus the pad state, which lives here
    /// rather than in the world because a pad belongs to the track and not to a
    /// craft - `docs/gameplay/pickups.md` states that and this does not overturn
    /// it. The two are folded into one hasher rather than one hashing the
    /// other's output, so the result is a single stream.
    ///
    /// # Why the pad timers have to be in it
    ///
    /// This is the field the known-gap section of `docs/gameplay/pickups.md`
    /// uses as its worked example: **a pad refresh timer one tick out shifts
    /// every subsequent draw**, because a pad that is ready a tick earlier grants
    /// a pickup a tick earlier and moves the generator with it. Nothing about
    /// that shows in a ship's dynamics until the pickup itself differs, by which
    /// point the cause is thousands of ticks back.
    ///
    /// **Render-only state is not here** - the exhaust, the sparks, the boost
    /// kick, the airbrake flaps and the blast flashes all have their own
    /// generators and none of them may reach the simulation. See
    /// [`Self::boost_kick`].
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let mut hasher = oag_core::hash::StateHasher::new();
        oag_gameplay::hash::write_world(&mut hasher, &self.world);

        for left in &self.weapon_pad_refresh_left {
            hasher.write_f32(*left);
        }
        // The broadphase caches, which are simulation state and not a cache in
        // the "can be recomputed" sense: a stale entry changes which tick a pad
        // is next measured on, and therefore which tick it triggers.
        // Every racer's row, not only the player's, and every slot of every row
        // whether or not a craft occupies it - the same argument the world hash
        // makes for its inactive ship slots.
        for row in &self.weapon_pad_distance {
            for distance in row {
                hasher.write_f32(*distance);
            }
        }
        for row in &self.pad_distance {
            for distance in row {
                hasher.write_f32(*distance);
            }
        }
        for current in self.weapon_pad_current {
            write_pad_index(&mut hasher, current);
        }
        for current in self.pad_current {
            write_pad_index(&mut hasher, current);
        }
        for previous in self.pad_previous_position {
            match previous {
                None => hasher.write_u8(0),
                Some(position) => {
                    hasher.write_u8(1);
                    hasher.write_vec3(position);
                }
            }
        }
        // Every slot, because every craft recovers now. Fixed length, so the
        // stream's shape does not depend on how many craft are on the grid.
        for cooldown in self.respawn_cooldown {
            hasher.write_u32(cooldown);
        }
        for run in self.respawns_in_a_row {
            hasher.write_u32(run);
        }
        for dwell in self.lost_ticks {
            hasher.write_u32(dwell);
        }
        // The second dwell, on the same terms: a stall counter one tick out moves
        // which tick a craft is put back on, and a respawn moves everything after
        // it.
        for dwell in self.stalled_ticks {
            hasher.write_u32(dwell);
        }

        hasher.finish()
    }
}

/// A pad index for [`Race::state_hash`], with a discriminant byte.
///
/// Or "not on a pad" hashes the same as "on pad zero", which is precisely the
/// edge the trigger is built on.
pub(super) fn write_pad_index(hasher: &mut oag_core::hash::StateHasher, index: Option<usize>) {
    match index {
        None => hasher.write_u8(0),
        Some(index) => {
            hasher.write_u8(1);
            hasher.write_u32(index as u32);
        }
    }
}
