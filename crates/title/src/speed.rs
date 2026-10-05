//! The speed-class ladder one title authors, and the union RACE REMIX offers.
//!
//! Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// The speed-class ladder one title authors, slowest first.
///
/// # Why this is an axis and not `SpeedClass::ALL`
///
/// [ADR-0022] licenses an axis once a *second* corpus disagrees, and this one
/// disagrees in **cardinality**: Wipeout Pure's per-team `handlingstats.xml`
/// authors **five** `<Class>` rungs and Wipeout Pulse's authors **four**, on
/// discs read separately. A shared constant would have to be either wrong for
/// Pure or silently reused as if measured - the exact trap
/// [`crate::MenuStrip`] was created to close for the layout numbers.
///
/// # Names, not `oag_physics::SpeedClass` values
///
/// Three reasons, in order of weight:
///
/// 1. **The authored datum is literally a name.** The disc writes
///    `<Class name="VENOM">` and `<GlobalClass name="VECTOR">`; a title
///    package that carried an enum would be recording a decoding of the file
///    rather than the file. [`crate::MenuStrip::selected_fill`] is the same
///    call made the same way - "a name, not a colour".
/// 2. **`oag-title` is deliberately thin**, and depends on `oag-disc` alone.
///    Carrying `SpeedClass` would drag `oag-physics` into a crate whose whole
///    job is to hold vocabulary rather than mechanism.
/// 3. **A name can outrun the engine, and here it does.** Pure authors a rung
///    this build cannot yet *select* - see [`Self::VECTOR`] - and a name
///    records that honestly where an enum could not represent it at all.
///
/// # What "measured" means for each entry
///
/// Every ladder below was read off that title's own per-team
/// `Data\Ships\<Team>\handlingstats.xml` (or its equivalent), which is the
/// file that decides what a class *does* rather than merely what a menu says.
/// The distinction matters because the two disagree on Pulse: Pulse's
/// **global** `Data\XML\HandlingStats.xml` authors five `<GlobalClass>` rungs
/// with `VECTOR` first, and its per-team files author four. The original's own
/// parser discards the global `VECTOR` block, so four is the real ladder. See
/// `docs/formats/handling-stats.md`.
///
/// [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeedClasses {
    /// The class names this title authors, slowest first, exactly as the disc
    /// spells them.
    ///
    /// Uppercase because that is what the XML writes. Matching is
    /// case-insensitive everywhere it happens, so nothing depends on the
    /// case; it is kept so a reviewer can compare a row against the file it
    /// came from without transforming it.
    pub names: &'static [&'static str],
}

impl SpeedClasses {
    /// The rung Wipeout Pure authors below Venom, and the one name here that
    /// only one measured title has.
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
    /// So every input a race needs is on the disc, and none of it is borrowed.
    ///
    /// # It is selectable, and what had to change first
    ///
    /// It used to be filtered out of every menu, because `oag_physics::SpeedClass`
    /// has four variants and nothing downstream could *name* a fifth rung. The
    /// fix was not to grow that enum: its discriminants index fixed four-wide
    /// tables, and a five-wide table would have had to invent a fifth entry for
    /// Pulse and HD, which author four. That is the stand-in this project
    /// forbids.
    ///
    /// Instead a race carries the rung's **name** and resolves it against the
    /// file that authored it -
    /// `oag_tables::handling::Stats::class_named`,
    /// `oag_tables::handling::Global::class_named` and
    /// `oag_weapons::pickup::table_for`. A title that authors four is asked
    /// for four and grows nothing; Pure is asked for five and answers with
    /// five. `oag_physics::SpeedClass` still has exactly four variants and its
    /// `ALL` still means "the rungs every measured title has", which is what
    /// keeps `has_pulse_class_ladder` meaningful.
    ///
    /// **Pulse authors a `<GlobalClass name="VECTOR">` too and still offers
    /// four**, because its per-team files author no such `<Class>` and its
    /// weapon table no such `<Pickupodds>`. The ladder is measured from the
    /// per-team files, which is the file that decides what a rung *does*. That
    /// is why "only if Pure's data is available" needs no flag: the union is
    /// built from the ladders of the titles actually mounted, and only Pure's
    /// names this rung.
    ///
    /// Whether *Pulse* has a hidden fifth class is a separate, still-open
    /// question - Pure authoring one says nothing about it.
    pub const VECTOR: &'static str = "VECTOR";

    /// The four rungs every title measured so far shares, slowest first.
    ///
    /// Not a default and not a fallback - each title names it explicitly, so a
    /// title whose ladder was never read cannot silently inherit one that was.
    pub const PULSE_LADDER: &'static [&'static str] = &["VENOM", "FLASH", "RAPIER", "PHANTOM"];

    /// Every rung any measured ladder names, slowest first.
    ///
    /// **A spell-checking vocabulary, not a table to index.** It exists for one
    /// caller: `--class` on the command line, which is parsed before a disc is
    /// opened and so cannot yet know which title's ladder applies. Catching a
    /// typo there is worth a list; racing on it is not, and nothing does - the
    /// rung is resolved against the file that authored it, and a name that is
    /// on this list but not in *that* file is still an honest load failure.
    ///
    /// It is the union of [`Self::PULSE_LADDER`] and Wipeout Pure's, which is
    /// every ladder read so far. Wipeout 2048's is unread, so a 2048 rung this
    /// does not name is a gap in the measurement rather than in the list.
    pub const MEASURED: &'static [&'static str] =
        &["VECTOR", "VENOM", "FLASH", "RAPIER", "PHANTOM"];

    /// Whether `name` is a rung some measured ladder carries.
    ///
    /// See [`Self::MEASURED`] for why this is not the check that decides a
    /// race.
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

    /// Whether this build can actually put a ship on the class called `name`.
    ///
    /// **Every name a measured ladder carries, since 2026-09-05.** It used to
    /// reject [`Self::VECTOR`], because nothing downstream could name a fifth
    /// rung; a race now carries the rung's name and resolves it against the
    /// file that authored it, so there is nothing left to filter. See
    /// [`Self::VECTOR`] for what that took.
    ///
    /// Kept as a function rather than deleted for two reasons. It is the one
    /// place every caller - RACE REMIX's union, the ordinary RACE page
    /// underneath [`Self::is_offered_outside_remix`], and any test - asks the
    /// question, so the next rung that turns out to be authored-but-unraceable
    /// has somewhere to go. And it states the invariant: **a class this
    /// rejects is drawn nowhere, on either page.** Not greyed, not shown as a
    /// locked row - an option that appears and then races on some other
    /// class's tuning is the invented stand-in this project forbids, and an
    /// honest absence is the alternative.
    #[must_use]
    pub fn is_selectable(_name: &str) -> bool {
        true
    }

    /// The subset of [`Self::names`] this build can actually put a ship on.
    ///
    /// **This is engine capability, not a menu-page rule** - it is what
    /// [`Self::is_selectable`] answers, and RACE REMIX's row is built from it
    /// directly (see `Session::remix_speed_classes`). The ordinary RACE page
    /// narrows this further, through [`Self::is_offered_outside_remix`],
    /// because it wants a stricter question: not "can this build race it" but
    /// "does this page offer it".
    pub fn selectable(&self) -> impl Iterator<Item = &'static str> + use<> {
        self.names
            .iter()
            .copied()
            .filter(|name| Self::is_selectable(name))
    }

    /// Whether `name` belongs on the ordinary per-title RACE page, as opposed
    /// to only RACE REMIX.
    ///
    /// **`VECTOR` is confined to RACE REMIX, by a 2026-09-05 maintainer
    /// decision.** Making the rung selectable at all made a Pure boot's
    /// ordinary RACE page offer five classes too, which is wider than what
    /// was asked for - see `docs/architecture/menus.md`'s "Wipeout Pure
    /// authors five, this build offers four outside remix" section for the
    /// reasoning and for why this is written down as a deliberate divergence
    /// rather than left to be found.
    ///
    /// This is a **menu-page** rule layered on top of [`Self::is_selectable`],
    /// not a narrower version of it. Everything below stays exactly as it
    /// was: `is_selectable` still answers "can this build put a ship on
    /// `name`" for `true` on every measured rung, RACE REMIX's union still
    /// unions and still offers `VECTOR` when Pure's data is available, and
    /// `oag-trace`'s `--class` still spell-checks and resolves it. Only the
    /// ordinary RACE page's own row stops listing it.
    #[must_use]
    pub fn is_offered_outside_remix(name: &str) -> bool {
        Self::is_selectable(name) && !name.eq_ignore_ascii_case(Self::VECTOR)
    }

    /// Every class any of `ladders` authors, as one ladder, slowest first.
    ///
    /// This is what RACE REMIX offers: a grid that mixes titles is not
    /// restricted to one title's rungs, so the row unions them. The ordinary
    /// per-title RACE page does **not** use this - it offers its own title's
    /// ladder and nothing else.
    ///
    /// # The merge preserves every ladder's own order
    ///
    /// Not a sort and not a concatenation. Each ladder states an *ordering* of
    /// the rungs it names - slowest first - and the union has to respect all
    /// of them at once, whatever order the titles themselves arrive in. So an
    /// unseen name is inserted directly after its own ladder's predecessor
    /// rather than appended, which is what puts Pure's `VECTOR` at the front
    /// even when Pulse's four-rung ladder was merged first.
    ///
    /// A name already present is never moved, so the result is deterministic
    /// in the order `ladders` yields - and that order is the caller's, not a
    /// `HashSet` iteration, which the determinism rules forbid feeding into
    /// anything the simulation reads.
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
