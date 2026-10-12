use super::*;

const SKIN: &str = r#"
<Screen name="Main Menu"><Menu name="Mode"><Entry idstring="Single Player"></Entry><Entry idstring="Multiplayer"></Entry></Menu></Screen>
<Screen name="Single Player">
  <Menu name="Mode"><Entry idstring="Arcade"/><Entry idstring="Tournament"/><Entry idstring="Zone"/></Menu>
  <Redirect>
    <Entry item="Mode" equals="Arcade" goto="Class Selection"></Entry>
    <Entry item="Mode" equals="Tournament" goto="Class Selection"></Entry>
    <Entry item="Mode" equals="Zone" goto="Zone Track Selection"></Entry>
    <Default goto="To Be Done"></Default>
  </Redirect>
</Screen>"#;

const SELECTION: &str = r#"
<Screen name="Class Selection">
  <Menu name="Class"><Entry idstring="Vector"/><Entry idstring="Venom"/></Menu>
  <Redirect name="Single Player Redirect">
    <Entry item="Single Player->Mode" equals="Tournament" goto="Tournament Selection"></Entry>
    <Default goto="League Selection"></Default>
  </Redirect>
  <Redirect name="Multiplayer Redirect">
    <Default goto="Team Selection"></Default>
  </Redirect>
</Screen>
<Screen name="League Selection"><Redirect><Default goto="Track Selection"></Default></Redirect></Screen>
<Screen name="Tournament Selection"><Redirect>
  <Entry item="Main Menu->Mode" equals="Multiplayer" goto="Multiplayer Redirect"></Entry>
  <Default goto="Team Selection"></Default></Redirect></Screen>
<Screen name="Track Selection">
  <Redirect>
    <Entry item="Main Menu->Mode" equals="Multiplayer" goto="Multiplayer Redirect"></Entry>
    <Default goto="Team Selection"></Default>
  </Redirect>
  <Redirect name="Go Back"><Default goto="League Selection"></Default></Redirect>
</Screen>
<Screen name="Zone Track Selection"><Redirect><Default goto="Launch Game"></Default></Redirect></Screen>
<Screen name="Team Selection"><Redirect>
  <Entry item="Main Menu->Mode" equals="Multiplayer" goto="Network Check"></Entry>
  <Default goto="Launch Game"></Default></Redirect></Screen>"#;

fn flow() -> Flow {
    parse(SKIN, SELECTION).expect("a flow")
}

#[test]
fn the_menus_read_in_order() {
    let flow = flow();
    assert_eq!(flow.main_modes, ["Single Player", "Multiplayer"]);
    assert_eq!(flow.modes, ["Arcade", "Tournament", "Zone"]);
    assert_eq!(flow.classes, ["Vector", "Venom"]);
}

#[test]
fn each_mode_walks_the_screens_its_redirects_name() {
    let flow = flow();
    assert_eq!(
        flow.route("Arcade").unwrap(),
        [
            Step::Class,
            Step::League,
            Step::Track,
            Step::Team,
            Step::Launch
        ]
    );
    assert_eq!(
        flow.route("Tournament").unwrap(),
        [Step::Class, Step::Tournament, Step::Team, Step::Launch]
    );
    assert_eq!(flow.route("Zone").unwrap(), [Step::ZoneTrack, Step::Launch]);
}

#[test]
fn a_redirect_to_a_screen_with_no_step_is_refused_by_name() {
    assert_eq!(
        flow().route("Free Play"),
        Err(Error::UnknownScreen("To Be Done".to_string()))
    );
}

#[test]
fn a_missing_menu_is_refused() {
    assert!(matches!(parse("", SELECTION), Err(Error::Missing(_))));
}
