//! Where a race's track and its craft come from: the same source for both in
//! the ordinary case, two different ones for a Race Remix.
//!
//! Lives beside [`crate::title`] rather than in a separate crate: it needs the
//! same concrete knowledge of every title package plus [`oag_assets::Archives`]
//! that `title.rs` already centralises, which is composition-root-shaped, not
//! library-shaped - `oag-title` itself stays deliberately types-only, no
//! title's data, and a `Title` value assumes one archive-candidate source, so
//! it is not the right shape to *represent* a remix, only to be one of the
//! two things a remix holds.
//!
//! Nothing here is a new abstraction over `Title` - two [`Opened`] values
//! coexisting is unremarkable, plain owned data with no global state
//! (`crate::prefetch`'s `Vec<oag_assets::Archive>` already does the same for
//! one title). What is new is naming the split: which loader reads track
//! content (collision, environment, pads, weapon tuning) and which reads
//! craft content (livery, HUD, exhaust/flare, handling stats, boost plume) -
//! see `race/load.rs`.

use anyhow::Context;

use crate::menu;
use crate::title::Opened;
use oag_assets::{Result, dlc::Pack};
use oag_title::Title;

/// A race's track and craft source, together.
#[derive(Debug)]
pub enum Remix {
    /// Track and craft load from the same opened source - today's behaviour,
    /// and the only case before Race Remix existed.
    Single(Opened),
    /// A Race Remix: track and craft come from different sources.
    Split { track: Opened, craft: Opened },
}

impl Remix {
    /// Opens `source` for the track, and `craft_source` for the craft when it
    /// names something other than `source`. `packs`/`pure_packs` mount
    /// behind `source` only - a remix does not widen what a DLC pack may be
    /// mounted behind, on the same terms [`crate::title::open_source`]
    /// already applies to an ordinary race.
    pub fn open(
        source: &str,
        craft_source: Option<&str>,
        packs: Vec<Pack>,
        pure_packs: Vec<Pack>,
    ) -> Result<Self> {
        let track = crate::title::open_source(source, packs, pure_packs)?;
        match craft_source {
            Some(craft_source) if craft_source != source => {
                let craft = crate::title::open_source(craft_source, Vec::new(), Vec::new())?;
                Ok(Self::Split { track, craft })
            }
            _ => Ok(Self::Single(track)),
        }
    }

    /// Splits into the track's opened source, the craft's own when this is a
    /// genuine remix (`None` in the ordinary single-source case), and both
    /// titles - track first, craft second, equal to each other when `Single`.
    ///
    /// Returned by value rather than through `&mut self` accessors on
    /// purpose: `race::load` reads both `archives` on and off throughout its
    /// whole body, and a borrow taken from a method call would have to stay
    /// live for that entire span, which rules out ever calling the other
    /// accessor again. Two owned locals - `archives` and `Option<Archives>` -
    /// have no such conflict.
    #[must_use]
    pub fn into_parts(self) -> (Opened, Option<Opened>, &'static Title, &'static Title) {
        match self {
            Self::Single(opened) => {
                let title = opened.title;
                (opened, None, title, title)
            }
            Self::Split { track, craft } => {
                let (track_title, craft_title) = (track.title, craft.title);
                (track, Some(craft), track_title, craft_title)
            }
        }
    }
}

/// `craft_archives` when this is a genuine remix, `archives` itself
/// otherwise - the one line every craft-governed load in `race/load.rs`
/// wraps its archive argument in, for the same reason [`Remix::into_parts`]
/// returns owned values rather than borrows: two short-lived reborrows here
/// beat one long-lived one there.
pub fn craft_of<'a>(
    craft_archives: &'a mut Option<oag_assets::Archives>,
    archives: &'a mut oag_assets::Archives,
) -> &'a mut oag_assets::Archives {
    craft_archives.as_mut().unwrap_or(archives)
}

/// The circuits and the roster a title offers, for the RACE REMIX page's
/// title-scoped pickers.
///
/// Tracks carry the label alongside the full [`crate::catalogue::Track`]
/// rather than as a `menu::Choice` outright, the same asymmetry the
/// composition root's own menu shell carries for the ordinary RACE page:
/// launching needs [`crate::catalogue::Track::entry_name`], which only the
/// whole record can answer, where a team's stored value *is* its id and a
/// `Choice` already carries everything launching needs.
#[derive(Debug)]
pub struct Catalogue {
    pub tracks: Vec<(crate::catalogue::Track, String)>,
    pub teams: Vec<menu::Choice>,
}

impl Catalogue {
    /// Which circuit a stored id names, if this title still offers it - the
    /// same rule the composition root's own menu shell applies for the
    /// ordinary RACE page, scoped to whichever title this catalogue was
    /// built from rather than the one this process booted from.
    #[must_use]
    pub fn track(&self, id: &str) -> Option<&crate::catalogue::Track> {
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
/// this page until this was wired in. See [`crate::catalogue::label`], which
/// also settles a reversed circuit against its forward twin.
///
/// # Errors
///
/// Propagates a source this build cannot open and a plugin definition that
/// will not read or parse.
pub fn catalogue(source: &str) -> anyhow::Result<Catalogue> {
    let opened = crate::title::open_source(source, Vec::new(), Vec::new())
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

    let language_plugins = title
        .front_end
        .map_or::<&[&str], _>(&[], |front_end| front_end.language_plugins);
    let languages = crate::boot::load_languages(&mut archives, language_plugins, &mut report);
    let language = crate::boot::chosen_language(&languages, None);
    let strings = crate::boot::load_strings(&mut archives, &languages, None, &mut report);

    let track_list = crate::catalogue::tracks(&track_xml);
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
            let label = crate::catalogue::label(track, &circuit_names, &strings, &track_list);
            (track.clone(), label)
        })
        .collect();
    let teams = crate::catalogue::teams(&team_xml)
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
