//! `[campaign-medals]` (`docs/tools/mods.md`, "Campaign medals"): the art the campaign screens
//! fill a medal hex with, resolved from the mod that asks for it into a
//! [`oag_ui_screens::campaign::MedalArt`] and the texture it samples.

use oag_ui_screens::campaign::{MedalArt, medal_art::Atlas};

/// A per-difficulty atlas is three 1024x256-shaped blocks tall; the base game's own copy is one.
const PER_DIFFICULTY_HEIGHT: u16 = 768;

/// An atlas resolved, with the sheet entry that holds its pixels.
#[derive(Debug)]
pub struct Resolved {
    /// What the screens draw medals with.
    pub art: MedalArt,
    /// The disc texture `art` samples, by the sheet key it names; empty for a mod's own atlas.
    pub blobs: Vec<(String, Vec<u8>)>,
    /// A mod's own atlas, decoded.
    pub images: Vec<oag_hud::sprite::DecodedImage>,
}

/// The tiers a `{ mod }` atlas holds, top to bottom, when the manifest names none.
const DEFAULT_ROWS: [&str; 3] = ["gold", "silver", "bronze"];

/// A mod's own atlas: the PNG `bytes` cut into `[campaign-medals] tiers` rows (and three
/// difficulty blocks when `per-difficulty`), under a sheet key of the mod's own.
fn resolve_mod(
    mod_id: &str,
    key: &str,
    medals: &oag_mod::CampaignMedals,
    bytes: Option<&[u8]>,
) -> Result<Resolved, String> {
    let bytes = bytes.ok_or_else(|| format!("{mod_id}: icon {key:?} could not be read"))?;
    let png = oag_texture::replacer::decode_png(bytes).ok_or_else(|| {
        format!("{mod_id}: icon {key:?} is not a PNG this build draws (an SVG atlas is not)")
    })?;
    let rows = if medals.tiers.is_empty() {
        DEFAULT_ROWS.map(String::from).to_vec()
    } else {
        medals.tiers.clone()
    };
    let image = format!("mod:{mod_id}/{key}");
    let atlas = Atlas::of_mod(
        image.clone(),
        medals.per_difficulty,
        rows,
        (png.width, png.height),
    )
    .map_err(|e| format!("{mod_id}: icon {key:?}: {e}"))?;
    Ok(Resolved {
        art: MedalArt::Atlas(atlas),
        blobs: Vec::new(),
        images: vec![oag_hud::sprite::DecodedImage {
            src: image,
            width: png.width,
            height: png.height,
            rgba: png.rgba,
            quad_extent: None,
            quad_centre: None,
            blend: None,
        }],
    })
}

/// Resolves `medals`, asked for by mod `mod_id`, against the discs in `rows`.
///
/// # Errors
///
/// Why the swatch stays: the source's disc is not here, the source is one this build does not
/// draw, or its atlas lacks the blocks the mod asked for.
pub fn resolve(
    ask: &super::custom::MedalAsk,
    rows: &[crate::launcher::Candidate],
) -> Result<Resolved, String> {
    let (mod_id, medals) = (ask.mod_id.as_str(), &ask.medals);
    let slug = match &medals.source {
        oag_mod::MedalSource::Mod(key) => {
            return resolve_mod(mod_id, key, medals, ask.atlas.as_deref());
        }
        oag_mod::MedalSource::Title(slug) => slug,
    };
    let title = oag_source::title::by_slug(slug)
        .ok_or_else(|| format!("{mod_id}: no title is called {slug:?}"))?;
    if title.campaign.dialect != oag_title::CampaignDialect::Hd {
        return Err(format!(
            "{mod_id}: {} ships no medal atlas this build reads",
            title.name
        ));
    }
    let row = crate::launcher::distinct_titles(rows)
        .into_iter()
        .find(|row| {
            row.playable()
                .is_some_and(|candidate| candidate.slug() == *slug)
        })
        .ok_or_else(|| format!("{mod_id}: needs a {} disc, and none is here", title.name))?;
    let mut archives = oag_source::title::open_source(&row.source, Vec::new(), Vec::new())
        .map_err(|e| format!("{mod_id}: {}: {e}", row.source))?
        .archives;
    let blob = super::read_hd_texture(&mut archives, oag_source::title::MEDAL_ATLAS)
        .map_err(|e| format!("{mod_id}: {}: {e}", oag_source::title::MEDAL_ATLAS))?;
    if medals.per_difficulty {
        let height = oag_texture::gtf::Gtf::parse(&blob)
            .ok()
            .and_then(|gtf| gtf.textures.first().map(|texture| texture.height));
        if height.is_none_or(|height| height < PER_DIFFICULTY_HEIGHT) {
            return Err(format!(
                "{mod_id}: {}'s medal atlas has no block per difficulty",
                title.name
            ));
        }
    }
    let image = oag_source::title::MEDAL_ATLAS.to_string();
    Ok(Resolved {
        art: MedalArt::Atlas(Atlas::of_title(
            image.clone(),
            medals.per_difficulty,
            title.campaign.medal_rows(),
        )),
        blobs: vec![(image, blob)],
        images: Vec::new(),
    })
}

/// The art the screens of the campaign `open` (`None`: the disc's own) draw medals with for
/// `installed`, and `sprites` extended with its texture; the swatch, with the reason logged,
/// when no mod asks for art there or the mod's art cannot be had. `rows` surveys the discs,
/// asked for only when a mod asks for a disc's art.
pub fn apply(
    installed: &super::custom::Installed,
    open: Option<&super::custom::Custom>,
    rows: impl FnOnce() -> Vec<crate::launcher::Candidate>,
    sprites: &mut oag_hud::sprite::Sheet,
) -> MedalArt {
    let Some(ask) = installed.medals_for(open) else {
        return MedalArt::default();
    };
    let mod_id = &ask.mod_id;
    let rows = match ask.medals.source {
        oag_mod::MedalSource::Title(_) => rows(),
        oag_mod::MedalSource::Mod(_) => Vec::new(),
    };
    match resolve(ask, &rows) {
        Ok(resolved) => {
            let mut report = Vec::new();
            *sprites = sprites.extended_with(&resolved.blobs, resolved.images, &mut report);
            oag_raceplay::loader_log::lines(
                report.iter().map(|line| format!("campaign medals {line}")),
            );
            log::debug!("campaign medals: {mod_id} draws {:?}", resolved.art);
            resolved.art
        }
        Err(reason) => {
            log::warn!("campaign medals: {reason}");
            MedalArt::default()
        }
    }
}
