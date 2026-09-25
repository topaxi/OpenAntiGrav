//! `oag_pulse::tag_input::ALPHABET`, against the real bytes in `BOOT.BIN` -
//! on both pressings, since the constant claims to be identical on each.
//!
//! `#[ignore]`d: needs a real image under `data/images/`. `just test-data`.

use oag_pulse::tag_input::{ALPHABET, LOCATIONS};

const BOOT: &str = "PSP_GAME/SYSDIR/BOOT.BIN";

/// `None`, with a printed reason, when the disc is not here - unless
/// `OAG_REQUIRE_GAME_DATA=1`, which turns absence into a failure.
fn image(name: &str) -> Option<String> {
    oag_testdata::image(name).map(|path| path.display().to_string())
}

fn read_boot(source: &str) -> Vec<u8> {
    let mut image = oag_disc::DiscImage::open(source).expect("the image opens");
    image
        .read_file(BOOT)
        .unwrap_or_else(|e| panic!("{source}:{BOOT}: {e}"))
}

/// The alphabet is the same 70 bytes, NUL-terminated, at each pressing's own
/// file offset - checked directly rather than trusting the constant that was
/// typed from this same read.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn alphabet_matches_both_pressings_boot_bin() {
    let discs = [("USA", "pulse-psp-usa.chd"), ("EU", "pulse-psp-eu.chd")];
    let mut checked = 0;
    for (region, disc) in discs {
        let Some(path) = image(disc) else { continue };
        let location = LOCATIONS
            .iter()
            .find(|l| l.region == region)
            .unwrap_or_else(|| panic!("no recorded BOOT.BIN offset for {region}"));
        let data = read_boot(&path);
        let end = location.file_offset + ALPHABET.len();
        let slice = &data[location.file_offset..end];
        assert_eq!(
            std::str::from_utf8(slice).expect("the run is ASCII"),
            ALPHABET,
            "{region} BOOT.BIN at offset {} does not match ALPHABET",
            location.file_offset
        );
        assert_eq!(
            data[end], 0,
            "{region}'s copy is not NUL-terminated where ALPHABET claims it ends"
        );
        checked += 1;
    }
    assert!(
        checked > 0 || std::env::var("OAG_REQUIRE_GAME_DATA").as_deref() != Ok("1"),
        "OAG_REQUIRE_GAME_DATA=1 but neither pressing was found"
    );
}
