//! Reading the cards' system art off the render thread and packing it into the
//! chooser's sprite sheet.

use oag_hud::sprite::{DecodedImage, Sheet};
use oag_source::art::{Art, Picture};

use super::cards::{COG_KEY, logo_key, picture_key};

/// The widest a picture is kept. A card is at most about 600 pixels across on a
/// 1080p screen; the 1920-wide ones are averaged down so the sheet's bilinear
/// sampling does not alias them.
const ART_WIDTH: u32 = 800;

/// The cog's raster size, a good deal over its size on screen.
const COG_PIXELS: u32 = 48;

const COG_SVG: &[u8] = include_bytes!("../../../../assets/ui/launcher-cog.svg");

/// What came back from [`load`].
#[derive(Debug)]
pub struct Loaded {
    /// Source to the names of the pictures it carries.
    pub pictures: Vec<(String, Vec<String>)>,
    /// Every picture and logo, with the cog.
    pub sheet: Sheet,
}

/// Reads every source's art and builds the sheet. Files and decoding: call it
/// from a worker.
#[must_use]
pub fn load(sources: &[String]) -> Loaded {
    load_with(sources, oag_source::art::read)
}

/// [`load`] with the reader handed in.
pub fn load_with(sources: &[String], read: impl Fn(&str) -> Vec<Art>) -> Loaded {
    let mut pictures = Vec::new();
    let mut images = Vec::new();
    for source in sources {
        let found = read(source);
        pictures.push((
            source.clone(),
            found.iter().map(|art| art.name.to_string()).collect(),
        ));
        for art in found {
            images.push(decoded(
                picture_key(source, art.name),
                downscale(art.picture, ART_WIDTH),
            ));
            if let Some(logo) = art.logo {
                images.push(decoded(
                    logo_key(source, art.name),
                    downscale(logo, ART_WIDTH),
                ));
            }
        }
    }
    images.extend(cog());
    let mut report = Vec::new();
    let sheet = Sheet::build_with(&[], images, &mut report);
    for line in report {
        log::debug!("launcher art: {line}");
    }
    Loaded { pictures, sheet }
}

fn decoded(src: String, picture: Picture) -> DecodedImage {
    DecodedImage {
        src,
        width: picture.width,
        height: picture.height,
        rgba: picture.rgba,
        quad_extent: None,
        quad_centre: None,
        blend: None,
    }
}

fn cog() -> Option<DecodedImage> {
    let icon = oag_hud::svg::VectorIcon::parse(COG_KEY.to_string(), COG_SVG)
        .inspect_err(|e| log::warn!("the launcher's cog will not parse: {e}"))
        .ok()?;
    let rgba = icon.rasterise(COG_PIXELS, COG_PIXELS)?;
    Some(decoded(
        COG_KEY.to_string(),
        Picture {
            width: COG_PIXELS,
            height: COG_PIXELS,
            rgba,
        },
    ))
}

/// `picture` no wider than `limit`, each new pixel the average of the source
/// pixels it covers.
fn downscale(picture: Picture, limit: u32) -> Picture {
    if picture.width <= limit {
        return picture;
    }
    let width = limit;
    let height = (u64::from(picture.height) * u64::from(width) / u64::from(picture.width)) as u32;
    let height = height.max(1);
    let span = |at: u32, from: u32, to: u32| {
        let start = (u64::from(at) * u64::from(from) / u64::from(to)) as u32;
        let end = (u64::from(at + 1) * u64::from(from) / u64::from(to)) as u32;
        start..end.max(start + 1).min(from)
    };
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let mut sum = [0u32; 4];
            let mut count = 0u32;
            for sy in span(y, picture.height, height) {
                for sx in span(x, picture.width, width) {
                    let at = ((sy * picture.width + sx) * 4) as usize;
                    for (total, value) in sum.iter_mut().zip(&picture.rgba[at..at + 4]) {
                        *total += u32::from(*value);
                    }
                    count += 1;
                }
            }
            rgba.extend(sum.map(|total| ((total + count / 2) / count) as u8));
        }
    }
    Picture {
        width,
        height,
        rgba,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wide_picture_is_averaged_down_and_a_narrow_one_kept() {
        let picture = Picture {
            width: 4,
            height: 2,
            rgba: [
                [0, 0, 0, 255],
                [100, 0, 0, 255],
                [10, 10, 10, 255],
                [30, 30, 30, 255],
                [0, 0, 0, 255],
                [100, 0, 0, 255],
                [10, 10, 10, 255],
                [30, 30, 30, 255],
            ]
            .concat(),
        };
        let half = downscale(picture.clone(), 2);
        assert_eq!((half.width, half.height), (2, 1));
        assert_eq!(half.rgba, [50, 0, 0, 255, 20, 20, 20, 255]);
        assert_eq!(downscale(picture.clone(), 8).rgba, picture.rgba);
    }
}
