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
  <PI_Team name="Assegai"><Values loyalty="1"/></PI_Team>
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
