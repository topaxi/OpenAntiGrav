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
    /// Chunks on a pad material that no pad node of either class names, which
    /// [`build_scene`]'s world-space pass draws: where the speed pads of the
    /// four original circuits are, because their `Speedup Pad` nodes name a
    /// hash no chunk carries. Only the pad passes count it - see
    /// [`pads::world_pass_pad_chunks`].
    pub routed_chunks: usize,
    /// Triangles in [`Self::routed_chunks`].
    pub routed_triangles: usize,
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
    /// Chunks whose own render-block flags call them track surface
    /// (`oag_rcs::rcsmodel::Mesh::is_track`), and which an HD Zone race
    /// therefore draws through the `Track` parameter set and
    /// `zoneModeTrack<n>.gtf`; every other chunk takes the `Scene` set. 124
    /// of Talon's Junction's 983. Counted per chunk, whichever pass drew it.
    pub track_surface: usize,
    /// Materials whose `.gtf` this build could not paint with - no path in the
    /// record, no such entry in the archive, or a container
    /// `oag_texture::gtf::Texture::to_rgba` refuses.
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
    /// `oag_rcs::rcsmodel::Material::lightmap`.
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
    /// Materials whose resolved fragment program has no `rcsmaterial::fragment
    /// ::Program::specular_exponent` chain, and therefore draw with the shared
    /// `mesh::DEFAULT_SPECULAR_EXPONENT` stand-in rather than a value read off
    /// the file.
    ///
    /// **Not necessarily wrong** - most materials genuinely have no specular
    /// term to read, and a default there is the correct answer, not a gap.
    /// Counted anyway, because the two look identical from this field alone:
    /// `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "Ships have no
    /// Lambert diffuse either" found `Program::dp3_feeding`'s writer search
    /// lane-unsound in both directions (crediting a `DP3` that wrote the
    /// wrong lane, and missing a real one behind an unrelated write to a
    /// different lane) - fixed 2026-09-04, moving the disc-wide resolved
    /// count from 6,141 to 10,087 fragment blocks - and still, separately,
    /// found `op3B` a real, minority cause of the population that remains
    /// unresolved after that fix (`23.0 %` of it, re-measured post-fix). A
    /// material landing here may be
    /// unlit, or may be exactly that residual false negative - a silent
    /// fallback would make both look the same, which is the failure mode
    /// every other counter on this struct exists to avoid. **422 of Talon's
    /// Junction's 442 materials (95 %)** land here as of 2026-09-04, after
    /// the `dp3_feeding` fix (down from 426 before it) - still too large a
    /// population to be all false negative, but too large to wave off as
    /// "mostly unlit" without checking either; unread, deliberately, rather
    /// than assumed.
    pub specular_exponent_unresolved: usize,
    /// Materials whose fragment program structurally accumulates its second
    /// texture (see [`crate::mesh::slots::ADD_SECOND`]) but whose *actually
    /// loaded* second texture - `Pick::aux`'s own choice, the same one
    /// [`Self::second_texture_loaded`] counts - is a sampler this project has
    /// disc-measured to be a normal or specular map and never a picture.
    ///
    /// **The bug this counts.** A ship hull's raw ordinal fallback is
    /// routinely its own normal map; adding that at full weight as a tinted
    /// glow is what painted a blue-purple cast over `feisar_c1`'s hull. See
    /// `mesh::rcs::emissive`'s own doc comment and
    /// `docs/rendering/hd-ship-materials.md`, "a ship's own second texture is
    /// added to the hull as a glow". Refused rather than drawn, so this
    /// counts an absence the picture would otherwise show as a colour cast.
    pub emissive_surface_map_excluded: usize,
    /// Materials that structurally accumulate but whose loaded second
    /// texture's own sampler hash this project has no disc-measured role
    /// for - not a known glow, not a known surface map.
    ///
    /// **Kept on the pre-existing path, deliberately**: per `CLAUDE.md`'s
    /// rule against inventing a role from a plausible-looking name alone, a
    /// hash with no disc-wide binding census stays exactly as it drew before
    /// this reading existed, so a real gap in this counter is invisible in
    /// the picture rather than guessed at. See
    /// [`Self::emissive_surface_map_excluded`] for the population this *is*
    /// acted on.
    pub emissive_role_unresolved: usize,
    /// HD pad materials whose `_ne` mask was bound and whose program's
    /// `_ne`-alpha term was wired - see `super::pad_ne`.
    pub pad_ne_bound: usize,
    /// HD pad materials that name the `_ne` sampler and were left without it:
    /// the file did not decode, or the material authors no value for the
    /// parameter the program multiplies the mask's alpha by. Drawn without
    /// the term rather than with an invented colour.
    pub pad_ne_unread: usize,
    /// Magstrip materials whose emissive picture and wave were bound - see
    /// `super::mag_wave`.
    pub mag_wave_bound: usize,
    /// Magstrip materials that declare the wave and could not be bound: a
    /// texture did not decode or an authored value is missing. Drawn without.
    pub mag_wave_unread: usize,
    /// Materials whose vertex program scrolls their one texture off `time`
    /// and an authored rate - see `super::vertex_scroll`.
    pub vertex_scrolls: usize,
    /// HD light-cone materials whose noise and ramp were bound - see
    /// `super::light_cone`.
    pub light_cone_bound: usize,
    /// Water materials whose picture is a normal map (`super::water`), drawn
    /// as lit vertex colour: the program's reflection term is not drawn.
    pub water_lit_colour: usize,
    /// Water materials whose only colour is the reflection and a sun glint:
    /// drawn black plus the generic specular, the reflection not drawn.
    pub water_glint_only: usize,
    /// Sebenco ice materials drawn as their authored water and ice lerp, and
    /// ones whose textures or colours could not be read (drawn as before).
    pub ice_bound: usize,
    pub ice_unread: usize,
    /// Screen-grab refraction materials bound, and ones whose weight or table
    /// entry could not be read (drawn as an ordinary lit surface).
    pub refraction_bound: usize,
    /// See [`Self::refraction_bound`].
    pub refraction_unread: usize,
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
        let lead = if self.drawn == 0 && self.routed_chunks > 0 {
            format!(
                "{} node(s) address no chunk, so the pad is the {} chunk(s) on a pad \
                 material that no pad node names, drawn in the circuit's world-space \
                 pass ({} triangle(s))",
                self.nodes - self.addressed - self.world_baked,
                self.routed_chunks,
                self.routed_triangles,
            )
        } else {
            format!(
                "{} of {} mesh node(s) drawn from the .rcsmodel ({} triangle(s)); \
                 {} addressed no chunk, {} had no recoverable vertex stride",
                self.drawn,
                self.nodes,
                self.triangles,
                self.nodes - self.addressed - self.world_baked,
                self.no_stride,
            )
        };
        lead + &match self.world_baked {
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
        } + &match self.track_surface {
            0 => String::new(),
            n => format!(", {n} chunk(s) flagged track surface for Zone's Track set"),
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
        } + &match self.specular_exponent_unresolved {
            0 => String::new(),
            n => format!(", {n} material(s) with no specular_exponent chain read (default used)"),
        } + &match self.emissive_surface_map_excluded {
            0 => String::new(),
            n => format!(
                ", {n} material(s) refused an additive glow whose second texture is a \
                 known normal/specular map"
            ),
        } + &match self.emissive_role_unresolved {
            0 => String::new(),
            n => format!(
                ", {n} accumulating material(s) with no disc-measured role for their \
                 second texture (kept as before)"
            ),
        } + &match self.pad_ne_bound {
            0 => String::new(),
            n => {
                format!(", {n} pad material(s) with their _ne mask bound as normal and light bars")
            }
        } + &match self.pad_ne_unread {
            0 => String::new(),
            n => format!(
                ", {n} pad material(s) whose _ne mask or its colour could not be read (drawn without)"
            ),
        } + &match self.mag_wave_bound {
            0 => String::new(),
            n => format!(", {n} magstrip material(s) with their scrolling wave bound"),
        } + &match self.mag_wave_unread {
            0 => String::new(),
            n => format!(", {n} magstrip material(s) whose wave could not be read (drawn without)"),
        } + &match self.vertex_scrolls {
            0 => String::new(),
            n => format!(", {n} material(s) scrolling their texture off an authored rate"),
        } + &match self.light_cone_bound {
            0 => String::new(),
            n => format!(", {n} light-cone material(s) with their facing ramp bound"),
        } + &match self.water_lit_colour + self.water_glint_only {
            0 => String::new(),
            n => format!(
                ", {n} water material(s) whose picture is a normal map, not painted \
                 ({} as lit vertex colour, {} as black plus a glint); their reflection \
                 (the engine's paraboloid probe) is NOT drawn",
                self.water_lit_colour, self.water_glint_only
            ),
        } + &match self.ice_bound {
            0 => String::new(),
            n => format!(", {n} ice material(s) drawn as their authored water and ice colours"),
        } + &match self.ice_unread {
            0 => String::new(),
            n => format!(", {n} ice material(s) whose mask or colours could not be read"),
        } + &match self.refraction_bound {
            0 => String::new(),
            n => format!(", {n} screen-grab refraction material(s) drawn over what is behind them"),
        } + &match self.refraction_unread {
            0 => String::new(),
            n => format!(
                ", {n} refraction material(s) whose weight could not be read (drawn lit, opaque)"
            ),
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
