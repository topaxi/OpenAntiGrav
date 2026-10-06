//! Scratch probe: for every Omega `.gnf` that has an HD `.gtf` of the same stem
//! and size, are the decoded rows the same as HD's as stored, or reversed?
//!
//! ```sh
//! HD_ISO=data/images/hdfury-ps3-eu-dec.iso cargo run -p oag-game --example omega_orient_census -- <a.psarc>...
//! ```
fn main() -> anyhow::Result<()> {
    let hd = std::env::var("HD_ISO")?;
    let mut archives = oag_assets::Archives::open(&hd, oag_hd::TITLE)?;
    for path in std::env::args().skip(1) {
        let mut archive = oag_assets::psarc::Archive::open_file(std::path::Path::new(&path))?;
        let names: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| {
                p.to_ascii_lowercase().ends_with(".gnf") && p.to_ascii_lowercase().contains("/fe/")
            })
            .cloned()
            .collect();
        for name in names {
            let Ok(bytes) = archive.read_path(&name) else {
                continue;
            };
            let Ok(tex) = oag_texture::gnf::Texture::parse(&bytes) else {
                continue;
            };
            let Ok(px) = tex.decode(&bytes) else { continue };
            let stem = name.rsplit('/').next().unwrap().trim_end_matches(".gnf");
            let hd_name = format!(r"Data\FE\Images\{stem}.gtf");
            let Ok(hb) = archives.read_name(&hd_name) else {
                println!("NOHD {name}");
                continue;
            };
            let Ok(g) = oag_texture::gtf::Gtf::parse(&hb) else {
                continue;
            };
            let Some(t) = g.only() else { continue };
            let Ok(hp) = t.to_rgba(&hb) else { continue };
            if t.width as u32 != tex.width || t.height as u32 != tex.height {
                println!(
                    "SIZE {name} omega {}x{} hd {}x{}",
                    tex.width, tex.height, t.width, t.height
                );
                continue;
            }
            let w = tex.width as usize;
            let h = tex.height as usize;
            let d = |flip: bool| -> usize {
                let mut n = 0;
                for y in 0..h {
                    let hy = if flip { h - 1 - y } else { y };
                    for x in 0..w {
                        let (a, b) = (px[y * w + x], hp[hy * w + x]);
                        if (0..4).any(|c| a[c].abs_diff(b[c]) > 24) {
                            n += 1;
                        }
                    }
                }
                n
            };
            let (same, flip) = (d(false), d(true));
            let verdict = if same * 20 < w * h && flip > same {
                "AS-STORED"
            } else if flip * 20 < w * h && same > flip {
                "FLIPPED"
            } else {
                "NEITHER"
            };
            println!("{verdict} {name} {w}x{h} same-diff {same} flip-diff {flip}");
        }
    }
    Ok(())
}
