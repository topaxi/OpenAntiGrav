//! Whether an opponent on this project's driver flies the Autopilot pickup it
//! holds, per speed class and difficulty, or absorbs it.
//!
//! **Chosen, not measured, on every title.** The original's own field never
//! fires an Autopilot outside an Eliminator (`WeaponAi_Update`'s default arm,
//! `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`), so there is no law to
//! read: the maintainer chose (2026-10-19) to let an opponent on this
//! project's driver fly the original's player driver for the pickup's `time`
//! where that beats absorbing it, and (2026-10-20) to let the stronger
//! difficulties absorb it where they are better without it. Each switch rests
//! on paired-fork measurements of this engine, not of the original:
//! `docs/gameplay/ai.md`, "An opponent's Autopilot, per class and difficulty".
//!
//! The difficulty is the race's own `oag_ai::Difficulty` by name (`novice`,
//! `skilled`, `elite`, `ace`), the same four levels on every title: the race
//! box's `AI DIFFICULTY` row (`assets/ui/menu.toml`, `ai.difficulty`) offers
//! all four by these names. A title's own `Easy`/`Medium`/`Hard` exist only
//! as campaign rungs and reach only the original's pack shuffle
//! (`oag_game::ai_choice::free_level`); the original-AI field (the campaign)
//! keeps absorbing the pickup.
//! Data, not a comparison on a title's, class's or level's name (ADR-0058).

use crate::pre_race::Sourced;

/// One class and difficulty's switch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Switch {
    /// The class name as the title's files spell it (matched case-insensitively).
    pub class: &'static str,
    /// The race's difficulty by name (matched case-insensitively).
    pub difficulty: &'static str,
    /// Whether an opponent in this cell flies the pickup.
    pub flies: Sourced<bool>,
}

/// One title's switches. A cell not listed absorbs the pickup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpponentAutopilot {
    /// Every cell the rule names.
    pub switches: &'static [Switch],
}

impl OpponentAutopilot {
    /// Whether an opponent at `class` and `difficulty` flies the pickup;
    /// `false` for a cell the rule does not name.
    #[must_use]
    pub fn flies(&self, class: &str, difficulty: &str) -> bool {
        self.switches
            .iter()
            .find(|switch| {
                switch.class.eq_ignore_ascii_case(class)
                    && switch.difficulty.eq_ignore_ascii_case(difficulty)
            })
            .is_some_and(|switch| switch.flies.value)
    }
}

/// Off in every cell: what every race did before the switch was wired.
pub const ABSORBED: OpponentAutopilot = OpponentAutopilot { switches: &[] };

const fn cell(class: &'static str, difficulty: &'static str, flies: bool) -> Switch {
    Switch {
        class,
        difficulty,
        flies: Sourced::chosen(flies),
    }
}

/// The split the measurement leaves, the same on every title: VECTOR, VENOM
/// and FLASH fly at every level, RAPIER at every level but Ace, PHANTOM at
/// none. SUPERPHANTOM is not listed (absorbs): the field does not race at it
/// on 2048 and Omega's handling authors no such class.
///
/// The rule, per class and level pooled over every circuit, each circuit the
/// median of five runs: walls summed do not rise, rescues summed do not rise
/// and no circuit is rescued where the absorb was not, and the gain summed is
/// positive. RAPIER Novice fails on Pulse `03_Track` alone (rescues 0 -> 1)
/// and flies anyway: the maintainer's choice (2026-10-20), clean in total.
/// RAPIER Ace fails on walls (3,321 -> 4,407) and 2048 (rescues 0 -> 1), PHANTOM on
/// walls or rescues from zero at every level.
pub const BY_CLASS_AND_DIFFICULTY: OpponentAutopilot = OpponentAutopilot {
    switches: &[
        cell("VECTOR", "novice", true),
        cell("VECTOR", "skilled", true),
        cell("VECTOR", "elite", true),
        cell("VECTOR", "ace", true),
        cell("VENOM", "novice", true),
        cell("VENOM", "skilled", true),
        cell("VENOM", "elite", true),
        cell("VENOM", "ace", true),
        cell("FLASH", "novice", true),
        cell("FLASH", "skilled", true),
        cell("FLASH", "elite", true),
        cell("FLASH", "ace", true),
        cell("RAPIER", "novice", true),
        cell("RAPIER", "skilled", true),
        cell("RAPIER", "elite", true),
        cell("RAPIER", "ace", false),
        cell("PHANTOM", "novice", false),
        cell("PHANTOM", "skilled", false),
        cell("PHANTOM", "elite", false),
        cell("PHANTOM", "ace", false),
    ],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::Origin;

    #[test]
    fn every_switch_is_chosen_and_an_unnamed_cell_absorbs() {
        assert!(
            BY_CLASS_AND_DIFFICULTY
                .switches
                .iter()
                .all(|switch| switch.flies.origin == Origin::Chosen)
        );
        assert!(!ABSORBED.flies("VENOM", "novice"));
        assert!(!BY_CLASS_AND_DIFFICULTY.flies("ZONE", "novice"));
        assert!(!BY_CLASS_AND_DIFFICULTY.flies("VENOM", ""));
        assert!(BY_CLASS_AND_DIFFICULTY.flies("Venom", "NOVICE"));
        assert!(!BY_CLASS_AND_DIFFICULTY.flies("SUPERPHANTOM", "novice"));
    }

    #[test]
    fn rapier_flies_at_every_level_but_ace() {
        let rapier: Vec<bool> = ["novice", "skilled", "elite", "ace"]
            .iter()
            .map(|level| BY_CLASS_AND_DIFFICULTY.flies("RAPIER", level))
            .collect();
        assert_eq!(rapier, [true, true, true, false]);
    }
}
