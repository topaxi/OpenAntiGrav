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
    if track_definition != definition {
        plugin_xml(&mut archives, source, track_definition)?;
    }

    let language_plugins = title.front_end.map_or::<&[&str], _>(&[], |front_end| {
        front_end.offered_languages(archives.layout.serial.as_deref())
    });
    let languages = oag_ui::language::load::load_languages(
        &mut archives,
        language_plugins,
        title.front_end.and_then(|front_end| front_end.disc_strings),
        &mut report,
    );
    let language = oag_ui::language::load::chosen_language(&languages, None);
    let strings =
        oag_ui::language::load::load_strings(&mut archives, &languages, None, &mut report);

    // The circuits the boot's own catalogue offers: the definitions and what a
    // mounted add-on package declares, minus a declared circuit whose geometry
    // is in a package the player does not own.
    let documents =
        oag_raceplay::catalogue::documents(&mut archives, &[definition, track_definition]);
    let track_list: Vec<_> = oag_raceplay::catalogue::all_tracks(&documents)
        .into_iter()
        .filter(|track| archives.locate(&track.entry_name()).is_some())
        .collect();
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

/// The list a table-form `values_from` query offers for `title`, whose catalogue is `catalogue`.
/// `None` for a query that takes no `of`.
///
/// `variants_of` is scoped to the catalogue's first team: the argument is one setting, so the
/// team cannot also be named, and a title whose variants are hull stems rather than team
/// suffixes answers the same for every team.
#[must_use]
pub fn query_choices(
    query: menu::Query,
    catalogue: &Catalogue,
    title: &'static oag_title::Title,
) -> Option<Vec<menu::Choice>> {
    match query {
        menu::Query::TracksOf => Some(
            catalogue
                .tracks
                .iter()
                .map(|(track, name)| menu::Choice::labelled(&track.id, name))
                .collect(),
        ),
        menu::Query::TeamsOf => Some(catalogue.teams.clone()),
        menu::Query::VariantsOf => {
            let team = catalogue
                .teams
                .first()
                .map_or("", |team| team.value.as_str());
            Some(crate::livery_row::variant_choices(title, team))
        }
        _ => None,
    }
}

/// Answers every table-form query on `model` from the value of the setting its `of` names, read
/// off the row that edits it (an engine setting or a `mod.<id>.<key>` one alike). With
/// `changed` set, only the queries whose `of` is that setting are re-listed. `lists` maps a
/// query and a title name to its list, `None` when the title cannot be opened here (the row
/// keeps its list: an unreadable source is not evidence the pick is wrong).
pub fn resolve_queries(
    model: &mut menu::Menu,
    changed: Option<&str>,
    mut lists: impl FnMut(menu::Query, &str) -> Option<Vec<menu::Choice>>,
) {
    for query in model.query_refs() {
        if changed.is_some_and(|setting| setting != query.of) {
            continue;
        }
        let Some(title) = model.value_of(&query.of).map(str::to_owned) else {
            continue;
        };
        if let Some(list) = lists(query.query, &title) {
            model.supply_query(&query, &list);
        }
    }
}
