//! The pinned builds: one table, one row per OS and architecture.
//!
//! BtbN's FFmpeg-Builds, the `lgpl` variants (static, LGPL v3: configured with
//! `--enable-version3` and without `--enable-gpl`, `libx264` and `libx265`
//! off). The URLs name a month-end autobuild tag, which the project keeps; a
//! rolling `latest` would turn the hashes below into a mismatch within a day.
//! Each build carries `libaom` (the movie transcode's encoder) and the native
//! ATRAC3+ decoder (Pulse's and Pure's music). `docs/overview/installing.md`
//! repeats this table.

/// How a pinned archive is packed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Archive {
    /// `.tar.xz` (Linux).
    TarXz,
    /// `.zip` (Windows).
    Zip,
}

/// One downloadable build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pin {
    /// The release's version string, in the install directory's name.
    pub version: &'static str,
    /// Where the archive is.
    pub url: &'static str,
    /// The archive's SHA-256, lower-case hex.
    pub sha256: &'static str,
    /// The archive's size in bytes, shown to the player before they say yes.
    pub size: u64,
    /// How it is packed.
    pub archive: Archive,
    /// The license the build is distributed under.
    pub license: &'static str,
}

const VERSION: &str = "n8.1.3-27-g8dca1586f4";

impl Pin {
    /// Every pin in the table, for the docs check and the tests.
    pub const ALL: [Self; 4] = [
        Self {
            version: VERSION,
            url: concat!(
                "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-10-31-12-59/",
                "ffmpeg-n8.1.3-27-g8dca1586f4-linux64-lgpl-8.1.tar.xz"
            ),
            sha256: "a64f3171e6ad1f7d358d28890a907ee478c331d1177ec696a93418f79ac3f1f3",
            size: 135_188_284,
            archive: Archive::TarXz,
            license: "LGPL-3.0-or-later",
        },
        Self {
            version: VERSION,
            url: concat!(
                "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-10-31-12-59/",
                "ffmpeg-n8.1.3-27-g8dca1586f4-linuxarm64-lgpl-8.1.tar.xz"
            ),
            sha256: "2ea30086a637e3f0ad0f49a1b38db34121422b85e93cc82df2e01d9ee5200809",
            size: 115_803_692,
            archive: Archive::TarXz,
            license: "LGPL-3.0-or-later",
        },
        Self {
            version: VERSION,
            url: concat!(
                "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-10-31-12-59/",
                "ffmpeg-n8.1.3-27-g8dca1586f4-win64-lgpl-8.1.zip"
            ),
            sha256: "d4c2a42673917b4a97104a3e9f082729a9730796b98664e37c0b7eba6c6c613d",
            size: 167_778_744,
            archive: Archive::Zip,
            license: "LGPL-3.0-or-later",
        },
        Self {
            version: VERSION,
            url: concat!(
                "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2026-10-31-12-59/",
                "ffmpeg-n8.1.3-27-g8dca1586f4-winarm64-lgpl-8.1.zip"
            ),
            sha256: "c918142d3920f6b15282739dd523885be0586cbd8684f400acea9d4a8c97002f",
            size: 115_921_422,
            archive: Archive::Zip,
            license: "LGPL-3.0-or-later",
        },
    ];

    /// The pin for the OS and architecture this binary was built for. `None` on
    /// macOS (no trustworthy static LGPL build is published), on Android and on
    /// the web, where nothing is downloaded.
    #[must_use]
    pub const fn current() -> Option<&'static Self> {
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            Some(&Self::ALL[0])
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            Some(&Self::ALL[1])
        } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            Some(&Self::ALL[2])
        } else if cfg!(all(target_os = "windows", target_arch = "aarch64")) {
            Some(&Self::ALL[3])
        } else {
            None
        }
    }

    /// The directory name under the install root: the version, so a pin bump
    /// installs beside the old copy rather than over it.
    #[must_use]
    pub fn dir_name(&self) -> String {
        format!("ffmpeg-{}", self.version)
    }

    /// The archive's size in whole megabytes, for the prompt.
    #[must_use]
    pub const fn megabytes(&self) -> u64 {
        self.size / 1_000_000
    }
}
