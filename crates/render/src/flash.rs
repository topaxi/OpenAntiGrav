//! The full-screen wash a weapon detonation starts: `ScreenFlash_Start`
//! (`0x088f00c0`) and its per-frame consumer, read off Pulse's PSP
//! `BOOT.BIN` on 2026-09-24. See
//! `docs/ghidra/functions/psp-pulse-usa/particle-system.md`, "The screen
//! flash's consumer".
//!
//! One node, `DAT_08ab2200`, holds one flash at a time:
//!
//! - **Start** writes the kind's parameters into a pending block and raises
//!   `+0x1a0`. Nothing more: a detonation never draws on its own.
//! - **Update** (`ScreenFlash_Update`, `0x088ef3a4`), once a frame. A pending
//!   flash replaces the running one only when it is the stronger of the two,
//!   strength being `(alpha * falloff)^2 * |rgb|^2` of the pending one's first
//!   key against the colour last drawn - stale when nothing runs. Then the
//!   running flash's clock grows by the frame's `dt` (`+0x180`, stored by the
//!   node's time-step virtual `0x088ef398`), its colour is the two keys
//!   lerped by `elapsed / duration`, and its alpha is scaled by the distance
//!   falloff. At `t >= 1` it stops.
//! - **The falloff** (`ScreenFlash_DistanceFalloff`, `0x088effb8`) is
//!   `clamp(1 - (d - near) / (far - near), 0, 1)` with `d` the distance from
//!   the eye (`-(camera + 0x70)`) to the flash's position, re-measured every
//!   frame; `far == 0` means no falloff.
//! - **Draw** (`ScreenFlash_Draw`, `0x088efb1c`): identity matrices, depth
//!   test, texturing, culling and lighting off, the glow mask protected, one
//!   flat-coloured quad over the whole screen, `Gu_Color` from the colour
//!   truncated to bytes. The blend is `(ADD, SRC_ALPHA, FIX 0xffffff)` -
//!   additive - for every kind but `0xb`, which is alpha-over and unwired
//!   here.
//!
//! **Measured against the original**: a craft-hit Rocket 51 units away
//! (kind 0, full falloff) adds `153 (1 - t)` to red and `153 (1 - t)^2` to
//! green and nothing to blue, `t` stepping by `(1/60) / 0.5` a frame - see the
//! doc page. **Pulse on the PSP only**: the caller holds no [`ScreenFlash`]
//! for any other source.

use oag_core::math::Vec3;

/// One kind's parameters, as `ScreenFlash_Start` fills them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Kind {
    /// `+0x44`, seconds.
    pub duration: f32,
    /// `+0x4c`: full strength inside this distance.
    pub near: f32,
    /// `+0x50`: gone beyond it; `0` means no falloff at all.
    pub far: f32,
    /// `+0x70` at time 0 and `+0x80` at time 1, RGBA in `0..=1`.
    pub keys: [[f32; 4]; 2],
}

/// Kind 0: `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`), a Rocket that
/// struck a craft - yellow fading to red.
pub const ROCKET_CRAFT_HIT: Kind = Kind {
    duration: 0.5,
    near: 75.0,
    far: 200.0,
    keys: [[1.0, 1.0, 0.0, 0.6], [1.0, 0.0, 0.0, 0.0]],
};

/// Kind 4: `Quake_Update` (`0x0891d268`), re-started every frame the wave
/// runs - an orange tint.
pub const QUAKE: Kind = Kind {
    duration: 0.4,
    near: 0.0,
    far: 300.0,
    keys: [[1.0, 0.2, 0.0, 0.5], [1.0, 0.2, 0.0, 0.0]],
};

impl Kind {
    /// `ScreenFlash_DistanceFalloff` for a flash at `at` seen from `eye`.
    #[must_use]
    pub fn falloff(&self, at: Vec3, eye: Vec3) -> f32 {
        if self.far <= 0.0 {
            return 1.0;
        }
        let distance = (at - eye).length();
        (1.0 - (distance - self.near) / (self.far - self.near)).clamp(0.0, 1.0)
    }

    /// The keys lerped at `t`, the node's one segment between times 0 and 1.
    fn colour_at(&self, t: f32) -> [f32; 4] {
        let [a, b] = self.keys;
        std::array::from_fn(|i| a[i] + t * (b[i] - a[i]))
    }
}

/// How strongly a colour reads to the replacement test in
/// `ScreenFlash_Update`: alpha squared times the colour's squared length.
fn strength(colour: [f32; 4]) -> f32 {
    let [r, g, b, a] = colour;
    a * a * (r * r + g * g + b * b)
}

/// The node's state: render-only, never in `World`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScreenFlash {
    pending: Option<(Kind, Vec3)>,
    running: Option<(Kind, Vec3, f32)>,
    /// `+0x190..+0x19c`: the colour last computed, falloff included. Kept
    /// when a flash ends, because the next pending one is measured against it.
    colour: [f32; 4],
    visible: bool,
}

impl ScreenFlash {
    /// `ScreenFlash_Start`: a flash of `kind` at world position `at`. The
    /// last call before [`Self::advance`] wins, as the pending block is
    /// simply overwritten.
    pub fn start(&mut self, kind: Kind, at: Vec3) {
        self.pending = Some((kind, at));
    }

    /// One `ScreenFlash_Update` of `dt` seconds, seen from `eye`.
    pub fn advance(&mut self, dt: f32, eye: Vec3) {
        if let Some((kind, at)) = self.pending.take() {
            let mut first = kind.keys[0];
            first[3] *= kind.falloff(at, eye);
            if strength(self.colour) < strength(first) {
                self.running = Some((kind, at, 0.0));
            }
        }
        self.visible = false;
        let Some((kind, at, elapsed)) = &mut self.running else {
            return;
        };
        *elapsed += dt;
        let t = *elapsed / kind.duration;
        if t >= 1.0 {
            self.running = None;
            return;
        }
        let mut colour = kind.colour_at(t);
        colour[3] *= kind.falloff(*at, eye);
        self.colour = colour;
        self.visible = true;
    }

    /// What to draw this frame: the colour `Gu_Color` receives, each channel
    /// truncated to a byte and back, or `None` when nothing runs.
    #[must_use]
    pub fn colour(&self) -> Option<[f32; 4]> {
        self.visible
            .then(|| self.colour.map(|c| f32::from((c * 255.0) as u8) / 255.0))
    }
}

mod pipeline;
pub use pipeline::Pipeline;

#[cfg(test)]
mod tests;
