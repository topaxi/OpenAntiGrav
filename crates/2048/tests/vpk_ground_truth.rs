//! Wipeout 2048 read straight out of `.vpk` files, against the same title read
//! out of its extracted folders.
//!
//! **`#[ignore]`d: needs the decrypted package in `data/extracted/vita/` and the
//! `.vpk` files `scripts/make-test-vpk.sh` builds from it** into `data/test-vpk/`
//! (with the system `zip`, an independent writer). No real NoNpDrm `.vpk` was on
//! hand, so these are the closest stand-ins: a ZIP of the same folders.
//!
//! ```sh
//! scripts/make-test-vpk.sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-2048 --run-ignored all \
//!     -E 'binary(vpk_ground_truth)'
//! ```

use oag_assets::Archives;

const VPK: &str = "data/test-vpk/2048-PCSF00007.vpk";
const FOLDER: &str = "data/extracted/vita/PCSF00007";

fn open(relative: &str) -> Option<Archives> {
    let source = oag_testdata::exact(relative)?;
    Some(oag_2048::open(&source.display().to_string()).expect("opens"))
}

fn tail(label: &str) -> &str {
    label.rsplit(['/', '\\']).next().expect("a tail")
}

#[test]
#[ignore = "needs data/test-vpk/ (scripts/make-test-vpk.sh) and the extracted package"]
fn a_vpk_opens_as_2048_with_its_patch_and_dlc_mounted_in_the_folders_order() {
    let (Some(vpk), Some(folder)) = (open(VPK), open(FOLDER)) else {
        return;
    };
    assert_eq!(vpk.layout.platform, oag_title::Platform::Vita);
    assert_eq!(vpk.layout.serial.as_deref(), Some("PCSF-00007"));
    let labels = |a: &Archives| {
        let mut all: Vec<String> = a
            .patch
            .iter()
            .map(|c| tail(c.label()).to_string())
            .collect();
        all.push(tail(a.data.label()).to_string());
        all.extend(a.extra.iter().map(|c| tail(c.label()).to_string()));
        all
    };
    assert_eq!(labels(&vpk), labels(&folder));
    assert_eq!(
        labels(&vpk),
        [
            "data2.psarc",
            "data1.psarc",
            "data.psarc",
            "dlc1.psarc",
            "dlc2.psarc"
        ],
        "the patch before the base, both DLC packs after it"
    );
    assert_eq!(vpk.data.entry_count(), folder.data.entry_count());
}

#[test]
#[ignore = "needs data/test-vpk/ (scripts/make-test-vpk.sh) and the extracted package"]
fn every_archive_of_the_set_reads_byte_identical_to_the_folder() {
    let (Some(mut vpk), Some(mut folder)) = (open(VPK), open(FOLDER)) else {
        return;
    };
    let mut compared = 0usize;
    for name in [
        "data/plugins/frontend/NEWGUI/Definition.xml",
        "data/art/published/environments/altima/track.vex",
    ] {
        for (a, b) in vpk
            .read_every_name(name)
            .into_iter()
            .zip(folder.read_every_name(name))
        {
            assert_eq!(tail(&a.0), tail(&b.0));
            assert!(a.1 == b.1, "{name} in {} differs", a.0);
            compared += a.1.len();
        }
    }
    assert!(compared > 100_000, "compared only {compared} bytes");
    println!("compared {compared} bytes across the mounted archives");
}
