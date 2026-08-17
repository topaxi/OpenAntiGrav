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
  <PI_Music name="A Piece Of Music">
    <Values location="Data\Music\SomeArtist"></Values>
    <Entry Artist="Some Artist"></Entry>
    <Entry Label="Some Label"></Entry>
  </PI_Music>
  <PI_Music name="Another Piece">
    <Values location="Data\Music\OtherArtist"></Values>
    <Entry Artist="Other Artist"></Entry>
    <Entry Label=""></Entry>
  </PI_Music>
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

/// The soundtrack is declared beside the circuits, in the same schema and
/// in the order the disc's own music screen walks. **The fixture's titles
/// and artists are invented**, unlike its `16_Track` and `Assegai` ids,
/// which are folder names: an id is not shipped text and a song title is.
#[test]
fn the_soundtrack_is_declared_in_file_order_and_addressed_by_location() {
    let music = music(DEFINITION);
    assert_eq!(music.len(), 2, "{music:?}");

    assert_eq!(music[0].location, r"Data\Music\SomeArtist");
    assert_eq!(
        music[0].entry_name("music.at3"),
        r"Data\Music\SomeArtist\music.at3",
        "MusicManager.cpp's own `%s\\%s` join"
    );
    assert_eq!(music[1].location, r"Data\Music\OtherArtist");

    // An `Entry` sitting beside `Values` must not be mistaken for one: the
    // node's own `name` is the id, and `Values` is the only child read.
    assert_ne!(music[0].id, music[1].id);
}

/// A definition with no `PI_Music` at all yields nothing rather than
/// guessing - the case a caller distinguishes to fall back on finding the
/// entries by what they are.
#[test]
fn a_definition_that_declares_no_music_yields_none() {
    assert!(music(PACK_MANIFEST).is_empty());
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
