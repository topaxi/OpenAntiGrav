//! Which cache files belong to which source.
//!
//! Both caches are keyed by content and share one directory across every disc,
//! so the directory alone cannot say what a given image needs. A prefetch knows:
//! it walked the source. This is what it leaves behind, one text file per source
//! under `<cache root>/manifests/`, so `scripts/push-game-data.sh` can copy
//! exactly one image's movies and sounds to a phone or a Deck that has no
//! `ffmpeg` to make them.
//!
//! The format is one cache-root-relative path per line (`movies/<file>`,
//! `audio/<file>`), sorted, after a comment line naming the source. Only files
//! that exist are listed, so a manifest never promises what a failed conversion
//! did not write.

use std::path::{Path, PathBuf};

/// What the walk found a source to own, whether or not it was already cached.
#[derive(Debug, Default)]
pub(super) struct Owned {
    /// Movie cache keys (`<hash>-<size>`); a cache file starts with its key and a dash.
    pub movie_keys: Vec<String>,
    /// Where each ATRAC3+ stream's PCM lives in the audio cache.
    pub sound_files: Vec<PathBuf>,
}

/// The manifest's own path: `<cache root>/manifests/<source file name>.txt`.
pub(super) fn path(movies: &Path, source: &str) -> Option<PathBuf> {
    let image = source.split(':').next().unwrap_or(source);
    let name = Path::new(image).file_name()?.to_string_lossy().into_owned();
    Some(
        movies
            .parent()?
            .join("manifests")
            .join(format!("{name}.txt")),
    )
}

/// The lines a manifest holds, for the files that exist now.
pub(super) fn lines(movies: &Path, owned: &Owned) -> Vec<String> {
    let mut lines = Vec::new();
    if let Ok(entries) = std::fs::read_dir(movies) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let owns = owned.movie_keys.iter().any(|key| {
                name.strip_prefix(key.as_str())
                    .is_some_and(|rest| rest.starts_with('-'))
            });
            if name.ends_with(".ivf") && owns {
                lines.push(format!("movies/{name}"));
            }
        }
    }
    for file in &owned.sound_files {
        if let Some(name) = file.file_name()
            && file.is_file()
        {
            lines.push(format!("audio/{}", name.to_string_lossy()));
        }
    }
    lines.sort();
    lines.dedup();
    lines
}

/// Writes the manifest, replacing any earlier one for the same source.
pub(super) fn write(
    movies: &Path,
    source: &str,
    owned: &Owned,
) -> std::io::Result<Option<PathBuf>> {
    let Some(path) = path(movies, source) else {
        return Ok(None);
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut text = format!("# oag cache manifest: {source}\n");
    for line in lines(movies, owned) {
        text.push_str(&line);
        text.push('\n');
    }
    std::fs::write(&path, text)?;
    Ok(Some(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_lists_the_files_this_source_owns_and_no_others() {
        let root = std::env::temp_dir().join(format!("oag-manifest-{}", std::process::id()));
        let (movies, audio) = (root.join("movies"), root.join("audio"));
        std::fs::create_dir_all(&movies).unwrap();
        std::fs::create_dir_all(&audio).unwrap();
        for name in [
            "aaaa0001-100-480x272-av1-1200.ivf",
            "aaaa0001-1000-480x272-av1-5.ivf",
            "bbbb0002-100-480x272-av1-9.ivf",
        ] {
            std::fs::write(movies.join(name), b"x").unwrap();
        }
        std::fs::write(audio.join("k1-2ch-44100hz.s16le"), b"x").unwrap();
        let owned = Owned {
            movie_keys: vec!["aaaa0001-100".to_string()],
            sound_files: vec![audio.join("k1-2ch-44100hz.s16le"), audio.join("gone.s16le")],
        };
        // `aaaa0001-1000` shares the prefix text but is a different key, and
        // `gone.s16le` was planned but never written.
        assert_eq!(
            lines(&movies, &owned),
            [
                "audio/k1-2ch-44100hz.s16le",
                "movies/aaaa0001-100-480x272-av1-1200.ivf"
            ]
        );
        let written = write(&movies, "data/images/pulse-psp-eu.chd:PSP_GAME/x", &owned)
            .unwrap()
            .unwrap();
        assert!(written.ends_with("manifests/pulse-psp-eu.chd.txt"));
        let text = std::fs::read_to_string(&written).unwrap();
        assert!(text.contains("movies/aaaa0001-100-480x272-av1-1200.ivf\n"));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
