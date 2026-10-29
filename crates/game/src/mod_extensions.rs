//! Which simulation modules a race runs: the rules modules of the mods that are on
//! (`oag_mod::Extension`), for the title, never in an original campaign
//! (`docs/architecture/wasm-extensions/README.md`, section 1.2).
//!
//! Only the manifests are read here; each `.wasm` is read by the race load
//! (`oag_raceplay::extensions::prepare`, on the load thread), so a menu, the MODS page
//! and a vanilla race open no module and build no wasm engine (README 4.5).

use std::sync::Arc;

use oag_raceplay::extensions::Source;

/// The rules modules of `mods`, in id order. The race load keeps the ones for its title.
#[must_use]
pub fn sources<'a>(mods: impl IntoIterator<Item = &'a oag_mod::Mod>) -> Vec<Source> {
    let mut found: Vec<Source> = mods
        .into_iter()
        .filter_map(|m| {
            let extension = m.manifest.extension.as_ref()?;
            let rules = extension.rules.as_ref()?;
            Some(Source {
                mod_id: m.manifest.id.clone(),
                module: rules.module.clone(),
                titles: m.manifest.titles.clone(),
                api: extension.api,
                memory_pages: extension.memory_pages,
                state_bytes: extension.state_bytes,
                read: reader(m, &rules.module),
            })
        })
        .collect();
    found.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));
    found
}

/// Reads `module` later: from disk for a folder mod, else by reopening the archive.
fn reader(module: &oag_mod::Mod, name: &str) -> oag_raceplay::extensions::Reader {
    let label = format!("{}: {name}", module.label());
    if let Some(path) = module.local_path(name) {
        return Arc::new(move || {
            std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
        });
    }
    let archive = std::path::PathBuf::from(module.label());
    let name = name.to_string();
    Arc::new(move || {
        oag_mod::Mod::open(&archive)
            .map_err(|e| format!("{label}: {e}"))?
            .read(&name)
            .map_err(|e| format!("{label}: {e}"))?
            .ok_or_else(|| format!("{label}: not in the mod"))
    })
}

/// Sets `options.find_extensions` for a race: the load thread searches the mods that are
/// on (switched off by `disabled`) for rules modules, or none for a race that stays
/// `vanilla` (an original campaign, a network race). Nothing is read here.
pub fn attach(
    options: &mut oag_raceplay::Options,
    disabled: &std::collections::BTreeSet<String>,
    vanilla: bool,
) {
    options.find_extensions = (!vanilla).then(|| {
        let disabled = disabled.clone();
        oag_raceplay::extensions::Finder(Arc::new(move || {
            crate::mods::Mods::load(&disabled)
                .map(|mods| sources(mods.player()))
                .map_err(|e| format!("{e:#}"))
        }))
    });
}

/// The dev overlay's `mods` block for `race`, `None` in a race with no simulation mod.
#[must_use]
pub fn readout(race: &oag_raceplay::Race) -> Option<oag_present::perf::mods::Readout> {
    race.is_modded().then(|| oag_present::perf::mods::Readout {
        modules: race
            .extension_reports()
            .into_iter()
            .map(|r| oag_present::perf::mods::Module {
                id: r.mod_id,
                fuel_used: r.fuel_used,
                fuel_peak: r.fuel_peak,
                fuel_budget: r.fuel_budget,
                call_us: r.call_us,
                faulted: r.faulted,
            })
            .collect(),
    })
}
