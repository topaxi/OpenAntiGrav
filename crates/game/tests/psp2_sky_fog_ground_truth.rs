//! Wipeout 2048's sky turn and distance fog, over every base circuit.
//!
//! **`#[ignore]`d and never run in CI**: it needs the decrypted Vita package at
//! `data/extracted/vita/PCSF00007`. Run with `just test-data`.
//!
//! What this pins, per circuit, against the circuit's own `track.EnvSettings`
//! read independently of the loader:
//!
//! - the dome is turned by `Lighting.Sky rotation`, in degrees about the
//!   vertical: every vertex keeps its height and horizontal radius against the
//!   unturned dome and moves round by exactly the authored angle;
//! - the race carries a fog whose colour is the first three numbers of
//!   `Lighting.Fog colour` and whose density is the fourth, on the curve the
//!   loader labels as inherited from HD (2048's own microcode is unread).
//!
//! Not pinned, because nothing reads them: `Sky brightness`, `Sky height
//! offset`, the `Fog Region Colour Override` ladder and `Depth Fog Offset
//! RecipRange`.

use std::path::{Path, PathBuf};

use oag_raceplay as race;
use oag_tables::envsettings::{EnvSettings, PSP2_FOG_COLOUR, SKY_ROTATION};

/// The dome's radius after the loader's rescale, `PSP2_SKY_TARGET_RADIUS`.
const TARGET_RADIUS: f32 = 32.0;

fn package() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted/vita/PCSF00007");
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn circuit(package: &Path, name: &str) -> (race::Loaded, EnvSettings, Vec<[f32; 3]>) {
    let track = format!("Data\\art\\published\\environments\\{name}\\track.vex");
    let loaded = race::load(&race::Options {
        source: package.display().to_string(),
        class: "VENOM".to_string(),
        track: Some(track),
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("{name}: loading the race: {e:#}"));

    let mut archives =
        oag_2048::open(&package.join("base").display().to_string()).expect("the package opens");
    let base = format!("Data\\art\\published\\environments\\{name}");
    let text = String::from_utf8(
        archives
            .read_name(&format!("{base}\\track.EnvSettings"))
            .expect("the circuit ships its EnvSettings"),
    )
    .expect("UTF-8");
    let env = EnvSettings::parse(&text).expect("the file parses");

    let blob = archives
        .read_name(&format!("{base}\\skycube.rcsmodel"))
        .expect("the circuit ships its dome");
    let (dome, _) = oag_mesh::mesh::rcs::psp2::build("dome", &blob, None, &mut |path| {
        archives.read_name(path).ok()
    })
    .expect("the dome builds");
    let scale = TARGET_RADIUS / dome.radius;
    let unturned = dome
        .vertices
        .iter()
        .map(|v| v.position.map(|a| a * scale))
        .collect();
    (loaded, env, unturned)
}

fn dome_turns(name: &str) {
    let Some(package) = package() else {
        return;
    };
    let (loaded, env, unturned) = circuit(&package, name);
    let degrees = env
        .scalar(SKY_ROTATION)
        .unwrap_or_else(|| panic!("{name}: no Sky rotation"));
    let sky = loaded
        .sky_model
        .as_ref()
        .unwrap_or_else(|| panic!("{name}: no sky"));
    assert_eq!(sky.vertices.len(), unturned.len(), "{name}");
    for (v, u) in sky.vertices.iter().zip(&unturned) {
        let p = v.position;
        assert!((p[1] - u[1]).abs() < 1e-2, "{name}: height moved");
        let (r, ur) = (p[0].hypot(p[2]), u[0].hypot(u[2]));
        assert!((r - ur).abs() < 1e-2, "{name}: horizontal radius moved");
        if ur > 1.0 {
            // The turn the loader applies maps (x, z) to
            // (x cos + z sin, z cos - x sin): the bearing grows by the angle.
            let delta = (p[0].atan2(p[2]) - u[0].atan2(u[2])).to_degrees() - degrees;
            let delta = (delta + 180.0).rem_euclid(360.0) - 180.0;
            assert!(delta.abs() < 0.1, "{name}: dome turned by the wrong angle");
        }
    }
}

fn fog_is_authored(name: &str) {
    let Some(package) = package() else {
        return;
    };
    let (loaded, env, _) = circuit(&package, name);
    let [r, g, b, density] = env
        .vec4(PSP2_FOG_COLOUR)
        .unwrap_or_else(|| panic!("{name}: no Fog colour"));
    let fog = loaded
        .authored_fog
        .unwrap_or_else(|| panic!("{name}: no fog"));
    assert_eq!(fog.colour, [r, g, b], "{name}");
    assert_eq!(fog.density, density, "{name}");
    assert_eq!(fog.curve, 1.0, "{name}");
    assert!(
        loaded
            .report
            .iter()
            .any(|l| l.contains("fog [") && l.contains("INHERITED")),
        "{name}: the fog is not labelled inherited"
    );
}

macro_rules! circuits {
    ($($name:ident),*) => {$(
        mod $name {
            #[test]
            #[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
            fn turns_its_dome_by_its_own_sky_rotation() {
                super::dome_turns(stringify!($name));
            }

            #[test]
            #[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
            fn carries_its_authored_fog_labelled_inherited() {
                super::fog_is_authored(stringify!($name));
            }
        }
    )*};
}

circuits!(
    altima, arena, bridge, cathedral, mall, park, sol, square, subway, tower
);
