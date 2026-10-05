//! Which `.rcsmaterial` fragment variants take the magstrip wave: declare the
//! wave sampler (`0x85c9fd48`), the emissive sampler (`0x1202d8df`) and the
//! engine `time`. One line per material with the variant count and whether it
//! also declares `Colour` (`0x02ab9f07`).
//!
//! ```sh
//! cargo run -p oag-render --example hd_mag_params
//! ```

use oag_rcs::rcsmaterial;

const WAVE: u32 = 0x85c9_fd48;
const EMISSIVE: u32 = 0x1202_d8df;
const TIME: u32 = 0x906b_67ba;
const COLOUR: u32 = 0x02ab_9f07;
const SCALE: u32 = 0x220c_f0e6;

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    for archive in [
        "PS3_GAME/USRDIR/DATA00.PSARC",
        "PS3_GAME/USRDIR/DATA01.PSARC",
        "PS3_GAME/USRDIR/DATA02.PSARC",
        "PS3_GAME/USRDIR/DATA03.PSARC",
    ] {
        let Ok(mut open) = oag_assets::psarc::Archive::open(&format!("{image}:{archive}")) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmaterial"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(m) = rcsmaterial::RcsMaterial::parse(&blob) else {
                continue;
            };
            let (mut wave, mut both, mut colour, mut scale) = (0, 0, 0, 0);
            for v in &m.variants {
                let Some(d) = rcsmaterial::Declared::parse(&blob, v.fragment.offset) else {
                    continue;
                };
                let has = |h: u32| d.samplers.iter().any(|&(s, _)| s == h);
                if has(WAVE) {
                    wave += 1;
                    if has(EMISSIVE) && d.parameters.contains(&TIME) {
                        both += 1;
                    }
                    colour += usize::from(d.parameters.contains(&COLOUR));
                    scale += usize::from(d.parameters.contains(&SCALE));
                }
            }
            if wave > 0 {
                println!(
                    "{archive} {path}: {} variants, wave {wave}, wave+emissive+time {both}, Colour {colour}, k {scale}",
                    m.variants.len()
                );
            }
        }
    }
    Ok(())
}
