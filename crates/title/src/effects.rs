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
//! - **[`Effects`] is a struct of `Option`s, not a slice of `(Trigger,
//!   EffectSpec)`.** ADR item 4 wants a title to build its table from Pulse's
//!   with struct-update syntax, and a `const` slice cannot be updated, only
//!   replaced whole. [`Title::effect_on`](crate::Title::effect_on) is a `match`
//!   over the fields, so a new [`Trigger`] cannot be added without answering it
//!   here.
//! - **[`EffectSpec::effects`] is a list.** A wreck throws three effects, and
//!   one string would have named one of them.
//! - **[`Burst`] lives here** (it was `oag_raceplay::AbsorbBurst`): this crate is
//!   below `oag-raceplay`, and a title's table has to be able to hold it.
//!
//! # 2048 and Omega
//!
//! Both carry [`Effects::NONE`], checked against each other as CLAUDE.md asks.
//! Omega's executable carries HD's weapon-spark and absorb strings, so its
//! entries are "checked, applies, not wired": the wiring is unread, and a
//! string in a binary is not a measured trigger. 2048's are unread outright.
//! Neither draws anything, which is the same silence the loader had before this
//! table; no report line was written for it then and none is now.
//!
//! [ADR-0058]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0058-per-title-behaviour-is-title-data-with-provenance.md

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
/// Added to per trigger as each is read: one measured on a single title alone
/// would stay a title-local constant until a second corpus exists (ADR-0058).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// A weapon hit lands on a craft, thrown from its own hull's locators.
    HitSpark,
    /// A weapon's own hit spark on the craft it strikes, thrown from the
    /// weapon's side rather than the craft's.
    WeaponSpark,
    /// A craft absorbs a pickup.
    ShieldAbsorb,
    /// A craft goes out of the race.
    Wreck,
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
    /// The `Data\Psys\<name>.POB` effects it throws, the disc's own. Names only:
    /// the generic particle library resolves them, so no render-side crate
    /// reaches a title package.
    pub effects: &'static [&'static str],
    /// How the effects are staggered over the hull's locators, for a trigger
    /// that has one.
    pub burst: Option<Burst>,
    /// Where this entry came from.
    pub origin: Origin,
}

/// Every trigger's answer for one title. `None` means the title draws nothing
/// there: it either does not do it or it is unread, which is visible here and
/// not Pulse's value by the fall through of an `else`.
///
/// Built from Pulse's with struct-update syntax where a title inherits any of
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Effects {
    /// [`Trigger::HitSpark`].
    pub hit_spark: Option<EffectSpec>,
    /// [`Trigger::WeaponSpark`].
    pub weapon_spark: Option<EffectSpec>,
    /// [`Trigger::ShieldAbsorb`].
    pub shield_absorb: Option<EffectSpec>,
    /// [`Trigger::Wreck`].
    pub wreck: Option<EffectSpec>,
}

impl Effects {
    /// A title that throws none of them: every trigger unread.
    pub const NONE: Self = Self {
        hit_spark: None,
        weapon_spark: None,
        shield_absorb: None,
        wreck: None,
    };

    /// The entry for `trigger`.
    #[must_use]
    pub const fn on(&self, trigger: Trigger) -> Option<&EffectSpec> {
        match trigger {
            Trigger::HitSpark => self.hit_spark.as_ref(),
            Trigger::WeaponSpark => self.weapon_spark.as_ref(),
            Trigger::ShieldAbsorb => self.shield_absorb.as_ref(),
            Trigger::Wreck => self.wreck.as_ref(),
        }
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
        match self.on {
            Platforms::Never => false,
            Platforms::Any => true,
            Platforms::Only(list) => list.contains(&platform),
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
    /// bloom's glow mask, the quake, weather, track stats and intro camera.
    /// Keyed on the race's own archive.
    pub measured_draws: Rule,
    /// The PS2's bloom glow-mask stamp rule. Keyed on the race's own archive.
    pub ps2_glow_mask: Rule,
    /// The shield tint.
    pub shield_palette: ShieldPalettes,
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
        }
    }
}
