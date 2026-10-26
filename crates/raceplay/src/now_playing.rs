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
    serial: u32,
    /// A song that started before the lights went green, shown when they do.
    waiting: Option<(String, f32)>,
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
            "now playing: {} bar {:?}, text from x {} clipped to {:?}",
            authored.item, geometry.bar, geometry.origin_x, geometry.clip
        ));
        Some(Self {
            display: Display::new(geometry, &authored),
            font: hud.small_font.clone(),
            separator: authored.separator,
            serial: 0,
            waiting: None,
        })
    }
}

impl Race {
    /// The race music started a song (or has not yet): shows its name the way the title does,
    /// once per song. `None` for a song the disc does not name, which shows nothing.
    pub fn note_song(&mut self, song: Option<&oag_sound::NowPlaying>) {
        let Some(view) = self.view.now_playing.as_mut() else {
            return;
        };
        let Some(song) = song.filter(|song| song.serial != view.serial) else {
            return;
        };
        view.serial = song.serial;
        if let Some(name) = &song.name {
            let text = name.text(view.separator);
            let width = oag_ui::font::measure(&view.font, &text) * view.display.scale();
            view.waiting = Some((text, width));
        }
    }

    /// One tick of the marquee.
    ///
    /// A song shows from the green light on: the original's `MusicPlayer_StartNextSong` only
    /// stores the pick while the race manager is in its intro or countdown states (0 and 1) and
    /// starts the song, and the display with it, in state 2. This build's music starts earlier,
    /// so the display waits for the light and the sound does not.
    pub(super) fn advance_now_playing(&mut self) {
        let racing = !oag_race::RaceState::thrust_gated(self.sim.world.tick);
        if let Some(view) = self.view.now_playing.as_mut() {
            if racing && let Some((text, width)) = view.waiting.take() {
                view.display.show(text, width);
            }
            view.display.step();
        }
    }

    /// The marquee as this tick draws it.
    #[must_use]
    pub fn now_playing(&self) -> Option<oag_hud::now_playing::Marquee> {
        self.view.now_playing.as_ref()?.display.marquee()
    }
}
