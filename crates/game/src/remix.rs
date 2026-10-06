//! What the RACE REMIX page offers: the circuits and the roster a title has, read
//! through [`oag_source::remix::Remix`]'s source pair rather than the front end's boot.
//!
//! The source pair itself - which loader reads track content and which reads craft
//! content - is `oag_source::remix`; this half is the page's own pickers.

use anyhow::Context;

use oag_ui::menu;

/// The circuits and the roster a title offers, for the RACE REMIX page's
/// title-scoped pickers.
///
/// Tracks carry the label alongside the full [`oag_raceplay::catalogue::Track`]
/// rather than as a `menu::Choice` outright, the same asymmetry the
/// composition root's own menu shell carries for the ordinary RACE page:
/// launching needs [`oag_raceplay::catalogue::Track::entry_name`], which only the
/// whole record can answer, where a team's stored value *is* its id and a
/// `Choice` already carries everything launching needs.
#[derive(Debug)]
pub struct Catalogue {
    pub tracks: Vec<(oag_raceplay::catalogue::Track, String)>,
    pub teams: Vec<menu::Choice>,
}

impl Catalogue {
    /// Which circuit a stored id names, if this title still offers it - the
    /// same rule the composition root's own menu shell applies for the
    /// ordinary RACE page, scoped to whichever title this catalogue was
    /// built from rather than the one this process booted from.
    #[must_use]
    pub fn track(&self, id: &str) -> Option<&oag_raceplay::catalogue::Track> {
        self.tracks
            .iter()
            .map(|(track, _)| track)
            .find(|track| track.id == id)
    }

    /// Whether a stored id is one this title's roster offers, and its id.
    #[must_use]
    pub fn team(&self, id: &str) -> Option<&str> {
        self.teams
            .iter()
            .find(|choice| choice.value == id)
            .map(|choice| choice.value.as_str())
    }
}

/// Opens `source` far enough to answer "what can this title race": its
/// plugin definition, one language's string table and its circuit names,
/// nothing else - not a full menu shell, which additionally builds a font
/// atlas, a sprite sheet and a menu frame off that title's own front end. A
/// remix picker draws in the *booted* title's chrome regardless of which
/// title TRACK TITLE or CRAFT TITLE names, so none of that is needed here.
///
/// **A track's real name is never simply `strings.get(&track.id)`.**
/// `crate::boot::roster::load_circuit_names` exists precisely because a circuit's
/// name lives in its own column of the string table, at a key the id does
/// not predict - on Wipeout HD least of all, where a direct lookup by id
/// resolves nothing and every track fell back to its raw id (`01_Track`) on
/// this page until this was wired in. See [`oag_raceplay::catalogue::label`], which
/// also settles a reversed circuit against its forward twin.
///
/// # Errors
///
/// Propagates a source this build cannot open and a plugin definition that
/// will not read or parse.
pub fn catalogue(source: &str) -> anyhow::Result<Catalogue> {
    let opened = oag_source::title::open_source(source, Vec::new(), Vec::new())
        .with_context(|| format!("opening {source}"))?;
    let title = opened.title;
    let mut archives = opened.archives;
    let mut report = Vec::new();

    let definition = title.plugin_definition;
    let team_xml = plugin_xml(&mut archives, source, definition)?;

    // **Wipeout 2048 alone splits `PI_Track` into its own file** -
    // `title.plugin_definition` names the teams one; see
    // `oag_title::Title::track_plugin_definition`'s own doc comment for why
    // one field is not enough there. `None` on every other title, so this is
    // the same read twice on Pulse/Pure/HD, not a second archive open.
    let track_definition = title.track_plugin_definition.unwrap_or(definition);
    let track_xml = if track_definition == definition {
        team_xml.clone()
    } else {
        plugin_xml(&mut archives, source, track_definition)?
    };

    let language_plugins = title.front_end.map_or::<&[&str], _>(&[], |front_end| {
        front_end.offered_languages(archives.layout.serial.as_deref())
    });
    let languages =
        oag_ui::language::load::load_languages(&mut archives, language_plugins, &mut report);
    let language = oag_ui::language::load::chosen_language(&languages, None);
    let strings =
        oag_ui::language::load::load_strings(&mut archives, &languages, None, &mut report);

    let track_list = oag_raceplay::catalogue::tracks(&track_xml);
    let circuit_names = crate::boot::roster::load_circuit_names(
        &mut archives,
        language,
        &strings,
        &track_list,
        &mut report,
    );
    let tracks = track_list
        .iter()
        .map(|track| {
            let label =
                oag_raceplay::catalogue::label(track, &circuit_names, &strings, &track_list);
            (track.clone(), label)
        })
        .collect();
    let teams = oag_raceplay::catalogue::teams(&team_xml)
        .iter()
        .map(|team| menu::Choice::labelled(&team.id, team.label(&strings)))
        .collect();
    Ok(Catalogue { tracks, teams })
}

/// Reads and parses one plugin definition, named in every error so a bad path
/// says which of the (possibly two) files it was.
fn plugin_xml(
    archives: &mut oag_assets::Archives,
    source: &str,
    definition: &str,
) -> anyhow::Result<String> {
    let blob = archives
        .read_name(definition)
        .with_context(|| format!("reading {definition} out of {source}"))?;
    oag_tables::fexml::text(&blob).map_err(|e| anyhow::anyhow!("{definition}: {e}"))
}
