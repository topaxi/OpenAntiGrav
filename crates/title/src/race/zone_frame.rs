//! What a title's engine does to a Zone race's frame beyond the stage table.

/// How a Zone race's frame differs from any other race's on the same circuit: the light
/// every chunk is drawn under, the probe tint every craft hull takes, and the `HDR and Bloom` values the post chain runs with in
/// place of the circuit's authored ones.
///
/// HD/Fury's frame preparation does both in one arm, every frame of a Zone race: it
/// switches the lightmap and baked-vertex light sources off for every draw, and it writes
/// the bloom values into the settings block from a table in the executable, so a
/// circuit's own `.envsettings` values for them never reach the picture there. The keys it
/// leaves alone (the adaption rate and the `Tone` family) stay the circuit's.
///
/// # Why this is an axis
///
/// The arm is one executable's. 2048 and Omega end their frame in other chains and light
/// their Zone surfaces by programs of their own, both unread for this, and Pulse and Pure
/// have no such block: they carry `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneFrame {
    /// Every chunk is lit by the stage's ambient and sun alone: no lightmap, no baked
    /// vertex light, whatever the chunk's own material bakes outside a Zone race.
    pub lit_by_stage_alone: bool,
    /// `Bloom from frame contribution`.
    pub frame_contribution: f32,
    /// `Bloom from frame exponent`.
    pub frame_exponent: f32,
    /// `Bloom from alpha contribution`.
    pub alpha_contribution: f32,
    /// `Bloom adaption boost`.
    pub adaption_boost: f32,
    /// `Bloom vertical size`.
    pub vertical_size: f32,
    /// `Bloom horizontal size`.
    pub horizontal_size: f32,
    /// `Bloom feedback`, a key no circuit authors: the resolve's weight of twice its
    /// feedback buffer against the scene.
    pub feedback: f32,
    /// What the showing stage's `Track.Base Colour Highlight` is multiplied by to make the
    /// probe gain every craft hull takes, its probe desaturated to the luma; `None` where
    /// the hull keeps its own tint.
    pub hull_probe_tint_scale: Option<f32>,
    /// Whether a Detonator race runs this table too, **except** `frame_contribution` and
    /// `adaption_boost`, which stay the circuit's there. HD's frame preparation has a mode
    /// arm for Detonator that writes the feedback, sizes, frame exponent and alpha
    /// contribution and skips those two
    /// (`docs/ghidra/functions/ps3-hdfury-eu/zone-frame.md`).
    pub detonator_arm: bool,
    /// A track-flagged chunk takes the stage's `Lighting.Track Fog` in place of its primary fog
    /// (`Scene_GetZoneTrackFog`, `docs/ghidra/functions/ps3-hdfury-eu/zone-plain-chunks.md`,
    /// "The fog pick"). Read on HD's own executable only; 2048 and Omega mark a chunk track by
    /// a shape-name rule and have no measured fog pick.
    pub track_fog: bool,
    /// The original's GL near plane in a Zone circuit race, which the visualiser glow's
    /// `1 - windowDepth` factor is `near / viewDepth` of (`0.0`: use this renderer's own window
    /// depth). Read off the projection recorded in the pairs' `original.camera.view_proj`.
    pub glow_window_near: f32,
    /// The same in a Detonator race, whose camera sits on another field of view.
    pub detonator_glow_window_near: f32,
    /// The visualiser strip's texture unit decodes its RGB with the sRGB curve, so the
    /// glow adds the decoded texel to the linear scene
    /// (`docs/ghidra/functions/ps3-hdfury-eu/zone-vis-gamma.md`, "The law").
    pub glow_vis_srgb: bool,
}
