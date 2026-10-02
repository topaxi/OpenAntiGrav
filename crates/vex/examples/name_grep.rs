//! Scratch probe: which `Data.wad` entries carry a byte string.
//!
//! ```sh
//! cargo run -q -p oag-vex --example name_grep -- data/images/pulse-psp-usa.chd WO_RAIN
//! ```

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let image = args.next().ok_or("usage: IMAGE NEEDLE...")?;
    let needles: Vec<String> = args.collect();
    if let Some(index) = needles.first().and_then(|n| n.strip_prefix("dump=")) {
        let mut archives = oag_pulse::open(&image)?;
        let wad = archives.data.as_wad_mut("data")?;
        let blob = wad.read(index.parse()?)?;
        println!("{}", String::from_utf8_lossy(&blob));
        return Ok(());
    }
    let mut archives = oag_pulse::open(&image)?;
    let wad = archives.data.as_wad_mut("data")?;
    for index in 0..wad.len() as usize {
        let Ok(blob) = wad.read(index) else { continue };
        for needle in &needles {
            let n = needle.as_bytes();
            let hits: Vec<usize> = blob
                .windows(n.len())
                .enumerate()
                .filter(|(_, w)| *w == n)
                .map(|(i, _)| i)
                .take(4)
                .collect();
            if !hits.is_empty() {
                let at = hits[0];
                let ctx: String = blob[at.saturating_sub(48)..(at + 48).min(blob.len())]
                    .iter()
                    .map(|&b| {
                        if (0x20..0x7f).contains(&b) {
                            b as char
                        } else {
                            '.'
                        }
                    })
                    .collect();
                println!("entry {index} {needle} at {hits:?}: {ctx}");
            }
        }
    }
    Ok(())
}
