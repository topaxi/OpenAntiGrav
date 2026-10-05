//! Classifying one package's archive entries against another's.
//!
//! The logic behind `oag-psarc-diff`: which family an archive path belongs to,
//! and whether an entry is **new** (no entry of that path on the base side),
//! **identical** (same path, same bytes) or **replaced** (same path, different
//! bytes - the cause is never read here, only that they differ).

use std::fmt;

/// What a patch or DLC entry is, against the base side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    /// No entry of this path on the base side.
    New,
    /// The path exists on the base side with different bytes.
    Replaced,
    /// The path exists on the base side with the same bytes.
    Identical,
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::New => "new",
            Self::Replaced => "replaced",
            Self::Identical => "identical",
        })
    }
}

/// The asset family a path belongs to, by extension first and directory second.
///
/// A reporting aid and nothing more: it is a heuristic over names, so a row's
/// family says where a human should look, not what the engine does with it.
#[must_use]
pub fn family(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    let ext = lower.rsplit_once('.').map_or("", |(_, e)| e);
    match ext {
        "bnk" | "xfx" | "at9" | "vag" | "wav" | "ogg" | "mp3" | "m4a" | "aac" | "mpc" | "sab"
        | "xwb" | "wem" | "bnks" => return "audio",
        "gxp" | "rcsmaterial" | "sho" | "cg" | "fp" | "vp" | "glsl" | "fx" => return "shaders",
        "mp4" | "bik" | "pmf" | "ipf" => return "video",
        "xml" | "xmlb" | "envsettings" | "effectsettings" | "txt" | "json" | "ini" => {
            return "xml tables";
        }
        _ => {}
    }
    if lower.contains("ship") || lower.contains("/craft") {
        "ships"
    } else if lower.contains("environment") || lower.contains("track") || lower.contains("circuit")
    {
        "tracks"
    } else if lower.contains("/fe/")
        || lower.contains("/ui/")
        || lower.contains("frontend")
        || lower.contains("newimages")
        || lower.contains("hud")
        || lower.contains("font")
        || lower.contains("menu")
        || ext == "fnt"
    {
        "ui"
    } else if matches!(ext, "gxt" | "gtf" | "gnf" | "dds" | "png" | "tga") {
        "textures (other)"
    } else if matches!(ext, "vex" | "rcsmodel" | "rcsskeleton" | "rcsanimclip") {
        "models (other)"
    } else if matches!(ext, "pob" | "pvs" | "pvsxml" | "probexml" | "shprobes") {
        "scenery data"
    } else {
        "other"
    }
}

/// Spells a path one way so the same file on either side compares equal.
#[must_use]
pub fn normalise(path: &str) -> String {
    path.trim_start_matches('/')
        .replace('\\', "/")
        .to_ascii_lowercase()
}

/// Names the class of an entry given what the base side holds for its path.
///
/// `base` is each base copy as `(size, md5-of-bytes-or-None-if-not-read)`;
/// a copy whose size differs is never hashed, so `None` only appears with a
/// size mismatch and is treated as different.
#[must_use]
pub fn classify(size: u64, md5: [u8; 16], base: &[(u64, Option<[u8; 16]>)]) -> Class {
    if base.is_empty() {
        Class::New
    } else if base.iter().any(|&(s, h)| s == size && h == Some(md5)) {
        Class::Identical
    } else {
        Class::Replaced
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families_follow_extension_then_directory() {
        assert_eq!(family("/data/audio/weapons.bnk"), "audio");
        assert_eq!(family("/data/xml/SP.xml"), "xml tables");
        assert_eq!(family("/data/hdships/auricom/ship.vex"), "ships");
        assert_eq!(family("/data/environments/x/track.vex"), "tracks");
        assert_eq!(family("/data/FE/logo.gxt"), "ui");
        assert_eq!(family("/data/tex/a.gxt"), "textures (other)");
        assert_eq!(family("/data/shaders/a.gxp"), "shaders");
    }

    #[test]
    fn classes_split_on_presence_then_bytes() {
        let a = [1u8; 16];
        let b = [2u8; 16];
        assert_eq!(classify(4, a, &[]), Class::New);
        assert_eq!(classify(4, a, &[(4, Some(a))]), Class::Identical);
        assert_eq!(classify(4, a, &[(4, Some(b))]), Class::Replaced);
        assert_eq!(classify(4, a, &[(5, None)]), Class::Replaced);
        assert_eq!(classify(4, a, &[(5, None), (4, Some(a))]), Class::Identical);
    }

    #[test]
    fn paths_normalise_across_spellings() {
        assert_eq!(normalise("/Data\\A/B.vex"), "data/a/b.vex");
    }
}
