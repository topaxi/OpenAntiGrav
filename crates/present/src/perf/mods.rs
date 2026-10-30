//! The simulation mods block of the `Dev` overlay: each module's fuel and call time,
//! drawn under the network block.
//!
//! Plain data in, [`Draw`]s out, like [`super::net`]: this crate knows no wasm host, so
//! the composition root maps `oag_raceplay::extensions::Report` into a [`Readout`].
//! Every number is a per-tick output of the host, never simulation state.
//!
//! Layout is chosen, not measured.

use oag_ui::frontend::{Align, Draw};

use super::{BAD, LINE, PAD, PANEL, TEXT, WARN};

/// The panel's width in grid units: 30 glyphs. Chosen.
pub const WIDTH: f32 = 192.0;
/// A module past this share of its fuel budget reads as a warning. Chosen.
const WARN_SHARE: f32 = 0.5;

/// One module, after the last tick.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Module {
    /// The mod's id.
    pub id: String,
    /// Fuel the last call used.
    pub fuel_used: u64,
    /// The most a call used this race.
    pub fuel_peak: u64,
    /// The per-call budget.
    pub fuel_budget: u64,
    /// The last call's wall time, microseconds.
    pub call_us: f32,
    /// The tick it faulted on and why; it has not run since.
    pub faulted: Option<(u64, String)>,
}

/// Every module of a modded race, in the order they run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Readout {
    /// The modules.
    pub modules: Vec<Module>,
}

impl Readout {
    fn rows(&self) -> Vec<(String, [f32; 4])> {
        let mut rows = vec![("MODS".to_string(), TEXT)];
        for module in &self.modules {
            let id: String = module
                .id
                .chars()
                .take(28)
                .collect::<String>()
                .to_uppercase();
            rows.push((id, TEXT));
            if let Some((tick, why)) = &module.faulted {
                rows.push((format!(" FAULTED AT TICK {tick}"), BAD));
                rows.push((
                    format!(
                        " {}",
                        why.chars().take(28).collect::<String>().to_uppercase()
                    ),
                    BAD,
                ));
                continue;
            }
            let budget = module.fuel_budget.max(1) as f32;
            let share = module.fuel_used as f32 / budget;
            let peak = module.fuel_peak as f32 / budget;
            let color = if peak > WARN_SHARE { WARN } else { TEXT };
            rows.push((
                format!(" FUEL {} PEAK {}", module.fuel_used, module.fuel_peak),
                color,
            ));
            rows.push((
                format!(
                    " {:.1}% OF {}  {:.0} US",
                    share * 100.0,
                    module.fuel_budget,
                    module.call_us
                ),
                color,
            ));
        }
        rows
    }
}

/// The block, its top-right corner at (`right`, `top`), and its height; nothing for
/// `None` (a race with no simulation mod).
#[must_use]
pub fn draw_list(readout: Option<&Readout>, right: f32, top: f32) -> (Vec<Draw>, f32) {
    let Some(readout) = readout else {
        return (Vec::new(), 0.0);
    };
    let rows = readout.rows();
    let height = PAD * 2.0 + rows.len() as f32 * LINE;
    let left = right - WIDTH;
    let mut out = vec![Draw::Fill {
        rect: [left, top, WIDTH, height],
        color: PANEL,
    }];
    for (row, (text, color)) in rows.into_iter().enumerate() {
        out.push(Draw::Text {
            x: left + PAD,
            y: top + PAD + row as f32 * LINE,
            scale: 1.0,
            color,
            border: None,
            align: Align::Left,
            text,
            wrap_width: None,
        });
    }
    (out, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(list: &[Draw]) -> Vec<String> {
        list.iter()
            .filter_map(|d| match d {
                Draw::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_module_shows_its_fuel_its_share_and_its_call_time() {
        let readout = Readout {
            modules: vec![Module {
                id: "oag.wall-penalty".to_string(),
                fuel_used: 1_000,
                fuel_peak: 3_000,
                fuel_budget: 200_000,
                call_us: 7.4,
                faulted: None,
            }],
        };
        let (list, height) = draw_list(Some(&readout), 400.0, 10.0);
        assert!(height > 0.0);
        assert_eq!(
            texts(&list),
            [
                "MODS",
                "OAG.WALL-PENALTY",
                " FUEL 1000 PEAK 3000",
                " 0.5% OF 200000  7 US"
            ]
        );
    }

    #[test]
    fn a_faulted_module_names_its_tick_and_no_mod_draws_nothing() {
        let readout = Readout {
            modules: vec![Module {
                id: "spin".to_string(),
                faulted: Some((42, "out of fuel".to_string())),
                ..Module::default()
            }],
        };
        let (list, _) = draw_list(Some(&readout), 400.0, 10.0);
        assert!(texts(&list).contains(&" FAULTED AT TICK 42".to_string()));
        assert_eq!(draw_list(None, 400.0, 10.0), (Vec::new(), 0.0));
    }
}
