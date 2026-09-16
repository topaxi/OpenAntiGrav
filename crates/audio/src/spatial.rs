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
//! # The cone, wired
//!
//! The emitter record carries a cone: a half-angle at `+0x40` defaulting to
//! `pi/2`, the angle to the listener at `+0x48`, an enable byte at `+0x4c`, and
//! the attenuation multiplies by `1 - angle / half_angle` when it is set,
//! **before** the `1.25` near-field boost - so a source outside its cone is
//! not merely dimmed, the multiply can and does go negative and the final
//! `volume_curve` clamp (below) is what turns that into silence, exactly the
//! way `SoundManager_VolumeCurve`'s own `[0,1]` clamp does for the original.
//!
//! **`soundcone` `0x3e9` is the class that carries the flag** - 134 nodes
//! across eight circuits, each carrying two angles that are whole degrees and a
//! `u8` at `+0x08` that is `1` on every cone and `0` on all 1,164 plain
//! `sound` nodes. `speaker` `0x3cc` has a registered class and **no instance
//! anywhere on the Pulse disc**, so it is not a suspect for anything.
//! `docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md` has the
//! layout.
//!
//! **Which authored angle feeds `+0x40` is now read.** `VexSoundCone_Init`
//! (`0x08925ff4`, see the evidence page's own section) is a four-line wrapper
//! around `VexSound_Init`: it calls it unchanged and then writes the node's
//! own `+0x00` (the varying angle, `40` to `120` degrees) to the emitter's
//! `+0x40` half-angle and `+0x08` (the enable byte) to `+0x4c`. So
//! `oag_vex::sound_emitters::Cone::angle_a` - already `Cone::wide()`,
//! since it is never smaller than `angle_b` on any of the 134 authored cones -
//! is the half-angle this module now uses, not a guess between the two.
//!
//! `angle_b` (`+0x04`, `40` degrees on every cone) writes to the emitter's
//! `+0x44` - a field this project has not seen anything read back, the same
//! standing as the already-documented `+0x3c` - so it is decoded and named,
//! not used here.
//!
//! The cone's own axis is the emitter's world matrix, raw and unnormalised,
//! exactly as `SoundEmitter_Update` (`0x08939720`) reads it: `node[0x10..0x1c]`,
//! row `1` of the same 64-byte matrix `oag_vex::sound_emitters::SoundEmitter::to_world`
//! already carries. Nothing here renormalises it - the original does not,
//! either, and `positional-audio.md`'s own law clamps the resulting dot
//! product into `-1..=1` rather than trusting the row's length.

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
    /// The directional cone, where the node authors one.
    ///
    /// `None` for every craft, engine flare and omnidirectional `sound` node -
    /// `SoundEmitter_Init`'s own default is cone-disabled, and nothing but a
    /// `soundcone` node ever sets the enable byte. See this module's own
    /// header for the law and where the two angles come from.
    pub cone: Option<Cone>,
}

/// A `soundcone` node's directional falloff.
///
/// See this module's own header for the law and the evidence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cone {
    /// The cone's axis, as the emitter's own world matrix carries it - row `1`
    /// of `oag_vex::sound_emitters::SoundEmitter::to_world`, raw and
    /// unnormalised. The dot product this feeds is clamped, not the vector.
    pub axis: [f32; 3],
    /// Half-angle, radians. `VexSoundCone_Init` writes the node's own `+0x00`
    /// angle here - the wider of the two authored, `Cone::wide()` in
    /// `oag_vex::sound_emitters::Cone`.
    pub half_angle: f32,
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
            cone: None,
        }
    }

    /// An emitter at `position` with the engine-flare radius.
    #[must_use]
    pub fn engine(position: [f32; 3]) -> Self {
        Self {
            position,
            radius: Self::ENGINE_RADIUS,
            cone: None,
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
        // `(radius - d) / radius`, the cone term where there is one, and only
        // then the 1.25 that flattens the near fifth - in that order, matching
        // `SoundEmitter_ComputeVolumeAndAngle`'s own sequence.
        let mut atten = (self.radius - distance) / self.radius;
        if let Some(cone) = self.cone {
            atten *= cone_factor(to_source, distance, cone);
        }
        let atten = (atten * NEAR_FIELD_BOOST).min(1.0);
        Some(Placed {
            gain: volume_curve(atten * volume),
            pan: pan_of(to_source, listener.right),
            distance,
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
    /// How far the ear is from the source, the input to [`Doppler`].
    pub distance: f32,
}

/// The per-instance doppler scale, `inst+0x0c`.
///
/// `SoundInstance_Init` (`0x08939b10`) writes `0x3a03126f` - `0.0005` - into
/// every instance `Sound_Play` builds, and the live capture of 2026-09-01 read
/// the same value on all 1,610 samples. One scale for every cue, so this is a
/// constant rather than a field.
pub const DOPPLER_SCALE: f32 = 0.0005;

/// Pitch units to the octave: the offset `SoundInstance_UpdateSpatial` writes
/// is in 1/128 of a semitone, which `Scream_ComputeVoiceNote` scales through a
/// `2^(i/1536)` table. See `sound.md`'s pitch section.
pub const PITCH_UNITS_PER_OCTAVE: f32 = 1536.0;

/// How far the listener may move in one frame before the doppler term is
/// suppressed: `mgr+0x94`, `24.0`, read off the manager's constructor.
///
/// A camera cut is not a velocity. `SoundManager_Update` compares the new
/// listener position against the previous frame's and clears `mgr+0x8d` when
/// the jump exceeds this, so every instance's pitch holds its base for that
/// frame instead of sweeping through an octave.
pub const LISTENER_JUMP: f32 = 24.0;

/// One held voice's doppler state: the distance it was heard at last frame.
///
/// `SoundInstance_UpdateSpatial` (`0x08939e58`):
///
/// ```text
/// pitch = -(distance_change / dt) * 0.0005 * 1536 + base_pitch
/// ```
///
/// in pitch units, where `distance_change` is the emitter's `+0x34`, this
/// frame's distance less last frame's. Receding raises the change and lowers
/// the pitch; a source closing at 100 units a second plays `2^0.05`, about
/// 3.5 % sharp. Confidence 85 for the law as a whole.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Doppler {
    last_distance: Option<f32>,
}

impl Doppler {
    /// The pitch ratio for this frame, from how the distance moved since the
    /// last one. The first frame, and any frame with `enabled` clear, is
    /// unity - the distance is recorded either way, so the frame after a
    /// suppressed one measures one frame's change and not two.
    pub fn ratio(&mut self, distance: f32, dt: f32, enabled: bool) -> f32 {
        let change = self.last_distance.map_or(0.0, |last| distance - last);
        self.last_distance = Some(distance);
        if !enabled || dt <= 0.0 {
            return 1.0;
        }
        (-(change / dt) * DOPPLER_SCALE).exp2()
    }

    /// Forgets the last distance, for a voice that closed or went out of
    /// range - the next frame it is heard starts from unity again.
    pub fn reset(&mut self) {
        self.last_distance = None;
    }
}

impl Listener {
    /// Whether the ear moved further than [`LISTENER_JUMP`] since `previous`,
    /// which is the frame the doppler term is held off on.
    #[must_use]
    pub fn jumped_from(&self, previous: &Listener) -> bool {
        let moved = [
            self.position[0] - previous.position[0],
            self.position[1] - previous.position[1],
            self.position[2] - previous.position[2],
        ];
        dot(moved, moved).sqrt() > LISTENER_JUMP
    }
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

/// `1 - angle / half_angle`, unclamped - the caller's own `min(1.0)` and
/// [`volume_curve`]'s `[0,1]` clamp are what turn a source outside the cone
/// into silence, the same way the original's own arithmetic does.
///
/// `to_source` is `emitter - listener`, the opposite sign of
/// `SoundEmitter_Update`'s own `e[0x10]` (`listener - emitter`); the law reads
/// `dot(normalize(e[0x10]), -axis)`, and the two negations cancel, so this
/// takes `dot(normalize(to_source), axis)` directly.
fn cone_factor(to_source: [f32; 3], distance: f32, cone: Cone) -> f32 {
    if cone.half_angle.is_nan() || cone.half_angle <= 0.0 {
        // Not authored on this disc - every one of the 134 cones is `40` to
        // `120` degrees - but a zero or negative half-angle would otherwise
        // divide into `NaN`/`inf` rather than a value a mixer can clamp.
        return 1.0;
    }
    let angle = if distance <= 0.0 {
        // On top of the emitter there is no direction to measure the cone
        // from - dead centre of every cone there is.
        0.0
    } else {
        let inv = 1.0 / distance;
        let direction = [to_source[0] * inv, to_source[1] * inv, to_source[2] * inv];
        oag_core::math::acos(dot(direction, cone.axis).clamp(-1.0, 1.0))
    };
    1.0 - angle / cone.half_angle
}

/// `Scream_PanVolumePair`'s four terms, and the extra gain they contribute on
/// top of everything above.
///
/// `Scream_PanVolumePair` (`0x08995a9c`) sits between
/// [`Emitter::place`]'s own `0..1024` volume and the two hardware channel
/// volumes, and multiplies **four** terms together, two of them squared under
/// a mode mask read as `10` (mask bit 1 and bit 3): `p1 * p2 * p4 * p6 * 0x102
/// / 0x7f^3`. **This whole stage was absent from this crate before it - grep
/// found nothing** - and it is a real, measured attenuation: read live
/// (`memory.disasm` on a PPSSPP breakpoint, full-throttle engine, full grid),
/// it cost `4.4` to `13.1 dB`, mean `11.7 dB`, at real gameplay values - not
/// the `2x` *gain* its own theoretical ceiling would have suggested.
///
/// # Which two terms are squared, and how that is known
///
/// **Confirmed two independent ways**, agreeing exactly: statically, reading
/// `Scream_PanVolumePair`'s own instructions rather than its decompile (the
/// project's own `lv.q`/`sv.q` trap does not apply here - this function is
/// pure scalar `mult`/`div`, no VFPU at all) shows the mask bit for each of
/// the four terms directly: bit 0 gates squaring the first argument, bit 1
/// the second, bit 2 the fourth, bit 3 a fifth value already resident in a
/// register at the function's own entry. Mask `10` is `0b1010`: bits 1 and 3
/// set, bits 0 and 2 clear - so the **second and fifth** arguments are
/// squared and the first and fourth are not. That matches a live capture from
/// the same day, reading the same mask off the stack at a running
/// breakpoint and the same four terms off `cpu.getAllRegs`.
///
/// # What each term is, traced to its caller
///
/// - **The first term (unsquared)** is `Emitter::place`'s own `0..1024`
///   volume, scaled to `0..127` - the SCREAM voice's `+0x3a` field,
///   traced through `Scream_SetSoundVolume` (`0x0898d614`) and
///   `Scream_StartSound` (`0x0898f864`). Already computed by this crate; not
///   a new input.
/// - **The second term (squared)** is the SCREAM voice's `+0xc` field,
///   traced to `Scream_StartSound`'s own third argument, which defaults
///   (when `-1`) to the **cue's own authored byte at offset `0x00`** -
///   [`oag_formats::sblk::Cue::volume`]. **Confirmed live**: a PPSSPP
///   breakpoint on `Scream_StartSound`'s entry read that argument as exactly
///   `-1` on 20 of 20 hits during a real Time Trial, and a second breakpoint
///   at the voice's own `+0xc` read back an exact match against the cue's
///   own byte on every hit taken - not inferred across sessions, the same
///   tick.
/// - **The fourth term (unsquared)** is the SCREAM voice's `+0x10` field,
///   unconditionally initialised to `0x7f` (its ceiling) by
///   `Scream_StartSound` and adjusted afterwards by an interpolator this
///   project has not traced. Every live sample taken - twenty breakpoint hits
///   across two sessions - read exactly `0x7f`. **Ported as the constant
///   `1.0`**: measured at construction and in every sample since, with the
///   stated gap that an explicit fade-out would move it and this port does
///   not model that.
/// - **The fifth value (squared)** is the waveform descriptor's own authored
///   byte at offset `0x01` - [`oag_formats::sblk::Sound::volume`], traced
///   through `Scream_OpKeyOn` (`0x0898fc78`).
///
/// # The constant collapses
///
/// In the original's fixed-point arithmetic the combine is
/// `a0 * a3 * t1_sq * a1_sq * 0x102 / 0x7f^5`(three integer divisions folded
/// into one exponent here; see the doc page for the instruction-level
/// derivation). Normalising each `0..127` term to `0..1` and carrying the
/// squaring through, `0x7f * 0x102 / 0x3fff` (the `Scream_PanVolumePair`
/// caller's own final division into a hardware gain) is **exactly `2.0`,
/// with no remainder** - matching the doc's independently-stated theoretical
/// ceiling of `2.0` for all four terms at their own maximum. So the whole
/// stage is
///
/// ```text
/// extra_gain = 2 * cue_volume^2 * sound_volume^2
/// ```
///
/// with the fourth (`+0x10`) term folded into the leading `2` as its
/// measured-constant `1.0`, and [`Emitter::place`]'s own gain applied by the
/// caller rather than here (it is the *first* term, unsquared, and this
/// crate already produces it in `0..1` form with nothing left to convert).
///
/// See `docs/ghidra/functions/psp-pulse-usa/positional-audio.md`'s
/// "`Scream_PanVolumePair`'s four terms" section for the full derivation,
/// the live capture tables and the corpus survey (36 banks, 582 cues, 880
/// waveform bindings, `Cue::volume` in `20..=127` and `Sound::volume` in
/// `60..=127` on every one of them - the `-1..=-5` sentinel codes the same
/// page documents for both bytes are real but do not arise anywhere in this
/// disc's own data).
#[must_use]
pub fn pan_volume_gain(cue_volume: i8, sound_volume: i8) -> f32 {
    let a1 = f32::from(cue_volume.unsigned_abs()) / 127.0;
    let t1 = f32::from(sound_volume.unsigned_abs()) / 127.0;
    2.0 * a1 * a1 * t1 * t1
}

#[cfg(test)]
mod tests;
