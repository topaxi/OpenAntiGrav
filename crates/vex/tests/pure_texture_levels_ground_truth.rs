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

/// Feisar's three hull textures: the mip count the GE was handed for each (`TEXMODE` `0x40001`),
/// and the FNV-1a 64 of the index bytes of its levels 0 to 4 as a PPSSPP GE list of Modesto
/// Heights' grid recorded them (two dumps agree), each level unswizzled at its own padded stride.
const FEISAR_HULL: [(&str, u8, [u64; 5]); 3] = [
    (
        "feis_01_shinemap.tga",
        5,
        [
            0x3f68062e1283416e,
            0x816e12e6159b0043,
            0x8065dfc3ff9a40fb,
            0x0c8e981c36c84007,
            0xb6223e63d0604fe6,
        ],
    ),
    (
        "feis_02_shinemap.tga",
        5,
        [
            0x71822c9136a6c8a1,
            0xcb568ba3352b02ea,
            0xd648e7d9fa9a2659,
            0x9e1eabe8a4e33427,
            0x521157f23378b4c4,
        ],
    ),
    (
        "feis_03_shinemap.tga",
        5,
        [
            0xf0a300626c6a9048,
            0xdbe7476e053e37fc,
            0xb2bde9974b67da0e,
            0x13119a5ba63647b9,
            0x0edff6b83dc911a8,
        ],
    ),
];

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

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
        }
    }
    assert!(
        seen > 150,
        "only {seen} textures read across {} files",
        files.len()
    );

    let feisar = textures_of(&mut archives, r"Data\Ships\Feisar\Ship.vex");
    for (name, mips, want) in FEISAR_HULL {
        let hull = feisar
            .iter()
            .find(|t| t.asset_path.as_deref().is_some_and(|p| p.ends_with(name)))
            .unwrap_or_else(|| panic!("Feisar's hull has no {name}"));
        assert_eq!(
            (hull.width, hull.height, hull.mip_count),
            (128, 128, mips),
            "{name}"
        );
        let got: Vec<u64> = std::iter::once(&hull.indices)
            .chain(&hull.levels)
            .map(|level| fnv1a(level))
            .collect();
        assert_eq!(got, want, "{name}: levels 0 to 4 against the GE's own");
    }
}
