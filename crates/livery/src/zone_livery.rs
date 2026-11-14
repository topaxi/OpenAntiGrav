//! The Zone hull's team texture: the one thing a craft's pick changes on it. The path
//! substitution it makes ([`Swap`], [`Reads`]) also repaints an HD-era model from another livery
//! folder ([`crate::LiveryFolder`]).
//!
//! The ship-model loader builds `hdships\Zone\Ship.vex` for every craft and
//! then substitutes one material key (`zoneship_zone` on 2048 v1.04,
//! `zoneship_team` on Omega) with the craft's livery name, which names
//! `hdships\Zone\<livery>\Team.<ext>` (`docs/ghidra/functions/ps4-omega-eu/zone-craft.md`,
//! confidence 75). The livery name is the craft's own `PI_TeamModel
//! name="zone"` `texturelocation` in the plugin definition, so nothing here
//! invents a name: a craft that authors none keeps the hull's default skin and
//! the loader report says so.

use oag_title::race::ShipPaths;

/// One craft's substitution: the path component to replace and what to put there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Swap {
    key: String,
    livery: String,
}

impl Swap {
    /// Reads what names `key` as a path component from `livery` instead.
    pub(crate) fn new(key: &str, livery: &str) -> Self {
        Self {
            key: key.to_ascii_lowercase(),
            livery: livery.to_ascii_lowercase(),
        }
    }

    /// The archive path a texture request becomes, or `None` when the request
    /// is not under the key and so is read as asked.
    pub(crate) fn rewrite(&self, path: &str) -> Option<String> {
        let lower = path.to_ascii_lowercase().replace('\\', "/");
        let from = format!("/{}/", self.key);
        lower
            .contains(&from)
            .then(|| lower.replacen(&from, &format!("/{}/", self.livery), 1))
    }
}

/// The substitution `team` takes on this Zone hull, or `None` with the reason
/// in `report`. `liveries` is `(team id, livery name)` read off the definition.
pub(crate) fn swap(
    ships: ShipPaths,
    mode: oag_race::Mode,
    liveries: &[(String, String)],
    team: &str,
    report: &mut Vec<String>,
) -> Option<Swap> {
    if mode != oag_race::Mode::Zone {
        return None;
    }
    let key = ships.zone.livery_key()?;
    let Some((_, livery)) = liveries
        .iter()
        .find(|(id, _)| id.eq_ignore_ascii_case(team))
    else {
        report.push(format!(
            "{team}: the definition authors no Zone livery for this craft - the Zone hull keeps \
             its default {key} skin"
        ));
        return None;
    };
    Some(Swap {
        key: key.to_ascii_lowercase(),
        livery: livery.to_ascii_lowercase(),
    })
}

/// A hull build's texture reads through one [`Swap`], counted for the report.
pub(crate) struct Reads<'a> {
    swap: Option<&'a Swap>,
    /// Whether a texture the swap names but the archive lacks falls back to the hull's own:
    /// the Zone hull's default skin does; a model's livery does not, since the own folder's
    /// texture is another livery's paint.
    fall_back: bool,
    swapped: usize,
    missing: Vec<String>,
}

impl<'a> Reads<'a> {
    pub(crate) fn new(swap: Option<&'a Swap>, fall_back: bool) -> Self {
        Self {
            swap,
            fall_back,
            swapped: 0,
            missing: Vec::new(),
        }
    }

    /// The texture at `path`, read through the swap.
    pub(crate) fn read(
        &mut self,
        archives: &mut oag_assets::Archives,
        path: &str,
    ) -> Option<Vec<u8>> {
        let Some(alt) = self.swap.and_then(|swap| swap.rewrite(path)) else {
            return archives.read_name(path).ok();
        };
        if let Ok(blob) = archives.read_name(&alt) {
            self.swapped += 1;
            return Some(blob);
        }
        self.missing.push(alt);
        if self.fall_back {
            archives.read_name(path).ok()
        } else {
            None
        }
    }

    /// The report line, when a swap was asked for.
    pub(crate) fn describe(&self, team: &str) -> Option<String> {
        let swap = self.swap?;
        Some(if self.fall_back {
            describe(team, swap, self.swapped, &self.missing)
        } else if self.missing.is_empty() {
            format!(
                "{team}: livery {} replaces {} on {} texture request(s)",
                swap.livery, swap.key, self.swapped
            )
        } else {
            format!(
                "{team}: livery {} does not hold {} - those draw untextured ({} swapped)",
                swap.livery,
                self.missing.join(", "),
                self.swapped
            )
        })
    }
}

/// What a finished hull build did with `swap`, as a report line.
pub(crate) fn describe(team: &str, swap: &Swap, swapped: usize, missing: &[String]) -> String {
    if missing.is_empty() {
        format!(
            "{team}: Zone livery {} replaces {} on {swapped} texture request(s)",
            swap.livery, swap.key
        )
    } else {
        format!(
            "{team}: Zone livery {} named {}, which the archive does not hold - the default \
             {} skin stays ({swapped} swapped)",
            swap.livery,
            missing.join(", "),
            swap.key
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_path_under_the_key_is_rewritten() {
        let swap = Swap {
            key: "zoneship_team".into(),
            livery: "zoneship_quirex".into(),
        };
        assert_eq!(
            swap.rewrite("data/art/published/hdships/zone/zoneship_team/team.gnf")
                .as_deref(),
            Some("data/art/published/hdships/zone/zoneship_quirex/team.gnf")
        );
        assert_eq!(
            swap.rewrite(r"Data\art\published\hdships\Zone\Zoneship_Team\Team.gnf")
                .as_deref(),
            Some("data/art/published/hdships/zone/zoneship_quirex/team.gnf")
        );
        assert_eq!(
            swap.rewrite("data/art/published/hdships/zone/zoneship_zone/zone_tp_1024.gnf"),
            None
        );
    }
}
