//! Probe: list the cues and waveform bindings of a title's `frontend.bnk`.
//!
//! `cargo run -p oag-game --example hd_frontend_bnk_probe -- <image> <entry name>`

fn main() {
    let mut args = std::env::args().skip(1);
    let image = args.next().expect("image");
    let entry = args
        .next()
        .unwrap_or_else(|| r"Data\Sound\frontend.bnk".into());
    let mut opened = oag_source::title::open_source(&image, Vec::new(), Vec::new()).expect("open");
    let blob = opened.archives.read_name(&entry).expect("read");
    println!("{} bytes", blob.len());
    let bank = oag_formats::sblk::Bank::parse(&blob).expect("parse");
    println!(
        "name {:?} cues {} waveforms {}",
        bank.name, bank.cue_count, bank.waveform_count
    );
    let mut names = bank.sound_names();
    names.sort_by(|a, b| a.name.cmp(&b.name));
    for n in names {
        let cue = bank.cue(n.cue).unwrap();
        let w = bank.cue_sounds(&cue);
        let rates: Vec<u32> = w.iter().map(|s| s.sample_rate()).collect();
        let lens: Vec<u32> = w.iter().map(|s| s.length).collect();
        println!(
            "{:<24} cue {:>3} waveforms {} rates {:?} len {:?}",
            n.name,
            n.cue,
            w.len(),
            rates,
            lens
        );
    }

    for want in args_names() {
        let Some(cue) = bank.cue_named(&want) else {
            continue;
        };
        for k in cue.range() {
            let w = &bank.commands[k * 8..k * 8 + 8];
            println!(
                "  cmd {k}: {:02x}{:02x}{:02x}{:02x} {:02x}{:02x}{:02x}{:02x}",
                w[0], w[1], w[2], w[3], w[4], w[5], w[6], w[7]
            );
        }
        let model = oag_formats::sblk::timeline::WalkModel {
            goto_markers: true,
            hd_alternates: true,
            ..Default::default()
        };
        match bank.cue_timelines_modelled(&cue, model) {
            None => println!("{want}: no timelines"),
            Some(all) => {
                println!("{want}: {} timelines", all.len());
                for (i, t) in all.iter().enumerate().take(24) {
                    let g: Vec<String> = t
                        .grains
                        .iter()
                        .map(|g| {
                            format!(
                                "t{} len{} vol{} sc{:.2} ang{}",
                                g.tick, g.sound.length, g.cue_volume, g.scale, g.angle
                            )
                        })
                        .collect();
                    println!(
                        "  #{i} complete={} groups={:?} unread={:?} grains: {}",
                        t.is_complete(),
                        t.groups,
                        t.unread,
                        g.join(" | ")
                    );
                }
            }
        }
    }
}

fn args_names() -> Vec<String> {
    std::env::var("CUES")
        .unwrap_or_default()
        .split(',')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
