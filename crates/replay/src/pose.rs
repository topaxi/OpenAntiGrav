//! [`Pose`] and [`GhostLap`]: what a ghost is drawn from.
//!
//! **A pose per tick, not inputs to resimulate.** ADR-0055 has the argument;
//! the short form is that a ghost has to keep showing the lap that set a
//! personal best after the simulation changes, and a resimulation on a changed
//! build shows a different lap. A pose is the craft's drawn transform - its
//! position and its orientation with any barrel roll folded in - which is
//! everything a hull needs to be placed, and nothing the simulation reads.

use oag_core::math::{Quat, Vec3};

use crate::codec::{Cursor, Truncated, put_varint};

/// Where a craft was and which way it faced, at one tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    /// World position.
    pub position: Vec3,
    /// World orientation, roll included: the rotation the hull is drawn with.
    pub rotation: Quat,
}

impl Pose {
    /// Bytes one pose occupies on disk: seven `f32`s.
    pub const BYTES: usize = 28;

    fn write(&self, out: &mut Vec<u8>) {
        for value in [
            self.position.x,
            self.position.y,
            self.position.z,
            self.rotation.x,
            self.rotation.y,
            self.rotation.z,
            self.rotation.w,
        ] {
            out.extend_from_slice(&value.to_bits().to_le_bytes());
        }
    }

    fn read(cursor: &mut Cursor<'_>) -> Result<Self, Truncated> {
        let mut values = [0.0f32; 7];
        for value in &mut values {
            *value = f32::from_bits(cursor.u32()?);
        }
        Ok(Self {
            position: Vec3::new(values[0], values[1], values[2]),
            rotation: Quat::from_xyzw(values[3], values[4], values[5], values[6]),
        })
    }
}

/// One lap, as a pose per tick of its own lap clock.
///
/// `poses[k]` is where the craft was `k` ticks after it crossed the line to
/// start the lap, so `poses[0]` is the crossing and `poses[lap_ticks]` the
/// crossing that ended it. A ghost is drawn by reading the entry at the
/// *player's* lap clock: the two clocks start on the same edge, so the ghost
/// sits exactly where the recorded craft was at the same point in its lap.
#[derive(Debug, Clone, PartialEq)]
pub struct GhostLap {
    /// The lap's time, in ticks.
    pub lap_ticks: u32,
    /// One pose per tick of the lap, `lap_ticks + 1` of them.
    pub poses: Vec<Pose>,
}

impl GhostLap {
    /// The pose `tick` ticks into the lap, or `None` once the lap is over.
    ///
    /// `None` rather than the last pose held: a ghost that has finished its lap
    /// has crossed the line, and parking it there would put a stationary hull
    /// in the middle of the start straight for however long the player takes
    /// to catch up.
    #[must_use]
    pub fn pose_at(&self, tick: u64) -> Option<Pose> {
        usize::try_from(tick)
            .ok()
            .and_then(|index| self.poses.get(index))
            .copied()
    }

    /// Whether this lap is quicker than `other`, which a missing lap always
    /// loses to.
    #[must_use]
    pub fn beats(&self, other: Option<&GhostLap>) -> bool {
        other.is_none_or(|other| self.lap_ticks < other.lap_ticks)
    }

    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.poses.len() * Pose::BYTES);
        put_varint(&mut out, u64::from(self.lap_ticks));
        put_varint(&mut out, self.poses.len() as u64);
        for pose in &self.poses {
            pose.write(&mut out);
        }
        out
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, Truncated> {
        let mut cursor = Cursor::new(bytes);
        let lap_ticks = u32::try_from(cursor.varint()?).map_err(|_| Truncated(0))?;
        let count = usize::try_from(cursor.varint()?).map_err(|_| Truncated(0))?;
        if count.saturating_mul(Pose::BYTES) != bytes.len() - cursor.position() {
            return Err(Truncated(cursor.position()));
        }
        let poses = (0..count)
            .map(|_| Pose::read(&mut cursor))
            .collect::<Result<_, _>>()?;
        Ok(Self { lap_ticks, poses })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lap(ticks: u32) -> GhostLap {
        GhostLap {
            lap_ticks: ticks,
            poses: (0..=ticks)
                .map(|k| Pose {
                    position: Vec3::new(k as f32, -0.0, 1.5),
                    rotation: Quat::from_rotation_y(k as f32 * 0.01),
                })
                .collect(),
        }
    }

    #[test]
    fn a_lap_round_trips_bit_for_bit() {
        let lap = lap(120);
        assert_eq!(GhostLap::decode(&lap.encode()), Ok(lap));
    }

    #[test]
    fn a_ghost_is_gone_once_its_lap_is_over() {
        let lap = lap(10);
        assert!(lap.pose_at(10).is_some());
        assert!(lap.pose_at(11).is_none());
    }

    #[test]
    fn a_quicker_lap_beats_a_slower_one_and_any_lap_beats_none() {
        assert!(lap(10).beats(Some(&lap(11))));
        assert!(!lap(11).beats(Some(&lap(11))), "a tie keeps the old ghost");
        assert!(lap(99).beats(None));
    }
}
