//! A Fury disc with its update folder beside it opens through the route a player's
//! does: `open_source` finds the folder by the disc's own release and mounts the chain.
//!
//! **`#[ignore]`d and never run in CI**: needs the disc image and the chain
//! `scripts/install-hd-updates.sh` lays out. Skips without them and fails under
//! `OAG_REQUIRE_GAME_DATA=1` when one is missing.

#![cfg(unix)]

use oag_source::title::open_source;

const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";
const CHAIN: &str = "data/extracted/ps3-update-chain/BCES00664/installed";

#[test]
#[ignore = "needs the disc and the update chain in data/extracted/ps3-update-chain"]
fn a_disc_mounts_the_update_folder_beside_it() {
    let (Some(image), Some(chain)) = (oag_testdata::image(PS3_IMAGE), oag_testdata::exact(CHAIN))
    else {
        return;
    };
    let beside = std::env::temp_dir().join(format!("oag-hd-update-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&beside);
    std::fs::create_dir_all(&beside).expect("a scratch folder");
    let linked = beside.join("hd.iso");
    let absolute = |path: &std::path::Path| path.canonicalize().expect("the path exists");
    std::os::unix::fs::symlink(absolute(&image), &linked).expect("the image link");
    std::os::unix::fs::symlink(absolute(&chain), beside.join("BCES00664"))
        .expect("the update link");

    let opened = open_source(linked.to_str().expect("utf-8 path"), Vec::new(), Vec::new())
        .expect("the disc opens");
    let layout = opened.archives.layout.clone();
    std::fs::remove_dir_all(&beside).expect("the scratch folder goes");

    assert_eq!(opened.title.name, oag_hd::TITLE.name);
    let tails: Vec<String> = layout
        .patch
        .iter()
        .map(|spec| spec.rsplit('/').next().unwrap_or(spec).to_string())
        .collect();
    assert_eq!(tails, ["data08.psarc", "data07.psarc"]);
    assert_eq!(layout.serial.as_deref(), Some("BCES-00664"));
}
