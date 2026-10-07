//! A circuit's whole scene - [`build_scene`] - and the two pictures it is
//! drawn into on Vineta K: the main view, and the behind-the-glass target its
//! tunnel glass reads ([`build_scene_views`]).
//!
//! # The behind-the-glass target
//!
//! A chunk whose render flag has `0x10` (`rcsmodel::RENDER_BEHIND_GLASS`) is
//! drawn by the original only into a 640x360 target of its own, with the sky,
//! never in the main view; the tunnel glass samples that target. Bit `0x20`
//! (`rcsmodel::RENDER_ALTERNATE_FOG`) picks the circuit's alternate fog pair
//! for a chunk. Both are measured on Vineta K's RPCS3 captures, each draw tied
//! to its chunk by vertex offset (confidence 88,
//! `docs/ghidra/functions/ps3-hdfury-eu/visibility.md`). Inside the target the
//! two bits split the chunks into two fog groups, so the target is built as
//! two models, each drawn under one fog: on Vineta K 198 chunks per direction
//! with both bits and 12 with `0x10` alone.
//!
//! **One material decode for all three models.** Decoding a circuit's
//! textures is most of a build's cost, so the two target models reuse the
//! main view's tables ([`Setup::Copy`]) and are then trimmed to the texture
//! slots their own draws name, which keeps the GPU from holding a second copy
//! of every texture.

use anyhow::{Context, Result};
use oag_rcs::rcsmodel;
use oag_vex::vex;

use super::{Model, Report, Textures, bounding_sphere, build_with_options, face_normals, pads};

/// Which of a circuit's chunks one build draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// The main view: every chunk but the behind-the-glass ones.
    Main,
    /// One fog group of the behind-the-glass target: the chunks with
    /// `rcsmodel::RENDER_BEHIND_GLASS` whose `RENDER_ALTERNATE_FOG` is
    /// `alternate_fog`.
    BehindGlass {
        /// Whether this group is fogged with the alternate pair.
        alternate_fog: bool,
    },
}

impl View {
    /// Whether a chunk belongs to this view.
    pub(super) fn draws(self, chunk: &rcsmodel::Mesh) -> bool {
        match self {
            Self::Main => !chunk.is_behind_glass(),
            Self::BehindGlass { alternate_fog } => {
                chunk.is_behind_glass() && chunk.uses_alternate_fog() == alternate_fog
            }
        }
    }
}

/// Where a build's material tables come from.
pub(super) enum Setup<'a> {
    /// Decoded from the file through the caller's texture reader.
    Decode(Textures<'a>),
    /// Read back off a model already built from the same file.
    Copy(&'a Model),
}

impl Setup<'_> {
    /// The same source, borrowed for one call.
    fn reborrow(&mut self) -> Setup<'_> {
        match self {
            Setup::Decode(textures) => Setup::Decode(&mut **textures),
            Setup::Copy(model) => Setup::Copy(model),
        }
    }
}

/// The behind-the-glass target's two fog groups, each `None` when the circuit
/// has no chunk in it - every circuit but Vineta K.
#[derive(Debug, Default)]
pub struct BehindGlass {
    /// The chunks fogged with the circuit's alternate pair.
    pub alternate_fog: Option<Model>,
    /// The chunks fogged with the primary pair.
    pub primary_fog: Option<Model>,
}

impl BehindGlass {
    /// Whether the circuit draws anything behind its glass.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.alternate_fog.is_none() && self.primary_fog.is_none()
    }
}

/// The whole of a PS3 model: the meshes its `.vex` places, and the geometry
/// nothing in the `.vex` mentions.
///
/// # Two passes, because Wipeout HD authors two kinds of geometry
///
/// **A craft is all first pass and a circuit is almost all second.** Every one
/// of Assegai's 15 `Mesh` nodes addresses a chunk, and its positions are in the
/// node's own space - the PSP arrangement with the vertices moved out. All 126
/// of Talon's Junction's `Mesh` nodes are *props*: blimps, girders, sky
/// traffic. The road, the walls, the scenery and both kinds of pad are among
/// the 913 of 983 chunks no `Mesh` node addresses, each carrying a
/// **world-space** bias, drawn without a node transform because there is no
/// node - a `Weapon Pad` or `Speedup Pad` node names its chunk at the mesh
/// payload's own `+0x30`, but as [`referenced`] records, the chunk's
/// coordinates ignore the node anyway.
///
/// So a circuit that drew only the first pass drew its skybox traffic and no
/// track, which is exactly what this looked like before the second existed.
///
/// **56 of Talon's Junction's prop nodes stay honestly absent.** Their hashes
/// are in no `.rcsmodel` on the disc except *other environments'* - the same
/// `tanker1aShape` hash appears in Amphiseum's and Tech De Ra's own track
/// models, so the hash is content-derived and those donors were simply never
/// baked into this circuit's file. The sky traffic that is visible here is
/// the world-space `animating_traffic` chunks, which the second pass draws.
///
/// **Both pad classes' chunks are excluded from this pass entirely**, not
/// drawn into a third bucket - [`pads::build_pads`]/[`pads::build_weapon_pads`]
/// draw them through their own node-ordered pass instead, which is what a
/// tintable, gameplay `Drawable` needs and this pass cannot give: see that
/// pair's own doc comment for why. A caller that wants one merged picture
/// regardless - the viewer's `--mesh`/`--track` - draws them back in itself;
/// see `oag_view::ps3_mesh::with_pads`.
///
/// # Errors
///
/// As [`build`].
pub fn build_scene(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    textures: Textures<'_>,
) -> Result<(Model, Report)> {
    build_view(label, data, model_blob, Setup::Decode(textures), View::Main)
}

/// [`build_scene`], plus the behind-the-glass target's two models - see this
/// module's doc. The report is the main view's.
///
/// # Errors
///
/// As [`build_scene`].
pub fn build_scene_views(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    textures: Textures<'_>,
) -> Result<(Model, BehindGlass, Report)> {
    let (main, report) = build_scene(label, data, model_blob, textures)?;
    if report.behind_glass == 0 {
        return Ok((main, BehindGlass::default(), report));
    }
    let group = |alternate_fog| -> Result<Option<Model>> {
        let view = View::BehindGlass { alternate_fog };
        let (mut model, _) = build_view(label, data, model_blob, Setup::Copy(&main), view)?;
        if model.draws.is_empty()
            && model.alpha_tested_draws.is_empty()
            && model.transparent_draws.is_empty()
        {
            return Ok(None);
        }
        keep_drawn_textures(&mut model);
        single_sided(&mut model);
        Ok(Some(model))
    };
    let behind_glass = BehindGlass {
        alternate_fog: group(true)?,
        primary_fog: group(false)?,
    };
    Ok((main, behind_glass, report))
}

/// Draws every chunk of a target model single-sided, as the original draws
/// the target's: all 35 chunk draws of Vineta K's pose-A capture have face
/// culling on (`0x183c` = 1, back faces, counter-clockwise front), where the
/// main view culls 109 of its 136 draws and this renderer culls no HD chunk.
/// It matters here and not there because the target's eye is inside shells
/// authored to be seen from outside: drawn two-sided, the tunnel's outer
/// concrete covers the target.
fn single_sided(model: &mut Model) {
    for draw in model
        .draws
        .iter_mut()
        .chain(&mut model.alpha_tested_draws)
        .chain(&mut model.transparent_draws)
    {
        draw.culled = true;
    }
}

/// Empties every texture slot no draw of `model` names, keeping the slots in
/// place because a draw names its material by position.
fn keep_drawn_textures(model: &mut Model) {
    let mut drawn = vec![false; model.textures.len()];
    let lists = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
        &model.shine_draws,
    ];
    for draw in lists.into_iter().flatten() {
        if let Some(slot) = draw.texture.and_then(|t| drawn.get_mut(t)) {
            *slot = true;
        }
    }
    for slots in [
        &mut model.textures,
        &mut model.lightmaps,
        &mut model.pad_masks,
        &mut model.wave_maps,
    ] {
        for (slot, texture) in slots.iter_mut().enumerate() {
            if !drawn.get(slot).copied().unwrap_or(false) {
                *texture = None;
            }
        }
    }
}

/// [`build_scene`] for one [`View`].
fn build_view(
    label: &str,
    data: &[u8],
    model_blob: &[u8],
    mut setup: Setup<'_>,
    view: View,
) -> Result<(Model, Report)> {
    let (mut out, mut report) = build_with_options(
        label,
        data,
        model_blob,
        setup.reborrow(),
        |c| c.mesh,
        true,
        view,
    )?;
    let model = rcsmodel::Model::parse(model_blob)
        .map_err(|e| anyhow::anyhow!("{label}: the .rcsmodel beside it: {e}"))?;
    let nodes = vex::nodes(data).context("walking the node tree")?;
    let classes = vex::classes_of(data).ok();
    classes
        .and_then(|c| c.mesh)
        .context("no mesh class id for this .vex version")?;
    let order = vex::byte_order(data);
    let placed = pads::placed_hashes(data, &nodes, classes, order, &model);

    // The pad masks are the main view's; a copied setup already carries them.
    if let Setup::Decode(textures) = setup {
        pads::bind_scene_pad_masks(&model, &placed, &mut out, textures, &mut report);
    }

    for (chunk_index, chunk) in model.meshes.iter().enumerate() {
        if placed.contains(&chunk.hash) {
            continue;
        }
        if pads::emit_chunk(
            &mut out,
            &model,
            model_blob,
            chunk_index,
            chunk,
            view,
            &mut report,
        ) {
            report.unreferenced += 1;
            out.mesh_count += 1;
        }
    }

    face_normals(&mut out);
    let (centre, radius) = bounding_sphere(&out.vertices);
    out.centre = centre;
    out.radius = radius;

    Ok((out, report))
}
