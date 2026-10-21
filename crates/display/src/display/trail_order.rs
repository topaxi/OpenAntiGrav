//! [`TrailOrder`]: where a craft's engine ribbon draws against the circuit's
//! translucent surfaces.

use serde::{Deserialize, Serialize};

/// Where the engine ribbons draw relative to the circuit's blended list (the
/// translucent floor patches, glass and decals a circuit authors as list B).
///
/// **Only meaningful where the ribbon is the PSP preset** (Pulse and Pure,
/// whose `Trail` the original queues at a constant key); HD/Fury, Omega and
/// 2048 keep their own order and offer no choice.
/// `[render_profiles.<title> (<platform>)] trail_order`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum TrailOrder {
    /// The original's order, and the default: every ribbon before the
    /// circuit's blended list, so a translucent floor patch drawn later paints
    /// over the ribbon where it passes its own depth test (the ribbon writes no
    /// depth). Measured: `Trail_Submit` queues every `Trail` at the constant
    /// key `0x4d000000`, ahead of every list-B mesh in the same layer
    /// (`docs/ghidra/functions/psp-pulse-usa/exhaust.md`, "The ribbon is queued
    /// first").
    #[default]
    Original,
    /// The ribbons after the circuit's blended list, with the flares, so no
    /// translucent floor dims them: the order this project drew before the
    /// original's was measured. **Chosen, not measured**, for a player who
    /// prefers the brighter trail.
    OverFloors,
}

impl TrailOrder {
    /// The spelling used in a settings file, on a menu row and on `--trail-order`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Original => "original",
            Self::OverFloors => "over-floors",
        }
    }

    /// Every choice, for the menus and for error messages.
    pub const ALL: [Self; 2] = [Self::Original, Self::OverFloors];

    /// Whether the ribbons draw ahead of the circuit's blended list.
    #[must_use]
    pub fn ribbons_first(self) -> bool {
        matches!(self, Self::Original)
    }
}

impl std::str::FromStr for TrailOrder {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|order| order.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a trail order; try original or over-floors"))
    }
}

impl std::fmt::Display for TrailOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl TryFrom<String> for TrailOrder {
    type Error = String;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<TrailOrder> for String {
    fn from(order: TrailOrder) -> Self {
        order.to_string()
    }
}
