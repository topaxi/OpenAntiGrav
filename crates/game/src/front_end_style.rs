//! The disc's own front-end styles (HD's `FE Style`: `HD` white and teal, `FURY` black and
//! red), as the colours and the corner shape a screen draws with.
//!
//! Each style is a copy of the front-end root with its own `FEGlobals`; the served copy is
//! the one the menus are built from, and a style whose ground differs takes its globals
//! from the copy that has it (`docs/formats/hd-frontend.md`, "The `HD_*` palette is the FE
//! style, and the archives disagree"). The loading screen and the `EndRace` screens both
//! resolve through [`resolve`]. A saved choice of `None` is the served copy's style, which on
//! HD is Fury, the style a first launch with the Fury content forces
//! (`docs/ghidra/functions/ps3-hdfury-eu/loading-screen.md`, "The byte at `0x00b979fd` is
//! \"the Fury content is present\"").

use oag_ui::screen::Screens;

/// The saved front-end styling, as a name a title can match, or `None` for
/// "whatever this source leads with".
///
/// An empty setting is `None` rather than `Some("")`: the key is always in the
/// file (see [`crate::settings::Display::front_end_style`]) and empty is what it holds
/// until somebody chooses, which must not be matched against a real style name.
#[must_use]
pub(crate) fn style_of(settings: &crate::settings::Settings) -> Option<&str> {
    let name = settings.display.front_end_style.trim();
    (!name.is_empty()).then_some(name)
}

/// What a saved style resolves to on one source.
#[derive(Debug)]
pub struct Resolved {
    /// The style, when the title ships any.
    pub chosen: Option<&'static oag_title::loading::FeatureStyle>,
    /// The front-end root whose `FEGlobals` this style draws with: the served copy, or the
    /// copy whose ground matches the style. `None` when the title names no palette.
    pub root: Option<Screens>,
    /// The archive `root` was taken from, only when it is not the served copy.
    pub taken_from: Option<String>,
}

/// Resolves `style` (a saved name, or `None` for the served copy's own) on `title`.
///
/// `context` prefixes the lines pushed to `notes`.
pub fn resolve(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    style: Option<&str>,
    context: &str,
    notes: &mut Vec<String>,
) -> Resolved {
    let none = Resolved {
        chosen: None,
        root: None,
        taken_from: None,
    };
    let Some(loading) = title.loading else {
        return none;
    };
    // Read through the title's own colour template, as its front end is, so the globals are
    // the ones the menus resolve (Omega's `WOHD_Colours` overwrites `HD_Grey`); a title with
    // no template reads the file as it stands.
    let screens = |blob: &[u8], template| {
        let xml = oag_ui::xml::expand(blob).ok()?;
        Some(Screens::from_xml_with_fallbacks(&xml, &[], &[], template))
    };
    let mut root = loading.palette.and_then(|_| {
        let front_end = title.front_end?;
        let blob = archives.read_name(front_end.root).ok()?;
        screens(&blob, front_end.boot.colour_template)
    });
    let dark_ground = loading
        .palette
        .zip(root.as_ref())
        .is_some_and(|(names, served)| served.global_is_dark(names.background));
    let chosen = crate::loading::pick_style(loading.features, style, dark_ground, context, notes);
    let mut taken_from = None;
    // A style chosen against the served front end's ground (HD's white pictures on the black
    // archive) takes the globals from the copy of the front-end file whose ground matches, as
    // the original does with a skin file per style; without one the served copy stands.
    if let (Some(names), Some(style), Some(front_end)) = (loading.palette, chosen, title.front_end)
        && root
            .as_ref()
            .is_some_and(|served| served.global_is_dark(names.background) != style.dark_ground)
    {
        let own = archives
            .read_every_name(front_end.root)
            .into_iter()
            .filter_map(|(label, blob)| {
                Some((label, screens(&blob, front_end.boot.colour_template)?))
            })
            .find(|(_, copy)| {
                copy.globals.contains_key(names.background)
                    && copy.global_is_dark(names.background) == style.dark_ground
            });
        match own {
            Some((label, copy)) => {
                notes.push(format!(
                    "{context}: {} style takes its colours from {label}",
                    style.name
                ));
                root = Some(copy);
                taken_from = Some(label);
            }
            None => notes.push(format!(
                "{context}: no copy of {} has a {} ground; the served colours stand",
                front_end.root,
                if style.dark_ground { "dark" } else { "light" }
            )),
        }
    }
    Resolved {
        chosen,
        root,
        taken_from,
    }
}

/// The globals and corner shape a screen drawn in a saved style takes.
#[derive(Debug)]
pub struct ScreenStyle {
    /// The served globals, overlaid with the style's own where it is not the served one.
    pub globals: Vec<(String, String)>,
    /// `FrontEnd_IsFuryStyle` for a style that is not the served one; `None` leaves the
    /// served frame's corners.
    fury: Option<bool>,
}

impl ScreenStyle {
    /// The style `saved` picks for screens that resolve `FEGlobals->` against `served`.
    pub fn new<'a>(
        archives: &mut oag_assets::Archives,
        title: &'static oag_title::Title,
        saved: Option<&str>,
        served: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Self {
        let mut globals: std::collections::HashMap<String, String> = served
            .into_iter()
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        // With no saved choice the served copy stands, so nothing is read for it.
        let mut fury = None;
        if let Some(saved) = saved {
            let mut notes = Vec::new();
            let resolved = resolve(archives, title, Some(saved), "front-end style", &mut notes);
            for note in notes {
                log::debug!("{note}");
            }
            if resolved.taken_from.is_some()
                && let (Some(root), Some(chosen)) = (resolved.root, resolved.chosen)
            {
                globals.extend(root.globals);
                fury = Some(chosen.dark_ground);
            }
        }
        Self {
            globals: globals.into_iter().collect(),
            fury,
        }
    }

    /// The globals as the borrowed pairs the screen readers take.
    #[must_use]
    pub fn global_refs(&self) -> Vec<(&str, &str)> {
        self.globals
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect()
    }

    /// `frame` with its block corners and fills in this style.
    #[must_use]
    pub fn frame(&self, frame: &oag_ui::menu::Frame) -> oag_ui::menu::Frame {
        let mut frame = frame.clone();
        if let (Some(fury), Some(blocks)) = (self.fury, frame.blocks.as_mut()) {
            blocks.fury = fury;
        }
        frame
    }
}
