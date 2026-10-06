//! Validates the [`ship_skin`](oag_texture::ship_skin) decoder against a real
//! shipped skin file.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The tests skip with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure, which is what a
//! release check wants: a skipped ground-truth test is green and proves
//! nothing.
//!
//! # What this is for
//!
//! `docs/ghidra/functions/psp-pulse-usa/ship-skin.md` claims a `.dat` skin is four
//! fixed-shape palette-plus-pixels blocks behind a NUL-terminated header, with no
//! dimension field (confidence 90: the arithmetic closes against all sixteen
//! shipped files). This checks that against the file the page cites:
//! `Data\Ships\Assegai\ship_alt.dat`, entry 632 of
//! `pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad`, 26,912 bytes, header `Assegai\0`
//! then the `ms` residue the page explains.

use std::path::PathBuf;

use oag_texture::ship_skin;

/// The one file `ship-skin.md` names directly.
const SKIN: &str = r"Data\Ships\Assegai\ship_alt.dat";

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn skin() -> Option<Vec<u8>> {
    let path = image()?;
    let mut archives =
        oag_pulse::open(path.to_str().expect("image path is utf-8")).expect("open as Pulse");
    Some(
        archives
            .read_name(SKIN)
            .expect("Assegai's ship_alt.dat resolves"),
    )
}

/// The size claim: every shipped skin is exactly the length the fixed layout
/// implies, with nothing left over.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn assegais_skin_is_exactly_the_disc_measured_length() {
    let Some(data) = skin() else { return };
    assert_eq!(
        data.len(),
        ship_skin::FILE_LEN,
        "ship-skin.md measured every shipped skin at 26,912 bytes"
    );
}

/// The header, the four blocks, and the residue `ship-skin.md` explains: Assegai's
/// `Assegai\0` is followed by `ms` from a reused `AG Systems\0` export buffer, so
/// it cannot tell a fixed-width field from a terminated one.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn assegais_skin_parses_into_its_display_name_and_four_blocks() {
    let Some(data) = skin() else { return };
    assert_eq!(&data[..8], b"Assegai\0", "the header the page cites");
    assert_eq!(
        &data[8..10],
        b"ms",
        "the export-buffer residue the page's investigation explains"
    );

    let parsed = ship_skin::parse(&data).expect("a real skin file must parse");
    assert_eq!(
        parsed.team_name, "Assegai",
        "the header stops at the NUL, not at the residue after it"
    );
    assert_eq!(parsed.blocks.len(), 4);
    for (index, block) in parsed.blocks.iter().enumerate() {
        let (width, height) = if index < 3 { (128, 128) } else { (64, 64) };
        assert_eq!(
            (block.width, block.height),
            (width, height),
            "block {index}"
        );
        assert_eq!(block.indices.len(), width * height, "block {index}");
    }
}

/// A palette entry decodes to a plausible colour on real data: the weakest check
/// that catches unpacking off by a nibble or byte, which would still "parse".
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_palette_entry_is_a_valid_rgba_alpha() {
    let Some(data) = skin() else { return };
    let parsed = ship_skin::parse(&data).expect("parses");
    for (index, block) in parsed.blocks.iter().enumerate() {
        for entry in block.palette {
            assert!(
                entry[3] == 0 || entry[3] == 255,
                "block {index}: alpha {} is neither fully on nor off, which every \
                 real Pulse palette entry is",
                entry[3]
            );
        }
        // Real data is not all one index; a constant block means the offsets hit padding.
        assert!(
            block.indices.iter().any(|&i| i != block.indices[0]),
            "block {index}: every index is the same value, which real ship \
             texture data is not"
        );
    }
}
