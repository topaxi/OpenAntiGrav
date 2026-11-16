//! Sony's HD update chain over both sources: which archive serves each path, and
//! what each package changed. `docs/formats/hd-updates.md`.
//!
//! **`#[ignore]`d and never run in CI.** It needs the disc image, the PSN install and
//! the chain `scripts/install-hd-updates.sh` installs under
//! `data/extracted/ps3-update-chain/`. Skips without them, and fails under
//! `OAG_REQUIRE_GAME_DATA=1` when one is missing.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-hd --run-ignored all \
//!     -E 'test(/^hd_update_chain_ground_truth::/)'
//! ```

use std::collections::{BTreeMap, BTreeSet};

use oag_assets::{Archives, Container, UpdateFolder};

const FURY: &str = "data/extracted/ps3-update-chain/BCES00664";
const PSN: &str = "data/extracted/ps3-update-chain/NPEA00057";
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";
const PSN_BASE: &str = "data/extracted/ps3/hd-psn-eu";

/// Every path of an archive with its bytes, compared by content: a PSARC's own entry
/// digest is of the path, and two copies of one file differ in their headers.
fn entries(spec: &str) -> BTreeMap<String, Vec<u8>> {
    let mut archive = Container::open(spec).expect("the archive opens");
    let Container::Psarc(psarc) = &archive else {
        panic!("{spec} is a PSARC")
    };
    let paths: Vec<String> = psarc.paths().to_vec();
    paths
        .into_iter()
        .map(|path| {
            let bytes = archive.read_entry(&path).expect("the entry reads");
            (path.to_ascii_lowercase(), bytes)
        })
        .collect()
}

fn file_name(spec: &str) -> String {
    spec.rsplit(['/', '\\', ':'])
        .next()
        .unwrap_or(spec)
        .to_ascii_lowercase()
}

/// The paths whose copies differ between two archives, which must hold the same paths.
fn differing(a: &BTreeMap<String, Vec<u8>>, b: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    assert_eq!(
        a.keys().collect::<Vec<_>>(),
        b.keys().collect::<Vec<_>>(),
        "the packages ship the same paths"
    );
    a.iter()
        .filter(|(path, bytes)| b[*path] != **bytes)
        .map(|(path, _)| path.rsplit('/').nth(1).unwrap_or(path).to_string())
        .collect()
}

fn fury() -> Option<(String, String)> {
    let chain = oag_testdata::exact(FURY)?;
    let image = oag_testdata::image(PS3_IMAGE)?;
    Some((
        image.to_str().expect("utf-8 path").to_string(),
        chain.to_str().expect("utf-8 path").to_string(),
    ))
}

/// Fury's chain over the disc, as `oag_source` hands it in.
fn fury_updated(image: &str, chain: &str) -> Archives {
    let update = UpdateFolder {
        serial: "BCES-00664".to_string(),
        root: format!("{chain}/installed"),
    };
    oag_hd::open_updated(image, &[update]).expect("the disc opens with its update")
}

/// The win table of the chain over the disc: for every path of either update archive,
/// `data08` beats `data07` and both beat every disc copy, and without the update the
/// disc serves exactly what it did. Counts are (winner, best disc copy lost to it).
#[test]
#[ignore = "needs the disc and the update chain in data/extracted/ps3-update-chain"]
fn the_chain_wins_over_the_disc_path_by_path() {
    let Some((image, chain)) = fury() else { return };
    let data07 = entries(&format!("{chain}/installed/USRDIR/data07.psarc"));
    let data08 = entries(&format!("{chain}/installed/USRDIR/data08.psarc"));
    let updated = fury_updated(&image, &chain);
    let disc = oag_hd::open(&image).expect("the disc opens");

    assert_eq!(
        updated
            .layout
            .patch
            .iter()
            .map(|spec| file_name(spec))
            .collect::<Vec<_>>(),
        ["data08.psarc", "data07.psarc"]
    );
    assert!(disc.layout.patch.is_empty(), "no update, no patch role");
    assert_eq!(updated.layout.update_gap(oag_hd::TITLE), None);

    let mut wins: BTreeMap<(String, String), usize> = BTreeMap::new();
    let paths: BTreeSet<&String> = data07.keys().chain(data08.keys()).collect();
    for path in paths {
        let winner = updated.locate(path).map(file_name).expect("served");
        let expected = if data08.contains_key(path) {
            "data08.psarc"
        } else {
            "data07.psarc"
        };
        assert_eq!(winner, expected, "{path}");

        let loser = disc.locate(path).map_or("none".to_string(), file_name);
        assert!(
            loser == "none" || loser.starts_with("data0"),
            "{path} on the disc alone: {loser}"
        );
        *wins.entry((winner, loser)).or_default() += 1;
    }
    let table: Vec<(&str, &str, usize)> = wins
        .iter()
        .map(|((winner, loser), count)| (winner.as_str(), loser.as_str(), *count))
        .collect();
    assert_eq!(
        table,
        [
            ("data07.psarc", "data00.psarc", 1),
            ("data07.psarc", "data02.psarc", 4),
            ("data07.psarc", "none", 8),
            ("data08.psarc", "data00.psarc", 1),
            ("data08.psarc", "data02.psarc", 19),
        ],
        "33 paths; 17 of data07's 30 are shadowed by data08"
    );
}

/// What the mix reads moves with the chain, the change a player can hear.
#[test]
#[ignore = "needs the disc and the update chain in data/extracted/ps3-update-chain"]
fn the_mix_reads_the_updates_copy() {
    let Some((image, chain)) = fury() else { return };
    let mut updated = fury_updated(&image, &chain);
    let mut disc = oag_hd::open(&image).expect("the disc opens");
    let name = r"Data\sound\GlobalAudioConfig.xml";
    let on_the_disc = disc.read_name(name).expect("the disc's copy reads");
    assert_eq!(
        updated.locate(name).map(file_name).as_deref(),
        Some("data07.psarc")
    );
    assert_ne!(
        updated.read_name(name).expect("the copy reads"),
        on_the_disc
    );
}

/// Which paths each package moved, by content. Fury's 02.30 and 02.50 `data08` differ
/// in Korean alone; PSN's revisions move further, and both releases' last archives
/// are one file.
#[test]
#[ignore = "needs the update chain in data/extracted/ps3-update-chain"]
fn each_package_moved_the_paths_named_here() {
    let (Some(fury), Some(psn)) = (oag_testdata::exact(FURY), oag_testdata::exact(PSN)) else {
        return;
    };
    let (fury, psn) = (fury.display(), psn.display());
    let read = |root: String| entries(&root);

    let fury_230 = read(format!("{fury}/02.30/data08.psarc"));
    let fury_250 = read(format!("{fury}/02.50/data08.psarc"));
    let psn_220 = read(format!("{psn}/02.20/data08.psarc"));
    let psn_230 = read(format!("{psn}/02.30/data08.psarc"));
    let psn_250 = read(format!("{psn}/02.50/data08.psarc"));

    assert_eq!(fury_230.len(), 20);
    assert_eq!(differing(&fury_230, &fury_250), ["korean"]);
    assert_eq!(
        differing(&psn_230, &psn_250),
        ["english", "french", "korean"]
    );
    assert_eq!(differing(&psn_220, &psn_230).len(), 12);
    assert_eq!(differing(&fury_230, &psn_230), ["english", "french"]);
    assert_eq!(differing(&fury_250, &psn_250), Vec::<String>::new());

    let fury_data07 = read(format!("{fury}/02.10/data07.psarc"));
    let psn_data07 = read(format!("{psn}/02.10/data07.psarc"));
    assert_eq!(fury_data07.len(), 30);
    assert_eq!(differing(&fury_data07, &psn_data07), Vec::<String>::new());
}

/// A PSN install patched in place is the same folder shape, found by the same names:
/// the chain over it, 02.01's archives in the base's own roles, `data04` the update's,
/// and still the PSN variant. The base folder beside it stays the unpatched source.
#[test]
#[ignore = "needs the PSN install and its chain in data/extracted/"]
fn a_patched_psn_install_mounts_the_whole_chain() {
    let (Some(chain), Some(base)) = (oag_testdata::exact(PSN), oag_testdata::exact(PSN_BASE))
    else {
        return;
    };
    let installed = format!("{}/installed", chain.display());
    let mut patched = oag_hd::open(&installed).expect("the patched install opens");
    let mut plain = oag_hd::open(base.to_str().expect("utf-8 path")).expect("the base opens");

    assert_eq!(
        patched
            .layout
            .patch
            .iter()
            .map(|spec| file_name(spec))
            .collect::<Vec<_>>(),
        ["data08.psarc", "data07.psarc"]
    );
    assert_eq!(file_name(&patched.layout.data), "data03.psarc");
    let extra: Vec<String> = patched.layout.extra.iter().map(|s| file_name(s)).collect();
    assert_eq!(
        extra,
        [
            "data01.psarc",
            "data04.psarc",
            "data05.psarc",
            "data06.psarc"
        ]
    );
    assert_eq!(
        oag_hd::title_of(&patched).race.track,
        oag_hd::psn::DEFAULT_TRACK
    );

    let strings = "data/plugins/languages/english/entries.xml";
    assert_eq!(
        patched.locate(strings).map(file_name).as_deref(),
        Some("data08.psarc")
    );
    assert_eq!(
        patched
            .locate("data/sound/globalaudioconfig.xml")
            .map(file_name)
            .as_deref(),
        Some("data07.psarc")
    );

    let medal = "data/fe/trophies/materials/medal.rcsmaterial";
    let data04 = |archives: &mut Archives| {
        archives
            .extra
            .iter_mut()
            .find(|archive| file_name(archive.label()) == "data04.psarc")
            .expect("data04 is mounted")
            .read_entry(medal)
            .expect("the entry reads")
    };
    assert!(
        data04(&mut patched) != data04(&mut plain),
        "02.01's data04 replaces one entry of the base's, in the same role"
    );
}
