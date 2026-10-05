//! The Repulser's field model: `Data\Weapons\pulse_repulsorwave.vex`, a flat
//! blue ring (radius 10.17 at scale 1, `noise2_ADD_GLOW`) around the firer that
//! fades in while it shrinks, then widens to four times its size and fades out
//! once the waves start. See
//! `docs/ghidra/functions/psp-pulse-usa/repulser.md#the-field-model`.
//!
//! **Recovered, confidence 80** (decompile plus the instruction listing of
//! `Repulser_Init` `0x08875210`, `Repulser_Update` `0x08875400` and
//! `Repulser_UpdateFieldModel` `0x088758cc`; not runtime-verified):
//!
//! - the three eases and their order inside one update ([`Field::step`]);
//! - the frame: the track's own basis at the firer's AI-track point, scaled
//!   uniformly, translated to the firer's position ([`field_matrix`]);
//! - the colour: white with alpha `+0x210 * 255` stamped on every mesh
//!   (`Image_SetVertexColours`), the ambient-alpha mechanism the Bomb's
//!   shockwave already uses;
//! - visible for the Repulser's whole life (`Repulser_Update` raises the
//!   model's visible bits every tick), so 1.6 s, gone with the slot.
//!
//! **The model does not spin.** `Node_SetLocalMatrix(model, scaled basis, 0)`
//! puts the node in mode `0x1000000`, which `Vex_UpdateNodeWorldMatrix`
//! composes with the parent's world matrix; the parent is the Repulser entity,
//! a bare object whose mode is `0` all the way to the root, so the model's world
//! matrix is the scaled basis alone. The `+0x21c` spin rotates a *second* copy
//! of the unscaled basis about its own up row (`Math_RotateByAxisAngle` at
//! `0x08876168`, axis `sp+0x140`) and is stored to the entity's `+0x1a0`, the
//! matrix `WO_REPULSER_BLAST` is anchored to - see
//! [`Field::spin`].
//!
//! **Chosen, not measured:** the basis comes from the nearest spline sample to
//! the firer, where the original reads the AI-track control point under the
//! firer's own cursor (`craft+0xad8`); and the eases step once per 60 Hz tick,
//! where the original steps `(int)(dt / (1/60))` times (the same choice
//! `bomb_blast` makes; see `ship-shockwave.md`).

use super::*;
use oag_weapons::projectile::repulser::POOL_SIZE;

/// One `(target, rate)` ease, stepped `current += (target - current) * rate`.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Toward {
    target: f32,
    rate: f32,
}

impl Toward {
    const fn step(self, current: f32) -> f32 {
        current + (self.target - current) * self.rate
    }
}

/// `+0x204` starts at `1.0` (`0x3f800000`) and eases toward `+0x208` = `0.6`
/// (`0x3f19999a`) at `+0x20c` = `0.1` (`0x3dcccccd`), all `Repulser_Init`.
const SHRINK_START: f32 = 1.0;
const SHRINK: Toward = Toward {
    target: 0.6,
    rate: 0.1,
};

/// The shrink scale is used while it is **above** this, read before the step:
/// `if (0.61 < +0x204)` in `Repulser_UpdateFieldModel`.
const SHRINK_UNTIL: f32 = 0.61;

/// `+0x1f8` starts at `0.7` (`0x3f333333`) and eases toward `+0x1fc` = `4.0`
/// (`0x40800000`) at `+0x200` = `0.05` (`0x3d4ccccd`). It takes over from the
/// shrink with a jump (0.61 to 0.865 on its first step), not a blend.
const GROW_START: f32 = 0.7;
const GROW: Toward = Toward {
    target: 4.0,
    rate: 0.05,
};

/// `+0x210` starts at `0`, eases toward `+0x214` = `1.0` at `+0x218` = `0.2`
/// (`0x3e4ccccd`) in `Repulser_Update`, before the field model is built.
const FADE_IN: Toward = Toward {
    target: 1.0,
    rate: 0.2,
};

/// On the tick the waves start `Repulser_Update` sets `+0x210 = 1.0`, `+0x214 = 0`
/// and `+0x218 = 0.1`, **after** that tick's field model was built.
const FADE_OUT: Toward = Toward {
    target: 0.0,
    rate: 0.1,
};

/// `+0x21c` starts at `0` and eases toward `+0x220` = `-2pi` (`0xc0c90fdb`) at
/// `+0x224` = `0.02` (`0x3ca3d70a`), only once the entity is older than this
/// (`if (0.4 < +0x1ec)`, `0x088760fc`).
const SPIN: Toward = Toward {
    target: -std::f32::consts::TAU,
    rate: 0.02,
};
const SPIN_AFTER_SECONDS: f32 = 0.4;

/// One live Repulser's field-model state: view state, not `World` state, on
/// `bomb_blast`'s terms - nothing here reaches the simulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Field {
    /// `+0x204`.
    shrink: f32,
    /// `+0x1f8`.
    grow: f32,
    /// `+0x210`.
    alpha: f32,
    /// `+0x21c`, radians.
    pub(super) spin: f32,
    /// Whether the waves have started (`+0x50 == 1`).
    waves: bool,
    /// The scale this tick's model was built with.
    scale: f32,
    /// The alpha this tick's model was stamped with.
    drawn_alpha: f32,
}

impl Field {
    /// `Repulser_Init`'s stores.
    pub(super) const fn new() -> Self {
        Self {
            shrink: SHRINK_START,
            grow: GROW_START,
            alpha: 0.0,
            spin: 0.0,
            waves: false,
            scale: SHRINK_START,
            drawn_alpha: 0.0,
        }
    }

    /// One `Repulser_Update`, in its own order: the alpha ease, then the scale
    /// (whichever branch the pre-step shrink value picks), then the spin past
    /// 0.4 s, then - on the tick the waves start - the fade-out's reset, which
    /// this tick's picture never sees.
    ///
    /// `age` is the Repulser's age after this tick's increment; `waves` is
    /// whether it is in its wave phase once this tick's update is done.
    pub(super) fn step(&mut self, age: f32, waves: bool) {
        let fade = if self.waves { FADE_OUT } else { FADE_IN };
        self.alpha = fade.step(self.alpha);
        if self.shrink > SHRINK_UNTIL {
            self.shrink = SHRINK.step(self.shrink);
            self.scale = self.shrink;
        } else {
            self.grow = GROW.step(self.grow);
            self.scale = self.grow;
        }
        self.drawn_alpha = self.alpha;
        if age > SPIN_AFTER_SECONDS {
            self.spin = SPIN.step(self.spin);
        }
        if waves && !self.waves {
            self.waves = true;
            self.alpha = 1.0;
        }
    }

    /// The uniform scale this tick's model is drawn at.
    pub(super) const fn scale(&self) -> f32 {
        self.scale
    }

    /// The alpha this tick's model is drawn with - the value
    /// `Image_SetVertexColours` stamped, which on the wave-start tick is
    /// still the fade-in's.
    pub(super) const fn drawn_alpha(&self) -> f32 {
        self.drawn_alpha
    }
}

/// `Repulser_UpdateFieldModel`'s basis at `position`, from the track's `lateral`
/// and `down` there, scaled by `scale`.
///
/// The original: `across = normalize(right_edge - left_edge)` (the lateral),
/// `up = -down` (the point's `+0x20` row, `vneg.q`), `forward =
/// normalize(across x up)`, `up' = normalize(up - forward * (forward . up))`,
/// `side = up' x forward`; rows `(side, up', forward) * scale` and the firer's
/// translation. The ring lies in its own `XZ` plane (`oag-view --mesh`), so
/// `up'` in `Y` lays it flat on the road.
pub(super) fn field_matrix(position: Vec3, lateral: Vec3, down: Vec3, scale: f32) -> Mat4 {
    let (side, up, forward) = field_basis(lateral, down);
    Mat4::from_cols(
        (side * scale).extend(0.0),
        (up * scale).extend(0.0),
        (forward * scale).extend(0.0),
        position.extend(1.0),
    )
}

/// The unscaled `(side, up', forward)` of [`field_matrix`].
pub(super) fn field_basis(lateral: Vec3, down: Vec3) -> (Vec3, Vec3, Vec3) {
    let across = lateral.normalize_or_zero();
    let up = -down;
    let forward = across.cross(up).normalize_or_zero();
    let up = (up - forward * forward.dot(up)).normalize_or_zero();
    let side = up.cross(forward);
    (side, up, forward)
}

impl Race {
    /// Steps every live Repulser's field model one tick, starting one on the
    /// tick its slot fills and dropping it when the slot empties. Called from
    /// [`Race::advance_repulser_visual`].
    pub(crate) fn advance_repulser_fields(&mut self) {
        for index in 0..POOL_SIZE {
            let Some(repulser) = self.sim.world.repulsers[index] else {
                self.view.repulser_fields[index] = None;
                continue;
            };
            let field = self.view.repulser_fields[index].get_or_insert_with(Field::new);
            field.step(repulser.age, repulser.fronts.is_some());
        }
    }

    /// This tick's model matrix, alpha and the Repulser's age (the texture
    /// track's time: `Repulser_Init` seeds the model with
    /// `Node_SetAnimTimeTree(0.0)`) for every pool slot with a live field and
    /// a spline sample under its firer.
    #[must_use]
    pub(crate) fn repulser_field_draws(&self) -> [Option<(Mat4, f32, f32)>; POOL_SIZE] {
        std::array::from_fn(|index| {
            let field = self.view.repulser_fields[index]?;
            let repulser = self.sim.world.repulsers[index]?;
            let firer = self.sim.world.ships[repulser.owner as usize]
                .physics
                .body
                .position;
            let (_, sample, _) = self.sim.spline.nearest(firer)?;
            let matrix = field_matrix(
                firer,
                Vec3::from_array(sample.lateral),
                Vec3::from_array(sample.down),
                field.scale(),
            );
            Some((matrix, field.drawn_alpha(), repulser.age))
        })
    }
}

#[cfg(test)]
mod tests;
