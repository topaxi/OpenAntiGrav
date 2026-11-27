//! [`ActiveMod`]: the mods that were on when a run was recorded, and what a machine with other
//! mods on says about it (`docs/formats/replay-file.md`, "Active mods").
//!
//! Every mod that is on for the race is listed, data-only ones included, by the id and release
//! its manifest states. [`Header::mods`](crate::Header::mods) is another list: the simulation
//! *modules* a race ran, by their bytes, which a rebuild must match exactly. This one decides
//! nothing about validity; the stored state hashes do. It names what differs.

use serde::{Deserialize, Serialize};

/// One mod that was on for a race.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ActiveMod {
    /// The mod's `[mod] id`.
    pub id: String,
    /// The mod's `[mod] version`.
    pub version: String,
}

impl ActiveMod {
    /// A mod `id` at `version`.
    #[must_use]
    pub fn new(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            version: version.into(),
        }
    }

    /// How the mods `here` differ from the `recorded` ones, each difference naming its mod, or
    /// `None` when they are the same mods at the same releases in the same load order.
    #[must_use]
    pub fn differences(recorded: &[Self], here: &[Self]) -> Option<String> {
        let mut why = Vec::new();
        for want in recorded {
            match here.iter().find(|m| m.id == want.id) {
                None => why.push(format!("missing {} {}", want.id, want.version)),
                Some(have) if have.version != want.version => why.push(format!(
                    "{} is {} here and {} in the recording",
                    want.id, have.version, want.version
                )),
                Some(_) => {}
            }
        }
        for have in here {
            if !recorded.iter().any(|m| m.id == have.id) {
                why.push(format!("extra {} {}", have.id, have.version));
            }
        }
        if why.is_empty() && recorded != here {
            let ids = |mods: &[Self]| {
                mods.iter()
                    .map(|m| m.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            why.push(format!(
                "load order is {} here and {} in the recording",
                ids(here),
                ids(recorded)
            ));
        }
        (!why.is_empty()).then(|| format!("mods differ from the recording: {}", why.join("; ")))
    }
}

#[cfg(test)]
mod tests {
    use super::ActiveMod;

    fn mods(list: &[(&str, &str)]) -> Vec<ActiveMod> {
        list.iter()
            .map(|(id, version)| ActiveMod::new(*id, *version))
            .collect()
    }

    #[test]
    fn the_same_mods_in_the_same_order_differ_in_nothing() {
        assert_eq!(ActiveMod::differences(&[], &[]), None);
        let both = mods(&[("a.one", "1.0.0"), ("b.two", "2.0.0")]);
        assert_eq!(ActiveMod::differences(&both, &both), None);
    }

    #[test]
    fn each_kind_of_difference_names_its_mod() {
        let recorded = mods(&[("a.gone", "1.0.0"), ("b.moved", "1.0.0")]);
        let here = mods(&[("b.moved", "1.1.0"), ("c.new", "3.0.0")]);
        assert_eq!(
            ActiveMod::differences(&recorded, &here).as_deref(),
            Some(
                "mods differ from the recording: missing a.gone 1.0.0; \
                 b.moved is 1.1.0 here and 1.0.0 in the recording; extra c.new 3.0.0"
            )
        );
    }

    #[test]
    fn the_same_mods_in_another_order_are_a_difference() {
        let recorded = mods(&[("a.one", "1.0.0"), ("b.two", "1.0.0")]);
        let here = mods(&[("b.two", "1.0.0"), ("a.one", "1.0.0")]);
        assert_eq!(
            ActiveMod::differences(&recorded, &here).as_deref(),
            Some(
                "mods differ from the recording: load order is b.two, a.one here and \
                 a.one, b.two in the recording"
            )
        );
    }
}
