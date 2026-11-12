//! `XPAwards.xml` and `XPRanks.xml`: a profile's experience points and the
//! rank they add up to (Wipeout 2048, and Omega's byte-identical awards file).
//!
//! The law is `docs/ghidra/functions/vita-2048-eu-v104/xp-award.md`:
//! `Profile_AwardXP` pays one column of an award row per game mode,
//! `Profile_GetXPRank` is the count of rank thresholds at or below the total,
//! and `Profile_GetRankThresholdXP` (`0x8101361a`) is `0` for rank 0, the
//! authored threshold for ranks 1 to the maximum, and the total itself above
//! that. Nothing is transcribed: every number is read off the shipped XML.

use crate::fexml::{self, Node};

/// Which of an award row's five columns a game mode pays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Zone,
    Elim,
    Zombie,
    Race,
    Speed,
}

/// The `index` of the awards a finished event pays (`XPAwardTypes`).
pub mod award {
    /// `Event complete- PASS`.
    pub const EVENT_PASS: u32 = 0;
    /// `Event complete- ELITE`.
    pub const EVENT_ELITE: u32 = 1;
    /// `Finished 1st!`; the following seven are 2nd to 8th.
    pub const FINISHED_1ST: u32 = 24;
    /// `Finished 8th!`.
    pub const FINISHED_8TH: u32 = 31;
}

/// One `<Award>` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Award {
    pub index: u32,
    pub name: String,
    /// Zone, elimination, zombie, race, speed, in the file's own order.
    pub xp: [u32; 5],
    pub group: Option<String>,
}

/// `XPAwards.xml`.
#[derive(Debug, Clone, PartialEq)]
pub struct Awards {
    /// `ClassXPMultipliers`: venom, flash, rapier, phantom, superphantom.
    pub class_multipliers: [f32; 5],
    /// `SkillXPMultipliers`: easy, medium, hard.
    pub skill_multipliers: [f32; 3],
    pub awards: Vec<Award>,
}

impl Awards {
    /// Parses the file's text; `None` when it carries no `<Award>` row.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let root = fexml::parse(text);
        let doc = root.children_named("XPAwards").next()?;
        let mults = |name: &str, keys: &[&str], out: &mut [f32]| {
            if let Some(node) = doc.children_named(name).next() {
                for (slot, key) in out.iter_mut().zip(keys) {
                    *slot = node.attr(key).and_then(|v| v.parse().ok()).unwrap_or(1.0);
                }
            }
        };
        let mut class_multipliers = [1.0; 5];
        mults(
            "ClassXPMultipliers",
            &["venom", "flash", "rapier", "phantom", "superphantom"],
            &mut class_multipliers,
        );
        let mut skill_multipliers = [1.0; 3];
        mults(
            "SkillXPMultipliers",
            &["easy", "medium", "hard"],
            &mut skill_multipliers,
        );
        let awards: Vec<Award> = doc.children_named("Award").filter_map(award_row).collect();
        (!awards.is_empty()).then_some(Self {
            class_multipliers,
            skill_multipliers,
            awards,
        })
    }

    /// The row with this `index`.
    #[must_use]
    pub fn get(&self, index: u32) -> Option<&Award> {
        self.awards.iter().find(|a| a.index == index)
    }

    /// What award `index` pays in `column`, unscaled; `0` for an unknown index.
    #[must_use]
    pub fn xp(&self, index: u32, column: Column) -> u32 {
        self.get(index).map_or(0, |a| a.xp[column as usize])
    }

    /// The XP a finished event pays in `column`: its completion award
    /// (`elite` picks `Event complete- ELITE`, else `PASS`; `None` pays no
    /// completion) plus `Finished Nth` for a finishing `place` (1-based, 1 to 8).
    #[must_use]
    pub fn race_end(&self, column: Column, completion: Option<bool>, place: Option<u8>) -> u32 {
        let complete = completion.map_or(0, |elite| {
            self.xp(
                if elite {
                    award::EVENT_ELITE
                } else {
                    award::EVENT_PASS
                },
                column,
            )
        });
        let finish = place.filter(|p| (1..=8).contains(p)).map_or(0, |p| {
            self.xp(award::FINISHED_1ST + u32::from(p) - 1, column)
        });
        complete + finish
    }
}

fn award_row(node: &Node) -> Option<Award> {
    let num = |k: &str| node.attr(k).and_then(|v| v.trim().parse::<u32>().ok());
    Some(Award {
        index: num("index")?,
        name: node.attr("name").unwrap_or_default().to_owned(),
        xp: [
            num("zone")?,
            num("elim")?,
            num("zombie")?,
            num("race")?,
            num("speed")?,
        ],
        group: node.attr("group").map(str::to_owned),
    })
}

/// `XPRanks.xml`: the cumulative XP at which each rank is reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ranks {
    /// `MaximumRank`.
    pub maximum: u32,
    /// Threshold of rank `n + 1` at `thresholds[n]`.
    pub thresholds: Vec<u32>,
}

impl Ranks {
    /// Parses the file's text; `None` when it has no `<Rank>` row.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let root = fexml::parse(text);
        let doc = root.children_named("XPRanks").next()?;
        let mut rows: Vec<(u32, u32)> = doc
            .children_named("Rank")
            .filter_map(|n| {
                Some((
                    n.attr("index")?.trim().parse().ok()?,
                    n.attr("value")?.trim().parse().ok()?,
                ))
            })
            .collect();
        rows.sort_by_key(|&(i, _)| i);
        let thresholds: Vec<u32> = rows.into_iter().map(|(_, v)| v).collect();
        let maximum = doc
            .children_named("MaximumRank")
            .next()
            .and_then(|n| n.attr("value")?.trim().parse().ok())
            .unwrap_or(thresholds.len() as u32);
        (!thresholds.is_empty()).then_some(Self {
            maximum,
            thresholds,
        })
    }

    /// `Profile_GetXPRank`: how many thresholds `total` has reached, at most
    /// [`Self::maximum`].
    #[must_use]
    pub fn rank(&self, total: u32) -> u32 {
        let reached = self.thresholds.iter().take_while(|&&t| t <= total).count() as u32;
        reached.min(self.maximum)
    }

    /// `Profile_GetRankThresholdXP`: `0` for rank 0, the authored threshold
    /// for ranks 1 to the maximum, and `total` itself above it.
    #[must_use]
    pub fn threshold_xp(&self, rank: u32, total: u32) -> u32 {
        match rank {
            0 => 0,
            r if r <= self.maximum => self
                .thresholds
                .get(r as usize - 1)
                .copied()
                .unwrap_or(total),
            _ => total,
        }
    }

    /// `GameModeBase_IsUnlockable`'s rank test: whether `total` reaches the
    /// threshold of `rank_required` (an event with no requirement is open).
    #[must_use]
    pub fn meets(&self, total: u32, rank_required: Option<i32>) -> bool {
        let Some(required) = rank_required.and_then(|r| u32::try_from(r).ok()) else {
            return true;
        };
        total >= self.threshold_xp(required, total)
    }
}

#[cfg(test)]
mod tests;
