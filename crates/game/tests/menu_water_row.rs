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
fn each_lender_disc_adds_its_own_value_and_never_on_itself() {
    let (hd, vita, omega) = (
        oag_hd::TITLE.slug(),
        oag_2048::TITLE.slug(),
        oag_omega::TITLE.slug(),
    );
    let all = [hd.as_str(), vita.as_str(), omega.as_str()];
    for (lender, value) in [(&hd, "hd"), (&vita, "2048"), (&omega, "omega")] {
        let got = row_after(oag_pulse::TITLE, &[lender.as_str()]);
        assert_eq!(got, ["off", "original", value], "Pulse with only {value}");
    }
    assert_eq!(
        row_after(oag_pure::TITLE, &all),
        ["off", "original", "hd", "2048", "omega"]
    );
    assert_eq!(
        row_after(oag_hd::TITLE, &all),
        ["off", "original", "2048", "omega"],
        "HD is never offered its own law"
    );
    assert_eq!(
        row_after(oag_2048::TITLE, &all),
        ["off", "original", "hd", "omega"]
    );
    assert_eq!(
        row_after(oag_omega::TITLE, &all),
        ["off", "original", "hd", "2048"]
    );
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
        Water::Hd
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
        ["off", "original", "hd", "2048", "omega"]
    );
}
