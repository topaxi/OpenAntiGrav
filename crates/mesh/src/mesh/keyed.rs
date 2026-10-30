//! [`Keyed`]: a node moved by keyframed translation, rotation and scale with
//! keys at any times, each track looping or holding at its end - the shape
//! glTF animation carries, for a model whose motion is authored that way
//! rather than as a `.vex` `Anim Transform` or an `.rcsanimclip` track.
//!
//! [`Motion::Rig`](super::Motion) is not reused: its keys are evenly spaced,
//! its last segment blends back to key 0 and it always wraps, so keys at
//! uneven times, a held end or a stepped key would change meaning in it.
//! The local matrix is the rig's own [`oag_rcs::rcsskeleton::local_matrix`]
//! with no pivot, in the row-vector convention every node matrix here uses.

/// One property's keys.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyTrack<const N: usize> {
    /// Key times in seconds, strictly increasing.
    pub times: Vec<f32>,
    /// One value per time.
    pub values: Vec<[f32; N]>,
    /// Each key holds until the next (no blend).
    pub step: bool,
    /// The clock wraps on (or holds at) this many seconds: the length of the
    /// animation the track belongs to, which may outlast its own last key.
    pub period: f32,
    /// Holds past [`Self::period`] instead of wrapping.
    pub clamp: bool,
}

impl<const N: usize> KeyTrack<N> {
    /// The track's own time at clock `seconds`.
    fn local(&self, seconds: f32) -> f32 {
        if self.period <= 0.0 {
            0.0
        } else if self.clamp {
            seconds.clamp(0.0, self.period)
        } else {
            seconds.rem_euclid(self.period)
        }
    }

    /// The keys straddling clock `seconds` and the blend between them; both
    /// the same key before the first time, after the last, or on a step.
    fn at(&self, seconds: f32) -> (usize, usize, f32) {
        let t = self.local(seconds);
        let last = self.times.len().saturating_sub(1);
        if self.times.is_empty() || t <= self.times[0] {
            return (0, 0, 0.0);
        }
        if t >= self.times[last] {
            return (last, last, 0.0);
        }
        let j = self.times.partition_point(|&k| k <= t);
        let i = j - 1;
        if self.step {
            return (i, i, 0.0);
        }
        let span = self.times[j] - self.times[i];
        (i, j, (t - self.times[i]) / span)
    }

    /// The value at clock `seconds`, blended linearly.
    #[must_use]
    pub fn lerp(&self, seconds: f32) -> Option<[f32; N]> {
        let (i, j, f) = self.at(seconds);
        let (a, b) = (self.values.get(i)?, self.values.get(j)?);
        Some(std::array::from_fn(|k| a[k] + (b[k] - a[k]) * f))
    }
}

impl KeyTrack<4> {
    /// The rotation at clock `seconds`: spherical interpolation along the
    /// shorter arc, as glTF's `LINEAR` asks of a rotation.
    #[must_use]
    pub fn slerp(&self, seconds: f32) -> Option<[f32; 4]> {
        let (i, j, f) = self.at(seconds);
        Some(slerp(*self.values.get(i)?, *self.values.get(j)?, f))
    }
}

/// `a` to `b` by `f` on the unit sphere, the shorter way; a near-parallel pair
/// blends linearly and renormalises.
#[must_use]
pub fn slerp(a: [f32; 4], b: [f32; 4], f: f32) -> [f32; 4] {
    let mut dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    let b = if dot < 0.0 {
        dot = -dot;
        b.map(|c| -c)
    } else {
        b
    };
    let (wa, wb) = if dot > 0.9995 {
        (1.0 - f, f)
    } else {
        let theta = dot.min(1.0).acos();
        let s = theta.sin();
        (((1.0 - f) * theta).sin() / s, (f * theta).sin() / s)
    };
    let q: [f32; 4] = std::array::from_fn(|k| a[k] * wa + b[k] * wb);
    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if len > 0.0 { q.map(|c| c / len) } else { a }
}

/// A node's rest pose and the tracks that move it.
#[derive(Debug, Clone, PartialEq)]
pub struct Keyed {
    /// Rest translation, for when no track drives it.
    pub translation: [f32; 3],
    /// Rest rotation, a unit quaternion `x, y, z, w`.
    pub rotation: [f32; 4],
    /// Rest scale.
    pub scale: [f32; 3],
    /// Translation keys.
    pub translation_keys: Option<KeyTrack<3>>,
    /// Rotation keys.
    pub rotation_keys: Option<KeyTrack<4>>,
    /// Scale keys.
    pub scale_keys: Option<KeyTrack<3>>,
    /// The channel (`oag_ext_abi::channel_key`) that starts the tracks: they hold their
    /// first key until it fires, then play from the moment it did. `None` plays from
    /// the clock's zero.
    pub on: Option<u32>,
}

impl Keyed {
    /// The rotation at `seconds`.
    #[must_use]
    pub fn rotation_at(&self, seconds: f32) -> [f32; 4] {
        self.rotation_keys
            .as_ref()
            .and_then(|k| k.slerp(seconds))
            .unwrap_or(self.rotation)
    }

    /// The clock the tracks read at world clock `seconds`, given when each channel last
    /// fired (`started`): the world clock itself for a node no channel starts, else the
    /// time since its channel fired, `0` before it has.
    #[must_use]
    pub fn clock(&self, seconds: f32, started: &dyn Fn(u32) -> Option<f32>) -> f32 {
        match self.on {
            None => seconds,
            Some(channel) => started(channel).map_or(0.0, |at| (seconds - at).max(0.0)),
        }
    }

    /// The local matrix at `seconds`: scale, then rotate, then translate,
    /// row-major with the translation in row 3.
    #[must_use]
    pub fn sample(&self, seconds: f32) -> [f32; 16] {
        let translation = self
            .translation_keys
            .as_ref()
            .and_then(|k| k.lerp(seconds))
            .unwrap_or(self.translation);
        let scale = self
            .scale_keys
            .as_ref()
            .and_then(|k| k.lerp(seconds))
            .unwrap_or(self.scale);
        oag_rcs::rcsskeleton::local_matrix(
            scale,
            self.rotation_at(seconds),
            translation,
            [0.0; 3],
            [0.0; 3],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track<const N: usize>(times: &[f32], values: &[[f32; N]], clamp: bool) -> KeyTrack<N> {
        KeyTrack {
            times: times.to_vec(),
            values: values.to_vec(),
            step: false,
            period: *times.last().unwrap(),
            clamp,
        }
    }

    fn point(m: &[f32; 16], p: [f32; 3]) -> [f32; 3] {
        std::array::from_fn(|c| p[0] * m[c] + p[1] * m[4 + c] + p[2] * m[8 + c] + m[12 + c])
    }

    fn near(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1e-5)
    }

    /// A node a channel starts reads the clock since the channel fired (zero before it has),
    /// and a node no channel starts reads the world clock.
    #[test]
    fn a_channel_started_node_reads_the_time_since_its_channel_fired() {
        let node = |on| Keyed {
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            translation_keys: Some(track(&[0.0, 2.0], &[[0.0; 3], [10.0, 0.0, 0.0]], false)),
            rotation_keys: None,
            scale_keys: None,
            on,
        };
        let fired_at = |channel: u32| (channel == 7).then_some(10.0);
        assert_eq!(node(None).clock(11.0, &fired_at), 11.0);
        assert_eq!(node(Some(7)).clock(11.0, &fired_at), 1.0);
        assert_eq!(node(Some(7)).clock(9.0, &fired_at), 0.0, "before the fire");
        assert_eq!(
            node(Some(8)).clock(11.0, &fired_at),
            0.0,
            "its channel never fired"
        );
    }

    /// glTF's `T * R * S` applied to a column vector is this matrix applied to
    /// a row vector: a point one unit along +X, scaled by 2, turned 90 degrees
    /// about +Y (to -Z) and moved by the translation halfway between its keys.
    #[test]
    fn the_matrix_is_scale_then_rotation_then_translation() {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let node = Keyed {
            translation: [0.0; 3],
            rotation: [0.0, h, 0.0, h],
            scale: [2.0; 3],
            translation_keys: Some(track(&[0.0, 2.0], &[[0.0; 3], [10.0, 4.0, 0.0]], false)),
            rotation_keys: None,
            scale_keys: None,
            on: None,
        };
        let m = node.sample(1.0);
        assert!(near(point(&m, [1.0, 0.0, 0.0]), [5.0, 2.0, -2.0]), "{m:?}");
    }

    /// A looping track wraps on its period, a clamped one holds its last key,
    /// and before the first key both hold the first.
    #[test]
    fn loop_wraps_and_clamp_holds() {
        let looping = track(&[1.0, 3.0], &[[0.0; 3], [2.0, 0.0, 0.0]], false);
        let clamped = KeyTrack {
            clamp: true,
            ..looping.clone()
        };
        assert_eq!(looping.lerp(2.0), Some([1.0, 0.0, 0.0]));
        assert_eq!(looping.lerp(5.0), Some([1.0, 0.0, 0.0]));
        assert_eq!(looping.lerp(0.5), Some([0.0; 3]));
        assert_eq!(clamped.lerp(5.0), Some([2.0, 0.0, 0.0]));
        assert_eq!(clamped.lerp(-1.0), Some([0.0; 3]));
        let stepped = KeyTrack {
            step: true,
            ..looping
        };
        assert_eq!(stepped.lerp(2.9), Some([0.0; 3]));
    }

    /// A quarter turn about +Y blended halfway is an eighth of a turn, the
    /// short way even when the second key is written as its negation.
    #[test]
    fn rotation_blends_by_slerp_along_the_short_arc() {
        let h = std::f32::consts::FRAC_1_SQRT_2;
        let q = track(
            &[0.0, 1.0],
            &[[0.0, 0.0, 0.0, 1.0], [0.0, -h, 0.0, -h]],
            false,
        );
        let mid = q.slerp(0.5).unwrap();
        let eighth = std::f32::consts::FRAC_PI_8;
        assert!(
            (mid[1] - eighth.sin()).abs() < 1e-5 && (mid[3] - eighth.cos()).abs() < 1e-5,
            "{mid:?}"
        );
    }
}
