//! Things a weapon puts in the air, and what they hit.
//!
//! Today that is the Rocket, the Missile and the Mine. The Plasma bolt, the
//! Bomb and the Shuriken all belong here when they land; what each of them adds
//! is a launch rule and an impact rule, not a second flight model.
//!
//! **The Mine is the exception that proves that**, and it is worth knowing
//! before reading [`Projectiles::advance`]: it adds no flight model because it
//! has none. It takes an early return at the top of the loop and shares only
//! the array and the blast. See [`mine`].
//!
//! # What is recovered and what is ours
//!
//! Stated at the top, the way [`crate::pickup`] states it, because this module
//! is **almost entirely ours** and that is easy to lose.
//!
//! **Recovered.** The numbers, and **how many rockets a press puts in the
//! air**. `<Weapon type="Rocket"><Stats>` authors `damage`, `blastforce`,
//! `blastradius`, `launchSpeed`, `spread` and a separate flight speed per speed
//! class, at confidence 92 (`docs/formats/weapon-stats.md`). `Weapon_FireRocket`
//! (`0x0886e104`) then spawns **three at once** - straight ahead, `+spread` and
//! `-spread` - at confidence 88
//! (`docs/ghidra/functions/psp-pulse-usa/weapon-fire.md`). Also recovered is
//! what happens at the far end: `Ship_Damage` (`0x088439ac`) takes a `source`
//! argument whose value 2 is a weapon, and the weapons-off halving and the clamp
//! are its own (`.../shield.md`).
//!
//! Also recovered, and now implemented: **a projectile follows the floor.**
//! `Rocket_Update` (`0x0885d2a8`) probes [`SURFACE_PROBE_LENGTH`] units toward
//! the surface every tick, rides [`RIDE_HEIGHT`] above what it finds, and falls
//! at [`FALL_ACCELERATION`] when it finds nothing
//! (`.../rocket-visuals.md`). Confirmed in PPSSPP, where the frames show rockets
//! hugging the track a body-length off the racing line, and corroborated by a
//! maintainer who has played the game: projectiles skim the floor and stop only
//! on a craft or a wall.
//!
//! **This is why the module was rewritten rather than tuned.** Flying straight,
//! a volley died in the tick it was fired - measured on a real track, all three
//! rockets gone before the next frame, which is what "they look like three dots
//! and disappear" was.
//!
//! Also recovered, **and what this module got wrong until 2026-09-13**: wall
//! versus floor is the surface *class* of the hit, never its angle. The
//! original's query returns the struck collider's surface type (`0` wall,
//! `1` floor, `3` mag floor, `4` a craft, `0x7f` nothing); this raycaster
//! returns the same tag in `RaycastHit::surface`, and the module branched on
//! the hit normal's angle anyway - a grazing wall let a rocket leave the
//! circuit, a steep floor detonated it. See `docs/gameplay/projectile-floor.md`.
//!
//! **Ours.**
//!
//! - **Turning the velocity parallel to the surface rather than reflecting it.**
//!   The original recomputes velocity from a corrected position, which is not
//!   quite either; this preserves speed and follows the track, which is the
//!   behaviour described.
//! - **That the rule is weapon-agnostic.** It lives in [`Projectiles::advance`],
//!   so the Missile and the rest inherit it when they land. A maintainer's
//!   account puts the Missile on the floor too and is **less sure about the
//!   Cannon**; nothing here has read either, so if the Cannon turns out to fire
//!   flat this is the place that has to grow a per-weapon flag.
//! - **The axis the fan rotates about.** See [`launch`].
//! - **A sphere for a hull**, and one that is *wider* than the hull on two of
//!   its three axes. The original tests a projectile against something and
//!   nothing says what. See [`hull_radius`], which is exact about which way it
//!   errs.
//! - **Full damage everywhere inside `blastradius`,** with no falloff - and
//!   the original's force *does* fall off. See [`Impact`].
//! - **The launch offset and the lifetime cap.**
//!
//! # Fixed-size, like everything else in the world
//!
//! [`MAX_PROJECTILES`] slots of plain `Copy` data, allocated once inside
//! [`crate::World`]. No `Vec`, so a world snapshot stays one `memcpy`-shaped
//! operation - `docs/architecture/adr/0003-no-ecs.md`. A full array drops the
//! shot rather than growing, and [`Projectiles::spawn`] says so.

mod blast;
pub mod cannon;
pub mod disruptor;
mod flight;
mod geometry;
pub mod leach_beam;
pub mod mine;
pub mod missile;
pub mod plasma;
pub mod quake;
mod rocket;
pub mod shuriken;

pub use geometry::hull_radius;
use geometry::{SweepHit, nearest_hit};

pub use blast::{BlastStats, blast};
pub use mine::TriggerRadii;
pub use rocket::{ROCKET_SHOTS, launch};

use oag_core::math::{Quat, Vec3};
use oag_physics::Raycaster;
use oag_tables::weapons::Weapon;

/// The most projectiles that can be in the air at once.
///
/// **Ours, and deliberately larger than the original's.**
///
/// The original's pool caps at **48** - `Weapon_FireRocket` bounds-checks
/// `live < 0x2e` before putting three more in the air, against a cap of `0x30`
/// elsewhere. That is the floor this has to clear, not the target: it is a 2007
/// handheld's budget, and an Eliminator race in the original genuinely runs out
/// and drops shots.
///
/// **This used to be 16, and 16 was too small to be correct**, not merely tight:
/// a Rocket puts [`ROCKET_SHOTS`] in the air per press, so one simultaneous
/// volley from a full grid is 24 and would have silently dropped a third of
/// itself at [`Projectiles::spawn`]. Surface-following flight made it worse by
/// keeping projectiles alive far longer than straight flight into the nearest
/// wall did.
///
/// 128 is eight craft with sixteen apiece in the air at once, which no rate of
/// fire this engine can reach will exhaust.
///
/// **That argument was written when everything here died on its first wall, and
/// the Mine is the first thing that does not.** A laid mine holds its slot for
/// the authored `timetodie` - seven seconds - whatever the craft does next, so
/// occupancy is now a function of how often pads are crossed rather than of how
/// far a shot flies. Eight craft laying [`mine::CLUSTER`] apiece is forty, and a
/// second full round inside seven seconds would be eighty: still inside 128, and
/// no longer by the margin the paragraph above assumes. Worth re-checking
/// against any weapon that persists longer - the Bomb's `timetodie` is
/// **twenty** seconds. Fixed-size rather than a `Vec`
/// because [ADR-0003](../../../docs/architecture/adr/0003-no-ecs.md) requires
/// the world to snapshot in one `memcpy`-shaped operation; at roughly 48 bytes
/// a slot the whole array is about 6 KiB, which is not a number worth
/// economising on.
pub const MAX_PROJECTILES: usize = 128;

/// How long a projectile flies before it gives up, in seconds.
///
/// **The number is ours; that there is a cap at all is not.** A rocket that
/// leaves the track through a gap in the collision soup would otherwise hold its
/// slot for the whole race. Long enough that no rocket fired at anything
/// reachable expires first: the disc's slowest class authors `800` km/h, which
/// is `222` units a second (see [`launch`] on the unit), so ten seconds is
/// better than two kilometres - more than a lap of any Pulse circuit is wide.
///
/// **The original caps a rocket at `5.0`** (`FUN_0886de60`, `5.0 < self+0x48`)
/// and reaps it the way this does - that branch reaches the trail release and no
/// explosion spawner. Not adopted: it halves every rocket's reach and belongs in
/// a change about the Rocket. See the handover thread.
///
/// **A Missile never reaches this**, having detonated at
/// [`missile::SELF_DETONATE_SECONDS`], but its age is still measured against
/// this constant - so a change to one moves the other's arithmetic.
pub const MAX_FLIGHT_SECONDS: f32 = 10.0;

/// One thing in the air.
///
/// A free slot is one whose [`Self::kind`] is `None`; the rest of the fields are
/// then stale and must not be read. Kept as one array of plain structs rather
/// than as a list with a length, so the array's contents depend on nothing but
/// the simulation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Projectile {
    /// Which weapon fired it, or `None` for a free slot.
    pub kind: Option<Weapon>,
    /// Where it is, in world space.
    pub position: Vec3,
    /// Where it is going, in world units a second.
    ///
    /// **Not constant.** It is turned parallel to the surface each tick the
    /// projectile finds one, and pulled down by [`FALL_ACCELERATION`] each tick
    /// it does not. Speed is preserved across a turn; only the direction moves.
    pub velocity: Vec3,
    /// The surface normal the projectile is riding, normalised.
    ///
    /// **Recovered as a concept**: the original keeps one at `self+0x100`,
    /// initialises it from the firing craft and re-probes along it every tick
    /// (`Rocket_Update`, `0x0885d2a8`). It is what makes a projectile follow a
    /// banked or rolling track rather than a horizontal plane - the probe goes
    /// along *this*, not along world down.
    ///
    /// Seeded to [`Vec3::Y`] by [`Projectiles::spawn`] rather than to the firing
    /// craft's up, which is **ours**: the original seeds it from the craft and
    /// this engine's spawn call does not carry one. The first probe corrects it,
    /// so the cost is at most one tick of a wrong probe direction on a steeply
    /// banked launch.
    pub surface: Vec3,
    /// Which ship slot fired it.
    ///
    /// Read for two things: the shot cannot hit its own launcher in flight, and
    /// an [`Impact`] carries it so a future kill-credit or telemetry pass has
    /// it. **It is not excluded from the blast** - see [`Impact`].
    pub owner: u8,
    /// Seconds left before [`MAX_FLIGHT_SECONDS`] reaps it.
    pub lifetime: f32,
    /// The ship slot a guided projectile is chasing, or `None` for an unguided
    /// one and for a guided one that locked nothing.
    ///
    /// **A slot index where the original keeps a pointer.** `Missile_Init` copies
    /// a raw pointer to the target object into `self+0xe0`; an index is what
    /// survives being inside a `Copy` world snapshot, and it is what
    /// [`crate::hash`] can hash.
    ///
    /// Set once at launch and **never re-evaluated** - that is the original's
    /// behaviour, not a simplification: `Missile_Update` reads `+0xe0` twice and
    /// writes it never, so there is no re-targeting, no range re-check and no
    /// give-up when the target gets away. See [`missile::lock`].
    pub target: Option<u8>,
    /// How many walls this projectile has already glanced off.
    ///
    /// Only a Missile ever raises it - see [`missile::MAX_BOUNCES`]. A Rocket
    /// detonates on its first wall, so its counter is always zero.
    pub bounces: u8,
    /// The speed this projectile left the rail at, in km/h.
    ///
    /// Only a Missile reads it, as the base its speed ramp blends away from over
    /// [`missile::SPEED_RAMP_SECONDS`]; see [`missile::speed_kmh`]. It is
    /// per-shot rather than per-weapon because it carries the firing craft's own
    /// speed at the moment of launch.
    pub launch_speed_kmh: f32,
    /// The pose a laid charge is drawn with, frozen at the moment it landed.
    ///
    /// Presentation only, so excluded from [`crate::hash`]'s reference -
    /// `Quat::IDENTITY` for a flying projectile, which draws from
    /// [`Self::velocity`] instead. See [`mine::frozen_pose`].
    pub orientation: Quat,
    /// Seconds of wind-up left before this one leaves the craft that fired it,
    /// and `0.0` for anything already flying.
    ///
    /// **Only the Plasma ever holds one, and it is recovered** - see
    /// [`plasma::CHARGE_SECONDS`] for the two functions that carry it. A bolt
    /// with a charge left occupies its pool slot and rides the firing craft's
    /// nose; it does not fly, sweep, collide or age. That is the original's
    /// own shape: `Weapon_FirePlasma` (`0x0886a868`) takes the slot and
    /// `Plasma_Init` (`0x0885bd18`) marks it charging in the same breath, so a
    /// second press while one is winding up finds the pool one entry fuller.
    ///
    /// Hashed, unlike [`Self::orientation`]: it decides *when* a bolt starts
    /// flying and therefore where it is on every later tick.
    pub charge: f32,
    /// The control effect this bolt lands on whoever it hits.
    ///
    /// **Only a Disruptor carries one** - `Disruptor_Init` (`0x08859010` on
    /// `psp-pure-usa`) copies the firing craft's primed kind into
    /// `bolt+0x114`, and `Disruptor_ApplyEffect` reads it off the bolt on the
    /// hit. `None` for every other weapon. See [`disruptor`].
    pub effect: Option<oag_tables::weapons::DisruptorEffectKind>,
}

/// What a projectile did when it stopped.
///
/// # The blast rule: half recovered, half ours
///
/// Everything within `blastradius` of [`Self::point`] takes the **full**
/// `damage`, and the **impulse falls off linearly** - `1.0 - d / blastradius`,
/// to nothing at the radius, so a craft on the rim is nudged and one at the
/// centre is thrown. This module had the impulse flat until 2026-08-26.
///
/// **The falloff is recovered twice over, from two functions read
/// independently and days apart**, which is worth more than either read alone:
///
/// - `Weapon_PostBlastImpulse` (`0x0886794c`), the Mine subsystem's, whose four
///   `<Stats>` offsets are the Mine's `damage`, `blastradius`, `blastforce` and
///   `slowdown_time` - see
///   `docs/ghidra/functions/psp-pulse-usa/mine.md#the-blast-confirmed-from-the-other-end-and-it-falls-off`.
/// - `FUN_08868ea4`, reached from a **craft hit** rather than from a fuse, which
///   walks every craft but the one struck and adds
///   `direction * (1 - distance/blastradius) * blastforce`.
///
/// Two subsystems, one arithmetic. That is why it is implemented for every
/// weapon rather than for the Mine alone.
///
/// # The damage is the half still to settle, and the two paths disagree
///
/// `blast` applies full `damage` to everything inside the radius, and **that is
/// still this engine's invention** - more so than it looked, because the two
/// recovered paths do not agree with each other:
///
/// - The Mine's sweep accumulates the authored `damage` into **every** craft it
///   reaches, flat, with no distance term.
/// - The craft-hit path does not damage through `FUN_08868ea4` at all;
///   `FUN_08869054` takes the **struck craft alone**.
///
/// Both readings are sound and they are describing different things - a mine
/// going off among a group against a rocket hitting somebody - so neither
/// cancels the other and nothing here is changed on the strength of one. What
/// this module does is the Mine's rule applied everywhere, which is at least a
/// recovered rule rather than a guess, and the disagreement is recorded so the
/// next reader starts from it rather than rediscovering it.
///
/// **The firing craft is not excluded.** A rocket launched into a wall at close
/// range hurts the ship that fired it, which follows from the blast being a
/// position rather than an ownership question, and is the behaviour a player can
/// discover. Nothing says the original agrees.
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
    /// For every weapon but the Plasma, a direct hit takes blast damage like
    /// any other craft inside the radius - this field is here to tell the two
    /// apart for a future direct-hit bonus or a sound cue, not because the
    /// damage differs today.
    ///
    /// **The Plasma is the one weapon where the damage already differs, and
    /// `struck` is what `blast::apply_impacts` reads to route it.** A Plasma
    /// impact with `struck: Some(_)` goes through `blast::blast_direct_hit`
    /// rather than `blast::blast` - full `damage`/`slowdown_time` to `struck`
    /// alone, an impulse-only sweep to everyone else in radius. See
    /// [`Self::blast`]'s own doc comment.
    pub struck: Option<u8>,
    /// Whether this detonation spends a blast, or only shows one.
    ///
    /// `true` everywhere except a missile that ran out of time - see
    /// [`missile::SELF_DETONATE_SECONDS`] - and that exception is **recovered
    /// rather than a choice**, at confidence 88.
    ///
    /// The original's damage (`FUN_08869054`) and blast force (`FUN_08868ea4`)
    /// have exactly two callers each, by exhaustive operand search rather than
    /// by reading: `FUN_088690fc`, the per-tick swept-segment test against each
    /// craft, and `FUN_08868a10`, the network "somebody else's missile died"
    /// handler. The pool's expiry teardown in `Projectiles_Update_q`
    /// (`0x08869588`) calls **neither** - only `FUN_08868d50`, which spawns the
    /// `MIEX` explosion where the missile was. A missile that hits nothing
    /// *looks* like it went off and hurts nobody.
    ///
    /// One thing disagrees and is left standing: `FUN_08868a10`'s `craft == -1`
    /// branch does call the blast force, so the network path spends a blast
    /// where the local path does not. Open on
    /// `docs/ghidra/functions/psp-pulse-usa/missile.md`.
    ///
    /// **The Plasma is `false` on a wall hit and the `10.0 < age` timeout,
    /// and `true` on a direct craft hit - three endings, not two, and each
    /// reads differently.** `Plasmas_Update`'s (`0x0886b490`) teardown - read
    /// at instruction level 2026-09-16, no elision - is `Psys_Release_q`,
    /// `Plasma_SpawnDetonation`, `PLASMAHITWALL`, and nothing else, for a wall
    /// hit and the age timeout alike; neither reaches `Weapon_PostBlastImpulse`
    /// (`0886794c`, the Mine's only caller, confirmed by `get_xrefs_to`) or
    /// writes `entity+0x110`, the slot every real blast function - checked
    /// directly against `Missile_ApplyBlastForce` - writes. See [`plasma`]'s
    /// expiry arm in `Projectiles::advance` for the timeout half, ported
    /// 2026-09-16.
    ///
    /// **A direct craft hit is different, found later the same day.**
    /// `Plasmas_Update`'s own `if (p->flags & 1)` call - previously read as
    /// network code and prose-labelled `Plasma_NetSend_q` with no database
    /// name behind it - is `Plasma_SweepCraftHit` (`0x0886afb8`): a per-tick
    /// hull-cylinder sweep against every craft, structurally the same shape
    /// `RocketPool_Update` runs for the Rocket. On a hit it calls
    /// `Plasma_HitCraft` (`0x0886ad60`), which credits full
    /// `damage`/`slowdown_time` to the struck craft unconditionally off
    /// `WeaponStats_ParsePlasma`'s own `+0xa0`/`+0xc4`, and
    /// `Plasma_ApplyBlastForce` (`0x0886ae08`), which pushes every craft but
    /// the bolt's own firer with a `(1 - d/blastradius) * blastforce`
    /// impulse - the exact shape `Weapon_PostBlastImpulse` and
    /// `Missile_ApplyBlastForce` already carry. This corrects an earlier
    /// version of this comment (and of `plasma.md`) that said "no
    /// `Plasma_ApplyBlast`-shaped function exists" - the function was simply
    /// not named yet, so a name search could not find it. See
    /// `docs/ghidra/functions/psp-pulse-usa/plasma.md`'s "a craft hit is the
    /// third ending" section, and [`Self::struck`]'s own doc comment for how
    /// the two credits differ.
    ///
    /// A flag rather than a second array, because every consumer already walks
    /// these and the visual side wants the entry either way: `oag_game`'s tick
    /// still plays the explosion for a `false` one.
    pub blast: bool,
    /// The effect a Disruptor bolt was carrying, for the craft it struck.
    ///
    /// `None` for every other weapon, and for a Disruptor that stopped on a
    /// wall - a wall takes no effect. Read by `blast::apply_impacts`, which
    /// routes a Disruptor hit through [`crate::disruption::land`] rather than
    /// through a blast the weapon does not author.
    pub effect: Option<oag_tables::weapons::DisruptorEffectKind>,
}

/// Everything in the air, in one fixed-size array.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projectiles {
    /// The slots. Public because [`crate::hash`] destructures them and a
    /// renderer walks them; a caller must test [`Projectile::kind`] before
    /// reading any other field.
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
    /// **Nothing calls this today**, and that is not an oversight to fix by
    /// finding a caller: `oag_game::race::Race::start` builds a whole new
    /// `World`, so a race begins with a fresh array and cannot inherit a rocket
    /// from the previous run. It is here for a caller that *reuses* a `World`
    /// (a replay scrubbing to a keyframe, or a restart that keeps the
    /// allocation), because for such a caller the alternative is a rocket
    /// arriving at a craft that never fired one.
    ///
    /// A respawn deliberately does **not** call it: a craft put back on the
    /// track has not unfired anything it launched.
    pub fn clear(&mut self) {
        *self = Self::new();
    }

    /// Puts one in the air, or does nothing if every slot is taken.
    ///
    /// **The first free slot in index order**, never a search that depends on
    /// anything but the array: which slot a shot lands in is simulation state,
    /// and a scan ordered by age or distance would make it depend on the whole
    /// history.
    ///
    /// A full array **drops the shot silently**. That is a real state only a
    /// bug can reach today - eight craft hold one pickup each - and the
    /// alternative, evicting the oldest, would let a full array change what an
    /// already-fired rocket does.
    ///
    /// Returns whether the shot was taken, so a caller can decline to spend the
    /// pickup on nothing.
    pub fn spawn(&mut self, kind: Weapon, position: Vec3, velocity: Vec3, owner: u8) -> bool {
        self.spawn_guided(kind, position, velocity, owner, None, 0.0)
    }

    /// The same, for a projectile that carries a lock and a launch speed.
    ///
    /// [`Self::spawn`] is this with both left empty, so an unguided weapon needs
    /// no new call site and - more to the point - lands in exactly the same slot
    /// with exactly the same fields it always did. That is what keeps a
    /// rocket-only run's determinism hash a question about the hash *stream*
    /// rather than about the rocket's flight.
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

    /// Throws one blade, with its own authored `fuse` instead of the safety net.
    ///
    /// A fourth entry point for [`Self::lay`]'s reason: a rocket and a missile
    /// must keep landing in the same slot with the same fields they always did,
    /// or a determinism reference stops being a question about the weapon that
    /// changed.
    ///
    /// **`fuse` is read and what running out *does* is not.** Unlike a mine's
    /// `timetodie`, which detonates, a blade that times out is **reaped**
    /// silently here - the Rocket's own pool behaviour, chosen because
    /// `Shuriken_Update` only counts `+0x48` up and the teardown that would read
    /// it was not followed. `WO_SHURIKEN_EXPIRE` is authored on the disc and
    /// stays unwired for the same reason. See
    /// `oag_tables::weapons::ShurikenStats`.
    pub fn throw(&mut self, position: Vec3, velocity: Vec3, owner: u8, fuse: f32) -> bool {
        self.place(Weapon::Shuriken, position, velocity, owner, None, 0.0, fuse)
            .is_some()
    }

    /// Fires one plasma bolt, held on the firing craft's nose for its wind-up
    /// before it flies.
    ///
    /// **A fifth entry point, for [`Self::throw`]'s reason** - a rocket and a
    /// missile must keep landing in the same slot with the same fields they
    /// always did.
    ///
    /// `charge` is [`plasma::CHARGE_SECONDS`]; while it lasts the slot is
    /// taken and the bolt is reseated on the craft every tick by
    /// [`Self::advance`], so the shot leaves along wherever that craft is
    /// pointing when the wind-up ends rather than where it pointed when the
    /// button went down. `velocity` is therefore read for its **length** while
    /// charging and for its direction only once the hold is over.
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

    /// Lays one mine or bomb, with its own authored fuse instead of the safety
    /// net.
    ///
    /// A third entry point rather than a widened [`Self::spawn_guided`], for the
    /// reason that one is a widened [`Self::spawn`]: a rocket and a missile must
    /// keep landing in the same slot with the same fields they always did, or a
    /// determinism reference stops being a question about the weapon that
    /// changed.
    ///
    /// `fuse` is `<Weapon type="Mine"><Stats timetodie>` and running out is what
    /// **detonates** a mine rather than what reaps it - see
    /// [`Self::advance`]'s expiry branch, which is the one place in this module
    /// where the countdown means two different things depending on the weapon.
    /// `orientation` is the pose it is drawn with from here on - see
    /// [`mine::frozen_pose`].
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

    /// The one place a slot is filled. Returns the slot itself rather than a
    /// bare `bool`, so [`Self::lay`] can set [`Projectile::orientation`]
    /// afterward without a second scan.
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
            // A flying projectile draws its basis from `velocity` instead; see the field.
            orientation: Quat::IDENTITY,
            // World up until the first probe corrects it - see the field.
            surface: Vec3::Y,
            target,
            bounces: 0,
            launch_speed_kmh,
            // Flying from this tick on. `Projectiles::charge_up` is the one
            // entry point that raises it, for `Self::lay`'s reason: every
            // other weapon must keep landing in the same slot with the same
            // fields it always did.
            charge: 0.0,
            // Only `Self::fire_disruptor` sets one, for the same reason.
            effect: None,
        };
        Some(slot)
    }

    /// Fires one Disruptor bolt carrying `effect`, homing on `target` if it
    /// has one.
    ///
    /// **A sixth entry point, for [`Self::throw`]'s reason.** The bolt's own
    /// ten-second life is [`MAX_FLIGHT_SECONDS`] exactly -
    /// `DisruptorPool_Update` reaps at `age > 10.0`, which is the one place
    /// that constant is recovered rather than ours; see
    /// [`disruptor::MAX_FLIGHT_SECONDS`]. `velocity` is
    /// [`disruptor::launch`]'s literal 500 km/h along the craft's forward,
    /// and `surface` is seeded from [`disruptor::launch`]'s up rather than
    /// world up, because the Disruptor is the one weapon whose probe
    /// direction is **never** re-read from a hit - `Disruptor_Update` writes
    /// `bolt+0x100` nowhere.
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
/// The seam a race loop wants: `oag-gameplay` owns both halves - the array and
/// the weapon table that says what a hit is worth - and the composition root
/// owns neither. Returns the impacts so a caller can put a spark where each one
/// landed, which is the only thing left for it to do.
///
/// `absorbed` is [`blast`]'s out-parameter, passed straight through: one flag
/// per ship slot for a craft whose fired Shield swallowed a blast this call.
/// See [`blast`] for why it is a parameter and not a second return value.
///
/// `weapons` is `None` for a race whose weapon table did not load, in which case
/// an impact does nothing but free its slot. That is the same "gated on the
/// table" rule the pickup grant follows, and for the same reason: a blast with
/// no authored radius or damage would be an invented number. The same is true per
/// *weapon*: a table that authors no Missile gives a missile impact no blast.
///
/// # The blast is looked up per weapon, and it used to not be
///
/// This took `Option<&RocketStats>` and spent it on **every** impact whatever
/// `Impact::kind` said, which was invisible while the Rocket was the only thing
/// in the air and would have quietly given a missile the rocket's `damage`,
/// `blastradius` and `blastforce`. The two differ on the shipped disc - the
/// Missile hits harder and pushes harder - so the bug would have read as a tuning
/// disagreement rather than as a wiring one.
///
/// # Ordering
///
/// Every projectile is flown before any blast is applied, so a rocket cannot be
/// deflected by a blast from another rocket that happened to be earlier in the
/// array. Two simultaneous impacts therefore both see the pre-blast poses, which
/// is the only ordering that does not make the array index part of the physics.
pub fn step<R: Raycaster + ?Sized>(
    world: &mut crate::World,
    dt: f32,
    raycaster: &R,
    weapons: Option<&oag_tables::weapons::WeaponStats>,
    class: &str,
    rules: oag_physics::DamageRules,
    absorbed: &mut [bool],
) -> [Option<Impact>; MAX_PROJECTILES] {
    let count = world.ship_count as usize;
    let missile_stats = weapons.and_then(oag_tables::weapons::WeaponStats::missile);
    let disruptor_stats = weapons.and_then(oag_tables::weapons::WeaponStats::disruptor);
    let impacts = world.projectiles.advance(
        dt,
        raycaster,
        &world.ships[..count],
        missile_stats.as_ref(),
        disruptor_stats.as_ref(),
        TriggerRadii::from_table(weapons),
        class,
    );

    blast::apply_impacts(
        &mut world.ships[..count],
        weapons,
        &impacts,
        rules,
        absorbed,
    );

    impacts
}

/// How many km/h one world unit per second is, for the authored weapon speeds.
///
/// **Recovered, confidence 84**, as the divisor both of
/// `Rocket_SpeedForClass`'s callers apply - see [`launch`]. It is
/// [`oag_core::math::SPEED_TO_KMH`], the same physical conversion the exhaust
/// ramp and the HUD's readout run the other way, named for this direction here.
///
/// Kept as a division rather than a multiply by a baked reciprocal, which is
/// what `Rocket_Update` itself does and which keeps the arithmetic one
/// operation instead of two roundings.
pub const KMH_PER_UNIT_PER_SECOND: f32 = oag_core::math::SPEED_TO_KMH;

/// How far a projectile looks toward the surface for something to ride.
///
/// **Recovered, confidence 82.** `Rocket_Update` (`0x0885d2a8`) scales the
/// stored normal by `6.0` and casts along it from the projected position, every
/// tick, before anything else. See
/// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
pub const SURFACE_PROBE_LENGTH: f32 = 6.0;

/// How far above the surface a projectile rides.
///
/// **Recovered, confidence 82.** The same function pushes out along the hit
/// normal by `3.0` after a surface hit, which is what holds a projectile off the
/// track rather than in it.
pub const RIDE_HEIGHT: f32 = 3.0;

/// How fast a projectile with nothing under it falls, in units per second squared.
///
/// **Recovered, confidence 82.** `velocity.y -= dt * 50.0` on the branch where
/// the surface probe finds nothing - a projectile that flies off the edge of the
/// track drops instead of sailing on forever.
pub const FALL_ACCELERATION: f32 = 50.0;

#[cfg(test)]
mod tests;
