//! What each screen puts on the display list: the logo and its press-start
//! line, the picker rows, the video quad, and the near-black lift.
//!
//! Split out of `frontend/tests.rs` under the file-length rule in
//! `scripts/check-file-size.py`.

use super::*;

/// `BOOT_PRESS_START`'s own `Draw::Text` out of a draw list, and its alpha.
fn press_start(draws: &[Draw]) -> (f32, f32, Align, [f32; 4], String) {
    draws
        .iter()
        .find_map(|d| match d {
            Draw::Text {
                x,
                y,
                align,
                color,
                text,
                ..
            } if text == "BOOT_PRESS_START" => Some((*x, *y, *align, *color, text.clone())),
            _ => None,
        })
        .expect("Show Logo draws its text widget")
}

#[test]
fn show_logo_draws_the_pulse_logo_and_its_press_start_line() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    pick_a_language(&mut frontend, &mut input);
    assert!(frontend.machine().is(states::SHOW_LOGO));

    let draws = frontend.draw_list();
    // The `Text` widget's own coordinates, colour and alignment, from the
    // USA disc's `Skin.xml`. `y` is 220 there and 230 on the EU disc, which
    // drops the `BOOT_LEGAL` line above it.
    let text = press_start(&draws);
    assert_eq!(text.0, 460.0);
    assert_eq!(text.1, 220.0);
    assert_eq!(text.2, Align::Right);
    // Alpha, not colour, is where `pulse="true"` and `delay="1"` act - see
    // `pulse_below_its_delay_is_invisible_and_above_it_throbs`. `pick_a_language`
    // has just fired `Show Logo`, so `on_screen_for` is a few frames, well under
    // the one-second `delay`: the widget is not drawn at all yet, on hardware or
    // here, which is why its own alpha channel reads zero even though its RGB
    // is the XML's own `0x7FFFFFFF`.
    let [r, g, b, a] = text.3;
    assert_eq!([r, g, b], [1.0, 1.0, 1.0]);
    assert_eq!(a, 0.0, "still within its one-second delay");
    assert_eq!(
        text.4, "BOOT_PRESS_START",
        "with no string table loaded the id stands in for its own string"
    );

    // The logo itself needs a sprite sheet, which these tests do not build.
    // What must hold either way is that the screen's `Image` was found and
    // nothing else crept in: a black fill, then the text.
    assert!(matches!(draws[0], Draw::Fill { .. }));
    assert!(!draws.iter().any(|d| matches!(d, Draw::Video { .. })));
}

/// Below `delay`, invisible; above it, throbbing between the measured floor
/// and its own authored (ceiling) alpha - `docs/architecture/frontend-boot.md`'s
/// `BOOT_PRESS_START does not pulse` section, confidence 75.
#[test]
fn pulse_below_its_delay_is_invisible_and_above_it_throbs() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    pick_a_language(&mut frontend, &mut input);
    assert!(frontend.machine().is(states::SHOW_LOGO));
    let ceiling = argb_to_rgba(0x7fff_ffff)[3];

    // `on_screen_for` already carries a few frames from `pick_a_language`
    // itself; read it back rather than assuming zero, so the times below
    // land exactly rather than a few milliseconds short.
    let already = frontend.on_screen_for;

    // A hair under one second: still within `delay="1"`.
    input.begin_frame(0);
    frontend.update(0.999 - already, &mut input, None);
    let (.., color, _) = press_start(&frontend.draw_list());
    assert_eq!(color[3], 0.0, "0.999s: still below the one-second delay");

    // A hair over it: the throb has started, so alpha is no longer zero, but
    // it has not yet had a full period to ramp up to its own ceiling either.
    input.begin_frame(0);
    frontend.update(0.01, &mut input, None); // now at 1.009s
    let (.., color, _) = press_start(&frontend.draw_list());
    assert!(
        color[3] > 0.0 && color[3] < ceiling,
        "1.009s: past the delay and ramping in, got alpha {}",
        color[3]
    );

    // `draw::widget_alpha::PULSE_PERIOD`/`PULSE_FLOOR`: five and a quarter periods past the
    // delay is a sine peak (`sin(0.25 * tau) == 1`) with the one-period ramp
    // long since at 1.0, so alpha lands exactly on the widget's own ceiling.
    let period = f64::from(draw::widget_alpha::PULSE_PERIOD);
    input.begin_frame(0);
    frontend.update(5.25 * period - 0.009, &mut input, None); // now at 1 + 5.25 periods
    let (.., color, _) = press_start(&frontend.draw_list());
    assert!(
        (color[3] - ceiling).abs() < 1e-5,
        "a sine peak, fully ramped in, should sit on the ceiling; got {}",
        color[3]
    );

    // Half a period later is the matching trough (`sin(0.75 * tau) == -1`),
    // at the measured floor fraction of that same ceiling.
    input.begin_frame(0);
    frontend.update(0.5 * period, &mut input, None); // now at 1 + 5.75 periods
    let (.., color, _) = press_start(&frontend.draw_list());
    let floor = ceiling * draw::widget_alpha::PULSE_FLOOR;
    assert!(
        (color[3] - floor).abs() < 1e-5,
        "a sine trough should sit on the measured floor fraction of the \
         ceiling ({floor}); got {}",
        color[3]
    );
}

/// `draw_screen` - the public, `--screen`-and-tests-facing entry point - is
/// unaffected: it is documented to freeze any pulsing widget at its own
/// ceiling rather than mid-throb, and this is the regression test for that
/// promise. Only the live boot order (`draw_list`, tested above) animates.
#[test]
fn draw_screen_freezes_a_pulsing_widget_at_its_ceiling() {
    let frontend = frontend(300);
    let draws = frontend.draw_screen(states::SHOW_LOGO);
    let (.., color, _) = press_start(&draws);
    assert_eq!(color, argb_to_rgba(0x7fff_ffff));
}

/// `BOOT_LEGAL`'s `Viewport`-derived `wrap_width` survives `draw_screen`
/// unwrapped - the actual line-splitting is `crate::render`'s job, since only
/// it holds the font metrics to split correctly. This is its own fixture
/// rather than [`XML`]'s, which deliberately omits the `Viewport` (see its own
/// comment) to keep the shared boot-sequence tests free of it.
#[test]
fn a_widthlimited_text_carries_its_wrap_width_into_the_draw_list() {
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen type="FEMain" name="Top FE Screen">
    <Screen name="FE Screen">
      <Screen name="Show Logo">
        <Viewport>
          <Values x="40" y="0" height="480" width="400"></Values>
          <Text name="USLegalText" delay="0" transition="0">
            <Values align="left" idstring="BOOT_LEGAL" font="small" scale="0.7" x="40" y="250" widthlimited="true" color="0x7FFFFFFF"></Values>
          </Text>
        </Viewport>
      </Screen>
    </Screen>
  </Screen>
</Screen>
"#,
    );
    let frontend = Frontend::new(
        screens,
        StringTable::default(),
        Vec::new(),
        Vec::new(),
        0,
        false,
    );
    let draws = frontend.draw_screen("Show Logo");
    let legal = draws
        .iter()
        .find_map(|d| match d {
            Draw::Text {
                text, wrap_width, ..
            } if text == "BOOT_LEGAL" => Some(*wrap_width),
            _ => None,
        })
        .expect("Show Logo draws BOOT_LEGAL");
    assert_eq!(legal, Some(400.0));
}

/// Two widgets sharing one texture at different `U`/`V` sub-rects, the shape
/// Pure's `Title Screen->Press start button` and its neighbours author on the
/// real disc: without `U`/`V` reaching the draw list, both would sample the
/// same patch - the texture's own top-left corner - rather than their own.
#[test]
fn two_images_sharing_a_texture_sample_their_own_sub_rects() {
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen name="Loose">
    <Image name="First"><Values x="10" y="20" width="23" height="11" U="0" V="15" TxtrWidth="23" TxtrHeight="12" src="Data\FE\Images\FETextures_startscreen.mip"></Values></Image>
    <Image name="Second"><Values x="200" y="240" width="16" height="9" U="26" V="15" TxtrWidth="16" TxtrHeight="9" src="Data\FE\Images\FETextures_startscreen.mip"></Values></Image>
  </Screen>
</Screen>
"#,
    );
    let placements = vec![(
        r"Data\FE\Images\FETextures_startscreen.mip".to_string(),
        crate::frontend::Placed {
            x: 100,
            y: 50,
            width: 128,
            height: 64,
            quad_extent: None,
            blend: None,
        },
    )];
    let frontend = Frontend::new(
        screens,
        StringTable::default(),
        Vec::new(),
        placements,
        0,
        false,
    );
    let draws = frontend.draw_screen("Loose");
    let sprites: Vec<_> = draws
        .iter()
        .filter_map(|d| match d {
            Draw::Sprite { rect, uv, .. } => Some((*rect, *uv)),
            _ => None,
        })
        .collect();
    assert_eq!(sprites.len(), 2, "both images must reach the draw list");
    // The atlas offset (100, 50) plus each widget's own `U`/`V`, and its own
    // `TxtrWidth`/`TxtrHeight` rather than the whole placed texture.
    assert_eq!(
        sprites[0],
        ([10.0, 20.0, 23.0, 11.0], [100.0, 65.0, 23.0, 12.0])
    );
    assert_eq!(
        sprites[1],
        ([200.0, 240.0, 16.0, 9.0], [126.0, 65.0, 16.0, 9.0])
    );
}

/// A colour-only `Image` with its own `width`/`height` draws at its own
/// rect, not across the whole screen - `Demo_Definition.xml`'s own shape,
/// shared by both Pulse's and Pure's disc: a decorative underline, not a
/// backdrop. One with neither still falls back to the whole screen, which is
/// what every colour-only `Image` measured without one actually is (`Title
/// Screen`'s and `Show Logo`'s own white/black background).
#[test]
fn a_colour_only_image_draws_its_own_rect_and_falls_back_to_the_whole_screen() {
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen name="Loose">
    <Image name="Line"><Values x="133" y="262" width="347" height="1" color="0xff3abcf2"></Values></Image>
    <Image name="Backdrop"><Values color="0xff000000"></Values></Image>
  </Screen>
</Screen>
"#,
    );
    let frontend = Frontend::new(
        screens,
        StringTable::default(),
        Vec::new(),
        Vec::new(),
        0,
        false,
    );
    let draws = frontend.draw_screen("Loose");
    // `draw_screen`'s own base black clear comes first, ahead of anything
    // `Loose` itself authors - not this screen's own fills.
    let fills: Vec<_> = draws[1..]
        .iter()
        .filter_map(|d| match d {
            Draw::Fill { rect, color } => Some((*rect, *color)),
            _ => None,
        })
        .collect();
    assert_eq!(
        fills,
        vec![
            ([133.0, 262.0, 347.0, 1.0], argb_to_rgba(0xff3a_bcf2)),
            ([0.0, 0.0, SCREEN.0, SCREEN.1], argb_to_rgba(0xff00_0000)),
        ]
    );
}

/// An `<Animation>`-wrapped `Fill` grows from hidden to its own authored
/// width as `elapsed` advances past its own `<Key>` timeline - Pure's
/// `Title Screen` wraps its frame lines this way, and this pins the wipe
/// `docs/ghidra/functions/psp-pure-eu/title-screen.md` reads off a live
/// capture: `TextureWidth` interpolates from `-width` (fully hidden) to `0`
/// (fully shown), added straight onto the rect this build already draws.
#[test]
fn an_animation_wrapped_fill_grows_from_hidden_to_its_own_width_over_its_keys() {
    let screens = Screens::from_xml(
        r#"
<Screen>
  <Screen name="Loose">
    <Animation name="Anim">
      <Values></Values>
      <Key Time="0" TextureWidth="-100"></Key>
      <Key Time="1" TextureWidth="0"></Key>
      <Image><Values x="10" y="20" width="100" height="1" color="0xff3abcf2"></Values></Image>
    </Animation>
  </Screen>
</Screen>
"#,
    );
    let frontend = Frontend::new(
        screens,
        StringTable::default(),
        Vec::new(),
        Vec::new(),
        0,
        false,
    );
    let width_at = |elapsed: f64| {
        frontend
            .draw_screen_at("Loose", elapsed)
            .into_iter()
            .find_map(|d| match d {
                Draw::Fill { rect, .. } if rect[2] <= 100.0 => Some(rect[2]),
                _ => None,
            })
            .expect("the wrapped Fill must still reach the draw list")
    };
    assert_eq!(width_at(0.0), 0.0, "at the first key: fully hidden");
    assert_eq!(width_at(0.5), 50.0, "halfway between the two keys");
    assert_eq!(width_at(1.0), 100.0, "at the final key: fully shown");
    assert_eq!(
        width_at(f64::INFINITY),
        100.0,
        "settled, the same as draw_screen's own public INFINITY"
    );
}

/// The live boot order actually reveals `Title Screen`'s own `<Animation>`-
/// wrapped widgets - not just `Frontend::draw_screen_at` in isolation above.
/// `Frontend::draw`'s own `pure_states::TITLE_SCREEN` arm used to call the
/// public `draw_screen` (`elapsed = f64::INFINITY`), so a player watching a
/// real boot never saw the wipe at all, the same gap `Show Logo`'s own pulse
/// closed by taking `self.on_screen_for` instead.
#[test]
fn titles_screen_reveals_over_a_live_boot_not_just_settled() {
    let mut frontend = pure(60);
    let mut input = Input::new();
    reach_the_second_movie(&mut frontend, &mut input);
    run_until(&mut frontend, &mut input, 600, |f| {
        f.machine().is(pure_states::TITLE_SCREEN)
    });

    // The fixture's own `TitleAnimTest`-wrapped `Fill`, at `x=14 y=240
    // width=100 height=1`, brackets `TextureWidth` between `-100` (hidden)
    // at `Time=0` and `0` (shown) at `Time=0.2` - see `PURE_XML`.
    let underline_width = |draws: &[Draw]| {
        draws.iter().find_map(|d| match d {
            Draw::Fill { rect, .. } if rect[0] == 14.0 && rect[1] == 240.0 && rect[3] == 1.0 => {
                Some(rect[2])
            }
            _ => None,
        })
    };

    // `self.advance` resets `on_screen_for` to `0.0` on every transition,
    // `Title Screen`'s own included - read it back rather than assuming
    // exactly zero, the same caution `pulse_below_its_delay...` above takes.
    let already = frontend.on_screen_for;
    let early = underline_width(&frontend.draw_list())
        .expect("the wrapped Fill must reach the live draw list, not just draw_screen_at");
    assert_eq!(
        early, 0.0,
        "on entry: fully hidden, not already drawn settled"
    );

    input.begin_frame(0);
    frontend.update(0.3 - already, &mut input, None);
    let late = underline_width(&frontend.draw_list())
        .expect("the wrapped Fill must still reach the live draw list");
    assert_eq!(late, 100.0, "0.3s in, past the 0.2s key: fully revealed");
}

#[test]
fn circle_does_not_select() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);

    input.begin_frame(Button::Circle.bit());
    frontend.update(FRAME, &mut input, None);
    assert_eq!(
        frontend.chosen(),
        None,
        "cancel is circle, activate is cross"
    );
}

#[test]
fn the_discs_own_redirect_is_reported_not_followed() {
    let frontend = frontend(300);
    assert_eq!(frontend.language_auto_redirect(), Some("LogoFMV"));
}

#[test]
fn the_picker_draws_one_row_per_language() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);

    let draws = frontend.draw_list();
    let rows: Vec<&String> = draws
        .iter()
        .filter_map(|d| match d {
            Draw::Text { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert!(rows.iter().any(|t| *t == "Français"), "{rows:?}");
    assert!(rows.iter().any(|t| *t == "Deutsch"), "{rows:?}");
    assert!(rows.iter().any(|t| *t == "English"), "{rows:?}");
}

/// The row `y`s of the language names, in draw order.
fn picker_row_ys(frontend: &Frontend) -> Vec<f32> {
    let names = ["Français", "Deutsch", "English"];
    frontend
        .draw_list()
        .iter()
        .filter_map(|d| match d {
            Draw::Text { text, y, .. } if names.contains(&text.as_str()) => Some(*y),
            _ => None,
        })
        .collect()
}

#[test]
fn the_picker_steps_by_the_table_until_told_a_face_height() {
    let mut frontend = frontend(300);
    let mut input = Input::new();
    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    let ys = picker_row_ys(&frontend);
    assert_eq!(ys.len(), 3, "{ys:?}");
    let table = ys[1] - ys[0];
    assert_eq!(
        table,
        13.0 * frontend
            .language_rows(frontend.language_screen().and_then(|s| s.menu.as_ref()))
            .scale
    );

    frontend.set_picker_line_height(33.0);
    let ys = picker_row_ys(&frontend);
    let scale = frontend
        .language_rows(frontend.language_screen().and_then(|s| s.menu.as_ref()))
        .scale;
    assert_eq!(ys[1] - ys[0], 33.0 * scale, "{ys:?}");
    assert_eq!(ys[2] - ys[1], 33.0 * scale, "{ys:?}");
}

/// Regression for the picker's own row-blank bug: the selected row used to
/// be recoloured to opaque white, which Pure's white picker backdrop
/// (`Intro Screen`'s own `Image`, see `insert_backdrop_parent_fills`)
/// swallows completely - proven on the real disc by dumping the queued
/// `Draw::Text` and watching the text disappear at *every* row once it was
/// selected, not only the one a fresh boot lands on.
#[test]
fn the_selected_row_keeps_the_menus_own_colour_rather_than_turning_white() {
    let mut frontend = pure(60);
    assert!(frontend.machine().is(pure_states::LANGUAGE_SELECTION));
    assert_eq!(frontend.selected(), 0, "a fresh boot lands on row 0");

    let draws = frontend.draw_list();
    let selected_color = draws
        .iter()
        .find_map(|d| match d {
            Draw::Text { text, color, .. } if text == "English" => Some(*color),
            _ => None,
        })
        .expect("the selected row still queues its own text");
    // `PURE_XML`'s own `Menu` colour, not an invented "selected" one - see
    // the draw loop's own comment for why there is no second colour to use.
    assert_eq!(
        selected_color,
        argb_to_rgba(0xFF88D6E8),
        "opaque white here is exactly the invisible-on-white regression"
    );

    input_down(&mut frontend);
    assert_eq!(frontend.selected(), 1);
    let draws = frontend.draw_list();
    let now_unselected = draws
        .iter()
        .find_map(|d| match d {
            Draw::Text { text, color, .. } if text == "English" => Some(*color),
            _ => None,
        })
        .expect("row 0 still queues its text once it is no longer selected");
    assert_eq!(
        selected_color, now_unselected,
        "selected and unselected must share one colour: the highlight fill \
         is the only signal a row is picked"
    );
}

fn input_down(frontend: &mut Frontend) {
    let mut input = Input::new();
    input.begin_frame(Button::Down.bit());
    frontend.update(FRAME, &mut input, None);
}

#[test]
fn the_intro_draws_no_video_quad_without_a_picture() {
    let frontend = frontend(0);
    let draws = frontend.draw_list();
    assert!(!draws.iter().any(|d| matches!(d, Draw::Video { .. })));
}

#[test]
fn the_intro_draws_a_video_quad_when_there_is_one() {
    let frontend = Frontend::new(
        Screens::from_xml(XML),
        StringTable::default(),
        languages(),
        Vec::new(),
        300,
        true,
    );
    let draws = frontend.draw_list();
    assert!(draws.iter().any(|d| matches!(d, Draw::Video { .. })));
}

#[test]
fn near_black_text_is_lifted_off_a_black_screen() {
    assert_ne!(lighten([0.0, 0.0, 0.0, 1.0]), [0.0, 0.0, 0.0, 1.0]);
    assert_eq!(lighten([1.0, 0.5, 0.2, 1.0]), [1.0, 0.5, 0.2, 1.0]);
}

#[test]
fn an_empty_language_list_does_not_panic() {
    let mut frontend = Frontend::new(
        Screens::from_xml(XML),
        StringTable::default(),
        Vec::new(),
        Vec::new(),
        300,
        false,
    );
    let mut input = Input::new();
    input.begin_frame(Button::Start.bit());
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(0);
    frontend.update(FRAME, &mut input, None);
    input.begin_frame(Button::Cross.bit());
    frontend.update(FRAME, &mut input, None);
    assert_eq!(frontend.chosen(), None);
    let _ = frontend.draw_list();
}
