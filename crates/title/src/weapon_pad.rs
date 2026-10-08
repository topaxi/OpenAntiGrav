//! What colours a **weapon pad's light bars** at run time, as a title answers it.
//!
//! A pad's own texture paints its plate and its outline; the bars are the
//! `_ne` mask's alpha, added after the light as `mask * colour`
//! (`docs/rendering/pads.md`). Where `colour` comes from is the axis:
//!
//! - **Pulse and Pure paint no colour into the pad texture** and recolour the
//!   whole mesh instead, through a different path (`oag_render::weapon_pad`);
//!   nothing here applies to them.
//! - **HD overwrites the `.rcsmaterial`'s authored `W_Cycle` value every frame**
//!   with the pad object's own cycle vector (`WeaponPad_UpdateRefreshTimer`,
//!   `0x002e02b8`), measured on RPCS3: the program's inline constant read
//!   `{1.166, 0.033, 0, 1}` on one pad and `{1.656, 0.041, 0, 1}` on another of
//!   the same circuit whose material authors cyan.
//! - **Everything else** keeps the authored value, which is what the loader
//!   already binds.

/// How a title colours its weapon pads' light bars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WeaponPadGlow {
    /// The material's authored `W_Cycle` value, unchanged: what every title
    /// had before HD's cycle was read, and what a title whose own pad code has
    /// not been read keeps.
    Authored,
    /// A per-pad cycle through keyframes while the pad is ready, and a flat
    /// colour while it cools down.
    Cycle(Cycle),
}

/// HD's weapon-pad colour cycle: the numbers `WeaponPad_UpdateRefreshTimer`
/// reads, as that function reads them.
///
/// Per pad, while its refresh timer is at the floor: a position `t` advances
/// by `dt * keys_per_second`; the colour is the linear blend of the current
/// and the next keyframe (wrapping) at `t`, times `scale`; when `t` reaches
/// `1.0` the pad moves on to the next keyframe and `t` restarts. While the pad
/// cools down the cycle does not advance and the colour is `cooling`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cycle {
    /// The keyframes, as authored (`0..512`, not scaled).
    pub keyframes: &'static [[f32; 3]],
    /// Keyframes per second: `dt * 3.0` in the original.
    pub keys_per_second: f32,
    /// Multiplier applied to the blended keyframe: `1/255` in the original.
    pub scale: f32,
    /// The colour while the pad cools down, already scaled.
    pub cooling: [f32; 3],
}

impl Cycle {
    /// The bar colour of one pad at `position` keyframes into its cycle.
    ///
    /// `position` counts from the first keyframe and wraps over the table, so
    /// `2.5` is halfway from the third keyframe to the fourth.
    #[must_use]
    pub fn colour(&self, position: f32) -> [f32; 3] {
        let keys = self.keyframes.len();
        if keys == 0 {
            return [0.0; 3];
        }
        let wrapped = position.rem_euclid(keys as f32);
        let from = (wrapped as usize).min(keys - 1);
        let t = wrapped - from as f32;
        let (a, b) = (self.keyframes[from], self.keyframes[(from + 1) % keys]);
        std::array::from_fn(|c| ((1.0 - t) * a[c] + t * b[c]) * self.scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CYCLE: Cycle = Cycle {
        keyframes: &[[512.0, 16.0, 0.0], [256.0, 0.0, 0.0], [64.0, 0.0, 0.0]],
        keys_per_second: 3.0,
        scale: 1.0 / 255.0,
        cooling: [0.025, 0.0, 0.01],
    };

    #[test]
    fn a_whole_position_is_its_keyframe_scaled() {
        let c = CYCLE.colour(1.0);
        assert!((c[0] - 256.0 / 255.0).abs() < 1e-6 && c[1] == 0.0 && c[2] == 0.0);
    }

    #[test]
    fn the_table_wraps_back_to_its_first_keyframe() {
        let c = CYCLE.colour(2.5);
        let (a, b) = (64.0 / 255.0, 512.0 / 255.0);
        assert!((c[0] - 0.5 * (a + b)).abs() < 1e-5);
        assert_eq!(CYCLE.colour(3.0), CYCLE.colour(0.0));
    }
}
