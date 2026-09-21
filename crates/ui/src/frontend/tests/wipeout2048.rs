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
</Screen>
"#;

const VITA: (u32, u32) = (960, 544);

/// The chain `oag_2048::frontend::BOOT_PROFILE` resolves to, with the intro
/// movie's plan as given - `0` frames is "no demuxer", the state every run
/// is in until `oag-video` reads MP4.
fn boot(intro_frames: usize) -> Frontend {
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

fn tick(frontend: &mut Frontend, input: &mut Input, held: u32) {
    input.begin_frame(held);
    frontend.update(1.0 / 60.0, input, None);
}

fn press(frontend: &mut Frontend, input: &mut Input, button: Button) {
    tick(frontend, input, button.bit());
    tick(frontend, input, 0);
}

fn reach_the_grid(frontend: &mut Frontend, input: &mut Input) {
    tick(frontend, input, 0);
    assert!(frontend.machine().is(w2048::BOOT_STUDIO_LOGO));
    press(frontend, input, Button::Cross);
    assert!(frontend.machine().is(w2048::BOOT_INTRO_MOVIE));
    press(frontend, input, Button::Cross);
    // The save check passes through on its own tick.
    tick(frontend, input, 0);
    assert!(frontend.machine().is(w2048::TITLE_SCREEN));
    press(frontend, input, Button::Start);
    assert!(frontend.machine().is(w2048::GAME_MODE_CHOICE));
}

#[test]
fn boot_connect_leaves_on_its_first_tick_and_the_card_on_its_authored_delay() {
    let mut frontend = boot(0);
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
fn an_unmeasured_intro_waits_for_an_authored_button_and_says_so() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    tick(&mut frontend, &mut input, 0);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::BOOT_INTRO_MOVIE));
    for _ in 0..600 {
        tick(&mut frontend, &mut input, 0);
    }
    assert!(frontend.machine().is(w2048::BOOT_INTRO_MOVIE));
    // Circle is not one of this fixture's redirects; square is.
    press(&mut frontend, &mut input, Button::Circle);
    assert!(frontend.machine().is(w2048::BOOT_INTRO_MOVIE));
    tick(&mut frontend, &mut input, Button::Square.bit());
    assert!(frontend.machine().is(w2048::LOAD_SAVE_BOOTUP));
    let notes = frontend.take_notes();
    assert_eq!(
        notes
            .iter()
            .filter(|note| note.contains("length is unknown"))
            .count(),
        1,
        "said once, not every tick: {notes:#?}"
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
fn the_shell_reaches_home_on_triangle_and_home_names_an_unloaded_screen() {
    let mut frontend = boot(0);
    let mut input = Input::new();
    reach_the_grid(&mut frontend, &mut input);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Cross);
    press(&mut frontend, &mut input, Button::Triangle);
    assert!(frontend.machine().is(w2048::HOME));
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::HOME), "`team` is not loaded");
    let notes = frontend.take_notes();
    assert!(
        notes
            .iter()
            .any(|note| note.contains("team is a screen this build does not load")),
        "{notes:#?}"
    );
    press(&mut frontend, &mut input, Button::Right);
    press(&mut frontend, &mut input, Button::Cross);
    assert!(frontend.machine().is(w2048::NEW_FE_SHELL));
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
    let mut frontend = boot(0);
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
