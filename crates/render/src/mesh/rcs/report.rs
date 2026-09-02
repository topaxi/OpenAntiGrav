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
    /// Of those, the ones whose hash found a chunk in the `.rcsmodel` **and**
    /// which the node pass actually attempts - see [`Self::world_baked`] for
    /// the third bucket a found hash can fall into.
    pub addressed: usize,
    /// Of the nodes whose hash found a chunk, the ones baked in world space
    /// despite the node naming them - `super::is_world_baked`. Left out of
    /// [`Self::addressed`] deliberately: they are not a stride or a fitting
    /// failure, they were never the node pass's chunk to draw, and folding
    /// them into "addressed no chunk" would call a chunk missing that the
    /// world-space pass draws one line down.
    pub world_baked: usize,
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
    ///
    /// **Excludes both pad classes' chunks.** A `Weapon Pad`/`Speedup Pad`
    /// node's chunk is drawn separately, through its own node-ordered pass -
    /// see [`pads::build_pads`]/[`pads::build_weapon_pads`] - so a caller can
    /// gate or tint it apart from the rest of the circuit, and this count
    /// stays a statement about the *ordinary* world-space geometry.
    pub unreferenced: usize,
    /// Vertices whose normal came out of the file rather than off the
    /// triangles. Reported because the difference is visible and the fallback
    /// is silent: see [`face_normals`].
    pub authored_normals: usize,
    /// Chunks whose material says the surface is not drawn solid, and which are
    /// therefore drawn blended rather than in the opaque pass. See [`surface`].
    ///
    /// **Blended only**, since the mode-2 cutouts split off into
    /// [`Self::cutout`]: they are see-through too, and they are not blended.
    pub see_through: usize,
    /// Chunks drawn as an alpha-test **cutout** rather than blended - Wipeout
    /// HD's `Transparency::Mode2`. See `super::cutout`.
    ///
    /// Counted beside [`Self::see_through`] rather than inside it because the
    /// two are different GPU features and this crate drew all of them as the
    /// other one until the state word was read: a circuit reporting zero here
    /// and a full count there is the old, wrong picture.
    pub cutout: usize,
    /// Mode-2 materials whose alpha test this build will not reproduce, and
    /// which are therefore drawn **opaque**: a comparison other than
    /// `cutout::GL_GREATER`, or a reference disagreeing with the one the model
    /// already settled on.
    ///
    /// Zero on every circuit measured. Non-zero is a fact about a disc this
    /// reading has not seen, and is reported rather than guessed at - drawing
    /// a `GL_LESS` cutout through a `GL_GREATER` shader inverts it.
    pub cutout_unread: usize,
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
    /// Materials whose second texture names a `.gtf` that is not the
    /// circuit's baked lighting atlas, and which decodes.
    ///
    /// **Loaded, and not yet drawn.** `docs/formats/rcsmaterial.md`'s
    /// "traced, not solved" section is why: which texture unit the resolved
    /// shader actually samples it through, and by what operation, is read
    /// only on `glass_texture_customr`'s own block and not confirmed there
    /// either. Counted so the loader report says a second slot exists and is
    /// unread, rather than that nothing does - the same asymmetry
    /// [`Self::lightmapped`] exists for.
    pub second_texture_loaded: usize,
    /// Materials whose second texture names a `.gtf` that is not the
    /// lighting atlas and which did not decode.
    pub second_texture_unread: usize,
    /// Chunks whose vertex declaration names no texture coordinate at all.
    ///
    /// 984 of the disc's chunks. They draw at the origin of their texture,
    /// which is the honest answer: `rcsmodel::Mesh::texcoords` refuses rather
    /// than picking an attribute that is not one, and this counts the refusals
    /// so a flat-coloured surface is a reported absence and not a mystery.
    pub no_texcoord: usize,
    /// Material slots whose shader variant the lit-race key resolved to a row
    /// the file actually ships.
    ///
    /// See `super::variants`. Nothing shades differently for it yet; the count
    /// is here because a reading that reaches 90 % of a circuit and one that
    /// reaches 20 % look identical from the picture.
    pub variants_resolved: usize,
    /// Slots whose material was read but which ship no row for the key.
    ///
    /// Expected rather than alarming for now: the pass half of the key is a
    /// single reading of an ordinary lit race, and a material that only ever
    /// draws in some other pass ships no row for this one.
    pub variants_unshipped: usize,
    /// Slots whose `.rcsmaterial` could not be read from the archive at all.
    pub materials_unread: usize,
    /// Chunks drawn by a slot whose variant resolved.
    ///
    /// **The number that means something.** A slot count treats a material used
    /// once the same as one used eighty times; this weights each by what it
    /// actually draws, and the two diverge sharply - on Talon's Junction the
    /// key resolves for the materials that draw most of the circuit and misses
    /// a long tail of rare ones.
    pub variant_chunks: usize,
    /// Chunks drawn by a slot that ships no row for the key.
    pub variant_chunks_missed: usize,
    /// Chunks a **diagnostic** environment filter took out of this build.
    ///
    /// Always zero in an ordinary run. Non-zero means `OAG_SKIP_MATERIAL` or
    /// `OAG_ONLY_MATERIAL` is set and the frame is an isolation render, not a
    /// picture of the game - see `super::isolate`. Counted, and said out loud
    /// in [`Self::describe`], because a mutilated frame read as the real one
    /// is exactly the mistake this diagnostic is meant to prevent.
    pub isolated: usize,
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
            self.nodes - self.addressed - self.world_baked,
            self.no_stride,
        ) + &match self.world_baked {
            0 => String::new(),
            n => format!(", {n} baked in world space despite a node naming them"),
        } + &match self.strays {
            0 => String::new(),
            n => format!(", {n} submesh(es) dropped as strays"),
        } + &match self.unreferenced {
            0 => String::new(),
            n => format!(", plus {n} chunk(s) no node references, drawn in world space"),
        } + &match self.see_through {
            0 => String::new(),
            n => format!(", {n} chunk(s) drawn see-through"),
        } + &match self.cutout {
            0 => String::new(),
            n => format!(", {n} chunk(s) drawn as an alpha-test cutout"),
        } + &match self.cutout_unread {
            0 => String::new(),
            n => format!(", {n} cutout material(s) whose alpha test is unread, drawn opaque"),
        } + &match self.untextured {
            0 => String::new(),
            n => format!(", {n} material(s) whose .gtf did not paint"),
        } + &match self.lightmapped {
            0 => String::new(),
            n => format!(", {n} material(s) lit through the circuit's lightmap"),
        } + &match self.lightmap_undecoded {
            0 => String::new(),
            n => format!(", {n} lightmap(s) named but not loaded"),
        } + &match self.second_texture_loaded {
            0 => String::new(),
            n => format!(", {n} second texture(s) loaded but not drawn (role unread)"),
        } + &match self.second_texture_unread {
            0 => String::new(),
            n => format!(", {n} second texture(s) named but not loaded"),
        } + &match (
            self.variants_resolved,
            self.variants_unshipped,
            self.materials_unread,
        ) {
            (0, 0, 0) => String::new(),
            (ok, none, unread) => format!(
                ", {ok} of {} drawn material(s) resolved to a shipped shader \
                 variant for the lit race pass, covering {} of {} chunk(s) \
                 ({unread} unread)",
                ok + none,
                self.variant_chunks,
                self.variant_chunks + self.variant_chunks_missed,
            ),
        } + &match self.no_texcoord {
            0 => String::new(),
            n => format!(", {n} chunk(s) declaring no texture coordinate"),
        } + &match self.authored_normals {
            0 => ", lit off face normals computed from the triangles".to_string(),
            n => format!(", {n} authored vertex normal(s)"),
        } + &match self.isolated {
            0 => String::new(),
            n => format!(
                ". DIAGNOSTIC: an OAG_SKIP_MATERIAL/OAG_ONLY_MATERIAL filter took \
                 {n} chunk(s) out of this build, so this frame is an isolation \
                 render and not the picture"
            ),
        }
    }
}
