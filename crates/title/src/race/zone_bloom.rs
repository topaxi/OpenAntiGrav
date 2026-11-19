//! The `HDR and Bloom` values a title's engine writes over the circuit's own in a Zone race.

/// What a Zone race's post chain runs with in place of the circuit's authored `HDR and
/// Bloom` values.
///
/// HD/Fury's frame preparation writes these into the settings block every frame of a
/// Zone race, from a table in the executable, so a circuit's own `.envsettings` values for
/// them never reach the picture there. The keys it leaves alone (the adaption rate and the
/// `Tone` family) stay the circuit's.
///
/// # Why this is an axis
///
/// The table is one executable's. 2048 and Omega end their frame in other chains, whose
/// Zone values are unread, and Pulse and Pure have no such block: they carry `None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoneBloom {
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
