//! A track's own loading screen: what its manifest's `[loading]` table supplies, read and ready
//! to lay over the running title's screen.
//!
//! The manifest names files and texts ([`oag_trackfile::Loading`]); this reads the files from the
//! track's own folder and decodes the picture. Each thing falls back on its own to the title's:
//! no layout means the title's file, no caption means the circuit's own name, and naming any of
//! the feature's texts or its picture makes the track the feature (the title's random draw is
//! not run and a key left out draws blank). A file that will not read or parse is said in
//! [`TrackLoading::notes`] and that one thing falls back; nothing is replaced by a stand-in.

use std::path::Path;

use oag_hud::loading_layout::LoadingLayouts;
use oag_hud::sprite::{DecodedImage, Sheet};

use super::{Assets, Screen};

/// The sheet key of the track's picture.
const IMAGE_KEY: &str = "track:feature-image";

/// What a track supplies of its loading screen, read.
#[derive(Debug, Clone, Default)]
pub struct TrackLoading {
    /// The track's layout file, parsed.
    pub layout: Option<LoadingLayouts>,
    /// The circuit's name on the caption line, instead of the catalogue's.
    pub caption: Option<String>,
    /// The feature's name, heading and paragraph.
    pub feature: [Option<String>; 3],
    /// Whether the track is the feature: it named a picture or any of the three texts.
    pub is_the_feature: bool,
    /// The decoded picture.
    pub image: Option<DecodedImage>,
    /// What did not read, for the loader report.
    pub notes: Vec<String>,
}

impl TrackLoading {
    /// Reads what `loading` names from `folder`, the manifest's own.
    #[must_use]
    pub fn read(folder: &Path, loading: &oag_trackfile::Loading) -> Self {
        let mut notes = Vec::new();
        let layout = loading.layout.as_ref().and_then(|file| {
            let path = folder.join(file);
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("{}: {e}", path.display()))
                .and_then(|text| {
                    LoadingLayouts::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
                });
            match text {
                Ok(layouts) => Some(layouts),
                Err(why) => {
                    notes.push(format!("track loading layout: {why}; the title's stands"));
                    None
                }
            }
        });
        let image = loading.feature_image.as_ref().and_then(|file| {
            let path = folder.join(file);
            match decode_png(&path) {
                Ok(image) => Some(image),
                Err(why) => {
                    notes.push(format!(
                        "track loading picture {}: {why}; it draws none",
                        path.display()
                    ));
                    None
                }
            }
        });
        Self {
            layout,
            caption: loading.caption.clone(),
            feature: [
                loading.feature_title.clone(),
                loading.feature_heading.clone(),
                loading.feature_text.clone(),
            ],
            is_the_feature: loading.supplies_feature(),
            image,
            notes,
        }
    }

    /// The title's sheet with the track's picture under it, and where the picture sits; `None`
    /// when the track has no picture to add.
    #[must_use]
    pub fn sheet(&self, assets: &Assets) -> Option<(Sheet, [f32; 4])> {
        let image = self.image.clone()?;
        let base = assets.art.as_ref().map(|art| &art.sheet)?;
        let mut notes = Vec::new();
        let sheet = base.extended_with(&[], vec![image], &mut notes);
        let placed = sheet.get(IMAGE_KEY)?;
        Some((
            sheet,
            placed.sheet_rect([0.0, 0.0, placed.width as f32, placed.height as f32]),
        ))
    }
}

/// An author's PNG as RGBA8.
fn decode_png(path: &Path) -> Result<DecodedImage, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let frame = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let pixels = &buf[..frame.buffer_size()];
    let rgba: Vec<u8> = match frame.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 0xff])
            .collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[g, a]| [g, g, g, a])
            .collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g, 0xff]).collect(),
        png::ColorType::Indexed => return Err("an indexed PNG survived expansion".into()),
    };
    Ok(DecodedImage {
        src: IMAGE_KEY.to_string(),
        width: frame.width,
        height: frame.height,
        rgba,
        quad_extent: None,
        quad_centre: None,
        blend: None,
    })
}

impl Screen {
    /// Lays a track's own loading screen over this one. `picture` is where the track's picture
    /// sits in the sheet the renderer was built with ([`TrackLoading::sheet`]).
    pub fn apply_track(&mut self, track: &TrackLoading, picture: Option<[f32; 4]>) {
        if let Some(layouts) = &track.layout {
            self.layouts = Some(layouts.clone());
        }
        if let Some(caption) = &track.caption {
            self.circuit = Some(caption.clone());
        }
        if track.is_the_feature {
            let [title, heading, text] = track.feature.clone();
            self.feature_slot = None;
            self.feature_title = title;
            self.feature_heading = heading;
            self.feature_text = text;
            self.illustration = picture;
        }
    }
}
