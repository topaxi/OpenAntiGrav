//! Wipeout 2048's `SBlk` version 5 banks: the FNV-1 name rule on every bank that
//! parses, and the eight `env_*` banks whose waveform size is stale.
//!
//! **`#[ignore]`d and never run in CI.** They need the decrypted Vita package
//! under `data/extracted/vita/`. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! The evidence page is `docs/formats/2048-audio.md`.

use std::collections::BTreeMap;

use oag_assets::psarc::Archive;
use oag_formats::sblk::Bank;
use oag_formats::sblk::cue::name_hash;

fn archive() -> Option<Archive> {
    let root = oag_testdata::exact("data/extracted/vita/PCSF00007")?;
    Some(Archive::open_file(&root.join("base/PSP2/data.psarc")).expect("the base archive opens"))
}

fn banks(archive: &mut Archive) -> Vec<(String, Vec<u8>)> {
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".bnk"))
        .cloned()
        .collect();
    paths
        .into_iter()
        .map(|p| {
            let bytes = archive.read_path(&p).expect("a listed bank reads");
            (p, bytes)
        })
        .collect()
}

/// The three banks the reader still refuses, and why: their section table
/// declares a waveform length that is not a whole number of ADPCM blocks
/// (915, 1,566 and 1,242,097 bytes). `speech_*_NGP`, a different matter from the
/// `env_*` one below and not read here.
const REFUSED: [&str; 3] = [
    "data/audio/sound/Speech_NGP.bnk",
    "data/audio/sound/speech_fe_NGP.bnk",
    "data/audio/sound/speech_zone_NGP.bnk",
];

/// The eight `env_*` banks whose section table and descriptor declare a larger
/// waveform section than the file holds.
const OVERDECLARED: [&str; 8] = [
    "env_altima",
    "env_arena",
    "env_bridge",
    "env_cathedral",
    "env_sol",
    "env_square",
    "env_subway",
    "env_tower",
];

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_v5_bank_that_parses_spells_names_that_hash_to_their_own_records() {
    let Some(mut archive) = archive() else {
        return;
    };
    let mut hashed = 0;
    let mut names = 0;
    let mut refused = Vec::new();
    for (path, bytes) in banks(&mut archive) {
        let bank = match Bank::parse(&bytes) {
            Ok(bank) => bank,
            Err(_) => {
                refused.push(path);
                continue;
            }
        };
        if !bank.is_hashed() {
            assert!(bank.hashed_names().is_empty(), "{path}");
            continue;
        }
        hashed += 1;
        for (name, cue, hash) in bank.hashed_names() {
            assert_eq!(
                name_hash(&name),
                hash,
                "{path}: {name:?} is not its record's hash"
            );
            assert_eq!(
                bank.cue_named_or_hashed(&name).map(|c| c.index),
                Some(cue).filter(|c| *c < bank.cue_count),
                "{path}: {name:?} does not look up to its own cue"
            );
            names += 1;
        }
    }
    refused.sort();
    assert_eq!(refused, REFUSED, "a bank stopped or started parsing");
    println!("{hashed} hashed banks, {names} spelled names, every one hashes to its record");
    assert!(
        hashed >= 30 && names >= 300,
        "{hashed} banks, {names} names"
    );
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn eight_env_banks_declare_more_waveform_than_they_ship_and_none_of_it_is_missing() {
    let Some(mut archive) = archive() else {
        return;
    };
    let mut seen = BTreeMap::new();
    for (path, bytes) in banks(&mut archive) {
        let Ok(bank) = Bank::parse(&bytes) else {
            continue;
        };
        let Some(declared) = bank.declared_waveform_len else {
            continue;
        };
        let stem = path
            .rsplit('/')
            .next()
            .and_then(|f| f.strip_suffix(".bnk"))
            .expect("a .bnk");
        // Every waveform the bank binds ends inside the bytes it ships, and the
        // last ends where the file does: nothing the descriptors reach is cut.
        let sounds = bank.sounds();
        let end = sounds
            .iter()
            .map(|s| u64::from(s.offset) + u64::from(s.length))
            .max()
            .expect("a bank with sounds");
        assert!(
            end <= bank.waveforms.len() as u64,
            "{path}: a waveform is cut off"
        );
        assert_eq!(
            end,
            bank.waveforms.len() as u64,
            "{path}: the pool has an unused tail"
        );
        assert!(u64::from(declared) > bank.waveforms.len() as u64, "{path}");
        seen.insert(stem.to_string(), sounds.len());
    }
    let seen: Vec<&str> = seen.keys().map(String::as_str).collect();
    assert_eq!(seen, OVERDECLARED);
}
