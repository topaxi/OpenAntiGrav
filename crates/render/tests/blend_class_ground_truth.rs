//! Pins that the recovered transparent blend classes are actually *used* by
//! shipped content, and in what proportion.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all \
//!     -E 'binary(blend_class_ground_truth)'
//! ```
//!
//! # Why this exists
//!
//! `Gfx_BuildBatchStateList` splits `Batch::is_transparent`'s `0x0700` mask
//! three ways - `0x100` alpha-over, `0x200` additive, `0x400` unblended - and
//! this crate drew every one of them alpha-over until that was recovered. The
//! change is only worth its blast radius if real content actually mixes the
//! classes, so that is what this measures rather than asserting the decode in
//! the abstract (`oag_formats::vex`'s own unit tests already cover the bit
//! arithmetic).
//!
//! Measured when this was written, and the reason the change shipped:
//!
//! | Model | transparent draws | alpha-over | additive |
//! | --- | ---: | ---: | ---: |
//! | `01_Track` | 142 | 103 (5,389 tri) | **39 (4,075 tri)** |
//! | `16_Track` | 254 | 154 (12,630 tri) | **100 (6,197 tri)** |
//! | `Assegai\Ship.vex` | 1 | 0 | **1 (37 tri)** |
//! | `Assegai\shipboost.vex` | 4 | 0 | **4 (113 tri)** |
//!
//! So a quarter to two fifths of a circuit's transparent geometry was being
//! blended with the wrong equation. **No `0x400` batch was found on any of
//! them** - that path is implemented from the decode and is untested by
//! content, which is asserted below so the day one appears is not silent.

use std::path::{Path, PathBuf};

use oag_assets::pulse;
use oag_formats::vex::BlendClass;
use oag_render::mesh;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");
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

/// Counts per class for one model, as `(alpha_over, additive, unblended, unset)`.
fn census(archives: &mut pulse::Archives, name: &str) -> (usize, usize, usize, usize) {
    let blob = archives
        .read_name(name)
        .unwrap_or_else(|e| panic!("reading {name}: {e}"));
    let model = mesh::build_with_textures(name, &blob, None, mesh::Lod::Both)
        .unwrap_or_else(|e| panic!("decoding {name}: {e}"));
    let mut counts = (0, 0, 0, 0);
    for draw in &model.transparent_draws {
        match draw.blend {
            Some(BlendClass::AlphaOver) => counts.0 += 1,
            Some(BlendClass::Additive) => counts.1 += 1,
            Some(BlendClass::None) => counts.2 += 1,
            None => counts.3 += 1,
        }
    }
    println!(
        "{name}: {} transparent draw(s) - {} alpha-over, {} additive, {} unblended, {} unset",
        model.transparent_draws.len(),
        counts.0,
        counts.1,
        counts.2,
        counts.3
    );
    counts
}

/// Two circuits and a ship, all of which mix the classes. If a future decode
/// change ever collapses them back to one class this fails, which is the point:
/// the per-batch pipeline selection would silently become dead code.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn shipped_content_mixes_the_transparent_blend_classes() {
    let Some(image) = image() else {
        return;
    };
    let mut archives =
        pulse::Archives::open(&image.display().to_string()).expect("opening the PSP archives");

    for name in [
        r"Data\Environments\01_Track\track.vex",
        r"Data\Environments\16_Track\track.vex",
    ] {
        let (over, additive, _, unset) = census(&mut archives, name);
        assert!(
            over > 0 && additive > 0,
            "{name}: {over} alpha-over and {additive} additive - a circuit that does not \
             mix the classes means the per-batch selection is doing nothing here"
        );
        assert_eq!(
            unset, 0,
            "{name}: {unset} transparent draw(s) carry no blend class, which cannot happen \
             for a batch that reached transparent_draws through is_transparent()"
        );
    }

    // The ship side: every transparent batch on both files is additive, so the
    // plume's blend is not a special case made for it - it is the class its
    // `pass_mask` asks for, the same one the flare uses.
    for name in [
        r"Data\Ships\Assegai\Ship.vex",
        r"Data\Ships\Assegai\shipboost.vex",
    ] {
        let (over, additive, none, unset) = census(&mut archives, name);
        assert_eq!(
            (over, none, unset),
            (0, 0, 0),
            "{name}: expected every transparent batch to be additive, got {over} alpha-over, \
             {none} unblended, {unset} unset"
        );
        assert!(additive > 0, "{name}: no additive batches at all");
    }
}

/// `BlendClass::None` (`pass_mask & 0x400`) is implemented from the decode and
/// no shipped model checked here uses it. Asserted rather than assumed so that
/// the day one appears, this fails and someone looks at it instead of trusting
/// an untested path.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn no_checked_model_uses_the_unblended_transparent_class() {
    let Some(image) = image() else {
        return;
    };
    let mut archives =
        pulse::Archives::open(&image.display().to_string()).expect("opening the PSP archives");

    for name in [
        r"Data\Environments\01_Track\track.vex",
        r"Data\Environments\16_Track\track.vex",
        r"Data\Ships\Assegai\Ship.vex",
        r"Data\Ships\Assegai\shipboost.vex",
    ] {
        let (_, _, unblended, _) = census(&mut archives, name);
        assert_eq!(
            unblended, 0,
            "{name} now carries {unblended} `0x400` batch(es). That path has never been \
             exercised by content - check it draws correctly rather than deleting this test."
        );
    }
}
