//! The track-description panel over the pre-race flyby, read off the disc at race load.
//!
//! The picture is `oag_game::track_panel::Overlay`; this half is the [`Assets`] a race load reads
//! for it and carries in `oag_raceplay::Options::track_panel`.
//!
//! The layout is `InGameTrackDescriptionScreen` in `InGame_Definition.xml`, the two strings are
//! the circuit's name and `MSC_TRACK_<nn>` paragraph in the language's string table, and the
//! timing is [`oag_ui_screens::track_panel`]'s. What reads and what does not:
//!
//! - **Pulse off a PSP disc only**, the one executable the screen was read off.
//! - A string, the screen or a texture that will not read leaves the panel out, and the load
//!   report says which. Nothing stands in for it.
//!
//! See `docs/gameplay/race-intro.md`.

use oag_ui_screens::track_panel::{DrawLists, Progress};

use oag_hud::sprite::Sheet;

/// Everything the panel draws from, read once at race load.
#[derive(Debug, Clone)]
pub struct Assets {
    /// Read once at race load; the overlay draws from it.
    pub layout: oag_ui_screens::campaign::Layout,
    /// Read once at race load; the overlay draws from it.
    pub sheet: Sheet,
    name: String,
    description: String,
    /// Read once at race load; the overlay draws from it.
    pub menu_font: oag_ui::font::Atlas,
    /// Read once at race load; the overlay draws from it.
    pub default_font: oag_ui::font::Atlas,
}

impl Assets {
    /// The panel at `progress`, for a test that has no GPU.
    #[must_use]
    pub fn draw_lists(&self, progress: Progress) -> DrawLists {
        oag_ui_screens::track_panel::draw_lists(
            &self.layout,
            &|src| self.sheet.get(src),
            &self.name,
            &self.description,
            progress,
        )
    }

    /// The circuit's name as the panel shows it.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The circuit's paragraph as the panel shows it.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }
}

/// How the panel's two strings are looked up: the circuit's `PI_Track` id for its name, and
/// [`oag_pulse::race::track_description_id`] of it for its paragraph.
///
/// `None` with a line in `report` for anything missing.
#[must_use]
pub fn read(
    archives: &mut oag_assets::Archives,
    track_id: &str,
    strings: &oag_ui::language::StringTable,
    fonts: (oag_ui::font::Atlas, oag_ui::font::Atlas),
    report: &mut Vec<String>,
) -> Option<Assets> {
    let entry = oag_pulse::race::TRACK_DESCRIPTION_DEFINITION;
    let xml = match archives
        .read_name(entry)
        .map_err(|e| e.to_string())
        .and_then(|blob| oag_tables::fexml::text(&blob).map_err(|e| e.to_string()))
    {
        Ok(xml) => xml,
        Err(why) => {
            report.push(format!(
                "track panel: {entry} did not read ({why}); no panel"
            ));
            return None;
        }
    };
    let screens = oag_ui::screen::Screens::from_xml(&xml);
    let Some(layout) = oag_ui_screens::campaign::Layout::read(
        &screens,
        oag_pulse::race::TRACK_DESCRIPTION_SCREEN,
        strings,
        oag_ui_screens::picker::FaceScales::default(),
        [480.0, 272.0],
    ) else {
        report.push(format!(
            "track panel: {entry} has no {}; no panel",
            oag_pulse::race::TRACK_DESCRIPTION_SCREEN
        ));
        return None;
    };
    let paragraph = oag_pulse::race::track_description_id(track_id)?;
    let (Some(name), Some(description)) = (strings.get(track_id), strings.get(&paragraph)) else {
        report.push(format!(
            "track panel: the string table has no {track_id} or {paragraph}; no panel"
        ));
        return None;
    };
    let mut blobs = Vec::new();
    for image in &layout.screen.images {
        if blobs
            .iter()
            .any(|(src, _): &(String, Vec<u8>)| *src == image.src)
        {
            continue;
        }
        match archives.read_name(&image.src) {
            Ok(blob) => blobs.push((image.src.clone(), blob)),
            Err(why) => report.push(format!(
                "track panel: {} did not read ({why}); that texture is left out",
                image.src
            )),
        }
    }
    let mut notes = Vec::new();
    let sheet = Sheet::default().extended(&blobs, &mut notes);
    report.extend(notes.into_iter().map(|n| format!("track panel: {n}")));
    report.push(format!(
        "track panel: {track_id} {name:?}, {} widget(s), {} texture(s)",
        layout.screen.texts.len() + layout.screen.fills.len() + layout.screen.images.len(),
        blobs.len()
    ));
    Some(Assets {
        layout,
        sheet,
        name: name.to_string(),
        description: description.to_string(),
        menu_font: fonts.0,
        default_font: fonts.1,
    })
}
