//! Positional audio: what a world position does to a cue's level and its
//! place between the speakers.
//!
//! Every number here is read out of the original. The evidence page is
//! [`positional-audio.md`](../../../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md);
//! this module is the port, and it deliberately repeats none of that page's
//! reasoning - only its results, each beside the code that uses it.
//!
//! # Why this is a module and not three lines in the mixer
//!
//! [`crate::mixer`] is a voice pool: it knows gains, not geometry. The law
//! below is pure arithmetic over two positions and one axis, with no device, no
//! voice and no allocation, which makes it checkable against the original's own
//! numbers on its own. That is the same split the crate already makes between
//! the mixer and [`crate::output`].
//!
//! # The law
//!
//! ```text
//! d     = |listener - emitter|
//! atten = (radius - d) / radius            linear, NOT inverse-square
//! atten = min(atten * 1.25, 1.0)           so full volume out to 0.2 * radius
//! gain  = d > radius ? silent : curve(atten * volume)
//! curve = pow(v, 1/1.7), quantised to 256 steps
//!
//! pan   = dot(normalize(emitter - listener), listener_right)   in -1..=1
//! L     = sqrt((1 - pan) / 2)              equal power
//! R     = sqrt((1 + pan) / 2)
//! ```
//!
//! # Three things about it that are not what you would have written
//!
//! **Beyond the radius a sound is gated off, not merely quiet.** The original
//! tests `d > radius` separately from the attenuation, and `Sound_Play` refuses
//! to *start* a cue on an emitter that is out of range at all. So
//! [`Emitter::place`] returns [`None`] rather than a tiny gain, and the caller
//! is expected to not play the sound.
//!
//! **The falloff is linear and then flat.** `atten * 1.25` clamped at one means
//! an emitter is at full level out to a fifth of its radius and falls linearly
//! to nothing at the radius. A craft is therefore un-attenuated within 40 units
//! and silent past 200.
//!
//! **A centred sound is 0.707 in each channel, not 1.0.** That is what the
//! original's own pan table says at its midpoint, and it is why
//! [`crate::mixer::Play::pan`] is an [`Option`]: a voice with no recovered
//! position - music, a movie, an announcer line - is left alone rather than
//! being quietly pulled down 3 dB by a law that was never applied to it.
//!
//! # The cone is recovered and deliberately absent
//!
//! The emitter record carries a cone: a half-angle at `+0x40` defaulting to
//! `pi/2`, the angle to the listener at `+0x48`, an enable byte at `+0x4c`, and
//! the attenuation multiplies by `1 - angle / half_angle` when it is set.
//! **Nothing found so far turns it on** - the `.vex` classes `soundcone`
//! `0x3e9` and `speaker` `0x3cc` are the suspects and neither has been read. An
//! effect whose trigger is not recovered stays unwired, so it is written down
//! here and not implemented.

/// Where the ears are.
///
/// The original's listener is the **active camera**, copied into the sound
/// manager once a frame off the same global `DAT_08ab10b0` that gates the
/// exhaust ribbon. It is not the player's craft: an external view hears from
/// behind and above the hull, which is audible when a chase camera swings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Listener {
    /// World-space position of the ear, which is the camera's eye.
    pub position: [f32; 3],
    /// The camera's world-space **right** axis, expected normalised.
    ///
    /// The original reads it as column 0 of the world-to-camera rotation it
    /// stores, which is the same vector as the camera world matrix's first
    /// column. `docs/.../camera.md` measured that transposition independently
    /// from the rendering side.
    pub right: [f32; 3],
}

impl Listener {
    /// A listener at the origin facing down `-Z` with `+X` to its right.
    ///
    /// The identity case, for tests and for a caller with no camera yet.
    #[must_use]
    pub fn at_origin() -> Self {
        Self {
            position: [0.0; 3],
            right: [1.0, 0.0, 0.0],
        }
    }
}

/// Something in the world that makes a noise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Emitter {
    /// World-space position, which the original reads off the owning scene
    /// node's transform every frame rather than storing.
    pub position: [f32; 3],
    /// How far away it can still be heard. Past this it is silent, not quiet.
    pub radius: f32,
}

impl Emitter {
    /// `SoundEmitter_Init`'s default, and what a racing craft keeps.
    ///
    /// `Craft_Construct_q` (`0x088417d0`) allocates the emitter, links it and
    /// points it at the ship's scene node without ever writing `+0x38`. So
    /// every contact, absorb and pad crossing in the field carries this radius.
    pub const CRAFT_RADIUS: f32 = 200.0;

    /// What the engine note carries instead.
    ///
    /// `ExhaustFlare_Init` (`0x08905308`) overrides `+0x38` with `50.0`, which
    /// is the whole reason a passing opponent's engine is so much quieter than
    /// its collisions.
    pub const ENGINE_RADIUS: f32 = 50.0;

    /// An emitter at `position` with the craft radius.
    #[must_use]
    pub fn craft(position: [f32; 3]) -> Self {
        Self {
            position,
            radius: Self::CRAFT_RADIUS,
        }
    }

    /// An emitter at `position` with the engine-flare radius.
    #[must_use]
    pub fn engine(position: [f32; 3]) -> Self {
        Self {
            position,
            radius: Self::ENGINE_RADIUS,
        }
    }

    /// Places this emitter for `listener`, given the cue's own volume.
    ///
    /// `volume` is the level the call site asks for, before any distance work -
    /// the original's first argument to `Sound_Play`, which every recovered
    /// call site passes as `1.0`. It is a separate argument rather than folded
    /// into the result so a caller with its own law - the engine note's
    /// `intensity * 0.6 + 0.4`, say - multiplies *before* the volume curve, the
    /// way the original does.
    ///
    /// Returns [`None`] when the emitter is past its radius. That is the
    /// original's own gate and it is a refusal to play, not a zero gain.
    #[must_use]
    pub fn place(&self, listener: &Listener, volume: f32) -> Option<Placed> {
        let to_source = [
            self.position[0] - listener.position[0],
            self.position[1] - listener.position[1],
            self.position[2] - listener.position[2],
        ];
        let distance = dot(to_source, to_source).sqrt();
        if self.radius.is_nan() || self.radius <= 0.0 || distance > self.radius {
            return None;
        }
        // `(radius - d) / radius`, then the 1.25 that flattens the near fifth.
        let atten = ((self.radius - distance) / self.radius * NEAR_FIELD_BOOST).min(1.0);
        Some(Placed {
            gain: volume_curve(atten * volume),
            pan: pan_of(to_source, listener.right),
        })
    }
}

/// What an emitter comes out as: one gain and one position between the
/// speakers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// Linear gain, already through the volume curve.
    pub gain: f32,
    /// `-1.0` hard left, `0.0` centred, `+1.0` hard right.
    pub pan: f32,
}

/// The multiply that flattens the near field: `atten * 1.25`, clamped at one.
///
/// Read directly off `SoundEmitter_ComputeVolumeAndAngle` (`0x08939c00`). Its
/// consequence is the `0.2 * radius` full-volume ball, which is worth naming
/// because nothing in the source says `0.2` anywhere.
pub const NEAR_FIELD_BOOST: f32 = 1.25;

/// The exponent's denominator in the volume curve, `mgr+0x168`.
///
/// `SoundManager_BuildVolumeCurve` (`0x0893a7e8`) fills 256 floats with
/// `powf(i / 255, 1 / gamma)`, and the manager's constructor sets `gamma` to
/// this. A gamma above one *raises* every partial level, so the falloff is much
/// gentler than the linear attenuation alone would suggest.
pub const VOLUME_GAMMA: f32 = 1.7;

/// How many steps the original's volume curve is tabulated at.
pub const VOLUME_STEPS: u16 = 256;

/// The original's volume curve, quantisation included.
///
/// The 256-step table is reproduced rather than smoothed over: the index is
/// `(int)(v * 255)`, **truncated**, so two levels a four-hundredth apart come
/// out identical, exactly as they do on the console. Computing `powf` per call
/// instead of holding the table is a deliberate simplification of the *storage*
/// and not of the result - the arithmetic is the same, and a cue starts a
/// handful of times a second.
#[must_use]
pub fn volume_curve(volume: f32) -> f32 {
    let steps = f32::from(VOLUME_STEPS - 1);
    let index = (volume.clamp(0.0, 1.0) * steps) as u16;
    if index == 0 {
        // The builder writes entry 0 as a literal zero before its loop starts
        // at 1, so silence is exactly silence and not `powf(0.0, x)`.
        return 0.0;
    }
    (f32::from(index) / steps).powf(1.0 / VOLUME_GAMMA)
}

/// The two speaker gains for a pan position, equal power.
///
/// `[left, right]`. The original tabulates this as 180 `(s16, s16)` pairs of
/// `floor(16383 * cos)` and `floor(16383 * sin)` at half-degree steps, which
/// is `cos`/`sin` of a quarter turn and nothing else - all 360 values match to
/// the unit. Two `-90`/`+90` rotations in the chain cancel exactly, so what is
/// left is the half-angle form below, and the half-angle identities remove the
/// `acos` the original's own code performs. **One square root, no
/// transcendental** - see `docs/architecture/determinism.md` for why that is
/// worth having even outside the simulation.
#[must_use]
pub fn pan_gains(pan: f32) -> [f32; 2] {
    let pan = pan.clamp(-1.0, 1.0);
    [((1.0 - pan) * 0.5).sqrt(), ((1.0 + pan) * 0.5).sqrt()]
}

/// The rightward component of the direction to a source, in `-1..=1`.
fn pan_of(to_source: [f32; 3], right: [f32; 3]) -> f32 {
    let length = dot(to_source, to_source).sqrt();
    if length <= 0.0 {
        // On top of the listener there is no direction to project. The original
        // divides by a reciprocal it guards to 1.0 in the same case, which
        // leaves the dot product at whatever the unnormalised vector gives -
        // zero, for a zero vector. Centred.
        return 0.0;
    }
    let inv = 1.0 / length;
    dot(
        [to_source[0] * inv, to_source[1] * inv, to_source[2] * inv],
        right,
    )
    .clamp(-1.0, 1.0)
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[cfg(test)]
mod tests;
