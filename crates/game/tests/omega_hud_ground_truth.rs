//! Omega's in-race HUD: five root layouts and nine `.gnf` atlases, read off the
//! PS4 package pair.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted PS4 packages in
//! `data/extracted/ps4/`, and the cross-checks need the decrypted PS3 image.
//! No PS4 emulator exists here, so the reference for a picture is HD's own
//! `.gtf` of the same stem, not a capture of the running game.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(omega_hud_ground_truth)'
//! ```
//!
//! What this fails on: pointing `oag_omega::hud` back at the unread placeholder
//! (the layouts then stop composing to the counts below), dropping
//! `texture_extension` (the `.gtf`/`.mip` spellings the XML authors never
//! resolve on this disc), or reversing the rows of a HUD atlas (they are
//! top-down, unlike the front end's eight `bottom_up_gnf` images).

use std::collections::BTreeSet;
use std::path::PathBuf;

fn omega() -> Option<PathBuf> {
    oag_testdata::exact("data/extracted/ps4")
}

fn open() -> Option<oag_assets::Archives> {
    let source = omega()?;
    Some(oag_omega::open(&source.display().to_string()).expect("Omega opens"))
}

/// `(root, sprites, files)` for the five modes, in the order [`modes`] lists
/// them. Measured 2026-10-06.
///
/// HD's own file counts are 17/15/14/15/5, so the include trees are the same
/// shape; the sprite counts differ because Omega's XML is its own (Arcade 148
/// against HD's 138).
const EXPECTED: &[(&str, usize, usize)] = &[
    (r"Data\XML\Arcade_HUD.xml", 148, 17),
    (r"Data\XML\Elimination_HUD.xml", 132, 15),
    (r"Data\XML\TimeTrial_HUD.xml", 52, 14),
    (r"Data\XML\SpeedLap_HUD.xml", 51, 15),
    (r"Data\XML\Zone_HUD.xml", 36, 5),
];

fn modes() -> [&'static str; 5] {
    let h = oag_omega::TITLE.hud;
    [h.arcade, h.elimination, h.time_trial, h.speed_lap, h.zone]
}

fn composed(archives: &mut oag_assets::Archives, root: &str) -> oag_hud::Composed {
    let mut read = |path: &str| archives.read_name(path).ok();
    oag_hud::compose(root, &mut read).unwrap_or_else(|| panic!("composing {root}"))
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn every_root_composes_with_nothing_missing_and_nothing_skipped() {
    let Some(mut archives) = open() else { return };
    assert_eq!(
        modes().to_vec(),
        EXPECTED.iter().map(|e| e.0).collect::<Vec<_>>(),
        "the title data names the five roots the executable's race managers read"
    );
    for (root, sprites, files) in EXPECTED {
        let c = composed(&mut archives, root);
        assert!(c.missing.is_empty(), "{root}: missing {:?}", c.missing);
        assert!(
            c.layout.skipped.is_empty(),
            "{root}: skipped {:?}",
            c.layout.skipped
        );
        assert_eq!(c.files.len(), *files, "{root}: files");
        assert_eq!(c.layout.sprites.len(), *sprites, "{root}: sprites");
    }
}

/// The distinct atlas references the five roots name, in the XML's spelling.
fn references(archives: &mut oag_assets::Archives) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for root in modes() {
        for r in composed(archives, root).layout.textures() {
            out.insert(r.to_string());
        }
    }
    out
}

/// `(reference stem, width, height)` of every atlas the five roots name.
const ATLASES: &[(&str, u32, u32)] = &[
    ("HUD_Components", 1024, 1024),
    ("HUD_Components_01", 1024, 512),
    ("HUD_Components_02", 128, 128),
    ("ZoneDamage", 1024, 256),
    ("fury_hud", 2048, 2048),
    ("hdHUD", 1024, 1024),
    ("missile_reticule", 256, 256),
    ("nitro_hud", 1024, 1024),
    ("voiceCom", 64, 64),
];

fn stem(reference: &str) -> &str {
    let leaf = reference.rsplit(['\\', '/']).next().unwrap_or(reference);
    leaf.rsplit_once('.').map_or(leaf, |(s, _)| s)
}

fn resolved(archives: &mut oag_assets::Archives, reference: &str) -> Option<Vec<u8>> {
    let extension = oag_omega::TITLE.hud_art.texture_extension?;
    let rewritten = oag_title::hud::replace_extension(reference, extension);
    archives.read_name(&rewritten).ok()
}

#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn every_atlas_a_root_names_resolves_as_gnf_and_decodes() {
    let Some(mut archives) = open() else { return };
    let refs = references(&mut archives);
    let got: BTreeSet<&str> = refs.iter().map(|r| stem(r)).collect();
    let want: BTreeSet<&str> = ATLASES.iter().map(|a| a.0).collect();
    assert_eq!(got, want, "the atlases the five roots name");

    for reference in &refs {
        assert!(
            archives.read_name(reference).is_err(),
            "{reference}: the authored spelling resolves, so the rewrite is not what finds it"
        );
        let blob = resolved(&mut archives, reference)
            .unwrap_or_else(|| panic!("{reference} does not resolve as .gnf"));
        let texture =
            oag_texture::gnf::Texture::parse(&blob).unwrap_or_else(|e| panic!("{reference}: {e}"));
        let (_, w, h) = ATLASES
            .iter()
            .find(|a| a.0 == stem(reference))
            .copied()
            .expect("listed");
        assert_eq!(
            (texture.width, texture.height),
            (w, h),
            "{reference}: dimensions"
        );
        assert_eq!(
            texture.surface_format,
            oag_texture::gnf::SurfaceFormat::Bc7,
            "{reference}: format"
        );
        let pixels = texture
            .decode(&blob)
            .unwrap_or_else(|e| panic!("{reference}: {e}"));
        assert_eq!(pixels.len(), (w * h) as usize, "{reference}: pixel count");
    }
}

fn mean_abs_diff(a: &[[u8; 4]], b: &[[u8; 4]], width: usize, reverse_b: bool) -> f64 {
    let height = a.len() / width;
    let mut sum = 0u64;
    for y in 0..height {
        let by = if reverse_b { height - 1 - y } else { y };
        for x in 0..width {
            for c in 0..4 {
                sum += u64::from(a[y * width + x][c].abs_diff(b[by * width + x][c]));
            }
        }
    }
    sum as f64 / (width * height * 4) as f64
}

/// A HUD atlas decodes top-down straight out of `gnf::Texture::decode`, which
/// is HD's `.gtf` as `oag_hud::sprite` draws it (reversed from the file).
#[test]
#[ignore = "needs the decrypted PS4 package pair and the decrypted HD image"]
fn hud_atlases_are_hds_picture_in_the_row_order_the_sheet_draws() {
    let (Some(mut archives), Some(hd_image)) = (
        open(),
        oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso"),
    ) else {
        return;
    };
    let mut hd = oag_hd::open(&hd_image.display().to_string()).expect("HD opens");
    let refs = references(&mut archives);
    let mut compared = 0;
    for reference in &refs {
        let name = stem(reference);
        // `fury_hud` is 2048 here against HD's 1024, `voiceCom` is not an HD
        // `.gtf` of the same size, and `HUD_Components_02` and `ZoneDamage` are
        // near-symmetric (unreversed diffs 0.93 and 0.77), so no order is told
        // from the other by them.
        if ["fury_hud", "voiceCom", "HUD_Components_02", "ZoneDamage"].contains(&name) {
            continue;
        }
        let blob = resolved(&mut archives, reference).expect("resolves");
        let texture = oag_texture::gnf::Texture::parse(&blob).expect("parses");
        let ours = texture.decode(&blob).expect("decodes");
        let hd_blob = hd
            .read_name(&oag_hd::hud::texture_entry(reference))
            .unwrap_or_else(|e| panic!("HD {reference}: {e}"));
        let gtf = oag_texture::gtf::Gtf::parse(&hd_blob).expect("HD parses");
        let theirs = gtf
            .only()
            .expect("one texture")
            .to_rgba(&hd_blob)
            .expect("HD decodes");
        assert_eq!(ours.len(), theirs.len(), "{reference}: same size as HD's");
        let width = texture.width as usize;
        let top_down = mean_abs_diff(&ours, &theirs, width, true);
        let as_filed = mean_abs_diff(&ours, &theirs, width, false);
        // `hdHUD` is the one atlas whose art Omega redrew in part (mean
        // difference 17.06 reversed, 80.59 as filed); the rest are HD's pixels.
        let bound = if name == "hdHUD" { 20.0 } else { 1.0 };
        assert!(top_down < bound, "{reference}: reversed diff {top_down}");
        assert!(
            as_filed > 4.0 * top_down.max(1.0),
            "{reference}: unreversed diff {as_filed} - the two orders must be told apart"
        );
        compared += 1;
    }
    assert_eq!(compared, 5, "atlases compared against HD's");
}

/// The widget names title data carries from HD's frame readings exist in
/// Omega's own layouts - so the inheritance is a name that matches, not a name
/// that matches nothing.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_widget_names_title_data_inherits_are_authored_by_omegas_layouts() {
    let Some(mut archives) = open() else { return };
    let mut names = BTreeSet::new();
    for root in modes() {
        let c = composed(&mut archives, root);
        names.extend(c.layout.sprites.iter().map(|s| s.name.clone()));
        names.extend(c.layout.labels.iter().map(|l| l.name.clone()));
    }
    let art = oag_omega::TITLE.hud_art;
    let runtime = art.runtime.expect("runtime readouts");
    let mut wanted: Vec<&str> = vec![
        runtime.shield.fill,
        runtime.shield.background,
        runtime.shield.text,
    ];
    if let oag_title::hud::Sights::Concentric {
        seeking, locked, ..
    } = art.sights
    {
        wanted.extend(seeking.iter());
        wanted.extend(locked.iter());
    }
    let missing: Vec<&&str> = wanted.iter().filter(|n| !names.contains(**n)).collect();
    assert!(missing.is_empty(), "not authored by Omega: {missing:?}");

    let absent: Vec<&&str> = art
        .always_on
        .iter()
        .filter(|n| !names.contains(**n))
        .collect();
    assert!(
        absent.is_empty(),
        "always_on names Omega's layouts never author: {absent:?}"
    );
}

/// Omega's HUD faces are HD's at twice the pixel size, which is what the
/// `texel_scale` the HUD font now takes assumes.
#[test]
#[ignore = "needs the decrypted PS4 package pair and the decrypted HD image"]
fn omegas_hud_faces_are_hds_at_twice_the_pixel_size() {
    let (Some(mut o), Some(hd_image)) = (
        open(),
        oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso"),
    ) else {
        return;
    };
    let mut h = oag_hd::open(&hd_image.display().to_string()).expect("HD opens");
    for name in ["PulseHud.fnt", "small.fnt"] {
        let path = format!(r"Data\FE\Fonts\{name}");
        let (fo, fh) = (
            o.read_font(&path).expect("Omega"),
            h.read_font(&path).expect("HD"),
        );
        assert_eq!(
            fo.line_height as f32 / fh.line_height as f32,
            2.0,
            "{name}: line height"
        );
        let mut advances = Vec::new();
        for g in &fo.glyphs {
            if let Some(x) = fh.glyphs.iter().find(|x| x.codepoint == g.codepoint)
                && x.advance > 0
            {
                advances.push(f32::from(g.advance) / f32::from(x.advance));
            }
        }
        advances.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        assert!(
            advances.len() > 80,
            "{name}: {} shared glyphs",
            advances.len()
        );
        let median = advances[advances.len() / 2];
        assert!(
            (median - 2.0).abs() < 0.1,
            "{name}: advance median {median}"
        );
    }
}

/// The 2048-lineage set Omega also ships (`Data\XML\2048_hud\`) composes
/// whole, and every atlas it names resolves, as authored or as `.gnf` -
/// recorded for the 2048 cross-check, not wired: `Title::hud` is one set per
/// title.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_2048_hud_set_omega_also_ships_composes_whole() {
    let Some(mut archives) = open() else { return };
    let roots = [
        r"Data\XML\2048_hud\Arcade_HUD.xml",
        r"Data\XML\2048_hud\Elimination_HUD.xml",
        r"Data\XML\2048_hud\SpeedLap_HUD.xml",
        r"Data\XML\2048_hud\SpeedLap_TimeTrial_HUD.xml",
        r"Data\XML\2048_hud\Zone_HUD.xml",
    ];
    let mut unresolved = BTreeSet::new();
    for root in roots {
        let c = composed(&mut archives, root);
        assert!(c.missing.is_empty(), "{root}: missing {:?}", c.missing);
        assert!(c.layout.skipped.is_empty(), "{root}: skipped");
        for r in c.layout.textures() {
            let named = archives.read_name(r).is_ok();
            let as_gnf = archives
                .read_name(&oag_title::hud::replace_extension(r, ".gnf"))
                .is_ok();
            if !named && !as_gnf {
                unresolved.insert(r.to_string());
            }
        }
    }
    assert!(unresolved.is_empty(), "2048_hud atlases: {unresolved:?}");
}

/// `data00.psarc` (base) and `data08.psarc` (patch) both ship the HUD atlases.
/// The patch copy carries a mip chain the base copy lacks, and `Archives`
/// serves one of them; the base level is the same picture either way, except
/// where the patch repainted it, which this records rather than hides.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_patch_and_base_copies_of_each_atlas_agree_at_the_base_level() {
    let Some(source) = omega() else { return };
    let mut base =
        oag_assets::psarc::Archive::open_file(&source.join("omega-eu/uroot/data00.psarc"))
            .expect("base");
    let mut patch =
        oag_assets::psarc::Archive::open_file(&source.join("omega-eu-patch/uroot/data08.psarc"))
            .expect("patch");
    let mut identical = Vec::new();
    let mut repainted = Vec::new();
    for (name, _, _) in ATLASES {
        if *name == "voiceCom" {
            continue;
        }
        let path = format!("Data/HUD/Textures/{name}.gnf");
        let (b, p) = (
            base.read_path(&path).expect("base copy"),
            patch.read_path(&path).expect("patch copy"),
        );
        let decode = |blob: &[u8]| {
            oag_texture::gnf::Texture::parse(blob)
                .expect("parses")
                .decode(blob)
                .expect("decodes")
        };
        if decode(&b) == decode(&p) {
            identical.push(*name);
        } else {
            repainted.push(*name);
        }
    }
    assert_eq!(identical.len(), 7, "{identical:?}");
    assert_eq!(repainted, ["HUD_Components_01"]);

    // The patch copy is the one the engine's archive order serves.
    let mut archives = open().expect("opens");
    let served = archives
        .read_name(r"Data\HUD\Textures\HUD_Components_01.gnf")
        .expect("served");
    let patched = patch
        .read_path("Data/HUD/Textures/HUD_Components_01.gnf")
        .expect("patch copy");
    assert_eq!(served, patched);
}
