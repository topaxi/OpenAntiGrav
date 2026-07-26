//! State hashing for determinism checks and replay validation.
//!
//! A [`StateHasher`] reduces a world snapshot to one 64-bit value. Two runs
//! that agree on every hash agree on every bit of state; two runs that diverge
//! are caught at the first tick where they differ, which is far easier to debug
//! than noticing the ships in the wrong place ten seconds later.
//!
//! Floats are hashed by their raw bits on purpose. `-0.0` and `0.0` compare
//! equal but are different states, and any difference at all is a determinism
//! failure worth surfacing.

/// FNV-1a over 64 bits.
///
/// Chosen for being trivially specified and dependency-free rather than for
/// speed or collision resistance. This is a tripwire, not a security boundary:
/// it must produce identical output on every platform forever, so the algorithm
/// being short enough to re-implement from memory is a feature.
#[derive(Debug, Clone)]
pub struct StateHasher {
    state: u64,
}

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

impl Default for StateHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl StateHasher {
    /// Creates an empty hasher.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: FNV_OFFSET_BASIS,
        }
    }

    /// Absorbs raw bytes.
    pub fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state ^= u64::from(b);
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
    }

    /// Absorbs a `u8`.
    pub fn write_u8(&mut self, v: u8) {
        self.write(&[v]);
    }

    /// Absorbs a `u32` in little-endian order.
    ///
    /// The byte order is fixed explicitly so a big-endian target hashes the
    /// same as a little-endian one.
    pub fn write_u32(&mut self, v: u32) {
        self.write(&v.to_le_bytes());
    }

    /// Absorbs a `u64` in little-endian order.
    pub fn write_u64(&mut self, v: u64) {
        self.write(&v.to_le_bytes());
    }

    /// Absorbs an `i32` in little-endian order.
    pub fn write_i32(&mut self, v: i32) {
        self.write(&v.to_le_bytes());
    }

    /// Absorbs an `f32` by its bit pattern.
    pub fn write_f32(&mut self, v: f32) {
        self.write(&v.to_bits().to_le_bytes());
    }

    /// Absorbs a [`Vec3`](crate::math::Vec3) component-wise.
    pub fn write_vec3(&mut self, v: crate::math::Vec3) {
        self.write_f32(v.x);
        self.write_f32(v.y);
        self.write_f32(v.z);
    }

    /// Absorbs a [`Quat`](crate::math::Quat) component-wise.
    pub fn write_quat(&mut self, q: crate::math::Quat) {
        self.write_f32(q.x);
        self.write_f32(q.y);
        self.write_f32(q.z);
        self.write_f32(q.w);
    }

    /// The hash so far.
    #[must_use]
    pub fn finish(&self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Vec3;

    #[test]
    fn matches_the_published_fnv1a_vector() {
        // "a" -> 0xaf63dc4c8601ec8c is a standard FNV-1a 64 test vector. If
        // this fails the implementation has drifted from the spec, and every
        // committed reference hash in the tree is invalid.
        let mut h = StateHasher::new();
        h.write(b"a");
        assert_eq!(h.finish(), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn empty_input_is_the_offset_basis() {
        assert_eq!(StateHasher::new().finish(), FNV_OFFSET_BASIS);
    }

    #[test]
    fn distinguishes_positive_and_negative_zero() {
        let mut a = StateHasher::new();
        a.write_f32(0.0);
        let mut b = StateHasher::new();
        b.write_f32(-0.0);
        assert_ne!(
            a.finish(),
            b.finish(),
            "0.0 and -0.0 are different states even though they compare equal"
        );
    }

    #[test]
    fn is_order_sensitive() {
        let mut a = StateHasher::new();
        a.write_u32(1);
        a.write_u32(2);
        let mut b = StateHasher::new();
        b.write_u32(2);
        b.write_u32(1);
        assert_ne!(a.finish(), b.finish());
    }

    #[test]
    fn vec3_hashes_component_wise() {
        let mut a = StateHasher::new();
        a.write_vec3(Vec3::new(1.0, 2.0, 3.0));

        let mut b = StateHasher::new();
        b.write_f32(1.0);
        b.write_f32(2.0);
        b.write_f32(3.0);

        assert_eq!(a.finish(), b.finish());
    }
}
