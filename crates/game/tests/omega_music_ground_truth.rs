//! Omega's soundtrack, read off the bank and played.
//!
//! **`#[ignore]`d and never run in CI**; needs `data/extracted/ps4`.

use oag_game::music::omega;

fn source() -> Option<String> {
    let path = oag_testdata::exact("data/extracted/ps4")?;
    Some(path.display().to_string())
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn explore() {
    let Some(source) = source() else { return };
    let mut archives = oag_omega::open(&source).expect("opens");
    let st = oag_omega::MUSIC.state_tracks.expect("state tracks");
    let plan = omega::plan(&mut archives, &st, true).expect("plan");
    for s in &plan.songs {
        let pcm = omega::mix(&mut archives, &st, s).expect("mix");
        let frames = pcm.samples.len() / 2;
        let peak = pcm
            .samples
            .iter()
            .map(|v| i32::from(*v).abs())
            .max()
            .unwrap();
        let mut stem_peaks = Vec::new();
        for id in &s.stems {
            let blob = archives
                .read_name(&format!("{}{id}.wem", st.media_dir))
                .unwrap();
            let p = oag_game::wem::decode(&blob).unwrap();
            stem_peaks.push(p.samples.iter().map(|v| i32::from(*v).abs()).max().unwrap());
        }
        println!(
            "clips {:?}",
            s.clips
                .iter()
                .map(|c| (c.play_at, c.begin_trim, c.end_trim, c.source_duration))
                .take(3)
                .collect::<Vec<_>>()
        );
        println!(
            "loc {} {:?} stems {} seg {:.2}s wem {:.2}s peak {} stems {:?}",
            s.location,
            s.title,
            s.stems.len(),
            s.seconds,
            frames as f64 / 48000.0,
            peak,
            stem_peaks
        );
    }
    for (t, l, why) in &plan.skipped {
        println!("skipped loc {l} {t:?}: {why:?}");
    }
}
