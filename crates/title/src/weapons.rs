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
    /// Nothing reads it yet: `oag_game::race::load` opens [`Self::race`] for
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
    /// by the hash, so the spelling is Pulse's). No other title's archives were
    /// searched. A race without it fires on this project's own rule rather than
    /// the original's - see `oag_game::race::FireLaw`.
    pub ai: Option<&'static str>,
}

/// Where a title keeps each weapon's own body model - the projectile itself,
/// not the pickup pad's icon.
///
/// **An axis because HD and Pulse disagree in *name*, not just in whether a
/// field is `None`.** `oag_game::race`'s own `ROCKET_MODEL_ENTRY` and friends
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
    /// `oag_game::race::CANNON_MODEL_ENTRY`); on Wipeout HD it is what the
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
    /// `oag_game::race::blast_models` for the mechanism this plays back.
    pub plasma_blast_pulse: Option<PulsePlasmaBlast>,
    /// Wipeout HD's own three-model detonation shell: a ring, a sphere and a
    /// halo, each eased by a per-tick `cur += (target - cur) * rate` scale
    /// ramp read off the executable - a different mechanism from Pulse's baked
    /// track, not a renamed copy of it. See
    /// `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`.
    pub plasma_blast_hd: Option<HdPlasmaBlast>,
    /// Pulse's own Bomb detonation: a hemisphere and a shockwave, each eased
    /// by a per-tick `cur += (target - cur) * rate` scale ramp of its own -
    /// see `oag_game::race::bomb_blast`. `None` on every other title: HD's
    /// own Bomb detonation is unread and Pure's Bomb authors no fuse at all.
    pub bomb_blast_pulse: Option<PulseBombBlast>,
    /// The LeachBeam's own ball, at the drawing end of the beam.
    ///
    /// **Named, not wired.** What places this model each tick has not been
    /// read this session - see `oag_game::race::load::weapon_models`'s own
    /// doc comment for where that stopped.
    pub leachbeam_ball: Option<&'static str>,
    /// How the Cannon's two hand-built quads and its [`Self::cannon`] model
    /// are drawn, where a title's own executable says so.
    ///
    /// `None` is Pulse's own reading, the one recovered first and the one
    /// `oag_render::weapon_quads::geometry`'s constants and
    /// `oag_game::race::CANNON_BOLT_TEXTURE_ENTRY` already carry - every
    /// title without its own reading keeps drawing on those terms, as it did
    /// before this field existed.
    pub cannon_look: Option<CannonLook>,
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
        leachbeam_ball: None,
        cannon_look: None,
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
}
