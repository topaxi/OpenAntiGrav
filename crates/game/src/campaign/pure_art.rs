//! The picture on the right-hand side of Wipeout Pure's SINGLE PLAYER screens,
//! read off the disc and played, never drawn from a table.
//!
//! Three screens author one, in two places:
//!
//! - `Class Selection` authors it on the screen itself
//!   (`Selection_Definition.xml`): a backdrop and, inside one `<Watch
//!   watch="Class" value="Vector">` per class, the class's stat graph.
//! - `League Selection`, `Tournament Selection`, `Track Selection` and `Zone
//!   Track Selection` author none. The focused entry's own `<location>\screen.xml`
//!   does - the league's emblem, the circuit's map and records - the file
//!   `Team Selection` already reads for its craft, so [`oag_ui_screens::picker::slideshow::Slideshow`]
//!   plays all of it.
//! - `Single Player` and the main menu author nothing on the right (census of
//!   all eleven definition files, `docs/formats/pure-campaign.md`).
//!
//! An art that will not read is an error the caller logs, and the screen draws
//! nothing there.

use anyhow::{Context, Result};
use oag_ui::language::StringTable;
use oag_ui::screen::Screens;
use oag_ui_screens::picker::slideshow::Slideshow;

use crate::preview::Still;

/// What to show: the focused row's art, by what it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// A speed class of `Class Selection`, by its disc idstring: the `<Watch>`
    /// that holds `Class` at that value.
    Class(String),
    /// An entry's directory, whose `screen.xml` is the art.
    Entry {
        /// `PI_League`'s, `PI_Tournament`'s or `PI_Track`'s `location`.
        location: String,
        /// Zone mode: `screen_zone.xml` and the `Zone` state where the circuit
        /// authors them.
        zone: bool,
        /// The `PI_Track` it is when it is one: the circuit whose records and
        /// medal figures fill the panel's placeholders ([`Fills`]).
        track: Option<String>,
    },
}

/// The art for one key, and the textures it draws.
#[expect(missing_debug_implementations, reason = "holds raw texture bytes")]
pub struct Art {
    /// What to draw, and when.
    pub show: Slideshow,
    /// Every texture [`Self::show`] names, decoded off the disc.
    pub stills: Vec<Still>,
}

/// Reads the art `key` names.
///
/// `selection` is the entry `Class Selection` is authored in
/// ([`oag_title::FlowFiles::selection`]) and `screen` that screen's name.
/// Textures that will not read are reported and left out.
///
/// # Errors
///
/// The definition or the entry's `screen.xml` is missing or authors nothing to
/// show, or the class names no `<Watch>`.
pub fn load(
    archives: &mut oag_assets::Archives,
    key: &Key,
    selection: (&str, &str),
    globals: &[(&str, &str)],
    strings: &StringTable,
    report: &mut Vec<String>,
) -> Result<Art> {
    match key {
        Key::Entry { location, zone, .. } => {
            let (show, stills) =
                crate::preview::slideshow(archives, location, *zone, globals, strings, report)?;
            Ok(Art { show, stills })
        }
        Key::Class(class) => {
            let (entry, screen) = selection;
            let blob = archives
                .read_name(entry)
                .with_context(|| format!("reading {entry}"))?;
            let xml = oag_tables::fexml::text(&blob)?;
            let screens = Screens::from_xml_with_fallback_globals(&xml, globals);
            let screen = screens
                .by_name(screen)
                .with_context(|| format!("{entry} has no {screen}"))?;
            let watch = screen
                .watches
                .iter()
                .find(|watch| watch.global == "Class" && watch.value == *class)
                .with_context(|| {
                    format!(
                        "{screen_name} authors no art for {class}",
                        screen_name = screen.name
                    )
                })?;
            let images: Vec<_> = screen.images.iter().chain(&watch.images).cloned().collect();
            let show = Slideshow::still(images);
            let stills = crate::preview::read_stills(archives, show.sources(), report);
            Ok(Art { show, stills })
        }
    }
}

/// The `Key::track` a race on the circuit at `location` records under: the
/// archive entry the launch races (`plan_cell_for`'s `leg_entries`), lower-cased
/// as the store keeps it. A disc-backed test holds it to the launch's own.
#[must_use]
pub fn record_entry(location: &str) -> String {
    format!(r"{location}\track.vex").to_ascii_lowercase()
}

/// What a screen's own code writes into the circuit panel's placeholders, by
/// `PI_Track` name: pairs of widget name and text.
///
/// The files author `Race Record` and `Lap Record` as `00' 00' 00` and the
/// Zone panel's counters as `00`, and the engine overwrites them
/// (`<DisplayRecords track="...">`). **Measured on a fresh profile** (PPSSPP,
/// 2026-10-12): `Race Record` and `Lap Record` read `0.00.00`, the Zone medal
/// counters `25`, `20`, `15` (the circuit's `<ZoneMedals>`), and `Best Zone` and
/// `Best Score` `0`. **Chosen, not measured:** with a stored result, which of
/// the profile's rows a record reads (the quickest finished race and the
/// quickest lap on that circuit in any mode or class), and `Best Zone` and
/// `Best Score` are left blank once a Zone result is stored, since how the
/// original words them was not captured.
#[derive(Debug, Clone, Default)]
pub struct Fills(std::collections::BTreeMap<String, Vec<(&'static str, String)>>);

impl Fills {
    /// The panels' values for every circuit of `campaign`, against `rows` (the
    /// profile's records) as they are now.
    #[must_use]
    pub fn of(
        campaign: &super::pure_list::PureCampaign,
        title: &str,
        rows: &[crate::records::Record],
    ) -> Self {
        use oag_hud::{Precision, format_lap_time};
        use oag_tables::pure_campaign::TrackKind;
        let time = |ticks: u64| format_lap_time(ticks, Precision::Hundredths);
        let title = title.to_ascii_lowercase();
        let mut out = std::collections::BTreeMap::new();
        for track in &campaign.definition.tracks {
            let entry = record_entry(&track.location);
            let mine: Vec<_> = rows
                .iter()
                .filter(|row| row.title == title && row.track == entry)
                .collect();
            let fills = if track.kind == TrackKind::Zone {
                let mut fills = Vec::new();
                if let Some(stats) = campaign.stats.get(&track.name) {
                    for (name, count) in ["Gold Zone", "Silver Zone", "Bronze Zone"]
                        .into_iter()
                        .zip(stats.zone)
                    {
                        fills.push((name, count.to_string()));
                    }
                }
                let held = if mine.is_empty() { "0" } else { "" };
                fills.push(("Best Zone", held.to_string()));
                fills.push(("Best Score", held.to_string()));
                fills
            } else {
                let race = mine.iter().filter_map(|row| row.best_total_ticks).min();
                let lap = mine
                    .iter()
                    .filter_map(|row| row.best_lap_ticks)
                    .min()
                    .map(u64::from);
                vec![
                    ("Race Record", time(race.unwrap_or(0))),
                    ("Lap Record", time(lap.unwrap_or(0))),
                ]
            };
            out.insert(track.name.clone(), fills);
        }
        Self(out)
    }

    /// Writes `track`'s values into the placeholders of its panel.
    pub fn apply(&self, track: &str, show: &mut Slideshow) {
        for (name, value) in self.0.get(track).map_or(&[][..], Vec::as_slice) {
            show.fill(name, value);
        }
    }
}

/// Seconds a picture takes to arrive when its widgets author no
/// `transition`. **Chosen, not measured**; see [`crate::preview::CARD_FADE_SECONDS`],
/// the same number for the same reason.
pub const FADE_SECONDS: f32 = crate::preview::CARD_FADE_SECONDS;

/// The art on screen for the focused row, loaded on demand and drawn from a
/// sheet of its own: the front end's, extended with every texture shown so far.
#[expect(missing_debug_implementations, reason = "holds an open archive")]
pub struct Player {
    archives: oag_assets::Archives,
    base: oag_hud::sprite::Sheet,
    globals: Vec<(String, String)>,
    strings: StringTable,
    selection: (String, String),
    fills: Fills,
    /// The key last asked for, whether or not its art read: a key that failed
    /// is not read again every tick.
    asked: Option<Key>,
    shown: Option<Slideshow>,
    sheet: Option<oag_hud::sprite::Sheet>,
    uploaded: bool,
    since: f32,
}

impl Player {
    /// A player reading `archives`, drawing on `base`'s textures.
    #[must_use]
    pub fn new(
        archives: oag_assets::Archives,
        base: oag_hud::sprite::Sheet,
        globals: Vec<(String, String)>,
        strings: StringTable,
        selection: (String, String),
        fills: Fills,
    ) -> Self {
        Self {
            archives,
            base,
            globals,
            strings,
            selection,
            fills,
            asked: None,
            shown: None,
            sheet: None,
            uploaded: false,
            since: 0.0,
        }
    }

    /// Shows the art `key` names, or nothing. Does nothing when it is the art
    /// already shown; otherwise reads it (once the first time) and restarts its
    /// arrival. A key whose art will not read draws nothing and says why.
    pub fn show(&mut self, key: Option<Key>) {
        if self.asked == key {
            return;
        }
        self.since = 0.0;
        self.shown = None;
        self.asked.clone_from(&key);
        let Some(key) = key else {
            return;
        };
        let globals: Vec<(&str, &str)> = self
            .globals
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        let mut report = Vec::new();
        let selection = (self.selection.0.as_str(), self.selection.1.as_str());
        let loaded = load(
            &mut self.archives,
            &key,
            selection,
            &globals,
            &self.strings,
            &mut report,
        );
        for line in report {
            log::warn!("SINGLE PLAYER art: {line}");
        }
        match loaded {
            Ok(mut art) => {
                if let Key::Entry {
                    track: Some(track), ..
                } = &key
                {
                    self.fills.apply(track, &mut art.show);
                }
                let current = self.sheet.as_ref().unwrap_or(&self.base);
                if art.stills.iter().any(|(src, _)| current.get(src).is_none()) {
                    let mut report = Vec::new();
                    self.sheet = Some(current.extended(&art.stills, &mut report));
                    self.uploaded = false;
                    for line in report {
                        log::debug!("SINGLE PLAYER art: {line}");
                    }
                }
                self.shown = Some(art.show);
            }
            Err(error) => {
                log::warn!("SINGLE PLAYER art for {key:?}: {error:#} - nothing is drawn there")
            }
        }
    }

    /// Moves the arrival clock on.
    pub fn advance(&mut self, seconds: f32) {
        self.since += seconds.max(0.0);
    }

    /// The sheet the renderer has to draw from, the first time it changes.
    pub fn take_sheet(&mut self) -> Option<&oag_hud::sprite::Sheet> {
        if self.uploaded {
            return None;
        }
        self.uploaded = true;
        self.sheet.as_ref()
    }

    fn placed(&self, src: &str) -> Option<oag_ui::frontend::Placed> {
        self.sheet.as_ref().unwrap_or(&self.base).get(src)
    }

    /// The picture on screen now, faded in over its own authored `transition`
    /// (the longest one its images carry), else [`FADE_SECONDS`].
    #[must_use]
    pub fn draws(&self) -> Vec<oag_ui::frontend::Draw> {
        let Some(show) = &self.shown else {
            return Vec::new();
        };
        let authored = show
            .states()
            .iter()
            .flat_map(|state| &state.images)
            .map(|image| image.transition)
            .fold(0.0_f32, f32::max);
        let over = if authored > 0.0 {
            authored
        } else {
            FADE_SECONDS
        };
        let alpha = (self.since / over).clamp(0.0, 1.0);
        show.draws(self.since, &|src| self.placed(src))
            .into_iter()
            .map(|draw| crate::preview::fade_draw(draw, alpha))
            .collect()
    }
}
