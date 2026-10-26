//! Wipeout 2048's downloadable circuits reach the catalogue from the packages'
//! own circuit list, and only for the packages the player owns.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted EU and USA
//! packages under `data/extracted/vita/` (`PCSF00007`: base, patch, `dlc1`,
//! `dlc2`; `PCSA00015`: base, patch, `dlc1`). Run with `just test-data`.
//!
//! The trap this holds the line on: `data.psarc` ships a sixteen-circuit,
//! forward-only copy of `Data\art\published\DLC1\plugins\tracks\Definition.xml`
//! and wins that name, and it also ships the geometry of four of the circuits
//! (`Vineta_K`, `Anulpha_Pass`, `Moa_Therma`, `Chenghou_Project`). Reading by
//! name, or keeping whatever has geometry, offers circuits with no package
//! mounted at all.

use std::path::{Path, PathBuf};

use oag_assets::Archives;
use oag_raceplay::catalogue::{self, Track};

const DEFINITION: &str = oag_2048::names::DLC_TRACK_DEFINITION;

fn package(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita")
        .join(name);
    if path.join("base/PSP2/data.psarc").exists() {
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

/// The EU package with its two downloadable archives left out: real folders
/// holding links to the base archive and the patch's two, because the source
/// walk does not follow a linked folder.
struct WithoutPacks(PathBuf);

impl WithoutPacks {
    fn of(package: &Path, tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!("oag-2048-no-packs-{tag}"));
        let _ = std::fs::remove_dir_all(&root);
        for archive in [
            "base/PSP2/data.psarc",
            "patch-v104/PSP2/data1.psarc",
            "patch-v104/PSP2/data2.psarc",
        ] {
            let link = root.join(archive);
            std::fs::create_dir_all(link.parent().expect("a folder")).expect("a scratch folder");
            std::os::unix::fs::symlink(
                package
                    .join(archive)
                    .canonicalize()
                    .expect("a package archive"),
                link,
            )
            .expect("a link");
        }
        Self(root)
    }
}

impl Drop for WithoutPacks {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// What the boot's catalogue offers: every circuit declared by the title's
/// definitions and the mounted packages, whose geometry is mounted.
fn offered(archives: &mut Archives) -> Vec<Track> {
    let documents = catalogue::documents(
        archives,
        &[
            oag_2048::TITLE.plugin_definition,
            oag_2048::TITLE
                .track_plugin_definition
                .expect("2048 splits its circuits out"),
        ],
    );
    catalogue::all_tracks(&documents)
        .into_iter()
        .filter(|track| archives.locate(&track.entry_name()).is_some())
        .collect()
}

fn ids(tracks: &[Track]) -> Vec<&str> {
    tracks.iter().map(|track| track.id.as_str()).collect()
}

const NATIVE: [&str; 10] = [
    "square",
    "park",
    "tower",
    "mall",
    "bridge",
    "arena",
    "subway",
    "cathedral",
    "sol",
    "altima",
];

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn both_packages_add_every_circuit_in_both_directions() {
    let Some(eu) = package("PCSF00007") else {
        return;
    };
    let mut archives = Archives::open(&eu.display().to_string(), oag_2048::TITLE).expect("EU");
    let labels: Vec<&str> = archives.add_ons.iter().map(|a| a.archive).collect();
    assert_eq!(labels, ["dlc1.psarc", "dlc2.psarc"]);

    let tracks = offered(&mut archives);
    let ids = ids(&tracks);
    assert_eq!(
        ids.len(),
        10 + 28,
        "ten native circuits and the twenty-eight"
    );
    assert_eq!(ids[..10], NATIVE);
    for id in [
        "Metropia",
        "Sol_2",
        "Vineta_K_R",
        "Ubermall_R",
        "Talons_Junction",
        "Tech_De_Ra_R",
        "Zone_1",
    ] {
        assert!(ids.contains(&id), "{id} is offered: {ids:?}");
    }
    let reversed = tracks
        .iter()
        .find(|track| track.id == "Vineta_K_R")
        .expect("Vineta_K_R");
    assert!(reversed.reversed, "the package's own lower-case `reversed`");
    assert!(
        reversed
            .entry_name()
            .ends_with(r"DLC1\environments\Vineta_K\track_reversed.vex"),
        "{}",
        reversed.entry_name()
    );
}

#[test]
#[ignore = "needs data/extracted/vita/PCSA00015"]
fn one_package_offers_only_the_circuits_it_holds() {
    let Some(usa) = package("PCSA00015") else {
        return;
    };
    let mut archives = Archives::open(&usa.display().to_string(), oag_2048::TITLE).expect("USA");
    let labels: Vec<&str> = archives.add_ons.iter().map(|a| a.archive).collect();
    assert_eq!(labels, ["dlc1.psarc"], "the USA store sold no second pack");

    let tracks = offered(&mut archives);
    let ids = ids(&tracks);
    for id in ["Vineta_K", "Metropia", "Sol_2_R"] {
        assert!(ids.contains(&id), "{id} is in dlc1: {ids:?}");
    }
    for id in ["Talons_Junction", "Amphiseum_R", "Zone_1", "Tech_De_Ra"] {
        assert!(!ids.contains(&id), "{id} is in dlc2, not mounted: {ids:?}");
    }
    assert_eq!(ids.len(), 10 + 16, "eight circuits in both directions");
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn no_package_offers_the_native_circuits_though_the_base_holds_four_more() {
    let Some(eu) = package("PCSF00007") else {
        return;
    };
    let bare = WithoutPacks::of(&eu, "catalogue");
    let mut archives =
        Archives::open(&bare.0.display().to_string(), oag_2048::TITLE).expect("no packs");
    assert!(archives.add_ons.is_empty());

    // The geometry is mounted: the base archive carries it. That alone does not
    // offer it, which is the point of gating on the package.
    assert!(
        archives
            .locate(r"Data\art\published\DLC1\environments\Vineta_K\track.vex")
            .is_some()
    );
    assert_eq!(ids(&offered(&mut archives)), NATIVE);
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_package_is_read_for_its_own_list_not_the_skeleton_the_base_wins() {
    let Some(eu) = package("PCSF00007") else {
        return;
    };
    let mut archives = Archives::open(&eu.display().to_string(), oag_2048::TITLE).expect("EU");
    let by_name =
        String::from_utf8(archives.read_name(DEFINITION).expect("served by name")).expect("text");
    assert!(
        !by_name.contains("Vineta_K_R"),
        "the base's copy has no reversed entries, or this test proves nothing"
    );
    let declared = catalogue::all_tracks(&archives.manifests);
    assert!(
        declared.iter().any(|track| track.id == "Vineta_K_R"),
        "the package's own copy reached the manifests"
    );
}
