//! Unit tests for [`super::anim_transform`], over payloads built here.
//!
//! The real payloads are exercised by
//! `crates/render/tests/scenery_animation_ground_truth.rs` (needs a disc image);
//! these pin the arithmetic: field offsets, the three quantisations, the
//! hold-and-blend rule and the composition order.

use super::*;
use oag_formats::ByteOrder;

/// Builds a payload in the layout the module doc states.
struct Builder {
    header: [u8; 0x50],
    body: Vec<u8>,
    /// Which way round to write every field, so one test builds the payload as
    /// Pulse and as Wipeout HD ship it.
    order: ByteOrder,
}

impl Builder {
    fn new() -> Self {
        Self::with_order(ByteOrder::Little)
    }

    fn with_order(order: ByteOrder) -> Self {
        let mut out = Self {
            header: [0u8; 0x50],
            body: Vec::new(),
            order,
        };
        let rate = (1.0f32 / 60.0).to_bits();
        out.header[0x3c..0x40].copy_from_slice(&Self::bytes32(order, rate));
        out
    }

    fn bytes16(order: ByteOrder, v: u16) -> [u8; 2] {
        match order {
            ByteOrder::Little => v.to_le_bytes(),
            ByteOrder::Big => v.to_be_bytes(),
        }
    }

    fn bytes32(order: ByteOrder, v: u32) -> [u8; 4] {
        match order {
            ByteOrder::Little => v.to_le_bytes(),
            ByteOrder::Big => v.to_be_bytes(),
        }
    }

    fn u16(&mut self, at: usize, v: u16) -> &mut Self {
        self.header[at..at + 2].copy_from_slice(&Self::bytes16(self.order, v));
        self
    }

    fn f32x3(&mut self, at: usize, v: [f32; 3]) -> &mut Self {
        for (i, f) in v.iter().enumerate() {
            self.header[at + i * 4..at + i * 4 + 4]
                .copy_from_slice(&Self::bytes32(self.order, f.to_bits()));
        }
        self
    }

    /// Appends a channel's key arrays and writes their offsets into the header.
    fn channel(
        &mut self,
        count_at: usize,
        times_at: usize,
        values_at: usize,
        keys: &[(u16, (i16, i16, i16))],
    ) -> &mut Self {
        self.u16(count_at, u16::try_from(keys.len()).expect("small"));
        let times = 0x50 + self.body.len();
        for (time, _) in keys {
            self.body
                .extend_from_slice(&Self::bytes16(self.order, *time));
        }
        let values = 0x50 + self.body.len();
        for (_, (x, y, z)) in keys {
            for axis in [x, y, z] {
                self.body
                    .extend_from_slice(&Self::bytes16(self.order, *axis as u16));
            }
        }
        let times = u32::try_from(times).expect("small");
        let values = u32::try_from(values).expect("small");
        self.header[times_at..times_at + 4].copy_from_slice(&Self::bytes32(self.order, times));
        self.header[values_at..values_at + 4].copy_from_slice(&Self::bytes32(self.order, values));
        self
    }

    fn build(&self) -> Vec<u8> {
        let mut out = self.header.to_vec();
        out.extend_from_slice(&self.body);
        out
    }
}

fn translation_only(keys: &[(u16, (i16, i16, i16))], base: [f32; 3], quantum: [f32; 3]) -> Vec<u8> {
    translation_only_in(ByteOrder::Little, keys, base, quantum)
}

fn translation_only_in(
    order: ByteOrder,
    keys: &[(u16, (i16, i16, i16))],
    base: [f32; 3],
    quantum: [f32; 3],
) -> Vec<u8> {
    let mut b = Builder::with_order(order);
    b.channel(0x02, 0x0c, 0x2c, keys)
        .f32x3(0x10, base)
        .f32x3(0x20, quantum)
        // Rotation and scale each store one key even at a count of zero.
        .channel(0x04, 0x08, 0x1c, &[(0, (0, 0, 0))])
        .u16(0x04, 0)
        .channel(0x06, 0x38, 0x40, &[(0, (256, 256, 256))])
        .u16(0x06, 0);
    b.build()
}

/// A payload shorter than the header is not a transform, nor a partly-filled one.
#[test]
fn a_short_payload_decodes_to_nothing() {
    assert!(anim_transform(&[0u8; 0x4f], ByteOrder::Little).is_none());
    assert!(anim_transform(&[], ByteOrder::Little).is_none());
}

/// A key offset past the payload fails the whole decode, not a transform with one
/// plausible channel and one arbitrary one.
#[test]
fn a_key_array_past_the_payload_fails_the_decode() {
    let mut payload = translation_only(&[(0, (0, 0, 0))], [0.0; 3], [1.0; 3]);
    // Point the translation values a kilobyte past the end.
    payload[0x2c..0x30].copy_from_slice(&4096u32.to_le_bytes());
    assert!(anim_transform(&payload, ByteOrder::Little).is_none());
}

/// `pos = value * quantum + base`, as `FUN_088fed44` computes (`vmul_q`, `vadd_q`).
#[test]
fn translation_is_the_key_times_the_quantum_plus_the_base() {
    let payload = translation_only(
        &[(0, (100, 200, -300))],
        [10.0, 20.0, 30.0],
        [0.5, 0.25, 0.125],
    );
    let anim = anim_transform(&payload, ByteOrder::Little).expect("decodes");
    let m = anim.sample(0.0);
    assert!((m[12] - (10.0 + 100.0 * 0.5)).abs() < 1e-4, "{}", m[12]);
    assert!((m[13] - (20.0 + 200.0 * 0.25)).abs() < 1e-4, "{}", m[13]);
    assert!((m[14] - (30.0 + -300.0 * 0.125)).abs() < 1e-4, "{}", m[14]);
    assert!((m[15] - 1.0).abs() < 1e-6);
}

/// Below the first key time and past the last the evaluators hold the end key (the
/// `TexAnim_EvalKeyframes` clamp).
#[test]
fn a_track_holds_its_first_and_last_key() {
    let payload = translation_only(&[(60, (0, 0, 0)), (120, (100, 0, 0))], [0.0; 3], [1.0; 3]);
    let anim = anim_transform(&payload, ByteOrder::Little).expect("decodes");
    // Key times are frames, so 60 frames is one second.
    assert!((anim.sample(0.0)[12] - 0.0).abs() < 1e-4, "before");
    assert!((anim.sample(0.5)[12] - 0.0).abs() < 1e-4, "still before");
    assert!((anim.sample(1.5)[12] - 50.0).abs() < 1e-3, "halfway");
    assert!((anim.sample(2.0)[12] - 100.0).abs() < 1e-4, "at the end");
    assert!((anim.sample(99.0)[12] - 100.0).abs() < 1e-4, "past it");
}

/// `FixedFrames` snaps to the preceding key instead of blending toward the
/// next one.
#[test]
fn the_step_flag_holds_the_preceding_key() {
    let payload = translation_only(&[(0, (0, 0, 0)), (60, (100, 0, 0))], [0.0; 3], [1.0; 3]);
    let mut anim = anim_transform(&payload, ByteOrder::Little).expect("decodes");
    assert!(
        !anim.step,
        "the payload alone cannot know: it is an attribute"
    );
    assert!((anim.sample(0.5)[12] - 50.0).abs() < 1e-3, "blended");
    anim.step = true;
    assert!((anim.sample(0.5)[12] - 0.0).abs() < 1e-4, "stepped");
}

/// `LoopEnd` wraps the clock: an animation past its end replays, not holds.
#[test]
fn loop_end_wraps_the_clock() {
    let payload = translation_only(&[(0, (0, 0, 0)), (60, (60, 0, 0))], [0.0; 3], [1.0; 3]);
    let mut anim = anim_transform(&payload, ByteOrder::Little).expect("decodes");
    assert_eq!(
        anim.loop_seconds, DEFAULT_LOOP_SECONDS,
        "no attribute means the engine's own 6,000 second default"
    );
    // Without a loop, two seconds in holds the last key.
    assert!((anim.sample(2.0)[12] - 60.0).abs() < 1e-4);
    // With a one-second loop, two seconds in is back at the start.
    anim.loop_seconds = 1.0;
    assert!((anim.sample(2.0)[12] - 0.0).abs() < 1e-4, "wrapped");
    assert!((anim.sample(2.5)[12] - 30.0).abs() < 1e-3, "halfway again");
}

/// Rotation keys are the `(x, y, z)` of a unit quaternion in 1/32767 units with
/// `w` reconstructed: `(0, 0, 0)` is the identity, a quarter turn about `y` is
/// `sin(45 deg) * 32767` in `y`.
#[test]
fn rotation_is_a_compressed_unit_quaternion() {
    let mut b = Builder::new();
    let quarter = (std::f32::consts::FRAC_1_SQRT_2 * 32767.0) as i16;
    b.channel(0x02, 0x0c, 0x2c, &[(0, (0, 0, 0))])
        .u16(0x02, 0)
        .channel(0x04, 0x08, 0x1c, &[(0, (0, quarter, 0))])
        .channel(0x06, 0x38, 0x40, &[(0, (256, 256, 256))])
        .u16(0x06, 0);
    let anim = anim_transform(&b.build(), ByteOrder::Little).expect("decodes");
    let m = anim.sample(0.0);
    // A 90-degree turn about +y sends +x to -z and +z to +x, in this crate's
    // row-vector convention.
    let x_axis = [m[0], m[1], m[2]];
    assert!((x_axis[0]).abs() < 1e-3, "{x_axis:?}");
    assert!((x_axis[2] + 1.0).abs() < 1e-3, "{x_axis:?}");
}

/// Scale keys are 1/256 fixed point - `256` is `1.0` - and multiply the basis
/// rows, leaving the translation row alone.
#[test]
fn scale_is_two_fifty_sixths_and_leaves_the_translation_alone() {
    let mut b = Builder::new();
    b.channel(0x02, 0x0c, 0x2c, &[(0, (4, 0, 0))])
        .f32x3(0x20, [1.0, 1.0, 1.0])
        .f32x3(0x10, [0.0, 0.0, 0.0])
        .channel(0x04, 0x08, 0x1c, &[(0, (0, 0, 0))])
        .u16(0x04, 0)
        .channel(0x06, 0x38, 0x40, &[(0, (512, 128, 256))]);
    let anim = anim_transform(&b.build(), ByteOrder::Little).expect("decodes");
    let m = anim.sample(0.0);
    assert!((m[0] - 2.0).abs() < 1e-5, "x doubled: {}", m[0]);
    assert!((m[5] - 0.5).abs() < 1e-5, "y halved: {}", m[5]);
    assert!((m[10] - 1.0).abs() < 1e-5, "z unchanged: {}", m[10]);
    assert!(
        (m[12] - 4.0).abs() < 1e-5,
        "translation untouched: {}",
        m[12]
    );
}

/// A channel whose count is zero is not evaluated, though the payload still stores
/// one key for it (which makes the six arrays tile the payload).
#[test]
fn a_zero_count_channel_contributes_nothing() {
    let payload = translation_only(&[(0, (0, 0, 0))], [0.0; 3], [1.0; 3]);
    let anim = anim_transform(&payload, ByteOrder::Little).expect("decodes");
    assert!(anim.rotation.is_empty());
    assert!(anim.scale.is_empty());
    let m = anim.sample(0.0);
    assert_eq!(m[0], 1.0, "no rotation and no scale is the identity basis");
    assert_eq!(m[5], 1.0);
    assert_eq!(m[10], 1.0);
}

/// A `seconds_per_key` of zero falls back to 60 Hz rather than dividing by it.
#[test]
fn a_zero_seconds_per_key_falls_back_to_sixty_hertz() {
    let mut payload = translation_only(&[(0, (0, 0, 0)), (60, (60, 0, 0))], [0.0; 3], [1.0; 3]);
    payload[0x3c..0x40].copy_from_slice(&0.0f32.to_le_bytes());
    let anim = anim_transform(&payload, ByteOrder::Little).expect("decodes");
    assert!((anim.sample(0.5)[12] - 30.0).abs() < 1e-3);
}

/// The same keys written big-endian decode to the same transform, and read the
/// wrong way round decode to nothing.
///
/// HD writes this class big-endian on 5,518 nodes, and a little-endian read does
/// **not** give a slightly wrong matrix: key counts inflate by 256, the arrays run
/// past the payload, and the node falls back to the identity, dropping its
/// placement with its motion. See `docs/formats/hd-status.md`.
#[test]
fn a_big_endian_payload_decodes_the_same_and_only_that_way() {
    let keys = [(0, (0, 0, 0)), (60, (100, -200, 300))];
    let base = [1.5, -2.5, 3.5];
    let quantum = [0.25, 0.5, 0.125];
    let little = translation_only_in(ByteOrder::Little, &keys, base, quantum);
    let big = translation_only_in(ByteOrder::Big, &keys, base, quantum);
    assert_ne!(little, big, "the two spellings are different bytes");

    let want = anim_transform(&little, ByteOrder::Little).expect("decodes");
    let got = anim_transform(&big, ByteOrder::Big).expect("decodes big-endian");
    assert_eq!(got, want);

    assert!(
        anim_transform(&big, ByteOrder::Little).is_none(),
        "a big-endian payload read little-endian must fail rather than mislead"
    );
}

/// A whole `f32` quaternion key is `(w, x, y, z)`, `w` first: a quarter turn about `y` is
/// `(cos 45, 0, sin 45, 0)`, and read `(x, y, z, w)` the same four floats are a half turn about
/// the other axis. The order was measured on HD's grid cameras, see [`ROTATION_IS_QUATERNION`].
#[test]
fn a_whole_float_quaternion_key_is_w_first() {
    let order = ByteOrder::Big;
    let mut b = Builder::with_order(order);
    b.u16(0x04, 1);
    let times = 0x50 + b.body.len();
    b.body.extend_from_slice(&Builder::bytes16(order, 0));
    b.body.extend_from_slice(&[0, 0]);
    let values = 0x50 + b.body.len();
    let half = std::f32::consts::FRAC_1_SQRT_2;
    for v in [half, 0.0, half, 0.0] {
        b.body.extend_from_slice(&v.to_bits().to_be_bytes());
    }
    b.header[0x08..0x0c].copy_from_slice(&Builder::bytes32(order, times as u32));
    b.header[0x1c..0x20].copy_from_slice(&Builder::bytes32(order, values as u32));
    // The flag widens the rotation key and nothing else, so the other channels keep their
    // `s16` keys (a count of zero stores one).
    b.header[0x34..0x38].copy_from_slice(&Builder::bytes32(order, ROTATION_IS_QUATERNION));
    for (count_at, times_at, values_at, key) in [
        (0x02, 0x0c, 0x2c, (0, 0, 0)),
        (0x06, 0x38, 0x40, (256, 256, 256)),
    ] {
        b.channel(count_at, times_at, values_at, &[(0, key)])
            .u16(count_at, 0);
    }
    let anim = anim_transform(&b.build(), order).expect("decodes");
    assert_eq!(anim.rotation.w, [half]);
    assert_eq!(anim.rotation.values, [[0.0, half, 0.0]]);
    let m = anim.sample(0.0);
    assert!(m[0].abs() < 1e-3 && (m[2] + 1.0).abs() < 1e-3, "{m:?}");
    assert!((m[5] - 1.0).abs() < 1e-3, "y stays the axis: {m:?}");
}
