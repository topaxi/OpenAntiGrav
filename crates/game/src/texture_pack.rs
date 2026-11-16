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

/// The [`oag_raceplay::texture_pack::Finder`] for the packs `switches` leave on and whose
/// `titles` admit the title being loaded: a race's load and the front end's boot
/// (`crate::boot::Options::find_texture_packs`) open them the same way. A pack for another
/// title, or any pack on a source that is not a PSP's, never reaches the load, so it is not
/// reported as unusable.
#[must_use]
pub fn finder(switches: &crate::settings::Mods) -> oag_raceplay::texture_pack::Finder {
    let switches = switches.clone();
    oag_raceplay::texture_pack::Finder(Arc::new(
        move |title, platform| match crate::mods::Mods::load(&switches) {
            Ok(mods) => packs_for(&mods, title, platform),
            Err(why) => vec![Err(format!("{why:#}"))],
        },
    ))
}

/// The pack each of `mods` that is on and made for `title` on `platform` is, other mods and
/// packs for other titles or platforms left out. Each is opened again from its path, so the
/// load thread owns its readers.
#[must_use]
pub fn packs_for(
    mods: &crate::mods::Mods,
    title: &oag_title::Title,
    platform: oag_assets::Platform,
) -> oag_raceplay::texture_pack::Packs {
    for module in mods
        .player_for(title)
        .filter(|m| m.manifest.texture_pack.is_some() && !crate::mods::is_for_platform(m, platform))
    {
        log::debug!(
            "texture pack {}: a PPSSPP pack is the PSP's, not used on {platform:?}",
            module.label()
        );
    }
    mods.player_on(title, platform)
        .filter(|module| module.manifest.texture_pack.is_some())
        .map(|module| {
            let label = module.label();
            oag_mod::Mod::open(Path::new(&label))
                .map_err(|why| format!("{label}: {why}"))
                .and_then(oag_mod::texture_pack::replacements)
                .map(Arc::new)
        })
        .collect()
}
