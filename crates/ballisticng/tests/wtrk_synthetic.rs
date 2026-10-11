//! The `.wtrk` wrapper on bytes built here: the layout parses, and every
//! truncation or flipped bit is a named refusal, never a panic.

use oag_ballisticng::wtrk::{Wtrk, csf_bundle};

fn dotnet(s: &str) -> Vec<u8> {
    let mut v = vec![s.len() as u8];
    v.extend_from_slice(s.as_bytes());
    v
}

fn entry(payload: &[u8]) -> Vec<u8> {
    let mut v = (payload.len() as u32).to_le_bytes().to_vec();
    v.extend_from_slice(payload);
    v
}

fn build(bundle: &[u8]) -> Vec<u8> {
    let names = [
        "WorkshopID",
        "WorkshopTitle",
        "WorkshopDescription",
        "WorkshopVisibility",
        "WorkshopTags",
        "WorkshopPreview",
        "TrackFile",
    ];
    let mut tags = 2u32.to_le_bytes().to_vec();
    tags.extend(dotnet("Track"));
    tags.extend(dotnet("Original"));
    let mut track = (bundle.len() as u32).to_le_bytes().to_vec();
    track.extend_from_slice(bundle);
    let values = [
        entry(&42u64.to_le_bytes()),
        entry(&dotnet("A Track")),
        entry(&dotnet("About it")),
        entry(&0u64.to_le_bytes()),
        entry(&tags),
        entry(&dotnet("E:/x.jpg")),
        entry(&track),
    ];
    let mut head = vec![26u8];
    head.extend_from_slice(b"Workshop Track (Version 1)");
    head.extend_from_slice(&7u32.to_le_bytes());
    let table: usize = names.iter().map(|n| 1 + n.len() + 8).sum();
    let mut at = (head.len() + table) as u64;
    for (n, v) in names.iter().zip(&values) {
        head.push(n.len() as u8);
        head.extend_from_slice(n.as_bytes());
        head.extend_from_slice(&at.to_le_bytes());
        at += v.len() as u64;
    }
    for v in values {
        head.extend(v);
    }
    head
}

#[test]
fn the_layout_parses() {
    let file = build(b"UnityFS\0rest");
    let w = Wtrk::parse(&file).expect("parses");
    assert_eq!(w.workshop_id, 42);
    assert_eq!(w.title, "A Track");
    assert_eq!(w.description, "About it");
    assert_eq!(w.tags, ["Track", "Original"]);
    assert!(w.has_tag("Original") && !w.has_tag("Port/Remake"));
    assert_eq!(w.preview, "E:/x.jpg");
    assert_eq!(w.bundle(&file), b"UnityFS\0rest");
}

#[test]
fn a_bundle_without_the_signature_is_refused() {
    let file = build(b"NotUnity");
    assert!(Wtrk::parse(&file).is_err());
}

#[test]
fn every_truncation_and_bit_flip_is_a_result_not_a_panic() {
    let file = build(b"UnityFS\0rest");
    for n in 0..file.len() {
        assert!(Wtrk::parse(&file[..n]).is_err(), "prefix {n}");
    }
    let mut refused = 0;
    for bit in 0..file.len() * 8 {
        let mut f = file.clone();
        f[bit / 8] ^= 1 << (bit % 8);
        if Wtrk::parse(&f).is_err() {
            refused += 1;
        }
    }
    // Flips inside text values and the bundle payload still parse.
    assert!(refused > file.len());
}

#[test]
fn a_csf_is_a_bare_bundle() {
    assert!(csf_bundle(b"UnityFS\0x").is_ok());
    assert!(csf_bundle(b"Workshop Track").is_err());
    assert!(csf_bundle(b"").is_err());
}
