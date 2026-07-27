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
    /// Archive entry name of the movie to play.
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

    let frames = movie.frames.as_ref().map_or(0, |f| f.len);
    let frontend = Frontend::new(screens, strings, languages, frames, movie.frames.is_some());

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
        report,
    })
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

fn load_movie(
    data: &mut oag_assets::Archive,
    options: &Options,
    report: &mut Vec<String>,
) -> Result<Movie> {
    let index = data
        .index_of_name(&options.movie)
        .with_context(|| format!("{} is not in {}", options.movie, data.label()))?;
    let size = data.entry_len(index)?;
    let hash = oag_formats::wad::hash_name(&options.movie);

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
