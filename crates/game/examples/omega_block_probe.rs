//! Scratch probe: dumps Omega's menu-block textures (`file2`, `cursor`,
//! `HD_options_arrow`) as upscaled PNGs so they can be compared with HD's.
//!
//! ```sh
//! cargo run -p oag-game --example omega_block_probe -- <out dir> <a.psarc>...
//! ```
fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let out = args.next().expect("out dir");
    if let Ok(hd) = std::env::var("HD_ISO") {
        let mut archives = oag_assets::Archives::open(&hd, oag_hd::TITLE)?;
        let blob = archives.read_name(r"Data\FE\Images\file2.gtf")?;
        let gtf = oag_texture::gtf::Gtf::parse(&blob)?;
        let tex = gtf.only().expect("one texture");
        let px = tex.to_rgba(&blob)?;
        let mut rows = String::new();
        for y in [0usize, 5, 58, 63] {
            let r: Vec<u8> = (0..64).map(|x| px[y * 64 + x][3]).collect();
            rows += &format!("HD file row {y}: {:?}\n", &r[..12]);
        }
        println!("HD file2: {}x{}\n{rows}", tex.width, tex.height);
        std::fs::write(
            format!("{out}/hd-file2-raw-alpha.bin"),
            px.iter().map(|p| p[3]).collect::<Vec<u8>>(),
        )?;
    }
    for path in args {
        let mut archive = oag_assets::psarc::Archive::open_file(std::path::Path::new(&path))?;
        let hits: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| {
                let l = p.to_ascii_lowercase();
                l.ends_with("/file2.gnf")
                    || l.ends_with("/cursor.gnf")
                    || l.ends_with("/hd_options_arrow.gnf")
            })
            .cloned()
            .collect();
        for name in hits {
            let bytes = archive.read_path(&name)?;
            let tex = oag_texture::gnf::Texture::parse(&bytes)?;
            let px = tex.decode(&bytes)?;
            println!(
                "{path}: {name}: {}x{} ({} bytes)",
                tex.width,
                tex.height,
                bytes.len()
            );
            let scale = if tex.width <= 64 { 8 } else { 4 };
            let (w, h) = (tex.width * scale, tex.height * scale);
            let mut rgba = Vec::new();
            for y in 0..h {
                for x in 0..w {
                    let p = px[((y / scale) * tex.width + x / scale) as usize];
                    // Show alpha over a checker so a translucent fill is visible.
                    let c: u8 = if (x / 16 + y / 16) % 2 == 0 { 90 } else { 160 };
                    let a = f32::from(p[3]) / 255.0;
                    let m = |v: u8| (f32::from(v) * a + f32::from(c) * (1.0 - a)) as u8;
                    rgba.extend_from_slice(&[m(p[0]), m(p[1]), m(p[2]), 255]);
                }
            }
            let leaf = name.rsplit('/').next().unwrap();
            let stem = std::path::Path::new(&path)
                .file_stem()
                .unwrap()
                .to_string_lossy();
            std::fs::write(
                format!("{out}/{stem}-{leaf}.png"),
                oag_texture::png::encode_rgba(w, h, &rgba),
            )?;
            let a: Vec<u8> = px.iter().map(|p| p[3]).collect();
            std::fs::write(
                format!(
                    "{out}/{}-{}-alpha.bin",
                    std::path::Path::new(&path)
                        .file_stem()
                        .unwrap()
                        .to_string_lossy(),
                    leaf
                ),
                &a,
            )?;
            println!(
                "  alpha min {} max {}; texel (4,5) = {:?}, (2,5) = {:?}",
                a.iter().min().unwrap(),
                a.iter().max().unwrap(),
                px[(5 * tex.width + 4) as usize],
                px[(5 * tex.width + 2) as usize]
            );
        }
    }
    Ok(())
}
