//! What each title's behaviour data answers, pinned against the five titles
//! the way `oag-game` used to ask by name (ADR-0058). No disc needed: these are
//! the `Title` consts themselves. A title that stops answering as it did turns
//! a test red here rather than moving a screen silently.

use oag_title::{CampaignDialect, Title};

const ALL: [&Title; 5] = [
    oag_pulse::TITLE,
    oag_pure::TITLE,
    oag_hd::TITLE,
    oag_2048::TITLE,
    oag_omega::TITLE,
];

#[test]
fn campaign_dialect_is_pulse_for_pulse_pure_2048_hd_for_hd_omega_for_omega() {
    let dialects: Vec<(&str, CampaignDialect)> = ALL
        .iter()
        .map(|title| (title.name, title.campaign.dialect))
        .collect();
    assert_eq!(
        dialects,
        [
            ("Wipeout Pulse", CampaignDialect::Pulse),
            ("Wipeout Pure", CampaignDialect::Pulse),
            ("Wipeout HD", CampaignDialect::Hd),
            ("Wipeout 2048", CampaignDialect::Pulse),
            ("Wipeout: Omega Collection", CampaignDialect::Omega),
        ]
    );
}

/// `--campaign-cell` reads Pulse's and HD's grids and refuses everyone else.
#[test]
fn only_pulse_and_hd_name_a_grids_file() {
    let named: Vec<&str> = ALL
        .iter()
        .filter(|title| title.campaign.definition_entry.is_some())
        .map(|title| title.name)
        .collect();
    assert_eq!(named, ["Wipeout Pulse", "Wipeout HD"]);
    assert_eq!(
        oag_pulse::TITLE.campaign.definition_entry,
        Some(oag_pulse::campaign::DEFINITION_ENTRY)
    );
    assert_eq!(
        oag_hd::TITLE.campaign.definition_entry,
        Some(oag_hd::campaign::DEFINITION_ENTRY)
    );
}

/// Circuit and loyalty gates are Pulse's alone; `Campaign Selection`'s ids are
/// overlaid on HD alone (not Omega).
#[test]
fn unlock_gates_are_pulses_and_the_selection_overlay_is_hds() {
    for title in ALL {
        let pulse = title.name == "Wipeout Pulse";
        assert_eq!(title.campaign.circuit_unlocks, pulse, "{}", title.name);
        assert_eq!(title.campaign.loyalty_unlocks, pulse, "{}", title.name);
        assert_eq!(
            title.campaign.selection_strings,
            title.name == "Wipeout HD",
            "{}",
            title.name
        );
        assert_eq!(oag_game::unlock::gates_variants(title), pulse);
    }
}

/// HD alone opens a fresh profile on a model of its own.
#[test]
fn only_hd_has_a_fresh_profile_variant() {
    for title in ALL {
        assert_eq!(
            title.race.fresh_variant.map(|fresh| fresh.variant),
            (title.name == "Wipeout HD").then_some(oag_hd::race::FRESH_PROFILE_VARIANT),
            "{}",
            title.name
        );
    }
}

/// 2048 reships HD's roster and names HD by its own name, which a title package
/// cannot read from `oag-hd`. Omega carries 2048's five teams the other way
/// round, as its own re-textured 2048-era craft.
#[test]
fn only_2048_and_omega_reship_a_roster() {
    for title in ALL {
        let expected = if title.name == oag_2048::TITLE.name {
            Some(oag_hd::TITLE.name)
        } else if title.name == oag_omega::TITLE.name {
            Some(oag_2048::TITLE.name)
        } else {
            None
        };
        assert_eq!(
            title.race.guest_roster.map(|guest| guest.reships),
            expected,
            "{}",
            title.name
        );
    }
}

/// Only Pure resolves anything per pressing.
#[test]
fn only_pure_carries_pressings() {
    for title in ALL {
        assert_eq!(
            title.pressings.is_some(),
            title.name == oag_pure::TITLE.name
        );
    }
}

/// Only the Fury disc's HD reads its screen file from `DATA06`; the PSN
/// download has no such archive and reads the copy its mounts serve.
#[test]
fn only_the_hd_disc_names_a_screen_archive() {
    for title in ALL {
        assert_eq!(
            title.campaign.screen_archive,
            (title.name == "Wipeout HD").then_some(oag_hd::campaign::SELECTION_SCREEN_ARCHIVE),
            "{}",
            title.name
        );
    }
    assert_eq!(oag_hd::psn::PSN.campaign.screen_archive, None);
}
