//! A PPSSPP texture pack as a mod.
//!
//! A pack is a folder or a `.zip` with `textures.ini` at its root, and it is not
//! edited to be a mod: **a container with that file and no `oag-mod.toml` is
//! recognised and given a manifest here**. The pack stays as its author made it, so
//! a PPSSPP memstick folder shared with the emulator gains nothing, and a 553 MB
//! `textures.zip` is read in place. A pack whose author wants a name, a release and a
//! licence writes an `oag-mod.toml` with a `[texture-pack]` table, which wins.
//!
//! Hashing, the ini and the PNGs are `oag-texture`'s and the race load's; this crate
//! finds the pack and says what it is (`docs/formats/ppsspp-texture-pack.md`, "A pack is
//! a mod to the game").

use std::path::Path;

use oag_texture::replacement_pack::Pack;
use oag_texture::replacer::{Files, Replacements};

use crate::{Container, License, Manifest, Mod, TexturePack};

/// A pack's ini, at the pack's root.
pub const INI: &str = "textures.ini";

/// The zip a pack ships its PNGs in, beside (or instead of) a loose tree.
pub const ZIP: &str = "textures.zip";

/// The disc serials `ini` names under `[games]`, as written; empty when it is absent or
/// has none.
#[must_use]
pub fn serials(container: &Container, ini: &str) -> Vec<String> {
    let Ok(Some(bytes)) = container.read(ini) else {
        return Vec::new();
    };
    Pack::parse(&String::from_utf8_lossy(&bytes))
        .games
        .into_iter()
        .map(|(serial, _)| serial)
        .collect()
}

/// The manifest of a container that holds a PPSSPP pack and no `oag-mod.toml`; `None`
/// when it holds no `textures.ini`.
#[must_use]
pub fn recognise(container: &Container) -> Option<Manifest> {
    if !container.contains(INI) {
        return None;
    }
    let name = display_name(Path::new(&container.label()));
    Some(Manifest {
        id: format!("ppsspp.{}", slug(&name)),
        name,
        version: semver::Version::new(0, 0, 0),
        authors: Vec::new(),
        license: License::None,
        description: "A PPSSPP texture pack, drawn in place of the disc's textures.".to_string(),
        repository: None,
        homepage: None,
        keywords: vec!["texture-pack".to_string()],
        titles: Vec::new(),
        oag: None,
        extension: None,
        icons: std::collections::BTreeMap::new(),
        effects: std::collections::BTreeMap::new(),
        pickup: None,
        channels: crate::Channels::default(),
        texture_pack: Some(TexturePack {
            ini: INI.to_string(),
            serials: serials(container, INI),
        }),
        default_enabled: false,
    })
}

impl Files for Mod {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        Mod::read(self, name).ok().flatten()
    }
}

/// The pack `module` is, indexed over the mod's own files: its `textures.ini` read once, its
/// PNGs read as a texture asks for them.
///
/// # Errors
///
/// The mod is not a texture pack, its ini will not read, or the pack hashes in a way this
/// build does not compute (`oag_texture::replacer::Refused`).
pub fn replacements(module: Mod) -> Result<Replacements, String> {
    let label = module.label();
    let ini = module
        .manifest
        .texture_pack
        .as_ref()
        .map(|pack| pack.ini.clone())
        .ok_or_else(|| format!("{label}: not a texture pack"))?;
    let text = module
        .read_text(&ini)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("{label}: no {ini}"))?;
    let name = module.manifest.name.clone();
    Replacements::new(&name, &text, Box::new(module)).map_err(|why| format!("{label}: {why}"))
}

/// What a pack is called: its folder, or its zip's stem; a bare `textures.zip` takes the
/// folder it lies in, which is the name the pack's author gave it.
fn display_name(path: &Path) -> String {
    let stem = |p: &Path| {
        p.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let own = stem(path);
    if own.eq_ignore_ascii_case("textures") {
        if let Some(parent) = path.parent().map(stem).filter(|n| !n.is_empty()) {
            return parent;
        }
    }
    own
}

/// `name` as the tail of a mod id: lower case, letters digits '.', '-' and '_', no
/// leading or trailing separator.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars().flat_map(char::to_lowercase) {
        let keep = c.is_ascii_alphanumeric() || c == '.' || c == '_';
        if keep {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches(|c| c == '-' || c == '.');
    if trimmed.is_empty() {
        "pack".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests;
