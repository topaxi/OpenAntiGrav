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
    <FE speed="8" thrust="8" handling="9" shield="7"/>
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

/// The `<FE>` element is the front end's own rating table - see
/// `Team::rating` for why it is the source of the `Team Selection` bars.
#[test]
fn a_team_carries_the_four_ratings_its_fe_element_authors() {
    let disc = teams(DEFINITION);
    assert_eq!(
        disc[0].rating,
        Some(Rating {
            speed: 8,
            thrust: 8,
            handling: 9,
            shield: 7
        })
    );
    let pack = teams(PACK_MANIFEST);
    assert_eq!(
        pack[0].rating, None,
        "a manifest with no <FE> rates nothing"
    );
}

#[test]
fn a_team_carries_its_ship_directory_and_its_description_key() {
    let teams = teams(DEFINITION);
    assert_eq!(teams.len(), 1, "{teams:?}");
    assert_eq!(teams[0].id, "Assegai");
    assert_eq!(teams[0].location, r"Data\Ships\Assegai");
    assert_eq!(teams[0].help_text.as_deref(), Some("MSC_TEAMDES_ASS"));
    assert!(
        teams[0].skins.is_empty(),
        "a team declaring no PI_TeamModel declares no skin either"
    );
}

/// The `PI_ModelSkin` schema `docs/formats/dlc-pack.md` measured, read the
/// way the loader reads it: both skins under `Normal`, in file order, each
/// keeping the **full path** its own `location` spells.
#[test]
fn a_team_carries_the_skins_its_normal_model_declares() {
    const WITH_SKINS: &str = r#"<Screen name="Top">
  <PI_Team name="Ersatz">
    <Values type="Race" location="Data\Ships\Ersatz"/>
    <PI_TeamModel name="Normal">
      <Values location="ship"/>
      <PI_ModelSkin name="Alternative">
        <Values location="Data\Ships\Ersatz\ship_alt.dat"/>
        <Unlock Team="Ersatz" loyalty="10" Exclusive="true"/>
      </PI_ModelSkin>
      <PI_ModelSkin name="Eliminator">
        <Values location="Data\Ships\Ersatz\ship_eliminator.dat"/>
      </PI_ModelSkin>
    </PI_TeamModel>
  </PI_Team>
</Screen>"#;
    let teams = teams(WITH_SKINS);
    let skins = &teams[0].skins;
    assert_eq!(skins.len(), 2, "{skins:?}");
    assert_eq!(skins[0].name, "Alternative");
    assert_eq!(skins[0].location, r"Data\Ships\Ersatz\ship_alt.dat");
    assert_eq!(skins[1].name, "Eliminator");
    assert_eq!(
        teams[0]
            .skin("ELIMINATOR")
            .map(|skin| skin.location.as_str()),
        Some(r"Data\Ships\Ersatz\ship_eliminator.dat"),
        "the name a player types is matched case-insensitively"
    );
    assert!(teams[0].skin("Concept").is_none());
}

/// **Only `Normal`.** `Concept` and `Zone` are single unlockable models in
/// their own right on every source measured, and a skin nested under either
/// would be a shape nothing has seen - collecting it would quietly widen a
/// measured schema. See `docs/formats/dlc-pack.md`.
#[test]
fn a_skin_under_a_variant_that_is_not_normal_is_not_collected() {
    const ODD: &str = r#"<Screen name="Top">
  <PI_Team name="Ersatz">
    <Values type="Race" location="Data\Ships\Ersatz"/>
    <PI_TeamModel name="Concept">
      <Values location="extra"/>
      <PI_ModelSkin name="Alternative">
        <Values location="Data\Ships\Ersatz\ship_alt.dat"/>
      </PI_ModelSkin>
    </PI_TeamModel>
  </PI_Team>
</Screen>"#;
    assert!(teams(ODD)[0].skins.is_empty());
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
    let strings = oag_ui::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="16_Track" String="VINETA K"/>
             <Entry ID="32_Track" String="VINETA K"/>
             <Entry ID="FE_REVERSE" String="RÜCKWÄRTS"/>
           </StringTable>"#,
    );
    let names = oag_ui::language::CircuitNames::default();

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
    let strings = oag_ui::language::StringTable::from_xml(
        r#"<StringTable>
             <Entry ID="16_Track" String="Talon's Junction White"/>
             <Entry ID="32_Track" String="Talon's Junction Black"/>
             <Entry ID="FE_REVERSE" String="REVERSE"/>
           </StringTable>"#,
    );
    let names = oag_ui::language::CircuitNames::default();

    assert_eq!(
        label(&tracks[1], &names, &strings, &tracks),
        "Talon's Junction Black"
    );
}

/// A circuit nothing names shows its id, and never a marker on its own.
#[test]
fn an_unnamed_circuit_shows_its_id() {
    let tracks = tracks(DEFINITION);
    let strings = oag_ui::language::StringTable::default();
    let names = oag_ui::language::CircuitNames::default();

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
    let served = oag_ui::language::StringTable::from_xml(
        r#"<StringTable><Entry ID="16_Track" String="THE WRONG ONE"/></StringTable>"#,
    );
    let chosen = oag_ui::language::StringTable::from_xml(
        r#"<StringTable><entry id="16_TRACK" string="VINETA K"/>
                        <entry id="32_TRACK" string="VINETA K"/></StringTable>"#,
    );
    let ids: Vec<String> = tracks.iter().map(|track| track.id.clone()).collect();
    let names = oag_ui::language::CircuitNames::choose(&[("DATA06".to_string(), chosen)], &ids)
        .expect("that copy names both circuits, case folded");

    assert_eq!(label(&tracks[0], &names, &served, &tracks), "VINETA K");
}

/// The team label falls through table, then declared name, then folder - in
/// that order.
///
/// The order is the point, not the fallback. `Mantis` is the case that fixes
/// it: its pack is sold as Mirage, the disc's own string table says so, and a
/// build that preferred a definition's spelling would put the folder name in
/// the menu for it. The middle step exists only for a source whose table cannot
/// answer, which is Wipeout Pure.
#[test]
fn a_team_label_prefers_the_table_then_the_declared_name_then_the_folder() {
    let strings = oag_ui::language::StringTable::from_xml(
        r#"<Screen name="Top"><Entry ID="Mantis" String="Mirage"/></Screen>"#,
    );
    let team = |id: &str, name: Option<&str>| Team {
        id: id.to_string(),
        name: name.map(str::to_string),
        location: format!(r"Data\Ships\{id}"),
        help_text: None,
        skins: Vec::new(),
        rating: None,
        models: Vec::new(),
        hull_unlocks: Vec::new(),
        zone_livery: None,
    };

    // In the table and also carrying a declared name: the table still wins, or
    // a pack team reads as its own folder instead of what it was sold as.
    assert_eq!(
        team("Mantis", Some("Something Else")).label(&strings),
        "Mirage"
    );
    // Not in the table, declared differently from the folder: Pure's case.
    assert_eq!(
        team("AG_Systems", Some("AG Systems")).label(&strings),
        "AG Systems"
    );
    // Neither: the folder, which is what a team with no table entry already
    // fell back to.
    assert_eq!(team("Feisar", None).label(&strings), "Feisar");
}

/// Wipeout HD/Fury's shape: `<FE>` on each `PI_TeamModel`, half points
/// included, and two models sharing a hull directory told apart by their
/// `<Unlock>`. Trimmed from `DATA00`'s own `Data\Plugins\Frontend\Definition.xml`.
#[test]
fn hd_ratings_are_per_model_in_tenths() {
    let xml = r#"<PI_Team name="Feisar">
    <Values type="Race" location="Data\Ships\Feisar"></Values>
    <PI_TeamModel name="chrome_c1"><Values modellocation="Data\Ships\Feisar_c1"></Values><FE speed="8" thrust="8.5" handling="10" shield="8"></FE><Unlock locked="true"></Unlock></PI_TeamModel>
    <PI_TeamModel name="concept1"><Values modellocation="Data\Ships\Feisar_c1"></Values><FE speed="8" thrust="8.5" handling="10" shield="8"></FE></PI_TeamModel>
    <PI_TeamModel name="normal"><Values modellocation="Data\Ships\Feisar"></Values><FE speed="7" thrust="8" handling="10" shield="8"></FE></PI_TeamModel>
    <PI_TeamModel name="zone"><Values modellocation="Data\Ships\Zone"></Values></PI_TeamModel>
  </PI_Team>"#;
    let teams = teams(xml);
    let team = &teams[0];
    assert!(team.rating.is_none());
    assert_eq!(team.models.len(), 3, "zone authors no FE");
    let concept = team.model_for_directory(r"Data\Ships\feisar_c1").unwrap();
    assert_eq!(concept.name, "concept1");
    assert_eq!(
        concept.rating_tenths,
        Some(Rating {
            speed: 80,
            thrust: 85,
            handling: 100,
            shield: 80
        })
    );
    assert_eq!(
        team.model_for_directory(r"Data\Ships\Feisar").unwrap().name,
        "normal"
    );
    assert!(team.model_for_directory(r"Data\Ships\Feisar_n1").is_none());
}

fn track(id: &str, folder: &str, reversed: bool) -> (Track, String) {
    (
        Track {
            id: id.into(),
            location: format!(r"Data\Environments\{folder}"),
            reversed,
            available_in_zone: false,
            unlock_grid: None,
        },
        if reversed {
            format!("{id} REVERSE")
        } else {
            id.to_string()
        },
    )
}

/// The order measured on RPCS3: every forward circuit, then every reverse
/// one in the same order, twelve columns wide there; a reverse circuit takes
/// its forward twin's name, and HD's four Zone-only environments are not in
/// the ordinary carousel.
#[test]
fn hd_lays_circuits_out_forward_then_reverse_without_the_zone_environments() {
    let listed = vec![
        track("01", "01_Vineta_K", false),
        track("08", "15_Anulpha_Pass", false),
        track("09", "01_Vineta_K", true),
        track("16", "15_Anulpha_Pass", true),
        track("25", "Zone_1", false),
    ];
    let (rows, columns) = direction_rows(oag_hd::TITLE, oag_race::Mode::SingleRace, &listed);
    let ids: Vec<&str> = rows.iter().map(|(t, _)| t.id.as_str()).collect();
    assert_eq!(ids, ["01", "08", "09", "16"]);
    assert_eq!(columns, 2);
    let labels: Vec<&str> = rows.iter().map(|(_, l)| l.as_str()).collect();
    assert_eq!(labels, ["01", "08", "01", "08"], "no direction in the name");

    let zone_list = vec![track("25", "Zone_1", false), track("26", "Zone_2", false)];
    let (zone, columns) = direction_rows(oag_hd::TITLE, oag_race::Mode::Zone, &zone_list);
    assert_eq!(zone.len(), 2, "Zone keeps its own environments");
    assert_eq!(columns, 2, "one row");
}
