//! The one place a texture's name is respelled for the platform that ships it.
//!
//! Wipeout HD authors its front end, HUD and weapon art as `.gtf`, 2048 ships
//! the same layouts as `.gxt` and Omega as `.gnf`; Pulse's PS2 build keeps
//! `.mip` and `.tga` as `.pct`. The XML and tables that name a texture were
//! written for one of them, so every source reads a name *as written* first and
//! then in its own platform's spelling. This module is that second step; every
//! reader of a texture by name goes through [`Archives::read_texture`] or
//! [`Archives::resolve_texture`] rather than carrying a retry of its own.
//!
//! The spellings, in the order they are tried ([`spellings`]):
//!
//! 1. the name as written;
//! 2. the stem with the platform's extension (`.gtf` on the PS3, `.gxt` on the
//!    Vita, `.gnf` on the PS4; the PSP keeps what it is given);
//! 3. on the PS4, the same two in `texturesps4`, the directory Omega's
//!    circuits keep a texture in beside the `textures` the model names
//!    (`docs/rendering/start-gantry.md`, "2048 and Omega");
//! 4. the PS2's `.pct` ([`oag_formats::wad::ps2_texture_name`]).
//!
//! A name matcher never lists these spellings: it compares [`stem`]s.

use crate::Platform;
use crate::source::Archives;

/// The extension `platform` ships textures with, when it is not the one the
/// title's layout was authored with.
#[must_use]
pub const fn platform_extension(platform: Platform) -> Option<&'static str> {
    match platform {
        Platform::Ps3 => Some("gtf"),
        Platform::Vita => Some("gxt"),
        Platform::Ps4 => Some("gnf"),
        _ => None,
    }
}

/// The directory name Omega keeps its own texture copies in.
const PS4_TEXTURE_DIRECTORY: &str = "texturesps4";

/// `name` without its file extension, or `name` itself when the last path
/// component has none: a dot in a directory name is not an extension.
#[must_use]
pub fn without_extension(name: &str) -> &str {
    match name.rfind('.') {
        Some(at) if !name[at..].contains(['/', '\\']) => &name[..at],
        _ => name,
    }
}

/// The file stem of a texture name: its last path component without the
/// extension. What a name matcher compares, case-insensitively, so one
/// comparison answers for every platform's spelling of the same texture.
#[must_use]
pub fn stem(name: &str) -> &str {
    let leaf = name.rsplit(['/', '\\']).next().unwrap_or(name);
    without_extension(leaf)
}

/// Whether the file stem of `name` is `wanted`, ignoring case.
#[must_use]
pub fn has_stem(name: &str, wanted: &str) -> bool {
    stem(name).eq_ignore_ascii_case(wanted)
}

/// `name` with its first `textures` path component renamed [`PS4_TEXTURE_DIRECTORY`].
fn ps4_directory(name: &str) -> Option<String> {
    let mut at = 0;
    for component in name.split_inclusive(['/', '\\']) {
        let bare = component.trim_end_matches(['/', '\\']);
        let is_directory = bare.len() != component.len();
        if is_directory && bare.eq_ignore_ascii_case("textures") {
            let separator = &component[bare.len()..];
            return Some(format!(
                "{}{PS4_TEXTURE_DIRECTORY}{separator}{}",
                &name[..at],
                &name[at + component.len()..]
            ));
        }
        at += component.len();
    }
    None
}

/// Every spelling of the texture `name` this `platform` may ship it under, the
/// name as written first, without repeats.
#[must_use]
pub fn spellings(platform: Platform, name: &str) -> Vec<String> {
    let mut out = vec![name.to_string()];
    let mut push = |candidate: String| {
        if !out.contains(&candidate) {
            out.push(candidate);
        }
    };
    let own = platform_extension(platform).map(|extension| {
        let stem = without_extension(name);
        format!("{stem}.{extension}")
    });
    if let Some(own) = &own {
        push(own.clone());
    }
    if platform == Platform::Ps4 {
        if let Some(moved) = ps4_directory(name) {
            push(moved);
        }
        if let Some(moved) = own.as_deref().and_then(ps4_directory) {
            push(moved);
        }
    }
    if let Some(ps2) = oag_formats::wad::ps2_texture_name(name) {
        push(ps2);
    }
    out
}

impl Archives {
    /// The spelling of the texture `name` this source holds, `name` itself
    /// when it is held as written. `None` (and a debug line naming every
    /// spelling tried) when none is.
    ///
    /// The search is [`Self::locate`]'s, so a player's override and this
    /// project's fallback layer answer for a spelling the same way the
    /// archives do.
    #[must_use]
    pub fn resolve_texture(&self, name: &str) -> Option<String> {
        let tried = spellings(self.layout.platform, name);
        if let Some(found) = tried
            .iter()
            .find(|candidate| self.locate(candidate).is_some())
        {
            return Some(found.clone());
        }
        log::debug!(
            "texture {name}: no spelling is held on {} (tried {})",
            self.layout.platform,
            tried.join(", ")
        );
        None
    }

    /// Reads the texture `name`, as written and then in this platform's
    /// spelling, and returns the name that was held beside its bytes.
    ///
    /// The resolved name, not the one asked for, is what tells a decoder the
    /// container (`.gtf`, `.gxt`, `.gnf`); a caller that keys its own tables by
    /// the declared name keeps `name`.
    ///
    /// # Errors
    ///
    /// The error [`Self::read_name`] gives for `name` as written, never for a
    /// respelling: a message about `hdhud.gtf` sends a reader looking for a
    /// name their XML does not contain.
    pub fn read_texture(&mut self, name: &str) -> crate::Result<(String, Vec<u8>)> {
        let Some(held) = self.resolve_texture(name) else {
            return self.read_name(name).map(|blob| (name.to_string(), blob));
        };
        match self.read_name(&held) {
            Ok(blob) => Ok((held, blob)),
            Err(error) if held == name => Err(error),
            Err(_) => self.read_name(name).map(|blob| (name.to_string(), blob)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_as_written_is_always_first() {
        for platform in [Platform::Psp, Platform::Ps3, Platform::Vita, Platform::Ps4] {
            assert_eq!(
                spellings(platform, r"Data\Weapons\Textures\Cannon_bolt.mip")[0],
                r"Data\Weapons\Textures\Cannon_bolt.mip"
            );
        }
    }

    #[test]
    fn each_platform_adds_its_own_extension_and_the_psp_adds_none() {
        let name = r"Data\HUD\atlas.tga";
        assert_eq!(
            spellings(Platform::Psp, name),
            [name, r"Data\HUD\atlas.pct"]
        );
        assert!(spellings(Platform::Ps3, name).contains(&r"Data\HUD\atlas.gtf".to_string()));
        assert!(spellings(Platform::Vita, name).contains(&r"Data\HUD\atlas.gxt".to_string()));
        assert!(spellings(Platform::Ps4, name).contains(&r"Data\HUD\atlas.gnf".to_string()));
    }

    #[test]
    fn a_dot_in_a_directory_is_not_an_extension() {
        assert_eq!(without_extension(r"Data\v1.2\atlas"), r"Data\v1.2\atlas");
        assert_eq!(
            spellings(Platform::Vita, r"Data\v1.2\atlas"),
            [r"Data\v1.2\atlas", r"Data\v1.2\atlas.gxt"]
        );
    }

    #[test]
    fn omega_also_looks_in_its_own_texture_directory() {
        let spelled = spellings(
            Platform::Ps4,
            "data/environments2048/altima/textures/billboard8.gtf",
        );
        assert!(
            spelled
                .contains(&"data/environments2048/altima/texturesps4/billboard8.gnf".to_string()),
            "{spelled:?}"
        );
        assert!(
            !spellings(Platform::Vita, "a/textures/b.gtf")
                .iter()
                .any(|s| s.contains("texturesps4"))
        );
    }

    #[test]
    fn a_stem_is_the_same_on_every_spelling() {
        for name in [
            "billboard8.tga",
            "billboard8.gtf",
            r"a\textures\dds\BILLBOARD8.gxt",
            "x/billboard8.gnf",
        ] {
            assert!(has_stem(name, "billboard8"), "{name}");
        }
        assert!(!has_stem("billboard81.gtf", "billboard8"));
    }
}
