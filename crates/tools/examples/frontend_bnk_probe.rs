//! Scratch probe: locate `frontend.bnk` inside 2048's `data.psarc`, extract
//! it, and list every cue `oag_formats::sblk::Bank` decodes out of it - the
//! evidence for naming `oag_title::Music::front_end` on 2048. See
//! `docs/formats/2048-frontend.md`'s "frontend.bnk" bullet for what this
//! found: an empty name table, so `front_end` names a standalone file
//! instead.
//!
//! `cargo run -p oag-tools --example frontend_bnk_probe -- <base data.psarc> [out.bnk]`

fn main() {
    let mut args = std::env::args().skip(1);
    let psarc_path = args
        .next()
        .expect("usage: frontend_bnk_probe <base data.psarc> [out.bnk]");
    let out_path = args.next();

    let mut archive = oag_assets::psarc::Archive::open(&psarc_path)
        .unwrap_or_else(|e| panic!("open {psarc_path}: {e}"));

    println!("== paths containing \"frontend\" (case-insensitive) ==");
    let candidates: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().contains("frontend"))
        .cloned()
        .collect();
    for p in &candidates {
        println!("{p}");
    }

    let bnk_path = candidates
        .iter()
        .find(|p| p.to_ascii_lowercase().ends_with(".bnk"))
        .unwrap_or_else(|| panic!("no *.bnk path among {candidates:?}"));
    println!("\n== reading {bnk_path} ==");
    let blob = archive.read_path(bnk_path).expect("read frontend.bnk");
    println!("{} bytes", blob.len());

    if let Some(out) = &out_path {
        std::fs::write(out, &blob).expect("write out.bnk");
        println!("wrote {out}");

        for at9 in candidates.iter().filter(|p| p.ends_with(".at9")) {
            let bytes = archive.read_path(at9).expect("read .at9");
            let stem = at9.replace(['/', '\\'], "_");
            let dest = format!("{out}.{stem}");
            std::fs::write(&dest, &bytes).expect("write .at9");
            println!("wrote {dest} ({} bytes)", bytes.len());
        }
    }

    let bank = oag_formats::sblk::Bank::parse(&blob).expect("parse sblk::Bank");
    println!("\n== sound_names (name, cue index) ==");
    for name in bank.sound_names() {
        println!("{:?}", name);
    }
    println!("\n== sounds (waveform bindings) ==");
    for sound in bank.sounds() {
        println!("{:?}", sound);
    }
}
