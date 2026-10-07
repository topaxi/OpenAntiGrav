//! What a title throws, and where it does so differently: per-title race
//! behaviour as data, so a generic crate asks the title instead of comparing its
//! name. [ADR-0058] is the decision; this is its first axis.
//!
//! # Differences from the ADR's sketch
//!
//! ADR-0058 section 5 is "a sketch to fix the vocabulary, not a signature". Where
//! the code forced a different shape:
//!
//! - **[`Origin`], not `Provenance`.** [`Provenance`](crate::Provenance) already
//!   names the boot chain's two-valued tag (`Measured` or `Declared`), is
//!   re-exported at this crate's root and matched in `oag-game`; a second enum
//!   under the same name would be a trap. `Origin` is the ADR's `Provenance`
//!   with its three variants.
//! - **[`Effects`] is an array indexed by [`Trigger`], built with
//!   [`Effects::with`], not a slice of `(Trigger, EffectSpec)`.** ADR item 4
//!   wants a title to build its table from another's, and a `const` slice
//!   cannot be updated, only replaced whole; `with` is a `const fn` over a
//!   fixed-size array, so a new [`Trigger`] cannot be added without growing
//!   [`Trigger::COUNT`] and answering it in [`Effects::engine`] or by a title.
//! - **[`EffectSpec::effect`] is one name.** A moment that throws several
//!   effects (a wreck throws three) is several triggers.
//! - **[`Burst`] lives here** (it was `oag_raceplay::AbsorbBurst`): this crate is
//!   below `oag-raceplay`, and a title's table has to be able to hold it.
//!
//! # 2048 and Omega
//!
//! Both carry [`Effects::engine`] only, taken from Pulse by inheritance, and
//! none of the four triggers a title answers for itself (the weapon-hit
//! sparks, the weapon spark, the absorb burst, the wreck), checked against
//! each other as CLAUDE.md asks. Omega's executable carries HD's weapon-spark
//! and absorb strings, so its entries are "checked, applies, not wired": the
//! wiring is unread, and a string in a binary is not a measured trigger.
//! 2048's are unread outright. Neither draws anything there, which is the same
//! silence the loader had before this table; no report line was written for it
//! then and none is now.
//!
//! [ADR-0058]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0058-per-title-behaviour-is-title-data-with-provenance.md

use crate::engine_effects;
use oag_disc::Platform;

/// Where one entry of a title's behaviour table came from.
///
/// The same idea [`Provenance`](crate::Provenance) gave the boot chain, extended
/// to a third answer, and required with no default for the same reason: it is
/// what stops a guess arriving as a measurement by omission. Anything that
/// shows a [`Origin::Chosen`] entry to a person says so.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Read off this title's own executable or data, or watched running. The
    /// comment beside the entry cites the page that measures it.
    Measured,
    /// The law of the named title, checked and found to apply (or taken to
    /// apply, per the project's rule that an unmeasured title runs Pulse's) and
    /// not re-measured on this one.
    InheritedFrom(&'static str),
    /// Not measured; picked by the project. Carries no confidence score.
    Chosen,
}

/// A moment in a race a title answers with an effect.
///
/// One effect per trigger. A moment that throws several (a wreck throws three)
/// is several triggers, so the effect a firing site wants is an index and a
/// misspelt one is a compile error. [`Self::ALL`] is in the order the loader
/// loads them, which is the order its report reads in.
///
/// Added to per trigger as each is read: one measured on a single title alone
/// would stay a title-local constant until a second corpus exists (ADR-0058).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// A craft's damaging wall contact.
    CollisionSpark,
    /// A weapon hit lands on a craft, thrown from its own hull's locators.
    HitSpark,
    /// A Rocket in flight.
    RocketFlare,
    /// A Missile in flight, at two anchors.
    MissileFlare,
    /// A Plasma bolt, charging and in flight.
    PlasmaFlare,
    /// A Plasma detonation's flash.
    PlasmaBlast,
    /// HD's Plasma detonation, at the blast.
    PlasmaLightningExpand,
    /// HD's Plasma detonation, 1.3 s later.
    PlasmaLightningCollapse,
    /// A Shuriken in flight.
    ShurikenFlare,
    /// A Shuriken's trail, riding beside its head.
    ShurikenTrail,
    /// A Shuriken glancing off a wall.
    ShurikenBounce,
    /// A Shuriken running out.
    ShurikenExpire,
    /// A Rocket detonating on the track.
    TrackBlast,
    /// A Rocket detonating on a craft.
    CraftBlast,
    /// A Missile detonating.
    MissileExplo,
    /// A Missile glancing off a wall.
    MissileBounce,
    /// A Mine detonating.
    MineExplo,
    /// A Bomb's smoke ring.
    BombSmokering,
    /// A Cannon bolt's impact.
    CannonSparks,
    /// A craft's engine flare.
    EngineFlare,
    /// A craft flying through an engine trail (HD).
    TrailHitship,
    /// The same, on a Fury-skinned trail.
    TrailHitshipRed,
    /// The Quake wave.
    Quake,
    /// A Repulser's blast.
    RepulserBlast,
    /// A Repulser's waves.
    Repulser,
    /// A LeachBeam's energy ball.
    LeachbeamEnergy,
    /// A LeachBeam charging.
    LeachbeamCharging,
    /// HD's LeachBeam drain-trip burst.
    LeachbeamAbsorb,
    /// A weapon's own hit spark on the craft it strikes, thrown from the
    /// weapon's side rather than the craft's.
    WeaponSpark,
    /// A craft absorbs a pickup.
    ShieldAbsorb,
    /// A landed LeachBeam drain, thrown from the struck hull's locators.
    LeachHitSpark,
    /// The blast at each wreck node as a craft goes out of the race.
    WreckNode,
    /// The sparks beside it.
    WreckSparks,
    /// The big blast after it.
    WreckExplosion,
    /// HD's damage smoke while a struck craft's shield is above its first
    /// threshold.
    DamageMild,
    /// The same between the two thresholds.
    DamageModerate,
    /// The same below the second one; played together with
    /// [`Self::DamageModerate`].
    DamageCritical,
    /// A craft over a magstrip (2048), outside a Zone.
    MagstripSparks,
    /// The same inside a Zone.
    MagstripZone,
}

impl Trigger {
    /// Every trigger, in load order.
    pub const ALL: [Self; 39] = [
        Self::CollisionSpark,
        Self::HitSpark,
        Self::RocketFlare,
        Self::MissileFlare,
        Self::PlasmaFlare,
        Self::PlasmaBlast,
        Self::PlasmaLightningExpand,
        Self::PlasmaLightningCollapse,
        Self::ShurikenFlare,
        Self::ShurikenTrail,
        Self::ShurikenBounce,
        Self::ShurikenExpire,
        Self::TrackBlast,
        Self::CraftBlast,
        Self::MissileExplo,
        Self::MissileBounce,
        Self::MineExplo,
        Self::BombSmokering,
        Self::CannonSparks,
        Self::EngineFlare,
        Self::TrailHitship,
        Self::TrailHitshipRed,
        Self::Quake,
        Self::RepulserBlast,
        Self::Repulser,
        Self::LeachbeamEnergy,
        Self::LeachbeamCharging,
        Self::LeachbeamAbsorb,
        Self::WeaponSpark,
        Self::ShieldAbsorb,
        Self::LeachHitSpark,
        Self::WreckNode,
        Self::WreckSparks,
        Self::WreckExplosion,
        Self::DamageMild,
        Self::DamageModerate,
        Self::DamageCritical,
        Self::MagstripSparks,
        Self::MagstripZone,
    ];

    /// How many there are: the length of a table indexed by trigger.
    pub const COUNT: usize = Self::ALL.len();

    /// This trigger's position in [`Self::ALL`].
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// How a title staggers a burst over a hull's locators.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Burst {
    /// Node `i` of at most `cap`, `i * stagger` seconds late.
    Sequential {
        /// The loop bound.
        cap: usize,
        /// Seconds between one node and the next.
        stagger: f32,
    },
    /// Six nodes in three mirrored pairs, `stagger` seconds apart - slots 0 and
    /// 5 first, then 1 and 4, then 2 and 3.
    MirroredPairs {
        /// Seconds between one pair and the next.
        stagger: f32,
    },
}

impl Burst {
    /// `(locator index, delay in seconds)` for a hull that carries `nodes`
    /// locators of this title's class, in the order the original spawns them.
    ///
    /// Empty when the hull carries none - and, for the mirrored shape, when it
    /// carries fewer than six, since the original tests the outer pair's two
    /// slots before it spawns anything. It never falls back to the craft's
    /// centre.
    #[must_use]
    pub fn schedule(self, nodes: usize) -> Vec<(usize, f32)> {
        match self {
            Self::Sequential { cap, stagger } => (0..nodes.min(cap))
                .map(|i| (i, i as f32 * stagger))
                .collect(),
            Self::MirroredPairs { stagger } if nodes >= 6 => {
                let late = stagger + stagger;
                vec![
                    (0, 0.0),
                    (5, 0.0),
                    (1, stagger),
                    (4, stagger),
                    (2, late),
                    (3, late),
                ]
            }
            Self::MirroredPairs { .. } => Vec::new(),
        }
    }
}

/// What a title does on one [`Trigger`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectSpec {
    /// The `Data\Psys\<name>.POB` effect it throws, the disc's own. A name
    /// only: the generic particle library resolves it, so no render-side crate
    /// reaches a title package.
    pub effect: &'static str,
    /// How the effect is staggered over the hull's locators, for a trigger
    /// that has one.
    pub burst: Option<Burst>,
    /// Where this entry came from.
    pub origin: Origin,
    /// The platforms whose archives author this effect. [`Platforms::Any`]
    /// unless an entry says otherwise; the loader asks only for what the
    /// platform it is reading can hold, so an absence known to be by design
    /// is not reported as a missing file.
    pub on: Platforms,
}

impl EffectSpec {
    /// An entry with no burst.
    #[must_use]
    pub const fn new(effect: &'static str, origin: Origin) -> Self {
        Self {
            effect,
            burst: None,
            origin,
            on: Platforms::Any,
        }
    }

    /// This entry limited to the platforms that author the effect.
    #[must_use]
    pub const fn with_platforms(self, on: Platforms) -> Self {
        Self { on, ..self }
    }

    /// This entry staggered over the hull's locators by `burst`.
    #[must_use]
    pub const fn with_burst(self, burst: Burst) -> Self {
        Self {
            burst: Some(burst),
            ..self
        }
    }
}

/// Every trigger's answer for one title. `None` means the title draws nothing
/// there: it either does not do it or it is unread, which is visible here and
/// not Pulse's value by the fall through of an `else`.
///
/// A title builds its own from [`Self::engine`] or [`Self::NONE`] with
/// [`Self::with`], which stands in for the struct-update syntax ADR-0058 item 4
/// names: an array cannot be updated field by field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effects {
    table: [Option<EffectSpec>; Trigger::COUNT],
    /// Effects a circuit's own data may name - placed scenery and weather -
    /// loaded for every race so the track's name can find them. A name the
    /// track data supplies cannot be a [`Trigger`], so these stay looked up by
    /// name.
    pub scenery: &'static [&'static str],
}

impl Effects {
    /// A title that throws none of them: every trigger unread.
    pub const NONE: Self = Self {
        table: [None; Trigger::COUNT],
        scenery: &[],
    };

    /// The entry for `trigger`.
    #[must_use]
    pub const fn on(&self, trigger: Trigger) -> Option<&EffectSpec> {
        self.table[trigger.index()].as_ref()
    }

    /// This table with `trigger` answered by `spec`.
    #[must_use]
    pub const fn with(mut self, trigger: Trigger, spec: EffectSpec) -> Self {
        self.table[trigger.index()] = Some(spec);
        self
    }

    /// This table with `trigger` answering nothing.
    #[must_use]
    pub const fn without(mut self, trigger: Trigger) -> Self {
        self.table[trigger.index()] = None;
        self
    }

    /// Every effect name this table loads, each once, in [`Trigger::ALL`]
    /// order and then the [`Self::scenery`] names. Order is the loader
    /// report's, so it reads the same way twice.
    #[must_use]
    pub fn names(&self) -> Vec<&'static str> {
        self.names_where(|_| true)
    }

    /// [`Self::names`] without the entries `platform`'s archives do not
    /// author. [`Self::scenery`] names are kept: a circuit's own data decides
    /// those.
    #[must_use]
    pub fn names_on(&self, platform: Platform) -> Vec<&'static str> {
        self.names_where(|spec| spec.on.applies(platform))
    }

    fn names_where(&self, keep: impl Fn(&EffectSpec) -> bool) -> Vec<&'static str> {
        let mut names: Vec<&'static str> = Vec::new();
        let triggered = Trigger::ALL.iter().filter_map(|&t| self.on(t));
        for name in triggered
            .filter(|spec| keep(spec))
            .map(|spec| spec.effect)
            .chain(self.scenery.iter().copied())
        {
            if !names.contains(&name) {
                names.push(name);
            }
        }
        names
    }

    /// The effects every weapon, craft and circuit of the engine plays, as
    /// Pulse's executable names them, each tagged `origin`.
    ///
    /// Pulse passes [`Origin::Measured`]; another title takes the same names
    /// by inheritance (`Origin::InheritedFrom("Wipeout Pulse")`) because a
    /// name that is on its disc is what its race has always played. An
    /// effect absent from a disc is reported and skipped by the loader, which
    /// is how a name this title does not author costs one report line and
    /// nothing else. The four triggers that vary by title - the weapon-hit
    /// sparks, the weapon spark, the absorb burst and the wreck - are *not*
    /// here: each title answers them itself.
    #[must_use]
    pub const fn engine(origin: Origin) -> Self {
        use Trigger as T;
        use engine_effects as n;
        let mut e = Self::NONE;
        e.scenery = &[
            n::BLUE_WELDER_EFFECT,
            n::MODESTO_STEAM_EFFECT,
            n::RAIN_EFFECT,
            n::RAIN_LENS_EFFECT,
            n::SNOW_EFFECT,
        ];
        e.with(
            T::CollisionSpark,
            EffectSpec::new(n::COLLISION_SPARK_EFFECT, origin),
        )
        .with(
            T::RocketFlare,
            EffectSpec::new(n::ROCKET_FLARE_EFFECT, origin),
        )
        .with(
            T::MissileFlare,
            EffectSpec::new(n::MISSILE_FLARE_EFFECT, origin),
        )
        .with(
            T::PlasmaFlare,
            EffectSpec::new(n::PLASMA_FLARE_EFFECT, origin),
        )
        .with(
            T::PlasmaBlast,
            EffectSpec::new(n::PLASMA_BLAST_EFFECT, origin),
        )
        .with(
            T::PlasmaLightningExpand,
            EffectSpec::new(n::PLASMA_LIGHTNING_EXPAND_EFFECT, origin),
        )
        .with(
            T::PlasmaLightningCollapse,
            EffectSpec::new(n::PLASMA_LIGHTNING_COLLAPSE_EFFECT, origin),
        )
        .with(
            T::ShurikenFlare,
            EffectSpec::new(n::SHURIKEN_FLARE_EFFECT, origin),
        )
        .with(
            T::ShurikenTrail,
            EffectSpec::new(n::SHURIKEN_TRAIL_EFFECT, origin),
        )
        .with(
            T::ShurikenBounce,
            EffectSpec::new(n::SHURIKEN_BOUNCE_EFFECT, origin),
        )
        .with(
            T::ShurikenExpire,
            EffectSpec::new(n::SHURIKEN_EXPIRE_EFFECT, origin),
        )
        .with(
            T::TrackBlast,
            EffectSpec::new(n::TRACK_BLAST_EFFECT, origin),
        )
        .with(
            T::CraftBlast,
            EffectSpec::new(n::CRAFT_BLAST_EFFECT, origin),
        )
        .with(
            T::MissileExplo,
            EffectSpec::new(n::MISSILE_EXPLO_EFFECT, origin),
        )
        .with(
            T::MissileBounce,
            EffectSpec::new(n::MISSILE_BOUNCE_EFFECT, origin),
        )
        .with(T::MineExplo, EffectSpec::new(n::MINE_EXPLO_EFFECT, origin))
        .with(
            T::BombSmokering,
            EffectSpec::new(n::BOMB_SMOKERING_EFFECT, origin),
        )
        .with(
            T::CannonSparks,
            EffectSpec::new(n::CANNON_SPARKS_EFFECT, origin),
        )
        .with(
            T::EngineFlare,
            EffectSpec::new(n::ENGINE_FLARE_EFFECT, origin),
        )
        .with(
            T::TrailHitship,
            EffectSpec::new(n::TRAIL_HITSHIP_EFFECT, origin),
        )
        .with(
            T::TrailHitshipRed,
            EffectSpec::new(n::TRAIL_HITSHIP_RED_EFFECT, origin),
        )
        .with(T::Quake, EffectSpec::new(n::QUAKE_EFFECT, origin))
        .with(
            T::RepulserBlast,
            EffectSpec::new(n::REPULSER_BLAST_EFFECT, origin),
        )
        .with(T::Repulser, EffectSpec::new(n::REPULSER_EFFECT, origin))
        .with(
            T::LeachbeamEnergy,
            EffectSpec::new(n::LEACHBEAM_ENERGY_EFFECT, origin),
        )
        .with(
            T::LeachbeamCharging,
            EffectSpec::new(n::LEACHBEAM_CHARGING_EFFECT, origin),
        )
        .with(
            T::LeachbeamAbsorb,
            EffectSpec::new(n::LEACHBEAM_ABSORB_EFFECT, origin),
        )
        .with(
            T::MagstripSparks,
            EffectSpec::new(n::MAGSTRIP_SPARKS_EFFECT, origin),
        )
        .with(
            T::MagstripZone,
            EffectSpec::new(n::MAGSTRIP_ZONE_EFFECT, origin),
        )
    }
}

/// On which platforms a title does something, as its own archive's platform
/// reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platforms {
    /// Never: unread or not done.
    Never,
    /// Whichever platform the disc is.
    Any,
    /// Only these.
    Only(&'static [Platform]),
}

/// One yes-or-no behaviour of a title, where it applies, and where that came
/// from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    /// The platforms it applies on.
    pub on: Platforms,
    /// Where this entry came from.
    pub origin: Origin,
}

impl Rule {
    /// Not done by this title, because nothing of it has been read there. The
    /// absence is chosen, and says so.
    pub const UNREAD: Self = Self {
        on: Platforms::Never,
        origin: Origin::Chosen,
    };

    /// Whether the rule applies whichever platform the disc is - for a behaviour
    /// no measured title restricts to one.
    #[must_use]
    pub fn applies_everywhere(&self) -> bool {
        self.on == Platforms::Any
    }

    /// Whether the rule applies to a disc of `platform`.
    #[must_use]
    pub fn applies(&self, platform: Platform) -> bool {
        self.on.applies(platform)
    }
}

impl Platforms {
    /// Whether a disc of `platform` is one of them.
    #[must_use]
    pub fn applies(self, platform: Platform) -> bool {
        match self {
            Self::Never => false,
            Self::Any => true,
            Self::Only(list) => list.contains(&platform),
        }
    }
}

/// Which shield tint a craft's shell takes. The palettes themselves are the
/// renderer's; a title only names one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShieldPalette {
    /// Pulse's PSP shell tint.
    Pulse,
    /// Pulse's PS2 shell tint.
    Ps2Pulse,
    /// Wipeout HD's.
    Hd,
}

/// Which [`ShieldPalette`] a title's craft takes, by the platform of the
/// archive the craft was loaded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShieldPalettes {
    /// On a PS2 disc.
    pub ps2: ShieldPalette,
    /// On any other.
    pub elsewhere: ShieldPalette,
    /// Where this entry came from.
    pub origin: Origin,
}

impl ShieldPalettes {
    /// The palette for a craft loaded from a disc of `platform`.
    #[must_use]
    pub fn on(&self, platform: Platform) -> ShieldPalette {
        if platform == Platform::Ps2 {
            self.ps2
        } else {
            self.elsewhere
        }
    }
}

/// What a title draws or does in a race beyond its effect triggers: the flags
/// `oag-raceplay`'s loader used to derive from the title's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Looks {
    /// The weapon-absorb hull overlay.
    pub hull_overlay: Rule,
    /// The hull's specular shine.
    pub hull_shine: Rule,
    /// The hull swapping to its `shipwreck.vex` on destruction. Keyed on the
    /// craft's archive.
    pub hull_wreck: Rule,
    /// The absorb shell around the craft (HD's).
    pub absorb_shell: Rule,
    /// A launched weapon's laid-down pose, Pulse's own.
    pub laid_pose: Rule,
    /// The draws measured live on the PSP only: the hull's GE lights, the
    /// bloom's glow mask, the quake, weather and track stats.
    /// Keyed on the race's own archive.
    pub measured_draws: Rule,
    /// The PS2's bloom glow-mask stamp rule. Keyed on the race's own archive.
    pub ps2_glow_mask: Rule,
    /// The shield tint.
    pub shield_palette: ShieldPalettes,
    /// The front end never waits at its language picker on a fresh run: the
    /// original leaves `Language Selection` within milliseconds, so this build
    /// defaults to English instead of stopping there. Not a race look, but the
    /// same yes-or-no-with-origin shape; kept here so a title carries one
    /// table of such rules.
    pub skips_language_picker: Rule,
}

impl Looks {
    /// A title none of whose race looks are read, but which still needs a shield
    /// tint: every rule [`Rule::UNREAD`], so it draws none of them, with
    /// `shield_palette` as given. Build a title's own table from this with
    /// struct-update syntax and override only what that title does.
    #[must_use]
    pub const fn unread(shield_palette: ShieldPalettes) -> Self {
        Self {
            hull_overlay: Rule::UNREAD,
            hull_shine: Rule::UNREAD,
            hull_wreck: Rule::UNREAD,
            absorb_shell: Rule::UNREAD,
            laid_pose: Rule::UNREAD,
            measured_draws: Rule::UNREAD,
            ps2_glow_mask: Rule::UNREAD,
            shield_palette,
            skips_language_picker: Rule::UNREAD,
        }
    }
}
