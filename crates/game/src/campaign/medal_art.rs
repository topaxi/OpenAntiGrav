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
    /// The texture `art` samples, by the sheet key it names.
    pub blobs: Vec<(String, Vec<u8>)>,
}

/// Resolves `medals`, asked for by mod `mod_id`, against the discs in `rows`.
///
/// # Errors
///
/// Why the swatch stays: the source's disc is not here, the source is one this build does not
/// draw, or its atlas lacks the blocks the mod asked for.
pub fn resolve(
    mod_id: &str,
    medals: &oag_mod::CampaignMedals,
    rows: &[crate::launcher::Candidate],
) -> Result<Resolved, String> {
    let oag_mod::MedalSource::Title(slug) = &medals.source else {
        return Err(format!(
            "{mod_id}: a mod's own atlas is not drawn yet, the title's swatch stays"
        ));
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
    let blob = super::read_hd_texture(&mut archives, oag_hd::campaign::MEDAL_ATLAS)
        .map_err(|e| format!("{mod_id}: {}: {e}", oag_hd::campaign::MEDAL_ATLAS))?;
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
    let image = oag_hd::campaign::MEDAL_ATLAS.to_string();
    Ok(Resolved {
        art: MedalArt::Atlas(Atlas {
            image: image.clone(),
            per_difficulty: medals.per_difficulty,
            tier_rows: title.campaign.medal_rows(),
        }),
        blobs: vec![(image, blob)],
    })
}

/// The art the campaign screens draw medals with for `installed`, and `sprites` extended with
/// its texture; the swatch, with the reason logged, when the mod's art cannot be had.
/// `rows` surveys the discs, asked for only when a mod asks for art.
pub fn apply(
    installed: &super::custom::Installed,
    rows: impl FnOnce() -> Vec<crate::launcher::Candidate>,
    sprites: &mut oag_hud::sprite::Sheet,
) -> MedalArt {
    let Some((mod_id, medals)) = &installed.medals else {
        return MedalArt::default();
    };
    match resolve(mod_id, medals, &rows()) {
        Ok(resolved) => {
            let mut report = Vec::new();
            *sprites = sprites.extended(&resolved.blobs, &mut report);
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
