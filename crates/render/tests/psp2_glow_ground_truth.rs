//! Wipeout 2048's scrolling surfaces, read off each material's own uniforms.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run it with `just test-data`.
//!
//! `arena` ships three materials with HD's glow layer
//! (`Emissive_UV_Offset`/`Scale`, `TimeScaler`, `GlowTint`: the `fc09`/`fc10`
//! `VScroll_LambertAlpha_Emissive` pair) and two with a plain
//! `speed_multipliaer` scroll. The test holds what the build does with them:
//! the layer draws carry `ADD_SECOND` with their emissive texture bound beside
//! their diffuse and a table entry whose rate is the authored `TimeScaler`; the
//! scroll draws carry a track that moves `v` by the authored speed per second.
//! Remove `glow::plan`'s call and every assertion here fails.

use std::path::{Path, PathBuf};

use oag_mesh::mesh::rcs::psp2::{self, Animation};
use oag_mesh::mesh::{Model, slots};
use oag_mesh::mesh_render::TexAnims;

fn package() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007/base/PSP2/data.psarc");
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

fn build(circuit: &str) -> Option<(Model, psp2::Report)> {
    build_at(circuit, &format!("environments/{circuit}"))
}

fn build_at(circuit: &str, directory: &str) -> Option<(Model, psp2::Report)> {
    let package = package()?;
    let mut archive =
        oag_assets::psarc::Archive::open(package.to_str().expect("utf-8")).expect("opens");
    let base = format!("data/art/published/{directory}/track");
    let model = archive
        .read_path(&format!("{base}.rcsmodel"))
        .expect("model");
    let skeleton = archive.read_path(&format!("{base}.rcsskeleton")).ok();
    let clip = archive.read_path(&format!("{base}.rcsanimclip")).ok();
    let animation = skeleton
        .as_deref()
        .and_then(|s| Animation::parse(s, clip.as_deref()).ok());
    Some(
        psp2::build(circuit, &model, animation.as_ref(), &mut |p| {
            archive.read_path(p).ok()
        })
        .expect("builds"),
    )
}

/// The first vertex each non-empty draw's indices reach.
fn first_vertices(
    model: &Model,
) -> impl Iterator<Item = (&oag_mesh::mesh::DrawCall, oag_mesh::mesh::GpuVertex)> {
    model.draws.iter().filter(|d| !d.range.is_empty()).map(|d| {
        (
            d,
            model.vertices[model.indices[d.range.start as usize] as usize],
        )
    })
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn arenas_glow_layers_carry_their_authored_rates_and_a_bound_emissive_texture() {
    let Some((model, report)) = build("arena") else {
        return;
    };
    assert_eq!(report.glow_layers, 3, "{}", report.describe());
    let layered: Vec<_> = first_vertices(&model)
        .filter(|(_, v)| v.slots & slots::ADD_SECOND != 0)
        .collect();
    assert!(layered.len() > 40, "only {} layered draws", layered.len());
    for (draw, vertex) in &layered {
        let texture = draw.texture.expect("a layered draw is textured");
        assert!(
            model.lightmaps.get(texture).is_some_and(Option::is_some),
            "layered draw has no emissive texture bound"
        );
        let slot = (vertex.slots >> slots::MATERIAL_SHIFT) as usize;
        assert!((1..=model.emissive.len()).contains(&slot));
    }
    // `TimeScaler` as authored on the three: 0.05, 1.0 and 3.0.
    let mut rates: Vec<f32> = model.emissive.iter().map(|e| e.rate).collect();
    rates.sort_by(f32::total_cmp);
    rates.dedup();
    assert_eq!(rates, vec![0.05, 1.0, 3.0], "{:?}", model.emissive);
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn arenas_plain_scrolls_move_v_by_the_authored_speed() {
    let Some((model, report)) = build("arena") else {
        return;
    };
    assert_eq!(report.scrolling_materials, 2, "{}", report.describe());
    let scrolling: Vec<_> = first_vertices(&model)
        .filter(|(_, v)| v.anim != 0)
        .collect();
    assert!(!scrolling.is_empty());
    let at_zero = TexAnims::sample(&model, 0.0);
    let at_one = TexAnims::sample(&model, 1.0);
    for (_, vertex) in scrolling {
        let slot = vertex.anim as usize;
        assert_eq!(at_zero.transform[slot], [1.0, 1.0, 0.0, 0.0]);
        let moved = at_one.transform[slot];
        assert_eq!(&moved[..3], &[1.0, 1.0, 0.0]);
        // `speed_multipliaer` is 0.3 on both of arena's scrolling materials.
        assert!((moved[3] - 0.3).abs() < 1e-5, "{moved:?}");
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn altima_scrolls_its_plain_materials_and_draws_the_rest_still() {
    let Some((model, report)) = build("altima") else {
        return;
    };
    assert_eq!(report.glow_layers, 0, "{}", report.describe());
    assert_eq!(report.scrolling_materials, 12, "{}", report.describe());
    assert!(model.emissive.is_empty());
}

/// Anulpha Pass is an HD circuit ported into 2048's DLC1: its
/// `mt_uvanim_diffuse_emissive` and `cf_uvanim_emssive*` records carry HD's own
/// values (`docs/formats/2048-material-params.md`, "Inherited from HD"). Its
/// glow layers author neither `TimeScaler` nor `time` and play HD's engine
/// clock, and 18 materials scroll off HD's vertex law by the rate hashes they
/// author, each confirmed by a readable vertex program (`inherited_unread` is
/// 0). Remove `inherited_rate`'s call and the count assertion fails. The glow
/// rate rule itself is pinned by `vineta_ks_authored_time_is_not_its_rate`,
/// because Anulpha's layers would read `1.0` under the old rule too.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn anulpha_pass_inherits_hds_clock_and_vertex_scroll() {
    let Some((model, report)) = build_at("anulpha", "DLC1/environments/Anulpha_Pass") else {
        return;
    };
    assert_eq!(report.glow_layers, 35, "{}", report.describe());
    assert!(
        model.emissive.iter().all(|e| e.rate == 1.0),
        "{:?}",
        model.emissive
    );
    assert_eq!(report.inherited_scrolls, 18, "{}", report.describe());
    assert!(
        report.describe().contains("inherited from HD"),
        "{}",
        report.describe()
    );
    let moving = first_vertices(&model).filter(|(_, v)| v.anim != 0).count();
    assert!(moving > 0, "no draw carries a scroll track");
    let at_one = TexAnims::sample(&model, 1.0);
    let slid = first_vertices(&model)
        .filter(|(_, v)| v.anim != 0)
        .any(|(_, v)| {
            let t = at_one.transform[v.anim as usize];
            t[2] != 0.0 || t[3] != 0.0
        });
    assert!(
        slid,
        "no scroll track moved the coordinate after one second"
    );
}

/// Omega ships the same HD circuits in `Data/environments/<n>_<name>`; its
/// materials read through the same plan, so Anulpha Pass inherits the same
/// two things there: the engine clock on its glow layers and HD's vertex law on
/// the families HD names (`docs/formats/omega-status.md`). Omega's programs are
/// GCN and unread, so the name and the authored rate hashes alone admit a
/// scroll there. Counts are the census of
/// `crates/render/examples/psp2_scroll_reach.rs`.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omegas_anulpha_pass_inherits_hds_clock_and_vertex_scroll() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/extracted/ps4");
    let entry = "Data/environments/15_anulpha_pass/track.final.rcsmodel";
    let mut found = None;
    for (dir, names) in [
        (
            "omega-eu",
            ["data00", "data01", "data02", "data03", "data04"],
        ),
        (
            "omega-eu-patch",
            ["data05", "data07", "data08", "data09", "data09"],
        ),
    ] {
        for name in names {
            let path = root.join(dir).join("uroot").join(format!("{name}.psarc"));
            if let Ok(mut archive) = oag_assets::psarc::Archive::open_file(&path)
                && archive.read_path(entry).is_ok()
            {
                found = Some(archive);
            }
        }
    }
    let Some(mut archive) = found else {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but Omega's Anulpha Pass is missing"
        );
        println!("skipping: Omega not present");
        return;
    };
    let blob = archive.read_path(entry).expect("model");
    let (model, report) = psp2::build("omega anulpha", &blob, None, &mut |p| {
        archive.read_path(p).ok()
    })
    .expect("builds");
    assert_eq!(report.glow_layers, 36, "{}", report.describe());
    assert!(
        model.emissive.iter().all(|e| e.rate == 1.0),
        "{:?}",
        model.emissive
    );
    assert_eq!(report.inherited_scrolls, 11, "{}", report.describe());
    // Omega's programs are GCN: every admission is by name and hashes alone.
    assert_eq!(report.inherited_unread, 11, "{}", report.describe());
}

/// 2048's Vineta K authors `time` on its `mt_uvanim_diffuse_emissive2` and
/// `and_anim_spec` records (`0.0`, `1.0`, `1.0144`, `1.1448`) and no
/// `TimeScaler`; HD's identical records author no `time`. The rule that read the
/// authored `time` as the rate gave `{0.0, 1.0, 1.0144, 1.1448}`; HD's engine
/// clock gives `{1.0}`.
fn distinct_rates(model: &Model) -> Vec<f32> {
    let mut rates: Vec<f32> = model.emissive.iter().map(|e| e.rate).collect();
    rates.sort_by(f32::total_cmp);
    rates.dedup();
    rates
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn vineta_ks_authored_time_is_not_its_rate() {
    let Some((model, report)) = build_at("vineta", "DLC1/environments/Vineta_K") else {
        return;
    };
    assert!(report.glow_layers >= 10, "{}", report.describe());
    assert_eq!(distinct_rates(&model), vec![1.0], "{:?}", model.emissive);
}
