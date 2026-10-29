//! What a scan of the track directories finds, one [`Listing`] per file, for the
//! menus to offer. See `oag_source::ballisticng` for where the files are looked for.
//!
//! A file that will not read is still listed, with the reason: a player who put
//! a track in the directory and does not see it learns nothing, and one who sees
//! it greyed with "not a track file" learns what to fix. Whether a listing is
//! raceable is decided by reading the track the way a race does ([`super::read_any`]),
//! so a menu can never offer a track the load would then refuse.

use std::path::{Path, PathBuf};

/// Which format a listed file is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A BallisticNG Workshop item (`.wtrk`), shown as a Workshop track.
    Workshop,
    /// This project's own format (`*.track.toml` and its `.glb`).
    Own,
}

impl Kind {
    /// The name a menu shows for it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Workshop => "Workshop",
            Self::Own => "Track file",
        }
    }
}

/// One file the scan found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// The file: what a race loads.
    pub path: PathBuf,
    /// The key a record or ghost is kept under (`foreign:<id>`, as
    /// [`super::entry`] makes it); the file name when the file would not read.
    pub id: String,
    /// The title the file carries; the file's stem when it carries none.
    pub title: String,
    /// The author, when the file names one. **Neither format does today**: a
    /// `.wtrk` carries the Workshop title, tags and description, and the author
    /// is a Steam account the file does not hold. Always `None` until one does.
    pub author: Option<String>,
    /// The title a track file names as its host (`oag_trackfile::Track::host`,
    /// as `oag_source::layers::slug` spells it), or `None` for a Workshop track
    /// or a file that would not read. A RACE REMIX of it takes this title's
    /// disc as the track source.
    pub host: Option<String>,
    /// Which format it is.
    pub kind: Kind,
    /// The Workshop tags, in file order (empty for the own format).
    pub tags: Vec<String>,
    /// Why a race could not load it, in a few words, or `None` when it can.
    pub problem: Option<String>,
}

/// Reads `path` as a menu lists it.
#[must_use]
pub fn listing(path: &Path) -> Listing {
    let kind = if super::is_track_file(path) {
        Kind::Own
    } else {
        Kind::Workshop
    };
    let stem = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = stem
        .strip_suffix(".track.toml")
        .or_else(|| stem.rsplit_once('.').map(|(s, _)| s))
        .unwrap_or(&stem)
        .to_string();
    let mut out = Listing {
        path: path.to_path_buf(),
        id: format!("foreign:{stem}"),
        title: stem,
        author: None,
        host: None,
        kind,
        tags: Vec::new(),
        problem: None,
    };
    #[cfg(feature = "ballisticng")]
    if kind == Kind::Workshop {
        match std::fs::read(path) {
            Ok(file) => match oag_ballisticng::wtrk::Wtrk::parse(&file) {
                Ok(wtrk) => {
                    out.id = format!("foreign:{}", wtrk.workshop_id);
                    if !wtrk.title.trim().is_empty() {
                        out.title = wtrk.title.trim().to_string();
                    }
                    out.tags = wtrk.tags;
                }
                Err(e) => {
                    log::warn!("{}: {e}", path.display());
                    // A file with the wrapper's own opening is a damaged track
                    // and says how; anything else was never one.
                    out.problem = Some(
                        if file
                            .get(1..)
                            .is_some_and(|f| f.starts_with(b"Workshop Track"))
                        {
                            short(&root_cause(&anyhow::anyhow!("{e}")), "damaged track file")
                        } else {
                            "not a track file".to_string()
                        },
                    );
                    return out;
                }
            },
            Err(e) => {
                out.problem = Some(format!("unreadable: {}", e.kind()));
                return out;
            }
        }
    }
    match super::read_any(path) {
        Ok((track, _)) => {
            out.id = format!("foreign:{}", track.id);
            if let Err(e) = super::host_title(&track) {
                out.problem = Some(short(&root_cause(&e), "names no known title"));
            }
            out.host.clone_from(&track.host);
            if kind == Kind::Own && !track.title.trim().is_empty() {
                out.title = track.title.trim().to_string();
            }
        }
        Err(e) => out.problem = Some(short(&root_cause(&e), "does not load")),
    }
    out
}

/// The innermost message of `error`, without the path every reader prefixes it with.
fn root_cause(error: &anyhow::Error) -> String {
    let text = error.root_cause().to_string();
    text.rsplit(": ").next().unwrap_or(&text).to_string()
}

/// `text` cut to a line a row's subtitle can hold, or `fallback` when empty.
fn short(text: &str, fallback: &str) -> String {
    let text = text.trim();
    if text.is_empty() {
        return fallback.to_string();
    }
    let mut cut: String = text.chars().take(44).collect();
    if cut.len() < text.len() {
        cut.push_str("...");
    }
    cut
}

/// A [`listing`] of every track in `paths`, in their order.
#[must_use]
pub fn survey(paths: &[PathBuf]) -> Vec<Listing> {
    paths.iter().map(|p| listing(p)).collect()
}
