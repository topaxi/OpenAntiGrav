//! What the front-end state machine in [`super`] is asserted to do.
//!
//! Its own directory rather than a `#[cfg(test)]` block at the end of
//! `frontend.rs`: the tests are 1,235 lines and the state machine is 1,990,
//! so the module that had to be read to change one screen was nearly half
//! something else. See `scripts/check-file-size.py`, which is the rule as a
//! gate.
//!
//! This file holds only the fixtures the themes below share; each theme is a
//! file of its own, split along the seams the tests already had.

mod aspect;
mod backdrop;
mod drawing;
mod intro;
mod screens;

use super::*;

const FRAME: f64 = 1001.0 / 30_000.0;

const XML: &str = r#"
<Screen>
  <Screen type="Language Selection" name="Language Selection">
<Text name="LanguageText"><Values idstring="Language Selection" font="Title" x="21" y="0"></Values></Text>
<DisplayLanguages><Values clear="true"></Values></DisplayLanguages>
<Menu name="Language"><Values align="left" font="Default" x="50" scale="1.0" y="46" color="0xFF33A6B9"></Values></Menu>
<Redirect name="LanguageAutoRedirect">
  <Values backward="none"></Values>
  <Default goto="LogoFMV"></Default>
</Redirect>
  </Screen>
  <Screen name="LogoFMV">
<Movie><Values src="Data\Movies\Intro" autostart="true" autoredirect="true"></Values></Movie>
<Redirect name="AutoRedirect"><Values backward="none" forward="none"></Values><Default goto="Show Logo"></Default></Redirect>
<Redirect><Values backward="none" forward="start"></Values><Default goto="LogoFMVRedirectScreen"></Default></Redirect>
  </Screen>
  <Screen name="LogoFMVRedirectScreen">
<Redirect><Values backward="none" forward="none"></Values><Default goto="Show Logo"></Default></Redirect>
  </Screen>
  <Screen type="FEMain" name="Top FE Screen">
<Screen name="FE Screen">
  <Screen name="Show Logo">
    <Image transition="0"><Values y="72" AutoLoad="true" src="Data\FE\Images\pulse_logo.mip"></Values></Image>
    <Text delay="1" transition="0"><Values align="right" idstring="BOOT_PRESS_START" font="Menu" pulse="true" x="460" y="220" color="0x7FFFFFFF"></Values></Text>
    <Redirect><Values backward="none"></Values><Default goto="RemoveMemoryStickWarning"></Default></Redirect>
    <Redirect><Values backward="none" forward="start"></Values><Default goto="RemoveMemoryStickWarning"></Default></Redirect>
  </Screen>
</Screen>
  </Screen>
</Screen>
"#;

fn languages() -> Vec<Language> {
    [
        ("PI008", "French", "Français"),
        ("PI009", "German", "Deutsch"),
        ("PI012", "English", "English"),
    ]
    .into_iter()
    .map(|(plugin, name, native)| Language {
        plugin: plugin.into(),
        name: name.into(),
        native_name: native.into(),
        entries: None,
        // The picker draws in the default face whatever the plugin's slots
        // say, so these fixtures need none.
        fonts: Vec::new(),
    })
    .collect()
}

/// Pure's shape, cut down to the screens the boot chain actually walks.
///
/// Trimmed from `pure-psp-usa`'s own `Skin.xml` rather than invented: the
/// picker really is a child of `Intro Screen`, that parent really is where
/// the white background lives, and `FMV Intro` really does carry nothing but
/// a redirect. See [pure-boot.md](../../../../docs/architecture/pure-boot.md).
///
/// The two screens between the picker and `FMV Intro` on the real disc -
/// `Developer Publisher Screen` and `MemoryStickWarning` - are left out
/// here on purpose: this fixture exists to exercise the movie leg, and the
/// chain those two sit in is covered against the disc itself in
/// `pure_boot_ground_truth.rs`.
const PURE_XML: &str = r#"
<Screen>
  <Screen type="Intro" name="Intro Screen">
<Image><Values width="480" height="272" TxtrWidth="480" TxtrHeight="272" color="0xFFFFFFFF"></Values></Image>
<Movie name="FMV Movie"><Values src="Data\Movies\WoFMVNew" sound="true" autoredirect="1" localised="true"></Values></Movie>
<Screen type="Language Selection" name="Language Selection">
  <DisplayLanguages><Values clear="true"></Values></DisplayLanguages>
  <Menu name="Language"><Values align="left" font="Default" x="50" scale="1.0" y="46" color="0xFF88D6E8"></Values></Menu>
  <Redirect name="LanguageAutoRedirect">
    <Values backward="none"></Values>
    <Default goto="Developer Publisher Screen"></Default>
  </Redirect>
</Screen>
<Screen type="FMV" name="FMV Intro">
  <Item></Item>
  <Redirect name="FMVRedirect"><Default goto="Title Screen"></Default></Redirect>
</Screen>
<Screen type="Title" name="Title Screen">
  <Text><Values align="right" idstring="PRESS START" font="Title" x="408" y="194" color="0xFFED4896"></Values></Text>
</Screen>
  </Screen>
</Screen>
"#;

/// The grid every fixture in here authors in.
const GRID: (u32, u32) = (SCREEN.0 as u32, SCREEN.1 as u32);

/// A Pure-shaped boot whose second movie has `fmv_frames` frames of picture.
///
/// The sequence is what `oag_pure::frontend::BOOT_PROFILE` resolves to
/// against this fixture's screens, written out rather than resolved so that
/// this file's tests stay independent of `boot::load`.
fn pure(fmv_frames: usize) -> Frontend {
    Frontend::booting(
        Sequence {
            steps: vec![
                Step {
                    state: pure_states::LANGUAGE_SELECTION,
                    movie: MoviePlan::none(GRID),
                },
                Step {
                    state: pure_states::DEVELOPER_PUBLISHER,
                    movie: MoviePlan::none(GRID),
                },
                Step {
                    state: pure_states::MEMORY_STICK_WARNING,
                    movie: MoviePlan::none(GRID),
                },
                Step {
                    state: pure_states::FMV_INTRO,
                    movie: MoviePlan {
                        frames: fmv_frames,
                        frame_rate: crate::movie::FRAME_RATE,
                        aspect: GRID,
                        has_picture: fmv_frames > 0,
                    },
                },
                Step {
                    state: pure_states::TITLE_SCREEN,
                    movie: MoviePlan::none(GRID),
                },
            ],
            backdrop_parent: Some(pure_states::INTRO_SCREEN),
        },
        Screens::from_xml(PURE_XML),
        StringTable::default(),
        languages(),
        Vec::new(),
    )
}

/// Walks a Pure boot from the picker to `FMV Intro`, the way the disc does.
///
/// Confirm a language, sit out the developer/publisher cards, acknowledge the
/// storage warning. Three steps, because the disc has three - which is the
/// whole point of the chain being a chain.
fn reach_the_second_movie(frontend: &mut Frontend, input: &mut Input) {
    input.begin_frame(1 << button::CROSS);
    frontend.update(FRAME, input, None);
    run_until(frontend, input, 1200, |f| {
        f.machine().is(pure_states::MEMORY_STICK_WARNING)
    });
    input.begin_frame(0);
    frontend.update(FRAME, input, None);
    input.begin_frame(1 << button::CROSS);
    frontend.update(FRAME, input, None);
}

/// The default leg: `LogoFMV`, playing the movie the disc plays.
fn frontend(frames: usize) -> Frontend {
    Frontend::new(
        Screens::from_xml(XML),
        StringTable::default(),
        languages(),
        Vec::new(),
        frames,
        false,
    )
}

/// The `--reel` leg: `Intro Screen->IntroMovie1`, with the frame holds.
fn reel(frames: usize) -> Frontend {
    Frontend::booting(
        Sequence {
            steps: vec![
                Step {
                    state: states::INTRO_MOVIE,
                    movie: MoviePlan {
                        frames,
                        frame_rate: crate::movie::FRAME_RATE,
                        aspect: GRID,
                        has_picture: false,
                    },
                },
                Step {
                    state: states::LANGUAGE_SELECTION,
                    movie: MoviePlan::none(GRID),
                },
                Step {
                    state: states::SHOW_LOGO,
                    movie: MoviePlan::none(GRID),
                },
            ],
            backdrop_parent: None,
        },
        Screens::from_xml(XML),
        StringTable::default(),
        languages(),
        Vec::new(),
    )
}

/// Runs until `predicate` holds or the step budget runs out.
fn run_until(
    frontend: &mut Frontend,
    input: &mut Input,
    steps: usize,
    predicate: impl Fn(&Frontend) -> bool,
) -> bool {
    for _ in 0..steps {
        if predicate(frontend) {
            return true;
        }
        input.begin_frame(0);
        frontend.update(FRAME, input, None);
    }
    predicate(frontend)
}

/// Walks from the picker to `Show Logo`, leaving the machine there.
fn pick_a_language(frontend: &mut Frontend, input: &mut Input) {
    input.begin_frame(1 << button::START);
    frontend.update(FRAME, input, None);
    input.begin_frame(0);
    frontend.update(FRAME, input, None);

    input.begin_frame(1 << button::DOWN);
    frontend.update(FRAME, input, None);
    input.begin_frame(0);
    input.begin_frame(1 << button::CROSS);
    frontend.update(FRAME, input, None);
}
