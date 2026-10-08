//! Where a title keeps the table its weapons are tuned by.
//!
//! An axis in the [ADR-0022] sense rather than a constant, because the two
//! answers differ in **shape** and not only in spelling: Pulse and Wipeout HD
//! ship two tables and pick between them by race mode, and Pure ships one that
//! every mode reads. A single `&str` could carry the spelling and could not
//! carry that.
//!
//! Recovered from each title's own executable. Pulse's two names are in
//! `oag_tables::weapons`' own constants, taken off `BOOT.BIN`'s string table;
//! Pure's is `Data\XML\weaponstats.xml` at `0x08a445a0` in
//! `/psp-pure-usa/BOOT.BIN`, three strings before the `"WeaponStats"` and
//! `"Weapon"` element names the parser matches - which is what says it is the
//! file this parser reads rather than some other table.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md

/// The tables a title tunes its weapons from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weapons {
    /// The table a race reads, and the only one on a title that ships one.
    pub race: &'static str,
    /// The table Eliminator reads instead, or `None` for a title with one table.
    ///
    /// **`None` is a measurement.** `WeaponStats_Elimination.xml` is how Pulse
    /// makes the same weapon hit differently in Eliminator - the global
    /// `DAT_08b32428` selects the file and not the speed class, recovered on
    /// `docs/ghidra/functions/psp-pulse-usa/missile.md`. Pure's single
    /// `weaponstats.xml` is the whole of its tuning, so there is no second file
    /// for a mode to reach for rather than one this build has not found.
    ///
    /// Nothing reads it yet: `oag_raceplay::load` opens [`Self::race`] for
    /// every mode. It is carried because the axis is about which files exist,
    /// and recording that Pure has one is the point of the axis.
    pub elimination: Option<&'static str>,
    /// The table an opponent's fire odds are read from - `useAgainstPlayer`,
    /// `useAgainstAI` and `absorb` a weapon, `oag_tables::weapons::ai` - or
    /// `None` where it has not been looked for.
    ///
    /// **`None` is not a measurement here**, unlike [`Self::elimination`]'s:
    /// Pulse's is read by `weapon_ai_ground_truth`, and Pure's `Data.wad`
    /// carries an entry under the same name's hash (`0xac744a8d`, case folded
    /// by the hash, so the spelling is Pulse's) that parses (2026-10-03). HD,
    /// Omega and 2048 each ship one too (searched the same day), so **every
    /// title here runs Pulse's law on its own odds; no title's own decision
    /// code is read** - the law is inherited from Pulse and unmeasured on the
    /// rest. A race without the file fires on this project's own rule - see
    /// `oag_raceplay::FireLaw` - which is what a title with no table would
    /// get, and none of the five does.
    pub ai: Option<&'static str>,
}

/// Where a title keeps each weapon's own body model - the projectile itself,
/// not the pickup pad's icon.
///
/// **An axis because HD and Pulse disagree in *name*, not just in whether a
/// field is `None`.** `oag_raceplay`'s own `ROCKET_MODEL_ENTRY` and friends
/// were Pulse spellings reached for on every source - `Data\Weapons\Rocket.vex`
/// resolves on no PS3 archive at all, so every HD projectile fell back to a
/// procedural billboard even though HD authors its own models under
/// `Data\Weapons\hd_*` - see `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`.
/// `None` on a field is a title that authors no such model (or where it is not
/// yet recovered), the same convention [`Weapons::elimination`] uses; whether a
/// named entry actually resolves on the mounted archives is a load-time
/// question the loader's own report line answers, not this table's.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WeaponModels {
    /// The Rocket's own body, oriented along its flight velocity.
    pub rocket: Option<&'static str>,
    /// The Mine's own body, drawn at its landed pose.
    pub mine: Option<&'static str>,
    /// The Bomb's own body - the Mine's, one size up.
    pub bomb: Option<&'static str>,
    /// The Cannon's own model, named `muzzleflash` on both titles that author
    /// one. On Pulse it is the round's body for its whole flight (see
    /// `oag_raceplay::CANNON_MODEL_ENTRY`); on Wipeout HD it is what the
    /// name says, a flash at the muzzle - [`Self::cannon_look`] says which.
    pub cannon: Option<&'static str>,
    /// The Plasma bolt's own head, ridden from charge through flight.
    ///
    /// **Not the wind-up/travel glow** - that is `WO_PLASMA_HEAD`/`_CHARGING`,
    /// a `Data\Psys` effect every source shares. This is the bolt's own mesh,
    /// which only HD authors (`HD_plasma_ball`); Pulse rides the glow alone.
    pub plasma_ball: Option<&'static str>,
    /// Pulse's own three-model detonation shell: a halo and two hemispheres,
    /// scrubbed by a baked anim-time track. See
    /// `oag_raceplay::blast_models` for the mechanism this plays back.
    pub plasma_blast_pulse: Option<PulsePlasmaBlast>,
    /// Wipeout HD's own three-model detonation shell: a ring, a sphere and a
    /// halo, each eased by a per-tick `cur += (target - cur) * rate` scale
    /// ramp read off the executable - a different mechanism from Pulse's baked
    /// track, not a renamed copy of it. See
    /// `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`.
    pub plasma_blast_hd: Option<HdPlasmaBlast>,
    /// Pulse's own Bomb detonation: a hemisphere and a shockwave, each eased
    /// by a per-tick `cur += (target - cur) * rate` scale ramp of its own -
    /// see `oag_raceplay::bomb_blast`. `None` on every other title: HD's
    /// own Bomb detonation is unread and Pure's Bomb authors no fuse at all.
    pub bomb_blast_pulse: Option<PulseBombBlast>,
    /// Wipeout HD's Bomb detonation: a fireball, its white core, a camera-
    /// facing bloom disc and eight shockwave rings, driven by one blast
    /// object (`NormalBomb`'s, vtable `0x00864ab8`) - a different mechanism
    /// from Pulse's, not a renamed copy. See `oag_raceplay::bomb_blast::hd`
    /// and `docs/ghidra/functions/ps3-hdfury-eu/weapons.md`, 2026-10-07.
    pub bomb_blast_hd: Option<HdBombBlast>,
    /// Wipeout HD's Missile detonation model, `Data\Weapons\HD_missile_explosion.vex`
    /// (`MissileManager`'s pool of sixteen, `0x00154cf0` loads it): played when
    /// a Missile reaches a craft, on its own keys over one second. See
    /// `oag_raceplay::missile_blast` and `weapons.md`, 2026-10-07.
    pub missile_blast_hd: Option<&'static str>,
    /// The Repulser's field model: a flat ring around the firer that shrinks,
    /// then widens and fades - `Repulser_Construct` (`0x08875008`) loads it and
    /// `Repulser_UpdateFieldModel` (`0x088758cc`) eases it. See
    /// `oag_raceplay::repulser_field`. `None` on every title without a
    /// Repulser law of its own.
    pub repulser_field: Option<&'static str>,
    /// The two models a craft shows while it is on a magstrip, `[MagEffect1,
    /// MagEffect2]`: one riding the craft, one laid on the track, both
    /// `(0, -2.5, 0)` below it. `MagFloorFx_Construct` (`0x088590a8`) loads
    /// them for every craft. See `oag_raceplay::mag_floor_fx`. `None` on every
    /// title whose archive was not checked for them or does not carry them.
    pub mag_floor: Option<[&'static str; 2]>,
    /// The HD-lineage magstrip arc wake: `MagstripWake` (`MagstripWake.cpp`),
    /// one object per craft, drawn while it is over a magstrip. See
    /// `oag_raceplay::magstrip_wake` and `oag_fx::magstrip`. `None` on every
    /// title that does not build the class - Pulse and Pure draw their own
    /// two-mesh `mag_floor` instead, and the 2048 modes play a `.pob`.
    pub magstrip_wake: Option<MagstripWake>,
    /// The `.pob` magstrip effect a craft plays over a strip instead of the
    /// arc wake: `WO_MAGSTRIP_SPARKS`, or `WO_MAGSTRIP_ZONE` in a Zone. The
    /// Vita 2048 build gates it on the opposite sense of the predicate that
    /// builds the wake, so a title sets this or [`Self::magstrip_wake`], never
    /// both. 2048's own events carry a CRC-id mode (`>= 0x17`), which is the
    /// `.pob` side - measured live, `docs/ghidra/functions/vita-2048-eu-v104/
    /// ships-effects.md`. See `oag_raceplay::magstrip_wake`.
    pub magstrip_pob: bool,
    /// The Rocket's smoke ribbon: `RibbonEffects` pool 4, its texture and the
    /// opacity ramp its nodes age through. See `oag_fx::rocket_smoke`. `None`
    /// on a title whose ribbon law is unread, which is every title but HD's.
    pub rocket_trail: Option<RocketTrail>,
    /// Wipeout HD's LeachBeam strip: the ribbon `LeachBeam_DrawStrip` builds
    /// from the target's anchor trail and its two textures. `None` on every
    /// title that draws Pulse's own ribbon instead. See `oag_fx::leach_strip`.
    pub leach_strip: Option<LeachStrip>,
    /// The LeachBeam's own ball, at the drawing end of the beam.
    ///
    /// **Named, not wired.** What places this model each tick has not been
    /// read this session - see `oag_raceplay::load::weapon_models`'s own
    /// doc comment for where that stopped.
    pub leachbeam_ball: Option<&'static str>,
    /// The Shuriken's own blade, drawn on the same basis the Rocket's dart
    /// is. `None` on a title whose blade model is not recovered.
    pub shuriken: Option<&'static str>,
    /// How the Cannon's two hand-built quads and its [`Self::cannon`] model
    /// are drawn, where a title's own executable says so.
    ///
    /// `None` is Pulse's own reading, the one recovered first and the one
    /// `oag_fx::weapon_quads::geometry`'s constants and
    /// `oag_raceplay::CANNON_BOLT_TEXTURE_ENTRY` already carry - every
    /// title without its own reading keeps drawing on those terms, as it did
    /// before this field existed.
    pub cannon_look: Option<CannonLook>,
    /// The texture the ghost ship's third pass stamps over the player's view,
    /// or `None` for a title that binds none.
    ///
    /// **`None` is a measurement on HD only**: `MeshNode_Ghost_LoadStaticGlow`
    /// (`0x08910ef8`) names `Data\Tex\staticglow.mip` in Pulse's `BOOT.BIN`,
    /// and a string search of HD's `EBOOT.elf` finds no `staticglow` at all, so
    /// HD never loads one. See `docs/ghidra/functions/psp-pulse-usa/ghost.md`.
    pub ghost_static: Option<GhostStatic>,
}

/// The ghost static texture a title binds - see [`WeaponModels::ghost_static`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GhostStatic {
    /// The archive entry, spelled as Pulse spells it (`.mip`); the loader swaps
    /// the extension for a Vita or PS4 source.
    pub entry: &'static str,
    /// Where the entry came from: `Measured` where the title's own executable
    /// names it, `InheritedFrom` where it ships the file and runs Pulse's law.
    pub origin: crate::Origin,
}

/// The two textures and the one sound cue `MagstripWake` is built from.
///
/// Names are archive entry names; the loader reads them through the same
/// archive set a race loads from, so the case and separator folding of a PSARC
/// applies. A title that ships them as another container (Omega's `.gnf`) names
/// that spelling here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MagstripWake {
    /// `HD_electric_arc_8x8`: the 8-by-8 atlas the arc body samples.
    pub atlas: &'static str,
    /// `HD_ElectricArc_Contact`: the quad at the arc's far end.
    pub contact: &'static str,
}

/// What the Rocket's smoke ribbon draws with - see [`WeaponModels::rocket_trail`].
///
/// Archive entry names, folded the same way [`MagstripWake`]'s are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RocketTrail {
    /// The texture the ribbon's material samples in a race.
    pub texture: &'static str,
    /// The 256-entry alpha table, as the executable names it.
    pub opacity_ramp: &'static str,
}

/// What HD's LeachBeam strip draws with - see [`WeaponModels::leach_strip`].
///
/// Archive entry names. `hd_leachbeam.rcsmaterial` samples the noise on unit 0
/// and the glow on unit 1 (the two sizes read off the live draw, the names off
/// `leachbeam_triangle.rcsmodel`'s own material).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeachStrip {
    /// `hd_leechbeam_glow`: 256x256, unit 1.
    pub glow: &'static str,
    /// `hd_waketrail_clouds`: 128x128, unit 0.
    pub noise: &'static str,
}

impl WeaponModels {
    /// Every field absent - a title with none of these recovered.
    ///
    /// A `const` rather than routing every all-`None` `TITLE` through
    /// `#[derive(Default)]`'s own `default()`, which is not itself `const fn`
    /// and so cannot sit inside another `const`'s initialiser the way a
    /// `Title` const does.
    pub const EMPTY: Self = Self {
        rocket: None,
        mine: None,
        bomb: None,
        cannon: None,
        plasma_ball: None,
        plasma_blast_pulse: None,
        plasma_blast_hd: None,
        bomb_blast_pulse: None,
        bomb_blast_hd: None,
        missile_blast_hd: None,
        repulser_field: None,
        mag_floor: None,
        magstrip_wake: None,
        magstrip_pob: false,
        rocket_trail: None,
        leach_strip: None,
        leachbeam_ball: None,
        shuriken: None,
        cannon_look: None,
        ghost_static: None,
    };
}

/// A title's own reading of how a Cannon round is drawn - see
/// [`WeaponModels::cannon_look`].
///
/// Only the parts that differ between the executables read so far are here:
/// the flash window (`0.1` s) and the flash quad's `* 3.0` size scale read the
/// same on both Pulse and HD, and stay `oag_render`'s own constants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CannonLook {
    /// The bolt streak's texture entry.
    pub bolt_texture: &'static str,
    /// The muzzle flash quad's texture entry.
    pub flash_texture: &'static str,
    /// The bolt streak's half-width, world units.
    pub bolt_half_width: f32,
    /// How much of the previous-to-current segment, from the current end,
    /// the streak leaves uncovered. `0.0` draws the whole segment.
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
    /// At the firing craft's `cannon_flash` locator, only for the flash
    /// window, scaled by the same roll the flash quad's half-size takes and
    /// stretched along its own Z by a second roll from `stretch_range`.
    Muzzle {
        /// The range the Z stretch is rolled from each tick.
        stretch_range: (f32, f32),
    },
}

/// Pulse's own plasma-blast trio, in `PlasmaBlast_Construct`'s load order -
/// see [`WeaponModels::plasma_blast_pulse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PulsePlasmaBlast {
    /// `Data\Weapons\pulse_plasma_halo1.vex`.
    pub halo: &'static str,
    /// `Data\Weapons\pulse_plasma_hemisphere2.vex` - loaded before
    /// [`Self::hemisphere1`], the file's own order.
    pub hemisphere2: &'static str,
    /// `Data\Weapons\pulse_plasma_hemisphere1.vex`.
    pub hemisphere1: &'static str,
}

/// Wipeout HD's own plasma-blast trio, in `WeaponExplosions_Start`'s load
/// order - see [`WeaponModels::plasma_blast_hd`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HdPlasmaBlast {
    /// `Data\Weapons\HD_plasma_ring.vex`, target scale 100.0, rate 0.01.
    pub ring: &'static str,
    /// `Data\Weapons\HD_plasma_sphere.vex`, target scale 7.1, rate 0.3.
    pub sphere: &'static str,
    /// `Data\Weapons\HD_plasma_halo.vex`, target scale 7.0, rate 0.2.
    pub halo: &'static str,
}

/// Wipeout HD's own Bomb-blast set, in the blast constructor's load order
/// (`0x00151ad8`, TOC slots `0x008ab18c..`) - see
/// [`WeaponModels::bomb_blast_hd`]. The blast holds the shockwave model eight
/// times over; that count is the executable's, not this table's.
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
    /// (`NormalBomb`'s second model, not the blast's). It rides this struct
    /// because HD is the only title that has one.
    pub halo: &'static str,
}

/// Pulse's own Bomb-blast pair, in `BombBlast_Construct`'s load order - see
/// [`WeaponModels::bomb_blast_pulse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PulseBombBlast {
    /// `Data\Weapons\explosion_hemisphere.vex`, uniform scale 2.0 -> 4.0,
    /// rate 0.1, hidden past 1.55s.
    pub hemisphere: &'static str,
    /// `Data\Weapons\Bomb_Shockwave.vex`, radial scale 0.0 -> 12.0, rate
    /// 0.075, gated to start after 0.1s.
    pub shockwave: &'static str,
    /// Whether the shockwave's alpha eases `1.0 -> 0.0` while it widens.
    /// Pulse's `BombBlast_Update` stamps that ease onto the ring's vertex
    /// colours every tick; Pure's update (`0x08858ad0`) makes no such call
    /// and leaves the ring at its authored alpha.
    pub shockwave_fades: bool,
}
