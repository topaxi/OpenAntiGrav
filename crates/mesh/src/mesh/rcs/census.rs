//! Which family matcher claims each material of one `.rcsmodel`, for the shader census
//! (`docs/rendering/shader-census.md`).
//!
//! A matcher calls [`claim`] beside the counter it already keeps. Outside [`recorded`] the
//! call does nothing, and no draw path reads what was recorded.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use oag_display::display::Water;
use oag_gpu::formats::SceneStore;
use oag_rcs::rcsmodel::{self, psp2};
use oag_rcs::{program_key, rcsmaterial};

use super::{Report, Textures, pads, setup};

/// What a matcher did with a material it recognised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    /// The family's own reading draws it.
    Drawn,
    /// Recognised, and drawn without the family's reading.
    NotDrawn,
}

/// What one recording gathered.
#[derive(Debug, Default)]
pub struct Claims {
    /// `(family, outcome)` by material index.
    pub families: BTreeMap<usize, BTreeSet<(&'static str, Outcome)>>,
    /// Shown submeshes by material index, which only the 2048/Omega builder reports.
    pub shown: BTreeMap<usize, usize>,
}

thread_local! {
    static CLAIMS: RefCell<Option<Claims>> = const { RefCell::new(None) };
}

fn record(f: impl FnOnce(&mut Claims)) {
    CLAIMS.with_borrow_mut(|claims| {
        if let Some(claims) = claims {
            f(claims);
        }
    });
}

/// Records that `family` claims material `slot`.
pub(super) fn claim(slot: usize, family: &'static str, outcome: Outcome) {
    record(|c| {
        c.families
            .entry(slot)
            .or_default()
            .insert((family, outcome));
    });
}

/// Records one shown submesh of material `slot`.
pub(super) fn shown(slot: usize) {
    record(|c| *c.shown.entry(slot).or_default() += 1);
}

/// Runs `f` collecting everything claimed on this thread.
pub fn recorded<R>(f: impl FnOnce() -> R) -> (R, Claims) {
    let outer = CLAIMS.with_borrow_mut(|claims| claims.replace(Claims::default()));
    let out = f();
    let claims = CLAIMS.with_borrow_mut(|claims| std::mem::replace(claims, outer));
    (out, claims.unwrap_or_default())
}

/// One material of one model, as the census sees it.
#[derive(Debug, Clone)]
pub struct Material {
    /// The material's own name: its `.rcsmaterial` path.
    pub name: String,
    /// Chunks (HD) or shown submeshes (2048, Omega) drawn with it.
    pub chunks: usize,
    /// The program's key ([`program_key`]), or `None` where no program could be named.
    pub key: Option<u64>,
    /// The `SHO` block drawn (HD), as its offset in the `.rcsmaterial`.
    pub block: Option<usize>,
    /// The families that claim it.
    pub claims: BTreeSet<(&'static str, Outcome)>,
}

/// HD: the whole material setup of one `.rcsmodel`, without emitting its geometry.
///
/// Chunks are counted over every surface of every mesh, as `skin::variants` weighs them.
/// The report is the setup's own, so a caller can check the two counts agree.
///
/// # Errors
///
/// Propagates [`rcsmodel::Model::parse`].
pub fn hd(blob: &[u8], textures: Textures<'_>) -> Result<(Vec<Material>, Report)> {
    let model = rcsmodel::Model::parse(blob).context("the .rcsmodel")?;
    let mut report = Report::default();
    let (setup, claims) = recorded(|| {
        setup::material_setup(
            &model,
            blob,
            textures,
            SceneStore::default(),
            Water::Original,
            true,
            &mut report,
        )
    });
    let mut chunks = vec![0usize; model.materials.len()];
    for surface in model.meshes.iter().flat_map(rcsmodel::Mesh::surfaces) {
        if let Some(count) = chunks.get_mut(surface.material as usize) {
            *count += 1;
        }
    }
    let materials = model
        .materials
        .iter()
        .enumerate()
        .filter(|&(slot, _)| chunks[slot] > 0)
        .map(|(slot, material)| {
            let variant: Option<rcsmaterial::Variant> =
                setup.material_variants.get(slot).copied().flatten();
            let block = variant.map(|v| v.fragment.offset);
            let key = block.and_then(|at| {
                program_key::hd_fragment(&textures(&format!("/{}", material.name))?, at)
            });
            let mut claims = claims.families.get(&slot).cloned().unwrap_or_default();
            if pads::is_pad_material(&material.name) {
                claims.insert(("pads", Outcome::Drawn));
            }
            Material {
                name: material.name.clone(),
                chunks: chunks[slot],
                key,
                block,
                claims,
            }
        })
        .collect();
    Ok((materials, report))
}

/// 2048 and Omega: one `.rcsmodel` built as a race builds a circuit.
///
/// Weighted by the submeshes shown: neither hidden nor a replaced crowd entity.
///
/// # Errors
///
/// Propagates the build.
pub fn psp2(
    label: &str,
    blob: &[u8],
    textures: Textures<'_>,
) -> Result<(Vec<Material>, super::psp2::Report)> {
    let ps4 = psp2::is_ps4(blob);
    let (built, claims) = recorded(|| {
        super::psp2::build_circuit(
            label,
            blob,
            None,
            textures,
            Water::Original,
            super::psp2::circuit_pads::HidePads::None,
        )
    });
    let (_, report) = built?;
    let decoded = psp2::parse(blob).map_err(|e| anyhow::anyhow!("{label}: {e}"))?;
    let materials = decoded
        .materials
        .iter()
        .enumerate()
        .filter(|(slot, _)| claims.shown.contains_key(slot))
        .map(|(slot, material)| {
            let key = textures(&material.name).and_then(|file| {
                if ps4 {
                    program_key::gcn_pixels(&file)
                } else {
                    program_key::gxp_fragments(&file)
                }
            });
            let mut families = claims.families.get(&slot).cloned().unwrap_or_default();
            if material.is_placeholder() {
                families.insert(("fc01_placeholder", Outcome::Drawn));
            }
            if material.zone_colour_slot().is_some() {
                families.insert(("zone_colour", Outcome::Drawn));
            }
            Material {
                name: material.name.clone(),
                chunks: claims.shown[&slot],
                key,
                block: None,
                claims: families,
            }
        })
        .collect();
    Ok((materials, report))
}
