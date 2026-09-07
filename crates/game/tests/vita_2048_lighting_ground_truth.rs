//! Wipeout 2048's own `track.EnvSettings` light rig, read against its own key
//! spelling rather than Wipeout HD's.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted Vita package
//! extracted with `oag-unpack`. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md` for how
//! `data/extracted/vita/PCSF00007` gets populated.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
//!     --run-ignored all -E 'binary(vita_2048_lighting_ground_truth)'
//! ```
//!
//! The regression this guards: before `GeometryKind` split HD's and 2048's
//! key sets, `envsettings_light` read every title's file against HD's own
//! constants (`"Lighting.Sun color"`, `"Lighting.Constant ambient color"`),
//! which never resolve against 2048's own spelling
//! (`"Lighting.Sun diffuse colour"`, `"Lighting.Constant ambient colour"`) -
//! see `docs/formats/envsettings.md`'s "Wipeout 2048 authors the same shape
//! under different key names".

use oag_game::race;
use std::path::{Path, PathBuf};

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but the 2048 package is not extracted"
    );
    println!("skipping: 2048 package not extracted under data/extracted/vita/");
    None
}

/// Altima's own light rig, read off the disc's own `track.EnvSettings`
/// exactly as `cargo run -p oag-assets --example psarc_cat` prints it:
/// `"Lighting.Sun diffuse colour"=2.000000 1.803922 1.701961` and
/// `"Lighting.Constant ambient colour"=0.149000 0.247000 0.382000`.
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn altimas_own_sun_and_ambient_are_drawn_not_the_stand_in_rig() {
    let Some(source) = source() else {
        return;
    };

    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading {}: {e:#}", source.display()));

    // A line matching `"sun ["` only ever comes from the authored branch -
    // the fallback's own line has no such shape ("no usable sun direction,
    // colour and ambient; lighting with the stand-in rig") - so finding one
    // at all is already the regression check: before `GeometryKind` split
    // the two titles' key sets, this line never appeared for a 2048 race.
    let light_line = loaded
        .report
        .iter()
        .find(|line| line.contains("sun ["))
        .unwrap_or_else(|| {
            panic!(
                "no authored light-rig line in the report - 2048 fell back to the \
             stand-in rig again; was {:#?}",
                loaded.report
            )
        });
    assert!(
        light_line.contains("colour [2.00, 1.80, 1.70]"),
        "expected Altima's own Sun diffuse colour (2.0, 1.8, 1.7); was {light_line}"
    );
    assert!(
        light_line.contains("ambient [0.15, 0.25, 0.38]"),
        "expected Altima's own Constant ambient colour (0.15, 0.25, 0.38); was {light_line}"
    );
}
