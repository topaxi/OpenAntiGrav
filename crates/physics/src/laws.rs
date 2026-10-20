//! The force-law constants a title changes from Pulse's, carried per craft.
//!
//! Pulse's values are the default. Wipeout Pure's craft update is Pulse's in shape and differs
//! in a handful of constants, read off its `BOOT.BIN` and measured live
//! (`docs/physics/pure-physics.md`). The race passes a title's own set in
//! [`crate::Environment::laws`], from `oag_title::craft_laws::CraftLaws`: a title's constants
//! are not per-craft state, so they live outside the hashed world.

/// One title's force-law constants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Laws {
    /// `Ship_UpdateEngine` doubles its output: `T = T * craft+0x294 * 2.0` on Pulse
    /// ([`crate::engine::ENGINE_OUTPUT_DOUBLE`]). Pure's engine (`0x0892f1f0`) has no such
    /// factor, measured live on 374 frames to `3e-6`.
    pub engine_output_double: bool,
    /// The grounded quadratic drag coefficient: Pulse's [`crate::passive::DRAG_GROUND`],
    /// Pure's [`crate::passive::DRAG_GROUND_PURE`].
    pub drag_ground: f32,
    /// The two-point hover's bank-to-yaw gain: Pulse's [`crate::hover::BANK_TO_YAW_GAIN`]
    /// (`30`), Pure's [`crate::hover::BANK_TO_YAW_GAIN_PURE`] (`50`).
    pub bank_to_yaw: f32,
    /// The bank-to-yaw gain a Zone craft ([`crate::ShipState::four_corner`]) flies: Pulse's
    /// and Pure's four-corner hovers load `50` ([`crate::hover::BANK_TO_YAW_GAIN_FOUR_CORNER`]);
    /// Wipeout HD's Zone runs its racing `Craft_HoverFourPoint`, whose epilogue loads `30`
    /// (`0x008a9198`, `docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md`).
    pub bank_to_yaw_zone: f32,
    /// The pitch damping while the craft is in the grid state ([`crate::ShipState::on_grid`]),
    /// in place of the racing `pitch_damping`. Pulse's is the racing value throughout
    /// (`None`). Pure's `Ship_ApplyAngularDamping` (`0x0892cc2c`) damps pitch at `3 / c` with
    /// `c` the roll coefficient, `-5` on the grid and `-2` racing, so `0.6` on the grid
    /// against `1.5`: measured live to `2e-6` (`docs/physics/pure-physics.md`).
    pub pitch_damping_grid: Option<f32>,
    /// The craft-wall restitution `e` in `j = -(1 + e) vn / D`: Pulse's
    /// [`crate::wall::BODY_RESTITUTION`] (`0.4`, `body+0x388`). Wipeout HD reads it from its
    /// pair table, which the global `handlingstats.xml`'s `<Collision shipWallCofR>`
    /// overwrites to `0.8`, measured live on 26 of 26 contact records
    /// (`docs/ghidra/functions/ps3-hdfury-eu/wall-response.md`). Data, not a title's const:
    /// the race passes the file's value where it authors one.
    pub wall_restitution: f32,
    /// A **wall** contact whose point already moves out (`vn > 0`) gets nothing: no impulse,
    /// no push-out, no record. Pulse PSP's `Body_ResolveContact` has no such test (a
    /// two-sided, "sticky" contact, [`crate::wall`]); Wipeout HD's returns at `0x000f7880`,
    /// measured live on wall contacts (`docs/ghidra/functions/ps3-hdfury-eu/wall-response.md`).
    /// A floor contact stays two-sided, as [`Self::restitution_against`] keeps Pulse's there.
    pub wall_one_sided: bool,
    /// Each resolved wall contact queues [`crate::wall::steer_away_torque`] for the next
    /// step's integration: Wipeout HD's. Pulse's resolver ends at the record.
    pub wall_steer_away: bool,
    /// A wall contact's point velocity is `v + (R^T omega) x r`, the craft's body-local rate
    /// crossed with a world lever arm, where Pulse's is the textbook `v + omega x r`. Wipeout
    /// HD's `Body_ResolveContact` reads its stored rate (negated, body-local) straight into
    /// `cross(r, rate)`; the two agree for a level craft and part with pitch or roll.
    pub wall_point_rate_body_local: bool,
}

impl Laws {
    /// The restitution a contact against `surface` takes. [`Self::wall_restitution`] is
    /// measured on walls alone (HD's material `3`, 26 of 26 records): a floor or reset
    /// surface keeps Pulse's [`crate::wall::BODY_RESTITUTION`], as nothing measured says what
    /// HD's pair table gives it (`(0, 2)` and `(0, 4)` read `1.0`, a pair outside it `0.0`).
    #[must_use]
    pub fn restitution_against(&self, surface: crate::Surface) -> f32 {
        if surface == crate::Surface::Wall {
            self.wall_restitution
        } else {
            crate::wall::BODY_RESTITUTION
        }
    }

    /// Pulse's laws, every title's unless it measured its own.
    pub const PULSE: Self = Self {
        engine_output_double: true,
        drag_ground: crate::passive::DRAG_GROUND,
        bank_to_yaw: crate::hover::BANK_TO_YAW_GAIN,
        bank_to_yaw_zone: crate::hover::BANK_TO_YAW_GAIN_FOUR_CORNER,
        pitch_damping_grid: None,
        wall_restitution: crate::wall::BODY_RESTITUTION,
        wall_one_sided: false,
        wall_steer_away: false,
        wall_point_rate_body_local: false,
    };
}

impl Default for Laws {
    fn default() -> Self {
        Self::PULSE
    }
}
