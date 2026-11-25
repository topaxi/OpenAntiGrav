//! `psp2::build_circuit`'s `HidePads` leaves a circuit's own pad submeshes out, and only those:
//! `All` drops exactly the triangles on a pad material or a `speedup_pad`/`weapon_pad` shape,
//! `Weapon` exactly the weapon pads' share, and `None` draws the whole file.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content; run with `just test-data`.
//!
//! Detonator draws its own `Detonator_Pad` in place of both
//! (`docs/ghidra/functions/ps3-hdfury-eu/detonator-hull.md`, "Pads"); a weapons-off race drops
//! the weapon pads alone (`docs/gameplay/pickups.md`). Remove the skip in
//! `psp2::build_scanned` and every difference here reads zero.

use std::path::Path;

use oag_mesh::mesh::rcs::psp2::{self, circuit_pads::HidePads};

const OMEGA: [&str; 9] = [
    "ps4/omega-eu/uroot/data00.psarc",
    "ps4/omega-eu/uroot/data01.psarc",
    "ps4/omega-eu/uroot/data02.psarc",
    "ps4/omega-eu/uroot/data03.psarc",
    "ps4/omega-eu/uroot/data04.psarc",
    "ps4/omega-eu-patch/uroot/data05.psarc",
    "ps4/omega-eu-patch/uroot/data07.psarc",
    "ps4/omega-eu-patch/uroot/data08.psarc",
    "ps4/omega-eu-patch/uroot/data09.psarc",
];

/// `(file's triangles, pad triangles, weapon pad triangles)` per circuit, counted once by summing
/// the file's submeshes by their raw material and shape names, and written down here so the test
/// does not rely on the predicates it checks.
/// Talons Junction on Omega: 4527 `weapon_pads` triangles and 5382 on `speedup_pad_*` shapes
/// that wear the start-line material.
const TALONS: (usize, usize, usize) = (1_067_844, 9909, 4527);
/// Arena (2048's circuit on Omega): `speed_pads` 4095 and `weapon_pads` 22022.
const ARENA: (usize, usize, usize) = (3_331_753, 26_117, 22_022);
/// Altima on the Vita.
const ALTIMA: (usize, usize, usize) = (419_023, 18_389, 14_762);
/// Omega's `zone_2`: its 5382 pad triangles are speed pads drawn on `weapon_pads.rcsmaterial`,
/// and it has no weapon pad.
const ZONE_2: (usize, usize, usize) = (0, 5382, 0);

/// Builds `blob` all three ways: each hidden build loses exactly its pads and keeps every other
/// submesh of the file (the start line, the walls).
fn check(
    name: &str,
    blob: &[u8],
    (all, pads, weapon): (usize, usize, usize),
    read: &mut dyn FnMut(&str) -> Option<Vec<u8>>,
) {
    let build = |hide, read: &mut dyn FnMut(&str) -> Option<Vec<u8>>| {
        psp2::build_circuit(
            name,
            blob,
            None,
            read,
            oag_display::display::Water::Off,
            hide,
        )
        .expect("builds")
    };
    let (_, kept) = build(HidePads::None, read);
    let (_, hidden) = build(HidePads::All, read);
    let (_, unarmed) = build(HidePads::Weapon, read);
    assert_eq!(
        kept.triangles - hidden.triangles,
        pads,
        "{name}: exactly the pad triangles go"
    );
    assert_eq!(
        kept.triangles - unarmed.triangles,
        weapon,
        "{name}: weapons off drops exactly the weapon pads"
    );
    // The builder also draws what the file does not hold (Omega's crowd rigs), so a plain build
    // has at least the file's triangles and the hidden one at least the file's non-pad ones.
    if all > 0 {
        assert!(
            kept.triangles >= all,
            "{name}: a plain build draws the whole file"
        );
        assert!(
            hidden.triangles >= all - pads,
            "{name}: the rest of the circuit stays"
        );
    }
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omegas_circuits_drop_exactly_their_pad_triangles() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted");
    if !root.join("ps4").exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} has no ps4/",
            root.display()
        );
        return;
    }
    let mut archives: Vec<_> = OMEGA
        .iter()
        .filter_map(|a| oag_assets::psarc::Archive::open_file(&root.join(a)).ok())
        .collect();
    // An HD circuit (`weapon_pads` and `speedup_pad` shapes) and a 2048 circuit (`speed_pads`
    // and `weapon_pads`).
    for (path, expected) in [
        (
            "Data/environments/talons_junction/track.final.rcsmodel",
            TALONS,
        ),
        ("Data/environments2048/arena/track.final.rcsmodel", ARENA),
        ("Data/environments/zone_2/track.rcsmodel", ZONE_2),
    ] {
        let at = (0..archives.len())
            .find(|&i| archives[i].contains(path))
            .unwrap_or_else(|| panic!("{path} is in no archive"));
        let blob = archives[at].read_path(path).expect("the model");
        let mut read = |p: &str| archives.iter_mut().find_map(|a| a.read_path(p).ok());
        check(path, &blob, expected, &mut read);
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn a_2048_circuit_drops_exactly_its_pad_triangles() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        return;
    }
    let mut archive =
        oag_assets::psarc::Archive::open(path.to_str().expect("utf-8")).expect("opens");
    let blob = archive
        .read_path("data/art/published/environments/altima/track.rcsmodel")
        .expect("model");
    let mut read = |p: &str| archive.read_path(p).ok();
    check("altima", &blob, ALTIMA, &mut read);
}
