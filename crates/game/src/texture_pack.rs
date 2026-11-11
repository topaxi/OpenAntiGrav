//! The texture packs a race draws with: the mods that are packs and on, opened for the load.

use std::path::Path;
use std::sync::Arc;

/// Sets `options.find_texture_packs` for a race: the load thread opens the packs that
/// `switches` leave on and the load picks the one that names its disc
/// (`oag_raceplay::texture_pack`). Nothing is read here; a pack's ini is read on the load
/// thread and its pictures as textures are decoded.
pub fn attach(options: &mut oag_raceplay::Options, switches: &crate::settings::Mods) {
    options.find_texture_packs = Some(finder(switches));
}

/// The [`oag_raceplay::texture_pack::Finder`] for the packs `switches` leave on: a race's load
/// and the front end's boot (`crate::boot::Options::find_texture_packs`) open them the same way.
#[must_use]
pub fn finder(switches: &crate::settings::Mods) -> oag_raceplay::texture_pack::Finder {
    let switches = switches.clone();
    oag_raceplay::texture_pack::Finder(Arc::new(move || {
        match crate::mods::Mods::load(&switches) {
            Ok(mods) => open(mods.player()),
            Err(why) => vec![Err(format!("{why:#}"))],
        }
    }))
}

/// The pack each of `mods` is, those that are not packs left out. Each is opened again from
/// its path, so the load thread owns its readers.
fn open<'a>(
    mods: impl Iterator<Item = &'a oag_mod::Mod>,
) -> Vec<Result<Arc<oag_texture::replacer::Replacements>, String>> {
    mods.filter(|module| module.manifest.texture_pack.is_some())
        .map(|module| {
            let label = module.label();
            oag_mod::Mod::open(Path::new(&label))
                .map_err(|why| format!("{label}: {why}"))
                .and_then(oag_mod::texture_pack::replacements)
                .map(Arc::new)
        })
        .collect()
}
