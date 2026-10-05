//! Emitter-record flag bits, `+0x20`.
//!
//! Named from their consumers in the runtime interpreter - see
//! `docs/ghidra/functions/psp-pulse-usa/particle-system.md`. Bits with no
//! constant here were not traced to a consumer.

/// The emitter never stops on its own: [`super::Emitter::duration_ticks`]
/// is not a countdown and the effect runs until whatever owns it says
/// otherwise.
///
/// **Corpus-derived, confidence 75.** The split is clean on both discs -
/// 76 emitters over 35 PSP effects and 41 PS2 ones, with no effect
/// mixing set and clear emitters. Every attached or environmental
/// effect has it: `WO_ROCKET_FLARE`, `WO_MISSILE_HEAD`,
/// `WO_SHURIKEN_HEAD`, `WO_SHURIKEN_TRAIL`, `WO_PLASMA_HEAD`,
/// `WO_LEACHBEAM_*`, `WO_REPULSER`, `WO_QUAKE`, `WO_RAIN`, `WO_SNOW`,
/// `WO_MODESTO_STEAM_*`, and on the PS2 disc `WO_SHIP_ENGINEFLARE` and
/// `WO_UNDERWATER_DEBRIS`. Every impact does not: all five explosions,
/// both bounces, every spark burst, `WO_PLASMA_FLASH`,
/// `WO_TRACK_ROCK_DEBRIS`. `WO_RAIN`, `WO_SNOW` and `WO_BLUE_WELDER`
/// settle it on their own - all three author a **one-tick** duration
/// and none of the three can be a one-tick effect.
///
/// The executable has the matching mechanism, at
/// `ParticleSystem_Update` (`0x088f5b9c`): under instance flag `0x10`
/// at `+0x160` the duration counter at `+0x138` counts *up* by one per
/// update instead of down by the frame's ticks, the emitter-level
/// channel age becomes `counter / 60` rather than `1 - counter /
/// duration`, and the branch that sets the finished bit is skipped
/// entirely. What is **not** traced is the spec-`0x1`-to-instance-`0x10`
/// assignment: the initialiser is behind an unresolved import stub, so
/// the two are joined by the corpus rather than by a read.
/// **Wipeout HD/Fury does not obey the "property of the whole effect" half of
/// this.** Four of its 88 systems mix set and clear emitters in one tree -
/// `WO_QUAKE_DETONATOR_TRAILS`, `WO_CANNON_MUZZLEFLASH`, `WO_MISSILE_HEAD`
/// and `WO_ROCKET_FLARE` - so on that disc the flag is per-emitter and a
/// caller must not read the root's and assume the rest. The bit's *meaning*
/// is untouched by this; only the corpus regularity the two Pulse discs
/// happened to have is.
pub const LOOPING: u32 = 0x1;
/// Spawn offsets and velocities skip the emitter node's matrix - and the draw
/// applies it instead, live (`ParticleSystem_DrawEmitterPool`, `0x08918bf8`), so
/// the particles are kept in the instance's frame and ride it when the owner moves
/// it. **Local space, despite the name**; see `oag_fx::psys::playback`.
pub const WORLD_SPACE: u32 = 0x2;
/// Each particle takes a random initial billboard roll.
pub const RANDOM_ROLL: u32 = 0x4;
/// The rotation speed is sign-flipped on half the particles.
pub const RANDOM_ROTATION_SIGN: u32 = 0x8;
/// A burst's particles are spread along the emitter's motion over the
/// frame rather than all placed at its current position.
pub const SUBFRAME_SPREAD: u32 = 0x20;
/// [`Emitter::gravity_per_tick2`] is applied. **Dormant when clear** -
/// the field still holds an authored value on emitters that never use
/// it.
pub const GRAVITY: u32 = 0x200;
/// Particles never expire.
pub const IMMORTAL: u32 = 0x800;
/// The spawn direction covers the sphere by a Fibonacci spiral instead
/// of sampling it randomly.
pub const UNIFORM_SPHERE: u32 = 0x20_0000;
/// [`Emitter::duration_ticks`] counts bursts, not ticks.
pub const REPEAT_COUNT: u32 = 0x80_0000;
/// A streak's second point stays at the particle's spawn position
/// instead of following it - what makes a burst radiate rather than
/// trail.
pub const STREAK_FROM_SPAWN: u32 = 0x200_0000;
/// Each particle takes a random sprite-atlas frame.
pub const RANDOM_ATLAS_FRAME: u32 = 0x400_0000;
