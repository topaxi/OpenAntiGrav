//! Whether an opponent on this project's driver flies the Autopilot pickup it
//! holds, per speed class, or absorbs it.
//!
//! **Chosen, not measured, on every title.** The original's own field never
//! fires an Autopilot outside an Eliminator (`WeaponAi_Update`'s default arm,
//! `docs/ghidra/functions/psp-pulse-usa/weapon-ai.md`), so there is no law to
//! read: the maintainer chose (2026-10-19) to let an opponent on this
//! project's driver fly the original's player driver for the pickup's `time`
//! below PHANTOM, and to keep PHANTOM absorbing it. The per-class split rests
//! on paired-fork measurements of this engine, not of the original:
//! `docs/gameplay/ai.md`, "An opponent's Autopilot, wired below PHANTOM".
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

/// The maintainer's rule, below PHANTOM: on at VENOM, FLASH and RAPIER, off at
/// PHANTOM. Pure's VECTOR (below VENOM) is on with them.
pub const BELOW_PHANTOM: OpponentAutopilot = OpponentAutopilot {
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
            flies: Sourced::chosen(true),
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
    fn below_phantom_flies_every_class_under_phantom_and_none_above() {
        for class in ["Vector", "VENOM", "flash", "RAPIER"] {
            assert!(BELOW_PHANTOM.flies(class), "{class}");
        }
        for class in ["PHANTOM", "SUPERPHANTOM", "ZONE", ""] {
            assert!(!BELOW_PHANTOM.flies(class), "{class}");
        }
        assert!(!ABSORBED.flies("VENOM"));
        assert!(
            BELOW_PHANTOM
                .classes
                .iter()
                .all(|switch| switch.flies.origin == Origin::Chosen)
        );
    }
}
