//! [`PoseTrack`]: every craft on the grid, one pose a tick, kept as the few keys that
//! reproduce it within a stated error. The `POSE` chunk of the file (`file.rs`).
//!
//! Where [`GhostLap`](crate::GhostLap) is one craft's lap at 28 bytes a tick, this is a whole
//! grid's race at about 160 bytes a craft a second (ADR-0087, decision 4;
//! `docs/formats/replay-file.md`, "POSE"):
//!
//! - **Keys.** A key every [`STRIDE`]th tick (10 Hz at 60 Hz ticks), plus the last tick, plus
//!   *refinement*: inside any segment, the tick that interpolates worst becomes a key, until no
//!   tick between two keys is more than [`MAX_POSITION_ERROR`] units or
//!   [`MAX_ROTATION_ERROR`] degrees from the truth. Playback is a lerp for the position and a normalised lerp for the rotation.
//! - **A key** is the position in 1/[`STEPS_PER_UNIT`] unit, delta-coded per axis against the
//!   previous key as zigzag varints, and the rotation as a four-byte smallest-three quaternion.
//! - **Which ticks are keys** is not stored per key: the base keys follow from the track's last
//!   tick, and the interior ones are a five-bit mask per segment.
//!
//! **Refinement measures against the keys as the reader will rebuild them** (quantised), so the
//! bounds hold at every recorded tick, not before quantisation. A segment is refined on its own,
//! which is what lets [`PoseRecorder`] do it as each segment closes: the keys come out the same
//! as an offline pass over the whole race (`tests::streaming_equals_offline`), and a recording
//! holds keys, never a raw pose a tick.
//!
//! This crate knows no race: a pose is whatever the caller says the craft's drawn transform is.

use oag_bytes::Reader;
use oag_core::math::{Quat, Vec3};

use crate::codec::{Truncated, put_varint};
use crate::pose::Pose;

/// Ticks between the keys every track has.
pub const STRIDE: u32 = 6;

/// Position steps in a unit: a key's position is a multiple of 1/64.
pub const STEPS_PER_UNIT: f32 = 64.0;

/// The most a tick between two keys may be from the truth, in world units.
pub const MAX_POSITION_ERROR: f32 = 0.05;

/// The most a tick between two keys may be from the truth, in degrees.
pub const MAX_ROTATION_ERROR: f32 = 0.5;

/// The longest a track grows: one hour at 60 Hz. [`PoseRecorder::push`] takes nothing past it,
/// so a Speed Lap session left running does not grow the capture for ever.
pub const MAX_TICKS: u32 = 216_000;

/// A jump of more than this many units between two consecutive ticks is a teleport (a respawn),
/// not motion: [`PoseTrack::pose`] holds the earlier pose across it rather than drawing the hull
/// streaking between the two. The recorded ticks are exact either way. A craft that is flying
/// moves under 7 units a tick at the fastest class measured; a respawn puts it back on the
/// racing line, tens of units away or more.
pub const TELEPORT_UNITS: f32 = 16.0;

/// One key: the tick, the quantised position and the packed rotation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Key {
    tick: u32,
    position: [i32; 3],
    rotation: u32,
}

impl Key {
    fn of(tick: u32, pose: &Pose) -> Self {
        let scaled = pose.position * STEPS_PER_UNIT;
        Self {
            tick,
            // `as` saturates and sends NaN to zero, which is what a corrupt pose should become.
            position: [
                scaled.x.round() as i32,
                scaled.y.round() as i32,
                scaled.z.round() as i32,
            ],
            rotation: pack_rotation(pose.rotation),
        }
    }

    fn pose(&self) -> Pose {
        Pose {
            position: Vec3::new(
                self.position[0] as f32,
                self.position[1] as f32,
                self.position[2] as f32,
            ) / STEPS_PER_UNIT,
            rotation: unpack_rotation(self.rotation),
        }
    }
}

const ROTATION_SCALE: f32 = 511.0 * std::f32::consts::SQRT_2;

/// A unit quaternion in four bytes: the largest component dropped (and the sign chosen so it is
/// positive), the other three in ten bits each over `[-1/sqrt 2, 1/sqrt 2]`, two bits to say
/// which was dropped. A non-finite quaternion packs as the identity.
fn pack_rotation(q: Quat) -> u32 {
    let q = if q.is_finite() && q.length_squared() > 0.0 {
        q.normalize()
    } else {
        Quat::IDENTITY
    };
    let c = [q.x, q.y, q.z, q.w];
    let largest = (0..4)
        .max_by(|&a, &b| c[a].abs().total_cmp(&c[b].abs()))
        .unwrap_or(3);
    let sign = if c[largest] < 0.0 { -1.0 } else { 1.0 };
    let mut word = largest as u32;
    for (_, v) in c.iter().enumerate().filter(|&(i, _)| i != largest) {
        let n = ((v * sign * ROTATION_SCALE).round() as i32 + 512).clamp(0, 1023) as u32;
        word = (word << 10) | n;
    }
    word
}

fn unpack_rotation(word: u32) -> Quat {
    let largest = (word >> 30) as usize;
    let parts = [
        ((word >> 20) & 1023) as f32,
        ((word >> 10) & 1023) as f32,
        (word & 1023) as f32,
    ]
    .map(|n| (n - 512.0) / ROTATION_SCALE);
    let dropped = (1.0 - parts.iter().map(|p| p * p).sum::<f32>())
        .max(0.0)
        .sqrt();
    let mut c = [0.0; 4];
    let mut next = 0;
    for (i, slot) in c.iter_mut().enumerate() {
        if i == largest {
            *slot = dropped;
        } else {
            *slot = parts[next];
            next += 1;
        }
    }
    Quat::from_xyzw(c[0], c[1], c[2], c[3]).normalize()
}

/// The angle between two orientations, in degrees.
#[must_use]
pub fn degrees_between(a: Quat, b: Quat) -> f32 {
    (2.0 * oag_core::math::acos(a.dot(b).abs().min(1.0))).to_degrees()
}

/// The rotation between `a` and `b` at `fraction`: a normalised lerp, the short way round.
///
/// Not a slerp, which needs the platform's `acos` and `sin`: the keys a recorder picks are then
/// the same on every machine, and the angular speed of the blend is uneven by less than the
/// refinement's bound anywhere it matters (refinement measures this function).
fn blend(a: Quat, b: Quat, fraction: f32) -> Quat {
    let b = if a.dot(b) < 0.0 { -b } else { b };
    (a * (1.0 - fraction) + b * fraction).normalize()
}

/// How far `truth` is from the blend of `a` and `b` at `fraction`, as a multiple of the
/// tolerance: over `1.0` is out of bounds.
fn over(a: &Pose, b: &Pose, fraction: f32, truth: &Pose) -> f32 {
    let position = (a.position.lerp(b.position, fraction) - truth.position).length();
    let rotation = degrees_between(blend(a.rotation, b.rotation, fraction), truth.rotation);
    (position / MAX_POSITION_ERROR).max(rotation / MAX_ROTATION_ERROR)
}

/// The keys a segment needs after its left key.
///
/// `truth[0]` is the pose at `left`'s tick, `truth[n]` the pose at the segment's right end, which
/// is always a key. Ticks are added at the worst interpolation until none is out of bounds;
/// each added key is quantised before it is measured against, so the bound is on what a reader
/// rebuilds. Returns the keys after `left`, in tick order, the right end last.
fn refine(left: &Key, truth: &[Pose]) -> Vec<Key> {
    let span = truth.len() - 1;
    let mut keys = vec![Key::of(left.tick + span as u32, &truth[span])];
    loop {
        let mut added = Vec::new();
        let mut from = (*left, left.pose());
        for key in &keys {
            let to = (*key, key.pose());
            let worst = (from.0.tick + 1..key.tick)
                .map(|tick| {
                    let fraction = (tick - from.0.tick) as f32 / (key.tick - from.0.tick) as f32;
                    let index = (tick - left.tick) as usize;
                    (tick, over(&from.1, &to.1, fraction, &truth[index]))
                })
                .max_by(|x, y| x.1.total_cmp(&y.1));
            if let Some((tick, excess)) = worst
                && excess > 1.0
            {
                added.push(Key::of(tick, &truth[(tick - left.tick) as usize]));
            }
            from = to;
        }
        if added.is_empty() {
            return keys;
        }
        keys.extend(added);
        keys.sort_by_key(|key| key.tick);
    }
}

/// Every craft's pose at every tick of a recording, as keys.
///
/// Tick `0` is the state before the recording's first tick, the same index as the recording's
/// own tick count ([`Replay::ticks`](crate::Replay::ticks)); craft `i` is grid slot `i`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoseTrack {
    last: u32,
    crafts: Vec<Vec<Key>>,
}

impl PoseTrack {
    /// How many craft the track holds, grid slots `0..crafts()`.
    #[must_use]
    pub fn crafts(&self) -> usize {
        self.crafts.len()
    }

    /// The last tick the track holds a pose for.
    #[must_use]
    pub fn last_tick(&self) -> u64 {
        u64::from(self.last)
    }

    /// How many keys `craft` has, or `0` for a craft the track does not hold.
    #[must_use]
    pub fn keys(&self, craft: usize) -> usize {
        self.crafts.get(craft).map_or(0, Vec::len)
    }

    /// Where craft `craft` was at `at` ticks into the recording, or `None` for a craft the
    /// track does not hold.
    ///
    /// `at` is a tick and a fraction, so a frame between two ticks is drawn between their poses.
    /// Outside `0..=last_tick()` the nearest end is held. Across a teleport ([`TELEPORT_UNITS`])
    /// the earlier pose is held up to the tick of the later one.
    #[must_use]
    pub fn pose(&self, craft: usize, at: f64) -> Option<Pose> {
        let keys = self.crafts.get(craft)?;
        let at = at.clamp(0.0, f64::from(self.last));
        let next = keys.partition_point(|key| f64::from(key.tick) <= at);
        let Some(after) = keys.get(next) else {
            return keys.last().map(Key::pose);
        };
        let before = &keys[next - 1];
        let (a, b) = (before.pose(), after.pose());
        let span = after.tick - before.tick;
        if span == 1 && (b.position - a.position).length() > TELEPORT_UNITS {
            return Some(a);
        }
        let fraction = ((at - f64::from(before.tick)) / f64::from(span)) as f32;
        Some(Pose {
            position: a.position.lerp(b.position, fraction),
            rotation: blend(a.rotation, b.rotation, fraction),
        })
    }

    /// The track cut at `ticks`: a key is added there if no key sits on it, and nothing after.
    #[must_use]
    pub fn truncated(&self, ticks: u64) -> Self {
        let last = u32::try_from(ticks).map_or(self.last, |ticks| ticks.min(self.last));
        if last == self.last {
            return self.clone();
        }
        let crafts = (0..self.crafts.len())
            .map(|craft| {
                let mut keys: Vec<Key> = self.crafts[craft]
                    .iter()
                    .copied()
                    .take_while(|key| key.tick <= last)
                    .collect();
                if keys.last().is_none_or(|key| key.tick < last)
                    && let Some(pose) = self.pose(craft, f64::from(last))
                {
                    keys.push(Key::of(last, &pose));
                }
                keys
            })
            .collect();
        Self { last, crafts }
    }

    /// The ticks `craft` has a key at, in order; empty for a craft the track does not hold.
    #[must_use]
    pub fn key_ticks(&self, craft: usize) -> Vec<u64> {
        self.crafts.get(craft).map_or_else(Vec::new, |keys| {
            keys.iter().map(|key| u64::from(key.tick)).collect()
        })
    }

    /// What the track costs in a file: the size of its `POSE` chunk's payload.
    #[must_use]
    pub fn encoded_len(&self) -> usize {
        self.encode().len()
    }

    /// The chunk's payload: the last tick, the craft count, then per craft the byte length of
    /// its keys, its interior-key mask, and its keys.
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_varint(&mut out, u64::from(self.last));
        out.push(u8::try_from(self.crafts.len()).expect("a grid of under 256 craft"));
        for craft in &self.crafts {
            let mut keys = Vec::with_capacity(craft.len() * 8);
            let mut mask = vec![0u8; mask_bytes(self.last)];
            let mut previous = [0i32; 3];
            for key in craft {
                if !key.tick.is_multiple_of(STRIDE) && key.tick != self.last {
                    let bit = (key.tick / STRIDE) * (STRIDE - 1) + (key.tick % STRIDE - 1);
                    mask[(bit / 8) as usize] |= 1 << (bit % 8);
                }
                for (now, before) in key.position.iter().zip(&previous) {
                    put_varint(&mut keys, zigzag(i64::from(*now) - i64::from(*before)));
                }
                previous = key.position;
                keys.extend_from_slice(&key.rotation.to_le_bytes());
            }
            put_varint(&mut out, keys.len() as u64);
            out.extend_from_slice(&mask);
            out.extend_from_slice(&keys);
        }
        out
    }

    /// The inverse of [`Self::encode`].
    ///
    /// # Errors
    ///
    /// [`Truncated`], with the byte it was found at, for a payload that ends early, has bytes
    /// left over, or sets a mask bit for a tick that cannot be an interior key.
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, Truncated> {
        let mut cursor = Reader::le(bytes);
        let last = u32::try_from(cursor.varint()?).map_err(|_| Truncated(0))?;
        if last > MAX_TICKS {
            return Err(Truncated(0));
        }
        let count = usize::from(cursor.u8()?);
        let mut crafts = Vec::with_capacity(count);
        for _ in 0..count {
            let at = cursor.position();
            let key_bytes = usize::try_from(cursor.varint()?).map_err(|_| Truncated(at))?;
            let mask = cursor.take(mask_bytes(last))?;
            let mut keys = Reader::le(cursor.take(key_bytes)?);
            let mut ticks = Vec::new();
            let mut tick = 0;
            while tick <= last {
                ticks.push(tick);
                tick += STRIDE;
            }
            if !last.is_multiple_of(STRIDE) {
                ticks.push(last);
            }
            for bit in 0..mask.len() as u32 * 8 {
                if mask[(bit / 8) as usize] & (1 << (bit % 8)) == 0 {
                    continue;
                }
                let tick = (bit / (STRIDE - 1)) * STRIDE + bit % (STRIDE - 1) + 1;
                if tick >= last {
                    return Err(Truncated(at));
                }
                ticks.push(tick);
            }
            ticks.sort_unstable();
            let mut previous = [0i32; 3];
            let mut list = Vec::with_capacity(ticks.len());
            for tick in ticks {
                for before in &mut previous {
                    let step = unzigzag(keys.varint()?);
                    *before =
                        i32::try_from(i64::from(*before) + step).map_err(|_| Truncated(at))?;
                }
                list.push(Key {
                    tick,
                    position: previous,
                    rotation: keys.u32()?,
                });
            }
            if !keys.is_empty() {
                return Err(Truncated(at));
            }
            crafts.push(list);
        }
        if !cursor.is_empty() {
            return Err(Truncated(cursor.position()));
        }
        Ok(Self { last, crafts })
    }
}

/// Bytes of interior-key mask for a track ending at `last`: five bits for each segment.
fn mask_bytes(last: u32) -> usize {
    (last.div_ceil(STRIDE) * (STRIDE - 1)).div_ceil(8) as usize
}

fn zigzag(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

fn unzigzag(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}

/// Builds a [`PoseTrack`] one tick at a time, refining each segment as it closes.
///
/// Holds a craft's keys and at most [`STRIDE`] raw poses, so a recording's memory is its keys
/// (about 300 bytes a craft a second here, 20 bytes a key), not a pose a tick.
#[derive(Debug, Clone)]
pub struct PoseRecorder {
    crafts: Vec<CraftRun>,
    /// The last tick pushed, once one has been.
    last: Option<u32>,
}

#[derive(Debug, Clone)]
struct CraftRun {
    keys: Vec<Key>,
    /// The truth from the last base key on: `segment[0]` is the pose at that key's tick.
    segment: Vec<Pose>,
}

impl PoseRecorder {
    /// A recorder for `crafts` craft (grid slots `0..crafts`), before tick `0`.
    #[must_use]
    pub fn new(crafts: usize) -> Self {
        Self {
            crafts: vec![
                CraftRun {
                    keys: Vec::new(),
                    segment: Vec::new(),
                };
                crafts.min(usize::from(u8::MAX))
            ],
            last: None,
        }
    }

    /// Takes the next tick's poses, one per craft, the first call being tick `0` (the state
    /// before the recording's first tick). Returns `false`, taking nothing, once the track holds
    /// [`MAX_TICKS`].
    ///
    /// # Panics
    ///
    /// If `poses` is not as long as the recorder has craft.
    pub fn push(&mut self, poses: &[Pose]) -> bool {
        assert_eq!(poses.len(), self.crafts.len(), "one pose per craft");
        let first = self.last.is_none();
        let tick = self.last.map_or(0, |last| last + 1);
        if tick > MAX_TICKS {
            return false;
        }
        for (run, pose) in self.crafts.iter_mut().zip(poses) {
            if first {
                run.keys.push(Key::of(0, pose));
                run.segment.push(*pose);
                continue;
            }
            run.segment.push(*pose);
            if tick.is_multiple_of(STRIDE) {
                run.close();
                run.segment.clear();
                run.segment.push(*pose);
            }
        }
        self.last = Some(tick);
        true
    }

    /// The last tick pushed, or `None` before the first.
    #[must_use]
    pub fn last_tick(&self) -> Option<u64> {
        self.last.map(u64::from)
    }

    /// The track so far, ending at the last tick pushed, or `None` before the first. A copy:
    /// the recorder goes on, and the open segment is closed in the copy alone.
    #[must_use]
    pub fn track(&self) -> Option<PoseTrack> {
        let last = self.last?;
        let crafts = self
            .crafts
            .iter()
            .map(|run| {
                let mut run = run.clone();
                if run.segment.len() > 1 {
                    run.close();
                }
                run.keys
            })
            .collect();
        Some(PoseTrack { last, crafts })
    }
}

impl CraftRun {
    /// Ends the open segment at its last pose: refines it and adds its keys.
    fn close(&mut self) {
        let left = *self.keys.last().expect("a segment opens on a key");
        self.keys.extend(refine(&left, &self.segment));
    }
}

#[cfg(test)]
mod tests;
