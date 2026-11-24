//! A circuit's host textures written as PNG files for the Blender add-on, off a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs `data/images/pulse-psp-eu.chd`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay host_export_ground_truth:: --run-ignored all
//! ```
//!
//! What only real data can say: the names a track file spells (full asset paths with a
//! `.tga` the disc's own table carries) resolve to decodable pictures, each lands at the
//! path the add-on computes, and the files are pictures, not flat fills.

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

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-host-export-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
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
    let road = out.join(texture_file(
        "wipeout-pulse",
        CIRCUIT,
        "Data\\Environments\\GenericTrackTextures\\_surface6_1.tga",
    ));
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
        decode(&out.join(texture_file("wipeout-pulse", entry, name)));
    }
    assert!(
        done.skipped.is_empty(),
        "a host row of hillside did not resolve: {:?}",
        done.skipped
    );
    let _ = std::fs::remove_dir_all(&out);
}
