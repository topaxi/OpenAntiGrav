//! Pulse's caustic pass: reading the sixteen ripple frames for a circuit whose `.vex` flags
//! batches for it (`oag_mesh::mesh::caustic_pass`).

use std::sync::Arc;

use oag_mesh::mesh::ModelTexture;
use oag_mesh::mesh::caustic_pass::{self, FRAMES};

use super::Loaded;

/// Reads one frame as a model texture: by its `.mip` name, then the PS2 disc's `.pct` rewrite of it
/// (`Texture_FindOrLoad`).
fn frame(archives: &mut oag_assets::Archives, index: usize) -> Result<Arc<ModelTexture>, String> {
    let name = caustic_pass::frame_name(index);
    let blob = archives
        .read_name(&name)
        .or_else(|first| {
            oag_formats::wad::ps2_texture_name(&name)
                .map_or(Err(first), |ps2| archives.read_name(&ps2))
        })
        .map_err(|e| format!("{name}: {e}"))?;
    let picture = oag_texture::decode::decode_sniffed(&blob).map_err(|e| format!("{name}: {e}"))?;
    Ok(Arc::new(ModelTexture {
        label: name,
        width: picture.width,
        height: picture.height,
        texels: oag_mesh::mesh::Texels::Rgba8(picture.rgba),
        mip_count: None,
        lod_code: 0.0,
    }))
}

/// Frame `index` among the circuit's own embedded textures, which is where Pure keeps them: the
/// `.vex` embeds `Z:\\data\\Tex\\caustics\\save.NN.tga` and a model texture's label is the file
/// name alone (`docs/ghidra/functions/psp-pure-usa/caustic-pass.md`).
fn embedded_frame(
    textures: &[Option<Arc<ModelTexture>>],
    index: usize,
) -> Option<Arc<ModelTexture>> {
    let wanted = format!("save.{:02}.tga", index * 2 + 1);
    textures
        .iter()
        .flatten()
        .find(|t| t.label.eq_ignore_ascii_case(&wanted))
        .cloned()
}

/// Gives the circuit's flagged batches their frames, or drops them. Silent for a circuit that
/// flags none; otherwise the loader report says what was drawn, or why nothing is. `enabled` is
/// the geometry being the `.vex`'s and the player's `graphics.water` not being `off`; the title's rule
/// is read here.
pub(super) fn attach(
    loaded: &mut Loaded,
    archives: &mut oag_assets::Archives,
    title: &oag_title::Title,
    enabled: bool,
) {
    if loaded.track_model.caustic.draws.is_empty() {
        return;
    }
    let enabled = enabled && title.looks.track_caustic.applies(archives.layout.platform);
    if enabled {
        let found = (0..FRAMES)
            .map(|i| {
                frame(archives, i)
                    .or_else(|first| embedded_frame(&loaded.track_model.textures, i).ok_or(first))
            })
            .collect();
        match found {
            Ok(frames) => loaded.track_model.caustic.frames = frames,
            Err(e) => loaded.report.push(format!(
                "caustic pass: not drawn, a frame did not load ({e})"
            )),
        }
    }
    let line = match caustic_pass::build(&loaded.track_model) {
        Some(pass) => format!(
            "caustic pass: {} batches drawn over {FRAMES} frames, {} left undrawn (animated nodes)",
            pass.per_frame, pass.skipped
        ),
        None if enabled => format!(
            "caustic pass: none drawn of {} flagged batches",
            loaded.track_model.caustic.draws.len()
        ),
        None => format!(
            "caustic pass: off, {} flagged batches left as the surface draws them",
            loaded.track_model.caustic.draws.len()
        ),
    };
    loaded.report.push(line);
    if loaded.track_model.caustic.frames.is_empty() {
        loaded.track_model.caustic.draws.clear();
    }
}
