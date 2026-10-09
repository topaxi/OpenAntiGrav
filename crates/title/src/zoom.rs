//! HD/Fury's **boost and damage zoom-streak ring** (`FunkLayerZoom`), as a title answers it.
//!
//! Only one title has read it: Wipeout HD/Fury re-draws a quarter-resolution
//! history of the scene over the screen's periphery while a pulse runs, the
//! ship boosting (`E`) or taking damage (`P`). It is **not a motion blur**
//! (nothing depends on velocity), so it has no setting value of its own. Every
//! number here is one the executable computes or one read off a live frame;
//! `docs/ghidra/functions/ps3-hdfury-eu/funklayer-zoom.md` carries the evidence
//! and a confidence per number.
//!
//! `None` on [`crate::RaceDefaults::zoom_ring`] means the title draws no such
//! ring, or that its own has not been read: Pulse and Pure are unread by this
//! lane, 2048 and Omega are unread (Omega authors a `Tonemap` block, not HD's
//! bloom chain, see the page's "Other titles").

/// The ring's geometry, the history draw's laws and the two pulses' laws.
///
/// Plain data, no behaviour: `oag_post::hd_zoom` reads the same numbers into
/// its own `Tuning` and steps the pulses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomRing {
    /// The history draw's alpha per unit of `E`: `10 * FunkLayer[0]`, with
    /// `FunkLayer[0] = 0.025` on Talon's Junction and Metropia.
    pub history_weight: f32,
    /// The cap on that alpha, the constant at `0x008b74e8`.
    pub history_weight_cap: f32,
    /// The history sample's crop per unit of `E`: the quarter-resolution
    /// history is read at `uv * (1 - 2c) + c` with `c = history_crop * E`.
    pub history_crop: f32,
    /// Radius, in NDC, of the ring's inner (transparent) edge.
    pub inner_radius: f32,
    /// Radius, in NDC, of the ring's outer (opaque) edge.
    pub outer_radius: f32,
    /// Quads around the ring.
    pub segments: u32,
    /// How far each outer vertex's texture coordinate is pulled toward the
    /// inner vertex of the same angle, for the two taps (`0.15` and `0.95`).
    pub tap_pull: [f32; 2],
    /// The boost pulse, `E`.
    pub boost: BoostPulse,
    /// The damage pulse, `P`.
    pub damage: DamagePulse,
    /// The ring's size jitter, `A = retain * A + (rand % range) * step * E`.
    pub jitter: SizeJitter,
}

/// The boost pulse `E`, fired by a speed pad or a Turbo.
///
/// `A0` starts at [`Self::start`] and falls by `dt * fall_rate`; while it is
/// above zero, `E = clamp(gain * ((1 - slope * A0) - offset), 0, 1)`; once it is
/// zero `E` falls by [`Self::decay_per_update`] **per update**, not per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoostPulse {
    /// The value the trigger writes, `0.8`.
    pub start: f32,
    /// Per second, `8/7`.
    pub fall_rate: f32,
    /// `1.25`.
    pub slope: f32,
    /// `0.075`.
    pub offset: f32,
    /// `5`.
    pub gain: f32,
    /// `0.035` per update once `A0` has run out.
    pub decay_per_update: f32,
}

/// The damage pulse `P`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamagePulse {
    /// The ring's colour is `(1 - tint[0] * P, 1 - tint[1] * P, 1)`.
    pub tint: [f32; 2],
    /// `P` falls by this per second, `1.6667`.
    pub decay_per_second: f32,
}

/// The ring's per-update size jitter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeJitter {
    /// `0.75`.
    pub retain: f32,
    /// `0.0002`.
    pub step: f32,
    /// The random draw's modulus, `100`.
    pub range: u32,
}
