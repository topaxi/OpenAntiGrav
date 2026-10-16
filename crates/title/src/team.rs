//! Which team a title's team id is, across titles.
//!
//! Loyalty is kept **per team and shared by every title that has that team**
//! (maintainer, 2026-10-16): Feisar's total counts in Pulse, Pure, HD, 2048 and
//! Omega alike. Each title spells its team ids its own way (`AG_Systems`,
//! `ag_systems`, `Feisar2048`), so a title lists the ids its roster can name and
//! the shared identity each stands for. The identity is a lower-case token; an id
//! a title does not list is its own case-folded spelling, which is what makes two
//! titles that spell a team the same way share it with no table at all.
//!
//! The roster does a second job: an `<Unlock Team="any" loyalty="N">` row is met
//! by the best single team **of that title**, so the set of identities a title
//! ranges over has to be its own - Omega's `Tigron` must not open Pulse's `any`
//! rows.

/// One team id as a title spells it and the identity it is kept under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamIdentity {
    /// The id the title's own archive and race code uses.
    pub id: &'static str,
    /// The shared, lower-case identity. Equal to the case-folded `id` unless the
    /// title spells the team differently from the others.
    pub identity: &'static str,
}

/// A title's team ids.
pub type Roster = &'static [TeamIdentity];

/// Shorthand for a roster entry whose identity is its own lower-case spelling.
#[must_use]
pub const fn team(id: &'static str, identity: &'static str) -> TeamIdentity {
    TeamIdentity { id, identity }
}

/// The identity `id` is kept under for a title with `roster`: the listed one, else
/// `id` case-folded and trimmed.
#[must_use]
pub fn identity_of(roster: Roster, id: &str) -> String {
    let id = id.trim();
    roster
        .iter()
        .find(|entry| entry.id.eq_ignore_ascii_case(id))
        .map_or_else(
            || id.to_ascii_lowercase(),
            |entry| entry.identity.to_owned(),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROSTER: Roster = &[team("AG_Systems", "ag_systems"), team("Mantis", "mirage")];

    #[test]
    fn a_listed_id_takes_its_identity_and_an_unlisted_one_its_own_spelling() {
        assert_eq!(identity_of(ROSTER, "mantis"), "mirage");
        assert_eq!(identity_of(ROSTER, " AG_SYSTEMS "), "ag_systems");
        assert_eq!(identity_of(ROSTER, "Feisar"), "feisar");
    }
}
