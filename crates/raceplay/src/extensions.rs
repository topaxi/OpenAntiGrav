//! Simulation mods in a race: the one call per tick, the event buffer, the command
//! sinks and each module's state blob (`docs/architecture/wasm-extensions/hooks.md`).
//!
//! - **Event**: `wall-contact{slot}`, recorded by [`Race::count_wall_contact`] for every
//!   racing craft, a person's or the AI's, on each tick it touches a wall (the same test
//!   [`RaceSim::wall_contact_ticks`] counts). Raised after the call, so it reaches the
//!   guest in the next tick's observation: a one-tick latency, identical everywhere.
//! - **The call**: [`Race::tick_extensions`], between `advance_cannons` and the people's
//!   steps (phase C of `hooks.md`). Modules run in id order; each sees the same events.
//! - **Command**: `scale-thrust{slot, factor}`, `factor` in `[0, 1]`, **multiplied** into
//!   `Ship::pending_thrust_scale`, which this tick's step consumes for a person's craft
//!   (`player_step.rs`) and an AI craft (`field.rs`) alike. Multiplied, not assigned, so a
//!   Leach Beam drain armed at the end of the last tick is kept.
//! - **State**: each module's blob and fault tick live in [`RaceSim`], cloned with a
//!   snapshot and hashed by [`Extensions::hash_into`]. A race with no module hashes
//!   nothing here, so every vanilla hash is unchanged.
//!
//! Behind the `extensions` cargo feature: off, [`prepare`] refuses every source by name
//! and nothing of wasmtime is built or linked.

use std::sync::Arc;

/// The most simulation modules one race runs. Chosen.
pub const MAX_SIM_MODS: usize = 4;

/// Reads a module's bytes, on the load thread.
pub type Reader = Arc<dyn Fn() -> Result<Vec<u8>, String> + Send + Sync>;

/// A simulation module a mod that is on offers this race: what the composition root
/// found in `oag-mod.toml`, with a reader for the `.wasm` so nothing is opened until
/// the race loads it.
#[derive(Clone)]
pub struct Source {
    /// The mod's id; modules run in the order these sort.
    pub mod_id: String,
    /// The module's path in the mod, for the load report.
    pub module: String,
    /// `[extension] memory_pages`.
    pub memory_pages: u32,
    /// `[extension] state_bytes`.
    pub state_bytes: u32,
    /// Reads the module.
    pub read: Reader,
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Source")
            .field("mod_id", &self.mod_id)
            .field("module", &self.module)
            .field("memory_pages", &self.memory_pages)
            .field("state_bytes", &self.state_bytes)
            .finish_non_exhaustive()
    }
}

/// The modules the load compiled, for [`Race::start`](crate::Race::start).
#[derive(Debug, Clone, Default)]
pub struct Prepared {
    #[cfg(feature = "extensions")]
    modules: Vec<(String, oag_modhost::Module)>,
}

/// Reads and compiles `sources`, at race load. A module that fails leaves its mod out
/// and says why in `report`; the race runs without it.
#[must_use]
pub fn prepare(sources: &[Source], report: &mut Vec<String>) -> Prepared {
    #[cfg(not(feature = "extensions"))]
    {
        for source in sources {
            report.push(format!(
                "mod {}: {} left out: this build has no `extensions` feature",
                source.mod_id, source.module
            ));
        }
        Prepared::default()
    }
    #[cfg(feature = "extensions")]
    {
        let mut sorted: Vec<&Source> = sources.iter().collect();
        sorted.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));
        let mut modules = Vec::new();
        for source in sorted {
            let label = format!("mod {}: {}", source.mod_id, source.module);
            if modules.len() == MAX_SIM_MODS {
                report.push(format!(
                    "{label} left out: a race runs at most {MAX_SIM_MODS} simulation modules"
                ));
                continue;
            }
            let limits = oag_modhost::Limits {
                memory_pages: source.memory_pages,
                state_bytes: source.state_bytes,
                fuel: oag_modhost::FUEL_PER_TICK,
            };
            match (source.read)()
                .map_err(|e| format!("cannot be read: {e}"))
                .and_then(|wasm| {
                    oag_modhost::Module::compile(&wasm, limits).map_err(|e| e.to_string())
                }) {
                Ok(module) => {
                    report.push(format!(
                        "{label}: simulation module loaded, {} state bytes, {} fuel a tick; the race is MODDED and records no replay",
                        source.state_bytes,
                        oag_modhost::FUEL_PER_TICK
                    ));
                    modules.push((source.mod_id.clone(), module));
                }
                Err(e) => report.push(format!("{label} left out: {e}")),
            }
        }
        Prepared { modules }
    }
}

/// One module's numbers for the dev overlay, after the last tick. An output, never
/// state, never hashed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Report {
    /// The mod's id.
    pub mod_id: String,
    /// Fuel the last call used.
    pub fuel_used: u64,
    /// The most any call this race used.
    pub fuel_peak: u64,
    /// Fuel a call may use.
    pub fuel_budget: u64,
    /// The last call's wall time, microseconds.
    pub call_us: f32,
    /// The tick the module faulted on, and why; it has not run since.
    pub faulted: Option<(u64, String)>,
}

/// The race's simulation modules: their state (hashed) and their reports (not).
#[derive(Debug, Clone, Default)]
pub struct Extensions {
    #[cfg(feature = "extensions")]
    on: on::On,
}

impl Extensions {
    /// The state for the modules `prepared` holds, blobs zeroed.
    #[must_use]
    pub fn new(prepared: Prepared) -> Self {
        #[cfg(not(feature = "extensions"))]
        {
            let _ = prepared;
            Self::default()
        }
        #[cfg(feature = "extensions")]
        {
            Self {
                on: on::On::new(prepared.modules),
            }
        }
    }

    /// Whether any module runs in this race: the race is MODDED.
    #[must_use]
    pub fn any(&self) -> bool {
        #[cfg(feature = "extensions")]
        {
            !self.on.slots.is_empty()
        }
        #[cfg(not(feature = "extensions"))]
        {
            false
        }
    }

    /// Records a wall contact for `slot` this tick (at most once per slot per tick).
    pub(super) fn wall_contact(&mut self, slot: usize) {
        #[cfg(feature = "extensions")]
        self.on.wall_contact(slot);
        #[cfg(not(feature = "extensions"))]
        let _ = slot;
    }

    /// The modules' state into the race hash: nothing at all when no module runs.
    pub(super) fn hash_into(&self, hasher: &mut oag_core::hash::StateHasher) {
        #[cfg(feature = "extensions")]
        self.on.hash_into(hasher);
        #[cfg(not(feature = "extensions"))]
        let _ = hasher;
    }

    /// Each module's numbers after the last tick, in id order.
    #[must_use]
    pub fn reports(&self) -> Vec<Report> {
        #[cfg(feature = "extensions")]
        {
            self.on.reports()
        }
        #[cfg(not(feature = "extensions"))]
        {
            Vec::new()
        }
    }
}

impl crate::Race {
    /// Phase C: each module's `tick`, its commands into this tick's step. With no
    /// module, returns at once and touches nothing.
    pub(super) fn tick_extensions(&mut self) {
        #[cfg(feature = "extensions")]
        on::tick(&mut self.sim);
    }

    /// The modules' numbers for the dev overlay ([`Report`]).
    #[must_use]
    pub fn extension_reports(&self) -> Vec<Report> {
        self.sim.extensions.reports()
    }

    /// Whether a simulation module runs in this race (MODDED: no replay, README 4.4).
    #[must_use]
    pub fn is_modded(&self) -> bool {
        self.sim.extensions.any()
    }
}

#[cfg(feature = "extensions")]
mod on {
    use oag_modhost::{Command, Event, Module, Observation, abi::MAX_EVENTS};

    use super::Report;
    use crate::sim::RaceSim;

    #[derive(Debug, Clone)]
    pub(super) struct Slot {
        mod_id: String,
        module: Module,
        blob: Vec<u8>,
        faulted: Option<(u64, String)>,
        fuel_used: u64,
        fuel_peak: u64,
        call_us: f32,
    }

    #[derive(Debug, Clone, Default)]
    pub(super) struct On {
        pub(super) slots: Vec<Slot>,
        /// One bit per slot that touched a wall since the last call: the event buffer,
        /// at most one `wall-contact` per craft per tick, raised in slot order.
        walls: u32,
    }

    impl On {
        pub(super) fn new(modules: Vec<(String, Module)>) -> Self {
            Self {
                slots: modules
                    .into_iter()
                    .map(|(mod_id, module)| Slot {
                        blob: vec![0; module.limits().state_bytes as usize],
                        mod_id,
                        module,
                        faulted: None,
                        fuel_used: 0,
                        fuel_peak: 0,
                        call_us: 0.0,
                    })
                    .collect(),
                walls: 0,
            }
        }

        pub(super) fn wall_contact(&mut self, slot: usize) {
            if !self.slots.is_empty() {
                self.walls |= 1 << slot;
            }
        }

        pub(super) fn hash_into(&self, hasher: &mut oag_core::hash::StateHasher) {
            if self.slots.is_empty() {
                return;
            }
            hasher.write_u32(self.slots.len() as u32);
            hasher.write_u32(self.walls);
            for slot in &self.slots {
                hasher.write(&slot.blob);
                match &slot.faulted {
                    None => hasher.write_u8(0),
                    Some((tick, _)) => {
                        hasher.write_u8(1);
                        hasher.write_u64(*tick);
                    }
                }
            }
        }

        pub(super) fn reports(&self) -> Vec<Report> {
            self.slots
                .iter()
                .map(|slot| Report {
                    mod_id: slot.mod_id.clone(),
                    fuel_used: slot.fuel_used,
                    fuel_peak: slot.fuel_peak,
                    fuel_budget: slot.module.limits().fuel,
                    call_us: slot.call_us,
                    faulted: slot.faulted.clone(),
                })
                .collect()
        }
    }

    pub(super) fn tick(sim: &mut RaceSim) {
        if sim.extensions.on.slots.is_empty() {
            return;
        }
        let tick = sim.world.tick;
        let active = sim
            .world
            .ships
            .iter()
            .enumerate()
            .fold(0_u32, |mask, (slot, ship)| {
                mask | u32::from(ship.active) << slot
            });
        let on = &mut sim.extensions.on;
        let events: Vec<Event> = (0..oag_gameplay::MAX_SHIPS)
            .filter(|slot| on.walls & (1 << slot) != 0)
            .map(|slot| Event::WallContact { slot: slot as u8 })
            .take(MAX_EVENTS)
            .collect();
        on.walls = 0;
        let observation = Observation {
            tick,
            active,
            events: &events,
        };
        let mut commands = Vec::new();
        for slot in &mut on.slots {
            if slot.faulted.is_some() {
                slot.fuel_used = 0;
                continue;
            }
            // Timed for the overlay only; nothing simulated reads it.
            let started = web_time::Instant::now();
            let outcome = slot
                .module
                .tick(&observation, &mut slot.blob, &mut commands);
            slot.call_us = started.elapsed().as_secs_f32() * 1.0e6;
            let cost = match outcome {
                Ok(cost) => {
                    for command in &commands {
                        match *command {
                            Command::ScaleThrust { slot, factor } => {
                                sim.world.ships[slot as usize].pending_thrust_scale *= factor;
                            }
                        }
                    }
                    cost
                }
                Err((fault, cost)) => {
                    log::warn!(
                        "mod {}: {fault} at tick {tick}; off for the rest of the race",
                        slot.mod_id
                    );
                    slot.faulted = Some((tick, fault.to_string()));
                    cost
                }
            };
            slot.fuel_used = cost.fuel_used;
            slot.fuel_peak = slot.fuel_peak.max(cost.fuel_used);
        }
    }
}
