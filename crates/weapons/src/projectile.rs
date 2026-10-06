//! Things a weapon puts in the air, and what they hit.
//!
//! Each weapon adds a launch rule and an impact rule, not a second flight
//! model. The Mine is the exception: it has no flight model, takes an early
//! return at the top of [`Projectiles::advance`] and shares only the array and
//! the blast. See [`mine`].
//!
//! # What is recovered and what is ours
//!
//! This module is mostly ours.
//!
//! **Recovered.** `<Weapon type="Rocket"><Stats>` authors `damage`,
//! `blastforce`, `blastradius`, `launchSpeed`, `spread` and a flight speed per
//! speed class (confidence 92, `docs/formats/weapon-stats.md`).
//! `Weapon_FireRocket` (`0x0886e104`) spawns three at once, straight and
//! `+-spread` (confidence 88,
//! `docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`). `Ship_Damage`
//! (`0x088439ac`) takes a `source` whose value 2 is a weapon; the weapons-off
//! halving and the clamp are its own (`.../shield.md`).
//!
//! **A projectile follows the floor.** `Rocket_Update` (`0x0885d2a8`) probes
//! [`SURFACE_PROBE_LENGTH`] units toward the surface every tick, rides
//! [`RIDE_HEIGHT`] above it, and falls at [`FALL_ACCELERATION`] when it finds
//! nothing (`.../rocket-visuals.md`). Confirmed in PPSSPP frames. Flying
//! straight, a volley died in the tick it was fired.
//!
//! **Wall versus floor is the surface class of the hit, never its angle.** The
//! original's query returns `0` wall, `1` floor, `3` mag floor, `4` a craft,
//! `0x7f` nothing; this raycaster returns the same tag in
//! `RaycastHit::surface`. See `docs/gameplay/projectile-floor.md`.
//!
//! **Ours.**
//!
//! - Turning the velocity parallel to the surface rather than reflecting it.
//!   The original recomputes velocity from a corrected position; this keeps
//!   speed and follows the track.
//! - The rule is weapon-agnostic, in [`Projectiles::advance`]. A maintainer's
//!   account puts the Missile on the floor too and is less sure about the
//!   Cannon; nothing here has read either, so a Cannon that fires flat needs a
//!   per-weapon flag here.
//! - The axis the fan rotates about. See [`launch`].
//! - A sphere for a hull, wider than the hull on two axes. See [`hull_radius`].
//! - Full damage everywhere inside `blastradius`, with no falloff. See
//!   [`Impact`].
//! - The launch offset and the lifetime cap.
//!
//! # Fixed-size
//!
//! [`MAX_PROJECTILES`] slots of plain `Copy` data inside
//! `oag_gameplay::World`, no `Vec`, so a snapshot stays `memcpy`-shaped
//! (`docs/architecture/adr/0003-no-ecs.md`). A full array drops the shot.

mod blast;
pub mod cannon;
pub mod disruptor;
mod flight;
mod geometry;
mod hit;
pub mod leach_beam;
pub mod mine;
pub mod missile;
pub mod plasma;
pub mod quake;
pub mod repulser;
mod rocket;
pub mod shuriken;

pub use geometry::hull_radius;
use geometry::{SweepHit, nearest_hit};
pub use hit::WeaponHit;

pub use blast::{BlastStats, blast, blast_stats};
pub use mine::TriggerRadii;
pub use rocket::{
    LAUNCH_SPEED_SCALE as ROCKET_LAUNCH_SPEED_SCALE, LIFETIME_SECONDS as ROCKET_LIFETIME_SECONDS,
    ROCKET_SHOTS, fire as fire_rocket, launch,
};

use crate::Craft;
use oag_core::math::{Quat, Vec3};
use oag_physics::Raycaster;
use oag_tables::weapons::Weapon;

/// The most projectiles that can be in the air at once.
///
/// **Ours, deliberately larger than the original's.** The original's pool caps
/// at 48 (`Weapon_FireRocket` checks `live < 0x2e` before three more). A full
/// grid's simultaneous Rocket volley is 24 and surface-following flight keeps
/// shots alive longer, so 16 silently dropped shots. 128 is eight craft with
/// sixteen apiece.
///
/// A laid mine holds its slot for `timetodie` (seven seconds) whatever the craft
/// does: eight craft laying [`mine::CLUSTER`] apiece is forty, a second round
/// inside seven seconds eighty. Re-check against any longer-lived weapon (the
/// Bomb's `timetodie` is twenty seconds). About 48 bytes a slot, 6 KiB total.
pub const MAX_PROJECTILES: usize = 128;

/// How long a projectile flies before it gives up, in seconds.
///
/// **The number is ours; that there is a cap is not.** A shot that leaves the
/// track through a collision gap would hold its slot all race. The slowest
/// class authors `800` km/h, `222` units a second (see [`launch`]), so ten
/// seconds is over two kilometres.
///
/// The original caps a rocket at `5.0` (`RocketPool_Update`, `5.0 < self+0x48`),
/// adopted as [`rocket::LIFETIME_SECONDS`], and the Cannon at `1.0`
/// ([`cannon::LIFETIME_SECONDS`], `CannonPool_Update`). This is the ceiling for
/// weapons with no recovered cap.
pub const MAX_FLIGHT_SECONDS: f32 = 10.0;

/// One thing in the air.
///
/// A free slot has [`Self::kind`] `None`; its other fields are stale and must
/// not be read.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Projectile {
    /// Which weapon fired it, or `None` for a free slot.
    pub kind: Option<Weapon>,
    /// Where it is, in world space.
    pub position: Vec3,
    /// Where it is going, in world units a second. Turned parallel to the surface
    /// each tick it finds one (speed preserved) and pulled down by
    /// [`FALL_ACCELERATION`] each tick it does not.
    pub velocity: Vec3,
    /// The surface normal the projectile is riding, normalised.
    ///
    /// The original keeps one at `self+0x100`, seeded from the firing craft and
    /// re-probed every tick (`Rocket_Update`, `0x0885d2a8`); it is what makes a
    /// projectile follow a banked track.
    ///
    /// [`Projectiles::spawn`] seeds [`Vec3::Y`] instead (**ours**); the first probe
    /// corrects it. The Rocket is seeded from the craft's up through
    /// [`Projectiles::spawn_riding`] (`Rocket_Init` writes `self+0x100` as the
    /// negated `craft+0xb10`); whether that is the craft's up or its contact normal
    /// was not separated.
    pub surface: Vec3,
    /// Which ship slot fired it. The shot cannot hit its own launcher in flight, and
    /// an [`Impact`] carries it. **Not excluded from the blast**, see [`Impact`].
    pub owner: u8,
    /// Seconds left before [`MAX_FLIGHT_SECONDS`] reaps it.
    pub lifetime: f32,
    /// The ship slot a guided projectile is chasing, or `None`.
    ///
    /// A slot index where the original keeps a pointer (`Missile_Init` copies it to
    /// `self+0xe0`), so it survives a `Copy` snapshot and can be hashed. Set once
    /// at launch and never re-evaluated: `Missile_Update` never writes `+0xe0`, so
    /// there is no re-targeting or give-up. See [`missile::lock`].
    pub target: Option<u8>,
    /// How many walls this projectile has glanced off. Only a Missile raises it,
    /// see [`missile::MAX_BOUNCES`].
    pub bounces: u8,
    /// The speed this projectile left the rail at, in km/h.
    ///
    /// The base a Missile's or Plasma's speed ramp blends away from over
    /// [`missile::SPEED_RAMP_SECONDS`] (see [`missile::speed_kmh`]). Per shot, as
    /// it carries the firing craft's speed at launch (for the Plasma, when the
    /// charge ends). `0.0` for every other weapon.
    pub launch_speed_kmh: f32,
    /// The pose a laid charge is drawn with, frozen at landing. Presentation only,
    /// so excluded from `oag_gameplay::hash`'s reference; `Quat::IDENTITY` for a
    /// flying projectile. See [`mine::frozen_pose`].
    pub orientation: Quat,
    /// Seconds of wind-up left before this one leaves the craft, `0.0` once
    /// flying.
    ///
    /// Only the Plasma holds one, recovered: see [`plasma::CHARGE_SECONDS`]. A
    /// charging bolt occupies its slot and rides the craft's nose; it does not
    /// fly, sweep, collide or age. `Weapon_FirePlasma` (`0x0886a868`) takes the
    /// slot and `Plasma_Init` (`0x0885bd18`) marks it charging.
    ///
    /// Hashed, unlike [`Self::orientation`]: it decides when a bolt starts flying.
    pub charge: f32,
    /// The control effect this bolt lands on whoever it hits. Only a Disruptor
    /// carries one: `Disruptor_Init` (`0x08859010` on `psp-pure-usa`) copies the
    /// craft's primed kind into `bolt+0x114`. See [`disruptor`].
    pub effect: Option<oag_tables::weapons::DisruptorEffectKind>,
}

/// What a projectile did when it stopped.
///
/// # The blast rule: half recovered, half ours
///
/// Everything within `blastradius` of [`Self::point`] takes the **full**
/// `damage`, and the **impulse falls off linearly**, `1.0 - d / blastradius`.
/// The falloff is recovered from two independent functions:
///
/// - `Weapon_PostBlastImpulse` (`0x0886794c`), the Mine's, see
///   `docs/ghidra/functions/psp-pulse-usa/mine.md#the-blast-confirmed-from-the-other-end-and-it-falls-off`.
/// - `FUN_08868ea4`, reached from a craft hit, which walks every craft but the
///   one struck and adds `direction * (1 - distance/blastradius) * blastforce`.
///
/// # The damage is still ours
///
/// Full `damage` inside the radius is this engine's invention, and the two
/// recovered paths disagree: the Mine's sweep adds the authored `damage` to
/// every craft it reaches with no distance term, while the craft-hit path
/// damages the struck craft alone (`FUN_08869054`, not `FUN_08868ea4`). They
/// describe different events, so nothing changes on one's strength. This
/// applies the Mine's rule everywhere.
///
/// **The firing craft is not excluded.** A rocket fired into a wall at close
/// range hurts its firer; nothing says the original agrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Impact {
    /// Where the blast is centred.
    pub point: Vec3,
    /// Which weapon it was, so the caller can look its stats back up.
    pub kind: Weapon,
    /// The ship slot that fired it.
    pub owner: u8,
    /// The ship slot it struck directly, or `None` for a hit on geometry.
    ///
    /// A direct hit takes blast damage like any craft in the radius, except for the
    /// Plasma: an impact with `struck: Some(_)` goes through
    /// `blast::blast_direct_hit` (full `damage`/`slowdown_time` to `struck` alone,
    /// an impulse-only sweep to the rest). See [`Self::blast`].
    pub struck: Option<u8>,
    /// Whether this detonation spends a blast, or only shows one.
    ///
    /// `true` everywhere except a missile that ran out of time
    /// ([`missile::SELF_DETONATE_SECONDS`]), recovered at confidence 88. The
    /// original's damage (`FUN_08869054`) and blast force (`FUN_08868ea4`) each have
    /// exactly two callers, by operand search: `FUN_088690fc` (the swept-segment
    /// test against each craft) and `FUN_08868a10` (the network "somebody else's
    /// missile died" handler). The pool's expiry teardown in `Projectiles_Update_q`
    /// (`0x08869588`) calls neither, only `FUN_08868d50`, which spawns the `MIEX`
    /// explosion. Open: `FUN_08868a10`'s `craft == -1` branch does call the blast
    /// force (`docs/ghidra/functions/psp-pulse-usa/missile.md`).
    ///
    /// **The Plasma has three endings.** A wall hit and the `10.0 < age` timeout are
    /// `false`: `Plasmas_Update`'s (`0x0886b490`) teardown is `Psys_Release_q`,
    /// `Plasma_SpawnDetonation`, `PLASMAHITWALL` and nothing else, reaching neither
    /// `Weapon_PostBlastImpulse` (`0886794c`, the Mine's only caller) nor
    /// `entity+0x110`. A direct craft hit is `true`: `Plasma_SweepCraftHit`
    /// (`0x0886afb8`), a per-tick hull-cylinder sweep, calls `Plasma_HitCraft`
    /// (`0x0886ad60`, full `damage`/`slowdown_time` to the struck craft off
    /// `WeaponStats_ParsePlasma`'s `+0xa0`/`+0xc4`) and `Plasma_ApplyBlastForce`
    /// (`0x0886ae08`, a `(1 - d/blastradius) * blastforce` push on every craft but
    /// the firer). See the "a craft hit is the third ending" section of
    /// `docs/ghidra/functions/psp-pulse-usa/plasma.md`.
    ///
    /// A flag rather than a second array: `oag_game`'s tick still plays the
    /// explosion for a `false` one.
    pub blast: bool,
    /// The effect a Disruptor bolt carried, for the craft it struck. `None` for
    /// other weapons and for a wall hit. Read by `blast::apply_impacts`, which
    /// routes it through [`crate::disruption::land`].
    pub effect: Option<oag_tables::weapons::DisruptorEffectKind>,
}

/// Everything in the air, in one fixed-size array.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projectiles {
    /// The slots. Public because `oag_gameplay::hash` and the renderer walk them;
    /// test [`Projectile::kind`] before reading any other field.
    pub slots: [Projectile; MAX_PROJECTILES],
}

impl Default for Projectiles {
    fn default() -> Self {
        Self::new()
    }
}

impl Projectiles {
    /// An empty array.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: [Projectile {
                kind: None,
                position: Vec3::ZERO,
                velocity: Vec3::ZERO,
                owner: 0,
                lifetime: 0.0,
                surface: Vec3::Y,
                target: None,
                bounces: 0,
                launch_speed_kmh: 0.0,
                orientation: Quat::IDENTITY,
                charge: 0.0,
                effect: None,
            }; MAX_PROJECTILES],
        }
    }

    /// How many slots are in use.
    #[must_use]
    pub fn live(&self) -> usize {
        self.slots.iter().filter(|p| p.kind.is_some()).count()
    }

    /// Empties every slot.
    ///
    /// Nothing calls this today by design: `oag_raceplay::Race::start` builds a new
    /// `World`. It is for a caller that reuses a `World` (replay keyframe, restart).
    /// A respawn does not call it: a craft put back on the track has not unfired
    /// anything.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Puts one in the air, or does nothing if every slot is taken.
    ///
    /// Takes the first free slot in index order: which slot a shot lands in is
    /// simulation state, so a scan by age or distance would make it depend on the
    /// whole history. A full array drops the shot silently rather than evicting the
    /// oldest (only a bug reaches it today). Returns whether the shot was taken.
    pub fn spawn(&mut self, kind: Weapon, position: Vec3, velocity: Vec3, owner: u8) -> bool {
        self.spawn_guided(kind, position, velocity, owner, None, 0.0)
    }

    /// The same, for a projectile that carries a lock and a launch speed.
    ///
    /// [`Self::spawn`] is this with both empty, so an unguided weapon lands in the
    /// same slot with the same fields as always.
    pub fn spawn_guided(
        &mut self,
        kind: Weapon,
        position: Vec3,
        velocity: Vec3,
        owner: u8,
        target: Option<u8>,
        launch_speed_kmh: f32,
    ) -> bool {
        self.place(
            kind,
            position,
            velocity,
            owner,
            target,
            launch_speed_kmh,
            MAX_FLIGHT_SECONDS,
        )
        .is_some()
    }

    /// The same as [`Self::spawn_guided`], seeding the surface normal: the original's
    /// first probe goes along the craft's normal, not world up. Its own entry point
    /// for [`Self::lay`]'s reason.
    pub fn spawn_riding(
        &mut self,
        kind: Weapon,
        position: Vec3,
        velocity: Vec3,
        owner: u8,
        surface: Vec3,
        launch_speed_kmh: f32,
    ) -> bool {
        let Some(projectile) = self.place(
            kind,
            position,
            velocity,
            owner,
            None,
            launch_speed_kmh,
            MAX_FLIGHT_SECONDS,
        ) else {
            return false;
        };
        projectile.surface = surface;
        true
    }

    /// Throws one blade, with its own authored `fuse`.
    ///
    /// Its own entry point for [`Self::lay`]'s reason. Running out is presentation
    /// only: the impact has `blast: false`, as `ShurikenPool_Update` (`0x0886ff38`)
    /// raises the destroy bit at `fuse < age` and its teardown `FUN_08870c78` plays
    /// `WO_SHURIKEN_EXPIRE` and a flash, spending no damage. See
    /// `oag_tables::weapons::ShurikenStats`.
    pub fn throw(&mut self, position: Vec3, velocity: Vec3, owner: u8, fuse: f32) -> bool {
        self.place(Weapon::Shuriken, position, velocity, owner, None, 0.0, fuse)
            .is_some()
    }

    /// Fires one plasma bolt, held on the firing craft's nose for its wind-up.
    ///
    /// Its own entry point for [`Self::lay`]'s reason. `charge` is
    /// [`plasma::CHARGE_SECONDS`]; [`Self::advance`] reseats the bolt on the craft
    /// each tick, so it leaves along where the craft points when the wind-up ends.
    /// `velocity`'s length is a hold-time visual only; [`Self::advance`] overwrites
    /// both from the craft's velocity then (see [`Projectile::launch_speed_kmh`]).
    pub fn charge_up(&mut self, position: Vec3, velocity: Vec3, owner: u8, charge: f32) -> bool {
        let Some(slot) = self.place(
            Weapon::Plasma,
            position,
            velocity,
            owner,
            None,
            0.0,
            MAX_FLIGHT_SECONDS,
        ) else {
            return false;
        };
        slot.charge = charge;
        true
    }

    /// Lays one mine or bomb, with its own authored fuse.
    ///
    /// A separate entry point so other weapons keep landing in the same slot with
    /// the same fields, or a determinism reference stops being about the weapon that
    /// changed. `fuse` is `<Weapon type="Mine"><Stats timetodie>`; running out
    /// **detonates** a mine rather than reaping it (see [`Self::advance`]'s expiry
    /// branch). `orientation` is the pose it is drawn with, see [`mine::frozen_pose`].
    pub fn lay(
        &mut self,
        kind: Weapon,
        position: Vec3,
        owner: u8,
        fuse: f32,
        orientation: Quat,
    ) -> bool {
        let Some(slot) = self.place(kind, position, mine::at_rest(), owner, None, 0.0, fuse) else {
            return false;
        };
        slot.orientation = orientation;
        true
    }

    /// The one place a slot is filled; returns it so callers can set more fields
    /// without a second scan.
    #[allow(clippy::too_many_arguments)]
    fn place(
        &mut self,
        kind: Weapon,
        position: Vec3,
        velocity: Vec3,
        owner: u8,
        target: Option<u8>,
        launch_speed_kmh: f32,
        lifetime: f32,
    ) -> Option<&mut Projectile> {
        let slot = self.slots.iter_mut().find(|p| p.kind.is_none())?;
        *slot = Projectile {
            kind: Some(kind),
            position,
            velocity,
            owner,
            lifetime,
            orientation: Quat::IDENTITY,
            surface: Vec3::Y,
            target,
            bounces: 0,
            launch_speed_kmh,
            // Flying from this tick on; only `charge_up` raises it.
            charge: 0.0,
            effect: None,
        };
        Some(slot)
    }

    /// Fires one Disruptor bolt carrying `effect`, homing on `target` if any.
    ///
    /// Its own entry point for [`Self::throw`]'s reason. The ten-second life is
    /// [`MAX_FLIGHT_SECONDS`] exactly (`DisruptorPool_Update` reaps at `age > 10.0`,
    /// see [`disruptor::MAX_FLIGHT_SECONDS`]). `velocity` is [`disruptor::launch`]'s
    /// 500 km/h along forward; `surface` is seeded from its up, because
    /// `Disruptor_Update` never writes `bolt+0x100`.
    pub fn fire_disruptor(
        &mut self,
        position: Vec3,
        velocity: Vec3,
        surface: Vec3,
        owner: u8,
        target: Option<u8>,
        effect: oag_tables::weapons::DisruptorEffectKind,
    ) -> bool {
        let Some(slot) = self.place(
            Weapon::Disruptor,
            position,
            velocity,
            owner,
            target,
            0.0,
            MAX_FLIGHT_SECONDS,
        ) else {
            return false;
        };
        slot.surface = surface;
        slot.effect = Some(effect);
        true
    }
}

/// Flies the world's projectiles and detonates whatever stopped, in one call.
///
/// Returns the impacts so a caller can put a spark where each landed. `hits` is
/// [`blast`]'s out-parameter, passed through: one [`WeaponHit`] per ship slot.
///
/// `weapons` is `None` for a race whose weapon table did not load: an impact
/// then only frees its slot, as a blast with no authored radius or damage would
/// be invented. The same holds per weapon (no authored Missile, no missile
/// blast); the blast is looked up by `Impact::kind`, never by the Rocket's stats.
///
/// Every projectile is flown before any blast is applied, so two simultaneous
/// impacts see the pre-blast poses and array index is not part of the physics.
#[allow(
    clippy::too_many_arguments,
    reason = "the seven facts of a tick it always took, now with the pool and the occupied craft passed in place of the `World` that held both"
)]
pub fn step<R: Raycaster + ?Sized, S: Craft>(
    projectiles: &mut Projectiles,
    ships: &mut [S],
    dt: f32,
    raycaster: &R,
    weapons: Option<&oag_tables::weapons::WeaponStats>,
    class: &str,
    rules: oag_physics::DamageRules,
    hits: &mut [WeaponHit],
) -> [Option<Impact>; MAX_PROJECTILES] {
    let missile_stats = weapons.and_then(oag_tables::weapons::WeaponStats::missile);
    let plasma_stats = weapons.and_then(oag_tables::weapons::WeaponStats::plasma);
    let disruptor_stats = weapons.and_then(oag_tables::weapons::WeaponStats::disruptor);
    let impacts = projectiles.advance(
        dt,
        raycaster,
        ships,
        missile_stats.as_ref(),
        plasma_stats.as_ref(),
        disruptor_stats.as_ref(),
        TriggerRadii::from_table(weapons),
        class,
    );

    blast::apply_impacts(ships, weapons, &impacts, rules, hits);

    impacts
}

/// How many km/h one world unit per second is, for the authored weapon speeds.
///
/// **Recovered, confidence 84**, the divisor both of `Rocket_SpeedForClass`'s
/// callers apply (see [`launch`]); [`oag_core::math::SPEED_TO_KMH`] named for
/// this direction. Kept a division, as `Rocket_Update` does, not a baked
/// reciprocal.
pub const KMH_PER_UNIT_PER_SECOND: f32 = oag_core::math::SPEED_TO_KMH;

/// How far a projectile looks toward the surface for something to ride.
///
/// **Recovered, confidence 82.** `Rocket_Update` (`0x0885d2a8`) casts
/// `6.0 * normal` every tick, first. See
/// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
pub const SURFACE_PROBE_LENGTH: f32 = 6.0;

/// How far above the surface a projectile rides.
///
/// **Recovered, confidence 82.** The same function pushes out along the hit
/// normal by `3.0` after a surface hit.
pub const RIDE_HEIGHT: f32 = 3.0;

/// How fast a projectile with nothing under it falls, in units per second squared.
///
/// **Recovered, confidence 82.** `velocity.y -= dt * 50.0` where the probe finds
/// nothing.
pub const FALL_ACCELERATION: f32 = 50.0;

#[cfg(test)]
mod tests;
