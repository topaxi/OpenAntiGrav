//! Which batches Pulse's caustic pass redraws: `Batch::draws_in_caustic_pass`
//! (header byte 3 bit `0x04`) over every `.vex` of both Pulse discs, and the
//! byte the pass scales its tint by (`mesh_caustic_scale`).
//!
//! `docs/ghidra/functions/psp-pulse-usa/caustic-pass.md`, "Answer": the PSP
//! disc has no such batch, and the PS2 disc has them on Vostok Reef alone. Pure's version-4 files
//! are read by their own mesh class (`0x11e`): Vineta K alone flags batches there.

use std::collections::BTreeMap;

use oag_disc::DiscImage;
use oag_vex::vex;

use crate::wad_walk;

/// A census: per directory index `(flagged, total)`, and the tint bytes.
type Census = (BTreeMap<usize, (usize, usize)>, Vec<u8>);

/// `(flagged, total)` batches of every `.vex` blob of `archive` that has any
/// flagged, by directory index, and the tint byte of each flagged mesh.
fn census(image: &str, archive: &str) -> Option<Census> {
    let path = oag_testdata::image(&format!("data/images/{image}"))?;
    let mut disc = DiscImage::open(&path).expect("image opens");
    let mut found = BTreeMap::new();
    let mut scales = Vec::new();
    for (index, bytes) in wad_walk::blobs(&mut disc, archive, wad_walk::VEX_HEAD, vex::has_magic) {
        let Ok(nodes) = vex::nodes(&bytes) else {
            continue;
        };
        let (mut flagged, mut total) = (0, 0);
        let mesh_class = vex::version(&bytes)
            .ok()
            .and_then(vex::classes::for_version)
            .and_then(|c| c.mesh)
            .unwrap_or(vex::CLASS_MESH);
        for node in nodes.iter().filter(|n| n.class_id == mesh_class) {
            let range = node.payload();
            if range.end > bytes.len() {
                continue;
            }
            let payload = &bytes[range];
            let mut mesh_flagged = false;
            for list in 0..2u8 {
                let Ok(batches) = vex::mesh_batches(payload, list) else {
                    continue;
                };
                for batch in &batches {
                    total += 1;
                    if batch.draws_in_caustic_pass() {
                        flagged += 1;
                        mesh_flagged = true;
                    }
                }
            }
            if mesh_flagged {
                scales.extend(vex::mesh_caustic_scale(payload));
            }
        }
        if flagged > 0 {
            found.insert(index, (flagged, total));
        }
    }
    Some((found, scales))
}

#[test]
#[ignore = "needs the PS2 EU disc image"]
fn only_vostok_reefs_three_ps2_files_carry_the_caustic_bit() {
    let Some((found, scales)) = census("pulse-ps2-eu.chd", "54748/WADS2.WAD") else {
        return;
    };
    let counts: Vec<(usize, usize)> = found.values().copied().collect();
    assert_eq!(
        counts,
        [(257, 1418), (257, 1447), (205, 1204)],
        "track.vex, the unresolved entry 1386, track_reversed.vex"
    );
    assert!(!scales.is_empty());
    println!("caustic tint bytes per flagged mesh: {scales:?}");
}

#[test]
#[ignore = "needs the PSP EU disc image"]
fn no_psp_batch_carries_the_caustic_bit() {
    let Some((found, _)) = census("pulse-psp-eu.chd", "PSP_GAME/USRDIR/Data.wad") else {
        return;
    };
    assert!(found.is_empty(), "{found:?}");
}

#[test]
#[ignore = "needs the Pure PSP EU disc image"]
fn only_vineta_k_carries_the_caustic_bit_on_pure() {
    let Some((found, scales)) = census("pure-psp-eu.chd", "PSP_GAME/USRDIR/Data.wad") else {
        return;
    };
    let counts: Vec<(usize, usize)> = found.values().copied().collect();
    assert_eq!(
        found.keys().copied().collect::<Vec<_>>(),
        [833],
        "Vineta K's track.vex alone, {found:?}"
    );
    assert_eq!(counts, [(352, 1335)]);
    assert!(scales.iter().any(|&b| b != 0), "the tint byte is authored");
}
