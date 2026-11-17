//! The WATER row offers a borrowed law only while the lender's disc is present, and never on
//! the lender itself. The definition is built in, so no disc and no mod is read.

use oag_display::display::Water;
use oag_ui::menu::{Definition, Entry};

fn built_in() -> Definition {
    Definition::built_in(&oag_ui::language::StringTable::default())
        .expect("the built-in menu must parse")
}

fn offered(definition: &Definition) -> Vec<String> {
    let mut found = Vec::new();
    for entry in definition.pages.iter().flat_map(|page| page.entries.iter()) {
        if let Entry::Choice {
            setting, values, ..
        } = entry
            && setting == "graphics.water"
        {
            found.extend(values.iter().map(|choice| choice.value.clone()));
        }
    }
    found
}

fn row_after(title: &oag_title::Title, present: &[&str]) -> Vec<String> {
    let mut definition = built_in();
    oag_game::water_setting::trim_row(&mut definition, title, &|slug| present.contains(&slug));
    offered(&definition)
}

#[test]
fn the_shipped_row_lists_every_water_value_in_order() {
    let every: Vec<String> = Water::ALL.iter().map(|w| w.name().to_string()).collect();
    assert_eq!(offered(&built_in()), every);
}

#[test]
fn with_no_lender_disc_the_row_offers_off_and_original_only() {
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_omega::TITLE,
        oag_2048::TITLE,
    ] {
        assert_eq!(row_after(title, &[]), ["off", "original"], "{}", title.name);
    }
}

#[test]
fn each_lender_is_available_only_with_its_disc_and_never_on_itself() {
    let (hd, vita, omega) = (
        oag_hd::TITLE.slug(),
        oag_2048::TITLE.slug(),
        oag_omega::TITLE.slug(),
    );
    let all = [hd.as_str(), vita.as_str(), omega.as_str()];
    let avail = |value, title: &oag_title::Title, present: &[&str]| {
        oag_game::water_setting::lender_available(value, title, &|slug| present.contains(&slug))
    };
    for (lender, value) in [
        (&hd, Water::Hd),
        (&vita, Water::Vita2048),
        (&omega, Water::Omega),
    ] {
        assert!(
            avail(value, oag_pulse::TITLE, &[lender.as_str()]),
            "{value}"
        );
        assert!(!avail(value, oag_pulse::TITLE, &[]), "{value} with no disc");
    }
    assert!(
        !avail(Water::Hd, oag_hd::TITLE, &all),
        "HD never borrows its own law"
    );
    assert!(!avail(Water::Vita2048, oag_2048::TITLE, &all));
    assert!(!avail(Water::Omega, oag_omega::TITLE, &all));
    assert!(avail(Water::Hd, oag_omega::TITLE, &all));
}

/// Each choice is hidden until its law is drawn: with every lender disc present the row still
/// holds only the laws `water_law_is_drawn` admits. A lane that lands a law flips that flag and
/// this list grows with it.
#[test]
fn with_every_lender_present_only_drawn_laws_are_offered() {
    let drawn: Vec<&str> = Water::ALL
        .iter()
        .filter(|w| oag_mesh::mesh::rcs::water_law_is_drawn(**w))
        .map(|w| w.name())
        .collect();
    assert_eq!(drawn, ["off", "original"], "no borrowed law is drawn yet");
    for title in [
        oag_pulse::TITLE,
        oag_pure::TITLE,
        oag_hd::TITLE,
        oag_omega::TITLE,
        oag_2048::TITLE,
    ] {
        assert_eq!(
            row_after(
                title,
                &[
                    &oag_hd::TITLE.slug(),
                    &oag_2048::TITLE.slug(),
                    &oag_omega::TITLE.slug()
                ]
            ),
            drawn,
            "{}",
            title.name
        );
    }
}

#[test]
fn a_saved_value_that_is_not_offered_draws_original() {
    let none = |_: &str| false;
    for value in [Water::Hd, Water::Vita2048, Water::Omega] {
        assert_eq!(
            oag_game::water_setting::resolve(value, oag_pulse::TITLE, &none),
            Water::Original,
            "{value} with no disc"
        );
    }
    let all = |_: &str| true;
    assert_eq!(
        oag_game::water_setting::resolve(Water::Hd, oag_hd::TITLE, &all),
        Water::Original,
        "HD's own law on HD"
    );
    assert_eq!(
        oag_game::water_setting::resolve(Water::Hd, oag_pulse::TITLE, &all),
        Water::Original,
        "an undrawn law falls back"
    );
    for value in [Water::Off, Water::Original] {
        assert_eq!(
            oag_game::water_setting::resolve(value, oag_pulse::TITLE, &none),
            value
        );
    }
}

/// The boot and every live rebuild trim through `finish_definition`: dropping its call to the
/// water trim leaves a lender value on a machine with no lender disc.
#[test]
fn the_boot_trim_applies_the_water_row_trim() {
    let strings = oag_ui::language::StringTable::default();
    let mut definition = built_in();
    oag_game::menu_tree::finish_definition(&mut definition, oag_pulse::TITLE, &strings, &|_| false);
    assert_eq!(offered(&definition), ["off", "original"]);
    let mut definition = built_in();
    oag_game::menu_tree::finish_definition(&mut definition, oag_pulse::TITLE, &strings, &|_| true);
    assert_eq!(
        offered(&definition),
        ["off", "original"],
        "undrawn laws stay hidden"
    );
}
