//! Pulse's caustic pass: reading the sixteen ripple frames for a circuit whose `.vex` flags
//! batches for it (`oag_mesh::mesh::caustic_pass`).

use std::sync::Arc;

use oag_mesh::mesh::ModelTexture;
use oag_mesh::mesh::caustic_pass::{self, FRAMES};

use super::Loaded;

/// Reads one frame as a model texture: through `oag_pulse::read_image`, which finds the PS2
/// disc's `.pct` under the `.mip` name.
fn frame(archives: &mut oag_assets::Archives, index: usize) -> Result<Arc<ModelTexture>, String> {
    let name = caustic_pass::frame_name(index);
    let blob = oag_pulse::read_image(archives, &name).map_err(|e| format!("{name}: {e}"))?;
    let picture = oag_texture::decode::decode_sniffed(&blob).map_err(|e| format!("{name}: {e}"))?;
    Ok(Arc::new(ModelTexture {
        label: name,
        width: picture.width,
        height: picture.height,
        texels: oag_mesh::mesh::Texels::Rgba8(picture.rgba),
        mip_count: None,
    }))
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
        match (0..FRAMES).map(|i| frame(archives, i)).collect() {
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
