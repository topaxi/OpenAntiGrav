//! Validates Pure's DLC decryption against the real packs: all seven decrypt,
//! parse as ordinary WADs, and the Gamma pack's `Vanuber` team is a real,
//! race-usable ship.
//!
//! **`#[ignore]`d and never run in CI.** It needs two independent pieces of
//! user-supplied data - `data/dlc/` and `data/keys/pure-dlc-keys.txt` - and
//! skips cleanly if either is absent. See
//! [ADR-0033](../../../docs/architecture/adr/0033-external-key-material-for-decryption.md)
//! for why the key table is never in this repository, and
//! `docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table`
//! for the algorithm and the entry counts asserted below.
//!
//! ```sh
//! just test-data
//! ```

use std::path::{Path, PathBuf};

use oag_game::dlc;

fn workspace(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

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

/// The key table, or `None` when this checkout has not sourced one - see
/// [ADR-0033](../../../docs/architecture/adr/0033-external-key-material-for-decryption.md).
/// Independent of [`dlc_root`]: a checkout can have the packs and not the
/// keys, or the keys and not the packs, and either skips these tests alone.
///
/// **Reads `workspace("data/keys/...")` directly rather than
/// `oag_game::boot::default_pure_dlc_keys_path`**, which resolves relative to
/// the process's current directory and is meant for the CLI binary, invoked
/// from a checkout root - not for a test binary, whose working directory is
/// not this crate's `CARGO_MANIFEST_DIR`. [`workspace`] is what every other
/// path in this file already uses for the same reason.
fn keys() -> Option<Vec<oag_formats::pure_dlc::DlcKey>> {
    let path = workspace("data/keys/pure-dlc-keys.txt");
    if path.is_file() {
        let text = std::fs::read_to_string(&path).expect("reading a file just proven to exist");
        return Some(oag_formats::pure_dlc::parse_keys(&text));
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
    workspace("data/cache/pure-dlc-ground-truth")
}

/// Every Pure pack under `data/dlc`, decrypted where a key fits.
///
/// `dlc::pure_packs`, not `dlc::packs` - `data/dlc/` also holds Pulse's four
/// packs, and the two are discovered from separate cache subtrees precisely
/// so a list like this one never has to filter Pulse's out by name (see
/// `crates/game/src/dlc.rs`'s module docs for why that split exists).
fn packs() -> Option<Vec<oag_assets::dlc::Pack>> {
    let root = dlc_root()?;
    let keys = keys()?;
    let (packs, problems) = dlc::pure_packs(&[root], &cache(), &keys);
    for problem in &problems {
        println!("skipped: {problem}");
    }
    Some(packs)
}

/// Wipeout Pure's seven PSN packs, one zip apiece except Omega (two packs in
/// one zip), by their `PARAM.sfo` content id and the entry count `oag-wad`
/// reports once decrypted - see the table on `dlc-pack.md`. Checked as a set
/// of archive entry counts rather than by name, because a pack's own manifest
/// entry 0 is what names it, not its content id, and that is
/// [`the_gamma_packs_manifest_names_vanuber`]'s job, not this one's.
const EXPECTED_ENTRY_COUNTS: [usize; 8] = [18, 216, 192, 13, 14, 147, 38, 10];

#[test]
#[ignore = "needs Pure's DLC packs in data/dlc/ and a key table in data/keys/pure-dlc-keys.txt"]
fn every_pure_pack_decrypts_and_parses_as_a_wad() {
    let Some(packs) = packs() else {
        return;
    };

    // `packs()` is `dlc::pure_packs`, so Pulse's own four packs under
    // `data/dlc/` never reach this list at all - nothing to filter here.
    let mut counts: Vec<usize> = packs
        .iter()
        .flat_map(|pack| &pack.archives)
        .map(|archive| archive.directory().entries.len())
        .collect();
    counts.sort_unstable();

    let mut expected = EXPECTED_ENTRY_COUNTS.to_vec();
    expected.sort_unstable();

    assert_eq!(
        counts, expected,
        "expected one archive per Pure pack, matching dlc-pack.md's table; \
         a mismatch means a pack failed to decrypt, or the key table has \
         changed shape"
    );
}

/// The finding that closed the `Van_Uber` thread: the Gamma pack's manifest
/// declares team id `Vanuber` (no underscore), and its `Ship.vex` and
/// `handlingstats.xml` are both really in the decrypted archive.
#[test]
#[ignore = "needs Pure's DLC packs in data/dlc/ and a key table in data/keys/pure-dlc-keys.txt"]
fn the_gamma_pack_carries_a_full_vanuber_team() {
    let Some(packs) = packs() else {
        return;
    };

    // The Gamma pack is the one whose manifest names Vanuber - identified by
    // content, not by which zip it came from, since a maintainer's `data/dlc/`
    // layout is not this test's business.
    let gamma = packs
        .into_iter()
        .flat_map(|pack| pack.manifests)
        .find(|manifest| manifest.contains("Vanuber"))
        .expect("one of the packs under data/dlc/ names Vanuber - is the Gamma pack present?");
    assert!(
        gamma.contains(r"Data\Ships\Vanuber"),
        "Vanuber is named but not under the folder id race::ship_entry_name composes paths from"
    );

    let archives = dlc::pure_packs(&[dlc_root().unwrap()], &cache(), &keys().unwrap()).0;
    let found = archives.iter().flat_map(|p| &p.archives).any(|archive| {
        archive.contains(r"Data\Ships\Vanuber\Ship.vex")
            && archive.contains(r"Data\Ships\Vanuber\handlingstats.xml")
    });
    assert!(
        found,
        "the manifest names Vanuber but no mounted archive holds both its \
         ship model and its handling stats"
    );
}
