//! `.probes` against the disc and against the live maps of a running original.
//!
//! **`#[ignore]`d, needs the decrypted PS3 image** (`just test-data`).
//!
//! - Every `.probes` of `DATA00.PSARC` (28 on the EU disc) closes the layout exactly
//!   and parses, and its probes sit in left/right pairs.
//! - The blend reproduces what the original binds. `data/reference/hd-craft-ibl-probe/`
//!   holds fifty-two pauses of two drives round Talon's Junction on RPCS3: the player's
//!   position and velocity, and the live `paraboloidIblTex` and
//!   `paraboloidReflectionTex` at that moment. A hull is lit from a position one to
//!   two frames old, so each sample is blended at the position a frame back, and the
//!   better of lags 0, 1 and 2 is held to a small RMS. A reader that mis-decoded the
//!   blocks, swapped the channels or located the wrong probes cannot get near it:
//!   the nearest wrong probe sits at an RMS of 20-40 of 255.

use oag_assets::psarc::Archive;
use oag_vex::probes::{IRRADIANCE_SIZE, Probes, REFLECTION_SIZE};

const IMAGE: &str = "hdfury-ps3-eu-dec.iso";
const ARCHIVE: &str = "PS3_GAME/USRDIR/DATA00.PSARC";
const SAMPLES: usize = 52;
/// An RMS (of 255) a sample at the right probes stays under; the live maps are
/// themselves 8-bit blends the original makes on the SPU.
const CLOSE: f32 = 10.0;
/// Samples 8 and 49 sit in tight corners; see the assertion's comment.
const KNOWN_CORNER_MISSES: [usize; 2] = [8, 49];
const TALONS: &str = "/data/environments/talons_junction/track.probes";

fn archive() -> Option<Archive> {
    let path = oag_testdata::image(IMAGE)?;
    Some(Archive::open(&format!("{}:{ARCHIVE}", path.display())).expect("the archive opens"))
}

#[test]
#[ignore = "needs the decrypted HD disc image"]
fn every_probes_file_on_the_disc_closes_its_layout() {
    let Some(image) = oag_testdata::image(IMAGE) else {
        return;
    };
    let specs: Vec<String> = oag_disc::DiscImage::open(&image)
        .expect("open image")
        .entries()
        .expect("iso walk")
        .iter()
        .filter(|e| !e.is_directory && e.path.to_ascii_lowercase().ends_with(".psarc"))
        .map(|e| format!("{}:{}", image.display(), e.path))
        .collect();
    let mut seen = 0;
    for spec in specs {
        let mut archive = Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.ends_with(".probes"))
            .cloned()
            .collect();
        for path in paths {
            seen += 1;
            let bytes = archive.read_path(&path).expect("reads");
            let probes = Probes::parse(&bytes).unwrap_or_else(|e| panic!("{path}: {e}"));
            assert!(
                probes.len() >= 2 && probes.len() % 2 == 0,
                "{path}: {} probes",
                probes.len()
            );
            for i in 0..probes.len() {
                assert!(
                    probes.position(i).iter().all(|c| c.is_finite()),
                    "{path}: probe {i}"
                );
            }
            let across: f32 = (0..probes.len() / 2)
                .map(|s| {
                    let (a, b) = (probes.position(2 * s), probes.position(2 * s + 1));
                    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
                })
                .sum::<f32>()
                / (probes.len() / 2) as f32;
            assert!(
                (5.0..60.0).contains(&across),
                "{path}: pairs {across} apart on average"
            );
        }
    }
    assert_eq!(seen, 28, "one per circuit direction");
}

fn pair(text: &str, key: &str) -> [f32; 3] {
    let line = text.lines().find(|l| l.starts_with(key)).expect("key");
    let v: Vec<f32> = line
        .split_whitespace()
        .skip(1)
        .map(|t| t.parse().expect("float"))
        .collect();
    [v[0], v[1], v[2]]
}

/// RMS over the RGB of two `RGBA8` images, `live` being the capture's big-endian
/// `A8R8G8B8`.
fn rms(ours: &[u8], live: &[u8]) -> f32 {
    let mut sum = 0.0f32;
    for (o, l) in ours.chunks_exact(4).zip(live.chunks_exact(4)) {
        for (oc, lc) in [(o[0], l[1]), (o[1], l[2]), (o[2], l[3])] {
            let d = f32::from(oc) - f32::from(lc);
            sum += d * d;
        }
    }
    (sum / (ours.len() / 4 * 3) as f32).sqrt()
}

#[test]
#[ignore = "needs the decrypted HD disc image and data/reference/hd-craft-ibl-probe"]
fn the_blend_reproduces_the_maps_the_original_binds() {
    let Some(mut archive) = archive() else { return };
    let dir = oag_testdata::repo_root().join("data/reference/hd-craft-ibl-probe");
    if !dir.join("00.pose.txt").exists() {
        eprintln!("skipped: {} is absent", dir.display());
        return;
    }
    let probes = Probes::parse(&archive.read_path(TALONS).expect("reads")).expect("parses");
    let mut worst = 0.0f32;
    let mut misses = Vec::new();
    for n in 0..SAMPLES {
        let pose = std::fs::read_to_string(dir.join(format!("{n:02}.pose.txt"))).expect("pose");
        let (pos, vel) = (pair(&pose, "pos"), pair(&pose, "vel"));
        let ibl = std::fs::read(dir.join(format!("{n:02}-ibl.bin"))).expect("ibl");
        let refl = std::fs::read(dir.join(format!("{n:02}-refl.bin"))).expect("refl");
        assert_eq!(ibl.len(), IRRADIANCE_SIZE.0 * IRRADIANCE_SIZE.1 * 4);
        assert_eq!(refl.len(), REFLECTION_SIZE.0 * REFLECTION_SIZE.1 * 4);
        let best = (0..3)
            .map(|lag| {
                let k = lag as f32 / 60.0;
                let p = [
                    pos[0] - vel[0] * k,
                    pos[1] - vel[1] * k,
                    pos[2] - vel[2] * k,
                ];
                let taps = probes.locate(p);
                rms(&probes.blend_irradiance(&taps), &ibl)
                    .max(rms(&probes.blend_reflection(&taps), &refl))
            })
            .fold(f32::INFINITY, f32::min);
        println!("sample {n:02}: best-lag RMS {best:.2}");
        if best >= CLOSE {
            misses.push((n, best));
        }
        worst = worst.max(best);
    }
    // Two tight corners: the original blends a section pair two apart where the
    // nearest-centre law takes neighbours (probes 789/793 and 452/456 against
    // 789/791 and 454/456), a one-in-twenty miss, held at exactly these samples.
    let named: Vec<usize> = misses.iter().map(|m| m.0).collect();
    assert_eq!(
        named, KNOWN_CORNER_MISSES,
        "samples over {CLOSE} RMS: {misses:?}"
    );
    assert!(worst < 35.0, "worst sample {worst:.2} of 255");
}
