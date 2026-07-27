//! Loading the boot sequence out of a disc image.
//!
//! Everything the front end needs, in the order the original needs it: the
//! front-end root XML, the language plugins, one language's string table, and
//! the intro movie. Kept apart from `main.rs` so the load can be exercised
//! without a window, and apart from [`crate::frontend`] so the sequence itself
//! stays free of file I/O.

use std::path::Path;

use anyhow::{Context, Result};
use oag_assets::pulse;
use oag_formats::fexml;

use crate::frontend::Frontend;
use crate::language::{Language, StringTable};
use crate::movie::{self, Extent, Movie};
use crate::screen::Screens;

/// A loaded boot sequence and the pieces the renderer needs alongside it.
pub struct Boot {
    /// The sequence itself.
    pub frontend: Frontend,
    /// The intro movie, whether or not it has a picture.
    pub movie: Movie,
    /// Every front-end image the screens reference, in one texture.
    pub sprites: crate::sprite::Sheet,
    /// The text atlas: the disc's own font when it decodes, ours when it does
    /// not.
    pub font: crate::font::Atlas,
    /// Lines worth printing once, describing what was found.
    pub report: Vec<String>,
}

impl std::fmt::Debug for Boot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Boot")
            .field("movie", &self.movie)
            .finish_non_exhaustive()
    }
}

/// What to load and how much of the movie to convert.
#[derive(Debug, Clone)]
pub struct Options {
    /// A disc image, or a directory extracted with `oag-unpack`.
    pub source: String,
    /// Which `Data.wad` entry the intro plays, as a name or a name hash.
    ///
    /// Three of the disc's movies have no recovered name, and one of them is
    /// the reel the original's intro state actually plays, so a name is not
    /// enough to address every candidate. See [`EntryRef`].
    pub movie: String,
    /// Where converted frames are cached.
    pub cache: std::path::PathBuf,
    /// How much of the movie to convert.
    pub extent: Extent,
    /// Skip conversion entirely.
    pub no_video: bool,
}

/// Loads everything and builds the sequence.
pub fn load(options: &Options) -> Result<Boot> {
    let mut report = Vec::new();
    let mut archives = pulse::Frontend::open(&options.source)
        .with_context(|| format!("opening the archives in {}", options.source))?;
    report.push(format!(
        "{}: {} entries",
        archives.data.label(),
        archives.data.directory().entries.len()
    ));

    let font = load_font(&mut archives.fe, &mut report);
    let screens = load_screens(&mut archives.data, &mut report)?;
    let languages = load_languages(&mut archives.data, &mut report);
    let strings = load_strings(&mut archives.data, &languages, &mut report);
    let movie = load_movie(&mut archives.data, options, &mut report)?;
    let sprites = load_sprites(&mut archives.fe, &mut archives.data, &screens, &mut report);

    let frames = movie.frames.as_ref().map_or(0, |f| f.len);
    let placements = screens
        .screens
        .iter()
        .flat_map(|s| s.images.iter())
        .filter_map(|image| sprites.get(&image.src).map(|p| (image.src.clone(), p)))
        .collect();
    let frontend = Frontend::new(
        screens,
        strings,
        languages,
        placements,
        frames,
        movie.frames.is_some(),
    );

    if let Some(goto) = frontend.language_auto_redirect() {
        report.push(format!(
            "the disc's Language Selection redirects to {goto}; this build goes to Launch Game, \
             which starts a race"
        ));
    }

    Ok(Boot {
        font,
        frontend,
        movie,
        sprites,
        report,
    })
}

/// Decodes every image the screens name.
///
/// The names come from the screens rather than from a list here, so a screen
/// that gains an `Image` gains its texture without this function changing.
///
/// **Both archives are searched, in that order, and they have to be.**
/// `pulse_logo.mip` is in `FE.wad` *and* `Data.wad` at the same size, which
/// makes `FE.wad` look sufficient; `gameshare_backdrop.mip` is in `Data.wad`
/// only, which proves it is not.
fn load_sprites(
    fe: &mut oag_assets::Archive,
    data: &mut oag_assets::Archive,
    screens: &Screens,
    report: &mut Vec<String>,
) -> crate::sprite::Sheet {
    let mut srcs: Vec<String> = Vec::new();
    for screen in &screens.screens {
        for image in &screen.images {
            if !srcs.contains(&image.src) {
                srcs.push(image.src.clone());
            }
        }
    }

    if srcs.is_empty() {
        return crate::sprite::Sheet::default();
    }

    let mut blobs: Vec<(String, Vec<u8>)> = Vec::new();
    for src in &srcs {
        match fe.read_name(src).or_else(|_| data.read_name(src)) {
            Ok(blob) => blobs.push((src.clone(), blob)),
            Err(e) => report.push(format!(
                "image {src}: in neither {} nor {}: {e}",
                fe.label(),
                data.label()
            )),
        }
    }

    let sheet = crate::sprite::Sheet::build(&blobs, report);
    report.push(format!(
        "{} of {} front-end image(s) decoded into a {}x{} sheet",
        sheet.len(),
        srcs.len(),
        sheet.width,
        sheet.height
    ));
    sheet
}

/// Reads the front end's own font, falling back to the built-in glyphs.
///
/// A missing or undecodable font is not fatal: the menu still draws, in the 5x7
/// approximation, and the report says which one is on screen. That matters more
/// than it sounds - the two look very different, and a silent fallback would
/// make a rendering bug indistinguishable from a loading one.
fn load_font(fe: &mut oag_assets::Archive, report: &mut Vec<String>) -> crate::font::Atlas {
    let name = oag_assets::pulse::names::DEFAULT_FONT;
    match fe
        .read_name(name)
        .map_err(|e| e.to_string())
        .and_then(|blob| oag_formats::fnt::Font::parse(&blob).map_err(|e| e.to_string()))
    {
        Ok(font) => {
            let atlas = crate::font::Atlas::from_font(&font);
            report.push(format!(
                "font {name}: {}x{} atlas, {} glyphs, line height {}",
                font.width,
                font.height,
                font.glyphs.len(),
                font.line_height
            ));
            atlas
        }
        Err(why) => {
            report.push(format!("font {name} unavailable ({why}); drawing with 5x7"));
            crate::font::Atlas::build()
        }
    }
}

fn load_screens(data: &mut oag_assets::Archive, report: &mut Vec<String>) -> Result<Screens> {
    let blob = data
        .read_name(pulse::names::FRONTEND_ROOT)
        .with_context(|| format!("reading {}", pulse::names::FRONTEND_ROOT))?;

    // Front-end XML is stored with its element and attribute names shortened
    // through a per-file dictionary. Files that begin `<?xml` are already plain.
    let xml = if fexml::is_fexml(&blob) {
        fexml::expand(&blob).map_err(|e| anyhow::anyhow!("expanding the front-end XML: {e}"))?
    } else {
        String::from_utf8(blob).context("the front-end XML is not text")?
    };

    let screens = Screens::from_xml(&xml);
    report.push(format!(
        "{}: {} screens, {} globals, {} LoadXML includes",
        pulse::names::FRONTEND_ROOT,
        screens.screens.len(),
        screens.globals.len(),
        screens.load_xml.len()
    ));
    for screen in screens.with_movies() {
        if let Some(movie) = &screen.movie {
            report.push(format!(
                "  screen {:?} plays {}",
                screen.name,
                movie.entry_name()
            ));
        }
    }
    Ok(screens)
}

fn load_languages(data: &mut oag_assets::Archive, report: &mut Vec<String>) -> Vec<Language> {
    let mut out = Vec::new();
    for plugin in pulse::LANGUAGE_PLUGINS {
        let name = pulse::names::language_definition(plugin);
        let Ok(blob) = data.read_name(&name) else {
            continue;
        };
        let Ok(xml) = expand(&blob) else { continue };
        if let Some(language) = Language::from_definition(plugin, &xml) {
            out.push(language);
        }
    }

    if out.is_empty() {
        report.push("no language plugins resolved; the picker will be empty".to_string());
    } else {
        report.push(format!(
            "{} language(s): {}",
            out.len(),
            out.iter()
                .map(|l| format!("{} ({}, {})", l.name, l.native_name, l.plugin))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out
}

fn load_strings(
    data: &mut oag_assets::Archive,
    languages: &[Language],
    report: &mut Vec<String>,
) -> StringTable {
    // English if the disc has it, otherwise whatever comes first. The original
    // has a saved language to fall back on; there is nothing saved yet.
    let chosen = languages
        .iter()
        .find(|l| l.name == "English")
        .or_else(|| languages.first());

    let Some(language) = chosen else {
        return StringTable::default();
    };
    let Some(entries) = language.entries.as_deref() else {
        report.push(format!("{} names no string table", language.name));
        return StringTable::default();
    };

    match data
        .read_name(entries)
        .and_then(|blob| expand(&blob).map_err(|e| oag_assets::Error::BadSpec(e.to_string())))
    {
        Ok(xml) => {
            let table = StringTable::from_xml(&xml);
            report.push(format!(
                "{entries}: {} strings for {}",
                table.len(),
                language.name
            ));
            table
        }
        Err(e) => {
            report.push(format!("{entries}: {e}"));
            StringTable::default()
        }
    }
}

/// What `--movie` defaults to: the European cut of the dev/pub reel.
///
/// Spelled as a hash because the reel has no recovered name. It is the reel
/// whose contents fit the intro state's constants - 260 frames, static at 144
/// and 231, which is exactly where that state pauses for two seconds.
/// `Data\Movies\Intro.PMF` is a different, 1200-frame movie that the `LogoFMV`
/// screen plays, and pointing the intro at it made the holds land mid-motion.
///
/// European rather than American despite the disc's `UCUS-98712` serial: the
/// executable on this image is the EU build throughout - 18 `UCES00465` strings
/// and no `UCUS` string at all - and the ISO's volume id and publisher are both
/// `SCEE`. Confidence 75; the selection itself has not been read out of the
/// binary. `--movie hash:3d2c85f8` is the American cut.
///
/// See `docs/architecture/frontend-boot.md`.
pub const DEFAULT_INTRO_REEL: &str = "hash:b1ba72c3";

/// How an archive entry was asked for.
///
/// A WAD directory stores only the hash of each name, and three of the disc's
/// movies - including the 260-frame reel the intro state really plays - have no
/// name anyone has recovered. Addressing one by hash is the only way to name it
/// at all, so `hash:3d2c85f8` is accepted anywhere an entry name is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryRef {
    /// A real name, which is hashed to find the entry.
    Name(String),
    /// A name hash, for an entry whose name is not known.
    Hash(u32),
}

impl EntryRef {
    /// Reads the `hash:` prefix, and treats anything else as a name.
    #[must_use]
    pub fn parse(spec: &str) -> Self {
        match spec.strip_prefix("hash:") {
            Some(digits) => match u32::from_str_radix(digits.trim_start_matches("0x"), 16) {
                Ok(hash) => Self::Hash(hash),
                // Not a hash after all. Falling through to a name keeps a
                // mistyped digit an honest "not in Data.wad" rather than a
                // silent match on something else.
                Err(_) => Self::Name(spec.to_string()),
            },
            None => Self::Name(spec.to_string()),
        }
    }

    /// The hash this reference resolves to.
    #[must_use]
    pub fn hash(&self) -> u32 {
        match self {
            Self::Name(name) => oag_formats::wad::hash_name(name),
            Self::Hash(hash) => *hash,
        }
    }
}

impl std::fmt::Display for EntryRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Name(name) => write!(f, "{name}"),
            Self::Hash(hash) => write!(f, "hash:{hash:08x}"),
        }
    }
}

fn load_movie(
    data: &mut oag_assets::Archive,
    options: &Options,
    report: &mut Vec<String>,
) -> Result<Movie> {
    let entry = EntryRef::parse(&options.movie);
    let hash = entry.hash();
    let index = data
        .index_of_hash(hash)
        .with_context(|| format!("{entry} is not in {}", data.label()))?;
    let size = data.entry_len(index)?;
    let options = &Options {
        movie: entry.to_string(),
        ..options.clone()
    };

    if options.no_video {
        // The header alone is 2048 bytes, so this reads kilobytes rather than
        // megabytes when there is no picture to make.
        let head = data.peek(index, oag_formats::pmf::HEADER_LEN as u64)?;
        let header = oag_formats::pmf::Header::parse(&head)
            .map_err(|e| anyhow::anyhow!("parsing {}: {e}", options.movie))?;
        let video = header.video.context("the movie declares no video stream")?;
        report.push(format!(
            "{}: {}x{}, {:.2}s, video disabled",
            options.movie,
            video.width,
            video.height,
            header.duration_seconds()
        ));
        return Ok(Movie {
            frame_count: header.expected_frame_count() as usize,
            width: u32::from(video.width),
            height: u32::from(video.height),
            header,
            frames: None,
            no_picture_reason: Some("--no-video was given".to_string()),
        });
    }

    let blob = data
        .read(index)
        .with_context(|| format!("reading {} out of {}", options.movie, data.label()))?;
    let key = format!("{hash:08x}-{size}");
    let movie = movie::open(&blob, &key, &options.cache, options.extent)?;

    report.push(format!(
        "{}: {}x{}, {:.2}s, {} frames, PSMF{}",
        options.movie,
        movie.width,
        movie.height,
        movie.header.duration_seconds(),
        movie.frame_count,
        String::from_utf8_lossy(&movie.header.version)
    ));
    if let Some(audio) = movie.header.audio {
        report.push(format!(
            "  audio: {} channel(s) at {}, ATRAC3+, not decoded",
            audio.channels,
            audio.frequency_hz().map_or_else(
                || format!("code {}", audio.frequency_code),
                |hz| format!("{hz} Hz")
            )
        ));
    }
    match (&movie.frames, &movie.no_picture_reason) {
        (Some(frames), _) => report.push(format!(
            "  {} frame(s) cached in {}",
            frames.len,
            frames.path().display()
        )),
        (None, Some(reason)) => report.push(format!("  no picture: {reason}")),
        (None, None) => {}
    }
    Ok(movie)
}

fn expand(blob: &[u8]) -> Result<String> {
    if fexml::is_fexml(blob) {
        fexml::expand(blob).map_err(|e| anyhow::anyhow!("{e}"))
    } else {
        String::from_utf8(blob.to_vec()).context("not text")
    }
}

/// The default cache directory, `data/cache/movies` beside the disc images.
#[must_use]
pub fn default_cache_dir() -> std::path::PathBuf {
    Path::new("data/cache/movies").to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::{EntryRef, pulse};

    #[test]
    fn a_name_resolves_through_the_wad_hash() {
        let entry = EntryRef::parse(pulse::names::INTRO_MOVIE);
        assert_eq!(entry, EntryRef::Name(pulse::names::INTRO_MOVIE.to_string()));
        assert_eq!(entry.hash(), 0x71d3_c1ec);
    }

    #[test]
    fn the_default_reel_is_the_european_cut() {
        assert_eq!(
            super::EntryRef::parse(super::DEFAULT_INTRO_REEL).hash(),
            pulse::hashes::DEVPUB_REEL_SCEE
        );
    }

    #[test]
    fn a_hash_addresses_an_entry_with_no_recovered_name() {
        // The SCEA dev/pub reel, which has no name to ask for.
        let entry = EntryRef::parse("hash:3d2c85f8");
        assert_eq!(entry, EntryRef::Hash(0x3d2c_85f8));
        assert_eq!(entry.hash(), 0x3d2c_85f8);
        assert_eq!(entry.to_string(), "hash:3d2c85f8");
        assert_eq!(EntryRef::parse("hash:0x3d2c85f8").hash(), 0x3d2c_85f8);
    }

    #[test]
    fn a_mistyped_hash_stays_a_name_rather_than_matching_something_else() {
        let entry = EntryRef::parse("hash:zzz");
        assert_eq!(entry, EntryRef::Name("hash:zzz".to_string()));
    }

    #[test]
    fn a_name_that_looks_like_a_hash_is_still_a_name() {
        // No prefix, so no hash. Names are never bare hex on these discs, but
        // the rule has to be the prefix rather than the shape.
        assert_eq!(
            EntryRef::parse("3d2c85f8"),
            EntryRef::Name("3d2c85f8".to_string())
        );
    }
}
