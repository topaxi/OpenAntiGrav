//! Omega's advert cards load from Omega's own `data00` through the Vita/PS4
//! container branch of `gantry::build`.
//!
//! **`#[ignore]`d and never run in CI.** It needs `data/extracted/ps4`:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/start_gantry_omega_ground_truth/)'
//! ```
//!
//! Each circuit's `trackstartup.xml` names the model every billboard slot
//! shows (`/Data/Billboards/HD_Adverts/...vex`). Before the branch every one of
//! them failed on the PS4 container ("a PS3 .vex with no .rcsmodel", then the
//! `.rcsmodel` refused as a Vita/PS4 one); now each builds with draws and
//! decoded textures. **Whether Omega draws them is a separate answer**
//! (`oag_omega::TITLE.adverts` stays `None`): see `docs/rendering/start-gantry.md`,
//! "Omega".

use oag_raceplay::gantry;

/// The four circuits HD's gantry tests walk (`start_gantry_hd_*`), as Omega
/// spells them.
const CIRCUITS: [&str; 4] = ["tech_de_ra", "talons_junction", "amphiseum", "01_vineta_k"];

fn archives() -> Option<oag_assets::Archives> {
    let source = oag_testdata::exact("data/extracted/ps4")?;
    Some(oag_omega::open(&source.display().to_string()).expect("opening Omega"))
}

/// Loads every model circuit `name`'s manifest names, returning `(slot, draws,
/// decoded textures)` per slot and the report.
/// `(slot, draws, decoded textures)` per slot, and the load report.
type Loaded = (Vec<(u32, usize, usize)>, Vec<String>);

fn load_circuit(name: &str) -> Option<Loaded> {
    let mut archives = archives()?;
    let manifest = archives
        .read_name(&format!("/data/environments/{name}/trackstartup.xml"))
        .unwrap_or_else(|e| panic!("{name}: no trackstartup.xml ({e})"));
    let manifest =
        oag_tables::trackstartup::TrackStartup::parse(&String::from_utf8_lossy(&manifest));
    let mut report = Vec::new();
    let mut loaded = Vec::new();
    for billboard in &manifest.billboards {
        let Some(model) = billboard.location() else {
            continue;
        };
        let (model, _blob) = gantry::load_with_blob(&mut archives, model, &mut report)
            .unwrap_or_else(|e| {
                panic!(
                    "{name} slot {}: {model} does not load: {e:#}",
                    billboard.num
                )
            });
        let draws =
            model.draws.len() + model.alpha_tested_draws.len() + model.transparent_draws.len();
        let textures = model.textures.iter().flatten().count();
        loaded.push((billboard.num, draws, textures));
    }
    Some((loaded, report))
}

fn check(name: &str) {
    let Some((loaded, report)) = load_circuit(name) else {
        return;
    };
    assert_eq!(
        loaded.len(),
        8,
        "{name}: eight slots name a model: {loaded:?}"
    );
    for (slot, draws, textures) in &loaded {
        assert!(*draws > 0, "{name} slot {slot}: no draws");
        assert!(*textures > 0, "{name} slot {slot}: no decoded texture");
    }
    let go = report
        .iter()
        .find(|l| l.starts_with("/Data/Billboards/HD_Adverts/321Go/321Go_StartFinish.vex:"))
        .unwrap_or_else(|| panic!("{name}: no 321Go_StartFinish line in {report:#?}"));
    assert!(go.contains("(Vita/PS4 container)"), "{go}");
    assert!(
        go.contains(
            "1530 triangle(s) over 19 submesh(es), 19/19 draw(s) textured from 5 of 5 material(s)"
        ),
        "{go}"
    );
    assert!(go.contains("19 moving on 19 animated node(s)"), "{go}");
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn tech_de_ra_adverts_load_from_omegas_container() {
    check(CIRCUITS[0]);
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn talons_junction_adverts_load_from_omegas_container() {
    check(CIRCUITS[1]);
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn amphiseum_adverts_load_from_omegas_container() {
    check(CIRCUITS[2]);
}

#[test]
#[ignore = "needs data/extracted/ps4"]
fn vineta_k_adverts_load_from_omegas_container() {
    check(CIRCUITS[3]);
}

/// The Zone board, `321Go_Zone.vex`, builds through the same branch. No circuit
/// shows it yet (`zone_gantry` is `None` on Omega), so this is a reader check.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn the_zone_board_builds_through_the_vita_ps4_branch() {
    let Some(mut archives) = archives() else {
        return;
    };
    let mut report = Vec::new();
    let (model, _) = gantry::load_with_blob(
        &mut archives,
        "/Data/Billboards/HD_Adverts/321Go/321Go_Zone.vex",
        &mut report,
    )
    .expect("321Go_Zone.vex builds");
    assert!(!model.draws.is_empty() || !model.transparent_draws.is_empty());
    println!("{report:#?}");
}

/// The 2048 check (CLAUDE.md, "A 2048 finding is checked against Omega"): 2048's
/// own `data.psarc` ships the same four `321Go_*.vex` names with its own bytes
/// (8,448-byte `321Go_Zone.vex`), and the same branch reads them.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_same_branch_reads_2048s_321go_files() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archives = oag_2048::open(&source.display().to_string()).expect("mounting 2048");
    for name in ["321Go_StartFinish", "321Go_Zone"] {
        let mut report = Vec::new();
        let path = format!("/Data/Billboards/HD_Adverts/321Go/{name}.vex");
        let (model, _) = gantry::load_with_blob(&mut archives, &path, &mut report)
            .unwrap_or_else(|e| panic!("{path}: {e:#}"));
        assert!(
            !model.draws.is_empty() || !model.transparent_draws.is_empty(),
            "{name}"
        );
        println!("{name}: {report:#?}");
    }
}
