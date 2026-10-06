//! Shared helpers for `search_tests.rs` and `micro_tile_tests.rs`: matching a
//! `.gnf` to its HD `.gtf` twin by team/livery folder (see `find_hd_twin`) and
//! the pixel-index bit order both files decode blocks with.

use oag_assets::psarc::Archive;

use crate::gnf::{SurfaceFormat, Texture};

pub(super) fn mean_abs_diff(a: &[[u8; 4]], b: &[[u8; 4]]) -> f64 {
    let mut total = 0u64;
    for (x, y) in a.iter().zip(b) {
        for c in 0..4 {
            total += u64::from(x[c].abs_diff(y[c]));
        }
    }
    total as f64 / (a.len() * 4) as f64
}

/// The team folder name a `.gnf`/`.gtf` path names, whichever of
/// `hdships/<team>/...` (PS4) or `ships/<team>/...` (PS3) it is.
pub(super) fn team_token(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase().replace('\\', "/");
    let parts: Vec<&str> = lower.split('/').collect();
    for (i, p) in parts.iter().enumerate() {
        if *p == "hdships" || *p == "ships" {
            return parts.get(i + 1).map(|s| (*s).to_string());
        }
    }
    None
}

/// The `liveryN` path component, if either title's path has one.
pub(super) fn livery_token(path: &str) -> Option<String> {
    let lower = path.to_ascii_lowercase().replace('\\', "/");
    lower
        .split('/')
        .find(|p| p.contains("livery"))
        .map(str::to_string)
}

/// One oracle pair: the `.gnf` path, its parsed descriptor, its raw bytes,
/// and the HD `.gtf` twin already decoded to RGBA8 texels.
pub(super) type OraclePair = (String, Texture, Vec<u8>, Vec<[u8; 4]>);

/// Collects up to `max` team/livery-matched, same-dimension `.gnf`/`.gtf`
/// pairs, favouring a spread of sizes (sorted by block area then taken
/// evenly) so a search is judged across several sizes, not one.
pub(super) fn collect_oracle_pairs(max: usize) -> Vec<OraclePair> {
    let Some(base) = oag_testdata::exact("data/extracted/ps4/omega-eu/uroot/data03.psarc") else {
        return Vec::new();
    };
    let Ok(mut gnf_archive) = Archive::open_file(&base) else {
        return Vec::new();
    };
    let mut candidates: Vec<(String, Texture, Vec<u8>)> = Vec::new();
    let paths: Vec<String> = gnf_archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".gnf"))
        .cloned()
        .collect();
    for path in paths {
        let Ok(blob) = gnf_archive.read_path(&path) else {
            continue;
        };
        let Ok(texture) = Texture::parse(&blob) else {
            continue;
        };
        if texture.surface_format != SurfaceFormat::Bc7 || texture.tile_mode.0 != 13 {
            continue;
        }
        candidates.push((path, texture, blob));
    }
    candidates.sort_by_key(|(_, t, _)| t.width * t.height);

    let mut out = Vec::new();
    let step = (candidates.len() / max.max(1)).max(1);
    let mut i = 0;
    while i < candidates.len() && out.len() < max {
        let (path, texture, blob) = &candidates[i];
        let basename = path
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .to_ascii_lowercase()
            .replace(".gnf", ".gtf");
        if let Some((hw, hh, truth)) = find_hd_twin(&basename, path)
            && (hw, hh) == (texture.width, texture.height)
        {
            println!("oracle pair: {path} ({hw}x{hh})");
            out.push((path.clone(), *texture, blob.clone(), truth));
        }
        i += step;
    }
    out
}

/// The single `.gtf` this `.gnf` path's team (and livery slot) most plausibly
/// re-ships as. Matching by basename alone can pair one ship's decal with a
/// different ship's or livery's same-named, same-sized file.
pub(super) fn find_hd_twin(basename: &str, gnf_path: &str) -> Option<(u32, u32, Vec<[u8; 4]>)> {
    let want_team = team_token(gnf_path)?;
    let want_livery = livery_token(gnf_path);
    for index in 0..7 {
        let path = oag_testdata::exact(&format!(
            "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/DATA0{index}.PSARC"
        ))?;
        let Ok(mut a) = Archive::open_file(&path) else {
            continue;
        };
        let entries: Vec<String> = a
            .paths()
            .iter()
            .filter(|p| p.to_ascii_lowercase().ends_with(basename))
            .filter(|p| team_token(p).as_deref() == Some(want_team.as_str()))
            .filter(|p| want_livery.is_none() || livery_token(p) == want_livery)
            .cloned()
            .collect();
        for entry in entries {
            if let Ok(blob) = a.read_path(&entry) {
                let Ok(parsed) = crate::gtf::Gtf::parse(&blob) else {
                    continue;
                };
                let Some(t) = parsed.only() else { continue };
                let Ok(rgba) = t.to_rgba(&blob) else { continue };
                return Some((u32::from(t.width), u32::from(t.height), rgba));
            }
        }
    }
    None
}

/// `ADDR_NON_DISPLAYABLE` ("Thin") micro tile type, `bpp == 128`:
/// `pixelBit0..5 = x0,y0,x1,y1,x2,y2` (`Lib::ComputePixelIndexWithinMicroTile`,
/// `addrlib1.cpp`), the intra-tile order both address formulas here use.
pub(super) fn micro_tile_index(bx: u32, by: u32) -> u32 {
    let x0 = bx & 1;
    let x1 = (bx >> 1) & 1;
    let x2 = (bx >> 2) & 1;
    let y0 = by & 1;
    let y1 = (by >> 1) & 1;
    let y2 = (by >> 2) & 1;
    x0 | (y0 << 1) | (x1 << 2) | (y1 << 3) | (x2 << 4) | (y2 << 5)
}
