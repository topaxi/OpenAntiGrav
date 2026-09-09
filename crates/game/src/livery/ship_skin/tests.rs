//! What [`super::declared`] picks out of a definition, and what it says when
//! it picks nothing. No game data in any test: the XML is the schema
//! `docs/formats/dlc-pack.md` records, with the shipped names left out.

use super::*;

/// A definition shaped exactly like a real one - two skins under `Normal`,
/// an `Unlock` pair on each, and a `Concept` sibling that carries its own
/// `Unlock` and no skin at all.
const DEFINITION: &str = r#"<Screen name="Top">
  <PI_Team name="Alpha">
    <Values type="Race" location="Data\Ships\Alpha" helpText="MSC_A"/>
    <PI_TeamModel name="Normal">
      <Values location="ship"/>
      <PI_ModelSkin name="Alternative">
        <Values location="Data\Ships\Alpha\ship_alt.dat"/>
        <Unlock Team="Alpha" loyalty="10" Exclusive="true"/>
        <Unlock Team="any" loyalty="90" Exclusive="true"/>
      </PI_ModelSkin>
      <PI_ModelSkin name="Eliminator">
        <Values location="Data\Ships\Alpha\ship_eliminator.dat"/>
        <Unlock Team="Alpha" loyalty="20" Exclusive="true"/>
        <Unlock Team="any" loyalty="95" Exclusive="true"/>
      </PI_ModelSkin>
    </PI_TeamModel>
    <PI_TeamModel name="Concept">
      <Values location="extra"/>
      <Unlock Team="Alpha" loyalty="30" Exclusive="true"/>
    </PI_TeamModel>
  </PI_Team>
  <PI_Team name="Beta">
    <Values type="Race" location="Data\Ships\Beta"/>
  </PI_Team>
</Screen>"#;

fn documents() -> Vec<String> {
    vec![DEFINITION.to_string()]
}

#[test]
fn a_declared_skin_resolves_to_its_own_path_verbatim() {
    let mut report = Vec::new();
    assert_eq!(
        declared(&documents(), "Alpha", "Eliminator", &mut report).as_deref(),
        Some(r"Data\Ships\Alpha\ship_eliminator.dat")
    );
}

/// The name comes from a player, so the disc's own capitalisation is not what
/// anyone types.
#[test]
fn the_name_is_matched_case_insensitively() {
    let mut report = Vec::new();
    assert_eq!(
        declared(&documents(), "Alpha", "alternative", &mut report).as_deref(),
        Some(r"Data\Ships\Alpha\ship_alt.dat")
    );
}

/// A team with no `PI_TeamModel` at all races the baseline, and the report
/// says which team had nothing rather than going quiet.
#[test]
fn a_team_that_declares_no_skin_is_reported_and_races_the_baseline() {
    let mut report = Vec::new();
    assert_eq!(
        declared(&documents(), "Beta", "Alternative", &mut report),
        None
    );
    assert!(
        report
            .iter()
            .any(|line| line.contains("Beta") && line.contains("none")),
        "{report:?}"
    );
}

/// Asking for a skin the team does not have names the ones it does, so a
/// typo is one line away from its own fix.
#[test]
fn an_unknown_skin_name_lists_what_the_team_does_declare() {
    let mut report = Vec::new();
    assert_eq!(declared(&documents(), "Alpha", "Zone", &mut report), None);
    let line = report.first().expect("a report line");
    assert!(line.contains("Alternative"), "{line}");
    assert!(line.contains("Eliminator"), "{line}");
}

#[test]
fn an_unknown_team_resolves_to_nothing() {
    let mut report = Vec::new();
    assert_eq!(
        declared(&documents(), "Gamma", "Alternative", &mut report),
        None
    );
    assert!(
        report.iter().any(|line| line.contains("Gamma")),
        "{report:?}"
    );
}

/// A pack's manifest is read on the same terms as the disc's own definition,
/// which is what makes a downloadable team's skin resolve at all.
#[test]
fn a_pack_manifest_adds_its_own_teams_skins() {
    const PACK: &str = r#"<Screen name="Top">
  <PI_Team name="Delta">
    <Values type="Race" location="Data\Ships\Delta"/>
    <PI_TeamModel name="Normal">
      <Values location="ship"/>
      <PI_ModelSkin name="Alternative">
        <Values location="Data\Ships\Delta\ship_alt.dat"/>
      </PI_ModelSkin>
    </PI_TeamModel>
  </PI_Team>
</Screen>"#;
    let documents = vec![DEFINITION.to_string(), PACK.to_string()];
    let mut report = Vec::new();
    assert_eq!(
        declared(&documents, "Delta", "Alternative", &mut report).as_deref(),
        Some(r"Data\Ships\Delta\ship_alt.dat")
    );
}
