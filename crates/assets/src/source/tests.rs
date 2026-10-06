//! What archive discovery and entry lookup in [`super`] are asserted to do.
//!
//! Split out of `source.rs` under the 200-line cap on inline `#[cfg(test)]`
//! modules; see `scripts/check-file-size.py`.

use super::*;
use oag_title::{ArchiveCandidates, ForeignSerial};

/// A stand-in for what a title crate supplies.
///
/// These are Pulse's real archive names, because the matching rules being
/// tested here - a serial-named parent directory, a case-insensitive tail,
/// a component boundary - were all derived from real pressings and a
/// synthetic name would test the rules against nothing. They are a
/// *fixture* rather than this crate's knowledge: `oag-assets` itself names
/// no archive, and swapping this table for Pure's would leave every
/// assertion below meaningful.
const TITLE: &Title = &Title {
    name: "Wipeout Pulse",
    archives: ArchiveCandidates {
        data: &[
            ("PSP_GAME/USRDIR/Data.wad", Platform::Psp),
            ("WADS2.WAD", Platform::Ps2),
        ],
        fe: &[
            ("PSP_GAME/USRDIR/FE.wad", Platform::Psp),
            ("WADSP.WAD", Platform::Ps2),
        ],
        // The rules under test here are about *finding* archives, and a
        // candidate that is always present adds nothing to them. The set-shaped
        // role has its own test below.
        extra: &[],
    },
    foreign_serials: &[ForeignSerial {
        serial: "UCUS-98612",
        title: "Wipeout Pure",
    }],
    // `None` rather than a filled-in fixture, and it costs these tests nothing:
    // `oag-assets` resolves archives and has no business in a menu layout or a
    // boot sequence, so neither half was ever read here. What it buys is that
    // the archive rules below are now exercised against a title shaped like the
    // one that has no front end, which is the case that motivated the field.
    front_end: None,
    exhaust: &oag_title::exhaust::Exhaust::Unread,
    // `Unread` on the same terms as the ribbon beside it: this crate resolves
    // archives and never draws an exhaust.
    flare: &oag_title::flare::Flare::Unread,
    // Present for the same reason and equally unread: resolving archives has
    // nothing to do with which circuit a race opens on.
    race: &oag_title::RaceDefaults {
        track: r"Data\Environments\00_Nowhere\track.vex",
        team: "Nobody",
        // Named after nothing on any disc, for the reason `sounds` below is.
        ship_dir: r"Data\Nowhere",
        handling_dir: r"Data\Nowhere",
        effect_dir: r"Data\Nowhere",
        effect_dir_by_circuit: &[],
        zone: oag_title::ZoneCircuit::Prefixed("zone_"),
        zone_craft: oag_title::ZoneCraft::ModelsInTeam {
            hull: "Zone",
            boost: "Zoneboost",
        },
        // Unread here too: resolving archives has nothing to do with which
        // boost model a race would load.
        boost: None,
        // Unread here too, and named after nothing on any disc so that a test
        // which started reading it would fail loudly rather than resolve.
        sounds: &oag_title::SoundBanks {
            hud: r"Data\Sound\nowhere.bnk",
            ship: r"Data\Sound\nowhere.bnk",
            ship_zone: r"Data\Sound\nowhere.bnk",
            weapons: r"Data\Sound\nowhere.bnk",
            speech: r"Data\Sound\nowhere.bnk",
            track_general: Some(r"Data\Sound\nowhere.bnk"),
            crossfade: None,
        },
        // Unread here too: nothing in this crate reads the Zone ladder.
        zone_announcer: None,
        countdown_voice: None,
        zone_class_announcer: None,
        // Unread on the same terms: resolving archives has nothing to do with
        // the colour grade a Zone race climbs.
        zone_palette: None,
        zone_stages: None,
        zone_transition: None,
        zone_stage_textures: None,
        zone_sky: None,
        // Unread here too: nothing in this crate resolves a team's variant.
        team_variants: None,
        guest_roster: None,
        hull_variants: None,
        // A fixture: this test needs a `Title` to exist, not a real ladder.
        fresh_variant: None,
        speed_classes: None,
    },
    // Unread here for the same reason as the circuit above: an in-race HUD is
    // read *through* archives, and which layout a mode wants is nothing this
    // crate decides.
    hud: &oag_title::HudLayouts {
        arcade: r"Data\XML\Nothing_HUD.xml",
        time_trial: r"Data\XML\Nothing_HUD.xml",
        speed_lap: r"Data\XML\Nothing_HUD.xml",
        zone: r"Data\XML\Nothing_HUD.xml",
        elimination: r"Data\XML\Nothing_HUD.xml",
    },
    // And unread on the same terms: which widgets a HUD keeps up, and out of
    // which texture, is decided long after an archive has been found.
    weapons: &oag_title::weapons::Weapons {
        race: r"Data\XML\WeaponStats_Race.xml",
        elimination: None,
        ai: None,
    },
    // Unread on the same terms as `weapons` above: this fixture is about
    // archive discovery, not about any weapon's own model.
    weapon_models: &oag_title::weapons::WeaponModels::EMPTY,
    effects: &oag_title::Effects::NONE,
    looks: &oag_title::Looks::unread(oag_title::ShieldPalettes {
        ps2: oag_title::ShieldPalette::Pulse,
        elsewhere: oag_title::ShieldPalette::Pulse,
        origin: oag_title::Origin::Chosen,
    }),
    // A fixture: nothing here reads a campaign.
    campaign: &oag_title::Campaign {
        dialect: oag_title::CampaignDialect::Pulse,
        definition_entry: None,
        circuit_unlocks: false,
        loyalty_unlocks: false,
        unlocks_origin: oag_title::Origin::Chosen,
        selection_strings: false,
        origin: oag_title::Origin::Chosen,
    },
    pressings: None,
    hud_art: &oag_title::HudArt {
        texture_extension: None,
        always_on: &[],
        sights: &oag_title::hud::Sights::Unread,
        pickup_backdrop_colour: None,
        pickup_colours: None,
        pickup_icon_models: None,
        pickup_icon_backdrop_model: None,
        pickup_icon_uv: None,
        zone_speed_classes: None,
        shield_percent: true,
        // Unread here too - this fixture is about archive discovery, not
        // about any language plugin's own font roles.
        hud_font_role: "HUD",
        hud_small_font_role: Some("HUDSmall"),
        total_time_timed_modes_only: false,
        kill_column: false,
        message_slots: false,
        runtime: None,
    },
    // Unread here for the fourth and fifth time, and the same reason both
    // times: an archive is found by name, and what is inside one is nothing
    // this crate looks at.
    plugin_definition: r"Data\Plugins\PI000\Definition.xml",
    track_plugin_definition: None,
    // This crate opens archives; nothing here draws a loading screen.
    loading: None,
    music: None,
    cursor: "",
};

/// The bulk archive's PSP and PS2 names, as the fixture spells them.
const PSP_DATA: &str = TITLE.archives.data[0].0;
const PS2_DATA: &str = TITLE.archives.data[1].0;

use crate::testing;

/// Builds `Archives` over hand-authored files, so the search order can be
/// tested without a disc. `Layout::resolve` is bypassed on purpose: what is
/// under test is which archive answers, not how a source is recognised.
fn mounted(
    dir: &std::path::Path,
    data: &[(&str, &[u8])],
    packs: Vec<crate::dlc::Pack>,
) -> Archives {
    let spec = testing::write_wad(dir, "Data.wad", data);
    let mut manifests = Vec::new();
    let mut mounted = Vec::new();
    for pack in packs {
        manifests.extend(pack.manifests);
        mounted.extend(
            pack.archives
                .into_iter()
                .map(|archive| Container::Wad(Box::new(archive))),
        );
    }
    Archives {
        layout: Layout {
            platform: Platform::Psp,
            data: spec.clone(),
            fe: None,
            extra: Vec::new(),
            serial: None,
        },
        data: Container::open(&spec).expect("the test archive"),
        fe: None,
        extra: Vec::new(),
        packs: mounted,
        manifests,
    }
}

fn pack(dir: &std::path::Path, name: &str, entries: &[(&str, &[u8])]) -> crate::dlc::Pack {
    let sub = dir.join(name);
    std::fs::create_dir_all(&sub).expect("a test pack directory");
    let _ = testing::write_wad(&sub, "PACK.edat", entries);
    crate::dlc::open_dir(&sub).expect("a pack")
}

/// The rule from [`Archives::open_with_packs`]: a pack cannot shadow the
/// disc, whatever it carries.
#[test]
fn the_source_wins_a_collision_with_a_pack() {
    let dir = testing::temp_dir("mount-disc-wins");
    let mut archives = mounted(
        &dir,
        &[("shared.bin", b"from the disc")],
        vec![pack(&dir, "one", &[("shared.bin", b"from the pack")])],
    );

    assert_eq!(archives.read_name("shared.bin").unwrap(), b"from the disc");
}

#[test]
fn a_pack_answers_for_a_name_the_source_does_not_have() {
    let dir = testing::temp_dir("mount-pack-only");
    let mut archives = mounted(
        &dir,
        &[("base.bin", b"disc")],
        vec![pack(&dir, "one", &[("extra.bin", b"pack")])],
    );

    assert_eq!(archives.read_name("extra.bin").unwrap(), b"pack");
    assert_eq!(
        archives.locate("extra.bin"),
        Some(archives.packs[0].label()),
        "and it reports which archive answered"
    );
}

/// First mounted wins. The shipped packs only ever overlap on
/// byte-identical entries, so this rule never has to choose between two
/// different pictures - see `docs/formats/dlc-pack.md`.
#[test]
fn the_first_mounted_pack_wins_between_packs() {
    let dir = testing::temp_dir("mount-first-wins");
    let mut archives = mounted(
        &dir,
        &[("base.bin", b"disc")],
        vec![
            pack(&dir, "first", &[("shared.bin", b"first")]),
            pack(&dir, "second", &[("shared.bin", b"second")]),
        ],
    );

    assert_eq!(archives.read_name("shared.bin").unwrap(), b"first");
}

/// Directory position is per archive. Flattening the mounted archives into
/// one sequence would hand a PS2 model the tail of the previous archive and
/// decode it as a texture set, so the neighbour must come from the archive
/// that held the entry.
#[test]
fn a_neighbour_comes_from_the_archive_that_held_the_entry() {
    let dir = testing::temp_dir("mount-neighbour");
    let mut archives = mounted(
        &dir,
        &[("base.bin", b"disc last entry")],
        vec![pack(
            &dir,
            "one",
            &[("before.bin", b"the neighbour"), ("model.bin", b"model")],
        )],
    );

    assert_eq!(
        archives.read_preceding("model.bin").unwrap(),
        b"the neighbour",
        "not the disc's last entry"
    );
}

/// An entry in nothing at all still fails against the bulk archive, so the
/// message names the file a reader would open first rather than whichever
/// pack happened to be mounted last.
#[test]
fn a_miss_is_reported_against_the_source() {
    let dir = testing::temp_dir("mount-miss");
    let mut archives = mounted(
        &dir,
        &[("base.bin", b"disc")],
        vec![pack(&dir, "one", &[("extra.bin", b"pack")])],
    );

    let error = archives.read_name("absent.bin").unwrap_err();
    let Error::NoSuchEntry { archive, .. } = error else {
        panic!("expected a missing entry, got {error:?}");
    };
    assert!(archive.ends_with("Data.wad"), "{archive}");
}

#[test]
fn a_disc_image_is_joined_with_a_colon() {
    assert_eq!(
        archive_spec("data/images/pulse-psp-usa.chd", PSP_DATA),
        "data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad"
    );
}

#[test]
fn an_extracted_directory_is_joined_with_a_separator() {
    // The current directory always exists, so it stands in for an extract.
    let spec = archive_spec(".", PSP_DATA);
    assert_eq!(spec, "./PSP_GAME/USRDIR/Data.wad");
}

/// A directory shaped like an extract, under a name of its own so two tests
/// never collide.
fn extract(name: &str, files: &[&str]) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("oag-assets-layout-{name}"));
    std::fs::remove_dir_all(&root).ok();
    for file in files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Never opened: resolving a layout is a question about paths, so
        // these do not have to be WADs to answer it.
        std::fs::write(&path, b"").unwrap();
    }
    root
}

#[test]
fn a_psp_extract_resolves_to_the_psp_archives() {
    let root = extract(
        "psp",
        &[
            "UMD_DATA.BIN",
            "PSP_GAME/USRDIR/Data.wad",
            "PSP_GAME/USRDIR/FE.wad",
            "PSP_GAME/USRDIR/BEData.wad",
        ],
    );
    let source = root.to_str().unwrap();

    let layout = Layout::resolve(source, TITLE).unwrap();
    assert_eq!(layout.platform, Platform::Psp);
    assert_eq!(layout.data, format!("{source}/PSP_GAME/USRDIR/Data.wad"));
    assert_eq!(
        layout.fe.as_deref(),
        Some(format!("{source}/PSP_GAME/USRDIR/FE.wad").as_str())
    );

    std::fs::remove_dir_all(&root).ok();
}

/// The set-shaped third role: every candidate present is resolved, and one
/// that is absent is skipped rather than reported.
///
/// A title with seven archives is what forced this - see
/// `ArchiveCandidates::extra`. Asserted against a fixture title rather than
/// Wipeout HD's own, because this crate names no archive.
#[test]
fn every_extra_archive_present_resolves_and_a_missing_one_is_skipped() {
    const SEVEN: &Title = &Title {
        archives: ArchiveCandidates {
            data: &[("USRDIR/DATA00.PSARC", Platform::Ps3)],
            fe: &[("USRDIR/DATA02.PSARC", Platform::Ps3)],
            extra: &[
                ("USRDIR/DATA01.PSARC", Platform::Ps3),
                ("USRDIR/DATA03.PSARC", Platform::Ps3),
                // Not in the extract below, and therefore not in the layout.
                ("USRDIR/DATA99.PSARC", Platform::Ps3),
            ],
        },
        ..*TITLE
    };

    let root = extract(
        "extra",
        &[
            "USRDIR/DATA00.PSARC",
            "USRDIR/DATA01.PSARC",
            "USRDIR/DATA02.PSARC",
            "USRDIR/DATA03.PSARC",
        ],
    );
    let source = root.to_str().unwrap();

    let layout = Layout::resolve(source, SEVEN).unwrap();
    assert_eq!(layout.data, format!("{source}/USRDIR/DATA00.PSARC"));
    assert_eq!(
        layout.fe.as_deref(),
        Some(format!("{source}/USRDIR/DATA02.PSARC").as_str())
    );
    assert_eq!(
        layout.extra,
        [
            format!("{source}/USRDIR/DATA01.PSARC"),
            format!("{source}/USRDIR/DATA03.PSARC"),
        ],
        "in candidate order, with the absent one left out entirely"
    );
    // And the report says how many, so a source that mounted fewer than
    // expected is visible without reading the field.
    assert!(
        layout.describe().ends_with("and 2 more"),
        "{}",
        layout.describe()
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn wipeout_pure_s_serial_is_rejected_by_name() {
    let error = reject_foreign_title("pure-psp-usa.chd", "UCUS-98612", TITLE).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("UCUS-98612"), "{message}");
    assert!(message.contains("Wipeout Pure"), "{message}");
    // **Points at the call that chooses, not at a milestone.** This asserted
    // "M8" until Pure actually opened: `oag_source::title::open_source` catches
    // this error and opens the disc as Pure, so the advice to wait for a
    // milestone became wrong while the error itself stayed right.
    assert!(message.contains("open_source"), "{message}");
}

#[test]
fn an_unrecognised_serial_is_not_rejected() {
    // The deny-list only rules a source *out*; an uncatalogued serial is
    // not assumed to be Pulse either, so it gets no verdict here and is
    // left to archive-name matching, same as before this check existed.
    assert!(reject_foreign_title("some.chd", "ULUS-99999", TITLE).is_ok());
}

#[test]
fn pulse_s_own_serials_are_not_rejected() {
    // Regression guard: this table must never grow a Pulse serial by
    // mistake, or a legitimately-owned disc would hard-reject.
    for serial in ["UCUS-98712", "SCES-54748"] {
        assert!(
            reject_foreign_title("pulse.chd", serial, TITLE).is_ok(),
            "{serial}"
        );
    }
}

#[test]
fn an_extracted_directory_is_never_checked_against_a_title() {
    // An extract carries no disc header, so it has no serial to reject -
    // ADR-0004's "load from your own already-unpacked originals" stays
    // permitted unconditionally, even for a title `OTHER_TITLES` names.
    let root = extract(
        "no-title-check",
        &[
            "UMD_DATA.BIN",
            "PSP_GAME/USRDIR/Data.wad",
            "PSP_GAME/USRDIR/FE.wad",
        ],
    );
    let source = root.to_str().unwrap();

    assert!(Layout::resolve(source, TITLE).is_ok());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_ps2_extract_resolves_through_its_serial_directory() {
    // The directory is named after the disc's serial, so nothing may depend
    // on `54748` in particular: the archive is found by its own name.
    let root = extract(
        "ps2",
        &[
            "SYSTEM.CNF",
            "12345/WADS2.WAD",
            "12345/WADSP.WAD",
            "12345/PS2MUSIC.WAD",
        ],
    );
    let source = root.to_str().unwrap();

    let layout = Layout::resolve(source, TITLE).unwrap();
    assert_eq!(layout.platform, Platform::Ps2);
    assert_eq!(layout.data, format!("{source}/12345/WADS2.WAD"));
    assert_eq!(
        layout.fe.as_deref(),
        Some(format!("{source}/12345/WADSP.WAD").as_str())
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_archive_alone_identifies_a_partial_extract() {
    // `oag-unpack` can be pointed at one directory, so the file that names
    // the console is often absent. The archive that matched is then the only
    // thing that knows which one it is.
    let root = extract("partial", &["WADS2.WAD"]);
    let source = root.to_str().unwrap();

    let layout = Layout::resolve(source, TITLE).unwrap();
    assert_eq!(layout.platform, Platform::Ps2);
    assert_eq!(layout.data, format!("{source}/WADS2.WAD"));
    assert_eq!(layout.fe, None);

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_source_with_no_archive_says_what_it_looked_for() {
    let root = extract("empty", &["SYSTEM.CNF", "IOP/LIBSD.IRX"]);
    let source = root.to_str().unwrap();

    let error = Layout::resolve(source, TITLE).unwrap_err().to_string();
    assert!(error.contains("PS2"), "{error}");
    assert!(error.contains(PSP_DATA), "{error}");
    assert!(error.contains(PS2_DATA), "{error}");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_archive_name_is_matched_on_a_component_boundary() {
    assert!(names("54748/WADS2.WAD", PS2_DATA));
    assert!(names("54748/wads2.wad", PS2_DATA));
    assert!(names("WADS2.WAD", PS2_DATA));
    assert!(names(r"54748\WADS2.WAD", PS2_DATA));
    assert!(names("PSP_GAME/USRDIR/Data.wad", PSP_DATA));

    // A name that merely ends with the candidate's characters is not that
    // candidate, which is the whole reason this is not `ends_with`.
    assert!(!names("54748/NOTWADS2.WAD", PS2_DATA));
    assert!(!names("WADS2.WADX", PS2_DATA));
    // The PSP candidate is a path, so a bare file of the same name in some
    // other directory does not answer for it.
    assert!(!names("elsewhere/Data.wad", PSP_DATA));
}
