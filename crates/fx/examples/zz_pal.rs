use oag_assets::psarc::Archive;
use oag_fx::psys::{ColourScale, Effect};
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mut ar = Archive::open_file(std::path::Path::new(&a[0])).unwrap();
    let blob = ar.read_path(&a[1]).unwrap();
    let mut ar2 = Archive::open_file(std::path::Path::new(&a[0])).unwrap();
    let e = Effect::parse_with(&blob, ColourScale::Full, &mut |authored| {
        let stem = authored.rsplit(['\\', '/']).next()?.rsplit_once('.')?.0;
        let at = authored.to_ascii_lowercase().find("data\\")?;
        let entry = format!(
            "{}{stem}.gnf",
            authored[at..authored.len() - stem.len() - 4].replace('\\', "/")
        );
        println!("  loading {entry}");
        let real = ar2
            .paths()
            .iter()
            .find(|p| p.eq_ignore_ascii_case(&entry))?
            .clone();
        let b = ar2.read_path(&real).ok()?;
        let sp = oag_fx::psys::sprite::Sprite::from_gnf(&b)?;
        let (mut r, mut g, mut bl, mut al, mut n) = (0u64, 0u64, 0u64, 0u64, 0u64);
        for px in sp.rgba.chunks(4) {
            r += px[0] as u64;
            g += px[1] as u64;
            bl += px[2] as u64;
            al += px[3] as u64;
            n += 1;
        }
        println!(
            "    {}x{} mean rgba {} {} {} {}",
            sp.width,
            sp.height,
            r / n,
            g / n,
            bl / n,
            al / n
        );
        Some(sp)
    })
    .unwrap();
    for s in &e.emitters {
        if let Some(sp) = &s.sprite {
            let (mut r, mut g, mut b, mut al, mut n) = (0u64, 0u64, 0u64, 0u64, 0u64);
            for px in sp.rgba.chunks(4) {
                r += px[0] as u64;
                g += px[1] as u64;
                b += px[2] as u64;
                al += px[3] as u64;
                n += 1;
            }
            println!(
                "  sprite {}x{} mean rgba {} {} {} {}",
                sp.width,
                sp.height,
                r / n,
                g / n,
                b / n,
                al / n
            );
        }
        let al = 0;
        println!(
            "{:<20} blend {:?} mode {:?} pal0 {:?} pal128 {:?} pal250 {:?} per {:?} life {:?} size {:?} cap {} al {:?}",
            s.name,
            s.blend,
            s.colour_mode,
            s.palette[0],
            s.palette[128],
            s.palette[250],
            s.per_emission,
            s.lifetime_ticks,
            s.size.mode,
            s.live_cap,
            al
        );
    }
}
