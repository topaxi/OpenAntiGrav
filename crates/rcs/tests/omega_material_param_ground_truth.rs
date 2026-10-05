//! Validates the PS4 material instance table (`docs/formats/omega-status.md`,
//! "The material uniform and sampler table") against every `.rcsmodel` in
//! Wipeout: Omega Collection's base and patch archives.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! PS4 ships no shader that names its inputs, so the Vita pass's name check
//! against the material's own program is unavailable. The invariants instead:
//!
//! - **the entry table is complete**: every entry is a uniform (`1`) or a
//!   sampler (`0x12`), and the reader returns every one of them;
//! - **a sampler is bound or declared empty**: a null path pointer, or a
//!   `.gnf`; nothing else;
//! - **the float pool tiles exactly**: a material's uniforms, each reading the
//!   component count it states, add up to the pool's stated count - on every
//!   material, so no value was missed and none overlaps;
//! - **the unused header words are zero**: `+0x34`, `+0x38`, `+0x44`, `+0x4c`,
//!   `+0x54`;
//! - **names agree with the family**: the 2048 glow layer's uniforms
//!   (`Emissive_UV_Offset`/`Scale` one component, `GlowTint` three) read the
//!   widths their names say.
//!
//! One archive per test, so the sweep parallelises across tests.

use std::path::{Path, PathBuf};

use oag_rcs::rcsmaterial::{name_hash, names};
use oag_rcs::rcsmodel::psp2;

fn package(dir: &str, name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/ps4")
        .join(dir)
        .join("uroot")
        .join(name);
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

#[derive(Default, Debug, PartialEq, Eq)]
struct Survey {
    files: usize,
    materials: usize,
    uniforms: usize,
    samplers: usize,
    /// Entries of a kind that is neither a uniform nor a sampler.
    other_kinds: usize,
    /// Samplers declared with a null path pointer: nothing is bound to them.
    unbound: usize,
    /// Entries the raw count says exist that the reader did not return.
    dropped: usize,
    /// Materials whose uniforms do not add up to the pool's stated count.
    pool_gaps: usize,
    /// Materials with a nonzero word in a header slot read as unused.
    stray_words: usize,
    /// Glow-layer uniforms of the wrong width.
    wrong_width: usize,
    /// Uniforms and samplers whose name some table here recovers.
    named_uniforms: usize,
    named_samplers: usize,
    /// Materials the render's glow layer acts on: both `Emissive_UV_*`
    /// uniforms and an emissive and a diffuse sampler bound.
    glow_layers: usize,
    /// Materials with a plain `speed_multipliaer` scroll and no glow layer.
    plain_scrolls: usize,
}

fn sweep(dir: &str, name: &str) -> Option<Survey> {
    let path = package(dir, name)?;
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let models: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
        .cloned()
        .collect();
    let widths = [
        (name_hash("Emissive_UV_Offset"), 1),
        (name_hash("Emissive_UV_Scale"), 1),
        (name_hash("GlowTint"), 3),
    ];
    let mut survey = Survey::default();
    for model_path in models {
        let blob = archive.read_path(&model_path).expect("the model reads");
        let model = psp2::parse(&blob).unwrap_or_else(|e| panic!("{model_path}: {e}"));
        let cpu = model.sections[0];
        let cpu = &blob[cpu.at..cpu.at + cpu.len];
        let w32 = |at: usize| u32::from_le_bytes(cpu[at..at + 4].try_into().unwrap());
        let w64 =
            |at: usize| usize::try_from(u64::from_le_bytes(cpu[at..at + 8].try_into().unwrap()));
        if model.materials.is_empty() {
            continue;
        }
        survey.files += 1;
        let table = w64(0x78).expect("a table offset");
        for (i, material) in model.materials.iter().enumerate() {
            survey.materials += 1;
            let header = w64(table + i * 8).expect("a header offset");
            let stray = [0x34, 0x38, 0x44, 0x4c, 0x54]
                .iter()
                .any(|o| w32(header + o) != 0);
            survey.stray_words += usize::from(stray);
            let (count, entries) = (w32(header + 0x48) as usize, w64(header + 0x50).unwrap());
            let (mut uniforms, mut samplers) = (0, 0);
            for e in 0..count {
                match w32(entries + e * 0x28 + 4) {
                    1 => uniforms += 1,
                    0x12 if w64(entries + e * 0x28 + 0x18).unwrap() == 0 => survey.unbound += 1,
                    0x12 => samplers += 1,
                    _ => survey.other_kinds += 1,
                }
            }
            survey.dropped +=
                (uniforms - material.params.len()) + (samplers - material.samplers.len());
            survey.uniforms += material.params.len();
            survey.samplers += material.samplers.len();
            let total: usize = material.params.iter().map(|p| p.bits.len()).sum();
            survey.pool_gaps += usize::from(total != w32(header + 0x3c) as usize);
            for p in &material.params {
                survey.named_uniforms += usize::from(
                    names::psp2_parameter_name(p.hash)
                        .or_else(|| names::parameter_name(p.hash))
                        .is_some(),
                );
                if let Some(&(_, w)) = widths.iter().find(|(h, _)| *h == p.hash) {
                    survey.wrong_width += usize::from(p.bits.len() != w);
                }
            }
            let has = |n: &str| material.param(name_hash(n)).is_some();
            let bound = |ns: &[&str]| ns.iter().any(|n| material.sampler(name_hash(n)).is_some());
            let glow = has("Emissive_UV_Offset")
                && has("Emissive_UV_Scale")
                && bound(&["EmissiveTexture", "EmissiveMap"])
                && bound(&["DiffuseTexture", "DiffuseAlphaMap", "DiffuseMap"]);
            survey.glow_layers += usize::from(glow);
            survey.plain_scrolls +=
                usize::from(!has("Emissive_UV_Offset") && has("speed_multipliaer"));
            for (h, _) in &material.samplers {
                survey.named_samplers += usize::from(names::sampler_name(*h).is_some());
            }
        }
    }
    Some(survey)
}

fn holds(dir: &str, name: &str, expected: &str) {
    let Some(s) = sweep(dir, name) else { return };
    assert_eq!(s.other_kinds, 0, "{name}: an entry that is neither kind");
    assert_eq!(s.dropped, 0, "{name}: an entry the reader dropped");
    assert_eq!(s.pool_gaps, 0, "{name}: the float pool does not tile");
    assert_eq!(s.stray_words, 0, "{name}: a nonzero unused header word");
    assert_eq!(
        s.wrong_width, 0,
        "{name}: a glow uniform of the wrong width"
    );
    assert_eq!(format!("{s:?}"), expected, "{name}");
}

macro_rules! archive {
    ($test:ident, $dir:literal, $name:literal, $expected:literal) => {
        #[test]
        #[ignore = "needs the Omega extraction"]
        fn $test() {
            holds($dir, $name, $expected);
        }
    };
}

archive!(
    base_data00,
    "omega-eu",
    "data00.psarc",
    "Survey { files: 620, materials: 4918, uniforms: 6833, samplers: 7236, other_kinds: 0, unbound: 3290, dropped: 0, pool_gaps: 0, stray_words: 0, wrong_width: 0, named_uniforms: 2720, named_samplers: 3597, glow_layers: 16, plain_scrolls: 2 }"
);
archive!(
    base_data01,
    "omega-eu",
    "data01.psarc",
    "Survey { files: 304, materials: 5871, uniforms: 5959, samplers: 11004, other_kinds: 0, unbound: 2879, dropped: 0, pool_gaps: 0, stray_words: 0, wrong_width: 0, named_uniforms: 2097, named_samplers: 9222, glow_layers: 128, plain_scrolls: 0 }"
);
archive!(
    base_data02,
    "omega-eu",
    "data02.psarc",
    "Survey { files: 32, materials: 8565, uniforms: 14668, samplers: 18515, other_kinds: 0, unbound: 4061, dropped: 0, pool_gaps: 0, stray_words: 0, wrong_width: 0, named_uniforms: 4782, named_samplers: 14728, glow_layers: 153, plain_scrolls: 0 }"
);
archive!(
    base_data03,
    "omega-eu",
    "data03.psarc",
    "Survey { files: 143, materials: 289, uniforms: 637, samplers: 446, other_kinds: 0, unbound: 289, dropped: 0, pool_gaps: 0, stray_words: 0, wrong_width: 0, named_uniforms: 355, named_samplers: 130, glow_layers: 0, plain_scrolls: 0 }"
);
archive!(
    base_data04,
    "omega-eu",
    "data04.psarc",
    "Survey { files: 40, materials: 13722, uniforms: 16415, samplers: 20548, other_kinds: 0, unbound: 7169, dropped: 0, pool_gaps: 0, stray_words: 0, wrong_width: 0, named_uniforms: 3781, named_samplers: 10024, glow_layers: 20, plain_scrolls: 17 }"
);
archive!(
    patch_data05,
    "omega-eu-patch",
    "data05.psarc",
    "Survey { files: 1, materials: 1, uniforms: 2, samplers: 2, other_kinds: 0, unbound: 1, dropped: 0, pool_gaps: 0, stray_words: 0, wrong_width: 0, named_uniforms: 1, named_samplers: 2, glow_layers: 0, plain_scrolls: 0 }"
);
archive!(
    patch_data07,
    "omega-eu-patch",
    "data07.psarc",
    "Survey { files: 1, materials: 6, uniforms: 13, samplers: 10, other_kinds: 0, unbound: 6, dropped: 0, pool_gaps: 0, stray_words: 0, wrong_width: 0, named_uniforms: 6, named_samplers: 4, glow_layers: 0, plain_scrolls: 0 }"
);
archive!(
    patch_data08,
    "omega-eu-patch",
    "data08.psarc",
    "Survey { files: 99, materials: 1051, uniforms: 990, samplers: 597, other_kinds: 0, unbound: 1051, dropped: 0, pool_gaps: 0, stray_words: 0, wrong_width: 0, named_uniforms: 391, named_samplers: 165, glow_layers: 0, plain_scrolls: 0 }"
);
archive!(
    patch_data09,
    "omega-eu-patch",
    "data09.psarc",
    "Survey { files: 0, materials: 0, uniforms: 0, samplers: 0, other_kinds: 0, unbound: 0, dropped: 0, pool_gaps: 0, stray_words: 0, wrong_width: 0, named_uniforms: 0, named_samplers: 0, glow_layers: 0, plain_scrolls: 0 }"
);
