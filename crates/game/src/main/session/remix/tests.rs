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

/// Omega carries 2048's five teams as its own re-textured craft, so with no
/// 2048 disc "Wipeout 2048" is still a CRAFT TITLE, backed by Omega; a real
/// 2048 source always wins and the row is never offered twice.
#[test]
fn omegas_2048_era_craft_are_reachable_without_a_2048_disc_and_never_twice() {
    let only_omega = [candidate(oag_omega::TITLE)];
    let names: Vec<String> = craft_title_choices(&only_omega)
        .iter()
        .map(|choice| choice.value.clone())
        .collect();
    assert!(
        names.iter().any(|name| name == oag_2048::TITLE.name),
        "{names:?}"
    );
    let backing = resolve_craft_backing(&only_omega, oag_2048::TITLE.name)
        .expect("Omega carries the 2048-era craft");
    assert_eq!(backing.title(), oag_omega::TITLE.name);

    let both = [candidate(oag_omega::TITLE), candidate(oag_2048::TITLE)];
    let names: Vec<String> = craft_title_choices(&both)
        .iter()
        .map(|choice| choice.value.clone())
        .collect();
    assert_eq!(
        names
            .iter()
            .filter(|name| *name == oag_2048::TITLE.name)
            .count(),
        1,
        "{names:?}"
    );
    assert_eq!(
        resolve_craft_backing(&both, oag_2048::TITLE.name)
            .unwrap()
            .title(),
        oag_2048::TITLE.name
    );
}

fn craft_names(titles: &[oag_game::launcher::Candidate]) -> Vec<String> {
    craft_title_choices(titles)
        .into_iter()
        .map(|choice| choice.value)
        .collect()
}

/// The three mount combinations of the maintainer's rule (2026-10-06): with
/// only Omega the plain "Wipeout 2048" entry, with only the Vita disc the
/// same entry unchanged, and with both a second entry naming Omega's.
#[test]
fn omegas_2048_entry_is_told_apart_only_when_the_vita_disc_is_mounted_too() {
    let omega_label = oag_omega::race::ERA_2048_ROSTER.alongside_label.unwrap();
    let only_omega = craft_names(&[candidate(oag_omega::TITLE)]);
    assert!(only_omega.iter().any(|n| n == oag_2048::TITLE.name));
    assert!(
        !only_omega.iter().any(|n| n == omega_label),
        "{only_omega:?}"
    );

    let only_vita = craft_names(&[candidate(oag_2048::TITLE)]);
    assert!(!only_vita.iter().any(|n| n == omega_label), "{only_vita:?}");

    let both = [candidate(oag_omega::TITLE), candidate(oag_2048::TITLE)];
    let names = craft_names(&both);
    let count = |wanted: &str| names.iter().filter(|n| *n == wanted).count();
    assert_eq!(count(oag_2048::TITLE.name), 1, "{names:?}");
    assert_eq!(count(omega_label), 1, "{names:?}");
}

/// Each of the two entries loads its own source and never the other's.
#[test]
fn with_both_mounted_each_2048_entry_backs_onto_its_own_source() {
    let omega_label = oag_omega::race::ERA_2048_ROSTER.alongside_label.unwrap();
    let both = [candidate(oag_omega::TITLE), candidate(oag_2048::TITLE)];
    assert_eq!(
        resolve_craft_backing(&both, oag_2048::TITLE.name)
            .unwrap()
            .title(),
        oag_2048::TITLE.name
    );
    assert_eq!(
        resolve_craft_backing(&both, omega_label).unwrap().title(),
        oag_omega::TITLE.name
    );
    let only_vita = [candidate(oag_2048::TITLE)];
    assert!(resolve_craft_backing(&only_vita, omega_label).is_none());
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

/// RACE REMIX's speed-class row, over the *real* title packages rather than a
/// shaped fixture - the composition `Session::remix_speed_classes` performs,
/// with the survey list supplied directly so no disc has to be present.
fn union_over(titles: &[&'static oag_title::Title]) -> Vec<&'static str> {
    let ladders = titles.iter().filter_map(|title| title.race.speed_classes);
    oag_title::SpeedClasses::union(ladders)
}

/// The offered row: the union, less the rungs this build cannot race.
fn offered_over(titles: &[&'static oag_title::Title]) -> Vec<&'static str> {
    union_over(titles)
        .into_iter()
        .filter(|name| oag_title::SpeedClasses::is_selectable(name))
        .collect()
}

/// **Arm one: a machine with no Pure image.** The union is Pulse's own four
/// rungs and there is no `VECTOR` anywhere in it - not filtered out, simply
/// never contributed, because the title that authors it is not here.
#[test]
fn without_pure_the_union_is_the_four_pulse_rungs() {
    let union = union_over(&[oag_pulse::TITLE]);

    assert_eq!(union, vec!["VENOM", "FLASH", "RAPIER", "PHANTOM"]);
    assert!(!union.contains(&oag_title::SpeedClasses::VECTOR));
}

/// **Arm two: the same machine with a Pure image on it.** The union grows by
/// exactly the rung Pure authors, at the front, where Pure's own file puts
/// it. This is the pair that proves the row is built from what is mounted
/// rather than from a fixed list with a filter bolted on.
#[test]
fn with_pure_the_union_gains_vector_at_the_front() {
    let without = union_over(&[oag_pulse::TITLE]);
    let with = union_over(&[oag_pulse::TITLE, oag_pure::TITLE]);

    assert_eq!(with.len(), without.len() + 1);
    assert_eq!(with[0], oag_title::SpeedClasses::VECTOR);
    assert_eq!(&with[1..], &without[..]);
}

/// The order the survey happens to list the titles in does not change the
/// row, so a machine that finds Pure first shows the same ladder as one that
/// finds Pulse first.
#[test]
fn the_union_does_not_depend_on_the_survey_order() {
    assert_eq!(
        union_over(&[oag_pulse::TITLE, oag_pure::TITLE]),
        union_over(&[oag_pure::TITLE, oag_pulse::TITLE])
    );
}

/// A title whose ladder is unread contributes nothing rather than lending the
/// union another title's measurement - Wipeout 2048's `speed_classes: None`.
#[test]
fn an_unread_ladder_adds_nothing_to_the_union() {
    assert_eq!(
        union_over(&[oag_pulse::TITLE, oag_2048::TITLE]),
        union_over(&[oag_pulse::TITLE])
    );
    assert!(union_over(&[oag_2048::TITLE]).is_empty());
}

/// **The third arm, and the one that bites**: a machine whose only playable
/// source is Wipeout 2048 unions to *nothing*, because that title's ladder is
/// unread.
///
/// An empty list must not be settled against. `settle` clears a stored value
/// that the offered list does not contain, and against an empty list it
/// clears unconditionally - see `settle_clears_against_an_empty_list` above -
/// so settling here would wipe the player's saved `race.class` to `""` on a
/// legitimate configuration. `Session::open_menus` guards the settle on the
/// union being non-empty for exactly this case; this pins the premise that
/// makes the guard necessary, since the two facts live in different files and
/// neither one alone looks dangerous.
#[test]
fn a_2048_only_machine_unions_to_nothing_which_must_not_be_settled_against() {
    let union = union_over(&[oag_2048::TITLE]);
    assert!(union.is_empty());

    // What the guard prevents, spelled out: settling a saved class against
    // this list would lose it.
    let mut saved = "phantom".to_string();
    let offered: Vec<menu::Choice> = union.iter().map(|n| menu::Choice::plain(*n)).collect();
    settle(&mut saved, &offered);
    assert_eq!(saved, "", "this is why open_menus does not settle on empty");
}

/// What the player actually sees on each arm, and the whole point of the
/// change that made `VECTOR` selectable: **four without Pure, five with it.**
///
/// This is "offer it only if Pure's data is available" in its entirety, and no
/// flag implements it. The union is built from the ladders of the titles this
/// machine can actually open; `VECTOR` is on exactly one measured ladder, so a
/// machine with no Pure image never sees the name at all.
///
/// The Pulse-only arm is the one worth reading twice. Pulse *does* author a
/// `<GlobalClass name="VECTOR">` in its engine-wide handling file - all four
/// measured discs do - and it still offers four here, because a title's ladder
/// is measured from its **per-team** `handlingstats.xml`, which is the file
/// that decides what a rung does. Pulse's author four `<Class>` blocks and no
/// `VECTOR`, so there is no fifth rung to offer and nothing borrows one.
#[test]
fn vector_is_offered_with_pure_present_and_not_without_it() {
    let without = offered_over(&[oag_pulse::TITLE]);
    let with = offered_over(&[oag_pulse::TITLE, oag_pure::TITLE]);

    assert_eq!(
        without,
        vec!["VENOM", "FLASH", "RAPIER", "PHANTOM"],
        "a Pulse-only machine must see no fifth rung"
    );
    assert_eq!(
        with,
        vec!["VECTOR", "VENOM", "FLASH", "RAPIER", "PHANTOM"],
        "with Pure mounted the union gains its rung, slowest first"
    );

    // Every name offered is one some mounted ladder authors - the row never
    // grows a rung out of a table's width.
    let mounted: Vec<&str> = oag_pulse::TITLE
        .race
        .speed_classes
        .into_iter()
        .chain(oag_pure::TITLE.race.speed_classes)
        .flat_map(|ladder| ladder.names().iter().copied())
        .collect();
    for name in &with {
        assert!(
            mounted.contains(name),
            "{name:?} is offered but no mounted title authors it"
        );
    }
}

/// `oag_physics::SpeedClass` **still names four**, and that is the load-bearing
/// half of how the fifth rung was added.
///
/// The alternative - a fifth variant - would have widened the fixed tables its
/// discriminants index, and every title that authors four would have grown a
/// fabricated fifth entry. Instead a race carries the rung's *name* and
/// resolves it against the file that authored it. This test fails the moment
/// somebody widens the enum, which is the change this design exists to avoid.
#[test]
fn the_physics_enum_still_names_exactly_the_four_every_title_shares() {
    assert_eq!(oag_physics::SpeedClass::ALL.len(), 4);
    assert!(
        oag_physics::SpeedClass::from_name("VECTOR").is_none(),
        "the fifth rung is carried as a name, never as a variant"
    );
    for name in oag_title::SpeedClasses::PULSE_LADDER {
        assert!(oag_physics::SpeedClass::from_name(name).is_some());
    }
}

/// The union is drawn from *playable* survey rows only: a source that will
/// not open contributes no classes, the same way it contributes no circuits.
#[test]
fn an_unavailable_source_contributes_no_classes() {
    let rows = [
        candidate(oag_pulse::TITLE),
        oag_game::launcher::Candidate {
            source: "fixture:broken".to_string(),
            name: "broken".to_string(),
            platform: oag_disc::Platform::Unknown,
            serial: None,
            state: oag_game::launcher::State::Unavailable("unreadable".to_string()),
        },
    ];

    let ladders = rows.iter().filter_map(|row| match &row.state {
        oag_game::launcher::State::Playable(title) => title.race.speed_classes,
        oag_game::launcher::State::Unavailable(_) => None,
    });

    assert_eq!(
        oag_title::SpeedClasses::union(ladders),
        union_over(&[oag_pulse::TITLE])
    );
}

/// Both 2048 entries, over the real sources: each opens its own and the craft
/// model it reads is that source's (the two differ in size for every craft).
#[test]
#[ignore = "needs the Vita 2048 and Omega extractions"]
fn each_2048_entry_reads_its_own_sources_craft() {
    let (Some(vita), Some(omega)) = (
        oag_testdata::exact("data/extracted/vita/PCSF00007"),
        oag_testdata::exact("data/extracted/ps4"),
    ) else {
        return;
    };
    let titles = oag_game::launcher::survey(&[omega, vita]);
    let names = craft_names(&titles);
    let omega_label = oag_omega::race::ERA_2048_ROSTER.alongside_label.unwrap();
    for wanted in [oag_2048::TITLE.name, omega_label] {
        assert!(names.iter().any(|n| n == wanted), "{wanted}: {names:?}");
    }
    let model = |entry: &str| {
        let backing = resolve_craft_backing(&titles, entry).expect("a source for the entry");
        let title = backing.playable().unwrap();
        let mut archives = oag_assets::Archives::open(&backing.source, title).expect("opens");
        let id = r"Feisar2048\3";
        let dir = title.race.ships_for(id).dir;
        archives
            .read_name(&format!(r"{dir}\{id}\ship.rcsmodel"))
            .expect("the craft model")
            .len()
    };
    assert_eq!(model(oag_2048::TITLE.name), 476_012, "the Vita disc's own");
    assert_eq!(
        model(omega_label),
        489_720,
        "Omega's own, not the Vita disc's"
    );
}
