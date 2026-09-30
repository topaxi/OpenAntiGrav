//! One live particle.

use oag_core::math::Vec3;

/// One live particle. Dead when [`Particle::life`] reaches zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Particle {
    pub(super) position: Vec3,
    pub(super) velocity: Vec3,
    /// The streak's other end: the spawn point under
    /// [`super::Render::Streak::from_spawn`], the previous tick's position
    /// otherwise - the particle-row-`+0x50` mechanism of
    /// `ParticleSystem_UpdateParticles`.
    pub(super) origin: Vec3,
    pub(super) life: f32,
    pub(super) max_life: f32,
    /// Which [`super::Effect::emitters`] entry authored it.
    pub(super) spec: u16,
    /// The palette entry drawn at spawn, under [`super::ColourMode::RandomEntry`].
    pub(super) colour_index: u8,
    /// The `0..=1` samples the two [`oag_vex::pob::ChannelMode::Random`] channels draw
    /// once at spawn; unused for the other modes.
    pub(super) size_sample: f32,
    pub(super) alpha_sample: f32,
    /// The system's scale at spawn - the original's severity, which
    /// multiplies both speed and drawn size.
    pub(super) scale: f32,
    /// Which cell of the emitter's [`super::Atlas`] it draws - see [`super::sprite`].
    pub(super) frame: u16,
    /// [`Particle::frame`] before its floor, as [`super::frames`] advances it.
    pub(super) frame_at: f32,
    /// A template's first draw is at age 0 - see [`super::template`].
    pub(super) fresh: bool,
    /// A rotating particle's angle and which way it turns, `1` or `-1` - see
    /// [`super::roll::Rotation`].
    pub(super) roll: f32,
    pub(super) turn: f32,
    /// The once-drawn sample of a random roll channel - see
    /// [`super::roll::Rotation::start`].
    pub(super) spin_sample: f32,
}

impl Particle {
    pub(super) const DEAD: Self = Self {
        position: Vec3::ZERO,
        velocity: Vec3::ZERO,
        origin: Vec3::ZERO,
        life: 0.0,
        max_life: 0.0,
        spec: 0,
        colour_index: 0,
        size_sample: 0.0,
        alpha_sample: 0.0,
        scale: 0.0,
        frame: 0,
        frame_at: 0.0,
        fresh: false,
        roll: 0.0,
        turn: 1.0,
        spin_sample: 0.0,
    };

    pub(super) fn alive(self) -> bool {
        self.life > 0.0
    }
}
