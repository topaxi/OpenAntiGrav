//! How much vertex and index data each shipped circuit asks a device to hold in one buffer,
//! and that every one of them is drawable on a device whose `max_buffer_size` is wgpu's
//! default 256 MiB (`oag_mesh::mesh_render::chunks`).
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. Run it with `just test-data`, or one title at a time:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-mesh --run-ignored all \
//!     -E 'test(/^vertex_buffer_census_ground_truth::/)' --no-capture
//! ```
//!
//! Each circuit is built with no textures (a vertex count does not depend on them) and its
//! plan read off `VertexChunks::plan` at 256 MiB. Remove the planner and Omega's Rockway
//! Stadium and Unity Square are one buffer of 294 and 296 MiB again; the first two tests
//! then fail.

use std::path::{Path, PathBuf};

use oag_mesh::mesh::{self, GpuVertex, Model};
use oag_mesh::mesh_render::chunks::VertexChunks;

/// wgpu's default `max_buffer_size`, which a Steam Deck, a phone and WebGPU all report.
const DEFAULT_MAX_BUFFER: u64 = 256 << 20;

const MIB: f64 = (1 << 20) as f64;

struct Row {
    circuit: String,
    vertices: usize,
    vertex_mib: f64,
    index_mib: f64,
    chunks: usize,
    largest_chunk_mib: f64,
}

fn row(circuit: &str, model: &Model) -> Row {
    let chunks =
        VertexChunks::plan(model, DEFAULT_MAX_BUFFER).unwrap_or_else(|e| panic!("{circuit}: {e}"));
    let stride = std::mem::size_of::<GpuVertex>();
    let largest = chunks.ranges().iter().map(|r| r.len()).max().unwrap_or(0);
    for draw in model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
    {
        let span = &model.indices[draw.range.start as usize..draw.range.end as usize];
        let Some(&first) = span.first() else { continue };
        let range = chunks.range(chunks.chunk_of(first));
        assert!(
            span.iter().all(|v| range.contains(v)),
            "{circuit}: a draw straddles the cut at {range:?}"
        );
    }
    Row {
        circuit: circuit.to_string(),
        vertices: model.vertices.len(),
        vertex_mib: (model.vertices.len() * stride) as f64 / MIB,
        index_mib: (model.indices.len() * 4) as f64 / MIB,
        chunks: chunks.len(),
        largest_chunk_mib: (largest * stride) as f64 / MIB,
    }
}

/// Prints the table and returns the largest vertex buffer a circuit needs, in MiB.
fn report(title: &str, mut rows: Vec<Row>) -> Vec<Row> {
    rows.sort_by(|a, b| b.vertex_mib.total_cmp(&a.vertex_mib));
    println!("{title}: {} circuit(s)", rows.len());
    for r in &rows {
        println!(
            "  {:<48} {:>9} vertices {:>7.1} MiB vertices {:>6.1} MiB indices -> {} buffer(s), largest {:.1} MiB",
            r.circuit, r.vertices, r.vertex_mib, r.index_mib, r.chunks, r.largest_chunk_mib
        );
    }
    for r in &rows {
        assert!(
            r.largest_chunk_mib * MIB <= DEFAULT_MAX_BUFFER as f64,
            "{}: a vertex buffer over the ceiling",
            r.circuit
        );
        assert!(
            r.index_mib * MIB <= DEFAULT_MAX_BUFFER as f64,
            "{}: {:.1} MiB of indices over the ceiling",
            r.circuit,
            r.index_mib
        );
    }
    rows
}

fn psp2_rows(archive: &mut oag_assets::psarc::Archive, wanted: impl Fn(&str) -> bool) -> Vec<Row> {
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| wanted(&p.to_ascii_lowercase()))
        .cloned()
        .collect();
    paths
        .iter()
        .map(|path| {
            let blob = archive.read_path(path).expect("the model reads");
            let (model, _) = mesh::rcs::psp2::build(path, &blob, None, &mut |_| None)
                .unwrap_or_else(|e| panic!("{path}: {e:#}"));
            row(path, &model)
        })
        .collect()
}

fn needs(path: &Path) -> Option<PathBuf> {
    if path.exists() {
        return Some(path.to_path_buf());
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn is_circuit(path: &str) -> bool {
    path.ends_with("/track.rcsmodel") || path.ends_with("/track.final.rcsmodel")
}

/// Omega's two largest 2048-mode circuits are the only ones over 256 MiB, and they plan into
/// two buffers each.
#[test]
#[ignore = "needs data/extracted/ps4"]
fn omega_arena_and_square_are_the_circuits_that_outgrow_a_default_device() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted/ps4/omega-eu/uroot");
    let Some(root) = needs(&root) else { return };
    let mut rows = Vec::new();
    for archive in [
        "data00.psarc",
        "data01.psarc",
        "data02.psarc",
        "data03.psarc",
        "data04.psarc",
    ] {
        let Ok(mut archive) = oag_assets::psarc::Archive::open_file(&root.join(archive)) else {
            continue;
        };
        rows.extend(psp2_rows(&mut archive, is_circuit));
    }
    let rows = report("Wipeout Omega Collection", rows);
    assert!(rows.len() >= 26, "{} circuits", rows.len());
    let cut: Vec<&str> = rows
        .iter()
        .filter(|r| r.chunks > 1)
        .map(|r| r.circuit.as_str())
        .collect();
    assert_eq!(cut.len(), 2, "{cut:?}");
    assert!(
        cut.iter()
            .any(|c| c.to_ascii_lowercase().contains("/arena/")),
        "{cut:?}"
    );
    assert!(
        cut.iter()
            .any(|c| c.to_ascii_lowercase().contains("/square/")),
        "{cut:?}"
    );
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn every_wipeout_2048_circuit_is_one_buffer() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
    let Some(path) = needs(&path) else { return };
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("opens");
    let rows = report("Wipeout 2048", psp2_rows(&mut archive, is_circuit));
    assert!(rows.len() >= 14, "{} circuits", rows.len());
    assert!(rows.iter().all(|r| r.chunks == 1));
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_wipeout_hd_circuit_is_one_buffer() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let mut rows = Vec::new();
    for n in 0..7 {
        let spec = format!("{}:PS3_GAME/USRDIR/DATA0{n}.PSARC", image.display());
        let mut archive = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let tracks: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with("/track.vex"))
            .cloned()
            .collect();
        for path in tracks {
            let data = archive.read_path(&path).expect("the .vex reads");
            let Some(geometry) = mesh::rcs::sibling_geometry(&spec, &path, &data) else {
                continue;
            };
            let Ok((model, _)) = mesh::rcs::build_scene(&path, &data, &geometry, &mut |_| None)
            else {
                continue;
            };
            rows.push(row(&path, &model));
        }
    }
    let rows = report("Wipeout HD Fury", rows);
    assert!(rows.len() >= 16, "{} circuits", rows.len());
    assert!(rows.iter().all(|r| r.chunks == 1));
}

fn psp_rows(image: &str, title: &str) -> Vec<Row> {
    let Some(image) = oag_testdata::image(image) else {
        return Vec::new();
    };
    let mut archive =
        oag_assets::Archive::open(&format!("{}:PSP_GAME/USRDIR/Data.wad", image.display()))
            .expect("opening the archive");
    // A WAD names its entries by hash, so the circuits are found by trying every directory
    // spelling: Pulse's `NN_Track`, and Pure's and the later titles' `NN_Name`.
    const NAMES: [&str; 22] = [
        "Track",
        "Vineta_K",
        "Metropia",
        "Moa_Therma",
        "Chenghou_Project",
        "Mandrake",
        "Sebenco_Climb",
        "Anulpha_Pass",
        "Amphiseum",
        "Sol",
        "Sol_2",
        "Talons_Junction",
        "Modesto_Heights",
        "Tech_De_Ra",
        "Ubermall",
        "Manor_Top",
        "Qirex",
        "Terminal",
        "Altima",
        "Citadel",
        "Temtesh_Bay",
        "Karbonis",
    ];
    let mut rows: Vec<Row> = Vec::new();
    for number in 0..64 {
        for stem in NAMES {
            let name = format!(r"Data\Environments\{number:02}_{stem}\track.vex");
            if !archive.contains(&name) || rows.iter().any(|r| r.circuit == name) {
                continue;
            }
            let blob = archive.read_name(&name).expect("the .vex reads");
            if let Ok(model) = mesh::build(&name, &blob) {
                rows.push(row(&name, &model));
            }
        }
    }
    println!("{title}: {} circuit(s) found by name", rows.len());
    rows
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn the_pulse_circuits_found_by_name_are_one_buffer() {
    let rows = report("Wipeout Pulse", psp_rows("pulse-psp-eu.chd", "Pulse"));
    assert!(rows.len() >= 12, "{} circuits", rows.len());
    assert!(rows.iter().all(|r| r.chunks == 1));
}

#[test]
#[ignore = "needs data/images/pure-psp-eu.chd"]
fn the_pure_circuits_found_by_name_are_one_buffer() {
    let rows = report("Wipeout Pure", psp_rows("pure-psp-eu.chd", "Pure"));
    assert!(rows.len() >= 5, "{} circuits", rows.len());
    assert!(rows.iter().all(|r| r.chunks == 1));
}

/// **The panic itself.** Rockway Stadium and Unity Square built through `build_with` on a
/// device holding wgpu's default 256 MiB buffer ceiling, which is what the race load does.
/// Before the vertex buffers were cut, `create_buffer` panicked here with "Buffer size
/// 308016200 is greater than the maximum buffer size (268435456)" and "310295700".
#[test]
#[ignore = "needs data/extracted/ps4 and a GPU"]
fn omega_arena_and_square_build_on_a_device_with_a_256_mib_buffer_ceiling() {
    use oag_mesh::mesh_render::{self, Anisotropy};

    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted/ps4/omega-eu/uroot");
    let Some(root) = needs(&root) else { return };
    let instance = wgpu::Instance::default();
    let Some(adapter) = oag_gpu::test_adapter::request(&instance) else {
        println!("skipping: no GPU adapter");
        return;
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: wgpu::Limits {
            max_buffer_size: DEFAULT_MAX_BUFFER,
            ..wgpu::Limits::default()
        },
        ..Default::default()
    }))
    .expect("a device");
    assert_eq!(device.limits().max_buffer_size, DEFAULT_MAX_BUFFER);
    let mut built_circuits = 0;
    for archive in [
        "data00.psarc",
        "data01.psarc",
        "data02.psarc",
        "data03.psarc",
        "data04.psarc",
    ] {
        let Ok(mut archive) = oag_assets::psarc::Archive::open_file(&root.join(archive)) else {
            continue;
        };
        let paths: Vec<String> = archive
            .paths()
            .iter()
            .filter(|p| {
                let p = p.to_ascii_lowercase();
                p.ends_with("/arena/track.final.rcsmodel")
                    || p.ends_with("/square/track.final.rcsmodel")
            })
            .cloned()
            .collect();
        for path in paths {
            let blob = archive.read_path(&path).expect("the model reads");
            let (model, _) = mesh::rcs::psp2::build(&path, &blob, None, &mut |_| None)
                .unwrap_or_else(|e| panic!("{path}: {e:#}"));
            assert!(
                model.vertices.len() * std::mem::size_of::<GpuVertex>()
                    > DEFAULT_MAX_BUFFER as usize,
                "{path} fits one buffer"
            );
            let built = mesh_render::build_with(
                &device,
                &queue,
                &model,
                wgpu::TextureFormat::Rgba8Unorm,
                oag_gpu::formats::SceneStore::Computed,
                Anisotropy::Off,
                1,
                mesh_render::Depth::Scene,
                mesh_render::TRANSPARENT_BLEND,
                mesh_render::GlowMask::Protected,
                mesh_render::Velocity::Write,
                &mesh_render::zone::StageArt::NONE,
                mesh_render::ShadowMaps::NONE,
                mesh_render::ShadowReceiver::Never,
                mesh_render::Texcoords::Interleaved,
                false,
                false,
                false,
                false,
            )
            .unwrap_or_else(|e| panic!("{path}: {e:#}"));
            assert_eq!(built.geometry.buffer_count(), 2, "{path}");
            assert!(built.geometry.largest_buffer() <= DEFAULT_MAX_BUFFER);
            built_circuits += 1;
        }
    }
    assert_eq!(built_circuits, 2, "arena and square");
}
