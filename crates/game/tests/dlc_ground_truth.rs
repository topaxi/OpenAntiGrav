//! Validates downloadable content against real packs: the four Pulse packs
//! open, declare the teams they should, and mount against a disc **from a
//! different territory**.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! Two claims, and neither is provable from a fixture.
//!
//! The first is the feature: **a European pack works against the American
//! disc.** The original locked a pack to its own territory and this build does
//! not, so the assertion that matters is a count taken off `pulse-psp-usa.chd`
//! with the `UCES00465` packs mounted - twelve teams where the disc alone has
//! eight. A unit test can prove the mounting rule; only this can prove the two
//! real files agree.
//!
//! The second is that the packs are what `docs/formats/dlc-pack.md` says: plain
//! WADs, with the ship model in `PACKn.edat` and the handling stats in
//! `PACKn_UI1.edat`. Both are re-derived here rather than asserted against a
//! constant, so a wrong reading fails rather than passing against its own copy
//! of itself.
//!
//! Nothing here spells a team name, a circuit name or a tuning value: the ids
//! are folder names, and the one label assertion checks only that the string
//! table *disagrees* with the id, never what it says.

use std::path::{Path, PathBuf};

use oag_game::boot;
use oag_pulse::race::DLC_TEAMS as PACK_TEAMS;
use oag_raceplay as race;
use oag_raceplay::catalogue;
use oag_source::dlc;

/// The American disc, on purpose. Mounting European packs on it is the whole
/// point of the test.
const IMAGE: &str = "pulse-psp-usa.chd";

/// What the disc alone offers, so the numbers below are a difference rather
/// than a magic constant.
const BASE_TEAMS: usize = 8;
const BASE_TRACKS: usize = 24;

fn workspace(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn image() -> Option<String> {
    oag_testdata::image(IMAGE).map(|path| path.display().to_string())
}

/// The DLC directory, or `None` when this checkout has no packs.
///
/// Separate from [`image`] because the two are independent: a checkout can have
/// the disc and no packs, which skips these and nothing else.
fn dlc_root() -> Option<PathBuf> {
    let path = workspace("data/dlc");
    if path.is_dir() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn cache() -> PathBuf {
    workspace("data/cache/dlc")
}

/// Every Pulse pack under `data/dlc`, unzipped into the cache if it is still a
/// zip. `dlc::packs`, not `dlc::pure_packs` - the two discover from separate
/// cache subtrees (see `crates/source/src/dlc.rs`'s module docs), which is what
/// keeps this list Pulse's own even though `data/dlc/` also holds seven
/// Wipeout Pure packs and whatever else a maintainer has downloaded.
///
/// **Problems are printed, not asserted on.** An unrelated zip failing to
/// open (empty keys here mean Pure's packs are among those, whether or not
/// this checkout has sourced its own key table) would send someone chasing a
/// Pulse regression that does not exist - and the production path treats
/// exactly the same condition as skip-and-report. What these tests assert is
/// that the four Pulse packs are found, which is a statement about what *is*
/// there.
fn packs() -> Option<Vec<oag_assets::dlc::Pack>> {
    let root = dlc_root()?;
    let (packs, problems) = dlc::packs(&[root], &cache(), &[]);
    for problem in &problems {
        println!("skipped: {problem}");
    }
    Some(packs)
}

/// The manifests, which is what a pack declares rather than what it stores.
fn manifests() -> Option<Vec<String>> {
    Some(packs()?.into_iter().flat_map(|p| p.manifests).collect())
}

#[test]
#[ignore = "needs DLC packs in data/dlc/"]
fn the_four_pulse_packs_declare_the_four_teams_they_add() {
    let Some(manifests) = manifests() else {
        return;
    };

    let mut ids: Vec<String> = manifests
        .iter()
        .flat_map(|xml| catalogue::teams(xml))
        .map(|team| team.id)
        .collect();
    ids.sort();
    ids.dedup();

    assert_eq!(
        ids, PACK_TEAMS,
        "the packs' own manifests name these teams; a mismatch means either a \
         different set of packs is present or the manifest reading has drifted"
    );
}

/// The id/label split, checked against the disc rather than assumed.
///
/// Two different claims, and they are not the same one:
///
/// 1. **Every pack team is named on the American disc**, in the string table
///    the front end already loads. Nothing had to be added for a pack's team to
///    read correctly in any of the five shipped languages - which is a large
///    part of why region-independent mounting is cheap here.
/// 2. **For `Mantis` the label is a different word from the id.** That is the
///    case the whole id/label distinction exists for: the packaging calls that
///    pack Mirage, the folder is `Mantis`, and a build that put the id in the
///    menu would show the wrong name. The other three happen to match, which is
///    exactly why asserting "differs" for all four would be wrong.
///
/// Neither asserts what the table *says*: that text is shipped content.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pack_team_is_named_by_the_disc_and_its_id_is_not_always_that_name() {
    let Some(image) = image() else {
        return;
    };
    let Some(_) = dlc_root() else {
        return;
    };

    let loaded = boot::load(&boot::Options {
        language: None,
        source: image,
        dlc: vec![workspace("data/dlc")],
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: Some(boot::DEFAULT_BOOT_MOVIE.to_string()),
        cache: std::env::temp_dir().join("oag-dlc-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(1),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    })
    .expect("booting with the packs mounted");

    for id in PACK_TEAMS {
        assert!(
            loaded.strings.get(id).is_some(),
            "{id} has no entry in the disc's own string table, so the menu \
             would fall back to showing the folder name"
        );
    }

    assert_ne!(
        loaded.strings.get("Mantis"),
        Some("Mantis"),
        "the Mirage pack's team is stored as Mantis and shown as something \
         else; if these ever agree, the id/label split has been flattened and \
         the menu is showing the wrong word"
    );
}

/// The assertion that encodes the whole feature: European packs, American
/// disc, twelve teams and thirty-two circuits.
#[test]
#[ignore = "needs a disc image in data/images/ and DLC packs in data/dlc/"]
fn european_packs_mount_against_the_american_disc() {
    let Some(image) = image() else {
        return;
    };
    let Some(root) = dlc_root() else {
        return;
    };

    let (packs, _) = dlc::packs(&[root], &cache(), &[]);
    let mut archives = oag_pulse::open_with_packs(&image, packs).expect("mounting the packs");

    let base = oag_pulse::open(&image).expect("the disc alone");
    let base_definition = read_definition(&mut { base });
    let mut documents = vec![read_definition(&mut archives)];
    documents.extend(archives.manifests.clone());

    assert_eq!(
        catalogue::teams(&base_definition).len(),
        BASE_TEAMS,
        "the disc alone"
    );
    assert_eq!(
        catalogue::all_teams(&documents).len(),
        BASE_TEAMS + PACK_TEAMS.len(),
        "the disc plus the packs"
    );

    assert_eq!(
        catalogue::tracks(&base_definition).len(),
        BASE_TRACKS,
        "the disc alone"
    );
    let tracks = catalogue::all_tracks(&documents);
    assert_eq!(
        tracks.len(),
        BASE_TRACKS + 8,
        "each pack declares two circuits and the four sets do not overlap"
    );

    // And every one of them is loadable, which is the difference between a
    // declaration and a circuit: two packs cross-declare the reversed halves of
    // each other's environments, so this is also what proves a full set is
    // self-contained.
    for track in &tracks {
        assert!(
            archives.locate(&track.entry_name()).is_some(),
            "{} declares {} and nothing mounted holds it",
            track.id,
            track.entry_name()
        );
    }
}

/// A pack ship is a real ship: it decodes, and it has the handling stats a
/// race needs. Both halves matter - the model is in `PACKn.edat` and the stats
/// are in `PACKn_UI1.edat`, so this fails if either archive is left unmounted.
#[test]
#[ignore = "needs a disc image in data/images/ and DLC packs in data/dlc/"]
fn every_pack_team_loads_a_ship_and_a_full_set_of_handling_stats() {
    let Some(image) = image() else {
        return;
    };
    let Some(root) = dlc_root() else {
        return;
    };

    for team in PACK_TEAMS {
        let loaded = race::load(&race::Options {
            source: image.clone(),
            dlc: vec![root.clone()],
            team: Some(team.to_string()),
            class: "VENOM".to_string(),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("loading {team} off {IMAGE} with the packs: {e:#}"));

        assert!(
            !loaded.liveries[0].hull.draws.is_empty(),
            "{team}'s ship decoded to nothing"
        );
        // Every speed class, not just the one loaded: a stats file short a
        // `<Class>` would load here and fail the moment a player changed the
        // class, which is the wrong place to find out.
        for class in oag_title::SpeedClasses::PULSE_LADDER {
            race::load(&race::Options {
                source: image.clone(),
                dlc: vec![root.clone()],
                team: Some(team.to_string()),
                class: (*class).to_string(),
                ..race::Options::default()
            })
            .unwrap_or_else(|e| panic!("loading {team} in {class}: {e:#}"));
        }
    }
}

/// The pack teams are absent from the disc, which is what makes them
/// downloadable content rather than something already there under another name.
///
/// This is the assertion that retired the repository's "confirmed PS2-only"
/// reading of `Auricom`, `Harimau` and `Icaras`: they are absent from the PSP
/// disc, and present in a PSP pack.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_disc_alone_carries_none_of_the_pack_teams() {
    let Some(image) = image() else {
        return;
    };
    let archives = oag_pulse::open(&image).expect("the disc alone");

    for team in PACK_TEAMS {
        let name = format!(r"Data\Ships\{team}\Ship.vex");
        assert!(
            archives.locate(&name).is_none(),
            "{name} is on the disc after all, which would make this not a pack team"
        );
    }
}

/// Mounting changes nothing the disc already answered for.
#[test]
#[ignore = "needs a disc image in data/images/ and DLC packs in data/dlc/"]
fn mounting_packs_leaves_the_disc_s_own_entries_alone() {
    let Some(image) = image() else {
        return;
    };
    let Some(root) = dlc_root() else {
        return;
    };

    let mut base = oag_pulse::open(&image).expect("the disc alone");
    let (packs, _) = dlc::packs(&[root], &cache(), &[]);
    let mut mounted = oag_pulse::open_with_packs(&image, packs).expect("mounting the packs");

    let name = oag_pulse::names::GAME_PLUGIN_DEFINITION;
    assert_eq!(
        base.read_name(name).expect("the disc's definition"),
        mounted.read_name(name).expect("the same, with packs"),
        "a pack must not shadow a disc entry"
    );
}

fn read_definition(archives: &mut oag_assets::Archives) -> String {
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    oag_tables::fexml::expand(&blob).expect("expanding it")
}
