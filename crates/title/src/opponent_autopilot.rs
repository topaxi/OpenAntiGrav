//! Whether an opponent on this project's driver flies the Autopilot pickup it
//! holds, per speed class, or absorbs it.
//!
//! **Chosen, not measured, on every title.** The original's own field never
//! fires an Autopilot outside an Eliminator (`WeaponAi_Update`'s default arm,
//! `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`), so there is no law to
//! read: the maintainer chose (2026-10-19) to let an opponent on this
//! project's driver fly the original's player driver for the pickup's `time`
//! below PHANTOM, and to keep PHANTOM absorbing it. RAPIER is held off as
//! well, by the measurement: there it costs walls and rescues the way PHANTOM
//! does. The per-class split rests on paired-fork measurements of this
//! engine, not of the original: `docs/gameplay/ai.md`, "An opponent's
//! Autopilot, wired below PHANTOM".
//! Data, not a comparison on a title's or a class's name (ADR-0058).

use crate::pre_race::Sourced;

/// One class's switch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassSwitch {
    /// The class name as the title's files spell it (matched case-insensitively).
    pub class: &'static str,
    /// Whether an opponent at this class flies the pickup.
    pub flies: Sourced<bool>,
}

/// One title's per-class switches. A class not listed absorbs the pickup.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpponentAutopilot {
    /// Every class the rule names, slowest first.
    pub classes: &'static [ClassSwitch],
}

impl OpponentAutopilot {
    /// Whether an opponent at `class` flies the pickup; `false` for a class
    /// the rule does not name.
    #[must_use]
    pub fn flies(&self, class: &str) -> bool {
        self.classes
            .iter()
            .find(|switch| switch.class.eq_ignore_ascii_case(class))
            .is_some_and(|switch| switch.flies.value)
    }
}

/// Off at every class: what every race did before the switch was wired.
pub const ABSORBED: OpponentAutopilot = OpponentAutopilot { classes: &[] };

/// The maintainer's rule, below PHANTOM, as the measurement leaves it: on at
/// VENOM and FLASH (and Pure's VECTOR below them), off at RAPIER and PHANTOM.
///
/// RAPIER, Ace, 19 cells: wall ticks up on 10 (3,321 -> 4,406 summed), and
/// rescues where the absorb had none on three cells (9 rescues), against
/// FLASH's 6 cells up (2,114 -> 1,416) and one rescue from zero. RAPIER Novice
/// alone was clean (walls 17,966 -> 7,556, rescues 23 -> 7) but has one
/// rescue-from-zero cell (Pulse 03, 4), and the switch has no difficulty axis.
pub const BELOW_RAPIER: OpponentAutopilot = OpponentAutopilot {
    classes: &[
        ClassSwitch {
            class: "VECTOR",
            flies: Sourced::chosen(true),
        },
        ClassSwitch {
            class: "VENOM",
            flies: Sourced::chosen(true),
        },
        ClassSwitch {
            class: "FLASH",
            flies: Sourced::chosen(true),
        },
        ClassSwitch {
            class: "RAPIER",
            flies: Sourced::chosen(false),
        },
        ClassSwitch {
            class: "PHANTOM",
            flies: Sourced::chosen(false),
        },
    ],
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::Origin;

    #[test]
    fn below_rapier_flies_every_class_under_rapier_and_none_above() {
        for class in ["Vector", "VENOM", "flash"] {
            assert!(BELOW_RAPIER.flies(class), "{class}");
        }
        for class in ["rapier", "PHANTOM", "SUPERPHANTOM", "ZONE", ""] {
            assert!(!BELOW_RAPIER.flies(class), "{class}");
        }
        assert!(!ABSORBED.flies("VENOM"));
        assert!(
            BELOW_RAPIER
                .classes
                .iter()
                .all(|switch| switch.flies.origin == Origin::Chosen)
        );
    }
}
