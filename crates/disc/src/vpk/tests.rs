use super::*;
use std::io::Write;

/// A ZIP written by hand: `(name, method, data, packed)`.
fn zip(entries: &[(&str, u16, &[u8], Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, method, data, packed) in entries {
        let local = out.len() as u32;
        out.extend_from_slice(&LOCAL.to_le_bytes());
        out.extend_from_slice(&[20, 0, 0, 0]);
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(packed);

        central.extend_from_slice(&CENTRAL.to_le_bytes());
        central.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
        central.extend_from_slice(&method.to_le_bytes());
        central.extend_from_slice(&[0, 0, 0, 0]);
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&(packed.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 12]);
        central.extend_from_slice(&local.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let dir_at = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(&EOCD.to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&dir_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn sfo(title: &str) -> Vec<u8> {
    let key = b"TITLE_ID\0";
    let value = format!("{title}\0");
    let mut out = b"\0PSF".to_vec();
    out.extend_from_slice(&0x101u32.to_le_bytes());
    out.extend_from_slice(&36u32.to_le_bytes());
    out.extend_from_slice(&(36 + key.len() as u32).to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0x0204u16.to_le_bytes());
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(key);
    out.extend_from_slice(value.as_bytes());
    out
}

fn deflate(data: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec(data, 6)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-vpk-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn put(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::File::create(&path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
    path
}

#[test]
fn stored_and_deflated_entries_read_by_range() {
    let dir = scratch("ranges");
    let big: Vec<u8> = (0..70_000u32).map(|i| (i * 7 % 251) as u8).collect();
    let param = sfo("PCSF00007");
    let bytes = zip(&[
        ("sce_sys/param.sfo", 0, &param, param.clone()),
        ("PSP2/data.psarc", 0, &big, big.clone()),
        ("PSP2/data1.psarc", 8, &big, deflate(&big)),
    ]);
    let path = put(&dir, "base.vpk", &bytes);
    let mut set = VpkSet::open(&path).unwrap();
    assert_eq!(set.files().len(), 3);
    assert_eq!(set.files()[1].path, "base/PSP2/data.psarc");
    for index in [1, 2] {
        assert_eq!(set.read(index, 0, 1 << 20).unwrap(), big, "whole {index}");
        assert_eq!(
            set.read(index, 66_000, 10_000).unwrap(),
            big[66_000..],
            "tail {index}"
        );
        assert_eq!(
            set.read(index, 123, 4096).unwrap(),
            big[123..123 + 4096],
            "mid {index}"
        );
        assert_eq!(set.read(index, 69_999, 5).unwrap(), big[69_999..]);
        assert!(set.read(index, 70_000, 5).unwrap().is_empty());
    }
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_patch_vpk_with_the_same_title_id_joins_and_another_title_does_not() {
    let dir = scratch("siblings");
    let param = sfo("PCSF00007");
    let other = sfo("PCSA00015");
    let a = b"base".to_vec();
    let b = b"patch".to_vec();
    let base = zip(&[
        ("sce_sys/param.sfo", 0, &param, param.clone()),
        ("PSP2/data.psarc", 0, &a, a.clone()),
    ]);
    let patch = zip(&[
        ("sce_sys/param.sfo", 0, &param, param.clone()),
        ("PSP2/data1.psarc", 0, &b, b.clone()),
    ]);
    let usa = zip(&[
        ("sce_sys/param.sfo", 0, &other, other.clone()),
        ("PSP2/data.psarc", 0, &a, a.clone()),
    ]);
    let base_path = put(&dir, "base.vpk", &base);
    put(&dir, "patch.vpk", &patch);
    put(&dir, "usa.vpk", &usa);
    let set = VpkSet::open(&base_path).unwrap();
    let names: Vec<&str> = set.files().iter().map(|f| f.path.as_str()).collect();
    assert!(names.contains(&"base/PSP2/data.psarc"));
    assert!(names.contains(&"patch/PSP2/data1.psarc"));
    assert!(!names.iter().any(|n| n.starts_with("usa/")), "{names:?}");
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn what_cannot_be_read_is_refused_by_name() {
    let dir = scratch("refuse");
    let data = b"x".to_vec();
    // Method 12 (bzip2) is not read.
    let bytes = zip(&[("a.bin", 12, &data, data.clone())]);
    let path = put(&dir, "a.vpk", &bytes);
    let mut set = VpkSet::open(&path).unwrap();
    let error = set.read(0, 0, 1).unwrap_err().to_string();
    assert!(error.contains("method 12"), "{error}");

    let not_zip = put(
        &dir,
        "b.vpk",
        b"PK\x03\x04 and then nothing like a directory",
    );
    assert!(VpkSet::open(&not_zip).is_err());
    std::fs::remove_dir_all(dir).ok();
}
