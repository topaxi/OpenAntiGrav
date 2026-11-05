//! A loading screen as data: the primitives it is drawn from and where each one goes.
//!
//! The file is `screen = "loading"` of the screen-layout format
//! (`schemas/v1/screen-layout.tosd`, `docs/formats/screen-layout.md`), read through the
//! asset layers: a title names one ([`oag_title::Loading::layout`]) and the fallback layer
//! ships it, so a player's override replaces it by path, and a mod track may carry its own.
//!
//! ```toml
//! screen = "loading"
//!
//! [[layout]]
//! name = "race-load"
//! grid = [1920, 1080]           # the unit of every `at`, `size` and `rect` below
//! modes = ["single_race"]       # absent = every mode
//! aspect = [1.0, 3.0]           # the surface's width / height, [min, max); absent = any
//!
//! [[layout.element]]            # drawn in array order, the first furthest back
//! id = "rule-top"
//! draw = "image"
//! asset = "rule"
//! at = [160, 110]
//! size = [1600, 8]
//! ```
//!
//! **Nothing here is drawn from a guess.** An element whose `asset`, `source` or
//! `string_id` the title does not resolve is skipped and named in the loader report, not
//! replaced by a stand-in; what the numbers are (measured, chosen) is said in the file's
//! comments, beside each one.

use serde::Deserialize;

use crate::screen_layout::{SCHEMA_KIND, SCHEMA_MAJOR, Screen, pair};

/// Why a loading layout was refused. Every variant names what it found.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Not TOML, or a key or shape the schema does not have.
    #[error("not a screen-layout file: {0}")]
    Parse(#[from] toml::de::Error),
    /// The `[toml-schema]` header names another kind, another major, or is malformed.
    #[error("{0}")]
    Header(#[from] oag_schema::Error),
    /// The file's `screen` is not `loading`, or is missing.
    #[error("not a loading layout: its `screen` is {0}")]
    Screen(String),
    /// A value outside what an element can mean.
    #[error("layout {layout}, element {element}: {what}")]
    Invalid {
        /// The layout's `name`, or its position.
        layout: String,
        /// The element's `id`.
        element: String,
        /// What is wrong.
        what: String,
    },
}

/// Where a text run sits against its `at`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Align {
    /// `at` is the run's left edge.
    #[default]
    Left,
    /// `at` is the run's middle.
    Centre,
    /// `at` is the run's right edge.
    Right,
}

/// What an element draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Draw {
    /// A textured quad: `asset`, `at`, `size`, optionally `uv` and `colour`.
    Image,
    /// A text run: `source` or `text`, `font`, `scale`, `at`, optionally `wrap` and `pitch`.
    Text,
    /// A square bullet and a line of text beside it: `string_id` or `text`, `at`, `asset`,
    /// `size` (the bullet), `gap` (how far right of `at` the text starts).
    Label,
    /// Bracket corners round `rect`: `asset`, `size` (one bracket's sprite), `back` (how far
    /// a bracket on the right or bottom edge is pulled back from it).
    Corners,
    /// A grid of tiles, the left `fill` of it lit: `asset`, `at`, `columns`, `rows`, `tile`.
    Dots,
    /// The mode's video clip: `source`, `at`, `size`.
    Clip,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    screen: Option<Screen>,
    #[serde(rename = "toml-schema")]
    toml_schema: Option<oag_schema::Header>,
    #[serde(default)]
    layout: Vec<LayoutFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LayoutFile {
    name: Option<String>,
    #[serde(default, deserialize_with = "pair")]
    grid: Option<[f32; 2]>,
    #[serde(default, deserialize_with = "pair")]
    aspect: Option<[f32; 2]>,
    #[serde(default)]
    modes: Vec<String>,
    #[serde(default)]
    element: Vec<ElementFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementFile {
    id: String,
    draw: Draw,
    #[serde(default, deserialize_with = "pair")]
    at: Option<[f32; 2]>,
    #[serde(default, deserialize_with = "pair")]
    size: Option<[f32; 2]>,
    rect: Option<[f32; 4]>,
    scale: Option<f32>,
    asset: Option<String>,
    uv: Option<[f32; 4]>,
    font: Option<String>,
    colour: Option<String>,
    unlit: Option<String>,
    string_id: Option<String>,
    text: Option<String>,
    source: Option<String>,
    wrap: Option<f32>,
    pitch: Option<f32>,
    gap: Option<f32>,
    back: Option<f32>,
    align: Option<Align>,
    columns: Option<u32>,
    rows: Option<u32>,
    tile: Option<f32>,
    fill: Option<String>,
}

/// One primitive and where it goes, in the layout's `grid`.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// Unique in its layout: a mod's handle for it.
    pub id: String,
    /// What it draws.
    pub draw: Draw,
    /// Top-left corner (text: where the text box starts).
    pub at: [f32; 2],
    /// Width and height, for an image or a clip.
    pub size: [f32; 2],
    /// `[x0, y0, x1, y1]`, for corners.
    pub rect: [f32; 4],
    /// Text scale; `1.0` is the face at its own size.
    pub scale: f32,
    /// A role the title's art table names (`rule`, `corner`, `dot`, `square`,
    /// `title-arrow`, `subtitle-arrow`), `@feature` or `@mode-icon`.
    pub asset: Option<String>,
    /// `[u0, v0, u1, v1]` of the asset, in fractions of it; `None` is all of it.
    pub uv: Option<[f32; 4]>,
    /// A front-end font role (`Default`, `Title`); `None` is `Default`.
    pub font: Option<String>,
    /// A palette role: `ink`, `accent`, `trough`, `ground`; `None` is `ink`.
    pub colour: Option<String>,
    /// For dots: the role of the tiles past the fill.
    pub unlit: Option<String>,
    /// A string id of the disc's table (a label).
    pub string_id: Option<String>,
    /// A literal.
    pub text: Option<String>,
    /// What the screen supplies: `caption`, `feature.title`, `feature.heading`,
    /// `feature.text`, `mode-icon`.
    pub source: Option<String>,
    /// Text: the wrap width.
    pub wrap: Option<f32>,
    /// Text: the line pitch.
    pub pitch: Option<f32>,
    /// Label: how far right of `at` the text starts.
    pub gap: f32,
    /// Corners: how far a right or bottom bracket is pulled back from its edge.
    pub back: f32,
    /// Text: which edge `at` names.
    pub align: Align,
    /// Dots: columns, rows and the tile's side.
    pub columns: u32,
    /// Dots: rows.
    pub rows: u32,
    /// Dots: one tile's side.
    pub tile: f32,
    /// Dots: what lights the left part (`progress`).
    pub fill: Option<String>,
}

/// One layout: the surface it answers and its elements.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// For a load report.
    pub name: String,
    /// The unit of every coordinate: the source's own pixels.
    pub grid: [f32; 2],
    /// The surface's width over height, `[min, max)`; `None` is any.
    pub aspect: Option<[f32; 2]>,
    /// `oag_race::Mode::name`s it answers; empty is every mode.
    pub modes: Vec<String>,
    /// In draw order.
    pub elements: Vec<Element>,
}

/// A parsed loading layout file: the first layout to answer wins.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadingLayouts {
    /// In file order.
    pub layouts: Vec<Layout>,
}

impl LoadingLayouts {
    /// Reads a file.
    ///
    /// # Errors
    ///
    /// [`Error`]: not TOML, another `screen`, another kind or major, or a value out of range.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let file: File = toml::from_str(text)?;
        match file.screen {
            Some(Screen::Loading) => {}
            Some(other) => return Err(Error::Screen(format!("\"{}\"", other.name()))),
            None => return Err(Error::Screen("absent".into())),
        }
        match oag_schema::check(file.toml_schema.as_ref(), SCHEMA_KIND, SCHEMA_MAJOR) {
            Ok(()) => {}
            Err(missing @ oag_schema::Error::Missing { .. }) => {
                log::warn!("loading layout: {missing}");
            }
            Err(refused) => return Err(refused.into()),
        }
        let layouts = file
            .layout
            .into_iter()
            .enumerate()
            .map(|(at, layout)| Layout::from_file(at, layout))
            .collect::<Result<_, _>>()?;
        Ok(Self { layouts })
    }

    /// The first layout that answers a race of `mode` on a surface of `aspect`.
    #[must_use]
    pub fn for_race(&self, mode: &str, aspect: f32) -> Option<&Layout> {
        self.layouts.iter().find(|layout| {
            (layout.modes.is_empty() || layout.modes.iter().any(|m| m == mode))
                && layout
                    .aspect
                    .is_none_or(|[low, high]| aspect >= low && aspect < high)
        })
    }
}

impl Layout {
    fn from_file(at: usize, file: LayoutFile) -> Result<Self, Error> {
        let name = file.name.unwrap_or_else(|| format!("#{}", at + 1));
        let invalid = |element: &str, what: String| Error::Invalid {
            layout: name.clone(),
            element: element.to_string(),
            what,
        };
        let grid = match file.grid {
            Some(grid) if grid.iter().all(|g| g.is_finite() && *g > 0.0) => grid,
            Some(grid) => return Err(invalid("-", format!("`grid` {grid:?} is not positive"))),
            None => return Err(invalid("-", "a loading layout needs a `grid`".into())),
        };
        if let Some([low, high]) = file.aspect
            && !(low.is_finite() && high.is_finite() && 0.0 <= low && low < high)
        {
            return Err(invalid(
                "-",
                format!("`aspect` [{low}, {high}] is not an increasing range from 0 or more"),
            ));
        }
        let mut seen = Vec::new();
        let mut elements = Vec::new();
        for element in file.element {
            if seen.contains(&element.id) {
                return Err(invalid(&element.id, "the id is used twice".into()));
            }
            seen.push(element.id.clone());
            elements.push(Element::from_file(element, &invalid)?);
        }
        Ok(Self {
            name,
            grid,
            aspect: file.aspect,
            modes: file.modes,
            elements,
        })
    }
}

impl Element {
    fn from_file(
        file: ElementFile,
        invalid: &dyn Fn(&str, String) -> Error,
    ) -> Result<Self, Error> {
        let id = file.id.as_str();
        let need = |present: bool, key: &str| {
            if present {
                Ok(())
            } else {
                Err(invalid(
                    id,
                    format!("`draw = \"{:?}\"` needs `{key}`", file.draw),
                ))
            }
        };
        match file.draw {
            Draw::Image => {
                need(file.asset.is_some(), "asset")?;
                need(file.at.is_some(), "at")?;
                need(file.size.is_some(), "size")?;
            }
            Draw::Text => {
                need(file.at.is_some(), "at")?;
                if file.source.is_none() == file.text.is_none() {
                    return Err(invalid(
                        id,
                        "a text needs one of `source` and `text`".into(),
                    ));
                }
            }
            Draw::Label => {
                need(file.at.is_some(), "at")?;
                need(file.asset.is_some(), "asset")?;
                need(file.size.is_some(), "size")?;
                need(file.gap.is_some(), "gap")?;
                if file.string_id.is_none() == file.text.is_none() {
                    return Err(invalid(
                        id,
                        "a label needs one of `string_id` and `text`".into(),
                    ));
                }
            }
            Draw::Corners => {
                need(file.rect.is_some(), "rect")?;
                need(file.asset.is_some(), "asset")?;
                need(file.size.is_some(), "size")?;
                need(file.back.is_some(), "back")?;
            }
            Draw::Dots => {
                need(file.asset.is_some(), "asset")?;
                need(file.at.is_some(), "at")?;
                need(file.columns.is_some(), "columns")?;
                need(file.rows.is_some(), "rows")?;
                need(file.tile.is_some(), "tile")?;
            }
            Draw::Clip => {
                need(file.source.is_some(), "source")?;
                need(file.at.is_some(), "at")?;
                need(file.size.is_some(), "size")?;
            }
        }
        let scale = file.scale.unwrap_or(1.0);
        if !(scale.is_finite() && scale > 0.0 && scale <= 8.0) {
            return Err(invalid(id, format!("`scale` {scale} is not in (0, 8]")));
        }
        for number in file
            .at
            .iter()
            .flatten()
            .chain(file.size.iter().flatten())
            .chain(file.rect.iter().flatten())
            .chain(file.wrap.iter())
            .chain(file.pitch.iter())
            .chain(file.tile.iter())
            .chain(file.gap.iter())
            .chain(file.back.iter())
        {
            if !number.is_finite() {
                return Err(invalid(id, "a number is not finite".into()));
            }
        }
        if let Some(uv) = file.uv
            && !uv.iter().all(|v| v.is_finite())
        {
            return Err(invalid(id, "`uv` is not finite".into()));
        }
        Ok(Self {
            id: file.id,
            draw: file.draw,
            at: file.at.unwrap_or([0.0, 0.0]),
            size: file.size.unwrap_or([0.0, 0.0]),
            rect: file.rect.unwrap_or([0.0; 4]),
            scale,
            asset: file.asset,
            uv: file.uv,
            font: file.font,
            colour: file.colour,
            unlit: file.unlit,
            string_id: file.string_id,
            text: file.text,
            source: file.source,
            wrap: file.wrap,
            pitch: file.pitch,
            gap: file.gap.unwrap_or(0.0),
            back: file.back.unwrap_or(0.0),
            align: file.align.unwrap_or_default(),
            columns: file.columns.unwrap_or(0),
            rows: file.rows.unwrap_or(0),
            tile: file.tile.unwrap_or(0.0),
            fill: file.fill,
        })
    }
}

#[cfg(test)]
mod tests;
