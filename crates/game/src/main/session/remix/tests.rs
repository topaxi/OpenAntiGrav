use super::*;

/// A survey row for `title`, with no real archive behind it - every field of
/// `oag_game::launcher::Candidate` is `pub`, so this needs nothing a real
/// disc would have to provide.
fn candidate(title: &'static oag_title::Title) -> oag_game::launcher::Candidate {
    oag_game::launcher::Candidate {
        source: format!("fixture:{}", title.name),
        name: title.name.to_string(),
        platform: oag_disc::Platform::Unknown,
        serial: None,
        state: oag_game::launcher::State::Playable(title),
    }
}

/// A real HD source, when present, always backs "Wipeout HD" - and is never
/// shadowed by 2048's guest roster even when 2048 is on the same machine.
#[test]
fn a_real_hd_source_wins_over_2048s_guest_roster() {
    let titles = [candidate(oag_hd::TITLE), candidate(oag_2048::TITLE)];
    let backing = resolve_craft_backing(&titles, oag_hd::TITLE.name)
        .expect("a real HD source is on the search path");
    assert_eq!(backing.title(), oag_hd::TITLE.name);
    assert_eq!(backing.source, format!("fixture:{}", oag_hd::TITLE.name));
}

/// With no real HD source, "Wipeout HD" falls back to 2048's own package -
/// the whole point of the guest roster.
#[test]
fn with_no_real_hd_source_wipeout_hd_falls_back_to_2048() {
    let titles = [candidate(oag_2048::TITLE)];
    let backing = resolve_craft_backing(&titles, oag_hd::TITLE.name)
        .expect("2048 carries HD's own roster too");
    assert_eq!(
        backing.title(),
        oag_2048::TITLE.name,
        "the real candidate backing the pick is 2048's, not a synthetic HD one"
    );
}

/// With neither on the search path, "Wipeout HD" resolves to nothing - the
/// fallback only ever reaches for 2048, never invents a source.
#[test]
fn with_neither_present_wipeout_hd_resolves_to_nothing() {
    let titles = [candidate(oag_pulse::TITLE)];
    assert!(resolve_craft_backing(&titles, oag_hd::TITLE.name).is_none());
}

/// A title that is not "Wipeout HD" never falls back - the guest-roster rule
/// is named-title-specific, not a general "try something else" fallback.
#[test]
fn a_different_title_never_falls_back_even_with_2048_present() {
    let titles = [candidate(oag_2048::TITLE)];
    assert!(
        resolve_craft_backing(&titles, oag_pure::TITLE.name).is_none(),
        "Wipeout Pure has no guest roster and no fallback to reach for"
    );
}

/// CRAFT TITLE's own list: every real title, and "Wipeout HD" is added only
/// when 2048 is present and HD is not.
#[test]
fn craft_title_choices_adds_wipeout_hd_only_when_2048_is_present_and_hd_is_not() {
    let real_hd = [candidate(oag_hd::TITLE), candidate(oag_2048::TITLE)];
    let names: Vec<String> = craft_title_choices(&real_hd)
        .iter()
        .map(|choice| choice.value.clone())
        .collect();
    assert_eq!(
        names
            .iter()
            .filter(|name| *name == oag_hd::TITLE.name)
            .count(),
        1,
        "a real HD source already offers the row; no second, synthetic one: {names:?}"
    );

    let only_2048 = [candidate(oag_2048::TITLE)];
    let names: Vec<String> = craft_title_choices(&only_2048)
        .iter()
        .map(|choice| choice.value.clone())
        .collect();
    assert!(
        names.iter().any(|name| name == oag_hd::TITLE.name),
        "2048 alone should still offer Wipeout HD as a synthetic choice: {names:?}"
    );

    let only_pulse = [candidate(oag_pulse::TITLE)];
    let names: Vec<String> = craft_title_choices(&only_pulse)
        .iter()
        .map(|choice| choice.value.clone())
        .collect();
    assert!(
        !names.iter().any(|name| name == oag_hd::TITLE.name),
        "no 2048 on this machine, so nothing to fall back to: {names:?}"
    );
}

/// `settle` keeps a stored value the list still offers - a saved pick
/// survives a menu reopen, which is the whole reason the section is
/// persisted.
#[test]
fn settle_keeps_a_value_the_list_still_offers() {
    let offered = [
        menu::Choice::plain("Wipeout Pulse"),
        menu::Choice::plain("Wipeout Pure"),
    ];
    let mut stored = "Wipeout Pure".to_string();
    settle(&mut stored, &offered);
    assert_eq!(stored, "Wipeout Pure");
}

/// **The fresh-settings-file case.** `settings::Remix` defaults every field
/// to empty, `Menu::supply` draws index 0 for it, and the setting has to
/// follow or START races nothing at all.
#[test]
fn settle_takes_the_first_offer_for_an_empty_setting() {
    let offered = [
        menu::Choice::plain("Wipeout Pulse"),
        menu::Choice::plain("Wipeout Pure"),
    ];
    let mut stored = String::new();
    settle(&mut stored, &offered);
    assert_eq!(
        stored, "Wipeout Pulse",
        "index 0 is what the row shows, so it is what the setting holds"
    );
}

/// A saved pick whose title has since left the search path falls to the
/// first one that has not - `Menu::supply`'s own rule for a row, applied to
/// the setting behind it.
#[test]
fn settle_replaces_a_value_the_list_no_longer_offers() {
    let offered = [menu::Choice::plain("Wipeout Pulse")];
    let mut stored = "Wipeout 2048".to_string();
    settle(&mut stored, &offered);
    assert_eq!(stored, "Wipeout Pulse");
}

/// An empty list is "this team has no second directory at all", not "pick
/// the first of none" - the row draws unusable and the setting holds
/// nothing, which is what `combine_variant` reads as "no variant applies".
#[test]
fn settle_clears_against_an_empty_list() {
    let mut stored = "_c1".to_string();
    settle(&mut stored, &[]);
    assert_eq!(stored, "");
}
