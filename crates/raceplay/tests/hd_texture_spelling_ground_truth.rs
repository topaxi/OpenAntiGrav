//! Every texture name Wipeout HD's own tables reference, resolved on HD, on
//! 2048 and on Omega through `oag_assets::Archives::resolve_texture`.
//!
//! The names are the ones the engine reads for HD: the HUD's twelve atlases and
//! every layout reference, the campaign and end-of-race art, the loading
//! screen's chrome, features and backdrops, the menu blocks, the Zone sky and
//! both fifteen-stage sets, the wing-tip ribbon and the engine flare sprite.
//! 2048 and Omega re-ship HD's circuits and front end in their own container
//! (`docs/formats/omega-status.md`), so a name HD reads is a name they should
//! hold under their platform's spelling; the lists below are the ones that do
//! not, checked against the packages rather than assumed.
//!
//! **The miss lists are asserted exactly**, so the test fails both when a
//! resolver change loses a texture (the list grows) and when a title gains one
//! (the list shrinks and the claim in `docs/formats/omega-status.md` moves).
//! Zone's stage textures and sky are `None` on 2048 and Omega by design
//! (`zone_stage_textures`/`zone_sky` are HD-only; lane P11 `hd-zone-stage` owns
//! wiring them), so they are listed here and nowhere wired.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^hd_texture_spelling_ground_truth::/)'
//! ```

use std::collections::BTreeSet;

fn hd_names() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut add = |name: &str| names.push(name.to_string());
    for name in oag_hd::hud::TEXTURES {
        add(name);
    }
    for (_, path) in oag_hd::campaign::HEX_TEXTURES {
        add(path);
    }
    for name in oag_hd::campaign::OTHER_TEXTURES {
        add(name);
    }
    for name in oag_hd::endrace::EXTRA_TEXTURES {
        add(name);
    }
    for (_, name) in oag_hd::loading::CHROME {
        add(name);
    }
    add(oag_hd::loading::BACKDROP);
    for (_, name) in oag_hd::loading::BACKDROP_VARIANTS {
        add(name);
    }
    if let Some(loading) = oag_hd::TITLE.loading {
        for style in loading.features {
            for feature in style.features {
                add(feature.image);
            }
        }
    }
    for name in [
        oag_hd::frontend::names::MENU_BLOCK_FRAME,
        oag_hd::frontend::names::MENU_BLOCK_FRAME_PSN,
        oag_hd::frontend::names::MENU_STRIP_CURSOR,
        oag_hd::frontend::names::MENU_LIST_ARROW,
    ] {
        add(name);
    }
    add(oag_hd::race::ZONE_SKY);
    let stages = oag_hd::race::ZONE_STAGE_TEXTURES;
    for stage in stages.stages() {
        add(&stages.general_entry(stage));
        add(&stages.track_entry(stage));
    }
    if let Some(wingtip) = oag_hd::TITLE.wingtip {
        add(wingtip.texture);
    }
    if let oag_title::flare::Flare::PerTeam(authored) = oag_hd::TITLE.flare {
        add(authored.sprite);
    }
    names.sort();
    names.dedup();
    names
}

fn misses(archives: &oag_assets::Archives, names: &[String]) -> BTreeSet<String> {
    names
        .iter()
        .filter(|name| archives.resolve_texture(name).is_none())
        .cloned()
        .collect()
}

fn hd() -> Option<oag_assets::Archives> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    Some(oag_hd::open(&image.display().to_string()).expect("HD opens"))
}

#[test]
#[ignore = "needs the decrypted PS3 image in data/images/"]
fn hd_holds_every_name_its_own_tables_reference() {
    let Some(archives) = hd() else { return };
    let names = hd_names();
    assert_eq!(names.len(), 85, "the names the census walks");
    let missing: Vec<String> = misses(&archives, &names).into_iter().collect();
    assert_eq!(missing, Vec::<String>::new(), "HD's own tables");
}

/// The names HD references that 2048 holds under no spelling.
///
/// Twelve are HD's loading-screen advert stills (`Data\FE\Ads`), art 2048 never
/// shipped; the thirteenth is the campaign's `Hexmedal_HD`. Every Zone texture
/// and the sky **do** resolve, so wiring them (lane `hd-zone-stage`) needs no
/// new asset search.
const MISSING_ON_2048: &[&str] = &[
    r"Data\FE\Ads\LSAD_FURY_16x9_01.gtf",
    r"Data\FE\Ads\LSAD_FURY_16x9_02.gtf",
    r"Data\FE\Ads\LSAD_FURY_16x9_03.gtf",
    r"Data\FE\Ads\LSAD_FURY_4x3_01.gtf",
    r"Data\FE\Ads\LSAD_FURY_4x3_02.gtf",
    r"Data\FE\Ads\LSAD_FURY_4x3_03.gtf",
    r"Data\FE\Ads\LSAD_HD_16x9_01.gtf",
    r"Data\FE\Ads\LSAD_HD_16x9_02.gtf",
    r"Data\FE\Ads\LSAD_HD_16x9_03.gtf",
    r"Data\FE\Ads\LSAD_HD_4x3_01.gtf",
    r"Data\FE\Ads\LSAD_HD_4x3_02.gtf",
    r"Data\FE\Ads\LSAD_HD_4x3_03.gtf",
    r"Data\FE\Images\Hexmedal_HD.gtf",
];

/// The names HD references that Omega holds under no spelling.
///
/// Only the Zone race's "general" stage set: fifteen entries of a flat white
/// (`oag_title::ZoneStageTextures::general_entry`). The "track" set that carries
/// the art and the sky resolve.
const MISSING_ON_OMEGA: &[&str] = &[
    "/data/tex/zonemode0.gtf",
    "/data/tex/zonemode1.gtf",
    "/data/tex/zonemode10.gtf",
    "/data/tex/zonemode11.gtf",
    "/data/tex/zonemode12.gtf",
    "/data/tex/zonemode13.gtf",
    "/data/tex/zonemode14.gtf",
    "/data/tex/zonemode2.gtf",
    "/data/tex/zonemode3.gtf",
    "/data/tex/zonemode4.gtf",
    "/data/tex/zonemode5.gtf",
    "/data/tex/zonemode6.gtf",
    "/data/tex/zonemode7.gtf",
    "/data/tex/zonemode8.gtf",
    "/data/tex/zonemode9.gtf",
];

fn check(label: &str, archives: &oag_assets::Archives, want: &[&str]) {
    let names = hd_names();
    let got = misses(archives, &names);
    let want: BTreeSet<String> = want.iter().map(|name| (*name).to_string()).collect();
    eprintln!(
        "{label}: {} of {} HD texture names resolve; {} do not:",
        names.len() - got.len(),
        names.len(),
        got.len()
    );
    for name in &got {
        eprintln!("  {name}");
    }
    assert_eq!(
        got, want,
        "{label}: the HD names it holds under no spelling"
    );
}

#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita/"]
fn wipeout_2048_resolves_the_hd_names_it_ships() {
    let Some(dir) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let archives = oag_2048::open(&dir.display().to_string()).expect("2048 opens");
    check("2048", &archives, MISSING_ON_2048);
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn omega_resolves_the_hd_names_it_ships() {
    let Some(dir) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    let archives = oag_omega::open(&dir.display().to_string()).expect("Omega opens");
    check("Omega", &archives, MISSING_ON_OMEGA);
}
