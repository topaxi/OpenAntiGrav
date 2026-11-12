//! The texture pack a race or the front end draws with, chosen once by the load.
//!
//! The composition root hands the load a [`Finder`] for the packs the player has on; the
//! load, which is the one place that knows the disc's serial, makes the first pack that
//! names it the active one (`oag_texture::replacer::install`) before any texture is
//! decoded, so the decoders ask it as they go. Rendering only: no pack reaches `World`,
//! a snapshot or a state hash (`docs/formats/ppsspp-texture-pack.md`).

use std::sync::Arc;

use oag_assets::{Archives, Platform};
use oag_texture::replacer::{self, Replacements};
use oag_title::Title;

/// What a [`Finder`] returns: each pack that is on, or why it would not open.
pub type Packs = Vec<Result<Arc<Replacements>, String>>;

/// Opens the packs that are on and made for `title` on `platform`, on the load thread: the pack
/// list reads each pack's `textures.ini`, which a front end has no reason to do before a race is
/// asked for. A PPSSPP pack is the PSP's, so on any other platform the answer is empty.
#[derive(Clone)]
pub struct Finder(pub Arc<dyn Fn(&Title, Platform) -> Packs + Send + Sync>);

impl std::fmt::Debug for Finder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Finder")
    }
}

/// Installs the pack for the disc `archives` opened, or none, and says which in `report`.
///
/// A PPSSPP pack is keyed by the PSP's guest memory, so it applies to a PSP disc whose
/// serial the pack's `[games]` names; another platform is the [`Finder`]'s to leave out, and
/// another serial draws the disc's own textures. Asked for and not usable is an absence in
/// the report.
pub fn engage(
    finder: Option<&Finder>,
    title: &Title,
    archives: &Archives,
    report: &mut Vec<String>,
) -> Engaged {
    replacer::install(None);
    let Some(finder) = finder else {
        return Engaged;
    };
    let mut packs = Vec::new();
    let layout = &archives.layout;
    for pack in (finder.0)(title, layout.platform) {
        match pack {
            Ok(pack) => packs.push(pack),
            Err(why) => report.push(format!("texture pack not used: {why}")),
        }
    }
    let serial = layout.serial.as_deref();
    let mut chosen = None;
    for pack in packs {
        let why = match serial {
            None => Some("this source has no readable disc serial to match".to_string()),
            Some(serial) if !pack.serves(serial) => {
                Some(format!("its [games] does not name {serial}"))
            }
            _ if chosen.is_some() => Some("another pack is already in use".to_string()),
            _ => None,
        };
        match why {
            Some(why) => report.push(format!("texture pack not used: {}: {why}", pack.name())),
            None => {
                report.push(format!(
                    "texture pack {}: {} key(s), pictures read as textures are decoded",
                    pack.name(),
                    pack.keys()
                ));
                chosen = Some(pack);
            }
        }
    }
    replacer::install(chosen);
    Engaged
}

/// The load's use of the pack, held until it returns. Dropping it says how many textures the
/// pack replaced (`debug`) and switches it off, so a decode after the load (a menu's track
/// preview) draws the disc's own textures and the next race chooses afresh.
#[derive(Debug)]
#[must_use = "the pack is switched off when this is dropped"]
pub struct Engaged;

impl Drop for Engaged {
    fn drop(&mut self) {
        if let Some(pack) = replacer::active() {
            log::debug!(
                "texture pack {}: {} texture(s) replaced",
                pack.name(),
                pack.replaced()
            );
        }
        replacer::install(None);
    }
}
