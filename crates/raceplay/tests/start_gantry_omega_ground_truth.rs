//! Omega's start gantry: the `321Go` advert cards load from Omega's own
//! `data00` through the Vita/PS4 container branch of `gantry::build`.
//!
//! **`#[ignore]`d and never run in CI.** Needs `data/extracted/ps4`.

use oag_raceplay as race;

fn source() -> Option<String> {
    oag_testdata::exact("data/extracted/ps4").map(|p| p.display().to_string())
}

fn load(track: &str) -> Option<race::Loaded> {
    let source = source()?;
    Some(
        race::load(&race::Options {
            source,
            track: Some(format!("/data/environments/{track}/track.vex")),
            ..race::Options::default()
        })
        .expect("loading the race"),
    )
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn probe() {
    let Some(loaded) = load("tech_de_ra") else {
        return;
    };
    for l in &loaded.report {
        println!("{l}");
    }
    for c in &loaded.billboards.adverts {
        println!("card slot {}", c.slot);
    }
}

#[test]
#[ignore = "scratch"]
fn probe_strings() {
    let Some(source) = source() else { return };
    let mut a = oag_omega::open(&source).expect("open");
    for n in [
        r"Data\environments\tech_de_ra\track.rcsmodel",
        r"Data\environments\tech_de_ra\track.vex",
    ] {
        let Ok(b) = a.read_name(n) else {
            println!("{n}: missing");
            continue;
        };
        println!("{n}: {} bytes", b.len());
        let s = String::from_utf8_lossy(&b);
        for m in s.match_indices("illboard") {
            let from = s[..m.0].rfind(|c: char| c.is_control()).map_or(0, |i| i + 1);
            let to = s[m.0..].find(|c: char| c.is_control()).map_or(s.len(), |i| m.0 + i);
            println!("  {}", &s[from..to]);
        }
    }
}
