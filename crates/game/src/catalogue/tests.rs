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
/// on formatting a path out of the id alone.
///
/// **True by construction now, and worth keeping anyway.** It used to be true
/// of Pulse by luck - every team's declared name happened to equal its folder -
/// and false on Wipeout Pure, which declares `name="AG Systems"` against
/// `location="Data\Ships\AG_Systems"` on both pressings and so lost a founding
/// team from its roster. `read_team` reads the folder now and keeps the
/// declared name as `Team::name`, which is the split `Mantis`/`Mirage` already
/// needed. This asserts the property a future parser change could quietly
/// break; `roster_declared_ground_truth` asserts it against real discs.
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
///
/// **Redeclaring is now same-folder rather than same-name**, since the id is
/// the folder: this fixture used to name `Assegai` over
/// `Data\Ships\Somewhere_Else`, which is two different ships wearing one name
/// and is covered separately below.
#[test]
fn the_source_wins_when_a_pack_redeclares_an_id() {
    let shadow = r#"
<Screen name="Top">
  <PI_Team name="Assegai">
    <Values type="Race" location="Data\Ships\Assegai" helpText="MSC_PACK_OVERRIDE"/>
  </PI_Team>
</Screen>
"#;
    let teams = all_teams(&[DEFINITION.to_string(), shadow.to_string()]);
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0].location, r"Data\Ships\Assegai");
    assert_eq!(
        teams[0].help_text.as_deref(),
        Some("MSC_TEAMDES_ASS"),
        "the disc's own declaration wins the whole row, not just the location"
    );
}

/// Two folders under one declared name are two teams, not one.
///
/// The other half of the id-is-the-folder rule. Nothing ships this, but the
/// old rule would have collapsed the pair into whichever came first, and
/// silently: a ship folder that no path is ever built for is a team the player
/// cannot race.
#[test]
fn one_name_over_two_folders_is_two_teams() {
    let elsewhere = r#"
<Screen name="Top">
  <PI_Team name="Assegai">
    <Values type="Race" location="Data\Ships\Somewhere_Else"/>
  </PI_Team>
</Screen>
"#;
    let teams = all_teams(&[DEFINITION.to_string(), elsewhere.to_string()]);
    assert_eq!(
        teams.iter().map(|t| t.id.as_str()).collect::<Vec<_>>(),
        ["Assegai", "Somewhere_Else"]
    );
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

/// One environment named twice gets the disc's own reverse marker on the
/// second row, because otherwise the RACE page shows the same word twice.
///
/// The shape Wipeout HD's own tables are in: `01_Track` and `09_Track` are one
/// piece of track each way round and the copy that pairs with its circuit list
/// reads `VINETA K` for both.
#[test]
fn a_reversed_circuit_named_the_same_as_its_twin_gets_the_discs_reverse_marker() {
    let tracks = tracks(DEFINITION);
    let strings = crate::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="16_Track" String="VINETA K"/>
             <Entry ID="32_Track" String="VINETA K"/>
             <Entry ID="FE_REVERSE" String="RÜCKWÄRTS"/>
           </StringTable>"#,
    );
    let names = crate::language::CircuitNames::default();

    assert_eq!(label(&tracks[0], &names, &strings, &tracks), "VINETA K");
    assert_eq!(
        label(&tracks[1], &names, &strings, &tracks),
        "VINETA K RÜCKWÄRTS"
    );
}

/// Where the table names the two directions separately, nothing is appended.
///
/// Which is every circuit on both PSP titles, and the reason the marker keys on
/// the names colliding rather than on `Reversed="True"`.
#[test]
fn a_reversed_circuit_with_a_name_of_its_own_is_left_alone() {
    let tracks = tracks(DEFINITION);
    let strings = crate::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="16_Track" String="Talon's Junction White"/>
             <Entry ID="32_Track" String="Talon's Junction Black"/>
             <Entry ID="FE_REVERSE" String="REVERSE"/>
           </StringTable>"#,
    );
    let names = crate::language::CircuitNames::default();

    assert_eq!(
        label(&tracks[1], &names, &strings, &tracks),
        "Talon's Junction Black"
    );
}

/// A circuit nothing names shows its id, and never a marker on its own.
#[test]
fn an_unnamed_circuit_shows_its_id() {
    let tracks = tracks(DEFINITION);
    let strings = crate::language::StringTable::default();
    let names = crate::language::CircuitNames::default();

    assert_eq!(label(&tracks[0], &names, &strings, &tracks), "16_Track");
    // The two ids differ, so the twin check does not fire - and even if it did,
    // there is no `FE_REVERSE` to append.
    assert_eq!(label(&tracks[1], &names, &strings, &tracks), "32_Track");
}

/// The chosen copy wins over the served one, which is the whole point of
/// resolving circuits separately.
#[test]
fn a_chosen_copy_beats_the_served_table() {
    let tracks = tracks(DEFINITION);
    let served = crate::language::StringTable::from_xml(
        r#"<StringTable><Entry ID="16_Track" String="THE WRONG ONE"/></StringTable>"#,
    );
    let chosen = crate::language::StringTable::from_xml(
        r#"<StringTable><entry id="16_TRACK" string="VINETA K"/>
                        <entry id="32_TRACK" string="VINETA K"/></StringTable>"#,
    );
    let ids: Vec<String> = tracks.iter().map(|track| track.id.clone()).collect();
    let names = crate::language::CircuitNames::choose(&[("DATA06".to_string(), chosen)], &ids)
        .expect("that copy names both circuits, case folded");

    assert_eq!(label(&tracks[0], &names, &served, &tracks), "VINETA K");
}
