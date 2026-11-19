//! `[ffmpeg]`: whether the game may fetch the `ffmpeg` it shells out to
//! (`docs/architecture/ffmpeg.md`).

use serde::{Deserialize, Serialize};

/// The player's standing answer to the download offer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Download {
    /// Not answered yet: the front end asks once when there is no `ffmpeg`.
    #[default]
    Ask,
    /// Yes: fetch it again without asking if it is gone (the cache was cleared).
    Yes,
    /// No: never ask. The OPTIONS page's DOWNLOAD FFMPEG row asks again.
    Never,
}

/// The `[ffmpeg]` table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ffmpeg {
    /// The standing answer to the download offer.
    #[serde(default)]
    pub download: Download,
    /// Use the pinned build even when a system `ffmpeg` is on `PATH`. Config file only: there
    /// is no menu row, and a system `ffmpeg` never prompts a download without it.
    #[serde(default)]
    pub use_pinned: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Ffmpeg {
        toml::from_str::<crate::settings::Settings>(text)
            .expect("parses")
            .ffmpeg
    }

    #[test]
    fn a_file_without_the_table_asks_and_uses_the_system_tool() {
        assert_eq!(read(""), Ffmpeg::default());
        assert_eq!(read("").download, Download::Ask);
        assert!(!read("").use_pinned);
    }

    #[test]
    fn the_three_answers_and_use_pinned_round_trip_through_the_file() {
        for (word, answer) in [
            ("ask", Download::Ask),
            ("yes", Download::Yes),
            ("never", Download::Never),
        ] {
            let ffmpeg = read(&format!("[ffmpeg]\ndownload = \"{word}\"\nuse_pinned = true\n"));
            assert_eq!(ffmpeg.download, answer);
            assert!(ffmpeg.use_pinned);
            let settings = crate::settings::Settings {
                ffmpeg,
                ..Default::default()
            };
            let text = toml::to_string(&settings).expect("serialises");
            assert_eq!(read(&text).download, answer, "{text}");
        }
    }
}
