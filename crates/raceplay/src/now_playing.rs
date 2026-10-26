//! The song-name marquee on the race screen: its layout read off the disc at load, and its
//! animation stepped once per tick.
//!
//! The picture is `oag_hud::now_playing`; this half holds the [`View`] a race load builds and
//! the two `Race` methods that drive it - [`Race::note_song`] when the music starts a song and
//! the step inside the tick. Display state, out of the hash, like the HUD's other bars.
//!
//! Wired for **Pulse** (PSP) only so far; a title whose [`oag_title::Music::now_playing`] is
//! `None` gets no [`View`], and a layout or font that will not read leaves it out with a line in
//! the load report. Nothing stands in. See `docs/formats/now-playing.md`.

use oag_hud::now_playing::{Display, geometry};

use crate::Race;

/// The display, its font (to measure how far the text has to scroll) and the last song seen.
#[derive(Debug, Clone)]
pub struct View {
    display: Display,
    font: oag_ui::font::Atlas,
    separator: &'static str,
    /// The language's strings, for a title whose nodes name a key ([`oag_title::now_playing::NowPlaying::title_key`]).
    strings: oag_ui::language::StringTable,
    title_key: Option<&'static str>,
    serial: u32,
    /// Whether the title's options can switch the HUD off ([`oag_title::HudArt::dynamic_hud`]).
    gated: bool,
    /// Whether the HUD is up. Only ever `false` on a `gated` title.
    hud_shown: bool,
}

impl View {
    /// Reads the title's layout, or says in `report` why there is none.
    #[must_use]
    pub fn read(
        archives: &mut oag_assets::Archives,
        title: &oag_title::Title,
        hud: &oag_hud::Assets,
        report: &mut Vec<String>,
    ) -> Option<Self> {
        let authored = title.music?.now_playing?;
        let xml = match archives
            .read_name(authored.layout)
            .map_err(|e| e.to_string())
            .and_then(|blob| oag_tables::fexml::text(&blob).map_err(|e| e.to_string()))
        {
            Ok(xml) => xml,
            Err(why) => {
                report.push(format!(
                    "now playing: {} did not read ({why}); no display",
                    authored.layout
                ));
                return None;
            }
        };
        let Some(geometry) = geometry(&xml, &authored) else {
            report.push(format!(
                "now playing: {} has no {}; no display",
                authored.layout, authored.item
            ));
            return None;
        };
        report.push(format!(
            "now playing: {} bar {:?}, text from x {} clipped to {:?} ({})",
            authored.item,
            geometry.bar,
            geometry.origin_x,
            geometry.clip,
            match authored.origin {
                oag_title::effects::Origin::Measured => "measured".to_string(),
                oag_title::effects::Origin::InheritedFrom(title) =>
                    format!("borrowed from {title}, chosen not measured"),
                oag_title::effects::Origin::Chosen => "chosen, not measured".to_string(),
            }
        ));
        Some(Self {
            display: Display::new(geometry, &authored),
            font: hud.small_font.clone(),
            separator: authored.separator,
            strings: hud.strings.clone(),
            title_key: authored.title_key,
            serial: 0,
            gated: title.hud_art.dynamic_hud.is_some(),
            hud_shown: true,
        })
    }
}

impl Race {
    /// The race music started a song (or has not yet): shows its name the way the title does,
    /// once per song, on the tick it is told. `None` for a song the disc does not name, which
    /// shows nothing.
    ///
    /// The display follows the sound and does not wait for the light itself: a title whose
    /// original starts the song at the release (`oag_title::Music::race_song_start`) holds the
    /// song until then in `oag_sound::Audio`, which reports nothing while it is held.
    pub fn note_song(&mut self, song: Option<&oag_sound::NowPlaying>) {
        let Some(view) = self.view.now_playing.as_mut() else {
            return;
        };
        let Some(song) = song.filter(|song| song.serial != view.serial) else {
            return;
        };
        view.serial = song.serial;
        // `ShowNowPlaying` shows nothing while the HUD is off, and the song it skipped is not
        // shown later: the next one is.
        if !view.hud_shown {
            return;
        }
        if let Some(name) = &song.name {
            let title = match view.title_key {
                None => Some(name.title.clone()),
                // The key's text is "Artist\nTitle"; a key the table lacks shows nothing.
                Some(key) => view
                    .strings
                    .get(&key.replace("{}", &name.title))
                    .and_then(|text| text.lines().last())
                    .map(str::to_string),
            };
            let Some(title) = title else {
                return;
            };
            let text = match &name.artist {
                Some(artist) => format!("{artist}{}{title}", view.separator),
                None => title,
            };
            let width = oag_ui::font::measure(&view.font, &text) * view.display.scale();
            view.display.show(text, width);
        }
    }

    /// The player's `Dynamic HUD` choice, on a title whose options have one (anything else is
    /// ignored). `off` takes a marquee that is up down and keeps the next songs from showing one;
    /// switching back on shows nothing until a song starts. Display state, so the frame loop
    /// may set it every frame.
    pub fn set_dynamic_hud(&mut self, choice: oag_display::display::DynamicHud) {
        let Some(view) = self.view.now_playing.as_mut() else {
            return;
        };
        view.hud_shown = !view.gated || choice.shows_hud();
        if !view.hud_shown {
            view.display.hide();
        }
    }

    /// One tick of the marquee.
    pub(super) fn advance_now_playing(&mut self) {
        if let Some(view) = self.view.now_playing.as_mut() {
            view.display.step();
        }
    }

    /// The marquee as this tick draws it.
    #[must_use]
    pub fn now_playing(&self) -> Option<oag_hud::now_playing::Marquee> {
        self.view.now_playing.as_ref()?.display.marquee()
    }
}
