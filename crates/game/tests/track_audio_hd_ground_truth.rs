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
