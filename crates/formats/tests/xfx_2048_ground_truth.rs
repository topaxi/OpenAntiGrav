//! Wipeout 2048's `xFShip_*.xfx` crossfade tables, every one in the package.
//!
//! **`#[ignore]`d and never run in CI.** They need the decrypted Vita package
//! under `data/extracted/vita/`. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The evidence page is `docs/formats/2048-xfx.md`. The headline: the HD reader
//! accounts for every byte of all 23 files once it knows the Vita's byte order
//! (little-endian, version word `0x00060000`); they carry five channels where
//! HD's carry four; and the five `<team>2048` tables and Zone's name their
//! layers by **cue index**, not by name.

use std::collections::BTreeSet;

use oag_assets::psarc::Archive;
use oag_formats::byte_order::ByteOrder;
use oag_formats::sblk::{self, Bank};
use oag_formats::xfx::Xfx;

/// The 23 tables, as the archive spells them (`xfShip_` and `xfship_` both).
const FILES: [&str; 23] = [
    "xfShip_ag_systems.xfx",
    "xfShip_ag_systems2048.xfx",
    "xfShip_assegai.xfx",
    "xfShip_auricom.xfx",
    "xfShip_auricom2048.xfx",
    "xfShip_det.xfx",
    "xfShip_egx.xfx",
    "xfShip_feisar.xfx",
    "xfShip_feisar2048.xfx",
    "xfShip_goteki.xfx",
    "xfShip_harimau.xfx",
    "xfShip_icaras.xfx",
    "xfShip_mirage.xfx",
    "xfShip_piranha.xfx",
    "xfShip_piranha2048.xfx",
    "xfShip_qirex.xfx",
    "xfShip_qirex2048.xfx",
    "xfShip_triakis.xfx",
    "xfship_ZONE_ag_systems2048.xfx",
    "xfship_ZONE_auricom2048.xfx",
    "xfship_ZONE_feisar2048.xfx",
    "xfship_ZONE_piranha2048.xfx",
    "xfship_ZONE_qirex2048.xfx",
];

const DIR: &str = "data/audio/sound/";

fn archive() -> Option<Archive> {
    let root = oag_testdata::exact("data/extracted/vita/PCSF00007")?;
    Some(Archive::open_file(&root.join("base/PSP2/data.psarc")).expect("the base archive opens"))
}

fn table(archive: &mut Archive, file: &str) -> Vec<u8> {
    archive
        .read_path(&format!("{DIR}{file}"))
        .unwrap_or_else(|e| panic!("{file}: {e}"))
}

fn bank_bytes(archive: &mut Archive, name: &str) -> Vec<u8> {
    archive
        .read_path(&format!("{DIR}{name}.bnk"))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn the_package_ships_exactly_these_tables_and_each_accounts_for_every_byte() {
    let Some(mut archive) = archive() else {
        return;
    };
    let on_disc: BTreeSet<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".xfx"))
        .map(|p| p.rsplit('/').next().unwrap().to_string())
        .collect();
    let expected: BTreeSet<String> = FILES.iter().map(|f| (*f).to_string()).collect();
    assert_eq!(on_disc, expected, "a table was added or is missing");
    for file in FILES {
        let bytes = table(&mut archive, file);
        let xfx = Xfx::parse(&bytes).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(xfx.byte_order(), ByteOrder::Little, "{file}");
        assert_eq!(xfx.accounted_bytes(), bytes.len(), "{file}");
        assert!(xfx.coverage().gaps(1).is_empty(), "{file}: a hole");
        assert_eq!(xfx.channels().len(), 5, "{file}");
    }
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn only_the_authored_triggers_exist_and_none_names_a_cue() {
    let Some(mut archive) = archive() else {
        return;
    };
    for file in FILES {
        let bytes = table(&mut archive, file);
        let xfx = Xfx::parse(&bytes).unwrap();
        for (n, channel) in xfx.channels().iter().enumerate() {
            let expected =
                usize::from(n == 0 && !file.contains("2048") || file.contains("ZONE") && n == 0)
                    * 26;
            assert_eq!(channel.trigger_count(), expected, "{file} channel {n}");
            for trigger in channel.triggers() {
                // The byte the game tests for a name and the cue index it falls
                // back to are both empty, so a trigger starts no voice.
                assert_eq!(trigger[0x2b], 0, "{file}");
                assert!(ByteOrder::Little.i16(trigger, 0x3c) < 0, "{file}");
            }
        }
    }
}

/// Which bank each family of tables addresses, and what it must bind there.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn every_layer_binds_a_looping_decodable_waveform_in_the_bank_its_family_reads() {
    let Some(mut archive) = archive() else {
        return;
    };
    let normal = bank_bytes(&mut archive, "Ship_NGP");
    let zone = bank_bytes(&mut archive, "Ship_NGP_Zone");
    let normal = Bank::parse(&normal).expect("Ship_NGP parses, tail slack and all");
    let zone = Bank::parse(&zone).expect("Ship_NGP_Zone parses");
    let mut checked = 0;
    for file in FILES {
        let is_zone = file.contains("ZONE");
        let is_2048 = file.contains("2048");
        if !is_2048 {
            continue;
        }
        let bank = if is_zone { &zone } else { &normal };
        let bytes = table(&mut archive, file);
        let xfx = Xfx::parse(&bytes).unwrap();
        for (n, layer) in xfx.layers().iter().enumerate() {
            if layer.kind() == 2 {
                // The modulator: no sound of its own, a link to layer 0.
                assert_eq!(layer.link(), 0, "{file} layer {n}");
                continue;
            }
            if layer.kind() == 1 {
                // A streamed layer: the one in the package is Zone's Auricom
                // table, layer 2, and the original starts it through a stream
                // callback that this port has no counterpart for.
                assert_eq!((file, n), ("xfship_ZONE_auricom2048.xfx", 2));
                continue;
            }
            assert_eq!(layer.kind(), 0, "{file} layer {n}");
            assert_eq!(layer.name(), "", "{file}: the 2048 tables name nothing");
            let cue = bank.cue(layer.cue_index()).unwrap_or_else(|| {
                panic!(
                    "{file} layer {n}: cue {} not in the bank",
                    layer.cue_index()
                )
            });
            let sounds = bank.cue_tree_sounds(&cue);
            assert!(!sounds.is_empty(), "{file} layer {n} binds nothing");
            for sound in &sounds {
                // Both codecs the SBlk reader decodes: PS-ADPCM, and the 16-bit
                // PCM Zone's Piranha third layer uses.
                let data = bank.waveform(sound).expect("the span is in the section");
                let pcm = if sound.is_adpcm() {
                    sblk::decode_adpcm(sblk::adpcm_played(data))
                } else {
                    sblk::decode_pcm16(data)
                };
                assert!(!pcm.is_empty(), "{file} layer {n}: decodes to nothing");
                assert!(sound.is_looping(), "{file} layer {n}: not a loop");
            }
            checked += 1;
        }
    }
    // Five teams of three sounding layers, and Zone's five of three or four.
    assert!(checked >= 15 + 13, "only {checked} sounding layers checked");
}

#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_layer_name_resolves_by_hash_in_the_zone_bank_to_the_cue_hd_era_tables_expect() {
    let Some(mut archive) = archive() else {
        return;
    };
    let zone = bank_bytes(&mut archive, "Ship_NGP_Zone");
    let zone = Bank::parse(&zone).unwrap();
    assert!(zone.is_hashed());
    let mut named = 0;
    for file in FILES.iter().filter(|f| !f.contains("2048")) {
        let bytes = table(&mut archive, file);
        for layer in Xfx::parse(&bytes).unwrap().layers() {
            if layer.name().is_empty() {
                continue;
            }
            assert!(
                zone.cue_named(layer.name()).is_some(),
                "{file}: {:?} does not resolve",
                layer.name()
            );
            named += 1;
        }
    }
    assert!(named > 50, "{named} named layers");
    // And in the normal bank they do not: this is why HD-era hulls are silent
    // in an ordinary 2048 race.
    let normal = bank_bytes(&mut archive, "Ship_NGP");
    let normal = Bank::parse(&normal).unwrap();
    assert!(normal.cue_named("~jet01_02").is_none());
    assert!(normal.cue_named("~afterburner").is_none());
}
