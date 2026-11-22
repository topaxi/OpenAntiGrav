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

#[cfg(feature = "view")]
use crate::RaceView;
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
    /// The title being played: the only author whose names the string table can hold a key for.
    played: &'static str,
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
            played: title.name,
            serial: 0,
            gated: title.hud_art.dynamic_hud.is_some(),
            hud_shown: true,
        })
    }
}

impl View {
    /// The text of `song`'s name, or `None` when it has none; see [`song_text`].
    fn text(&self, song: &oag_sound::NowPlaying) -> Option<String> {
        song_text(
            &self.strings,
            self.title_key,
            self.played,
            self.separator,
            song,
        )
    }

    /// Starts the marquee on `song`'s name; nothing for a song nothing names.
    fn show(&mut self, song: &oag_sound::NowPlaying) {
        if let Some(text) = self.text(song) {
            let width = oag_ui::font::measure(&self.font, &text) * self.display.scale();
            self.display.show(text, width);
        }
    }
}

/// The marquee's text for `song`.
///
/// A disc track nothing names reads through the string table (`OAG_HUD_TRACK_N`). The title's
/// `title_key` applies only to a name the title being played authored, the one name the table
/// can hold a key for; a key it lacks shows the name's own title. Any other name is shown as it
/// is. **Chosen, not measured.**
fn song_text(
    strings: &oag_ui::language::StringTable,
    title_key: Option<&str>,
    played: &str,
    separator: &str,
    song: &oag_sound::NowPlaying,
) -> Option<String> {
    let name = song.name.as_ref()?;
    let own = song.author == Some(played);
    let title = match (song.track, title_key) {
        (Some(number), _) => strings.get("OAG_HUD_TRACK_N").map_or_else(
            || name.title.clone(),
            |text| text.replace("{}", &number.to_string()),
        ),
        (None, Some(key)) if own => strings
            .get(&key.replace("{}", &name.title))
            .and_then(|text| text.lines().last())
            .map_or_else(|| name.title.clone(), str::to_string),
        (None, _) => name.title.clone(),
    };
    Some(match &name.artist {
        Some(artist) => format!("{artist}{separator}{title}"),
        None => title,
    })
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
        view.show(song);
    }

    /// Shows the song the music is on again, on the player's request. Unlike [`Self::note_song`]
    /// it shows a song already shown; it still shows nothing while the HUD is off, for a song
    /// the disc does not name, or when the title has no display.
    pub fn show_song_again(&mut self, song: Option<&oag_sound::NowPlaying>) {
        let Some(view) = self.view.now_playing.as_mut() else {
            return;
        };
        let Some(song) = song else {
            return;
        };
        view.serial = song.serial;
        if view.hud_shown {
            view.show(song);
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

    /// The marquee as this tick draws it. **None in a split screen**: HD's
    /// `RaceManager_ShowNowPlaying` shows the display only while `g_GameState+0xe4`, the count of
    /// human entrants (1 in single player, 2 in split screen), is 1, and its split-screen layouts
    /// author no such item; Omega and 2048 follow it, Pulse's and Pure's originals have no split
    /// screen to read (`docs/formats/now-playing.md`). The song's state keeps stepping, so a race
    /// that goes back to one view shows what is left of it.
    #[must_use]
    pub fn now_playing(&self) -> Option<oag_hud::now_playing::Marquee> {
        if self.view.viewport_count() > 1 {
            return None;
        }
        self.view.now_playing.as_ref()?.display.marquee()
    }
}

#[cfg(feature = "view")]
impl RaceView {
    /// One tick of the marquee.
    pub(super) fn advance_now_playing(&mut self) {
        if let Some(view) = self.now_playing.as_mut() {
            view.display.step();
        }
    }
}

#[cfg(test)]
mod tests {
    use oag_ui::language::StringTable;

    use super::song_text;

    fn song(
        title: &str,
        author: Option<&'static str>,
        track: Option<usize>,
    ) -> oag_sound::NowPlaying {
        oag_sound::NowPlaying {
            serial: 1,
            started: 1,
            name: Some(oag_sound::SongName {
                artist: Some("Artist".to_string()),
                title: title.to_string(),
            }),
            author,
            track,
        }
    }

    fn table() -> StringTable {
        StringTable::from_xml(
            r#"<Entry ID="MT_01" String="Artist
Keyed Title"></Entry>
               <Entry ID="OAG_HUD_TRACK_N" String="Track {}"></Entry>"#,
        )
    }

    #[test]
    fn the_key_applies_to_the_played_titles_own_name_only() {
        let text = |song| song_text(&table(), Some("MT_{}"), "Wipeout 2048", " - ", &song);
        assert_eq!(
            text(song("01", Some("Wipeout 2048"), None)).as_deref(),
            Some("Artist - Keyed Title")
        );
        assert_eq!(
            text(song("01", Some("Wipeout HD"), None)).as_deref(),
            Some("Artist - 01"),
            "another title's name is shown as it is"
        );
        assert_eq!(text(song("01", None, None)).as_deref(), Some("Artist - 01"));
    }

    #[test]
    fn a_key_the_table_lacks_falls_back_to_the_raw_title() {
        let text = song_text(
            &table(),
            Some("MT_{}"),
            "Wipeout 2048",
            " - ",
            &song("09", Some("Wipeout 2048"), None),
        );
        assert_eq!(text.as_deref(), Some("Artist - 09"));
    }

    #[test]
    fn a_disc_track_nothing_names_is_worded_by_the_table() {
        let named = song("Track 3", None, Some(3));
        assert_eq!(
            song_text(
                &StringTable::from_xml(""),
                None,
                "Wipeout Pulse",
                " - ",
                &named
            )
            .as_deref(),
            Some("Artist - Track 3"),
            "no row: the English wording it carries"
        );
        let mut german = StringTable::from_xml("");
        german.merge([("OAG_HUD_TRACK_N".to_string(), "Titel {}".to_string())].into());
        assert_eq!(
            song_text(&german, None, "Wipeout Pulse", " - ", &named).as_deref(),
            Some("Artist - Titel 3")
        );
    }
}
