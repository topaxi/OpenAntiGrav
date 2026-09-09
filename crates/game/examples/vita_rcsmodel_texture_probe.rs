//! Scratch probe: three cheap measurements that decide whether Wipeout
//! 2048's texture and material binding is reachable at all this session.
//!
//! 1. **UV attribute census** - what fraction of the corpus's submeshes
//!    declare a `Uv1` attribute at all, broken down by stride. `Uv1` is only
//!    documented on the stride-28 layout; stride 20 explicitly has none.
//! 2. **Clear-text path scan** - Wipeout HD keeps its material's `.gtf` path
//!    in the clear inside the `.rcsmodel`'s own string pool
//!    (`crates/formats/src/rcsmodel/material.rs`). Does 2048's section B
//!    carry any NUL-terminated ASCII naming a `.gxt`/`.rcsmaterial`/`.gxmt`
//!    anywhere at all, corpus-wide?
//! 3. **Hash preimage search** - if no clear text exists, hash every archive
//!    path (full path, basename, stem; both cases) under a handful of
//!    plausible 64-bit schemes and look for a match against the raw 64-bit
//!    words sitting at candidate offsets inside a submesh record (the record
//!    starts at `INDEX_POINTER - 0x10 = 0`; `+0x08` is the one unclaimed
//!    64-bit-aligned slot before the index pointer at `+0x10`, and `+0x30`
//!    is the first unclaimed word after the vertex pointer at `+0x2c`).
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_rcsmodel_texture_probe
//! ```

use std::collections::{BTreeMap, HashMap, HashSet};

use oag_rcs::rcsmodel::psp2;

const PACKAGES: [&str; 3] = [
    "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

const POSITION_HASH: u32 = 0xb9d3_1b0a;
const HEADER_LEN: usize = 4;
const ATTRIBUTE_LEN: usize = 8;
const MAX_ATTRIBUTES: usize = 16;
const UV1_HASH: u32 = 0x4272_14fc;

struct Attr {
    name_hash: u32,
    offset: u8,
}

struct Decl {
    stride: usize,
    attributes: Vec<Attr>,
}

fn decode_declaration(bytes: &[u8], position_at: usize) -> Option<Decl> {
    let header_at = position_at.checked_sub(HEADER_LEN)?;
    let count = usize::from(*bytes.get(header_at)?);
    let stride = usize::from(*bytes.get(header_at + 1)?);
    if count == 0 || count > MAX_ATTRIBUTES || stride == 0 {
        return None;
    }
    if header_at + HEADER_LEN + count * ATTRIBUTE_LEN > bytes.len() {
        return None;
    }
    let mut attributes = Vec::with_capacity(count);
    for k in 0..count {
        let base = header_at + HEADER_LEN + k * ATTRIBUTE_LEN;
        let record_stride = u16::from_be_bytes([bytes[base + 4], bytes[base + 5]]) as usize;
        if record_stride != stride {
            return None;
        }
        attributes.push(Attr {
            name_hash: u32::from_le_bytes(bytes[base..base + 4].try_into().unwrap()),
            offset: bytes[base + 7],
        });
    }
    Some(Decl { stride, attributes })
}

fn find_uv1_offset_by_stride(blob: &[u8], section: psp2::Section) -> HashMap<usize, u8> {
    let bytes = &blob[section.at..section.at + section.len];
    let anchor = POSITION_HASH.to_le_bytes();
    let mut out = HashMap::new();
    for (i, _) in bytes.windows(4).enumerate().filter(|(_, w)| *w == anchor) {
        if let Some(decl) = decode_declaration(bytes, i) {
            for a in &decl.attributes {
                if a.name_hash == UV1_HASH {
                    out.entry(decl.stride).or_insert(a.offset);
                }
            }
        }
    }
    out
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

// CRC-64/XZ (ECMA-182 polynomial, reflected) - the commonest "crc64" in the wild.
fn crc64_table() -> [u64; 256] {
    let poly: u64 = 0xC96C_5795_D787_0F42;
    let mut table = [0u64; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u64;
        let mut j = 0;
        while j < 8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ poly
            } else {
                crc >> 1
            };
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

fn crc64(table: &[u64; 256], bytes: &[u8]) -> u64 {
    let mut crc = !0u64;
    for &b in bytes {
        let idx = ((crc ^ u64::from(b)) & 0xff) as usize;
        crc = table[idx] ^ (crc >> 8);
    }
    !crc
}

// Two packed `~crc32`s, the way HD's own attribute-name hash already works,
// just widened: high 32 = crc32(path), low 32 = crc32(basename) or similar.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xffff_ffff;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn candidates_for(path: &str) -> Vec<(String, u64)> {
    let lower = path.to_ascii_lowercase();
    let basename = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let basename_lower = basename.to_ascii_lowercase();
    let stem = basename.rsplit_once('.').map_or(basename, |(s, _)| s);
    let stem_lower = stem.to_ascii_lowercase();

    let table = crc64_table();
    let mut out = Vec::new();
    for (label, s) in [
        ("path", path.to_string()),
        ("path_lower", lower.clone()),
        ("basename", basename.to_string()),
        ("basename_lower", basename_lower.clone()),
        ("stem", stem.to_string()),
        ("stem_lower", stem_lower.clone()),
    ] {
        let bytes = s.as_bytes();
        out.push((format!("fnv1a64({label})"), fnv1a64(bytes)));
        out.push((format!("crc64xz({label})"), crc64(&table, bytes)));
        let packed = (u64::from(crc32(bytes)) << 32) | u64::from(crc32(bytes));
        out.push((format!("crc32^2({label})"), packed));
    }
    // path+basename cross pair, the two-crc32 shape the vertex declaration's
    // own name hashing already uses elsewhere in this format.
    let packed_cross =
        (u64::from(crc32(path.as_bytes())) << 32) | u64::from(crc32(basename.as_bytes()));
    out.push(("crc32(path)<<32|crc32(basename)".to_string(), packed_cross));
    out
}

fn ascii_string_scan(blob: &[u8], needle: &str) -> usize {
    let needle = needle.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i + needle.len() <= blob.len() {
        if &blob[i..i + needle.len()] == needle {
            count += 1;
        }
        i += 1;
    }
    count
}

fn u64_le(bytes: &[u8], at: usize) -> Option<u64> {
    bytes
        .get(at..at + 8)
        .map(|b| u64::from_le_bytes(b.try_into().unwrap()))
}

fn main() -> anyhow::Result<()> {
    let mut all_paths: Vec<String> = Vec::new();
    let mut stride_total: BTreeMap<usize, usize> = BTreeMap::new();
    let mut stride_with_uv1: BTreeMap<usize, usize> = BTreeMap::new();
    let mut gxt_hits = 0usize;
    let mut rcsmaterial_hits = 0usize;
    let mut gxmt_hits = 0usize;
    let mut files_scanned = 0usize;

    // Sample of raw candidate hash words, from a handful of submeshes, kept
    // for the preimage search and for eyeballing.
    let mut samples: Vec<(String, usize, u64, u64)> = Vec::new(); // (entry, record, at+0x08, at+0x30)

    for package in PACKAGES {
        let Ok(mut archive) = oag_assets::psarc::Archive::open(package) else {
            println!("{package}: unavailable");
            continue;
        };
        all_paths.extend(archive.paths().iter().cloned());

        let mut entries: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
            .cloned()
            .collect();
        entries.sort();

        for entry in entries {
            let Ok(blob) = archive.read_path(&entry) else {
                continue;
            };
            let Ok(model) = psp2::parse(&blob) else {
                continue;
            };
            files_scanned += 1;

            if let Some(&gpu) = model.sections.get(1) {
                let cpu = model.sections[0];
                let uv1_by_stride = find_uv1_offset_by_stride(&blob, cpu);
                for submesh in &model.submeshes {
                    *stride_total.entry(submesh.stride).or_insert(0) += 1;
                    if uv1_by_stride.contains_key(&submesh.stride) {
                        *stride_with_uv1.entry(submesh.stride).or_insert(0) += 1;
                    }
                }

                // Clear-text scan, whole file (cheap: at most ~17 MiB).
                gxt_hits += ascii_string_scan(&blob, ".gxt");
                rcsmaterial_hits += ascii_string_scan(&blob, ".rcsmaterial");
                gxmt_hits += ascii_string_scan(&blob, ".gxmt");

                if samples.len() < 40 {
                    for submesh in model.submeshes.iter().take(4) {
                        let base = cpu.at + submesh.record;
                        let a = u64_le(&blob, base + 0x08).unwrap_or(0);
                        let b = u64_le(&blob, base + 0x30).unwrap_or(0);
                        samples.push((entry.clone(), submesh.record, a, b));
                        if samples.len() >= 40 {
                            break;
                        }
                    }
                }
                let _ = gpu;
            }
        }
    }

    println!("=== 1. UV1 attribute census ({files_scanned} .rcsmodel files) ===");
    let total_submeshes: usize = stride_total.values().sum();
    let total_with_uv1: usize = stride_with_uv1.values().sum();
    for (stride, count) in &stride_total {
        let with_uv1 = stride_with_uv1.get(stride).copied().unwrap_or(0);
        println!(
            "  stride {stride:3}: {count:7} submesh(es), {with_uv1:7} with a declared Uv1 ({:.1}%)",
            100.0 * with_uv1 as f64 / *count as f64
        );
    }
    println!(
        "  TOTAL: {total_with_uv1} / {total_submeshes} submeshes ({:.1}%) have a declared Uv1 attribute",
        100.0 * total_with_uv1 as f64 / total_submeshes.max(1) as f64
    );

    println!("=== 2. Clear-text path scan ===");
    println!("  \".gxt\" occurrences:         {gxt_hits}");
    println!("  \".rcsmaterial\" occurrences: {rcsmaterial_hits}");
    println!("  \".gxmt\" occurrences:        {gxmt_hits}");

    println!("=== 3. Hash preimage search ===");
    println!("  candidate paths: {}", all_paths.len());
    let mut preimages: HashMap<u64, Vec<String>> = HashMap::new();
    for path in &all_paths {
        for (label, hash) in candidates_for(path) {
            preimages
                .entry(hash)
                .or_default()
                .push(format!("{path} [{label}]"));
        }
    }
    let sample_hashes: HashSet<u64> = samples
        .iter()
        .flat_map(|(_, _, a, b)| [*a, *b])
        .filter(|&h| h != 0)
        .collect();
    println!(
        "  distinct nonzero sample hash words: {}",
        sample_hashes.len()
    );
    let mut found = 0usize;
    for hash in &sample_hashes {
        if let Some(paths) = preimages.get(hash) {
            found += 1;
            println!("  MATCH {hash:#018x}: {}", paths.join(", "));
        }
    }
    println!(
        "  {found} of {} sample words matched a candidate hash",
        sample_hashes.len()
    );

    println!("=== raw sample words (entry, record, +0x08, +0x30) ===");
    for (entry, record, a, b) in samples.iter().take(20) {
        println!("  {entry} @{record:#x}: +0x08={a:#018x} +0x30={b:#018x}");
    }

    Ok(())
}
