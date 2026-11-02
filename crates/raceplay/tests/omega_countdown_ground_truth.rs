//! Omega's race start posts the Wwise events `_321_NGP` and `Go_NGP`, and no
//! shipped bank defines either, so there is nothing to play: the title carries
//! no start voice rather than Pulse's `ready` and `go` lent from a `speech.bnk`
//! that is a Wwise stub the `SBlk` reader refuses.
//!
//! **`#[ignore]`d and never run in CI**: it needs the decrypted Omega packages
//! under `data/extracted/ps4`. Evidence:
//! `docs/ghidra/functions/vita-2048-eu-v104/countdown-voice.md` ("Omega").

use std::path::Path;

use oag_formats::wwise::{Bank, Kind, name_hash};

#[test]
fn omega_lends_no_pulse_voice_to_its_start() {
    assert!(
        oag_omega::TITLE.race.countdown_voice.is_none(),
        "Omega's start is Wwise events; Pulse's ready/go are not its voice"
    );
}

#[test]
#[ignore = "needs the decrypted Omega packages in data/extracted/ps4/"]
fn no_omega_bank_defines_the_start_events() {
    let mut checked = 0;
    for (dir, file) in [
        ("omega-eu", "data00.psarc"),
        ("omega-eu-patch", "data08.psarc"),
    ] {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/extracted/ps4")
            .join(dir)
            .join("uroot")
            .join(file);
        if !path.exists() {
            assert!(
                std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
                "OAG_REQUIRE_GAME_DATA is set but {} is missing",
                path.display()
            );
            println!("skipping: {} not present", path.display());
            return;
        }
        let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("archive opens");
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.ends_with(".bnk"))
            .cloned()
            .collect();
        let blobs: Vec<Vec<u8>> = paths
            .iter()
            .map(|p| archive.read_path(p).expect("a listed bank reads"))
            .collect();
        let banks: Vec<Bank> = blobs
            .iter()
            .map(|b| Bank::parse(b).expect("a Wwise bank"))
            .collect();
        assert!(banks.len() >= 50, "{file}: {} banks", banks.len());
        for name in ["_321_NGP", "321_NGP", "Go_NGP", "HBEAT_ZCHANGE", "HBEAT"] {
            let id = name_hash(name);
            assert!(
                !banks
                    .iter()
                    .any(|b| b.objects_with_id(id).any(|o| o.kind == Kind::Event)),
                "{file}: an event named {name} exists"
            );
        }
        checked += 1;
    }
    assert_eq!(checked, 2);
}
