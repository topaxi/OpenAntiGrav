//! Pure's sun lens flare at load: the circuit's `lensflare` node and its own
//! `LensflareComponents.tga`, read into [`oag_fx::lens_flare::Setup`].
//!
//! The title's `Looks::lens_flare` rule is the law and the node is the trigger; a
//! circuit with no node, a node that will not parse, or an atlas that will not decode
//! draws no flare and says so in the loader report. See `docs/rendering/lens-flare.md`.

use oag_fx::lens_flare::{Atlas, Setup};

/// The atlas's asset path as the circuit's `Texture` node spells it, compared without
/// case on the file name.
const ATLAS: &str = "LensflareComponents.tga";

/// The flare of a circuit file, or `None` (with the reason in `report`) when it draws
/// none.
pub(super) fn read(track_blob: &[u8], report: &mut Vec<String>) -> Option<Setup> {
    let nodes = oag_vex::vex::nodes(track_blob).unwrap_or_default();
    let flares = oag_vex::lensflare::lensflares(track_blob, &nodes);
    let Some(flare) = flares.first() else {
        report.push("lens flare: the circuit authors no lensflare node, none drawn".into());
        return None;
    };
    if flare.direction == [0.0; 3] {
        report.push("lens flare: the node's anchor is zero, none drawn".into());
        return None;
    }
    let textures = oag_vex::vex::textures(track_blob).unwrap_or_default();
    let Some(texture) = textures.iter().flatten().find(|t| {
        t.asset_path.as_deref().is_some_and(|p| {
            p.rsplit(['\\', '/'])
                .next()
                .is_some_and(|f| f.eq_ignore_ascii_case(ATLAS))
        })
    }) else {
        report.push(format!(
            "lens flare: the circuit carries no {ATLAS}, none drawn"
        ));
        return None;
    };
    let levels = texture.levels_rgba();
    report.push(format!(
        "lens flare: sun direction ({:.3}, {:.3}, {:.3}), colour ({:.3}, {:.3}, {:.3}), \
         {ATLAS} {}x{}, {} authored level(s); ten additive quads after the scene, rays \
         and rings as the law, random factors fixed at the engine flag's values",
        flare.direction[0],
        flare.direction[1],
        flare.direction[2],
        flare.colour[0],
        flare.colour[1],
        flare.colour[2],
        texture.width,
        texture.height,
        levels.len(),
    ));
    Some(Setup {
        direction: flare.direction,
        colour: flare.colour,
        atlas: Atlas {
            width: u32::from(texture.width),
            height: u32::from(texture.height),
            levels,
        },
    })
}
