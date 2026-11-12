//! Where a title keeps the table its weapons are tuned by.
//!
//! An axis ([ADR-0022]) because the answers differ in **shape**: Pulse and HD ship
//! two tables and pick by race mode, Pure ships one that every mode reads.
//!
//! Recovered from each title's executable. Pulse's two names are in
//! `oag_tables::weapons`' constants, off `BOOT.BIN`'s string table; Pure's is
//! `Data\XML\weaponstats.xml` at `0x08a445a0` in `/psp-pure-usa/BOOT.BIN`, three
//! strings before the `"WeaponStats"` and `"Weapon"` element names the parser
//! matches, which says it is the file this parser reads.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// The tables a title tunes its weapons from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weapons {
    /// The table a race reads, and the only one on a title that ships one.
    pub race: &'static str,
    /// The table Eliminator reads instead, or `None` for a title with one table.
    /// The table Eliminator reads instead, or `None` for a title with one table.
    ///
    /// **`None` is a measurement.** `WeaponStats_Elimination.xml` is how Pulse
    /// makes the same weapon hit differently in Eliminator: the global
    /// `DAT_08b32428` selects the file, not the speed class
    /// (`docs/ghidra/functions/psp-pulse-usa/missile.md`). Pure's single
    /// `weaponstats.xml` is its whole tuning.
    ///
    /// `oag_raceplay`'s `load_weapons` opens it for `Mode::Eliminator` and falls
    /// back to [`Self::race`], with a report line, where it is `None`.
    pub elimination: Option<&'static str>,
    /// The table an opponent's fire odds are read from (`useAgainstPlayer`,
    /// `useAgainstAI` and `absorb` per weapon, `oag_tables::weapons::ai`), or `None`
    /// where it has not been looked for.
    ///
    /// **`None` is not a measurement here**, unlike [`Self::elimination`]'s. Every
    /// title ships the table and rolls [`Self::opponent_fire`]'s law on its own
    /// odds: Pulse's and HD's decision code are read, the others inherit
    /// (`docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`, "Every title carries
    /// the table"). A race without the file fires on this
    /// project's own rule (`oag_raceplay::FireLaw`).
    pub ai: Option<&'static str>,
    /// When a locked LeachBeam moves energy: every tick, or once a ball trip.
    pub leach_drain: LeachDrain,
    /// How a weapon blast's push reaches the craft it hits.
    pub blast_push: crate::pre_race::Sourced<BlastPush>,
    /// How many LeachBeams a race holds at once (at most
    /// `oag_weapons::projectile::leach_beam::POOL_SIZE`, which the race clamps to).
    ///
    /// 2048 and Omega measure eight; Pulse and HD measure one and take eight as a
    /// chosen divergence, recorded in `docs/gameplay/weapon-lineage.md`.
    pub leach_beams: crate::pre_race::Sourced<u8>,
    /// How a Quake's wave reaches a craft: what gates its hit and whether the
    /// cockpit eye rides it.
    pub quake_hit: QuakeHit,
    /// Whether the absorb press cancels a running Autopilot pickup.
    ///
    /// Pulse's `Ship_AbsorbHeldPickup` tests the running bit before it reads the
    /// slot and stores `0.0` to the timer, so the craft is handed back next tick
    /// whatever is held, and the held pickup is neither paid out nor spent
    /// (`docs/ghidra/functions/psp-pulse-usa/autopilot.md`, "Absorb cancels it, fire
    /// does not"). HD's `Ship_UpdatePickup_q` does the same. `false` is a
    /// measurement on Pure: nothing on its absorb side reads the running byte. The
    /// fire press never cancels on any title (`Race::spend_pickup`).
    pub autopilot_absorb_cancels: crate::pre_race::Sourced<bool>,
    /// Whether a weapon pad hands out a pickup to a craft whose Autopilot is
    /// running.
    ///
    /// Pulse's `Ship_TestWeaponPads` raises the grant request only with the running
    /// bit clear (`0x0883f0c0`), so a pad crossed mid-stint grants nothing; the pad
    /// still stamps its refresh timer. Read on Pulse alone; the others run its law.
    pub autopilot_pad_grants: crate::pre_race::Sourced<bool>,
    /// Whether a weapon pad is stamped inert for a while after a hit in an Eliminator race.
    ///
    /// Pulse's `WeaponPads_TestCraft` stamps `elimination_refresh_time` (`0.05` s, two grey
    /// frames). HD, 2048 and Omega author `0.15` and never stamp in Eliminator: the pad stays
    /// armed and a craft with an empty slot is handed a pickup on every entry
    /// (`docs/gameplay/pickups.md`, "Eliminator and the pads, per title"). Other modes stamp
    /// `refresh_time` on every title. A title with no Eliminator (Pure) never reads it.
    pub pads_stamp_in_eliminator: crate::pre_race::Sourced<bool>,
    /// Whether a Rocket, Missile, Shuriken or Plasma bolt is born carrying the track's up at
    /// the craft's located point (`true`) or the craft's own up (`false`).
    ///
    /// Pulse's `Rocket_Init`, `Missile_Init`, `Shuriken_Init` and `Plasma_Launch` store
    /// `-(craft+0xb10)`, the down axis of the track sample the craft is located on: on Outpost 7's
    /// ramp that is `(0, 1, 0)` where the craft's up is 5.24 degrees off
    /// (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "The launch normal is the
    /// track's, not the craft's"). The Cannon and the Disruptor are not in that list and keep the
    /// craft's up on every title.
    ///
    /// HD's `Rocket_Init`, `Missile_Init`, `Plasma_Init` and `Shuriken_Init` store the same
    /// record's down negated (confidence 86), Omega's and 2048's `Plasma_Init` too (80, 75), so
    /// all five titles are `true` (`docs/ghidra/functions/ps3-hdfury-eu/projectile-launch-normal.md`).
    pub launch_normal_is_track_up: crate::pre_race::Sourced<bool>,
    /// Which terms of `WeaponAi_DecideFireOrAbsorb` the title's opponents roll
    /// by, on the [`Self::ai`] odds.
    pub opponent_fire: crate::pre_race::Sourced<OpponentFire>,
}

/// What differs between titles in the opponents' fire-or-absorb decision.
///
/// Pulse's skeleton (gates, rate tables, hold times) is shared; these are the
/// terms HD changed (`docs/ghidra/functions/ps3-hdfury-eu/weapon-ai-fire.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpponentFire {
    /// Multiplies the per-step rates at the 60 Hz step: HD's `dt * 30` is
    /// `0.5`, Pulse has no such term (`1.0`).
    pub step_scale: f32,
    /// Whether the `<AllWeapons>` row multiplies every weapon's odds.
    pub all_weapons: bool,
    /// Whether an Eliminator uses the `<EliminatorAIStats>` use and absorb
    /// scales (HD) or Pulse's `use *= 5` with the absorb index forced to zero.
    pub authored_eliminator: bool,
    /// What an authored Eliminator does when the title's file has no
    /// `<EliminatorAIStats>` row.
    pub eliminator_without_row: MissingEliminatorRow,
}

/// What the decision reads for an Eliminator when `WeaponAIstats.xml` has no
/// `<EliminatorAIStats>` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingEliminatorRow {
    /// Pulse's `use *= 5` with the absorb index forced to zero. HD's loader
    /// would leave its scales at zero (an Eliminator opponent never fires),
    /// which is the loader's gap, so this stands in. **Chosen, not measured.**
    PulseScale,
    /// Scales `1.0`, the absorb index as the setup gives it: Omega's loader
    /// (`0x01322f10`) defaults them so and its decision reads them as they are
    /// (`docs/ghidra/functions/ps4-omega-eu/weapon-ai-fire.md`).
    Unscaled,
}

impl OpponentFire {
    /// `WeaponAi_DecideFireOrAbsorb` on Pulse (`0x088518b4`).
    pub const PULSE: Self = Self {
        step_scale: 1.0,
        all_weapons: false,
        authored_eliminator: false,
        eliminator_without_row: MissingEliminatorRow::PulseScale,
    };
    /// `WeaponAi_DecideFireOrAbsorb` on HD (`0x000f9a18`), with `dt = 1/60`.
    pub const HD: Self = Self {
        step_scale: 0.5,
        all_weapons: true,
        authored_eliminator: true,
        eliminator_without_row: MissingEliminatorRow::PulseScale,
    };
    /// `WeaponAi_Update` on Omega (`0x01321000`): HD's terms, but a missing
    /// Eliminator row leaves the scales at the loader's `1.0`. 2048's loader
    /// (`0x811d75c6`) has the same defaults.
    pub const OMEGA: Self = Self {
        eliminator_without_row: MissingEliminatorRow::Unscaled,
        ..Self::HD
    };
}

/// How a Quake's wave reaches a craft.
///
/// Pulse's `Quake_UpdateSpan` smooths the wave's height at each craft
/// (`entity+0x870`, `h += (raw - h) * 8 * dt`), lands the hit on the rising edge
/// of `h > 0.1`, trips a laid mine on the raw height `> 0.1`, and lifts the
/// cockpit eye by `h` along the ship's up
/// (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, the section
/// on what a Quake does to the craft it reaches; `oag_weapons::projectile::quake`).
/// A title whose gate is read later gets its own variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuakeHit {
    /// The smoothed wave-height law above.
    WaveHeight {
        /// Where this came from.
        origin: crate::Origin,
    },
    /// [`Self::WaveHeight`]'s gate with the cockpit eye held level: hit, mine trip
    /// and smoothed height unchanged, the eye stays where the hull puts it. Pure,
    /// whose road does not lift under a Quake either.
    WaveHeightEyeLevel {
        /// Where the gate came from, as [`Self::WaveHeight`]'s `origin`.
        origin: crate::Origin,
        /// Where the level eye came from.
        eye_origin: crate::Origin,
    },
}

impl QuakeHit {
    /// Whether the cockpit eye rises by the craft's smoothed wave height. Read by
    /// the camera alone: the simulation computes the height either way, since the
    /// hit gate and a laid mine's trip read it.
    #[must_use]
    pub const fn lifts_cockpit_eye(self) -> bool {
        matches!(self, Self::WaveHeight { .. })
    }
}

/// When a locked LeachBeam transfers `damage` and `repair`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeachDrain {
    /// Every tick: Pulse's `LeachBeam_UpdatePool`
    /// (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`).
    PerTick,
    /// Once per ball trip, 0.3 to 1.0 s on the beam's length: HD's
    /// `LeachBeamManager_Update_q` behind `LeachBall_Advance`'s arrival count
    /// (`docs/ghidra/functions/ps3-hdfury-eu/weapons.md`, confidence 80). See
    /// `oag_weapons::projectile::leach_beam::trip`.
    PerTrip,
}

/// How a weapon blast's push reaches the craft it hits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlastPush {
    /// Pulse's: the blast adds `(1 - d / blastradius) * blastforce` to the craft's pending
    /// impulse, measured from its nose point, and `Ship_ApplyCollisionImpulse` drains it as a
    /// push along the craft's forward axis plus the `0.5` s collision stun, nothing under a
    /// running Shield (`docs/ghidra/functions/psp-pulse-usa/contact-response.md`, "Wired:
    /// every blast posts its push"). See `oag_weapons::projectile::blast`.
    Posted,
    /// A velocity change at once, with the same falloff, radially away from the craft's
    /// centre, and no stun. For a title whose own consumer is unread and whose measured pace
    /// disagrees with the stun (`docs/gameplay/weapon-lineage.md`, "Blast push").
    Immediate,
}

/// Where a title keeps each weapon's own body model: the projectile itself, not
/// the pickup pad's icon.
///
/// **An axis because HD and Pulse disagree in *name*.** `Data\Weapons\Rocket.vex`
/// resolves on no PS3 archive, so every HD projectile fell back to a procedural
/// billboard though HD authors its own models under `Data\Weapons\hd_*`
/// (`docs/ghidra/functions/ps3-hdfury-eu/plasma.md`). `None` is a title that
/// authors no such model (or where it is not recovered), as for
/// [`Weapons::elimination`]; whether a named entry resolves on the mounted
/// archives is the loader's report line.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WeaponModels {
    /// The Rocket's own body, oriented along its flight velocity.
    pub rocket: Option<&'static str>,
    /// The Mine's own body, drawn at its landed pose.
    pub mine: Option<&'static str>,
    /// The Bomb's own body - the Mine's, one size up.
    pub bomb: Option<&'static str>,
    /// The Cannon's own model, named `muzzleflash` on both titles that author one.
    /// On Pulse it is the round's body for its whole flight
    /// (`oag_raceplay::CANNON_MODEL_ENTRY`); on HD, a flash at the muzzle
    /// ([`Self::cannon_look`]).
    pub cannon: Option<&'static str>,
    /// The Plasma bolt's own head, ridden from charge through flight. Not the
    /// wind-up/travel glow (`WO_PLASMA_HEAD`/`_CHARGING`, a `Data\Psys` effect every
    /// source shares): the bolt's mesh, which only HD authors (`HD_plasma_ball`).
    pub plasma_ball: Option<&'static str>,
    /// Pulse's three-model detonation shell: a halo and two hemispheres, scrubbed by
    /// a baked anim-time track (`oag_raceplay::blast_models`).
    pub plasma_blast_pulse: Option<PulsePlasmaBlast>,
    /// HD's three-model detonation shell: a ring, a sphere and a halo, each eased
    /// by a per-tick `cur += (target - cur) * rate` scale ramp read off the
    /// executable, a different mechanism from Pulse's baked track
    /// (`docs/ghidra/functions/ps3-hdfury-eu/plasma.md`).
    pub plasma_blast_hd: Option<HdPlasmaBlast>,
    /// Pure's Disruptor explosion halo, a camera-facing model on an eased scale
    /// beside the `WO_DISRUPTOR_EXPLO` effect. `None` where there is no Disruptor.
    pub disruptor_halo: Option<DisruptorHalo>,
    /// Pure's on-craft Disruptor effect: a model drawn on the craft a bolt disrupted, for as
    /// long as the disruption runs. `None` where there is no Disruptor.
    pub disruptor_craft: Option<DisruptorCraftFx>,
    /// Pulse's Bomb detonation: a hemisphere and a shockwave, each eased by a
    /// per-tick `cur += (target - cur) * rate` scale ramp (`oag_raceplay::bomb_blast`).
    /// `None` on HD (see [`Self::bomb_blast_hd`]) and where the Bomb is unread.
    pub bomb_blast_pulse: Option<PulseBombBlast>,
    /// HD's Bomb detonation: a fireball, its white core, a camera-facing bloom disc
    /// and eight shockwave rings, driven by one blast object (`NormalBomb`'s, vtable
    /// `0x00864ab8`), a different mechanism from Pulse's
    /// (`oag_raceplay::bomb_blast::hd`;
    /// `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`, "2026-10-07").
    pub bomb_blast_hd: Option<HdBombBlast>,
    /// HD's Missile detonation model, `Data\Weapons\HD_missile_explosion.vex`
    /// (`MissileManager`'s pool of sixteen, `0x00154cf0` loads it): played when a
    /// Missile reaches a craft, on its own keys over one second
    /// (`oag_raceplay::missile_blast`; `weapons.md`, "2026-10-07").
    pub missile_blast_hd: Option<&'static str>,
    /// The Repulser's field model: a flat ring around the firer that shrinks, then
    /// widens and fades. `Repulser_Construct` (`0x08875008`) loads it and
    /// `Repulser_UpdateFieldModel` (`0x088758cc`) eases it
    /// (`oag_raceplay::repulser_field`). `None` without a Repulser law of its own.
    pub repulser_field: Option<&'static str>,
    /// The two models a craft shows on a magstrip, `[MagEffect1, MagEffect2]`: one
    /// riding the craft, one laid on the track, both `(0, -2.5, 0)` below it.
    /// `MagFloorFx_Construct` (`0x088590a8`) loads them for every craft
    /// (`oag_raceplay::mag_floor_fx`). `None` where the archive was not checked or
    /// lacks them.
    pub mag_floor: Option<[&'static str; 2]>,
    /// The HD-lineage magstrip arc wake: `MagstripWake` (`MagstripWake.cpp`), one
    /// object per craft, drawn over a magstrip (`oag_raceplay::magstrip_wake`,
    /// `oag_fx::magstrip`). `None` where the class is not built: Pulse and Pure draw
    /// their two-mesh `mag_floor`, and the 2048 modes play a `.pob`.
    pub magstrip_wake: Option<MagstripWake>,
    /// The `.pob` magstrip effect a craft plays over a strip instead of the arc
    /// wake: `WO_MAGSTRIP_SPARKS`, or `WO_MAGSTRIP_ZONE` in a Zone. The Vita 2048
    /// build gates it on the opposite sense of the predicate that builds the wake,
    /// so a title sets this or [`Self::magstrip_wake`], never both. 2048's events
    /// carry a CRC-id mode (`>= 0x17`), the `.pob` side, measured live
    /// (`docs/ghidra/functions/vita-2048-eu-v104/ships-effects.md`;
    /// `oag_raceplay::magstrip_wake`).
    pub magstrip_pob: bool,
    /// The Rocket's smoke ribbon: `RibbonEffects` pool 4, its texture and the opacity
    /// ramp its nodes age through (`oag_fx::rocket_smoke`). `None` where the ribbon
    /// law is unread, every title but HD.
    pub rocket_trail: Option<RocketTrail>,
    /// The flying Missile's own ribbons: per head a short stub and a helix, both
    /// `FatLine` elements (`oag_fx::fat_line`). `None` where the Missile's ribbons
    /// are unread, every title but HD.
    pub missile_trail: Option<MissileTrail>,
    /// HD's LeachBeam strip: the ribbon `LeachBeam_DrawStrip` builds from the
    /// target's anchor trail and its two textures (`oag_fx::leach_strip`). `None`
    /// where Pulse's ribbon is drawn instead.
    pub leach_strip: Option<LeachStrip>,
    /// The LeachBeam's own ball, at the drawing end of the beam. **Named, not
    /// wired**: what places the model each tick is unread (see
    /// `oag_raceplay::load::weapon_models`).
    pub leachbeam_ball: Option<&'static str>,
    /// The flying Missile's own head: one model, built twice per missile
    /// (`Missile.cpp`'s constructor, `0x0011ccc8`), drawn in place of the
    /// glow billboard a Missile otherwise rides. The LeachBeam ball shares its
    /// materials. **Placement is chosen, not measured** (see
    /// `oag_raceplay::Race::missile_head_model_matrices`). `None` where the
    /// billboard is the Missile's body (Pulse's body is a flare effect).
    pub missile_head: Option<&'static str>,
    /// The Shuriken's own blade, drawn on the Rocket dart's basis. `None` where the
    /// blade model is not recovered.
    pub shuriken: Option<&'static str>,
    /// How the Cannon's two hand-built quads and its [`Self::cannon`] model are
    /// drawn, where a title's executable says so.
    ///
    /// `None` is Pulse's reading, recovered first and carried by
    /// `oag_fx::weapon_quads::geometry`'s constants and
    /// `oag_raceplay::CANNON_BOLT_TEXTURE_ENTRY`.
    pub cannon_look: Option<CannonLook>,
    /// The texture the ghost ship's third pass stamps over the player's view, or
    /// `None` for a title that binds none.
    ///
    /// **`None` is a measurement on HD only**: `MeshNode_Ghost_LoadStaticGlow`
    /// (`0x08910ef8`) names `Data\Tex\staticglow.mip` in Pulse's `BOOT.BIN`, and a
    /// string search of HD's `EBOOT.elf` finds no `staticglow`
    /// (`docs/ghidra/functions/psp-pulse-usa/ghost.md`).
    pub ghost_static: Option<GhostStatic>,
    /// The noise the cockpit view's hit static is drawn from, or `None` for a title
    /// that draws no such static.
    ///
    /// `CockpitHit` (`Ship_Damage` arms it on every weapon hit to the human,
    /// `CockpitHitFx_Draw` queues it in the internal view only) is measured on
    /// Pulse's PSP (`oag_fx::cockpit_hit`). **`None` is a measurement on HD and
    /// 2048**: a string search of HD's `EBOOT.elf` and 2048's decrypted `eboot.elf`
    /// finds no `noisematt`. Omega's executable cannot be read here, so it follows
    /// HD (`None`, not measured).
    pub cockpit_hit: Option<CockpitHit>,
}

/// The cockpit hit static's texture a title binds - see [`WeaponModels::cockpit_hit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CockpitHit {
    /// The archive entry, spelled as Pulse spells it (`.mip`); the loader swaps the
    /// extension for another source's container.
    pub entry: &'static str,
    /// Where the entry came from: `Measured` where the title's executable names it
    /// and its arm and draw were read, `InheritedFrom` where it names the texture
    /// but the weapon-hit call that arms it is unread, so the law is Pulse's.
    pub origin: crate::Origin,
}

/// The ghost static texture a title binds - see [`WeaponModels::ghost_static`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GhostStatic {
    /// The archive entry, spelled as Pulse spells it (`.mip`); the loader swaps the
    /// extension for a Vita or PS4 source.
    pub entry: &'static str,
    /// Where the entry came from: `Measured` where the title's own executable
    /// names it, `InheritedFrom` where it ships the file and runs Pulse's law.
    pub origin: crate::Origin,
}

/// The two textures `MagstripWake` is built from.
///
/// Archive entry names, read through the archive set a race loads from, so a
/// PSARC's case and separator folding applies. A title shipping another container
/// (Omega's `.gnf`) names that spelling here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagstripWake {
    /// `HD_electric_arc_8x8`: the 8-by-8 atlas the arc body samples.
    pub atlas: &'static str,
    /// `HD_ElectricArc_Contact`: the quad at the arc's far end.
    pub contact: &'static str,
}

/// What the Rocket's smoke ribbon draws with - see [`WeaponModels::rocket_trail`].
/// Archive entry names, folded as [`MagstripWake`]'s are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RocketTrail {
    /// The texture the ribbon's material samples in a race.
    pub texture: &'static str,
    /// The 256-entry alpha table, as the executable names it.
    pub opacity_ramp: &'static str,
}

/// One `FatLine` element's config block: the numbers the engine copies out of its
/// static table into the element (`docs/ghidra/functions/ps3-hdfury-eu/missile-ribbon.md`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FatLineSpec {
    /// Half-width of a node when it is laid, and as it dies.
    pub width: [f32; 2],
    /// Seconds a node lives.
    pub lifetime: f32,
    /// Texture `u` per world unit of arc length.
    pub u_per_unit: f32,
    /// Chains laid per push, spaced `pi / chains` apart in phase.
    pub chains: u32,
    /// Nodes one chain's draw buffer holds.
    pub capacity: u32,
    /// Phase added after each chain of a push, radians.
    pub phase_step: f32,
    /// Distance of each node from the anchor, along the rotating reference axis.
    pub radius: f32,
    /// Whether a node's alpha reads the opacity ramp (else `1 - age`).
    pub ramp: bool,
}

/// What a flying Missile's ribbons draw with - see [`WeaponModels::missile_trail`].
/// Each of its two heads lays one `stub` and one `helix`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MissileTrail {
    /// The texture both elements sample, an archive entry name folded as
    /// [`MagstripWake`]'s are.
    pub texture: &'static str,
    /// The short straight element (`RibbonEffects` pool `+0x50`).
    pub stub: FatLineSpec,
    /// The winding element (pool `+0x68`).
    pub helix: FatLineSpec,
    /// The Missile explosion's sixteen streaks, drawn with the same element class from a
    /// pool of their own. `None` where the explosion's streaks are unread.
    pub blast_streak: Option<BlastStreak>,
}

/// The Missile explosion's streak element: `RibbonEffects` pool `+0x5c`, twenty elements
/// shared by every live explosion (`docs/ghidra/functions/ps3-hdfury-eu/missile-explosion.md`,
/// "The sixteen entries").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlastStreak {
    /// The texture the pool's config names, an archive entry name.
    pub texture: &'static str,
    /// The pool's config block.
    pub line: FatLineSpec,
    /// Elements in the pool.
    pub pool: usize,
}

/// What HD's LeachBeam strip draws with - see [`WeaponModels::leach_strip`].
///
/// Archive entry names. `hd_leachbeam.rcsmaterial` samples the noise on unit 0 and
/// the glow on unit 1 (sizes read off the live draw, names off
/// `leachbeam_triangle.rcsmodel`'s material).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeachStrip {
    /// `hd_leechbeam_glow`: 256x256, unit 1.
    pub glow: &'static str,
    /// `hd_waketrail_clouds`: 128x128, unit 0.
    pub noise: &'static str,
}

impl WeaponModels {
    /// Every field absent: a title with none of these recovered. A `const` because
    /// `#[derive(Default)]`'s `default()` is not `const fn` and cannot sit inside a
    /// `Title` const's initialiser.
    pub const EMPTY: Self = Self {
        rocket: None,
        mine: None,
        bomb: None,
        cannon: None,
        plasma_ball: None,
        plasma_blast_pulse: None,
        plasma_blast_hd: None,
        disruptor_halo: None,
        disruptor_craft: None,
        bomb_blast_pulse: None,
        bomb_blast_hd: None,
        missile_blast_hd: None,
        repulser_field: None,
        mag_floor: None,
        magstrip_wake: None,
        magstrip_pob: false,
        rocket_trail: None,
        missile_trail: None,
        leach_strip: None,
        leachbeam_ball: None,
        missile_head: None,
        shuriken: None,
        cannon_look: None,
        ghost_static: None,
        cockpit_hit: None,
    };
}

/// A title's own reading of how a Cannon round is drawn - see
/// [`WeaponModels::cannon_look`].
///
/// Only the parts that differ between executables read so far: the flash window
/// (`0.1` s) and the flash quad's `* 3.0` size scale read the same on Pulse and HD
/// and stay `oag_render`'s constants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CannonLook {
    /// The bolt streak's texture entry.
    pub bolt_texture: &'static str,
    /// The muzzle flash quad's texture entry.
    pub flash_texture: &'static str,
    /// The bolt streak's half-width, world units.
    pub bolt_half_width: f32,
    /// How much of the previous-to-current segment, from the current end, the
    /// streak leaves uncovered. `0.0` draws the whole segment.
    pub bolt_near_fraction: f32,
    /// The range the flash's half-size is rolled from each tick of the flash
    /// window, before the `* 3.0` scale.
    pub flash_size_range: (f32, f32),
    /// Where [`WeaponModels::cannon`] is drawn.
    pub body: CannonBody,
}

/// Where a title hangs its Cannon model - see [`CannonLook::body`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CannonBody {
    /// On the round, for its whole flight: Pulse's dart.
    Round,
    /// At the firing craft's `cannon_flash` locator, only for the flash window,
    /// scaled by the flash quad's half-size roll and stretched along its Z by a
    /// second roll from `stretch_range`.
    Muzzle {
        /// The range the Z stretch is rolled from each tick.
        stretch_range: (f32, f32),
    },
}

/// A three-model Plasma detonation shell: a halo and two hemispheres, in
/// `PlasmaBlast_Construct`'s load order - see [`WeaponModels::plasma_blast_pulse`].
/// Pulse and Pure run the same mechanism (a grey tint ramp and an eased scale
/// per model) over their own models and constants ([`Self::law`]); the fields
/// keep Pulse's names and are matched by ordinal position (Pure's
/// `plasma_hemisphere_noglow` is the second, `plasma_hemisphere` the third).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PulsePlasmaBlast {
    /// The halo: `Data\Weapons\pulse_plasma_halo1.vex`, Pure's `plasma_halo.vex`.
    pub halo: &'static str,
    /// The second model, loaded before [`Self::hemisphere1`] (the file's order):
    /// `Data\Weapons\pulse_plasma_hemisphere2.vex`, Pure's `plasma_hemisphere_noglow.vex`.
    pub hemisphere2: &'static str,
    /// The third: `Data\Weapons\pulse_plasma_hemisphere1.vex`, Pure's `plasma_hemisphere.vex`.
    pub hemisphere1: &'static str,
    /// The tints, scale eases, life and placement this title's `PlasmaBlast_Update` runs.
    pub law: PlasmaBlastLaw,
}

/// One title's Plasma-blast constants, in [`PulsePlasmaBlast`]'s ordinal order
/// (halo, second model, third model).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlasmaBlastLaw {
    /// Each model's grey tint as `(hold, end)` seconds of age: full white up to
    /// `hold`, linear to black at `end`, black after (a three-key table `255,
    /// 255, 0` at `0, hold, end`).
    pub tint: [(f32, f32); 3],
    /// Each model's scale-ease target, `cur += (target - cur) * rate` per 60 Hz
    /// tick from `0`, in the unit `oag_raceplay`'s `PULSE_MODEL_UNIT` multiplies
    /// by the model's extent: the executable's own target divided by that unit.
    pub ease_targets: [f32; 3],
    /// Each model's ease rate per tick.
    pub ease_rates: [f32; 3],
    /// The age at which the object retires, seconds.
    pub life: f32,
    /// The age at which the halo stops drawing, if the title hides it before
    /// its tint reaches black. `None` where nothing hides it.
    pub halo_hide_at: Option<f32>,
    /// How far the two hemispheres sit below the impact point along the
    /// track's up, world units (`PlasmaBlast_Construct`'s `-3.5 x row1`).
    /// `0.0` where the offset is read but not yet applied.
    pub hemisphere_drop: f32,
    /// Where these constants come from.
    pub origin: crate::Origin,
}

/// Pure's Disruptor explosion halo - see [`WeaponModels::disruptor_halo`]
/// (`DisruptorExplo_Update`, `0x0885a5bc`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisruptorHalo {
    /// `Data\Weapons\electric_halo2.vex`.
    pub model: &'static str,
    /// The eased scale's target: `cur += (target - cur) * rate` per 60 Hz tick
    /// from `0`, as a multiple of the model's own extent.
    pub target: f32,
    /// The ease's rate per tick.
    pub rate: f32,
    /// The age the object retires at, seconds; its grey tint is `1 - age / life`
    /// before that.
    pub life: f32,
    /// Where these constants come from.
    pub origin: crate::Origin,
}

/// Pure's on-craft Disruptor effect - see [`WeaponModels::disruptor_craft`]
/// (`DisruptorFx_Construct`, `0x08859b78`, and its update `0x0885a010`).
///
/// The object is built when a bolt lands on a craft and eases a scale-and-grey value `cur`
/// from `0` towards [`Self::target`] by `cur += (target - cur) * rate` every 60 Hz tick; when
/// the disruption's timer runs out the target drops to `0` and the object is freed once `cur`
/// is under [`Self::retire_below`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisruptorCraftFx {
    /// `Data\Weapons\disruptor_effect.vex`, drawn with the craft's own matrix. The second
    /// model the object loads, `disruptor_cockpit.vex`, is the cockpit view's and is not read.
    pub model: &'static str,
    /// The ease's target while the disruption runs (`+0x1a0`).
    pub target: f32,
    /// The ease's rate per tick (`+0x1a8`).
    pub rate: f32,
    /// The object is freed once the eased value falls below this after the timer ran out.
    pub retire_below: f32,
    /// Where these constants come from.
    pub origin: crate::Origin,
}

/// HD's plasma-blast trio, in `WeaponExplosions_Start`'s load order - see
/// [`WeaponModels::plasma_blast_hd`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HdPlasmaBlast {
    /// `Data\Weapons\HD_plasma_ring.vex`, target scale 100.0, rate 0.01.
    pub ring: &'static str,
    /// `Data\Weapons\HD_plasma_sphere.vex`, target scale 7.1, rate 0.3.
    pub sphere: &'static str,
    /// `Data\Weapons\HD_plasma_halo.vex`, target scale 7.0, rate 0.2.
    pub halo: &'static str,
}

/// HD's Bomb-blast set, in the blast constructor's load order (`0x00151ad8`, TOC
/// slots `0x008ab18c..`) - see [`WeaponModels::bomb_blast_hd`]. The blast holds
/// the shockwave model eight times over; that count is the executable's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HdBombBlast {
    /// `Data\Weapons\HD_bomb_sphere.vex`, the fireball (`BOMB_FIRE`).
    pub sphere: &'static str,
    /// `Data\Weapons\HD_bomb_sphere_white.vex`, the same mesh, hotter.
    pub sphere_white: &'static str,
    /// `Data\Weapons\hd_bomb_sphere_bloomring.vex`, a flat bloom disc.
    pub bloom_ring: &'static str,
    /// `Data\Weapons\hd_bomb_shockwaves.vex`, one ring's mesh.
    pub shockwaves: &'static str,
    /// `Data\Weapons\HD_bomb_halo.vex`, the *laid* bomb's pulsing shell
    /// (`NormalBomb`'s second model, not the blast's); here because HD is the only
    /// title with one.
    pub halo: &'static str,
}

/// Pulse's Bomb-blast pair, in `BombBlast_Construct`'s load order - see
/// [`WeaponModels::bomb_blast_pulse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PulseBombBlast {
    /// `Data\Weapons\explosion_hemisphere.vex`, uniform scale 2.0 -> 4.0,
    /// rate 0.1, hidden past 1.55s.
    pub hemisphere: &'static str,
    /// `Data\Weapons\Bomb_Shockwave.vex`, radial scale 0.0 -> 12.0, rate
    /// 0.075, gated to start after 0.1s.
    pub shockwave: &'static str,
    /// Whether the shockwave's alpha eases `1.0 -> 0.0` while it widens. Pulse's
    /// `BombBlast_Update` stamps that ease onto the ring's vertex colours every
    /// tick; Pure's (`0x08858ad0`) makes no such call and leaves the authored alpha.
    pub shockwave_fades: bool,
}
