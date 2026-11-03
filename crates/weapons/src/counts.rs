//! How many rockets a press fires and how many mines a drop lays, per craft.
//!
//! 2048 and Omega read `number_of_rockets` and `number_of_mines` off the craft's own
//! `<Weapons>` line; the executable truncates the number and clamps it to `1..=3`
//! and `1..=5`, and a craft whose file leaves the attribute out keeps the value its
//! stats record is constructed with, three and five
//! (`docs/ghidra/functions/ps4-omega-eu/weapon-counts.md`). Pulse, Pure and HD author
//! neither attribute and their executables carry neither string, so every craft there
//! is the default.

use oag_tables::handling::Stats;

/// The most rockets one press puts in the air: the array every volley is built in.
pub const MAX_ROCKETS: u8 = 3;

/// The most mines one drop lays.
pub const MAX_MINES: u8 = 5;

/// One craft's rocket and mine counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeaponCounts {
    /// Rockets per press, `1..=`[`MAX_ROCKETS`].
    pub rockets: u8,
    /// Mines per drop, `1..=`[`MAX_MINES`].
    pub mines: u8,
}

impl Default for WeaponCounts {
    /// The stats record's constructor values: a craft that authors no count.
    fn default() -> Self {
        Self {
            rockets: MAX_ROCKETS,
            mines: MAX_MINES,
        }
    }
}

impl WeaponCounts {
    /// The counts a craft's authored `<Weapons>` line gives, the executable's law:
    /// truncate toward zero, then clamp (rockets `1..=3`, mines `1..=5`); an attribute
    /// the line leaves out is the [`Default`].
    #[must_use]
    pub fn from_authored(rockets: Option<f32>, mines: Option<f32>) -> Self {
        let default = Self::default();
        Self {
            rockets: rockets.map_or(default.rockets, |n| clamp(n, MAX_ROCKETS)),
            mines: mines.map_or(default.mines, |n| clamp(n, MAX_MINES)),
        }
    }

    /// The counts one craft's `handlingstats.xml` gives.
    #[must_use]
    pub fn from_stats(stats: &Stats) -> Self {
        match stats.weapons {
            Some(weapons) => {
                Self::from_authored(weapons.number_of_rockets, weapons.number_of_mines)
            }
            None => Self::default(),
        }
    }
}

/// Truncates and clamps one authored count to `1..=max`.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn clamp(authored: f32, max: u8) -> u8 {
    // `as i32` truncates toward zero and saturates, the executable's `(int)` cast.
    (authored as i32).clamp(1, i32::from(max)) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_constructor_values() {
        assert_eq!(
            WeaponCounts::default(),
            WeaponCounts {
                rockets: 3,
                mines: 5
            }
        );
        assert_eq!(
            WeaponCounts::from_authored(None, None),
            WeaponCounts::default()
        );
    }

    #[test]
    fn an_authored_count_truncates_then_clamps() {
        let counts = |r, m| WeaponCounts::from_authored(Some(r), Some(m));
        assert_eq!(
            counts(2.0, 4.0),
            WeaponCounts {
                rockets: 2,
                mines: 4
            }
        );
        assert_eq!(
            counts(1.9, 3.9),
            WeaponCounts {
                rockets: 1,
                mines: 3
            }
        );
        assert_eq!(
            counts(0.0, 0.0),
            WeaponCounts {
                rockets: 1,
                mines: 1
            }
        );
        assert_eq!(
            counts(-2.0, -1.0),
            WeaponCounts {
                rockets: 1,
                mines: 1
            }
        );
        assert_eq!(
            counts(9.0, 9.0),
            WeaponCounts {
                rockets: 3,
                mines: 5
            }
        );
    }

    #[test]
    fn one_attribute_leaves_the_other_at_its_default() {
        assert_eq!(
            WeaponCounts::from_authored(Some(1.0), None),
            WeaponCounts {
                rockets: 1,
                mines: 5
            }
        );
    }
}
