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
    /// The rung Wipeout Pure authors below Venom, and the one name in this
    /// module that **no title's engine side can select yet**.
    ///
    /// Pure authors it fully: a `<Class name="VECTOR">` block in every one of
    /// its race teams' `handlingstats.xml` files, plus a `<GlobalClass>` of
    /// its own, plus five `<Menu name="Class">` entries in its front end. So
    /// the tuning is real, measured and readable - this is **not** a name with
    /// nothing behind it.
    ///
    /// What is missing is a way to *name* it downstream.
    /// `oag_physics::SpeedClass` has four variants and its `ALL` means "the
    /// rungs every title has": it backs `from_name`, it backs
    /// `oag_formats::handling::Stats::has_pulse_class_ladder`'s
    /// `classes.len() == ALL.len()`, and `class as usize` indexes fixed
    /// four-wide arrays for per-class weapon speeds and the global
    /// speed-pad/gravity/weapon-pad tables. Growing it to five is a real
    /// change to two determinism-bound crates and is its own piece of work,
    /// not a side effect of listing a name here.
    ///
    /// Until that lands, [`Self::selectable`] filters this out, so the menu
    /// offers nothing it cannot race. Recorded here rather than dropped
    /// because dropping it would lose the measurement.
    pub const VECTOR: &'static str = "VECTOR";

    /// The four rungs every title measured so far shares, slowest first.
    ///
    /// Not a default and not a fallback - each title names it explicitly, so a
    /// title whose ladder was never read cannot silently inherit one that was.
    pub const PULSE_LADDER: &'static [&'static str] = &["VENOM", "FLASH", "RAPIER", "PHANTOM"];

    /// Every name this title authors, slowest first.
    #[must_use]
    pub const fn names(&self) -> &'static [&'static str] {
        self.names
    }

    /// Whether this build can actually put a ship on the class called `name`.
    ///
    /// Today the answer is "everything except [`Self::VECTOR`]", for the
    /// reason that constant gives. It is a function rather than a list so that
    /// every caller - the ordinary RACE page, RACE REMIX's union, and any test
    /// - answers the question the same way and in one place.
    ///
    /// **A class this rejects is drawn nowhere.** It is not greyed and it is
    /// not shown as a locked row: an option that appears and then races on
    /// some other class's tuning is the invented stand-in this project
    /// forbids, and an honest absence is the alternative.
    #[must_use]
    pub fn is_selectable(name: &str) -> bool {
        !name.eq_ignore_ascii_case(Self::VECTOR)
    }

    /// The subset of [`Self::names`] this build can actually put a ship on.
    pub fn selectable(&self) -> impl Iterator<Item = &'static str> + use<> {
        self.names
            .iter()
            .copied()
            .filter(|name| Self::is_selectable(name))
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
