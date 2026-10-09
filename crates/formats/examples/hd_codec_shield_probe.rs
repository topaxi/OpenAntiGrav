//! Scratch probe: dump the `~SHIELD` cue's not-PS-ADPCM waveforms from HD's
//! `weapons.bnk`, a known sound (a shield pickup/impact effect) rather than
//! an unlabelled environment ambience - a stronger identification check than
//! `hd_codec_dump_probe`'s generic census.
//!
//! ```sh
//! cargo run -q -p oag-formats --example hd_codec_shield_probe
//! ```

use std::path::Path;

use oag_formats::sblk::Bank;

const ISO: &str = "data/images/hdfury-ps3-eu-dec.iso";
const ARCHIVES: usize = 7;
const OUT_DIR: &str = concat!("data", "/scratch/hd_codec");

fn main() {
    let iso = Path::new(ISO);
    if !iso.exists() {
        println!("skipping: {ISO} not present");
        return;
    }
    std::fs::create_dir_all(OUT_DIR).expect("create out dir");

    for n in 0..ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", iso.display());
        let Ok(mut archive) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let Some(path) = archive
            .paths()
            .iter()
            .find(|p| p.ends_with("weapons.bnk"))
            .cloned()
        else {
            continue;
        };
        let Ok(blob) = archive.read_path(&path) else {
            continue;
        };
        let Ok(bank) = Bank::parse(&blob) else {
            continue;
        };
        let Some(cue) = bank.cue_named("~SHIELD") else {
            continue;
        };
        let sounds = bank.cue_sounds(&cue);
        println!("DATA0{n}:{path} ~SHIELD -> {} waveform(s)", sounds.len());
        let mut dumped = 0;
        for sound in &sounds {
            let adpcm = sound.is_adpcm();
            println!(
                "  command={} mode=0x{:04x} len={} adpcm={adpcm}",
                sound.command, sound.mode, sound.length
            );
            if adpcm {
                continue;
            }
            let Some(data) = bank.waveform(sound) else {
                continue;
            };
            let name = format!("shield_DATA0{n}_{:05}.bin", sound.command);
            std::fs::write(Path::new(OUT_DIR).join(&name), data).expect("write dump");
            dumped += 1;
        }
        println!("  dumped {dumped} not-PS-ADPCM waveform(s)");
    }
}
