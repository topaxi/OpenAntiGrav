//! A circuit's host textures written as PNG files for the Blender add-on, off a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image per title: Pulse (PSP and PS2),
//! Pure, HD, Omega, and 2048's extracted package (`data/extracted/vita/PCSF00007`).
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay host_export_ground_truth:: --run-ignored all
//! ```
//!
//! What only real data can say: the names a track file spells (full asset paths with a
//! `.tga` the disc's own table carries) resolve to decodable pictures, each lands at the
//! path the add-on computes, and the files are pictures, not flat fills.
//!
//! One circuit of each title is exported whole. Pulse's and Pure's pictures are in the
//! `.vex`; HD's, 2048's and Omega's are the files the circuit's materials name, read the way
//! a race reads them. Each of those three pins two pixels of one picture, which fail if its
//! rows were turned over (HD's `.gtf` rows run bottom-up, and the race's 3D path uses them
//! as stored).

use std::collections::BTreeSet;
use std::path::Path;

use oag_raceplay::foreign::art::host_export::texture_file;

const CIRCUIT: &str = "Data\\Environments\\03_Track\\track.vex";
const IMAGE: &str = "pulse-psp-eu.chd";

fn options() -> Option<oag_raceplay::Options> {
    let image = oag_testdata::image(IMAGE)?;
    Some(oag_raceplay::Options {
        source: image.display().to_string(),
        ..oag_raceplay::Options::default()
    })
}

fn scratch(tag: &str) -> oag_testdata::scratch::TempDir {
    oag_testdata::scratch::dir(&format!("host-export-{tag}"))
}

fn decode(file: &Path) -> oag_texture::replacer::Image {
    let bytes = std::fs::read(file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let image = oag_texture::replacer::decode_png(&bytes)
        .unwrap_or_else(|| panic!("{} is not a decodable PNG", file.display()));
    assert!(image.width > 0 && image.height > 0, "{}", file.display());
    image
}

/// A texture that is a picture; some are flat by design (`whitenonalpha`).
fn assert_a_picture(file: &Path) {
    let image = decode(file);
    let first = &image.rgba[..4];
    assert!(
        image.rgba.as_chunks::<4>().0.iter().any(|px| px != first),
        "{} is one flat colour",
        file.display()
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_whole_circuit_writes_every_texture_it_decodes() {
    let Some(options) = options() else { return };
    let out = scratch("circuit");
    let done = oag_raceplay::export_host_textures(&options, CIRCUIT, &out).expect("an export");
    assert!(done.written.len() > 100, "{} written", done.written.len());
    let road = out.join(
        texture_file(
            "wipeout-pulse",
            CIRCUIT,
            "Data\\Environments\\GenericTrackTextures\\_surface6_1.tga",
        )
        .expect("a safe path"),
    );
    assert_a_picture(&road);
    for file in done.written.iter().take(12) {
        decode(file);
    }
    let _ = std::fs::remove_dir_all(&out);
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn a_track_files_host_rows_each_resolve_to_a_file() {
    let Some(options) = options() else { return };
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/mods/examples/hillside/tracks/hillside.track.toml");
    if !manifest.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but the examples submodule is missing"
        );
        return;
    }
    let loaded = oag_trackfile::read(&manifest).expect("hillside reads");
    let art = loaded.track.art.expect("hillside has art");
    let out = scratch("hillside");
    let done = oag_raceplay::export_host_textures(&options, &manifest.display().to_string(), &out)
        .expect("an export");
    let mut hosts = BTreeSet::new();
    for source in &art.textures {
        if let oag_trackfile::art::TextureSource::Host { entry, name } = source {
            hosts.insert((entry.clone(), name.clone()));
        }
    }
    assert!(!hosts.is_empty());
    for (entry, name) in &hosts {
        decode(&out.join(texture_file("wipeout-pulse", entry, name).expect("a safe path")));
    }
    assert!(
        done.skipped.is_empty(),
        "a host row of hillside did not resolve: {:?}",
        done.skipped
    );
    let _ = std::fs::remove_dir_all(&out);
}

/// One title's circuit: its texture list checked whole, its pictures written for a sample.
struct Circuit {
    /// What `oag_testdata::exact` opens: an image, or 2048's extracted package.
    source: &'static str,
    slug: &'static str,
    entry: &'static str,
    /// Fewer than this many names is a regression (the count the disc gave, less a margin).
    at_least: usize,
    /// One texture, spelled as the circuit names it.
    name: &'static str,
    size: (u32, u32),
    /// Pixels `(x, y)` of that texture, straight RGBA.
    pixels: &'static [((u32, u32), [u8; 4])],
    /// Write every texture, not a sample: only where that is cheap (a circuit of embedded
    /// pictures). A PS4 circuit is 1.5 GB written.
    whole: bool,
}

/// A texture path as a list spells it, for comparing names.
fn spelled(name: &str) -> String {
    name.trim_start_matches(['\\', '/'])
        .replace('\\', "/")
        .to_ascii_lowercase()
}

fn listed(source: &str, entry: &str) -> Option<(oag_raceplay::Options, Vec<String>)> {
    let source = oag_testdata::exact(source)?;
    let options = oag_raceplay::Options {
        source: source.display().to_string(),
        ..oag_raceplay::Options::default()
    };
    let names = oag_raceplay::list_host_textures(&options, entry)
        .unwrap_or_else(|e| panic!("{entry}: {e:#}"));
    Some((options, names))
}

fn export_one_circuit(tag: &str, circuit: &Circuit) {
    let Some((options, names)) = listed(circuit.source, circuit.entry) else {
        return;
    };
    assert!(
        names.len() >= circuit.at_least,
        "{tag}: {} names, expected at least {}",
        names.len(),
        circuit.at_least
    );
    assert!(
        names.iter().any(|n| spelled(n) == spelled(circuit.name)),
        "{tag}: {} is not in the list",
        circuit.name
    );
    let out = scratch(tag);
    let done = if circuit.whole {
        oag_raceplay::export_host_textures(&options, circuit.entry, &out)
    } else {
        let mut sample: BTreeSet<String> = names
            .iter()
            .step_by(names.len().div_ceil(11))
            .cloned()
            .collect();
        sample.insert(circuit.name.to_string());
        oag_raceplay::export_named_host_textures(&options, circuit.entry, &sample, &out)
    }
    .unwrap_or_else(|e| panic!("{tag}: {e:#}"));
    assert!(
        done.skipped.is_empty(),
        "{tag}: {} texture(s) left out, the first: {:?}",
        done.skipped.len(),
        done.skipped.first()
    );
    assert!(
        !circuit.whole || done.written.len() == names.len(),
        "{tag}: {} written of {} listed",
        done.written.len(),
        names.len()
    );
    assert!(
        done.written.len() >= 12,
        "{tag}: {} written",
        done.written.len()
    );
    let file = out.join(
        texture_file(circuit.slug, circuit.entry, circuit.name)
            .unwrap_or_else(|| panic!("{tag}: {} has no safe path", circuit.name)),
    );
    let picture = decode(&file);
    assert_eq!((picture.width, picture.height), circuit.size, "{tag}");
    for &((x, y), want) in circuit.pixels {
        let at = (y * picture.width + x) as usize * 4;
        assert_eq!(&picture.rgba[at..at + 4], &want, "{tag}: pixel ({x}, {y})");
    }
    assert_a_picture(&file);
    for file in done.written.iter().take(40) {
        decode(file);
    }
    let _ = std::fs::remove_dir_all(&out);
}

/// A circuit's list holds `wanted`: the textures only the race's water law asks for.
fn lists(source: &str, entry: &str, wanted: &[&str]) {
    let Some((_, names)) = listed(source, entry) else {
        return;
    };
    for want in wanted {
        assert!(
            names.iter().any(|n| spelled(n) == spelled(want)),
            "{entry}: {want} is not listed"
        );
    }
}

#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_writes_its_circuit_textures() {
    export_one_circuit(
        "pulse-ps2",
        &Circuit {
            source: "pulse-ps2-eu.chd",
            slug: "wipeout-pulse",
            entry: CIRCUIT,
            at_least: 150,
            name: "Wipeout PSP\\PS2\\Data\\Environments\\DirtyGenericTrackTextures\\_pitwall6_1.tga",
            size: (256, 128),
            pixels: &[
                ((64, 32), [115, 116, 132, 255]),
                ((64, 96), [140, 146, 164, 255]),
            ],
            whole: true,
        },
    );
}

#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn pure_writes_its_circuit_textures() {
    export_one_circuit(
        "pure",
        &Circuit {
            source: "pure-psp-eu.chd",
            slug: "wipeout-pure",
            entry: "Data\\Environments\\01_Vineta_K\\track.vex",
            at_least: 180,
            name: "Data\\Environments\\01_Vineta_K\\Textures\\ag03.tga",
            size: (64, 128),
            pixels: &[],
            whole: true,
        },
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_writes_the_pictures_its_circuit_materials_name() {
    export_one_circuit(
        "hd",
        &Circuit {
            source: "hdfury-ps3-eu-dec.iso",
            slug: "wipeout-hd",
            entry: CIRCUIT,
            at_least: 280,
            name: "/data/environments/03_track/hd_textures/dds/ds_speedup_cs.gtf",
            size: (512, 512),
            pixels: &[
                ((128, 128), [68, 69, 82, 55]),
                ((128, 384), [186, 187, 186, 238]),
            ],
            whole: false,
        },
    );
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn wipeout_2048_writes_the_pictures_its_circuit_materials_name() {
    export_one_circuit(
        "2048",
        &Circuit {
            source: "data/extracted/vita/PCSF00007",
            slug: "wipeout-2048",
            entry: "Data\\art\\published\\environments\\altima\\track.vex",
            at_least: 195,
            name: "data/art/published/environments/altima/textures/jreilly_crane_da.gxt",
            size: (128, 128),
            pixels: &[
                ((32, 32), [102, 65, 28, 143]),
                ((32, 96), [103, 70, 24, 153]),
            ],
            whole: false,
        },
    );
}

#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn omega_writes_the_pictures_its_circuit_materials_name() {
    export_one_circuit(
        "omega",
        &Circuit {
            source: "data/images/omega-ps4-eu.pkg",
            slug: "wipeout-omega-collection",
            entry: "Data\\environments\\tech_de_ra\\track.vex",
            at_least: 185,
            name: "data/environments/tech_de_ra/texturesps4/hull.gnf",
            size: (512, 512),
            pixels: &[
                ((128, 128), [57, 69, 83, 255]),
                ((128, 384), [30, 36, 44, 255]),
            ],
            whole: false,
        },
    );
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn wipeout_2048_lists_the_textures_its_seas_ask_for_under_the_races_water_law() {
    // A sea of `fc15`/`fc17_effects_vscroll_water` asks for its normal map only where water is
    // drawn, as it is in a race by default; Altima's blended edge hides this one.
    for (circuit, normal) in [
        ("mall", "waternormalmap.gxt"),
        ("tower", "waternormalmap.gxt"),
        ("square", "dc_waternormalmap.gxt"),
        ("park", "water_lapping_waves2_n.gxt"),
    ] {
        lists(
            "data/extracted/vita/PCSF00007",
            &format!("Data\\art\\published\\environments\\{circuit}\\track.vex"),
            &[&format!(
                "data/art/published/environments/{circuit}/textures/{normal}"
            )],
        );
    }
}

#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn omega_lists_its_2048_era_sea_normal_map() {
    // Omega's seas list the same files under either water law (checked on ten circuits); the
    // normal map is one of them.
    lists(
        "data/images/omega-ps4-eu.pkg",
        "Data\\environments2048\\mall\\track.vex",
        &["data/environments2048/mall/textures/dc_waternormalmap.gnf"],
    );
}
