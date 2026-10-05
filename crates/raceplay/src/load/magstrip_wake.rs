//! What a race loads for the HD-lineage magstrip arc wake: the two textures
//! `MagstripWake_Construct` binds, and each craft's `arc_anchor_point`.
//!
//! Absence is reported, never substituted - a title without the class, a
//! texture that will not decode and a hull without the locator each say so in
//! the loader report and draw no wake. See `oag_fx::magstrip`.

use oag_core::math::Mat4;
use oag_fx::exhaust::FlareTexture;
use oag_livery::Livery;
use oag_title::weapons::{MagstripWake, WeaponModels};

/// The atlas and the contact texture, in that order.
pub(crate) type Textures = [FlareTexture; 2];

/// The wake's per-slot anchors, and its two textures.
pub(super) struct Loaded {
    pub(super) anchors: Option<[Option<Mat4>; oag_gameplay::MAX_SHIPS]>,
    pub(super) textures: Option<Textures>,
}

pub(super) fn load(
    archives: &mut oag_assets::Archives,
    models: &WeaponModels,
    liveries: &[Livery],
    report: &mut Vec<String>,
) -> Loaded {
    let Some(wake) = models.magstrip_wake else {
        return Loaded {
            anchors: None,
            textures: None,
        };
    };
    let platform = archives.layout.platform;
    let textures = textures(archives, platform, wake, report);
    let mut anchors = [None; oag_gameplay::MAX_SHIPS];
    for (slot, livery) in liveries.iter().enumerate().take(anchors.len()) {
        anchors[slot] = livery.arc_anchor;
    }
    let with = anchors.iter().flatten().count();
    report.push(format!(
        "magstrip arc wake: {with} of {} craft carry an arc_anchor_point{}",
        liveries.len().min(anchors.len()),
        if textures.is_some() {
            ""
        } else {
            " - and no texture decoded, so nothing draws"
        }
    ));
    Loaded {
        anchors: Some(anchors),
        textures,
    }
}

fn textures(
    archives: &mut oag_assets::Archives,
    platform: oag_assets::Platform,
    wake: MagstripWake,
    report: &mut Vec<String>,
) -> Option<Textures> {
    let atlas = texture(archives, platform, wake.atlas, report)?;
    let contact = texture(archives, platform, wake.contact, report)?;
    Some([atlas, contact])
}

fn texture(
    archives: &mut oag_assets::Archives,
    platform: oag_assets::Platform,
    name: &str,
    report: &mut Vec<String>,
) -> Option<FlareTexture> {
    let decoded = archives
        .read_name(name)
        .map_err(|why| format!("{name}: not in the archive set ({why})"))
        .and_then(|blob| decode(&blob, platform).ok_or_else(|| format!("{name}: does not decode")));
    match decoded {
        Ok(texture) => {
            report.push(format!(
                "{name}: {}x{}, magstrip arc wake",
                texture.width, texture.height
            ));
            Some(texture)
        }
        Err(why) => {
            report.push(format!("{why} - the magstrip arc wake draws nothing"));
            None
        }
    }
}

/// The wake's texture as pixels: a `.gxt` on the Vita (2048), a `.gtf` on every
/// other title that names one (HD; the shadow silhouette reads one the same way).
fn decode(blob: &[u8], platform: oag_assets::Platform) -> Option<FlareTexture> {
    if platform == oag_assets::Platform::Vita {
        let gxt = oag_texture::gxt::Gxt::parse(blob).ok()?;
        let texture = gxt.only()?;
        let rgba = texture.to_rgba(blob).ok()?;
        let (width, height) = texture.level_size(0);
        return Some(FlareTexture {
            width,
            height,
            rgba: rgba.into_iter().flatten().collect(),
        });
    }
    let gtf = oag_texture::gtf::Gtf::parse(blob).ok()?;
    let texture = gtf.only()?;
    let rgba = texture.to_rgba(blob).ok()?;
    let (width, height) = texture.level_size(0);
    Some(FlareTexture {
        width,
        height,
        rgba: rgba.into_iter().flatten().collect(),
    })
}
