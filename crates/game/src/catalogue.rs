//! What a source offers to race on, read from the source.
//!
//! The menus need a list of circuits, and the two obvious ways to get one are
//! both wrong. Hard-coding names would put shipped content in this repository,
//! which [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md)
//! forbids. Listing `Data\Environments\*` would list *directories*, and a
//! directory is not a race: `16_Track` and `32_Track` are two entries on the
//! menu and one folder on the disc, the second being the first driven the other
//! way.
//!
//! So the list comes from where the original's own front end gets it:
//! `Data\Plugins\PI001\Definition.xml`, whose `PI_Track` nodes each carry an
//! id, the environment they load and whether they run reversed. The name a
//! player sees is the id looked up in the language's string table, so it is as
//! localised as the rest of the front end and never appears in this repository.
//!
//! ```text
//! <PI_Track name="32_Track">
//!   <Values type="Race" soundregister="2" location="Data\Environments\16_Track"
//!           Reversed="True" availableInZone="true"/>
//! </PI_Track>
//! ```
//!
//! Confidence **88**: the structure is read off the shipped file and every
//! entry on the PSP disc resolves to an environment that exists, but nothing
//! here has been watched running under an emulator, and what `soundregister`,
//! `Grid` and `availableInZone` select is not established. See
//! `docs/formats/fexml.md`.
//!
//! # Teams come from the same file, and so does downloadable content
//!
//! `PI_Team` sits beside `PI_Track` in that definition and is read the same
//! way, for the same reason: the roster is data. A [DLC
//! pack](../../../docs/formats/dlc-pack.md) declares its additions as a
//! *fragment of this exact schema*, so mounting one adds nothing to parse -
//! [`teams`] and [`tracks`] run over the pack's manifest unchanged, and
//! [`all_teams`] / [`all_tracks`] concatenate the results.
//!
//! ## An id is not a name
//!
//! `PI_Team name="Mantis"` is an **id**: it is the folder under `Data\Ships\`,
//! it is what a setting stores, and it is not what a player reads. The name on
//! screen comes from the string table, keyed by that id - the disc's own
//! `entries.xml` maps `Mantis` to the name the game shows for it, in every
//! language the release carries. Spelling either one here would put shipped
//! content in this repository; see
//! [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md).

use oag_formats::fexml::{Node, parse};

/// One thing a player can pick on the Race page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// The plugin's own id, e.g. `32_Track`. This is what a setting stores: it
    /// names the *race*, where a directory names only the geometry.
    pub id: String,
    /// The environment directory, e.g. `Data\Environments\16_Track`.
    pub location: String,
    /// Whether this entry is the circuit run the other way round.
    pub reversed: bool,
}

impl Track {
    /// The archive entry name of the `.vex` this race loads.
    ///
    /// The four-file rule from `docs/formats/track.md`: a circuit ships
    /// `track.vex`, `track_reversed.vex` and the two zone variants, and a
    /// `PI_Track` picks between the first two with `Reversed`. Zone entries are
    /// a separate mode this build has no way into yet, so nothing here selects
    /// them.
    #[must_use]
    pub fn entry_name(&self) -> String {
        let file = if self.reversed {
            "track_reversed"
        } else {
            "track"
        };
        format!(r"{}\{file}.vex", self.location)
    }
}

/// Reads every raceable `PI_Track` out of a plugin definition, in file order.
///
/// File order on purpose: it is the order the original's own front end would
/// walk, and it puts `16_Track` first, which is the circuit every capture under
/// `data/traces/` was taken on.
///
/// Entries whose `type` is not `Race` are skipped rather than guessed at - the
/// shipped file has none, so what another value would mean is unestablished and
/// silently racing one would be inventing behaviour.
#[must_use]
pub fn tracks(definition_xml: &str) -> Vec<Track> {
    let root = parse(definition_xml);
    let mut out = Vec::new();
    collect(&root, &mut out);
    out
}

fn collect(node: &Node, out: &mut Vec<Track>) {
    for child in &node.children {
        if child.name == "PI_Track" {
            if let Some(track) = read_track(child) {
                out.push(track);
            }
        } else {
            collect(child, out);
        }
    }
}

/// One team a player can race for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    /// The plugin's own id, e.g. `Feisar`. What a setting stores, what
    /// `--team` accepts, and the key the string table's display name is under -
    /// **not** the display name itself. See the module docs.
    pub id: String,
    /// The ship directory, e.g. `Data\Ships\Feisar`.
    pub location: String,
    /// The string-table id of the team's description, e.g. the `helpText`
    /// attribute. `None` for a team that declares none.
    pub help_text: Option<String>,
}

/// Reads every raceable `PI_Team` out of a plugin definition, in file order.
///
/// Alternate hulls (`PI_TeamModel`, `PI_ModelSkin` - the concept, zone and
/// unlockable liveries) are deliberately **not** read. They are declared right
/// here and would be easy to collect, but nothing draws a second hull yet, and
/// a public field no caller reads is worse than one that appears when it is
/// needed. See `HANDOVER.md`.
#[must_use]
pub fn teams(definition_xml: &str) -> Vec<Team> {
    let root = parse(definition_xml);
    let mut out = Vec::new();
    collect_teams(&root, &mut out);
    out
}

fn collect_teams(node: &Node, out: &mut Vec<Team>) {
    for child in &node.children {
        if child.name == "PI_Team" {
            if let Some(team) = read_team(child) {
                out.push(team);
            }
        } else {
            collect_teams(child, out);
        }
    }
}

fn read_team(node: &Node) -> Option<Team> {
    let id = node.attr("name")?.to_string();
    let values = node.children_named("Values").next()?;
    if values.attr("type").is_some_and(|kind| kind != "Race") {
        return None;
    }
    Some(Team {
        id,
        location: values.attr("location")?.to_string(),
        help_text: values.attr("helpText").map(str::to_string),
    })
}

/// Every team across `documents`, the first spelling of an id winning.
///
/// Pass the source's own `Definition.xml` first and each mounted pack's
/// manifest after it, which is the order
/// [`oag_pulse::open_with_packs`] mounts them in. First-wins
/// then means the same thing here as it does there: the disc is authoritative
/// and a pack can only add.
#[must_use]
pub fn all_teams(documents: &[String]) -> Vec<Team> {
    let mut out: Vec<Team> = Vec::new();
    for document in documents {
        for team in teams(document) {
            if !out.iter().any(|seen| seen.id == team.id) {
                out.push(team);
            }
        }
    }
    out
}

/// Every race across `documents`, the first spelling of an id winning.
///
/// The counterpart of [`all_teams`], with one caveat that matters for a partial
/// pack set: a `PI_Track` is only a *declaration*, and a player who owns one
/// pack of a set can hold a declaration for a circuit whose geometry is in a
/// pack they do not have. Callers filter on
/// [`oag_assets::Archives::locate`] of [`Track::entry_name`] for that
/// reason - the check belongs where the archives are, not here.
#[must_use]
pub fn all_tracks(documents: &[String]) -> Vec<Track> {
    let mut out: Vec<Track> = Vec::new();
    for document in documents {
        for track in tracks(document) {
            if !out.iter().any(|seen| seen.id == track.id) {
                out.push(track);
            }
        }
    }
    out
}

fn read_track(node: &Node) -> Option<Track> {
    let id = node.attr("name")?.to_string();
    let values = node.children_named("Values").next()?;
    if values.attr("type").is_some_and(|kind| kind != "Race") {
        return None;
    }
    Some(Track {
        id,
        location: values.attr("location")?.to_string(),
        // The shipped file spells it `Reversed="True"`, capitalised, which is
        // why this goes through `flag` rather than comparing to "true".
        reversed: values.flag("Reversed").unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Shaped like `Data\Plugins\PI001\Definition.xml`, with the two cases that
    /// matter next to each other: two entries sharing one environment, one of
    /// them reversed.
    const DEFINITION: &str = r#"
<Screen name="Top">
  <PI_Skin name="UI"><Values location="Data\Plugins\PI001\GUI" activate="true"/></PI_Skin>
  <PI_Track name="16_Track">
    <Values type="Race" soundregister="1" location="Data\Environments\16_Track"
            availableInZone="true"/>
  </PI_Track>
  <PI_Track name="32_Track">
    <Values type="Race" soundregister="2" location="Data\Environments\16_Track"
            Reversed="True" availableInZone="true"/>
    <Entry Label="Grid0"/>
  </PI_Track>
  <PI_Team name="Assegai">
    <Values type="Race" soundregister="4" location="Data\Ships\Assegai" helpText="MSC_TEAMDES_ASS"/>
  </PI_Team>
</Screen>
"#;

    /// Shaped like entry 0 of a `PACKn.edat`: the same schema, holding only
    /// what the pack adds. See `docs/formats/dlc-pack.md`.
    const PACK_MANIFEST: &str = r#"
<Screen name="Top">
  <PI_Team name="Ersatz">
    <Values type="Race" soundregister="11" multiplayer="true"
            location="Data\Ships\Ersatz" helpText="MSC_TEAMDES_ERS"/>
    <PI_TeamModel name="Normal"><Values location="ship"/></PI_TeamModel>
  </PI_Team>
  <PI_Track name="99_Track">
    <Values type="Race" soundregister="30" location="Data\Environments\99_Track"/>
  </PI_Track>
  <LoadXML><Values Src="download09.xml"/></LoadXML>
</Screen>
"#;

    #[test]
    fn two_entries_can_share_one_environment_and_differ_by_direction() {
        let tracks = tracks(DEFINITION);
        assert_eq!(tracks.len(), 2, "{tracks:?}");

        assert_eq!(tracks[0].id, "16_Track");
        assert!(!tracks[0].reversed);
        assert_eq!(
            tracks[0].entry_name(),
            r"Data\Environments\16_Track\track.vex",
            "and it is the same name race::DEFAULT_TRACK spells"
        );

        assert_eq!(tracks[1].id, "32_Track");
        assert!(tracks[1].reversed, "Reversed=\"True\" is capitalised");
        assert_eq!(tracks[1].location, tracks[0].location, "one environment");
        assert_eq!(
            tracks[1].entry_name(),
            r"Data\Environments\16_Track\track_reversed.vex"
        );
    }

    /// The id names the race and the directory names the geometry, and reading
    /// the directory as the race is the mistake this whole module exists to
    /// stop. Pinned as its own assertion because it is the thing a reader will
    /// not believe.
    #[test]
    fn the_id_is_not_the_directory() {
        let tracks = tracks(DEFINITION);
        assert!(
            !tracks[1].location.contains(&tracks[1].id),
            "{} loads {}",
            tracks[1].id,
            tracks[1].location
        );
    }

    #[test]
    fn nodes_that_are_not_tracks_are_left_alone() {
        assert!(tracks("<Screen name=\"Top\"><PI_Team name=\"Qirex\"/></Screen>").is_empty());
    }

    #[test]
    fn a_team_carries_its_ship_directory_and_its_description_key() {
        let teams = teams(DEFINITION);
        assert_eq!(teams.len(), 1, "{teams:?}");
        assert_eq!(teams[0].id, "Assegai");
        assert_eq!(teams[0].location, r"Data\Ships\Assegai");
        assert_eq!(teams[0].help_text.as_deref(), Some("MSC_TEAMDES_ASS"));
    }

    /// The id is the folder leaf, which is why `race::ship_entry_name` can go
    /// on formatting a path out of the id alone. It holds for every team on
    /// the disc and in every pack; `dlc_ground_truth` re-checks it against
    /// real content rather than trusting this fixture.
    #[test]
    fn a_team_id_is_the_leaf_of_its_directory() {
        for team in teams(DEFINITION) {
            assert!(
                team.location.ends_with(&format!(r"\{}", team.id)),
                "{} loads {}",
                team.id,
                team.location
            );
        }
    }

    /// A `PI_Team` with no `Values` names no ship directory, so there is
    /// nothing to load - skipped, like a `PI_Track` with no `location`.
    #[test]
    fn a_team_with_nothing_to_load_is_skipped() {
        assert!(teams(r#"<Screen name="Top"><PI_Team name="Qirex"/></Screen>"#).is_empty());
    }

    /// The whole point of the DLC path: a pack's manifest is this schema, so
    /// it needs no reader of its own.
    #[test]
    fn a_pack_manifest_parses_with_the_same_readers() {
        let teams = teams(PACK_MANIFEST);
        assert_eq!(teams.len(), 1, "{teams:?}");
        assert_eq!(teams[0].id, "Ersatz");
        assert_eq!(teams[0].location, r"Data\Ships\Ersatz");

        let tracks = tracks(PACK_MANIFEST);
        assert_eq!(tracks.len(), 1, "{tracks:?}");
        assert_eq!(
            tracks[0].entry_name(),
            r"Data\Environments\99_Track\track.vex"
        );
    }

    #[test]
    fn a_pack_adds_to_the_source_without_replacing_any_of_it() {
        let documents = vec![DEFINITION.to_string(), PACK_MANIFEST.to_string()];

        let teams = all_teams(&documents);
        assert_eq!(
            teams.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["Assegai", "Ersatz"],
            "the source's roster first, then what the pack adds"
        );

        let tracks = all_tracks(&documents);
        assert_eq!(
            tracks.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
            ["16_Track", "32_Track", "99_Track"]
        );
    }

    /// A pack that redeclared a team the disc already has must not shadow it.
    /// No shipped pack does, and this is what keeps it that way.
    #[test]
    fn the_source_wins_when_a_pack_redeclares_an_id() {
        let shadow = r#"
<Screen name="Top">
  <PI_Team name="Assegai">
    <Values type="Race" location="Data\Ships\Somewhere_Else"/>
  </PI_Team>
</Screen>
"#;
        let teams = all_teams(&[DEFINITION.to_string(), shadow.to_string()]);
        assert_eq!(teams.len(), 1);
        assert_eq!(teams[0].location, r"Data\Ships\Assegai");
    }

    /// A `PI_Track` with no `location` names no environment, so there is
    /// nothing to load; skipped rather than turned into a path that will fail
    /// at the archive with a confusing message.
    #[test]
    fn an_entry_with_nothing_to_load_is_skipped() {
        let tracks = tracks(
            r#"<Screen name="Top"><PI_Track name="99_Track"><Values type="Race"/></PI_Track></Screen>"#,
        );
        assert!(tracks.is_empty());
    }
}
