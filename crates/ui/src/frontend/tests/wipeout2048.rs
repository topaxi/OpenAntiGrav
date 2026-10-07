//! Wipeout 2048's boot screens and touch grids, on a fixture shaped like
//! `NEWGUI/Intro_Definition.xml` and `Definition.xml` with no disc at all.
//!
//! The numbers are the real files' (`docs/formats/2048-frontend.md`), cut to
//! two tiles per grid; the disc-backed twin of this file is
//! `crates/game/tests/vita_2048_boot_ground_truth.rs`.

use super::*;

use crate::pointer::Pointer;
use oag_2048::frontend::states as w2048;

const XML: &str = r#"
<Screen>
  <Variable global="Blue2048"><Values String="0xff19295d"></Values></Variable>
  <Variable global="Orange2048"><Values String="0xffdd580b"></Values></Variable>
  <Variable global="White2048"><Values String="0xffe1e4eb"></Values></Variable>
  <Variable global="Grey2048"><Values String="0xff717b96"></Values></Variable>
  <Variable global="Pass2048"><Values String="0xff018400"></Values></Variable>
  <Variable global="ElitePass2048"><Values String="0xfffef502"></Values></Variable>
  <Screen name="Boot Connect">
    <Image><Values x="0" y="0" width="960" height="544" Color="0xffffffff"></Values></Image>
  </Screen>
  <Screen name="Boot Studio Logo">
    <Image><Values x="480" y="272" width="960" height="128" Centred="true" src="Data\FE\Images\StudioLogo.gtf"></Values></Image>
    <Redirect name="Redirect" delay="4.0"><Values backward="none" forward="none"></Values><Default goto="Boot Intro Movie"></Default></Redirect>
    <Redirect name="Redirectbutton"><Values backward="none" forward="cross"></Values><Default goto="Boot Intro Movie"></Default></Redirect>
  </Screen>
  <Screen name="Boot Intro Movie">
    <Movie name="LogoMovie"><Values src="data/Videos/intro.mp4" repeat="false"></Values></Movie>
    <Redirect name="AutoRedirect" StartEnabled="false"><Values backward="none" forward="none"></Values><Default goto="Load Save Bootup"></Default></Redirect>
    <Redirect name="cross press"><Values forward="cross" backward="none"></Values><Default goto="Load Save Bootup"></Default></Redirect>
    <Redirect name="square press"><Values forward="square" backward="none"></Values><Default goto="Load Save Bootup"></Default></Redirect>
  </Screen>
  <Screen name="Load Save Bootup" type="HDDBoot">
    <Redirect name="Main Menu Redirect" StartEnabled="false"><Values forward="none" backward="none"></Values><Default goto="TitleScreen"></Default></Redirect>
  </Screen>
  <Screen name="TitleScreen">
    <Text delay="1.0"><Values x="480" y="370" align="centre" vertalign="middle" pulse="true" font="default" scale="1.0" idstring="BOOT_PRESS_ANY" color="FEGlobals->Blue2048"></Values></Text>
    <Redirect><Values forward="start" backward="none"></Values><Default goto="GameModeChoice"></Default></Redirect>
    <Redirect><Values forward="cross" backward="none"></Values><Default goto="GameModeChoice"></Default></Redirect>
  </Screen>
  <Screen name="overshell">
    <Screen name="newFEshell">
      <Screen name="GameModeChoice" type="GameModeChoice_Screen">
        <Image name="optionsTitleImg"><Values x="15" y="15" width="70" height="70" src="Data\FE\NewImages\Icon_Mode_HomeBut.gtf" color="FEGlobals->White2048"></Values></Image>
        <Text><Values idstring="FE_GAME_MODE" font="default" x="105" y="30" color="FEGlobals->Blue2048"></Values></Text>
        <TouchButton name="offline"><Values idstring="FE_SP_CAMPAIGN" x="167" y="190" width="140" height="140" toggle="true" StringWidthLimit="175" Src="Data\FE\NewImages\gamemodechoice\gm_SPCampaign.gtf"></Values></TouchButton>
        <TouchButton name="multiplayer"><Values idstring="FE_MP_CAMPAIGN" x="329" y="190" width="140" height="140" toggle="true" StringWidthLimit="150" Src="Data\FE\NewImages\gamemodechoice\gm_MPCampaign.gtf"></Values></TouchButton>
        <TouchButton name="confirmButton"><Values string="" redirect="newFEshell" x="822" y="432" width="122" height="96" Src="Data\FE\NewImages\Icon_Tick.gtf"></Values></TouchButton>
      </Screen>
      <Screen name="Home" type="Home">
        <TouchButton name="HomeTeamButton"><Values idstring="ER_TEAM" redirect="team" x="86" y="190" width="140" height="140" Src="Data\FE\NewImages\Icon_Team_HomeBut.gtf"></Values></TouchButton>
        <TouchButton><Values redirect="newFEshell" x="822" y="432" width="122" height="96" Src="Data\FE\NewImages\Icon_Tick.gtf"></Values></TouchButton>
      </Screen>
    </Screen>
  </Screen>
  <Screen name="teamshell">
    <Screen name="team">
      <TouchButton name="select_button"><Values redirect="PreviousScreen" x="822" y="432" width="122" height="96" Src="Data\FE\NewImages\Icon_Tick.gtf"></Values></TouchButton>
    </Screen>
  </Screen>
</Screen>
"#;

const VITA: (u32, u32) = (960, 544);

/// The chain `oag_2048::frontend::BOOT_PROFILE` resolves to, with the intro
/// movie's plan as given - `0` frames is "no demuxer", the state every run
/// is in until `oag-video` reads MP4.
pub(super) fn boot(intro_frames: usize) -> Frontend {
    let mut frontend = Frontend::booting(
        Sequence {
            steps: vec![
                Step {
                    state: w2048::BOOT_CONNECT,
                    movie: MoviePlan::none(VITA),
                },
                Step {
                    state: w2048::BOOT_STUDIO_LOGO,
                    movie: MoviePlan::none(VITA),
                },
                Step {
                    state: w2048::BOOT_INTRO_MOVIE,
                    movie: MoviePlan {
                        frames: intro_frames,
                        frame_rate: FRAME_RATE,
                        aspect: VITA,
                        has_picture: intro_frames > 0,
                    },
                },
                Step {
                    state: w2048::LOAD_SAVE_BOOTUP,
                    movie: MoviePlan::none(VITA),
                },
                Step {
                    state: w2048::TITLE_SCREEN,
                    movie: MoviePlan::none(VITA),
                },
            ],
            backdrop_parent: None,
        },
        Screens::from_xml(XML),
        StringTable::default(),
        Vec::new(),
        Vec::new(),
    );
    frontend.set_space(Space::VITA);
    frontend
}

pub(super) fn tick(frontend: &mut Frontend, input: &mut Input, held: u32) {
    input.begin_frame(held);
    frontend.update(1.0 / 60.0, input, None);
}

pub(super) fn press(frontend: &mut Frontend, input: &mut Input, button: Button) {
    tick(frontend, input, button.bit());
    tick(frontend, input, 0);
}

fn reach_the_grid(frontend: &mut Frontend, input: &mut Input) {
    tick(frontend, input, 0);
    assert!(frontend.machine().is(w2048::BOOT_STUDIO_LOGO));
    // The fixture's intro has no picture, so it leaves by itself on the tick
    // after the press, and the save check passes through on the next one.
    press(frontend, input, Button::Cross);
    tick(frontend, input, 0);
    assert!(frontend.machine().is(w2048::TITLE_SCREEN));
    press(frontend, input, Button::Start);
    assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
}

#[test]
fn boot_connect_leaves_on_its_first_tick_and_the_card_on_its_authored_delay() {
    let mut frontend = boot(30);
    let mut input = Input::new();
    assert!(frontend.machine().is(w2048::BOOT_CONNECT));
    tick(&mut frontend, &mut input, 0);
    assert!(frontend.machine().is(w2048::BOOT_STUDIO_LOGO));
    // 3.9 s: still the card.
    for _ in 0..234 {
        tick(&mut frontend, &mut input, 0);
    }
    assert!(frontend.machine().is(w2048::BOOT_STUDIO_LOGO));
    for _ in 0..8 {
        tick(&mut frontend, &mut input, 0);
    }
    assert!(frontend.machine().is(w2048::BOOT_INTRO_MOVIE));
    let notes = frontend.take_notes();
    assert!(
        notes.iter().any(|note| note.contains("4.0s delay ran out")),
        "{notes:#?}"
    );
}

#[test]
fn an_unmeasured_intro_is_skipped_and_says_so() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    tick(&mut frontend, &mut input, 0);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(
        frontend.machine().is(w2048::LOAD_SAVE_BOOTUP),
        "no picture, so the intro leaves with no button"
    );
    let notes = frontend.take_notes();
    assert_eq!(
        notes
            .iter()
            .filter(|note| note.contains("no picture"))
            .count(),
        1,
        "said once: {notes:#?}"
    );
}

#[test]
fn a_measured_intro_plays_out_and_fires_its_auto_redirect() {
    let mut frontend = boot(30);
    let mut input = Input::new();
    tick(&mut frontend, &mut input, 0);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.is_playing_movie());
    for _ in 0..90 {
        tick(&mut frontend, &mut input, 0);
    }
    assert!(frontend.machine().is(w2048::TITLE_SCREEN));
}

#[test]
fn the_title_screen_leaves_on_its_own_redirects_to_the_mode_grid() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    assert!(!frontend.is_finished(), "the grid is not the end");
}

#[test]
fn the_pad_chooses_a_mode_with_two_taps_and_the_tick_confirms() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    assert_eq!(frontend.chosen_mode(), Some("FE_SP_CAMPAIGN"));
    assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
}

#[test]
fn the_pad_walks_right_to_the_tick_and_a_network_mode_is_refused_by_note() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Right);
    press(&mut frontend, &mut input, Button::Cross);
    assert_eq!(frontend.chosen_mode(), Some("FE_MP_CAMPAIGN"));
    press(&mut frontend, &mut input, Button::Right);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
    let notes = frontend.take_notes();
    assert!(
        notes
            .iter()
            .any(|note| note.contains("network session this build does not have")),
        "{notes:#?}"
    );
}

#[test]
fn the_shell_reaches_home_and_home_leads_to_team_and_back() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Triangle);
    assert!(frontend.machine().is(w2048::HOME));
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::TEAM), "`team` is now loaded");
    let notes = frontend.take_notes();
    assert!(
        notes.iter().any(|note| note.contains("firing team")),
        "{notes:#?}"
    );
    // Circle fires `select_button`'s own `redirect="PreviousScreen"`
    // (`Frontend::update_team`), which pops the back stack `redirect_touch`
    // pushed on the way in, landing back on `Home` rather than on a screen
    // name the tick never states.
    press(&mut frontend, &mut input, Button::Circle);
    assert!(
        frontend.machine().is(w2048::HOME),
        "PreviousScreen should return to Home"
    );
}

#[test]
fn a_click_on_a_tile_is_the_tap_and_a_click_elsewhere_is_nothing() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    let click = |at| Pointer {
        at: Some(at),
        moved: true,
        clicked: true,
        ..Pointer::default()
    };
    // Inside the second tile.
    assert!(frontend.pointer(&click((400.0, 250.0))));
    tick(&mut frontend, &mut input, 0);
    assert_eq!(frontend.chosen_mode(), Some("FE_MP_CAMPAIGN"));
    // Between tiles: taken by the grid, choosing nothing.
    assert!(frontend.pointer(&click((320.0, 250.0))));
    tick(&mut frontend, &mut input, 0);
    assert_eq!(frontend.chosen_mode(), Some("FE_MP_CAMPAIGN"));
    // The first tile, twice: chosen, then confirmed through the tick.
    assert!(frontend.pointer(&click((200.0, 250.0))));
    tick(&mut frontend, &mut input, 0);
    assert!(frontend.pointer(&click((200.0, 250.0))));
    tick(&mut frontend, &mut input, 0);
    assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
}

#[test]
fn a_hover_moves_the_cursor_without_choosing() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    let hover = Pointer {
        at: Some((850.0, 470.0)),
        moved: true,
        ..Pointer::default()
    };
    assert!(frontend.pointer(&hover));
    assert_eq!(frontend.chosen_mode(), None);
    // The ring is on the tick now: a fill at the tick's own rect, outset.
    let list = frontend.draw_list();
    assert!(list.iter().any(|draw| matches!(
        draw,
        Draw::Fill { rect, .. } if (rect[0] - 818.0).abs() < 0.01 && (rect[1] - 428.0).abs() < 0.01
    )));
}

#[test]
fn the_boot_screens_draw_off_the_pad_and_only_the_movie_screen_plays() {
    let mut frontend = boot(30);
    let mut input = Input::new();
    tick(&mut frontend, &mut input, 0);
    assert!(!frontend.is_playing_movie());
    let card = frontend.draw_list();
    // Black clear, then the white ground, then the centred logo.
    assert!(matches!(card[1], Draw::Fill { color, .. } if color == [1.0, 1.0, 1.0, 1.0]));
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.is_playing_movie());
}

#[test]
fn a_centred_image_draws_from_its_middle() {
    let mut frontend = boot(0);
    frontend.placements = vec![(
        r"Data\FE\Images\StudioLogo.gtf".to_string(),
        Placed {
            x: 0,
            y: 0,
            width: 1024,
            height: 128,
            quad_extent: None,
            blend: None,
        },
    )];
    let mut input = Input::new();
    tick(&mut frontend, &mut input, 0);
    let list = frontend.draw_list();
    let logo = list
        .iter()
        .find_map(|draw| match draw {
            Draw::Sprite { rect, .. } => Some(*rect),
            _ => None,
        })
        .expect("the card's logo");
    assert_eq!(logo, [0.0, 208.0, 960.0, 128.0]);
}

#[test]
fn a_middle_aligned_text_centres_its_line_once_the_face_is_known() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    // Back on the title screen's static draw: the pen at the authored `y`
    // until the face height is set, half a line up after.
    let pen = |frontend: &Frontend| {
        frontend
            .draw_screen(w2048::TITLE_SCREEN)
            .iter()
            .find_map(|draw| match draw {
                Draw::Text { y, text, .. } if text == "BOOT_PRESS_ANY" => Some(*y),
                _ => None,
            })
            .expect("the prompt")
    };
    assert_eq!(pen(&frontend), 370.0);
    frontend.set_face_scales(vec![("NEOSANS_BOLD".to_string(), 0.6)], 37.0);
    assert_eq!(pen(&frontend), 370.0 - 18.5);
}

pub(super) fn reach_the_shell(frontend: &mut Frontend, input: &mut Input) {
    reach_the_grid(frontend, input);
    press(frontend, input, Button::Cross);
    press(frontend, input, Button::Cross);
    assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
}

fn three_events() -> Vec<MapEvent> {
    [
        ("2048 - Event 2", 3, 5),
        ("2048 - Event 1", 1, 5),
        ("2049 - Event 1", 11, 9),
    ]
    .into_iter()
    .map(|(name, x, y)| MapEvent {
        name: name.to_string(),
        x,
        y,
        detail: "circuit / mode".to_string(),
        requires: None,
        kind: EventIcon::Race,
        forced_craft: None,
        refused_craft: Vec::new(),
        card: crate::frontend::EventCard::default(),
    })
    .collect()
}

/// Two chained events - `"2048 - Event 2"` gated on `"2048 - Event 1"` -
/// for the locked/open/passed/elite tests below. Kept apart from
/// [`three_events`] so every test written against that fixture keeps
/// exercising an all-open map, the same shape it always has.
fn a_gated_event() -> Vec<MapEvent> {
    vec![
        MapEvent {
            name: "2048 - Event 1".to_string(),
            x: 1,
            y: 5,
            detail: "circuit / mode".to_string(),
            requires: None,
            kind: EventIcon::Race,
            forced_craft: None,
            refused_craft: Vec::new(),
            card: crate::frontend::EventCard::default(),
        },
        MapEvent {
            name: "2048 - Event 2".to_string(),
            x: 3,
            y: 5,
            detail: "circuit / mode".to_string(),
            requires: Some("2048 - Event 1".to_string()),
            kind: EventIcon::Race,
            forced_craft: None,
            refused_craft: Vec::new(),
            card: crate::frontend::EventCard::default(),
        },
    ]
}

#[test]
fn the_extra_tiles_come_after_the_authored_ones_and_launch_this_builds_pages() {
    let mut frontend = boot(0);
    frontend.set_extra_tiles(vec![
        ExtraTile {
            label: "RACEBOX".to_string(),
            launch: Launch::RaceBox,
        },
        ExtraTile {
            label: "REMIX".to_string(),
            launch: Launch::Remix,
        },
    ]);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    // Three authored buttons, then the two extras: right four times lands
    // on REMIX.
    for _ in 0..4 {
        press(&mut frontend, &mut input, Button::Right);
    }
    let list = frontend.draw_list();
    assert!(list.iter().any(|draw| matches!(
        draw,
        Draw::Text { text, .. } if text == "REMIX"
    )));
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.is_finished());
    assert_eq!(frontend.launch(), Some(&Launch::Remix));
}

#[test]
fn a_click_on_the_race_box_tile_asks_for_the_race_box() {
    let mut frontend = boot(0);
    frontend.set_extra_tiles(vec![ExtraTile {
        label: "RACEBOX".to_string(),
        launch: Launch::RaceBox,
    }]);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    let [x, y, w, h] = EXTRA_TILES[0];
    let click = Pointer {
        at: Some((x + w * 0.5, y + h * 0.5)),
        moved: true,
        clicked: true,
        ..Pointer::default()
    };
    assert!(frontend.pointer(&click));
    tick(&mut frontend, &mut input, 0);
    assert_eq!(frontend.launch(), Some(&Launch::RaceBox));
}

#[test]
fn the_map_opens_on_the_first_seasons_first_event_and_the_pad_walks_it() {
    let mut frontend = boot(0);
    frontend.set_campaign(three_events());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    assert_eq!(
        frontend.selected_event().map(|e| e.name.as_str()),
        Some("2048 - Event 1")
    );
    press(&mut frontend, &mut input, Button::Right);
    assert_eq!(
        frontend.selected_event().map(|e| e.name.as_str()),
        Some("2048 - Event 2")
    );
    // Nothing further right on the same row within reach: the next season
    // is right and down, still the nearest that way.
    press(&mut frontend, &mut input, Button::Right);
    assert_eq!(
        frontend.selected_event().map(|e| e.name.as_str()),
        Some("2049 - Event 1")
    );
    press(&mut frontend, &mut input, Button::Left);
    assert_eq!(
        frontend.selected_event().map(|e| e.name.as_str()),
        Some("2048 - Event 2")
    );
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.is_finished());
    assert_eq!(
        frontend.launch(),
        Some(&Launch::Event("2048 - Event 2".to_string()))
    );
}

/// A tap on the event card's Launch button - the centre of its hit rect.
fn launch_button_tap() -> Pointer {
    Pointer {
        at: Some((880.0, 480.0)),
        moved: true,
        clicked: true,
        ..Pointer::default()
    }
}

#[test]
fn a_click_on_an_unselected_marker_selects_and_a_second_click_launches() {
    let mut frontend = boot(0);
    frontend.set_campaign(three_events());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    // The markers of the first two events are in view: cell (3, 5) is at
    // 65 + 2 * 146 = 357, 63 + 4 * 111 = 507, before scrolling. The view
    // follows the selection, so read the marker's rect off the draw list.
    let marker_of = |frontend: &Frontend, at: usize| {
        let list = frontend.draw_list();
        list.iter()
            .filter_map(|draw| match draw {
                Draw::Fill { rect, color } if rect[2] == 108.0 && color[3] == 1.0 => Some(*rect),
                _ => None,
            })
            .nth(at)
            .expect("a marker")
    };
    let second = marker_of(&frontend, 0);
    let click = |rect: [f32; 4]| Pointer {
        at: Some((rect[0] + 54.0, rect[1] + 54.0)),
        moved: true,
        clicked: true,
        ..Pointer::default()
    };
    assert!(frontend.pointer(&click(second)));
    tick(&mut frontend, &mut input, 0);
    assert!(!frontend.is_finished());
    assert_eq!(
        frontend.selected_event().map(|e| e.name.as_str()),
        Some("2048 - Event 2")
    );
    let again = marker_of(&frontend, 0);
    assert!(frontend.pointer(&click(again)));
    tick(&mut frontend, &mut input, 0);
    assert!(frontend.event_card_open());
    assert!(!frontend.is_finished(), "the second click opens the card");
    assert!(frontend.pointer(&launch_button_tap()));
    tick(&mut frontend, &mut input, 0);
    assert_eq!(
        frontend.launch(),
        Some(&Launch::Event("2048 - Event 2".to_string()))
    );
}

#[test]
fn a_gated_event_with_nothing_earned_yet_refuses_a_launch() {
    let mut frontend = boot(0);
    frontend.set_campaign(a_gated_event());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Right);
    assert_eq!(
        frontend.selected_event().map(|e| e.name.as_str()),
        Some("2048 - Event 2")
    );
    press(&mut frontend, &mut input, Button::Cross);
    assert!(
        !frontend.is_finished(),
        "a locked event must refuse to launch"
    );
    let notes = frontend.take_notes();
    assert!(
        notes.iter().any(|note| note.contains("is locked")),
        "{notes:#?}"
    );
}

/// One event restricted the way `"2050 - Event 5"` really is (no Agility, no
/// Speed) - `Feisar2048\3` (Feisar's own speed craft) is the one refused id
/// these tests exercise.
pub(super) fn a_restricted_event() -> Vec<MapEvent> {
    vec![MapEvent {
        name: "2048 - Event 1".to_string(),
        x: 1,
        y: 5,
        detail: "circuit / mode".to_string(),
        requires: None,
        kind: EventIcon::Race,
        forced_craft: None,
        refused_craft: vec!["Feisar2048\\3".to_string()],
        card: crate::frontend::EventCard::default(),
    }]
}

/// One event forcing a specific craft the way `"2048 - Event 4-2"` really
/// does - `refused_craft` is non-empty here too, to prove
/// [`Frontend::launch_selected_event`] never consults it once
/// [`MapEvent::forced_craft`] is `Some`, the precedence `race::load_event`
/// already applies.
pub(super) fn a_forced_event() -> Vec<MapEvent> {
    vec![MapEvent {
        name: "2048 - Event 1".to_string(),
        x: 1,
        y: 5,
        detail: "circuit / mode".to_string(),
        requires: None,
        kind: EventIcon::Race,
        forced_craft: Some("Qirex2048\\1".to_string()),
        refused_craft: vec!["Feisar2048\\3".to_string()],
        card: crate::frontend::EventCard::default(),
    }]
}

#[test]
fn a_restricted_event_refuses_a_launch_when_the_seeded_craft_is_forbidden() {
    let mut frontend = boot(0);
    frontend.set_campaign(a_restricted_event());
    frontend.seed_craft("Feisar2048\\3".to_string());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(
        !frontend.is_finished(),
        "the seeded craft is this event's own refused one"
    );
    let notes = frontend.take_notes();
    assert!(
        notes.iter().any(|note| note.contains("forbids")),
        "{notes:#?}"
    );
}

#[test]
fn a_restricted_event_launches_when_the_seeded_craft_is_allowed() {
    let mut frontend = boot(0);
    frontend.set_campaign(a_restricted_event());
    frontend.seed_craft("Feisar2048\\1".to_string());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.is_finished());
    assert_eq!(
        frontend.launch(),
        Some(&Launch::Event("2048 - Event 1".to_string()))
    );
}

#[test]
fn touching_team_overrides_the_seed_in_either_direction() {
    // Seeded with an allowed craft, then the player moves on `Team` to the
    // one this event forbids - `team_choice()` wins.
    let mut frontend = boot(0);
    frontend.set_campaign(a_restricted_event());
    frontend.seed_craft("Feisar2048\\1".to_string());
    frontend.touch.team_choice = Some((2, 2)); // Feisar2048, speed (suffix "3")
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(
        !frontend.is_finished(),
        "team_choice() names the forbidden craft, overriding an allowed seed"
    );

    // The other way: seeded with the forbidden craft, then the player moves
    // to an allowed one.
    let mut frontend = boot(0);
    frontend.set_campaign(a_restricted_event());
    frontend.seed_craft("Feisar2048\\3".to_string());
    frontend.touch.team_choice = Some((2, 0)); // Feisar2048, fighter (suffix "1")
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.is_finished());
}

#[test]
fn a_forced_event_launches_regardless_of_refused_craft_or_the_seed() {
    let mut frontend = boot(0);
    frontend.set_campaign(a_forced_event());
    frontend.seed_craft("Feisar2048\\3".to_string());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.is_finished());
    assert_eq!(
        frontend.launch(),
        Some(&Launch::Event("2048 - Event 1".to_string()))
    );
}

#[test]
fn a_click_on_a_restricted_events_marker_refuses_the_same_way_the_pad_does() {
    let mut frontend = boot(0);
    frontend.set_campaign(a_restricted_event());
    frontend.seed_craft("Feisar2048\\3".to_string());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    let marker = {
        let list = frontend.draw_list();
        list.iter()
            .filter_map(|draw| match draw {
                Draw::Fill { rect, color } if rect[2] == 108.0 && color[3] == 1.0 => Some(*rect),
                _ => None,
            })
            .next()
            .expect("a marker")
    };
    let click = Pointer {
        at: Some((marker[0] + 54.0, marker[1] + 54.0)),
        moved: true,
        clicked: true,
        ..Pointer::default()
    };
    // First click selects (matching `a_click_on_an_unselected_marker...`'s
    // own shape) - the map opens on this event already selected here since
    // it is the only one, so this click already lands on the selected
    // marker and opens the card, whose Launch refuses.
    assert!(frontend.pointer(&click));
    tick(&mut frontend, &mut input, 0);
    assert!(frontend.pointer(&launch_button_tap()));
    tick(&mut frontend, &mut input, 0);
    assert!(
        !frontend.is_finished(),
        "the seeded craft is this event's own refused one"
    );
    let notes = frontend.take_notes();
    assert!(
        notes.iter().any(|note| note.contains("forbids")),
        "{notes:#?}"
    );
}

#[test]
fn refresh_campaign_progress_opens_a_gated_event_once_its_own_gate_is_passed() {
    let mut frontend = boot(0);
    frontend.set_campaign(a_gated_event());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    frontend
        .refresh_campaign_progress(|name| (name == "2048 - Event 1").then_some(EarnedTier::Pass));
    press(&mut frontend, &mut input, Button::Right);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    assert_eq!(
        frontend.launch(),
        Some(&Launch::Event("2048 - Event 2".to_string())),
        "Event 1 was passed, so Event 2's own gate is now open"
    );
}

#[test]
fn refresh_campaign_progress_colours_a_passed_marker_with_the_discs_own_green() {
    let mut frontend = boot(0);
    frontend.set_campaign(a_gated_event());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    frontend
        .refresh_campaign_progress(|name| (name == "2048 - Event 1").then_some(EarnedTier::Elite));
    let pass_green = frontend.global_colour("Pass2048");
    let elite_yellow = frontend.global_colour("ElitePass2048");
    assert_ne!(pass_green, elite_yellow, "the fixture must tell them apart");
    let list = frontend.draw_list();
    assert!(
        list.iter()
            .any(|draw| matches!(draw, Draw::Fill { color, .. } if *color == elite_yellow)),
        "Event 1's own marker should draw in ElitePass2048"
    );
}

#[test]
fn campaign_event_state_reads_locked_open_passed_and_elite_without_driving_input() {
    let mut frontend = boot(0);
    frontend.set_campaign(a_gated_event());
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    assert_eq!(
        frontend.campaign_event_state("2048 - Event 1"),
        Some(ProgressState::Open)
    );
    assert_eq!(
        frontend.campaign_event_state("2048 - Event 2"),
        Some(ProgressState::Locked)
    );
    assert_eq!(frontend.campaign_event_state("no such event"), None);
    frontend
        .refresh_campaign_progress(|name| (name == "2048 - Event 1").then_some(EarnedTier::Pass));
    assert_eq!(
        frontend.campaign_event_state("2048 - Event 1"),
        Some(ProgressState::Passed)
    );
    assert_eq!(
        frontend.campaign_event_state("2048 - Event 2"),
        Some(ProgressState::Open),
        "Event 1's own gate is satisfied by a bare pass"
    );
}

#[test]
fn a_map_with_no_events_says_so_and_launches_nothing() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_shell(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(!frontend.is_finished());
    assert!(frontend.draw_list().iter().any(|draw| matches!(
        draw,
        Draw::Text { text, .. } if text.contains("no campaign events")
    )));
}
