use super::*;

/// A menu with just the four rows this file cares about, so a test can
/// exercise the free functions above without a real window, GPU or disc.
fn fixture_menu() -> menu::Menu {
    let text = "\
version = 1
root = \"pilots\"
[[page]]
id = \"pilots\"
[[page.entry]]
kind = \"choice\"
label = \"PILOT\"
setting = \"pilot.selected\"
values_from = \"pilots\"
[[page.entry]]
kind = \"choice\"
label = \"AXIS\"
setting = \"pilot.axis\"
values_from = \"pilot_axes\"
[[page.entry]]
kind = \"choice\"
label = \"LOW\"
setting = \"pilot.low\"
values_from = \"pilot_axis_low\"
[[page.entry]]
kind = \"choice\"
label = \"HIGH\"
setting = \"pilot.high\"
values_from = \"pilot_axis_high\"
";
    let strings = oag_ui::language::StringTable::default();
    let definition = menu::Definition::parse(text, &strings).expect("a tiny valid definition");
    menu::Menu::new(definition)
}

/// A directory holding one pilot file, cleaned up on drop - the same
/// idiom `pilots`' own tests use, kept local rather than shared so this
/// file does not reach into `oag_raceplay::pilots`' private test helpers.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn with(name: &str, body: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "oag-pilot-editor-{}-{:p}",
            std::process::id(),
            name.as_ptr()
        ));
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        std::fs::write(dir.join(format!("{name}.toml")), body).expect("a scratch pilot");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Lands PILOT on `name` and AXIS on `axis`, the way opening the real
/// page does through [`Session::supply_pilot_menu`] - reproduced here
/// with the two free functions directly, since there is no [`Session`]
/// in this test.
fn land_on(model: &mut menu::Menu, roster: &pilots::Roster, name: &str, axis: &str) {
    supply_pilot_choices(model, roster, Some(name), None);
    model.supply(menu::ValueSource::PilotAxes, &[menu::Choice::plain(axis)]);
    model.seed("pilot.axis", &Value::Text(axis.to_string()));
    resupply_bounds(model, roster);
}

/// The generated steps must never carry float noise into a saved file -
/// see [`quantize`]'s own doc for the `0.17250001` this exists to catch.
#[test]
fn quantized_steps_never_print_more_than_a_handful_of_decimals() {
    for choice in axis_choices(0.1, 3.0, 1.7) {
        assert!(
            choice.value.len() <= 7,
            "{:?} looks like float noise, not a step",
            choice.value
        );
    }
}

/// The exact current value is always offered, even when it is not one
/// of the round steps - a hand-authored `0.427` must still seed exactly.
#[test]
fn the_pilots_own_exact_value_is_always_one_of_the_choices() {
    let choices = axis_choices(0.1, 3.0, 0.427);
    assert!(choices.iter().any(|choice| choice.value == "0.427"));
}

/// SAVE writes AXIS's own axis alone. Pinned here: an edit to a
/// *different* axis, left untouched when AXIS moves on to another one,
/// is discarded without a word - see this file's own module doc.
#[test]
fn moving_axis_discards_an_untouched_low_edit_without_saving() {
    let scratch = Scratch::with("winston", "commitment = [0.90, 1.00]\n");
    let roster = pilots::load_from(&scratch.0).expect("a readable directory");
    let mut model = fixture_menu();
    land_on(&mut model, &roster, "winston", "commitment");
    assert_eq!(held_text(&model, "pilot.low"), Some("0.9".to_string()));

    // The player nudges LOW - "0.5" is `commitment`'s own floor, always
    // one of `axis_choices`' round steps, so it is on the list without
    // depending on how the steps are spread.
    model.seed("pilot.low", &Value::Text("0.5".to_string()));
    assert_eq!(
        held_text(&model, "pilot.low"),
        Some("0.5".to_string()),
        "the seed above did not land - this test is not exercising what it claims to"
    );

    // AXIS moves - to the same axis is enough, since `resupply_bounds`
    // does not know or care that it "moved"; it only re-derives from the
    // roster's own stored span, which is the whole of the behaviour
    // being pinned.
    land_on(&mut model, &roster, "winston", "commitment");
    assert_eq!(
        held_text(&model, "pilot.low"),
        Some("0.9".to_string()),
        "the untouched-file value should have won back over the discarded edit"
    );
}

/// A directory holding `winston` alongside the four built-ins, and no
/// `aggressive.toml` - which is what makes `aggressive` a name that is *not*
/// taken while still being a built-in, the case the two notes have to tell
/// apart.
fn rename_roster() -> (Scratch, pilots::Roster) {
    let scratch = Scratch::with("winston", "look = [0.9, 1.0]\n");
    let roster = pilots::load_from(&scratch.0).expect("a readable directory");
    (scratch, roster)
}

/// Every note below is asserted against `None` for the string table, so this
/// pins the **English fallback** in [`say`] as well as the rule - a language
/// with no file overlays nothing, and a blank note is worse than none.
///
/// The name a rename opened on says nothing at all: a player who has typed
/// nothing has done nothing wrong.
#[test]
fn the_name_the_keyboard_opened_on_is_not_flagged() {
    let (_scratch, roster) = rename_roster();
    assert_eq!(rename_note("winston", "winston", &roster, None), None);
    assert_eq!(rename_note("gerald", "winston", &roster, None), None);
}

/// `rename_pilot` refuses a destination that exists rather than overwriting
/// it, and finding that out after typing a name on a grid is what makes an
/// on-screen keyboard unbearable.
#[test]
fn a_name_already_held_by_a_file_is_flagged_while_it_is_being_typed() {
    let scratch = Scratch::with("gerald", "look = [0.9, 1.0]\n");
    std::fs::write(scratch.0.join("winston.toml"), "look = [0.9, 1.0]\n")
        .expect("a second scratch pilot");
    let roster = pilots::load_from(&scratch.0).expect("a readable directory");
    let note = rename_note("gerald", "winston", &roster, None).expect("a note");
    assert!(note.contains("gerald"), "{note}");
    assert!(note.to_lowercase().contains("already"), "{note}");
}

/// A built-in's name with no file behind it is a **legitimate** destination -
/// the rename succeeds - so the note says what will happen rather than
/// refusing. This is the trap the whole feature walks into, seen from the
/// rename side.
#[test]
fn a_built_in_name_warns_that_a_file_of_it_replaces_the_built_in() {
    let (_scratch, roster) = rename_roster();
    let note = rename_note("aggressive", "winston", &roster, None).expect("a note");
    assert!(note.contains("aggressive"), "{note}");
    assert!(note.to_uppercase().contains("REPLACES"), "{note}");
    // And it is genuinely allowed, which is why this is a note and not a
    // refusal - the two would be indistinguishable from the message alone.
    assert!(pilots::check_name("aggressive").is_ok());
}

/// Deleting every character is the easiest thing to do on this screen, and
/// an empty buffer accepted in silence would be a rename that did nothing.
#[test]
fn an_empty_name_says_so_rather_than_failing_only_at_accept() {
    let (_scratch, roster) = rename_roster();
    let note = rename_note("", "winston", &roster, None).expect("a note");
    assert!(note.to_uppercase().contains("NEEDS A NAME"), "{note}");
}

/// The substitution is `%s` and not `{}`, and it has to actually happen -
/// a note reading "%s IS BUILT IN" would be worse than none.
#[test]
fn a_table_entry_has_its_percent_s_replaced_by_the_name() {
    let mut table = oag_ui::language::StringTable::default();
    table.merge(std::collections::HashMap::from([(
        "OAG_PILOT_NAME_TAKEN".to_string(),
        "%s IS TAKEN, AND SO IS %s".to_string(),
    )]));
    assert_eq!(
        say_of(Some(&table), "OAG_PILOT_NAME_TAKEN", "unused", "winston"),
        "winston IS TAKEN, AND SO IS winston"
    );
    // With no table at all, the English beside the id is what shows.
    assert_eq!(
        say_of(None, "OAG_PILOT_NAME_TAKEN", "NO %s HERE", "winston"),
        "NO winston HERE"
    );
}
