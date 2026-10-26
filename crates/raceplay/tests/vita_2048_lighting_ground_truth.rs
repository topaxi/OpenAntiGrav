//! Wipeout 2048's own `track.EnvSettings` light rig, read against its own key
//! spelling rather than Wipeout HD's.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted Vita package
//! extracted with `oag-unpack`. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md` for how
//! `data/extracted/vita/PCSF00007` gets populated.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-raceplay \
//!     --run-ignored all -E 'test(/^vita_2048_lighting_ground_truth::/)'
//! ```
//!
//! The regression this guards: before `GeometryKind` split HD's and 2048's
//! key sets, `envsettings_light` read every title's file against HD's own
//! constants (`"Lighting.Sun color"`, `"Lighting.Constant ambient color"`),
//! which never resolve against 2048's own spelling
//! (`"Lighting.Sun diffuse colour"`, `"Lighting.Constant ambient colour"`) -
//! see `docs/formats/envsettings.md`'s "Wipeout 2048 authors the same shape
//! under different key names".

use oag_raceplay as race;
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
    // 2048 authors the Nova triple too (`1 0 1 0`), but its shader is unread:
    // only Wipeout: Omega Collection's PS4 container takes the Nova path.
    assert_eq!(
        loaded.light.nova, 0.0,
        "2048 must not take Omega's prelit combination"
    );
    altima_draws_its_own_wo_composite(&loaded);
}

/// The same load reaches `oag_post::wo_composite` with Altima's own five
/// keys, `"Lighting.ExposureScale"=0.400000`, `ExposureMax` 12,
/// `ExposureSpeed` 0.03, `BloomFactor` 0.3 and `BloomGate` 0.3, adapting as
/// the original does in play. Fails when `Looks::wo_composite` or the staging
/// stops carrying them: the race then ends in no composite at all.
fn altima_draws_its_own_wo_composite(loaded: &race::Loaded) {
    use oag_post::wo_composite::{Exposure, Params};
    assert_eq!(
        loaded.wo_composite,
        Some(Params {
            exposure_scale: 0.4,
            exposure_max: 12.0,
            exposure_speed: 0.03,
            bloom_factor: 0.3,
            bloom_gate: 0.3,
            exposure: Exposure::Adapting,
        }),
        "{:#?}",
        loaded.report
    );
    assert!(
        loaded.hd_bloom.is_none(),
        "2048 ends in its own composite, not HD's FunkLayerBloom"
    );
}

const KEYS: [&str; 3] = [
    oag_tables::envsettings::PSP2_SUN_DIFFUSE_COLOUR,
    oag_tables::envsettings::PSP2_AMBIENT_COLOUR,
    oag_tables::envsettings::PSP2_FOG_COLOUR,
];

fn key_values(env: &oag_tables::envsettings::EnvSettings) -> Vec<Option<[f32; 4]>> {
    KEYS.iter()
        .map(|key| {
            env.vec4(key)
                .or_else(|| env.vec3(key).map(|v| [v[0], v[1], v[2], 0.0]))
        })
        .collect()
}

/// `(circuit entry, base copy, pack copy)` for every offered circuit whose
/// pack and base `track.EnvSettings` disagree on the sun, ambient or fog key.
fn differing_circuits(
    source: &Path,
) -> Vec<(
    String,
    oag_tables::envsettings::EnvSettings,
    oag_tables::envsettings::EnvSettings,
)> {
    use oag_tables::envsettings::EnvSettings;
    let mut archives = oag_assets::Archives::open(&source.display().to_string(), oag_2048::TITLE)
        .expect("the 2048 package opens");
    let documents = race::catalogue::documents(
        &mut archives,
        &[
            oag_2048::TITLE.plugin_definition,
            oag_2048::TITLE.track_plugin_definition.expect("split"),
        ],
    );
    let packs: Vec<&str> = archives.add_ons.iter().map(|a| a.archive).collect();
    let mut out = Vec::new();
    for track in race::catalogue::all_tracks(&documents) {
        let entry = track.entry_name();
        let Some(stem) = entry
            .strip_suffix("track.vex")
            .or_else(|| entry.rsplit_once('\\').map(|(dir, _)| &entry[..=dir.len()]))
        else {
            continue;
        };
        let name = format!("{stem}track.EnvSettings");
        let copies: Vec<_> = archives
            .read_every_name(&name)
            .into_iter()
            .filter_map(|(label, blob)| {
                let env = EnvSettings::parse(&String::from_utf8(blob).ok()?).ok()?;
                Some((label, env))
            })
            .collect();
        let pack = copies
            .iter()
            .find(|(label, _)| packs.iter().any(|p| label.to_lowercase().ends_with(p)));
        let base = copies
            .iter()
            .find(|(label, _)| !packs.iter().any(|p| label.to_lowercase().ends_with(p)));
        if let (Some((_, pack)), Some((_, base))) = (pack, base) {
            if key_values(pack) != key_values(base) {
                out.push((entry, base.clone(), pack.clone()));
            }
        }
    }
    out
}

/// A 2048 DLC race lights and fogs with the add-on pack's copy of the
/// circuit's `track.EnvSettings`, not the base archive's copy of the same path
/// (the base ships HD Campaign circuits' paths with different values). Vineta
/// K's pack sun diffuse (2.0, 1.66, 0.96) is about twice the base's (1.0, 0.83, 0.49).
#[test]
#[ignore = "needs the decrypted Vita package in data/extracted/vita/"]
fn a_dlc_circuit_lights_and_fogs_with_the_packs_copy() {
    let Some(source) = source() else {
        return;
    };
    let differing = differing_circuits(&source);
    println!("circuits whose pack and base copies differ:");
    for (entry, base, pack) in &differing {
        let (base, pack) = (key_values(base), key_values(pack));
        for (key, (b, p)) in KEYS.iter().zip(base.iter().zip(&pack)) {
            if b != p {
                println!("  {entry}: {key}: base {b:?} -> pack {p:?}");
            }
        }
    }
    assert!(
        differing
            .iter()
            .any(|(entry, ..)| entry.contains("Vineta_K")),
        "Vineta K's pack and base copies differ: {differing:?}"
    );
    // A reversed circuit reads the same `track.EnvSettings` as its forward twin
    // (the census above lists both); Moa Therma reversed does not decode at all
    // today, so the forward races stand for them.
    for (entry, base, pack) in differing
        .iter()
        .filter(|(entry, ..)| !entry.contains("reversed"))
    {
        let loaded = race::load(&race::Options {
            source: source.display().to_string(),
            mode: oag_race::Mode::TimeTrial,
            track: Some(entry.clone()),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("loading {entry}: {e:#}"));
        let pack_sun = pack
            .vec3(oag_tables::envsettings::PSP2_SUN_DIFFUSE_COLOUR)
            .expect("the pack authors a sun");
        let base_sun = base.vec3(oag_tables::envsettings::PSP2_SUN_DIFFUSE_COLOUR);
        assert_eq!(
            loaded.light.sun, pack_sun,
            "{entry}: the sun is the pack's (base {base_sun:?}); {:#?}",
            loaded.report
        );
        let pack_ambient = pack
            .vec3(oag_tables::envsettings::PSP2_AMBIENT_COLOUR)
            .expect("the pack authors an ambient");
        assert_eq!(
            loaded.light.ambient, pack_ambient,
            "{entry}: the ambient is the pack's"
        );
        let pack_fog = pack.vec4(oag_tables::envsettings::PSP2_FOG_COLOUR);
        if let Some(fog) = loaded.authored_fog.as_ref().zip(pack_fog) {
            assert_eq!(
                (fog.0.colour, fog.0.density),
                ([fog.1[0], fog.1[1], fog.1[2]], fog.1[3]),
                "{entry}: the fog is the pack's"
            );
        }
    }
}
