//! Validates the PS2 image substitution against the real discs: the entries
//! `oag_pulse::PS2_IMAGES` names really do hold the pictures the PSP
//! keeps under those `.mip` names.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! The mapping is the one thing in the PS2 asset path that is **not** derived
//! from a rule. A model finds its texture set by directory position and a font
//! finds its atlas the same way, but `Data\HUD\Textures\PulseHUD.mip` hashes to
//! an entry that is not on the disc and no spelling of it helps, so the three
//! entries were identified by their pictures instead. A hardcoded hash that
//! nothing checks is exactly the kind of constant that rots silently when
//! someone reaches for a different pressing, so this re-derives the match
//! rather than asserting the hash against itself:
//!
//! - the PSP `.mip` and the PS2 entry are the **same shape**, and
//! - their **silhouettes agree** - which palette entry is transparent, per
//!   pixel - to better than 99 %, a measure that survives the two builds
//!   ordering their palettes differently.
//!
//! `gameshare_backdrop.mip` is deliberately not in the table: it is a PSP
//! ad-hoc Game Sharing asset and is genuinely absent from the PS2 disc. The
//! last test pins that, so "missing" does not quietly become "we never looked".

use std::path::{Path, PathBuf};

use oag_formats::{ps2_texture, texture};
use oag_pulse as pulse;

/// How much of the silhouette has to agree before two images are the same
/// picture. The three real pairs score 0.9946 to 0.9999 and the best
/// runner-up in each search scores 0.71 or less, so anything in between
/// separates them.
const SILHOUETTE: f64 = 0.95;

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
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

/// One bit per pixel: whether this texel's palette entry is transparent.
fn silhouette<'a>(indices: &'a [u8], palette: &'a [[u8; 4]]) -> impl Iterator<Item = bool> + 'a {
    indices
        .iter()
        .map(move |&i| palette[usize::from(i)][3] != 0)
}

#[test]
#[ignore = "needs both disc images under data/images"]
fn every_substituted_ps2_entry_holds_the_picture_the_psp_names() {
    let (Some(psp_path), Some(ps2_path)) = (image("pulse-psp-eu.chd"), image("pulse-ps2-eu.chd"))
    else {
        return;
    };
    let mut psp = pulse::open(&psp_path.to_string_lossy()).expect("open psp");
    let mut ps2 = pulse::open(&ps2_path.to_string_lossy()).expect("open ps2");

    assert!(!pulse::PS2_IMAGES.is_empty(), "the table is not empty");

    for &(name, hash) in pulse::PS2_IMAGES {
        // The name has to be a real PSP entry, or the table is mapping from
        // something no XML actually asks for.
        let blob = psp
            .read_name(name)
            .unwrap_or_else(|e| panic!("{name} on the PSP disc: {e}"));
        let want = texture::Texture::parse(&blob).unwrap_or_else(|e| panic!("{name}: {e}"));

        // And it has to be absent from the PS2 disc, or the substitution is
        // unnecessary and would shadow a real entry.
        assert!(
            ps2.read_name(name).is_err(),
            "{name} is on the PS2 disc after all; the substitution should be removed"
        );

        let got = ps2
            .read_hash(hash)
            .unwrap_or_else(|e| panic!("{name} -> {hash:08x}: {e}"));
        let got = ps2_texture::parse(&got).unwrap_or_else(|e| panic!("{name}: {e}"));

        assert_eq!(
            (got.width, got.height),
            (want.width, want.height),
            "{name}: the PS2 entry is a different shape"
        );

        let agree = silhouette(&want.indices, &want.palette)
            .zip(silhouette(&got.indices, &got.palette))
            .filter(|(a, b)| a == b)
            .count();
        #[expect(clippy::cast_precision_loss, reason = "at most 512x512 pixels")]
        let rate = agree as f64 / want.indices.len() as f64;
        println!(
            "{name:38} -> {hash:08x}  {}x{}  silhouette {rate:.4}",
            got.width, got.height
        );
        assert!(
            rate >= SILHOUETTE,
            "{name}: silhouette agreement {rate:.4} is below {SILHOUETTE}"
        );

        // And the read the game actually performs resolves to the same bytes.
        let via_read_image =
            oag_pulse::read_image(&mut ps2, name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(
            ps2_texture::parse(&via_read_image).map(|t| (t.width, t.height)),
            Ok((got.width, got.height)),
            "{name}: read_image does not reach the substituted entry"
        );
    }
}

#[test]
#[ignore = "needs the PS2 disc image under data/images"]
fn the_psp_path_is_untouched_by_the_substitution() {
    let Some(path) = image("pulse-psp-eu.chd") else {
        return;
    };
    let mut psp = pulse::open(&path.to_string_lossy()).expect("open");

    // Every substituted name is a real PSP entry, so `read_image` must never
    // reach the table there: it has to return exactly what `read_name` does.
    for &(name, _) in pulse::PS2_IMAGES {
        let by_name = psp
            .read_name(name)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let by_image =
            oag_pulse::read_image(&mut psp, name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(by_name, by_image, "{name}: read_image diverged on the PSP");
    }
}

#[test]
#[ignore = "needs the PS2 disc image under data/images"]
fn game_sharing_art_is_confirmed_absent_rather_than_unlooked_for() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut ps2 = pulse::open(&path.to_string_lossy()).expect("open");

    let name = r"Data\FE\Images\gameshare_backdrop.mip";
    assert!(
        ps2.read_name(name).is_err(),
        "{name} is not on the PS2 disc"
    );
    assert!(
        pulse::ps2_image_hash(name).is_none(),
        "{name} has no substitution: Game Sharing is a PSP ad-hoc feature and \
         the picture is not on this disc under any entry - correlating it \
         against all 28 same-shaped PS2 textures peaks at 0.02"
    );
    assert!(
        oag_pulse::read_image(&mut ps2, name).is_err(),
        "{name} still fails, and says so with its own name"
    );
}
