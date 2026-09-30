//! The card's shell: what the placeholder `Data/FE/Flyers/00_flyer.vex` says
//! the card is shaped like, and how its reflection fades.
//!
//! **The widget's `Src` is not a stand-in for the flyer, it is the card.**
//! `Flyer_Item` draws the model `Src` names - two surfaces named `cardShape`
//! and `card_reflectShape`, one material (`cf_basicvertexalpha`), two textures
//! `flyer_dummy_front.gtf` and `flyer_dummy_back.gtf` - and the executable's
//! own format string `flyer_dummy_` is what names the textures it swaps for
//! each flyer's picture. So the card is a rectangle with a chamfered corner
//! and a few notches, 102.4 by 66.6 units, that shows the flyer's camera image
//! (`super::clip`), and a half-height reflection of it that fades out.
//!
//! What is read here, off the `.rcsmodel`'s inline chunks (18-byte vertices:
//! three `s16` positions, a packed normal, two `half` texture coordinates and
//! four colour bytes - `docs/formats/rcsmodel.md`, "The placeholder's two
//! surfaces"):
//!
//! - **the outline**: `cardShape`'s front face, the triangles at `z = +0.2`,
//!   twenty-five of them covering 6,783.6 square units of the 6,819.8 the
//!   rectangle has (the chamfer and the notches are the difference);
//! - **the reflection's fade**: `card_reflectShape`'s vertex alpha, `0x4c`
//!   at the card's bottom edge, `0x26` a third of the way down and `0` at
//!   the bottom, half the card's height below.
//!
//! The shell's `.vex` and `.rcsmodel` are the disc's own; nothing here is a
//! constant typed in from them.

use anyhow::{Context, Result};

use super::clip::Fade;

/// The placeholder's own entry, `Data\FE\Flyers\00_flyer.vex` in the
/// widget's `Src`.
pub const ENTRY: &str = "Data/FE/Flyers/00_flyer.vex";

/// The card's shape and its reflection, in card units centred on the card.
#[derive(Debug, Clone, PartialEq)]
pub struct Shell {
    /// The front face's triangles.
    pub outline: Vec<[[f32; 2]; 3]>,
    /// How the reflection fades below the outline's bottom edge.
    pub fade: Fade,
    /// The card's half width and half height.
    pub half_size: [f32; 2],
}

impl Shell {
    /// Reads the shell off `archives`.
    ///
    /// # Errors
    ///
    /// The placeholder is missing, will not decode, or lacks either surface.
    pub fn load(archives: &mut oag_assets::Archives) -> Result<Self> {
        let blob = archives
            .read_name(ENTRY)
            .with_context(|| format!("reading {ENTRY}"))?;
        let nodes = oag_vex::vex::nodes(&blob).context("reading the shell's scene tree")?;
        let (mut model, _) = crate::preview::model_named(archives, ENTRY)?;
        super::clip::bake(&mut model, 0.0);
        let named = |name: &str| {
            let draws = model
                .draws
                .iter()
                .chain(&model.alpha_tested_draws)
                .chain(&model.transparent_draws);
            let draw = draws.into_iter().find(|draw| {
                draw.node
                    .and_then(|node| nodes.get(node as usize))
                    .and_then(|node| node.name.as_deref())
                    == Some(name)
            });
            draw.map(|draw| {
                let range = draw.range.start as usize..draw.range.end as usize;
                model.indices[range]
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .map(|triangle| triangle.map(|i| model.vertices[i as usize]))
                    .collect::<Vec<_>>()
            })
            .with_context(|| format!("the shell has no {name}"))
        };
        let reflect_alphas = reflect_alphas(archives, &blob, &nodes)?;
        Self::from_surfaces(
            &named("cardShape")?,
            &named("card_reflectShape")?,
            reflect_alphas,
        )
    }

    fn from_surfaces(
        card: &[[oag_render::mesh::GpuVertex; 3]],
        reflect: &[[oag_render::mesh::GpuVertex; 3]],
        reflect_alphas: (f32, f32),
    ) -> Result<Self> {
        let front = |triangle: &&[oag_render::mesh::GpuVertex; 3]| {
            triangle.iter().all(|v| v.position[2] > 0.0)
        };
        let corners = card.iter().flatten();
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for v in corners {
            for k in 0..2 {
                lo[k] = lo[k].min(v.position[k]);
                hi[k] = hi[k].max(v.position[k]);
            }
        }
        anyhow::ensure!(hi[0] > lo[0] && hi[1] > lo[1], "cardShape has no extent");
        let centre = [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0];
        let outline: Vec<[[f32; 2]; 3]> = card
            .iter()
            .filter(front)
            .map(|triangle| {
                triangle.map(|v| [v.position[0] - centre[0], v.position[1] - centre[1]])
            })
            .collect();
        anyhow::ensure!(!outline.is_empty(), "cardShape has no front face");

        let (reflect_lo, reflect_hi) = reflect
            .iter()
            .flatten()
            .fold((f32::MAX, f32::MIN), |(lo, hi), v| {
                (lo.min(v.position[1]), hi.max(v.position[1]))
            });
        anyhow::ensure!(reflect_hi > reflect_lo, "card_reflectShape has no extent");
        Ok(Self {
            outline,
            fade: Fade {
                edge_y: lo[1] - centre[1],
                depth: reflect_hi - reflect_lo,
                top_alpha: reflect_alphas.1,
            },
            half_size: [(hi[0] - lo[0]) / 2.0, (hi[1] - lo[1]) / 2.0],
        })
    }
}

/// The reflection's vertex alpha at its far end and its near end, `(bottom,
/// top)`, read off the `.rcsmodel`'s `card_reflectShape` chunk - which the
/// scene builder does not carry (an HD chunk's vertex colour is a baked light
/// there, and zero).
fn reflect_alphas(
    archives: &mut oag_assets::Archives,
    vex: &[u8],
    nodes: &[oag_vex::vex::Node],
) -> Result<(f32, f32)> {
    let sibling = oag_render::mesh::rcs::sibling_name(ENTRY)
        .with_context(|| format!("{ENTRY} names no .rcsmodel"))?;
    let geometry = archives
        .read_name(&sibling)
        .with_context(|| format!("reading {sibling}"))?;
    let model = oag_rcs::rcsmodel::Model::parse(&geometry).context("reading the shell's chunks")?;
    let node = nodes
        .iter()
        .find(|node| node.name.as_deref() == Some("card_reflectShape"))
        .context("the shell has no card_reflectShape")?;
    // The node's `+0x30` word is its chunk's own first word, big-endian on a
    // PS3 file (`docs/formats/rcsmodel.md`, "The `.vex` is not optional").
    let at = node.payload().start + 0x30;
    let hash = vex
        .get(at..at + 4)
        .map(|bytes| u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .context("card_reflectShape's payload is short")?;
    let mesh = model
        .meshes
        .iter()
        .find(|mesh| mesh.hash == hash)
        .context("card_reflectShape names no chunk")?;
    let alphas: Vec<u8> = mesh
        .submeshes
        .iter()
        .map(|submesh| mesh.inline_colours(&geometry, submesh))
        .collect::<Result<Vec<_>, _>>()
        .context("reading the reflection's colours")?
        .into_iter()
        .flatten()
        .map(|colour| colour[3])
        .collect();
    let (lo, hi) = (alphas.iter().min(), alphas.iter().max());
    match (lo, hi) {
        (Some(&lo), Some(&hi)) => Ok((f32::from(lo) / 255.0, f32::from(hi) / 255.0)),
        _ => anyhow::bail!("card_reflectShape has no vertex"),
    }
}
