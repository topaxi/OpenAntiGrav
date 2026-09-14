//! The Fury menu backdrop's assets: the settings file, twelve of the nineteen
//! point clouds, and the per-screen tints - what `BackgroundAnimFury_Load`
//! reads when the Fury style enables its widget.
//!
//! Its own file rather than a stretch of `boot.rs`, for the reason
//! `sprites.rs` gives: that file is baselined by `scripts/check-file-size.py`
//! and may shrink but not grow.

use std::sync::Arc;

use oag_core::rng::Rng;
use oag_rcs::points2::PointCloud;
use oag_tables::envsettings::EnvSettings;
use oag_tables::fury_backdrop::FuryBackdrop;
use oag_ui::backdrop::Tints;

/// What the picks are seeded with.
///
/// **Fixed, where the original rolls from the clock.** A `--menu-page` capture
/// is one picture and has to be the same picture twice, and nothing in the
/// boot carries a per-run seed yet; when one does, this is the one place to
/// read it.
pub const SEED: u64 = 0x4655_5259;

/// Everything the backdrop needs, read once at boot and shared with the
/// menus by `Arc` because the clouds are twelve times 55,000 points.
#[derive(Debug)]
pub struct FuryAssets {
    /// `fury.envsettings`, typed.
    pub settings: FuryBackdrop,
    /// The twelve clouds `Load` picked, in table order.
    pub clouds: Vec<PointCloud>,
    /// Their names, for the report.
    pub names: Vec<&'static str>,
    /// The `<ScreenSetting>` tints off the skin.
    pub tints: Tints,
    /// Which of [`Self::clouds`] the first clip draws - `Load`'s own second
    /// roll, distinct from the previous cloud it also rolls.
    pub first: usize,
}

/// Reads the settings, twelve clouds and the tints, or says why not.
///
/// `None` with a report line for every reason: the style is HD rather than
/// Fury (the widget stays `startenabled="false"`), the skin authors no
/// widget, the settings do not read, or no cloud does. A cloud that fails on
/// its own is skipped and named; the rest still play, as `Load`'s own retry
/// loop would.
pub(super) fn load(
    archives: &mut oag_assets::Archives,
    fury_style: bool,
    skin_xml: Option<&str>,
    report: &mut Vec<String>,
) -> Option<Arc<FuryAssets>> {
    use oag_hd::frontend::names::{FURY_CLOUDS, FURY_CLOUDS_LOADED, FURY_SETTINGS};
    if !fury_style {
        return None;
    }
    let Some(tints) = skin_xml.and_then(|xml| Tints::read(&oag_tables::fexml::parse(xml))) else {
        report.push("menu backdrop: the skin authors no BackgroundAnimFury widget".to_string());
        return None;
    };
    let settings = match archives.read_name(FURY_SETTINGS) {
        Ok(bytes) => match String::from_utf8(bytes)
            .map_err(|e| e.to_string())
            .and_then(|text| EnvSettings::parse(&text).map_err(|e| e.to_string()))
        {
            Ok(parsed) => FuryBackdrop::read(&parsed),
            Err(error) => {
                report.push(format!(
                    "menu backdrop: {FURY_SETTINGS} does not parse: {error}"
                ));
                return None;
            }
        },
        Err(error) => {
            report.push(format!("menu backdrop: {FURY_SETTINGS}: {error:#}"));
            return None;
        }
    };
    // `Load`'s pick: `rand() % 19` into a used-set, re-rolled forward until a
    // free slot, twelve times.
    let mut rng = Rng::new(SEED);
    let mut used = [false; FURY_CLOUDS.len()];
    let mut clouds = Vec::with_capacity(FURY_CLOUDS_LOADED);
    let mut names = Vec::with_capacity(FURY_CLOUDS_LOADED);
    let mut failed = Vec::new();
    while clouds.len() < FURY_CLOUDS_LOADED && used.iter().filter(|u| !**u).count() > 0 {
        let mut slot = rng.below(FURY_CLOUDS.len() as u32) as usize;
        while used[slot] {
            slot = (slot + 1) % FURY_CLOUDS.len();
        }
        used[slot] = true;
        let name = FURY_CLOUDS[slot];
        match archives
            .read_name(name)
            .map_err(|e| format!("{e:#}"))
            .and_then(|bytes| oag_rcs::points2::parse(&bytes).map_err(|e| e.to_string()))
        {
            Ok(cloud) => {
                clouds.push(cloud);
                names.push(name);
            }
            Err(error) => failed.push(format!("{name}: {error}")),
        }
    }
    for failure in &failed {
        report.push(format!("menu backdrop: {failure} - skipped"));
    }
    if clouds.is_empty() {
        report.push(
            "menu backdrop: no cloud read, so the menus draw on the page's clear".to_string(),
        );
        return None;
    }
    // Two distinct table slots, previous then current; only the current
    // draws until a mode that swaps them is read.
    let previous = rng.below(clouds.len() as u32) as usize;
    let mut first = rng.below(clouds.len() as u32) as usize;
    while clouds.len() > 1 && first == previous {
        first = rng.below(clouds.len() as u32) as usize;
    }
    let authored = settings.authored_static_paths().count();
    report.push(format!(
        "menu backdrop: Fury point clouds, {} of {} loaded ({} points in the first, {}), {authored} static paths of 8, {} screen tints; morph and dynamic paths re-rolled, no music",
        clouds.len(),
        FURY_CLOUDS.len(),
        clouds[first].points.len(),
        names[first].rsplit('/').next().unwrap_or(names[first]),
        tints.len(),
    ));
    Some(Arc::new(FuryAssets {
        settings,
        clouds,
        names,
        tints,
        first,
    }))
}
