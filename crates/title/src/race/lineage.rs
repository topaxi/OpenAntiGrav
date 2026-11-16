//! A circuit family a title races under another title's rules: the HUD it draws and the laps
//! it runs.
//!
//! Omega ships two games in one executable, HD's and 2048's, and the Race Box's `GAME MODE`
//! picks between them. Our circuit list is one list, and a circuit's directory says which game
//! it belongs to (`Data\Environments2048\`), the same key [`RaceDefaults::effect_dir_by_circuit`]
//! already uses for the particle directory.

use super::RaceDefaults;
use crate::hud::{HudArt, HudLayouts};

/// What a circuit family draws and runs instead of the title's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CircuitLineage {
    /// The leading part of a circuit's track path (either slash, any case).
    pub prefix: &'static str,
    /// The HUD layouts a race on this family reads, in place of [`crate::Title::hud`] and any
    /// `HUD Style` skin.
    pub layouts: &'static HudLayouts,
    /// How those layouts' sprites, fonts and readouts reach the screen, in place of
    /// [`crate::Title::hud_art`].
    pub art: &'static HudArt,
    /// The platform whose layout grid the set is authored in, when it is not the title's own
    /// (`oag_display::space::Space::of`): 2048's HUD is laid out for the Vita's 960x544 panel
    /// and Omega draws it scaled onto 1920x1080.
    pub grid: Option<oag_disc::Platform>,
    /// Laps of a single race per speed class, slowest rung first (the order of
    /// `oag_race::SpeedClass::ALL`), or `None` to keep the engine's table.
    pub laps_by_class: Option<[u32; 4]>,
}

impl RaceDefaults {
    /// The [`CircuitLineage`] of `circuit`: the first of [`Self::circuit_lineages`] whose prefix
    /// the track path starts with, or `None` for the title's own circuits.
    #[must_use]
    pub fn circuit_lineage(&self, circuit: &str) -> Option<&'static CircuitLineage> {
        let norm = |s: &str| s.replace('/', "\\").to_ascii_lowercase();
        let circuit = norm(circuit);
        self.circuit_lineages
            .iter()
            .find(|lineage| circuit.starts_with(&norm(lineage.prefix)))
    }
}
