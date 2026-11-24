//! Pure's version-4 `.vex` textures author their mip levels, and the reader hands them on.
//!
//! The GE draws every Pure texture in slope mode (`TEXLEVEL` mode 2, `TEXLODSLOPE`
//! `1/256` on all 430 PRIMs of a Modesto Heights race frame), selecting among the
//! levels the file ships; a reader that dropped them left the renderer on its own
//! derivative-driven selection over a box-filtered chain, which samples the craft
//! two or three levels coarser than the original does
//! (`docs/ghidra/functions/psp-pure-usa/hull-shine.md`, "The hull's texture level").
//!
//! **`#[ignore]`d, needs the Pure PSP disc.** Skips when it is absent;
//! `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.

use oag_vex::vex;

/// Every team's hull, in the order Pure's own loader lists them.
const TEAMS: [&str; 8] = [
    "Feisar",
    "Piranha",
    "Triakis",
    "AG_Systems",
    "Assegai",
    "Qirex",
    "Auricom",
    "Harimau",
];

/// Feisar's three hull textures, with the mip count the GE was handed for each (its
/// `TEXMODE` read `0x40001`: four levels below the base).
const FEISAR_HULL: [(&str, u16, u8); 3] = [
    ("feis_01_shinemap.tga", 128, 5),
    ("feis_02_shinemap.tga", 128, 5),
    ("feis_03_shinemap.tga", 128, 5),
];

fn textures_of(
    archives: &mut oag_assets::source::Archives,
    name: &str,
) -> Vec<vex::EmbeddedTexture> {
    let blob = archives
        .read_name(name)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(vex::version(&blob), Ok(4), "{name} is a version-4 file");
    vex::textures(&blob)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
        .into_iter()
        .flatten()
        .collect()
}

#[test]
#[ignore = "needs the Pure PSP disc in data/images/"]
fn every_pure_texture_reads_every_level_it_declares() {
    let Some(image) = oag_testdata::image("pure-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pure::open(&image.display().to_string()).expect("Pure's archives");
    let mut files = TEAMS
        .iter()
        .map(|team| format!(r"Data\Ships\{team}\Ship.vex"))
        .collect::<Vec<_>>();
    files.push(r"Data\Environments\03_Modesto_Heights\track.vex".to_string());

    let mut seen = 0;
    for file in &files {
        for texture in textures_of(&mut archives, file) {
            seen += 1;
            assert_eq!(
                texture.levels.len() + 1,
                usize::from(texture.mip_count),
                "{file}: {:?} declares {} levels and reads {}",
                texture.name,
                texture.mip_count,
                texture.levels.len() + 1
            );
            for (n, level) in texture.levels.iter().enumerate() {
                let w = usize::from((texture.width >> (n + 1)).max(1));
                let h = usize::from((texture.height >> (n + 1)).max(1));
                assert_eq!(
                    level.len(),
                    w * h,
                    "{file}: level {} of {:?}",
                    n + 1,
                    texture.name
                );
            }
        }
    }
    assert!(
        seen > 150,
        "only {seen} textures read across {} files",
        files.len()
    );

    let feisar = textures_of(&mut archives, r"Data\Ships\Feisar\Ship.vex");
    for (name, width, mips) in FEISAR_HULL {
        let hull = feisar
            .iter()
            .find(|t| t.asset_path.as_deref().is_some_and(|p| p.ends_with(name)))
            .unwrap_or_else(|| panic!("Feisar's hull has no {name}"));
        assert_eq!((hull.width, hull.mip_count), (width, mips), "{name}");
        assert_eq!(
            hull.levels.len(),
            usize::from(mips) - 1,
            "{name} is chained"
        );
    }
}
