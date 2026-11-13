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
    /// The ids of the teams the definition marks `multiplayer="true"`.
    pub multiplayer_teams: Vec<String>,
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
    let declared = oag_raceplay::catalogue::teams(&team_xml);
    let multiplayer_teams = declared
        .iter()
        .filter(|team| team.multiplayer)
        .map(|team| team.id.clone())
        .collect();
    let teams = declared
        .iter()
        .map(|team| menu::Choice::labelled(&team.id, team.label(&strings)))
        .collect();
    Ok(Catalogue {
        tracks,
        teams,
        multiplayer_teams,
    })
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

/// What a table-form `values_from` query is asked with: the value of the setting its `of`
/// names, and of its [`menu::Query::companion`] setting when the row gives one.
#[derive(Debug, Clone, Copy)]
pub struct QueryArgs<'a> {
    /// The title the `of` setting holds.
    pub title: &'a str,
    /// The companion setting's value: a craft title, a class or a team.
    pub companion: Option<&'a str>,
}

/// The list a table-form `values_from` query offers for `title`, whose catalogue is `catalogue`.
/// `None` for a query that takes no `of`. A `teams_of` answer is the catalogue's teams as
/// given: the caller builds the catalogue a craft title offers (its guest roster and class).
///
/// `variants_of` lists the variants of `team`, or of the catalogue's first team without one.
#[must_use]
pub fn query_choices(
    query: menu::Query,
    catalogue: &Catalogue,
    title: &'static oag_title::Title,
    team: Option<&str>,
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
            let team = team.unwrap_or_else(|| {
                catalogue
                    .teams
                    .first()
                    .map_or("", |team| team.value.as_str())
            });
            Some(crate::livery_row::variant_choices(title, team))
        }
        _ => None,
    }
}

/// Answers every table-form query on `model` from the values of the settings its `of` and
/// companion name, read off the rows that edit them (an engine setting or a `mod.<id>.<key>`
/// one alike). With `changed` set, only the queries that read that setting are re-listed.
/// `lists` maps a query and its arguments to its list, `None` when the title cannot be opened
/// here (the row keeps its list: an unreadable source is not evidence the pick is wrong).
pub fn resolve_queries(
    model: &mut menu::Menu,
    changed: Option<&str>,
    lists: impl FnMut(menu::Query, QueryArgs<'_>) -> Option<Vec<menu::Choice>>,
) {
    resolve_each(model, changed, lists, |_| {});
}

/// [`resolve_queries`] with `after` run on the model after each row is re-listed, before the
/// next query reads its arguments: a re-listed row falls to its first value, and `after` is
/// where a saved pick goes back on it so a query below reads the pick, not the first value.
fn resolve_each(
    model: &mut menu::Menu,
    changed: Option<&str>,
    mut lists: impl FnMut(menu::Query, QueryArgs<'_>) -> Option<Vec<menu::Choice>>,
    mut after: impl FnMut(&mut menu::Menu),
) {
    for query in model.query_refs() {
        let reads =
            |setting: &str| query.of == setting || query.companion.as_deref() == Some(setting);
        if changed.is_some_and(|setting| !reads(setting)) {
            continue;
        }
        let Some(title) = model.value_of(&query.of).map(str::to_owned) else {
            continue;
        };
        let companion = query
            .companion
            .as_deref()
            .and_then(|key| model.value_of(key))
            .map(str::to_owned);
        let args = QueryArgs {
            title: &title,
            companion: companion.as_deref(),
        };
        if let Some(list) = lists(query.query, args) {
            model.supply_query(&query, &list);
            after(model);
        }
    }
}

/// [`resolve_queries`], then a mod's saved picks (or a row's `default` while nothing is saved)
/// put back on the rows it re-listed and every
/// `mod.<id>.<key>` row's saved value brought into step with the row: a value its list no
/// longer offers becomes the one the row shows (the first), so what is saved is what a
/// launch reads. A pick that changes re-lists the queries that read it, until nothing moves.
/// A row with an empty list keeps its saved value. Returns whether anything saved changed.
pub fn settle_queries(
    model: &mut menu::Menu,
    mod_settings: &mut crate::settings::mod_settings::ModSettings,
    changed: Option<&str>,
    mut lists: impl FnMut(menu::Query, QueryArgs<'_>) -> Option<Vec<menu::Choice>>,
) -> bool {
    let mut moved = false;
    let mut pending: Vec<Option<String>> = vec![changed.map(str::to_owned)];
    // Each pass settles at least one more row of a finite page, so this bound is never the
    // reason it stops on a well-formed page; it only keeps a cycle of `of`s from spinning.
    for _ in 0..16 {
        let Some(changed) = pending.pop() else {
            break;
        };
        // A row's `default` stands for a pick its mod has never saved.
        let defaults: Vec<(String, String)> = model
            .choice_defaults()
            .into_iter()
            .filter(|(key, _)| crate::settings::mod_settings::get(mod_settings, key).is_none())
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect();
        let saved = crate::settings::mod_settings::seeds(mod_settings);
        let put_back = |model: &mut menu::Menu| {
            for (key, value) in &defaults {
                model.seed(key, &menu::Value::Text(value.clone()));
            }
            for (key, value) in &saved {
                model.seed(key, value);
            }
        };
        put_back(model);
        resolve_each(model, changed.as_deref(), &mut lists, put_back);
        let settled: Vec<(String, String)> = model
            .held_choices()
            .into_iter()
            .filter(|(key, value)| {
                crate::settings::mod_settings::split(key).is_some()
                    && crate::settings::mod_settings::get(mod_settings, key) != Some(*value)
            })
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect();
        for (key, value) in settled {
            crate::settings::mod_settings::set(mod_settings, &key, &value);
            moved = true;
            pending.push(Some(key));
        }
    }
    moved
}
