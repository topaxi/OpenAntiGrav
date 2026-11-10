//! A PPSSPP texture pack as a mod.
//!
//! A pack is a folder or a `.zip` with `textures.ini` at its root, and it is not
//! edited to be a mod: **a container with that file and no `oag-mod.toml` is
//! recognised and given a manifest here**. The pack stays as its author made it, so
//! a PPSSPP memstick folder shared with the emulator gains nothing, and a 553 MB
//! `textures.zip` is read in place. An `oag-mod.toml` with a `[texture-pack]` table
//! wins over recognition: a pack's author writes one for a name, a release and a
//! licence, and [`write_manifest`] writes one for a checkout `just fetch-texture-packs`
//! makes, from the same [`recognise`] so the two cannot drift.
//!
//! Hashing, the ini and the PNGs are `oag-texture`'s and the race load's; this crate
//! finds the pack and says what it is (`docs/formats/ppsspp-texture-pack.md`, "A pack is
//! a mod to the game").

use std::path::{Path, PathBuf};

use oag_texture::replacement_pack::Pack;
use oag_texture::replacer::{Files, Replacements};
use serde::Serialize;

use crate::{
    Container, Error, License, MANIFEST, Manifest, Mod, SCHEMA_KIND, SCHEMA_MAJOR, TexturePack,
};

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
        shaders: std::collections::BTreeMap::new(),
        sounds: std::collections::BTreeMap::new(),
        pickup: None,
        pickup_titles: Vec::new(),
        pickup_odds: Vec::new(),
        channels: crate::Channels::default(),
        texture_pack: Some(TexturePack {
            ini: INI.to_string(),
            serials: serials(container, INI),
        }),
        default_enabled: false,
    })
}

/// What [`write_manifest`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Written {
    /// A new `oag-mod.toml` at this path.
    Wrote(PathBuf),
    /// An `oag-mod.toml` was already there and is untouched, whoever wrote it.
    Kept(PathBuf),
}

/// The author a written manifest names when the repository's owner cannot be read from its
/// URL: the schema wants one.
pub const UNKNOWN_AUTHOR: &str = "unknown";

#[derive(Serialize)]
struct File<'a> {
    #[serde(rename = "toml-schema")]
    header: oag_schema::Header,
    #[serde(rename = "mod")]
    module: ModTable<'a>,
    #[serde(rename = "texture-pack")]
    texture_pack: PackTable,
}

#[derive(Serialize)]
struct ModTable<'a> {
    id: &'a str,
    name: &'a str,
    version: String,
    authors: Vec<String>,
    description: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    repository: Option<&'a str>,
    keywords: &'a [String],
    #[serde(rename = "default-enabled")]
    default_enabled: bool,
}

#[derive(Serialize)]
struct PackTable {
    format: &'static str,
}

/// `<owner>` of `https://host/<owner>/<repo>[.git]`; [`UNKNOWN_AUTHOR`] for anything else.
fn owner(repository: Option<&str>) -> String {
    repository
        .and_then(|url| url.split_once("://"))
        .and_then(|(_, rest)| rest.split('/').nth(1))
        .filter(|owner| !owner.trim().is_empty())
        .map_or_else(|| UNKNOWN_AUTHOR.to_string(), str::to_string)
}

/// `manifest` (what [`recognise`] made) as the text of an `oag-mod.toml`. The
/// `[texture-pack]` table names no `serials`: the ini is read on every load, so they follow
/// the pack when it moves to another commit. The author is the repository's owner, and no
/// licence is named, as recognition has none.
///
/// # Errors
///
/// The manifest is not one [`recognise`] makes: it has no `[texture-pack]`.
pub fn manifest_text(manifest: &Manifest, repository: Option<&str>) -> Result<String, String> {
    if manifest.texture_pack.is_none() {
        return Err(format!("{}: not a texture pack", manifest.id));
    }
    let file = File {
        header: oag_schema::Header {
            location: oag_schema::location(SCHEMA_KIND, SCHEMA_MAJOR),
            version: oag_schema::LANGUAGE_VERSION.to_string(),
        },
        module: ModTable {
            id: &manifest.id,
            name: &manifest.name,
            version: manifest.version.to_string(),
            authors: vec![owner(repository)],
            description: &manifest.description,
            repository,
            keywords: &manifest.keywords,
            default_enabled: manifest.default_enabled,
        },
        texture_pack: PackTable { format: "ppsspp" },
    };
    toml::to_string(&file).map_err(|e| e.to_string())
}

/// Writes `folder`'s `oag-mod.toml` from what [`recognise`] makes of it, unless one is
/// there: **an existing manifest is never touched**, so a hand edit survives every re-run.
/// Nothing the file says goes stale (id and name follow the folder, serials the ini), which
/// is why keeping it is enough. A pack read from a `textures.zip` is not written for: a
/// folder manifest would shadow the zip PPSSPP prefers.
///
/// # Errors
///
/// `folder` holds no `textures.ini` at its root, or the file cannot be written.
pub fn write_manifest(folder: &Path, repository: Option<&str>) -> Result<Written, Error> {
    let path = folder.join(MANIFEST);
    if path.exists() {
        return Ok(Written::Kept(path));
    }
    let container = Container::open(folder)?;
    let not_written = |why: String| Error::Io {
        origin: container.label(),
        source: std::io::Error::other(why),
    };
    let manifest =
        recognise(&container).ok_or_else(|| not_written(format!("no {INI} at its root")))?;
    let text = manifest_text(&manifest, repository).map_err(not_written)?;
    std::fs::write(&path, text).map_err(|source| Error::Io {
        origin: path.display().to_string(),
        source,
    })?;
    Ok(Written::Wrote(path))
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
    if own.eq_ignore_ascii_case("textures")
        && let Some(parent) = path.parent().map(stem).filter(|n| !n.is_empty())
    {
        return parent;
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
