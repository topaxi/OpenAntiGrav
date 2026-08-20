//! What a `.rcsmodel` build found, for the loader report.
//!
//! Split out of `mesh/rcs.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// What a build found, for the loader report.
///
/// **Every field here is a way the picture is incomplete**, which is why they
/// are counted rather than logged and forgotten: a circuit that silently drew
/// two thirds of itself would look like a working feature.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Report {
    /// Mesh nodes of the wanted class in the `.vex`.
    pub nodes: usize,
    /// Of those, the ones whose hash found a chunk in the `.rcsmodel`.
    pub addressed: usize,
    /// Of those, the ones whose vertex stride the authored box settled.
    pub drawn: usize,
    /// Of those, the ones no rule could settle a vertex stride for.
    ///
    /// Counted rather than derived from `addressed - drawn`, which that
    /// difference used to be: it silently absorbed every other reason a mesh
    /// contributes nothing - a chunk whose submeshes are all strays among them -
    /// and so read as a stride failure whatever went wrong.
    pub no_stride: usize,
    /// Triangles emitted.
    pub triangles: usize,
    /// Submeshes dropped because they do not share their mesh's vertex stride.
    pub strays: usize,
    /// Chunks no `.vex` node references, drawn in world space. On a circuit
    /// this is the circuit; see [`build_scene`].
    pub unreferenced: usize,
    /// Vertices whose normal came out of the file rather than off the
    /// triangles. Reported because the difference is visible and the fallback
    /// is silent: see [`face_normals`].
    pub authored_normals: usize,
    /// Chunks whose material says the surface is not drawn solid, and which are
    /// therefore drawn blended rather than in the opaque pass. See [`surface`].
    pub see_through: usize,
    /// Materials whose `.gtf` this build could not paint with - no path in the
    /// record, no such entry in the archive, or a container
    /// `oag_formats::gtf::Texture::to_rgba` refuses.
    ///
    /// **The one to watch**, because its failure mode is the one this module
    /// has spent the most effort removing: a draw call with no texture binds
    /// `mesh_render::build`'s white 1x1 and paints a white sheet, which is
    /// exactly what a *working* surface looks like at a glance. See
    /// [`decode_texture`].
    pub untextured: usize,
    /// Materials that carry the circuit's baked lighting atlas in their second
    /// texture slot, and are therefore drawn multiplied by it.
    ///
    /// Counted because it is the one use of that slot this project identifies
    /// and the other three are left unsampled: a circuit reporting zero here is
    /// drawing its surfaces unlit by the artists' bake, which is a visible
    /// absence and should be a stated one. See
    /// `oag_formats::rcsmodel::Material::lightmap`.
    pub lightmapped: usize,
    /// Materials naming a lightmap whose `.gtf` did not load or decode.
    pub lightmap_undecoded: usize,
    /// Chunks whose vertex declaration names no texture coordinate at all.
    ///
    /// 984 of the disc's chunks. They draw at the origin of their texture,
    /// which is the honest answer: `rcsmodel::Mesh::texcoords` refuses rather
    /// than picking an attribute that is not one, and this counts the refusals
    /// so a flat-coloured surface is a reported absence and not a mystery.
    pub no_texcoord: usize,
}

impl Report {
    /// One line for a load report.
    #[must_use]
    pub fn describe(&self) -> String {
        format!(
            "{} of {} mesh node(s) drawn from the .rcsmodel ({} triangle(s)); \
             {} addressed no chunk, {} had no recoverable vertex stride",
            self.drawn,
            self.nodes,
            self.triangles,
            self.nodes - self.addressed,
            self.no_stride,
        ) + &match self.strays {
            0 => String::new(),
            n => format!(", {n} submesh(es) dropped as strays"),
        } + &match self.unreferenced {
            0 => String::new(),
            n => format!(", plus {n} chunk(s) no node references, drawn in world space"),
        } + &match self.see_through {
            0 => String::new(),
            n => format!(", {n} chunk(s) drawn see-through"),
        } + &match self.untextured {
            0 => String::new(),
            n => format!(", {n} material(s) whose .gtf did not paint"),
        } + &match self.lightmapped {
            0 => String::new(),
            n => format!(", {n} material(s) lit through the circuit's lightmap"),
        } + &match self.lightmap_undecoded {
            0 => String::new(),
            n => format!(", {n} lightmap(s) named but not loaded"),
        } + &match self.no_texcoord {
            0 => String::new(),
            n => format!(", {n} chunk(s) declaring no texture coordinate"),
        } + &match self.authored_normals {
            0 => ", lit off face normals computed from the triangles".to_string(),
            n => format!(", {n} authored vertex normal(s)"),
        }
    }
}
