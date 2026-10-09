//! What the front-end XML reader in [`super`] is asserted to do: the nesting
//! and attribute rules, the `FEGlobals` indirection, the movie widgets and
//! their filenames, the redirects, and the colour spellings.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `screen.rs`: the tests are 335 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;
use oag_core::buttons::Button;

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
    let names: Vec<String> = screen.movies.iter().map(|m| m.entry_name("EU")).collect();
    assert_eq!(
        names,
        vec![
            r"Data\Movies\IntroMovieP1_EU.PMF",
            r"Data\Movies\WoFMVNew_EU.PMF",
            r"Data\Movies\Profile_part1.PMF",
        ],
        "document order, and none of them dropped"
    );
    // The whole point: a widget that is neither first nor last is findable,
    // so its own `sound` is what decides.
    let found = screen
        .movies
        .iter()
        .find(|m| m.entry_name("EU") == r"Data\Movies\WoFMVNew_EU.PMF")
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
    // `region` is ignored entirely when the widget is not `localised`.
    assert_eq!(movie.entry_name("EU"), r"Data\Movies\Intro.PMF");
    assert_eq!(movie.entry_name("US"), r"Data\Movies\Intro.PMF");

    let localised = Movie {
        localised: true,
        ..movie
    };
    // A `localised` widget takes whichever region the caller resolved -
    // `Movie_ParseAttributes` bakes exactly one such suffix per pressing's own
    // executable rather than reading one at runtime. See
    // `docs/ghidra/functions/psp-pure-eu/movie-localised-suffix.md`.
    assert_eq!(localised.entry_name("EU"), r"Data\Movies\Intro_EU.PMF");
    assert_eq!(localised.entry_name("US"), r"Data\Movies\Intro_US.PMF");
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
        assert_eq!(movie.entry_name("EU"), src);
        // `localised` must not reintroduce the suffix either.
        let localised = Movie {
            localised: true,
            ..movie
        };
        assert_eq!(localised.entry_name("EU"), src);
    }
}

#[test]
fn an_extensionless_src_still_gets_the_psp_suffix() {
    // The PSP rule is settled and this is its guard: nothing about the PS2
    // divergence may change what `Data\Movies\Intro` resolves to.
    let screens = Screens::from_xml(SAMPLE);
    let movie = screens.by_name("LogoFMV").unwrap().movies[0].clone();
    assert!(!has_movie_extension(&movie.src));
    assert_eq!(movie.entry_name("EU"), r"Data\Movies\Intro.PMF");
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
    assert_eq!(
        text.wrap_width, None,
        "no widthlimited attribute, so nothing to wrap against"
    );
}

/// `BOOT_LEGAL`'s `widthlimited="true"` resolves to its enclosing
/// `Viewport`'s own `width` - the number `crate::render` wraps against, not
/// a scissor rect. See `docs/architecture/frontend-boot.md`.
#[test]
fn a_widthlimited_text_wraps_against_its_viewports_width() {
    let screens = Screens::from_xml(SAMPLE);
    let legal = &screens.by_name("Show Logo").unwrap().texts[1];
    assert_eq!(legal.idstring.as_deref(), Some("BOOT_LEGAL"));
    assert_eq!(legal.wrap_width, Some(400.0));
}

/// `widthlimited="true"` with no enclosing `Viewport` has nothing to wrap
/// against, so it stays `None` rather than wrapping at some invented default.
#[test]
fn widthlimited_outside_a_viewport_has_no_wrap_width() {
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen name="Loose">
    <Text><Values idstring="LOOSE" font="small" widthlimited="true"></Values></Text>
  </Screen>
</Screen>
"#,
    );
    let text = &screens.by_name("Loose").unwrap().texts[0];
    assert_eq!(text.wrap_width, None);
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
    // No `U`/`V` on this widget, so nothing to sample but the whole texture.
    assert_eq!(image.u, None);
    assert_eq!(image.v, None);
    assert_eq!(image.texture_width, None);
    assert_eq!(image.texture_height, None);
}

#[test]
fn an_image_with_u_v_captures_its_own_sub_rect() {
    // `ArrowSelect`'s own attributes, off `pure-psp-usa.chd`'s `Skin.xml`:
    // one widget sampling one corner of a texture several other widgets
    // share.
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen name="Loose">
    <Image name="ArrowSelect">
      <Values x="0" y="0" width="6" height="11" U="53" V="0" TxtrWidth="6" TxtrHeight="11" Color="0xFFFFFFFF" src="Data\FE\Images\FETextures.mip"></Values>
    </Image>
  </Screen>
</Screen>
"#,
    );
    let image = &screens.by_name("Loose").unwrap().images[0];
    assert_eq!(image.u, Some(53.0));
    assert_eq!(image.v, Some(0.0));
    assert_eq!(image.texture_width, Some(6.0));
    assert_eq!(image.texture_height, Some(11.0));
}

#[test]
fn an_image_wrapped_in_an_animation_is_still_collected() {
    // `TitleAnim1`'s own shape, off `pure-psp-usa.chd`'s `Skin.xml`: a
    // `<Key>` reveal timeline wrapping the one widget it reveals. The
    // timeline is not modelled - only the widget has to survive.
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen name="Loose">
    <Animation name="TitleAnim1" focus="true">
      <Values Incbutton="none" Decbutton="none" Loop="false"></Values>
      <Key Time="0" TextureWidth="-1"></Key>
      <Key Time="0.1" TextureWidth="0"></Key>
      <Image>
        <Values x="14" y="240" width="1" height="16" TxtrWidth="1" TxtrHeight="16" Color="0xFFFFFFFF" src="Data\FE\Images\FETextures_startscreen.mip"></Values>
      </Image>
    </Animation>
  </Screen>
</Screen>
"#,
    );
    let images = &screens.by_name("Loose").unwrap().images;
    assert_eq!(
        images.len(),
        1,
        "the `Key` elements must not become widgets"
    );
    assert_eq!(images[0].x, 14.0);
    assert_eq!(images[0].y, 240.0);
}

#[test]
fn a_colour_only_image_inside_an_animation_keeps_its_own_rect_and_carries_the_reveal() {
    // `Demo_Definition.xml`'s own shape, shared by both Pulse's and Pure's
    // disc: a decorative underline, not a backdrop - `width="347" height="1"`
    // at `x="133" y="262"`. `Fill` carries its own rect precisely so this
    // draws as a thin line rather than washing the whole screen in its
    // colour, and now its own `<Key>` timeline too - see [`RevealKey`].
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen name="Loose">
    <Animation name="Anim">
      <Values></Values>
      <Key Time="0" TextureWidth="-347"></Key>
      <Key Time="0.5" TextureWidth="0"></Key>
      <Image>
        <Values x="133" y="262" width="347" height="1" color="0xff3abcf2"></Values>
      </Image>
    </Animation>
  </Screen>
</Screen>
"#,
    );
    let screen = screens.by_name("Loose").unwrap();
    assert_eq!(
        screen.fills,
        vec![Fill {
            name: None,
            x: 133.0,
            y: 262.0,
            width: Some(347.0),
            height: Some(1.0),
            color: 0xff3a_bcf2,
            gradient: None,
            reveal: vec![
                RevealKey {
                    time: 0.0,
                    texture_width: -347.0,
                },
                RevealKey {
                    time: 0.5,
                    texture_width: 0.0,
                },
            ],
            transition: 0.0,
        }]
    );
    assert!(
        screen.images.is_empty(),
        "a colour-only widget is not a textured one either"
    );
}

#[test]
fn an_animation_wrapped_fill_resolves_a_feglobals_colour() {
    // `Title Screen`'s own frame lines author `Color="FEGlobals->
    // FrameLineColor"`, unlike `Demo_Definition.xml`'s literal one - the
    // fill branch has to resolve the indirection the same way `image_from_node`
    // and `screenclear` already do, or these stay invisible even with a rect.
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Variable global="FrameLineColor"><Values String="0xFFFFFFFF"></Values></Variable>
  <Screen name="Loose">
    <Animation name="Anim">
      <Values></Values>
      <Image>
        <Values x="14" y="240" width="1" height="16" Color="FEGlobals->FrameLineColor"></Values>
      </Image>
    </Animation>
  </Screen>
</Screen>
"#,
    );
    let screen = screens.by_name("Loose").unwrap();
    assert_eq!(
        screen.fills,
        vec![Fill {
            name: None,
            x: 14.0,
            y: 240.0,
            width: Some(1.0),
            height: Some(16.0),
            color: 0xffff_ffff,
            gradient: None,
            reveal: Vec::new(),
            transition: 0.0,
        }]
    );
}

#[test]
fn an_animation_inside_a_viewport_still_carries_the_wrap_width_down() {
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen name="Loose">
    <Viewport>
      <Values width="400"></Values>
      <Animation name="Anim">
        <Values></Values>
        <Text><Values idstring="X" widthlimited="true"></Values></Text>
      </Animation>
    </Viewport>
  </Screen>
</Screen>
"#,
    );
    let text = &screens.by_name("Loose").unwrap().texts[0];
    assert_eq!(text.wrap_width, Some(400.0));
}

#[test]
fn collects_solid_colour_backdrops() {
    let screens = Screens::from_xml(SAMPLE);
    assert_eq!(
        screens.by_name("LogoFMV").unwrap().fills,
        vec![Fill {
            name: None,
            x: 0.0,
            y: 0.0,
            width: Some(480.0),
            height: Some(272.0),
            color: 0xff00_0000,
            gradient: None,
            reveal: Vec::new(),
            transition: 0.0,
        }]
    );
}

#[test]
fn collects_load_xml_sources() {
    let screens = Screens::from_xml(SAMPLE);
    assert_eq!(
        screens.load_xml,
        vec![r"Data\Plugins\PI001\GUI\MainMenu_Definition.xml"]
    );
}

/// `TitleFrame`'s own shape, off `pure-psp-usa.chd`'s `Skin.xml`: a widget
/// with a rect and a texture size but no `src` at all, assigned
/// programmatically on the original.
const SRCLESS_IMAGE: &str = r#"
<Screen>
  <Screen name="Title Screen">
    <Image name="TitleFrame" StartEnabled="false">
      <Values width="480" x="0" y="76" height="128" TxtrWidth="480" TxtrHeight="128"></Values>
    </Image>
  </Screen>
</Screen>
"#;

/// Wipeout HD/Fury's own `EndRace Results` shape, cut down to the one thing
/// this test cares about: a `<NavigationController>` nested directly inside
/// a named screen, the same as `EndRace_Definition.xml`'s own copy
///.
const ENDRACE_RESULTS_WITH_NAV_CONTROLLER: &str = r#"
<Screen>
  <Screen type="EndRace Results" name="EndRace Results">
    <Text name="ResultsTitle"><Values idstring="ER_RESULTS"></Values></Text>
    <NavigationController name="NavigationController">
      <Text name="ControlTextConfirmButton"><Values idstring="FE_CONFIRM_BUTTON" font="buttons"></Values></Text>
      <Text name="ControlTextConfirm"><Values idstring="FE_CONFIRM"></Values></Text>
    </NavigationController>
  </Screen>
</Screen>
"#;

/// The gap `docs/formats/hd-endrace-screens.md`'s own widget table named -
/// "that container is not walked" - closed: a `NavigationController`'s own
/// `Text` children now reach `screen.texts` the same as any other widget's,
/// scoped to the named screen that encloses them rather than leaking
/// sideways.
#[test]
fn a_navigation_controllers_text_children_reach_the_enclosing_named_screens_texts() {
    let screen = Screens::from_xml(ENDRACE_RESULTS_WITH_NAV_CONTROLLER)
        .by_name("EndRace Results")
        .unwrap()
        .clone();
    let names: Vec<Option<&str>> = screen
        .texts
        .iter()
        .map(|text| text.name.as_deref())
        .collect();
    assert_eq!(
        names,
        vec![
            Some("ResultsTitle"),
            Some("ControlTextConfirmButton"),
            Some("ControlTextConfirm"),
        ]
    );
}

#[test]
fn a_leftlayers_transition_is_inherited_by_everything_nested_under_it() {
    // `Selection_Definition.xml`'s own shape: one `LeftLayer` with no
    // `transition` at all (inherits the screen's default of zero, so its
    // text is not faded), one with `transition="0"` (explicit, same
    // result), and one with `transition="0.5"` wrapping an `Item` - whose
    // own children inherit it too, the same way `OffsetX`/`OffsetY` already
    // thread through `Item`.
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen name="Track Creation">
    <LeftLayer>
      <Text name="untouched" String="Track Select"></Text>
    </LeftLayer>
    <LeftLayer transition="0">
      <Text name="instant" String="1/3"></Text>
    </LeftLayer>
    <LeftLayer transition="0.5">
      <Image name="panel" width="170" height="200" color="0x2f000000"></Image>
      <Item OffsetY="59">
        <Text name="nested" String="SPEED"></Text>
      </Item>
    </LeftLayer>
  </Screen>
</Screen>
"#,
    );
    let screen = screens.by_name("Track Creation").unwrap();
    let text_transition = |name: &str| {
        screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some(name))
            .unwrap()
            .transition
    };
    assert_eq!(text_transition("untouched"), 0.0);
    assert_eq!(text_transition("instant"), 0.0);
    assert_eq!(text_transition("nested"), 0.5, "inherited through Item");
    assert_eq!(
        screen
            .fills
            .iter()
            .find(|fill| fill.name.as_deref() == Some("panel"))
            .unwrap()
            .transition,
        0.5
    );
}

#[test]
fn a_src_less_image_stays_dropped_with_no_fallback_table() {
    // The behaviour before `fallback_images` existed, pinned so a title with
    // an empty table (Pulse, HD) keeps it: nothing to sample, so nothing
    // drawn, rather than a guess.
    let screen = Screens::from_xml(SRCLESS_IMAGE)
        .by_name("Title Screen")
        .unwrap()
        .clone();
    assert!(screen.images.is_empty());
    assert!(
        screen.fills.is_empty(),
        "there is no `color` to fall back to either"
    );
}

#[test]
fn a_src_less_image_is_resolved_by_name_against_the_fallback_table() {
    let screens =
        Screens::from_xml_with_fallbacks(SRCLESS_IMAGE, &[], &[("TitleFrame", "hash:3af18d90")]);
    let image = &screens.by_name("Title Screen").unwrap().images[0];
    assert_eq!(image.src, "hash:3af18d90");
    assert_eq!((image.x, image.y), (0.0, 76.0));
    assert_eq!((image.width, image.height), (Some(480.0), Some(128.0)));
    assert_eq!(
        (image.texture_width, image.texture_height),
        (Some(480.0), Some(128.0))
    );
}

#[test]
fn a_real_src_in_the_xml_always_wins_over_the_fallback_table() {
    // The same contract `fallback_globals` carries for a colour: a name in
    // the table is consulted only where the widget's own XML is silent.
    let screens = Screens::from_xml_with_fallbacks(
        r#"
<Screen>
  <Screen name="Title Screen">
    <Image name="TitleFrame">
      <Values width="480" x="0" y="76" height="128" src="Data\FE\Images\Real.mip"></Values>
    </Image>
  </Screen>
</Screen>
"#,
        &[],
        &[("TitleFrame", "hash:3af18d90")],
    );
    let image = &screens.by_name("Title Screen").unwrap().images[0];
    assert_eq!(image.src, r"Data\FE\Images\Real.mip");
}

#[test]
fn parses_argb() {
    assert_eq!(parse_argb("0xFF5FDBF6"), Some(0xff5f_dbf6));
    assert_eq!(parse_argb("nonsense"), None);
    assert_eq!(argb_to_rgba(0xff00_0000), [0.0, 0.0, 0.0, 1.0]);
}

#[test]
fn a_text_that_authors_its_own_offset_with_no_wrapping_item_positions_itself_by_it() {
    // The shape `Data\Plugins\Frontend\Gui\CellMode_Definition.xml` authors
    // for `Medals Title`/`Points Title`/`EPoints Title`: `OffsetY` directly
    // on the `<Text>` tag, no `x`/`y`, no wrapping `<Item>`. Before this was
    // fixed, `collect_widgets` folded a text's own `OffsetX`/`OffsetY` into
    // what its *children* saw but not into its own position, so this label
    // landed at `(179, 0)` instead of `(179, 300)` - see
    // `oag_ui_screens::campaign::hd`'s own module doc for the visible bug that was.
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen name="Grid Selection">
    <Item OffsetX="179">
      <Text name="Medals Title" OffsetY="300">
        <Values idstring="RC_POINTSACH"></Values>
        <Text name="Points">
          <Values string="00/16" x="30" y="36"></Values>
        </Text>
      </Text>
    </Item>
  </Screen>
</Screen>
"#,
    );
    let screen = screens.by_name("Grid Selection").unwrap();
    let title = screen
        .texts
        .iter()
        .find(|t| t.name.as_deref() == Some("Medals Title"))
        .unwrap();
    assert_eq!((title.x, title.y), (179.0, 300.0));
    // The child's own position already included the parent's `OffsetY`
    // before this fix and still does - `inner` was already threaded to
    // grandchildren, only the text's own position was wrong.
    let points = screen
        .texts
        .iter()
        .find(|t| t.name.as_deref() == Some("Points"))
        .unwrap();
    assert_eq!((points.x, points.y), (179.0 + 30.0, 300.0 + 36.0));
}

#[test]
fn an_image_used_as_a_positioned_container_collects_its_own_children() {
    // `CellMode_Definition.xml`'s own detail column: `<Image name="Event
    // Emblem" OffsetX="630" OffsetY="340">` wraps a `Text`/`Image`/`Text`
    // group the same way `Item`/`LeftLayer` wrap one elsewhere - before this
    // was fixed, the `"image"` arm never recursed into a widget's own
    // children at all, so the whole detail column was silently dropped.
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen name="Cell Selection">
    <Image name="Event Emblem" OffsetX="630" OffsetY="340">
      <Values x="64" y="0" width="128" height="128" src="Data\FE\Images\bracket.gtf"></Values>
      <Text name="Event Title">
        <Values idstring="RB_EVENT_TYPE" x="20" y="150"></Values>
      </Text>
      <Text name="Event">
        <Values string="Event line" x="50" y="182"></Values>
      </Text>
    </Image>
  </Screen>
</Screen>
"#,
    );
    let screen = screens.by_name("Cell Selection").unwrap();
    let emblem = screen
        .images
        .iter()
        .find(|i| i.name.as_deref() == Some("Event Emblem"))
        .unwrap();
    assert_eq!((emblem.x, emblem.y), (630.0 + 64.0, 340.0));
    let title = screen
        .texts
        .iter()
        .find(|t| t.name.as_deref() == Some("Event Title"))
        .unwrap();
    assert_eq!((title.x, title.y), (630.0 + 20.0, 340.0 + 150.0));
    let value = screen
        .texts
        .iter()
        .find(|t| t.name.as_deref() == Some("Event"))
        .unwrap();
    assert_eq!((value.x, value.y), (630.0 + 50.0, 340.0 + 182.0));
}

/// Wipeout 2048's own widget shapes: a `LoadXML` with `SrcRel`, `localised`
/// and `DirectEmbed`, a `Redirect` with a `delay`, a `Centred` image, a
/// `vertalign="middle"` text and a row of `TouchButton`s.
#[test]
fn reads_the_touch_front_ends_own_attributes() {
    let screens = Screens::from_xml(
        r#"
<Screen>
  <LoadXML><Values SrcRel="Definition.xml"></Values></LoadXML>
  <LoadXML><Values SrcRel="Bootup_Definition.xml" localised="true"></Values></LoadXML>
  <Screen name="Boot Studio Logo">
    <Image><Values x="480" y="272" width="960" height="128" Centred="true" src="Data\FE\Images\StudioLogo.gtf"></Values></Image>
    <Redirect name="Redirect" delay="4.0"><Values backward="none" forward="none"></Values><Default goto="Boot Intro Movie"></Default></Redirect>
    <Redirect name="Redirectbutton"><Values backward="none" forward="cross"></Values><Default goto="Boot Intro Movie"></Default></Redirect>
  </Screen>
  <Screen name="TitleScreen">
    <Text><Values x="480" y="370" align="centre" vertalign="middle" idstring="BOOT_PRESS_ANY"></Values></Text>
    <LoadXML><Values SrcRel="Legal_Line_Definition.xml" localised="true" DirectEmbed="true"></Values></LoadXML>
  </Screen>
  <Screen name="GameModeChoice" type="GameModeChoice_Screen">
    <TouchButton name="offline" delay="0">
      <Values idstring="FE_SP_CAMPAIGN" x="167" y="190" width="140" height="140" toggle="true" StringWidthLimit="175" Src="Data\FE\NewImages\gamemodechoice\gm_SPCampaign.gtf"></Values>
    </TouchButton>
    <TouchButton name="confirmButton">
      <Values string="" redirect="newFEshell" x="822" y="432" width="122" height="96" Src="Data\FE\NewImages\Icon_Tick.gtf"></Values>
    </TouchButton>
  </Screen>
</Screen>
"#,
    );
    assert_eq!(
        screens.includes,
        vec![
            Include {
                src: "Definition.xml".to_string(),
                relative: true,
                localised: false,
                direct_embed: false,
                screen: None,
            },
            Include {
                src: "Bootup_Definition.xml".to_string(),
                relative: true,
                localised: true,
                direct_embed: false,
                screen: None,
            },
            Include {
                src: "Legal_Line_Definition.xml".to_string(),
                relative: true,
                localised: true,
                direct_embed: true,
                screen: Some("TitleScreen".to_string()),
            },
        ]
    );
    assert!(screens.load_xml.is_empty(), "`SrcRel` is not `src`");

    let card = screens.by_name("Boot Studio Logo").unwrap();
    assert!(card.images[0].centred);
    assert_eq!((card.images[0].x, card.images[0].y), (480.0, 272.0));
    assert_eq!(card.redirects[0].delay, Some(4.0));
    assert_eq!(card.redirects[1].delay, None);
    assert_eq!(card.redirects[1].forward, Some(Button::Cross));

    let title = screens.by_name("TitleScreen").unwrap();
    assert!(title.texts[0].middle);

    let grid = screens.by_name("GameModeChoice").unwrap();
    assert_eq!(grid.touch_buttons.len(), 2);
    let offline = &grid.touch_buttons[0];
    assert_eq!(offline.name.as_deref(), Some("offline"));
    assert_eq!(offline.idstring.as_deref(), Some("FE_SP_CAMPAIGN"));
    assert_eq!(
        (offline.x, offline.y, offline.width, offline.height),
        (167.0, 190.0, 140.0, 140.0)
    );
    assert!(offline.toggle);
    assert_eq!(offline.redirect, None);
    assert_eq!(offline.string_width_limit, Some(175.0));
    assert_eq!(
        offline.src.as_deref(),
        Some(r"Data\FE\NewImages\gamemodechoice\gm_SPCampaign.gtf")
    );
    let tick = &grid.touch_buttons[1];
    assert_eq!(tick.idstring, None);
    assert_eq!(tick.string, None, "an empty `string` is no label");
    assert_eq!(tick.redirect.as_deref(), Some("newFEshell"));
    assert!(!tick.toggle);
}

/// [`interpolate_reveal`]'s own shape, off `Animation_InterpolateKeys`
/// (`docs/ghidra/functions/psp-pure-eu/title-screen.md`): linear between the
/// two keys bracketing `elapsed`, holding the boundary key's own value
/// before the first and after the last, and `0.0` with no keys at all - the
/// pre-existing "no `<Animation>` wraps this widget" case, unchanged.
#[test]
fn interpolate_reveal_holds_at_the_ends_and_is_linear_between_keys() {
    let keys = [
        RevealKey {
            time: 0.0,
            texture_width: -100.0,
        },
        RevealKey {
            time: 0.63,
            texture_width: -100.0,
        },
        RevealKey {
            time: 0.65,
            texture_width: 0.0,
        },
    ];
    assert_eq!(
        interpolate_reveal(&[], 0.0),
        0.0,
        "no keys: no widget wraps"
    );
    assert_eq!(
        interpolate_reveal(&keys, -1.0),
        -100.0,
        "before the first key"
    );
    assert_eq!(interpolate_reveal(&keys, 0.0), -100.0, "at the first key");
    assert_eq!(
        interpolate_reveal(&keys, 0.3),
        -100.0,
        "the held middle span"
    );
    assert_eq!(
        interpolate_reveal(&keys, 0.63),
        -100.0,
        "at the hold's own end"
    );
    assert_eq!(
        interpolate_reveal(&keys, 0.64),
        -50.0,
        "halfway through the fast final transition"
    );
    assert_eq!(interpolate_reveal(&keys, 0.65), 0.0, "at the final key");
    assert_eq!(interpolate_reveal(&keys, 10.0), 0.0, "after the final key");
}
