//! Scratch probe for the PS4 Omega Collection lane: does a `.rcsmaterial` (or
//! `.rcsmodel`) blob whose *layout* is unread still carry recognisable
//! `~crc32` name hashes somewhere in its bytes?
//!
//! This does not assume a field position - it scans every byte-aligned `u32`
//! window (not just 4-byte-aligned), both little- and big-endian, and reports
//! how many offsets resolve through
//! [`oag_rcs::rcsmaterial::names::sampler_name`] or
//! [`oag_rcs::rcsmaterial::names::parameter_name`] (about 180 names between
//! them). A single hit is weak on its own: with ~180 candidates checked in
//! both byte orders against every window of an N-byte file, the expected
//! hit count under uniform-random 32-bit words is `N * 360 / 2^32`, which
//! for a 40 KiB file is about 0.0013 - so 5-11 hits in one file, or the same
//! offset (e.g. `+0x724`) recurring across unrelated files, is evidence the
//! read is genuine rather than a coincidence, without this tool asserting
//! which field carries it.
//!
//! `cargo run -p oag-rcs --example ps4_hash_scan -- <path.rcsmaterial>`

use oag_rcs::rcsmaterial::name_hash;
use oag_rcs::rcsmaterial::names::{KNOWN_PARAMETER_NAMES, KNOWN_SAMPLER_NAMES};
use std::collections::HashMap;

/// A `hash -> name` lookup built once, so the sweep over a large file is
/// O(bytes) rather than O(bytes * names) - a linear scan through both name
/// lists per window is fine for one file but too slow across the hundreds
/// this lane's disc-wide sweep needs.
fn hash_table() -> HashMap<u32, &'static str> {
    KNOWN_SAMPLER_NAMES
        .iter()
        .chain(KNOWN_PARAMETER_NAMES)
        .map(|name| (name_hash(name), *name))
        .collect()
}

fn scan(
    data: &[u8],
    table: &HashMap<u32, &'static str>,
) -> (Vec<(usize, &'static str, &'static str)>, usize) {
    let mut hits = Vec::new();
    let mut windows = 0;
    for at in 0..data.len().saturating_sub(3) {
        windows += 1;
        let le = u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
        let be = u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
        if let Some(name) = table.get(&le) {
            hits.push((at, "le", *name));
        }
        if let Some(name) = table.get(&be) {
            hits.push((at, "be", *name));
        }
    }
    (hits, windows)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("usage: ps4_hash_scan <path> [--summary]")?;
    let summary = args.next().as_deref() == Some("--summary");
    let data = std::fs::read(&path)?;
    let table = hash_table();
    let (hits, windows) = scan(&data, &table);
    if summary {
        println!("{path}\t{}\t{windows}\t{}", data.len(), hits.len());
        return Ok(());
    }
    println!(
        "{path}: {} bytes, {windows} u32 windows scanned",
        data.len()
    );
    for (at, order, name) in &hits {
        println!("  +0x{at:06x} ({order})  {name}");
    }
    println!("{} hits", hits.len());
    Ok(())
}
