//! Damaged real bundles: one LZ4HC ship and one LZMA track, with bytes flipped
//! and the file cut short, must come back as results and never panic or hang.

use crate::common;

use oag_unity::assets::Assets;
use oag_unity::bundle::Bundle;

fn damage(bytes: &[u8], flips: u32, seed: u32) -> (usize, usize) {
    let (mut opened, mut refused) = (0, 0);
    let mut x = seed;
    for i in 0..flips {
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        let mut b = bytes.to_vec();
        let at = x as usize % b.len();
        b[at] ^= 1 << (x >> 29);
        if i % 16 == 0 {
            b.truncate(x as usize % bytes.len());
        }
        match Bundle::parse(&b) {
            Ok(bundle) => {
                if let Ok(a) = Assets::open(&bundle) {
                    for sf in a.files() {
                        for o in &sf.objects {
                            let _ = sf.read_object(o);
                        }
                    }
                }
                opened += 1;
            }
            Err(_) => refused += 1,
        }
    }
    (opened, refused)
}

#[test]
#[ignore = "needs data/unity"]
fn damaged_lz4hc_ship() {
    let Some(root) = common::workshop() else {
        return;
    };
    let item = common::items(&root)
        .into_iter()
        .find(|i| i.id == "2050936018")
        .unwrap();
    let bytes = common::bundle_bytes(&item.path).unwrap();
    let (opened, refused) = damage(&bytes, 1500, 0x9e37_79b9);
    assert_eq!(opened + refused, 1500);
}

#[test]
#[ignore = "needs data/unity"]
fn damaged_lzma_track() {
    let Some(root) = common::workshop() else {
        return;
    };
    let item = common::items(&root)
        .into_iter()
        .find(|i| i.id == "1392209921")
        .unwrap();
    let bytes = common::bundle_bytes(&item.path).unwrap();
    let (opened, refused) = damage(&bytes, 60, 0x1234_5679);
    assert_eq!(opened + refused, 60);
}
