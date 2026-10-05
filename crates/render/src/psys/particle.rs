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
    /// The `0..=1` samples the two [`oag_pob::ChannelMode::Random`] channels draw
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
    /// The spawn tick does not age or move it, so its first draw is at age 0,
    /// at its spawn point.
    ///
    /// **Measured for an emitter's particles as well as a template's**
    /// ([`super::template`]): `WO_ROCKET_FLARE` read live on PPSSPP 2026-10-01
    /// (`psp-weapon-pair.py --probe flare` and `--probe rolled`, the pool read in
    /// `FUN_089194d0` and `ParticleSystem_DrawRolledQuads`). The flare's first
    /// draw is `size 1.05`, the size channel's `lo + (hi - lo) * 0.0068`, then
    /// `1.28`, `1.51`. `WO_ROCKET_SHAZZAM` fixes the life: it authors `1 +- 1`
    /// ticks and, of 39 particles read, 22 were drawn twice and 17 once - a life
    /// drawn from `0..2`, dead the update it runs out. Before this the spawn tick
    /// integrated the particle too, so a life of one tick or less was never drawn
    /// and the Rocket's big white `WO_ROCKET_SHAZZAM` flash showed on about every
    /// other tick instead of every one.
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

/// The index of a dead particle, or the one with the least life left.
///
/// Ascending scan, so the choice depends only on the pool's own state.
pub(super) fn expendable_slot(particles: &[Particle]) -> usize {
    let mut best = 0;
    let mut best_life = f32::INFINITY;
    for (i, particle) in particles.iter().enumerate() {
        if !particle.alive() {
            return i;
        }
        if particle.life < best_life {
            best_life = particle.life;
            best = i;
        }
    }
    best
}
