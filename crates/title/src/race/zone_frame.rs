//! What a title's engine does to a Zone race's frame beyond the stage table.

/// How a Zone race's frame differs from any other race's on the same circuit: the light
/// every chunk is drawn under, and the `HDR and Bloom` values the post chain runs with in
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
}
