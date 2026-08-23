//! What the front-end XML reader in [`super`] is asserted to do: the nesting
//! and attribute rules, the `FEGlobals` indirection, the movie widgets and
//! their filenames, the redirects, and the colour spellings.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `screen.rs`: the tests are 335 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;
use crate::input::Button;

/// The shape of the real `Language Selection` and `LogoFMV` screens, cut
/// down to what this module reads. Attribute spellings, the `FEGlobals->`
/// indirection and the newline inside a value are all as they appear in
/// `Data\Plugins\PI001\GUI\Skin.xml`.
const SAMPLE: &str = r#"
<Screen>
  <Variable global="TitleColor"><Values String="0xFF000000"></Values></Variable>
  <Variable global="TextColor"><Values String="0xFF33A6B9"></Values></Variable>
  <Variable global="MenuXOffset"><Values String="50"></Values></Variable>
  <Variable global="MenuScale"><Values String="1.0"></Values></Variable>
  <Screen type="Language Selection" name="Language Selection">
    <Text name="LanguageText" StartEnabled="false">
      <Values idstring="Language Selection" font="Title" x="FEGlobals->
        MenuXOffset" scale="1.0" color="FEGlobals->TitleColor"></Values>
    </Text>
    <DisplayLanguages><Values clear="true"></Values></DisplayLanguages>
    <Menu name="Language" focus="false" StartEnabled="false" save="true">
      <Values align="left" font="Default" x="FEGlobals->MenuXOffset" scale="FEGlobals->MenuScale" y="46" color="FEGlobals->TextColor"></Values>
    </Menu>
    <Redirect name="LanguageAutoRedirect">
      <Values backward="none"></Values>
      <Default goto="LogoFMV"></Default>
    </Redirect>
  </Screen>
  <Screen name="LogoFMV">
    <Image transition="0"><Values width="480" height="272" color="0xFF000000"></Values></Image>
    <Movie transition="0" name="BackdropMovie">
      <Values src="Data\Movies\Intro" sound="true" autostart="true" repeat="false" autoredirect="true"></Values>
    </Movie>
    <Redirect name="AutoRedirect" StartEnabled="false">
      <Values backward="none" forward="none"></Values>
      <Default goto="Show Logo"></Default>
    </Redirect>
    <Redirect>
      <Values backward="none" forward="start"></Values>
      <Default goto="LogoFMVRedirectScreen"></Default>
    </Redirect>
  </Screen>
  <Screen name="LogoFMVRedirectScreen">
    <Redirect>
      <Values backward="none" forward="none"></Values>
      <Default goto="Show Logo"></Default>
    </Redirect>
  </Screen>
  <Screen type="FEMain" name="Top FE Screen">
    <Screen name="FE Screen">
      <Movie transition="0" name="BackdropMovie">
        <Values src="Data\Movies\Backdrop" sound="false" autostart="true" repeat="true"></Values>
      </Movie>
      <Screen name="Show Logo">
        <Image transition="0"><Values y="72" AutoLoad="true" src="Data\FE\Images\pulse_logo.mip"></Values></Image>
        <Text delay="1" transition="0">
          <Values align="right" idstring="BOOT_PRESS_START" font="Menu" pulse="true" x="460" y="220" color="0x7FFFFFFF"></Values>
        </Text>
        <Viewport>
          <Values x="40" y="0" height="480" width="400"></Values>
          <Text name="USLegalText" delay="0" transition="0">
            <Values align="left" idstring="BOOT_LEGAL" font="small" scale="0.7" x="40" y="250" widthlimited="true" color="0x7FFFFFFF"></Values>
          </Text>
        </Viewport>
        <Redirect>
          <Values backward="none"></Values>
          <Default goto="RemoveMemoryStickWarning"></Default>
        </Redirect>
        <Redirect>
          <Values backward="none" forward="start"></Values>
          <Default goto="RemoveMemoryStickWarning"></Default>
        </Redirect>
      </Screen>
    </Screen>
  </Screen>
  <LoadXML><Values src="Data\Plugins\PI001\GUI\MainMenu_Definition.xml"></Values></LoadXML>
</Screen>
"#;

#[test]
fn parses_nested_elements() {
    let root = parse(SAMPLE);
    assert_eq!(root.name, "#document");
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].name, "Screen");
}

#[test]
fn values_children_carry_their_parents_attributes() {
    let screens = Screens::from_xml(SAMPLE);
    let movie = screens.by_name("LogoFMV").unwrap().movies[0].clone();
    assert_eq!(movie.src, r"Data\Movies\Intro");
    assert!(movie.sound);
    assert!(movie.autostart);
    assert!(movie.autoredirect);
    assert!(!movie.repeat);
}

/// A screen with several `Movie` widgets keeps all of them.
///
/// Pure's `Intro Screen` declares four - `IntroMovie1`, `FMV Movie`,
/// `ProfileMovie1`, `ProfileMovie2` - and a single-slot field kept whichever
/// parsed last, so `boot`'s "the widget decides whether it is heard" rule
/// could not find the widget for any of the other three. Shaped after the
/// real disc rather than invented.
#[test]
fn a_screen_keeps_every_movie_widget_it_declares() {
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen type="Intro" name="Intro Screen">
    <Movie name="IntroMovie1"><Values src="Data\Movies\IntroMovieP1" sound="true" localised="true"></Values></Movie>
    <Movie name="FMV Movie"><Values src="Data\Movies\WoFMVNew" sound="true" localised="true"></Values></Movie>
    <Movie name="ProfileMovie1"><Values src="Data\Movies\Profile_part1" sound="false"></Values></Movie>
  </Screen>
</Screen>
"#,
    );
    let screen = screens.by_name("Intro Screen").unwrap();
    let names: Vec<String> = screen.movies.iter().map(Movie::entry_name).collect();
    assert_eq!(
        names,
        vec![
            r"Data\Movies\IntroMovieP1_US.PMF",
            r"Data\Movies\WoFMVNew_US.PMF",
            r"Data\Movies\Profile_part1.PMF",
        ],
        "document order, and none of them dropped"
    );
    // The whole point: a widget that is neither first nor last is findable,
    // so its own `sound` is what decides.
    let found = screen
        .movies
        .iter()
        .find(|m| m.entry_name() == r"Data\Movies\WoFMVNew_US.PMF")
        .expect("the second boot movie's widget");
    assert!(found.sound);
    assert!(
        !screen.movies[2].sound,
        "a sound=\"false\" widget behind others still mutes its own movie"
    );
    assert_eq!(screens.with_movies().count(), 1);
}

#[test]
fn the_movie_filename_is_built_from_src() {
    let screens = Screens::from_xml(SAMPLE);
    let movie = screens.by_name("LogoFMV").unwrap().movies[0].clone();
    assert_eq!(movie.entry_name(), r"Data\Movies\Intro.PMF");

    let localised = Movie {
        localised: true,
        ..movie
    };
    assert_eq!(localised.entry_name(), r"Data\Movies\Intro_US.PMF");
}

#[test]
fn a_ps2_src_already_names_its_container() {
    // Both spellings are read off the PS2 disc's own Skin.xml. Appending
    // .PMF to either asks for a file that exists nowhere, which is what
    // this build did before: `Data\Movies\Backdrop.ipf.PMF`.
    let screens = Screens::from_xml(SAMPLE);
    let template = screens.by_name("LogoFMV").unwrap().movies[0].clone();

    for src in [r"Data\Movies\Intro.pss", r"Data\Movies\Backdrop.ipf"] {
        let movie = Movie {
            src: src.to_string(),
            ..template.clone()
        };
        assert_eq!(movie.entry_name(), src);
        // `localised` must not reintroduce the suffix either.
        let localised = Movie {
            localised: true,
            ..movie
        };
        assert_eq!(localised.entry_name(), src);
    }
}

#[test]
fn an_extensionless_src_still_gets_the_psp_suffix() {
    // The PSP rule is settled and this is its guard: nothing about the PS2
    // divergence may change what `Data\Movies\Intro` resolves to.
    let screens = Screens::from_xml(SAMPLE);
    let movie = screens.by_name("LogoFMV").unwrap().movies[0].clone();
    assert!(!has_movie_extension(&movie.src));
    assert_eq!(movie.entry_name(), r"Data\Movies\Intro.PMF");
}

#[test]
fn finds_the_language_selection_screen_two_ways() {
    let screens = Screens::from_xml(SAMPLE);
    let screen = screens.language_selection().unwrap();
    assert_eq!(screen.name, "Language Selection");
    assert_eq!(screen.kind.as_deref(), Some("Language Selection"));
    assert!(screen.display_languages);
    let menu = screen.menu.as_ref().unwrap();
    assert_eq!(menu.name, "Language");
}

#[test]
fn the_menu_widget_s_own_layout_resolves_through_feglobals() {
    let screens = Screens::from_xml(SAMPLE);
    let menu = screens
        .by_name("Language Selection")
        .unwrap()
        .menu
        .as_ref()
        .unwrap();
    assert_eq!(menu.x, 50.0, "x came from FEGlobals->MenuXOffset");
    assert_eq!(menu.y, 46.0);
    assert_eq!(menu.scale, 1.0, "scale came from FEGlobals->MenuScale");
    assert_eq!(
        menu.color, 0xff33_a6b9,
        "color came from FEGlobals->TextColor"
    );
    assert_eq!(menu.align, "left");
    assert_eq!(menu.font, "Default");
}

#[test]
fn resolves_the_feglobals_indirection_across_a_newline() {
    let screens = Screens::from_xml(SAMPLE);
    let text = &screens.by_name("Language Selection").unwrap().texts[0];
    assert_eq!(text.x, 50.0, "x came from FEGlobals->MenuXOffset");
    assert_eq!(text.color, 0xff00_0000);
    assert_eq!(text.idstring.as_deref(), Some("Language Selection"));
    assert!(!text.start_enabled);
}

#[test]
fn redirects_map_buttons_to_screens() {
    let screens = Screens::from_xml(SAMPLE);
    let logo = screens.by_name("LogoFMV").unwrap();

    let auto = logo.redirect_named("AutoRedirect").unwrap();
    assert_eq!(auto.goto.as_deref(), Some("Show Logo"));
    assert_eq!(auto.forward, None, "\"none\" is not a button");

    let start = logo.redirect_for(Button::Start).unwrap();
    assert_eq!(start.goto.as_deref(), Some("LogoFMVRedirectScreen"));
}

#[test]
fn nested_screens_get_a_hierarchical_path() {
    let screens = Screens::from_xml(SAMPLE);
    // `Show Logo` is nested three deep and **not** under `LogoFMV`, which
    // only names it as a `goto` target. Worth asserting precisely, because
    // its parent is where the menu backdrop movie lives.
    assert_eq!(
        screens.by_name("Show Logo").unwrap().path,
        "Top FE Screen->FE Screen->Show Logo"
    );
    assert_eq!(
        screens.by_name("FE Screen").unwrap().path,
        "Top FE Screen->FE Screen"
    );
    assert_eq!(screens.by_name("LogoFMV").unwrap().path, "LogoFMV");
}

#[test]
fn show_logo_advances_on_start_and_nothing_else() {
    let screens = Screens::from_xml(SAMPLE);
    let logo = screens.by_name("Show Logo").unwrap();

    let start = logo.redirect_for(Button::Start).unwrap();
    assert_eq!(start.goto.as_deref(), Some("RemoveMemoryStickWarning"));
    // `LogoFMV` takes all five; the screen that says "Press START button"
    // takes one. Nothing else on it is a redirect with a button, and there
    // is no timer of any kind.
    assert!(logo.redirect_for(Button::Cross).is_none());
    assert!(logo.redirect_for(Button::Circle).is_none());
    assert_eq!(logo.redirects.len(), 2);
}

#[test]
fn a_text_inside_a_viewport_is_collected_alongside_its_siblings() {
    // `BOOT_LEGAL` sits inside a `Viewport`, and `collect_widgets` recurses
    // through one rather than stopping at it - so both the screen's direct
    // text and its viewport-nested one are read. Order is document order:
    // `BOOT_PRESS_START` is declared before the `Viewport`. See
    // `docs/architecture/frontend-boot.md`.
    let screens = Screens::from_xml(SAMPLE);
    let logo = screens.by_name("Show Logo").unwrap();
    let ids: Vec<&str> = logo
        .texts
        .iter()
        .filter_map(|t| t.idstring.as_deref())
        .collect();
    assert_eq!(ids, ["BOOT_PRESS_START", "BOOT_LEGAL"]);
}

#[test]
fn the_press_start_widget_keeps_its_own_layout() {
    let screens = Screens::from_xml(SAMPLE);
    let text = &screens.by_name("Show Logo").unwrap().texts[0];
    assert_eq!(text.x, 460.0);
    assert_eq!(text.y, 220.0, "230 on the EU disc, which has no BOOT_LEGAL");
    assert_eq!(text.align, "right");
    assert_eq!(text.color, 0x7fff_ffff);
    assert_eq!(text.font, "Menu");
}

#[test]
fn the_logo_image_has_a_y_and_no_x() {
    // Which is what makes the centring rule in `frontend.rs` load-bearing.
    let screens = Screens::from_xml(SAMPLE);
    let image = &screens.by_name("Show Logo").unwrap().images[0];
    assert_eq!(image.src, r"Data\FE\Images\pulse_logo.mip");
    assert_eq!(image.y, 72.0);
    assert_eq!(image.x, 0.0);
    assert!(image.auto_load);
}

#[test]
fn collects_solid_colour_backdrops() {
    let screens = Screens::from_xml(SAMPLE);
    assert_eq!(screens.by_name("LogoFMV").unwrap().fills, vec![0xff00_0000]);
}

#[test]
fn collects_load_xml_sources() {
    let screens = Screens::from_xml(SAMPLE);
    assert_eq!(
        screens.load_xml,
        vec![r"Data\Plugins\PI001\GUI\MainMenu_Definition.xml"]
    );
}

#[test]
fn parses_argb() {
    assert_eq!(parse_argb("0xFF5FDBF6"), Some(0xff5f_dbf6));
    assert_eq!(parse_argb("nonsense"), None);
    assert_eq!(argb_to_rgba(0xff00_0000), [0.0, 0.0, 0.0, 1.0]);
}
