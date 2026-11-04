//! HD's circuit emitters, every circuit: which authored `sound` cones resolve
//! to a cue, which references dangle, and that the shared banks are what makes
//! most of them resolve.
//!
//! **`#[ignore]`d and never run in CI.** They need `hdfury-ps3-eu-dec.iso` under
//! `data/images/`. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! Evidence: `docs/formats/hd-audio.md`. The shared list is
//! `oag_hd::race::SOUND_BANKS.track.shared`; a circuit's own bank is the one its
//! `trackstartup.xml` `<LoadSoundBank>` names, beside the track.

use oag_sound::sfx::TrackEmitters;

/// `(circuit, emitters, resolved, dangling references, unassigned nodes)` for
/// every environment directory with a `track.vex`.
///
/// Every reference listed is one the disc authors against a label or a cue that
/// the circuit's own loaded banks do not carry; `docs/formats/hd-audio.md` gives
/// the proof for each (the bank's own name list, and which bank on the disc
/// carries the label). `modesto_heights` and `tech_de_ra` are the two whose
/// manifest names `env1_vinetak.bnk`, Vineta K's bank, while their nodes spell
/// the Zone bank's label and the bank beside the track: no manifest names the
/// right bank.
const CIRCUITS: [(&str, usize, usize, &[&str], usize); 16] = [
    (
        "amphiseum",
        54,
        49,
        &[
            "env_amp~neon",
            "g_sbkEnv~neoon_small",
            "g_sbkEnv~rndmnship_1",
            "gentrak~startlineneon",
        ],
        0,
    ),
    (
        "modesto_heights",
        48,
        5,
        &[
            "env_mod~advert_fem_l01",
            "env_mod~bridge",
            "env_mod~ind_factory_1",
            "env_mod~neoon_small",
            "env_mod~rndmnship_1",
            "env_mod~rndmnship_2",
            "env_mod~rndmnship_3",
            "env_mod~shipsdocked",
            "env_mod~startline",
        ],
        0,
    ),
    ("talons_junction", 66, 66, &[], 0),
    (
        "tech_de_ra",
        62,
        2,
        &[
            "crowd~neoon_small",
            "env_tec~ind_factory_1",
            "env_tec~neoon_small",
            "env_tec~rndmnship_1",
            "env_tec~rndmnship_2",
            "env_tec~rndmnship_3",
            "env_tec~startline",
            "techder~bluelight",
            "techder~tunnelsigns",
        ],
        0,
    ),
    ("zone_1", 0, 0, &[], 0),
    ("zone_2", 0, 0, &[], 0),
    ("zone_3", 0, 0, &[], 0),
    ("zone_4", 0, 0, &[], 0),
    ("01_vineta_k", 54, 52, &["GENTRAK~STARTLINE"], 0),
    ("02_track", 42, 42, &[], 0),
    ("03_track", 40, 33, &[], 7),
    (
        "04_chenghou_project",
        92,
        71,
        &[
            "CHENGOU~CRAFT",
            "env_met~rndmnship_1",
            "env_moa~rndmnship_1",
            "env_moa~rndmnship_2",
            "generalt~startline",
            "shipHD\t~ship_idle",
        ],
        0,
    ),
    ("05_ubermall", 132, 131, &["generalt~startline"], 0),
    (
        "10_sebenco_climb",
        68,
        64,
        &["generalt~laser", "generalt~windturbines"],
        0,
    ),
    ("12_sol_2", 67, 67, &[], 0),
    ("15_anulpha_pass", 52, 51, &[], 1),
];

fn open() -> Option<oag_source::title::Opened> {
    let path = oag_testdata::image("hdfury-ps3-eu-dec.iso")?;
    Some(
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source"),
    )
}

fn playing(loaded: &TrackEmitters) -> usize {
    loaded
        .omni
        .iter()
        .chain(&loaded.directional)
        .filter(|n| n.sound.is_some())
        .count()
}

/// One test per circuit, so the matrix is the test axis rather than a loop.
fn check(index: usize) {
    let (env, total, resolved, dangling, unassigned) = CIRCUITS[index];
    let Some(opened) = open() else {
        return;
    };
    let mut archives = opened.archives;
    let track = oag_hd::names::track(env);
    let blob = archives.read_name(&track).expect("the circuit");
    let loaded = TrackEmitters::load(&mut archives, opened.title.race.sounds, &track, &blob);
    for line in &loaded.report {
        println!("{env}: {line}");
    }
    assert_eq!(
        (
            loaded.omni.len() + loaded.directional.len(),
            playing(&loaded)
        ),
        (total, resolved),
        "{env}"
    );

    // Moa Therma's `AudioConfig.xml` alone has `<DistanceFilter low_pass="0">`.
    assert_eq!(
        loaded.low_pass_off,
        env == "03_track",
        "{env}: the circuit's low-pass switch"
    );

    let dangled: Vec<&String> = loaded
        .report
        .iter()
        .filter(|l| l.contains("node(s) dangle") || l.contains("node(s) play nothing"))
        .collect();
    assert_eq!(dangled.len(), dangling.len(), "{env}: {dangled:#?}");
    for reference in dangling {
        assert!(
            dangled.iter().any(|l| {
                l.starts_with(&format!("track audio {reference}: ")) && l.contains("node(s) dangle")
            }),
            "{env}: {reference} was not reported as dangling"
        );
    }
    let says = |what: &str| loaded.report.iter().any(|l| l.contains(what));
    assert_eq!(
        says("cue(s) the circuit names and its banks do not author"),
        !dangling.is_empty(),
        "{env}: the one summary line"
    );
    assert_eq!(
        says(&format!(
            "{unassigned} node(s) author no bank and no cue name"
        )),
        unassigned > 0,
        "{env}: unassigned nodes"
    );
    assert!(
        !says("not read") && !says("is not a sound bank"),
        "{env}: a bank did not read: {:#?}",
        loaded.report
    );
}

macro_rules! circuits {
    ($($name:ident = $index:expr),* $(,)?) => {$(
        #[test]
        #[ignore = "needs a disc image in data/images/"]
        fn $name() {
            check($index);
        }
    )*};
}

circuits! {
    amphiseum_resolves_forty_nine_of_fifty_four = 0,
    modesto_heights_names_vinetaks_bank_and_five_resolve = 1,
    talons_junction_resolves_all_sixty_six = 2,
    tech_de_ra_names_vinetaks_bank_and_two_resolve = 3,
    zone_one_authors_no_emitter = 4,
    zone_two_authors_no_emitter = 5,
    zone_three_authors_no_emitter = 6,
    zone_four_authors_no_emitter = 7,
    vineta_k_resolves_fifty_two_of_fifty_four = 8,
    metropia_resolves_all_forty_two = 9,
    moa_therma_leaves_seven_nodes_unassigned = 10,
    chenghou_resolves_seventy_one_of_ninety_two = 11,
    ubermall_resolves_all_but_one = 12,
    sebenco_climb_resolves_sixty_four_of_sixty_eight = 13,
    sol_two_resolves_all_sixty_seven = 14,
    anulpha_pass_leaves_one_node_unassigned = 15,
}

/// The shared banks are what resolve ubermall's `voppler` and `crowd` nodes:
/// with the list empty only the circuit's own bank resolves.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_shared_banks_are_what_resolve_the_voppler_and_crowd_nodes() {
    let Some(opened) = open() else {
        return;
    };
    let shared = opened.title.race.sounds.track.shared;
    assert_eq!(
        shared.len(),
        5,
        "generaltrack, voppler, crowd, radios, shipHD"
    );
    let mut archives = opened.archives;
    let track = oag_hd::names::track("05_ubermall");
    let blob = archives.read_name(&track).expect("the circuit");
    let sounds = opened.title.race.sounds;
    let none = oag_title::SoundBanks {
        track: oag_title::TrackBanks {
            shared: &[],
            ..sounds.track
        },
        ..*sounds
    };
    let without = TrackEmitters::load(&mut archives, &none, &track, &blob);
    let with = TrackEmitters::load(&mut archives, sounds, &track, &blob);
    assert_eq!(playing(&with), 131);
    assert_eq!(
        playing(&without),
        27,
        "only the circuit's own bank resolves without the shared list"
    );
}

fn render_voice(
    sound: &std::sync::Arc<oag_audio::Sound>,
    front: f32,
    law: Option<f32>,
) -> Vec<f32> {
    use oag_audio::{Bus, Mixer, Play, SpatialMode};
    let mut mixer = Mixer::new(48_000);
    mixer.set_limiting(false);
    mixer.set_spatial_mode(SpatialMode::Original);
    mixer.set_rear_occlusion(law);
    let id = mixer
        .play(Play {
            pan: Some(0.0),
            front,
            ..Play::looping(std::sync::Arc::clone(sound), Bus::Sfx)
        })
        .expect("voice");
    // Two seconds of ticks, so the cutoff has eased to where it is going.
    for _ in 0..120 {
        mixer.set_front(id, front);
    }
    let mut out = vec![0.0; 2 * 48_000];
    mixer.render(&mut out);
    out
}

/// Energy above `above_hz` over a 4096-frame window of the left channel, and the window's total.
fn band_energy(out: &[f32], above_hz: f32) -> (f32, f32) {
    const N: usize = 4096;
    let left: Vec<f32> = out.chunks(2).skip(24_000).take(N).map(|f| f[0]).collect();
    let hann = |i: usize| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / N as f32).cos();
    let first = (above_hz / 48_000.0 * N as f32).ceil() as usize;
    let (mut high, mut all) = (0.0_f32, 0.0_f32);
    for k in 1..N / 2 {
        let (mut re, mut im) = (0.0_f32, 0.0_f32);
        for (i, x) in left.iter().enumerate() {
            let a = std::f32::consts::TAU * (k * i % N) as f32 / N as f32;
            re += x * hann(i) * a.cos();
            im -= x * hann(i) * a.sin();
        }
        let power = re * re + im * im;
        all += power;
        if k >= first {
            high += power;
        }
    }
    (high, all)
}

/// Scratch sweep, not a claim: renders one HD circuit voice in front of and behind the listener
/// under `original` with the file's `RearOcclusion`, writes the two WAVs into `OAG_SWEEP`'s
/// directory and prints the energy above the cutoff in each.
#[test]
#[ignore = "scratch sweep: set OAG_SWEEP to an output directory"]
fn rear_occlusion_wav_pair() {
    let Some(dir) = std::env::var_os("OAG_SWEEP") else {
        return;
    };
    let Some(opened) = open() else {
        return;
    };
    let cutoff = 14_000.0_f32;
    let mut archives = opened.archives;
    let mut rates = std::collections::BTreeMap::<u32, usize>::new();
    let mut best: Option<(f32, String, std::sync::Arc<oag_audio::Sound>)> = None;
    let mut seen = Vec::new();
    for (env, ..) in CIRCUITS.iter().filter(|c| c.1 > 0) {
        let track = oag_hd::names::track(env);
        let blob = archives.read_name(&track).expect("the circuit");
        let loaded = TrackEmitters::load(&mut archives, opened.title.race.sounds, &track, &blob);
        for node in loaded.omni.iter().chain(&loaded.directional) {
            let Some(cue) = &node.sound else { continue };
            for (sound, _) in &cue.waveforms {
                if seen.iter().any(|s| std::sync::Arc::ptr_eq(s, sound)) {
                    continue;
                }
                seen.push(std::sync::Arc::clone(sound));
                *rates.entry(sound.sample_rate()).or_default() += 1;
                // Nothing above half the source rate to cut.
                if sound.sample_rate() < 2 * cutoff as u32 {
                    continue;
                }
                let (high, all) = band_energy(&render_voice(sound, 1.0, None), cutoff);
                let share = high / all.max(1e-12);
                if all > 1e-3 && best.as_ref().is_none_or(|(b, ..)| share > *b) {
                    best = Some((share, (*env).to_string(), std::sync::Arc::clone(sound)));
                }
            }
        }
    }
    println!("source rates of HD's circuit voices: {rates:?}");
    let (_, env, sound) = best.expect("a voice whose source reaches the cutoff");
    let dir = std::path::Path::new(&dir);
    std::fs::create_dir_all(dir).expect("dir");
    for (name, front, law) in [
        ("front", 1.0, Some(cutoff)),
        ("behind", -1.0, Some(cutoff)),
        ("behind_without_law", -1.0, None),
    ] {
        let out = render_voice(&sound, front, law);
        let (high, all) = band_energy(&out, cutoff);
        println!(
            "{env} {name}: {:.1} s, source rate {} Hz, energy above {cutoff} Hz {high:.4e} of {all:.4e} ({:.3}%)",
            sound.seconds(),
            sound.sample_rate(),
            100.0 * high / all.max(1e-12)
        );
        std::fs::write(
            dir.join(format!("hd_{env}_{name}.wav")),
            oag_audio::wav::from_samples(&out, 48_000),
        )
        .expect("write");
    }
}
