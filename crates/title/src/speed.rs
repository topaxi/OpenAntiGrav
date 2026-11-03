//! The speed-class ladder one title authors, and the union RACE REMIX offers.

/// The speed-class ladder one title authors, slowest first.
///
/// # An axis, not `SpeedClass::ALL`
///
/// [ADR-0022] licenses an axis once a *second* corpus disagrees, and this one
/// disagrees in **cardinality**: Pure's per-team `handlingstats.xml` authors
/// **five** `<Class>` rungs and Pulse's **four**, on discs read separately. A
/// shared constant would be wrong for Pure or reused as if measured (the trap
/// [`crate::MenuStrip`] closes for layout numbers).
///
/// # Names, not `SpeedClass` values
///
/// 1. **The authored datum is a name**: the disc writes `<Class name="VENOM">` and
///    `<GlobalClass name="VECTOR">`; an enum would record a decoding of the file
///    ([`crate::MenuStrip::selected_fill`] makes the same call).
/// 2. **`oag-title` is thin** and depends on `oag-disc` alone; carrying
///    `oag_tables::handling::SpeedClass` would pull `oag-tables` into a vocabulary
///    crate.
/// 3. **A name can outrun the engine.** Pure authors a rung this build once could
///    not *select* ([`Self::VECTOR`]); a name records that where an enum could not.
///
/// # What "measured" means
///
/// Every ladder was read off that title's per-team
/// `Data\Ships\<Team>\handlingstats.xml` (or equivalent), the file that decides what
/// a class *does*. On Pulse it disagrees with the global file: Pulse's
/// `Data\XML\HandlingStats.xml` authors five `<GlobalClass>` rungs with `VECTOR`
/// first and its per-team files four; the original's parser discards the global
/// `VECTOR` block, so four is the real ladder (`docs/formats/handling-stats.md`).
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeedClasses {
    /// The class names this title authors, slowest first, as the disc spells them.
    /// Uppercase as the XML writes; matching is case-insensitive everywhere, the
    /// case is kept so a row compares against its file without transformation.
    pub names: &'static [&'static str],
    /// The names among [`Self::names`] only RACE REMIX offers; the ordinary
    /// per-title RACE page draws the rest.
    ///
    /// **Chosen, not measured**, per rung: `VECTOR` is confined to RACE REMIX and
    /// 2048's `SUPERPHANTOM` follows (no ordinary page of that title offers it, it
    /// has no race box); `docs/architecture/menus.md`, "SPEED CLASS: `VECTOR` is
    /// confined to RACE REMIX". Data on the ladder so a rung is added or confined
    /// without a name check in code.
    pub remix_only: &'static [&'static str],
}

impl SpeedClasses {
    /// The rung Pure authors below Venom, the one name here only one measured
    /// title has.
    ///
    /// Pure authors it fully, on both pressings:
    ///
    /// | Where | What |
    /// | --- | --- |
    /// | `Data\Ships\<Team>\handlingstats.xml` | `<Class name="VECTOR">`, in every race team |
    /// | `Data\XML\HandlingStats.xml` | `<GlobalClass name="VECTOR">`, first of five |
    /// | `Data\XML\weaponstats.xml` | `<Pickupodds class="Vector">`, beside the four |
    /// | the front end | five `<Menu name="Class">` entries |
    ///
    /// Every input a race needs is on the disc, none borrowed.
    ///
    /// # Selectable
    ///
    /// `oag_tables::handling::SpeedClass` is a name on one ladder, and the four-wide
    /// census tables stay four wide (`SpeedClass::slot`) so Pulse and HD are given no
    /// invented fifth entry. A race carries the rung's **name** and resolves it
    /// against the file that authored it (`oag_tables::handling::Stats::class_named`,
    /// `Global::class_named`, `WeaponStats::pickups_for`): a title authoring
    /// four is asked for four, Pure for five. `SpeedClass::ALL` means "the rungs
    /// every measured title has", which keeps `has_pulse_class_ladder` meaningful.
    ///
    /// **Pulse authors a `<GlobalClass name="VECTOR">` too and offers four**: its
    /// per-team files author no such `<Class>` and its weapon table no such
    /// `<Pickupodds>`. So "only if Pure's data is available" needs no flag: the
    /// union is built from the ladders of the titles mounted, and only Pure's names
    /// this rung. Whether *Pulse* has a hidden fifth class is open.
    pub const VECTOR: &'static str = "VECTOR";

    /// The four rungs every title measured so far shares, slowest first. Not a
    /// default: each title names it explicitly, so a title whose ladder was never
    /// read cannot inherit one that was.
    pub const PULSE_LADDER: &'static [&'static str] = &["VENOM", "FLASH", "RAPIER", "PHANTOM"];

    /// Wipeout Pure's five rungs, `VECTOR` under Pulse's four.
    pub const PURE_LADDER: &'static [&'static str] =
        &["VECTOR", "VENOM", "FLASH", "RAPIER", "PHANTOM"];

    /// The rung 2048 authors above Phantom, spelled as its files spell it.
    pub const SUPER_PHANTOM: &'static str = "SUPERPHANTOM";

    /// Wipeout 2048's native roster: Pulse's four with `SUPERPHANTOM` above.
    ///
    /// Read off the five native teams' `handlingstats.xml` (20 files per region,
    /// both regions, `VENOM, FLASH, RAPIER, PHANTOM, SUPERPHANTOM` in that order;
    /// `oag-tables`' `speed_ladder_ground_truth`). The twelve HD teams 2048 reships
    /// author the four only ([`crate::GuestRoster`]).
    pub const WIPEOUT_2048_LADDER: &'static [&'static str] =
        &["VENOM", "FLASH", "RAPIER", "PHANTOM", "SUPERPHANTOM"];

    /// Every rung any measured ladder names, slowest first.
    ///
    /// **A spell-checking vocabulary, not a table to index**, for one caller:
    /// `--class`, parsed before a disc is opened, so it cannot know which ladder
    /// applies. Racing resolves the rung against the file that authored it, and a
    /// name on this list but not in *that* file is a load failure.
    ///
    /// The union of [`Self::PULSE_LADDER`], [`Self::PURE_LADDER`] and
    /// [`Self::WIPEOUT_2048_LADDER`]; HD's is Pulse's four, Omega's is unmeasured.
    pub const MEASURED: &'static [&'static str] = &[
        "VECTOR",
        "VENOM",
        "FLASH",
        "RAPIER",
        "PHANTOM",
        "SUPERPHANTOM",
    ];

    /// Whether `name` is a rung some measured ladder carries (not the check that
    /// decides a race, see [`Self::MEASURED`]).
    #[must_use]
    pub fn is_measured_name(name: &str) -> bool {
        Self::MEASURED
            .iter()
            .any(|known| known.eq_ignore_ascii_case(name))
    }

    /// Every name this title authors, slowest first.
    #[must_use]
    pub const fn names(&self) -> &'static [&'static str] {
        self.names
    }

    /// Whether this build can put a ship on the class called `name`.
    ///
    /// Every name a measured ladder carries: a race carries the rung's name and
    /// resolves it against the authoring file, so nothing is filtered (see
    /// [`Self::VECTOR`]).
    ///
    /// Kept as a function because every caller (RACE REMIX's union, the ordinary
    /// RACE page via [`Self::is_offered_outside_remix`], tests) asks the question
    /// here, so the next authored-but-unraceable rung has somewhere to go. The
    /// invariant: **a class this rejects is drawn nowhere, on either page**, not
    /// greyed or locked. An option that races on another class's tuning is the
    /// invented stand-in this project forbids.
    #[must_use]
    pub fn is_selectable(_name: &str) -> bool {
        true
    }

    /// The subset of [`Self::names`] this build can put a ship on: engine
    /// capability ([`Self::is_selectable`]), from which a `speed_classes_union` row is
    /// built (`Session::speed_classes_union`). The ordinary RACE page narrows it through
    /// [`Self::is_offered_outside_remix`].
    pub fn selectable(&self) -> impl Iterator<Item = &'static str> + use<> {
        self.names
            .iter()
            .copied()
            .filter(|name| Self::is_selectable(name))
    }

    /// Whether `name` belongs on the ordinary per-title RACE page, as opposed to
    /// only RACE REMIX: it is on this ladder and not in [`Self::remix_only`].
    ///
    /// **`VECTOR` is confined to RACE REMIX** by decision, and 2048's
    /// `SUPERPHANTOM` follows (chosen, not measured). Making a rung selectable at
    /// all made a Pure boot's ordinary RACE page offer five classes, wider than
    /// asked; `docs/architecture/menus.md`, "SPEED CLASS: `VECTOR` is confined to
    /// RACE REMIX", has the reasoning and the deliberate divergence.
    ///
    /// A **menu-page** rule on top of [`Self::is_selectable`]: RACE REMIX's union
    /// still offers the rung, only the ordinary page's row stops listing it.
    #[must_use]
    pub fn is_offered_outside_remix(&self, name: &str) -> bool {
        Self::is_selectable(name)
            && self.names.iter().any(|n| n.eq_ignore_ascii_case(name))
            && !self.remix_only.iter().any(|n| n.eq_ignore_ascii_case(name))
    }

    /// The rungs of this ladder the ordinary RACE page draws, slowest first.
    pub fn race_page(&self) -> impl Iterator<Item = &'static str> + use<'_> {
        self.names
            .iter()
            .copied()
            .filter(|name| self.is_offered_outside_remix(name))
    }

    /// Every class any of `ladders` authors, as one ladder, slowest first.
    ///
    /// What RACE REMIX offers: a grid that mixes titles is not restricted to one
    /// title's rungs. The ordinary per-title RACE page offers its own ladder only.
    ///
    /// # The merge preserves every ladder's own order
    ///
    /// Not a sort or a concatenation: each ladder states an *ordering* the union
    /// must respect at once, whatever order the titles arrive in. An unseen name is
    /// inserted directly after its own ladder's predecessor, which puts Pure's
    /// `VECTOR` at the front even when Pulse's four-rung ladder merged first. A name
    /// already present is never moved, so the result is deterministic in the order
    /// `ladders` yields (the caller's, not a `HashSet` iteration, which the
    /// determinism rules forbid feeding anything the simulation reads).
    #[must_use]
    pub fn union(ladders: impl IntoIterator<Item = Self>) -> Vec<&'static str> {
        let mut merged: Vec<&'static str> = Vec::new();
        for ladder in ladders {
            let mut after: Option<usize> = None;
            for name in ladder.names {
                match merged
                    .iter()
                    .position(|seen| seen.eq_ignore_ascii_case(name))
                {
                    Some(at) => after = Some(at),
                    None => {
                        let at = after.map_or(0, |prev| prev + 1);
                        merged.insert(at, name);
                        after = Some(at);
                    }
                }
            }
        }
        merged
    }
}

#[cfg(test)]
mod tests;
