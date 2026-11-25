//! What one `psp2::build` found, and its load-report line.

use super::*;

/// What one build found.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Report {
    /// Submeshes drawn.
    pub submeshes: usize,
    /// Triangles emitted.
    pub triangles: usize,
    /// Relocation entries naming a GPU pointer this reading does not account
    /// for - see [`psp2::Model::unpaired_pointers`].
    pub unpaired: usize,
    /// Vertices whose normal came from the file rather than off the
    /// triangles - see [`super::face_normals`]. Reported for the same reason
    /// HD's own build counts it: the fallback is silent and the difference is
    /// visible.
    pub authored_normals: usize,
    /// Vertices whose decoded diffuse texcoord was non-finite and got
    /// substituted with the origin - see
    /// [`psp2::SubMesh::non_finite_texcoords`]. `0` on almost every model;
    /// surfaced because a hidden substitution here is a hidden difference
    /// from what the file authored, the same reasoning
    /// [`Self::authored_normals`] already applies to its own fallback.
    pub non_finite_texcoords: usize,
    /// Material slots whose vertex programs write `1 - v` into the diffuse coordinate
    /// ([`super::uv_flip`]).
    pub flipped_v_materials: usize,
    /// Vertices whose submesh's declaration named a `tangent` attribute for
    /// this stride, so [`psp2::SubMesh::tangents`] carries one - **not** a
    /// count of non-zero/well-formed tangents the way [`Self::authored_normals`]
    /// counts real values against a zero sentinel. `docs/formats/2048-rcsmodel.md`'s
    /// own reading measures ~42% of decoded tangents as short or near-zero
    /// (degenerate at UV poles/seams), so this field answers "was the
    /// attribute present", not "was the value good" - naming it
    /// `authored_tangents` would have implied the latter.
    ///
    /// **Drawn with on a PS4 model only** ([`Self::normal_maps`]). Nothing in 2048's
    /// mesh path samples a tangent-space normal map (its programs are unread GXP), so
    /// there is no visual difference between a Vita model with this at `0` and one with
    /// it equal to [`Self::submeshes`]' own vertex total - see
    /// `docs/formats/2048-rcsmodel.md#tangent-is-cracked-too-at-a-lower-confidence`
    /// for why this is reported anyway (confidence 76, not yet wired into a
    /// consumer) rather than silently dropped on the way from `oag_rcs`.
    pub decoded_tangents: usize,
    /// What the PS4 normal-map binding found - see `normal_map`. All zero on a Vita model.
    pub normal_maps: NormalMapCensus,
    /// 2048's own sea (`psp2::water`). All zero on a PS4 model.
    pub sea: super::water::Census,
    /// Materials that multiply `VertexColour1` into the light (`psp2::hull_colour`).
    pub hull_colour: HullColourCensus,
    /// How many materials this model's own table names, whether or not this
    /// build could bind one to a draw - see [`psp2::material`]. `0` for a
    /// model with no readable table, which is not the same state as a model
    /// this build simply chose not to texture.
    pub materials: usize,
    /// The first texture this build painted anything with, for a one-line
    /// report. `None` when nothing painted at all.
    pub diffuse_texture: Option<String>,
    /// Distinct `.gxt` files decoded and uploaded.
    pub textures: usize,
    /// Draws that got a texture bound.
    pub textured_draws: usize,
    /// Distinct lightmap atlases decoded and bound as materials' second
    /// texture - see [`psp2::material::Material::lightmap`].
    pub lightmaps: usize,
    /// Lightmap atlases a material named that did not resolve in the archive or
    /// would not decode. Such a draw binds the black placeholder, which is the
    /// same picture as no lightmap at all.
    pub lightmap_misses: usize,
    /// Submeshes lit by their `NOVAColor` vertex stream (a Vita circuit's vertex-lit surfaces).
    pub nova_vertex_submeshes: usize,
    /// Submeshes whose program lights the albedo's alpha (`lit_alpha`, Anulpha Pass).
    pub lit_alpha_submeshes: usize,
    /// Chevron materials drawing their emissive mask and wave (`chevron`).
    pub chevron_materials: usize,
    /// Chevron materials whose mask or wave would not decode, or found no table row.
    pub chevron_unread: usize,
    /// Textured draws routed to the blended list by their material's state word
    /// (alpha-over, chosen) - see [`transparency`].
    pub blended_draws: usize,
    /// Of [`Self::blended_draws`], those drawn with the factor pair Wipeout HD
    /// authors for a material of the same name, not the alpha-over default - see
    /// [`psp2::lineage_blend`].
    pub inherited_blend_draws: usize,
    /// Textured draws routed to the alpha-tested list (reference `0.5`, chosen).
    pub cutout_draws: usize,
    /// Draws whose material named a texture that did not resolve in the
    /// archive or would not decode - the honest count of what is still
    /// missing, kept apart from a submesh that simply has no material.
    pub unresolved_draws: usize,
    /// [`Self::unresolved_draws`] by the exit that left each one.
    pub unbound: UnboundDraws,
    /// Materials that draw an additive glow layer off their own
    /// `Emissive_UV_Offset`/`Scale` - see `glow`'s module doc.
    pub glow_layers: usize,
    /// Materials that scroll their one texture off `speed_multipliaer`.
    pub scrolling_materials: usize,
    /// Materials that scroll off HD's vertex law, **inherited by shader name and
    /// authored rate hashes, no 2048 or Omega instruction read** - see
    /// `vertex_scroll::inherited_rate`.
    pub inherited_scrolls: usize,
    /// Of those, the ones admitted without a readable shader program to
    /// confirm the vertex stage declares `time` (every Omega one: GCN).
    pub inherited_unread: usize,
    /// `nr_twinblend` materials drawn by HD's law - **inherited from HD**, no 2048 or Omega
    /// instruction read - see `twin_blend`.
    pub twin_blend_materials: usize,
    /// `nr_twinblend` materials left as the generic reading drew them: a picture would not
    /// decode, or a submesh carries no `barsUV_blendUV`.
    pub twin_blend_unread: usize,
    /// `dc_hologramwithstatic` materials drawn by HD's mode 7 without its `/2` - see
    /// `static_hologram`.
    pub static_hologram_materials: usize,
    /// Of that name, the ones left as the generic reading drew them (a picture or a rate
    /// missing).
    pub static_hologram_unread: usize,
    /// Submeshes on a placeholder shader (`fc01_dummy`), drawn as the fog colour - see
    /// [`psp2::material::Material::is_placeholder`].
    pub placeholder_submeshes: usize,
    /// Submeshes whose albedo is their material's `Zone_ColourN` uniform - see
    /// [`psp2::material::Material::zone_colour`]. Counted among
    /// [`Self::unresolved_draws`] too, since they have no texture.
    pub zone_colour_submeshes: usize,
    /// Submeshes of a shape whose name says it is track ([`super::is_track_shape`]), which a
    /// Zone race draws through the `Track` colour group.
    pub track_shape_submeshes: usize,
    /// Submeshes of Omega's `emissive` family, drawn unlit in their `EmissiveColour` (`emissive_colour`).
    pub emissive_colour_submeshes: usize,
    /// Submeshes bound to a node of the model's own table and placed by
    /// it - through the skeleton where one was given, through the model's
    /// bind matrix otherwise. See [`placement`].
    pub node_bound: usize,
    /// Of those, the submeshes the shader's node table moves per frame.
    pub moving: usize,
    /// Nodes the table moves - the entries of [`Model::anim_nodes`].
    pub anim_nodes: usize,
    /// Submeshes under a node authored invisible that nothing ever shows,
    /// not emitted.
    pub hidden: usize,
    /// Submeshes on a node the file gives no transform - no written bind
    /// matrix, no skeleton entry - not emitted, since nothing authored says
    /// where they go. Counted apart from [`Self::hidden`], which is the
    /// file's own choice. See [`placement::Placement::unplaced`].
    pub unplaced: usize,
    /// Submeshes of a track's crowd-entity shapes ([`oag_rcs::rcsmodel::psp2::nodes::CROWD_ENTITY_FLAGS`]),
    /// not emitted: Omega replaces each with a `Data/Crowd/Rigs` instance, which is not loaded, so
    /// the stand draws no figure there (`docs/formats/omega-status.md`, "The crowd-entity shapes").
    /// Counted apart from [`Self::hidden`].
    pub crowd_entities: usize,
    /// Crowd-entity shapes given a `Data/Crowd/Rigs` figure in the bind pose (`psp2::crowd_rigs`).
    pub crowd_rigs: usize,
    /// Vertices of those rigs, which [`crate::mesh::Model::vertices`] holds beside the file's own.
    pub crowd_rig_vertices: usize,
    /// Crowd-entity shapes whose rig file did not read; they draw nothing.
    pub crowd_rigs_missing: usize,
    /// Model nodes the skeleton did not name, placed by the model's own bind
    /// matrix - see [`placement::Plan::unmatched`].
    pub unmatched_nodes: usize,
    /// Nodes frozen at time zero past the table's ceiling - see
    /// [`placement::Plan::frozen`].
    pub frozen_nodes: usize,
    /// How the `.gnf` textures this build decoded were held - see
    /// [`ModelTexture::from_gnf_form`]. Counted per decode, so a texture two
    /// materials name is counted once for each.
    pub gnf: crate::mesh::GnfCounts,
    /// Materials whose picture the build refused because the material's own
    /// fragment programs declare it only in their lit permutations
    /// (`oag_rcs::gxp::sampler_use`): a normal, specular or reflection lookup.
    pub lighting_only_materials: usize,
    /// Of those, the materials that drew the constant colour their ambient-only
    /// program names, as a one-texel picture.
    pub constant_colour_materials: usize,
    /// Of those, the ones whose answer is 2048's, carried over because the
    /// material's record matches it exactly and its own programs (PS4) are
    /// unread - `oag_rcs::rcsmodel::psp2::lighting_only`.
    pub inherited_lighting_only: usize,
    /// Materials of `m_diffuse_normal_constantspecular` whose picture is
    /// `Constant1` times their map's alpha, the map's rgb a normal
    /// (`psp2::alpha_product`).
    pub alpha_product_materials: usize,
    /// Omega materials with no colour texture whose GCN program multiplies the light by a
    /// `diffuse` constant, drawn as that constant (`constant_diffuse`).
    pub constant_diffuse_materials: usize,
    /// Materials of `diffuse_normal_specular_emmissive_alpha` that add `Texture2Alpha^2 * Constant1`
    /// after the light (`psp2::emissive_add`), Vita only.
    pub emissive_add_materials: usize,
    /// Of that family, materials left to the generic reading: no `Texture2Alpha`, no `Constant1`,
    /// or a picture that would not decode.
    pub emissive_add_unread: usize,
    /// Weapon pad classes left on their material's own entry because the glow table is full:
    /// their bars keep the authored colour.
    pub pad_bars_unheld: usize,
    /// Omega's `tarmac_markings_dirt_markingsnormal[_emis]` roads that lay their dirt and
    /// markings pictures over the diffuse (`psp2::tarmac`).
    pub tarmac_materials: usize,
    /// Of that family, materials left to the generic reading: a submesh without both pairs, or a
    /// picture that would not decode.
    pub tarmac_unread: usize,
    /// Materials of `fc07_blend_vertred_lambert_2diff` whose albedo mixes `DiffuseMap01` in by
    /// the vertex colour's red (`psp2::vertex_blend`), Vita only.
    pub vertex_blend_materials: usize,
    /// Of that family, materials left to the generic reading: a submesh without `VertexColour1`,
    /// a second picture that would not decode, another reading on the slot, or a full glow table.
    pub vertex_blend_unread: usize,
    /// Omega's `crowd_sprite_diffuse_normal_alpha[_shadow]` materials lit by their `Shadowed` and
    /// `Ambient` (`psp2::crowd`).
    pub crowd_materials: usize,
    /// Of those, materials left to the generic reading: the glow table was full.
    pub crowd_unread: usize,
    /// Of the admitted, ones whose programs weight the normal map's `z` lane by the eye vector.
    pub crowd_eye_frames: usize,
    /// Omega's `RT_snow1`-shaped materials drawn by their own pixel program (`psp2::snow`).
    pub snow_materials: usize,
    /// Of that shape, materials left to the generic reading: a submesh without `NOVAColor`,
    /// `VertexColour2` or a tangent, a constant unauthored, a picture that would not decode or a
    /// full glow table.
    pub snow_unread: usize,
    /// Omega's `DC_Rocks`-shaped materials drawn by their own pixel program (`psp2::snow`).
    pub rock_materials: usize,
    /// Of that shape, materials left to the generic reading: a submesh without a lightmap pair
    /// or a tangent, a constant unauthored, a picture that would not decode or a full glow table.
    pub rock_unread: usize,
    /// Materials of the `alpha_emissive` families that add the picture's alpha times `glowtinrt`
    /// after the light (`psp2::alpha_emissive`).
    pub alpha_emissive_materials: usize,
    /// Of those, materials left to the generic reading: another reading holds the slot, or the
    /// drawn picture is not the `alpha_emissive` sampler's.
    pub alpha_emissive_unread: usize,
    /// Of those and the glow-less twin, the materials whose specular term is weighted by their
    /// `SpecularTexture` (bound third) and whose picture is `alpha_emissive`.
    pub alpha_emissive_specular_maps: usize,
    /// `2048_shipshield_old` shells that draw their Lightning layer (`psp2::shield_lightning`).
    pub shield_lightning_materials: usize,
    /// Of that family, shells left to the diffuse alone: no picture, weight or program rates.
    pub shield_lightning_unread: usize,
    /// Tunnel-glass materials that read the refraction target (`psp2::refraction`).
    pub refraction_bound: usize,
    /// Of that family, materials without the grab's weight or a table slot.
    pub refraction_unread: usize,
    /// Meshes whose `.vex` node authors UV keys (`psp2::uv_keys`), a billboard build only.
    pub uv_keyed_meshes: usize,
    /// Omega's panel-map roads and the sky reflection map (`psp2::panel_map`).
    pub panel: super::panel_map::Census,
}

impl Report {
    /// One line for a load report.
    #[must_use]
    pub fn describe(&self) -> String {
        let texture = match (&self.diffuse_texture, self.materials) {
            (Some(path), n) => format!(
                "{}/{} draw(s) textured from {} of {n} material(s) (e.g. {path}), \
                 {} unresolved{}",
                self.textured_draws,
                self.submeshes,
                self.textures,
                self.unresolved_draws,
                self.unbound.describe()
            ),
            (None, 0) => "untextured, no material table".to_string(),
            (None, n) => format!("untextured, {n} material(s), none of whose textures resolved"),
        };
        let placeholders = if self.placeholder_submeshes == 0 {
            String::new()
        } else {
            format!(
                "; {} submesh(es) on fc01_dummy drawn as the circuit's fog colour (its three-instruction program outputs `fogColour` and nothing else)",
                self.placeholder_submeshes
            )
        };
        let scrolls = if self.glow_layers + self.scrolling_materials + self.inherited_scrolls == 0 {
            String::new()
        } else {
            format!(
                "; {} glow layer(s) and {} plain scroll(s) off the materials' own uniforms \
                 (rates chosen, not measured), {} scroll(s) inherited from HD's vertex law by \
                 shader name and authored rate (no instruction read here)",
                self.glow_layers, self.scrolling_materials, self.inherited_scrolls
            )
        };
        let twins = if self.twin_blend_materials + self.twin_blend_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} nr_twinblend material(s) drawn by HD's law (two pictures, a wipe scrolling at HD's rate, bars; inherited from HD, not measured here), {} left as the generic reading",
                self.twin_blend_materials, self.twin_blend_unread
            )
        };
        let holograms = if self.static_hologram_materials + self.static_hologram_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} dc_hologramwithstatic material(s) drawn by HD's law without its /2 (units and rates from the material record; the GCN arithmetic read, the units' gamma inherited from HD), {} left as the generic reading",
                self.static_hologram_materials, self.static_hologram_unread
            )
        };
        let zone_colours = if self.zone_colour_submeshes == 0 {
            String::new()
        } else {
            format!(
                "; {} submesh(es) take a Zone_ColourN uniform as their colour (Colour * (ambient + sun * sat(N.L) + Emissive), the stage's own Colour k where the Zone table authors it)",
                self.zone_colour_submeshes
            )
        };
        let track_shapes = if self.track_shape_submeshes == 0 {
            String::new()
        } else {
            format!(
                "; {} submesh(es) on track-named shapes take the Track Zone group (chosen, not measured: the shape name stands in for HD's per-chunk track bit)",
                self.track_shape_submeshes
            )
        };
        let poisoned = if self.non_finite_texcoords == 0 {
            String::new()
        } else {
            format!(
                ", {} non-finite texcoord(s) zeroed",
                self.non_finite_texcoords
            )
        };
        let flipped = if self.flipped_v_materials == 0 {
            String::new()
        } else {
            format!(
                "; {} material(s) write 1 - v into the diffuse coordinate (read off their vertex programs)",
                self.flipped_v_materials
            )
        };
        let tangents = (if self.decoded_tangents == 0 {
            String::new()
        } else if self.normal_maps.submeshes == 0 {
            format!(
                "; {} tangent(s) decoded, unused (no normal-map consumer yet)",
                self.decoded_tangents
            )
        } else {
            self.normal_maps.describe()
        }) + &self.sea.describe();
        let see_through = if self.blended_draws + self.cutout_draws == 0 {
            String::new()
        } else {
            format!(
                "; {} draw(s) blended off the state word ({} with HD's authored factors, for the same material name or by equivalence (the start-grid laser), the rest alpha-over, chosen: neither title authors factors) and {} alpha-tested at 0.5 (chosen)",
                self.blended_draws, self.inherited_blend_draws, self.cutout_draws
            )
        };
        let lightmaps = if self.lightmaps == 0 && self.lightmap_misses == 0 {
            String::new()
        } else {
            format!(
                "; {} lightmap atlas(es) bound, {} unresolved",
                self.lightmaps, self.lightmap_misses
            )
        };
        let lightmaps = if self.nova_vertex_submeshes == 0 {
            lightmaps
        } else {
            format!(
                "{lightmaps}; {} submesh(es) lit by their NOVAColor vertex stream",
                self.nova_vertex_submeshes
            )
        };
        let lightmaps = if self.lit_alpha_submeshes == 0 {
            lightmaps
        } else {
            format!(
                "{lightmaps}; {} submesh(es) light the albedo's alpha, {} chevron material(s) add their mask and wave ({} unread, the wave offset held)",
                self.lit_alpha_submeshes, self.chevron_materials, self.chevron_unread
            )
        };
        let nodes = if self.node_bound == 0 {
            String::new()
        } else {
            let frozen = if self.frozen_nodes == 0 {
                String::new()
            } else {
                format!(", {} frozen past the table", self.frozen_nodes)
            };
            let unmatched = if self.unmatched_nodes == 0 {
                String::new()
            } else {
                format!(", {} node(s) not in the skeleton", self.unmatched_nodes)
            };
            let hidden = if self.hidden == 0 {
                String::new()
            } else {
                format!(", {} hidden", self.hidden)
            };
            let unplaced = if self.unplaced == 0 {
                String::new()
            } else {
                format!(
                    ", {} on a node with no matrix and no skeleton entry, not drawn",
                    self.unplaced
                )
            };
            let crowd = if self.crowd_entities == 0 {
                String::new()
            } else {
                format!(
                    ", {} crowd-entity placeholder(s) not drawn, {} replaced by a rig in its bind pose, the group's clip not played ({} missing)",
                    self.crowd_entities, self.crowd_rigs, self.crowd_rigs_missing
                )
            };
            format!(
                "; {} node-bound submesh(es), {} moving on {} animated node(s){hidden}{crowd}{unplaced}{frozen}{unmatched}",
                self.node_bound, self.moving, self.anim_nodes
            )
        };
        let gnf = self.gnf.describe();
        let emissive_add = if self.emissive_add_materials + self.emissive_add_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} emissive-alpha material(s) add Texture2Alpha^2 * Constant1 after the light ({} left to the generic reading)",
                self.emissive_add_materials, self.emissive_add_unread
            )
        };
        let pad_bars = if self.pad_bars_unheld == 0 {
            String::new()
        } else {
            format!(
                "; {} weapon pad class(es) with no room in the glow table (authored bar colour)",
                self.pad_bars_unheld
            )
        };
        let tarmac = if self.tarmac_materials + self.tarmac_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} tarmac road material(s) lay dirt and markings over the diffuse ({} left to the generic reading)",
                self.tarmac_materials, self.tarmac_unread
            )
        };
        let vertex_blend = if self.vertex_blend_materials + self.vertex_blend_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} vertex-blend material(s) mix DiffuseMap01 in by the vertex red ({} left to the generic reading)",
                self.vertex_blend_materials, self.vertex_blend_unread
            )
        };
        let crowd = if self.crowd_materials + self.crowd_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} crowd material(s) lit by Shadowed * (Ambient + light), {} with the eye frame ({} left to the generic reading)",
                self.crowd_materials, self.crowd_eye_frames, self.crowd_unread
            )
        };
        let snow = if self.snow_materials + self.snow_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} snow material(s) lit by their own pixel program ({} left to the generic reading)",
                self.snow_materials, self.snow_unread
            )
        };
        let rock = if self.rock_materials + self.rock_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} rock material(s) lit by their own pixel program ({} left to the generic reading)",
                self.rock_materials, self.rock_unread
            )
        };
        let alpha_emissive = if self.alpha_emissive_materials + self.alpha_emissive_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} alpha-emissive material(s) add the picture's alpha * glowtinrt after the light ({} left to the generic reading, {} weight their specular by a map)",
                self.alpha_emissive_materials,
                self.alpha_emissive_unread,
                self.alpha_emissive_specular_maps
            )
        };
        let lightning = if self.shield_lightning_materials + self.shield_lightning_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} shield shell(s) draw their Lightning layer ({} left to the diffuse alone)",
                self.shield_lightning_materials, self.shield_lightning_unread
            )
        };
        let refraction = if self.refraction_bound + self.refraction_unread == 0 {
            String::new()
        } else {
            format!(
                "; {} tunnel-glass material(s) read the refraction target ({} left to the generic reading)",
                self.refraction_bound, self.refraction_unread
            )
        };
        let lighting_only = if self.lighting_only_materials == 0 {
            String::new()
        } else {
            format!(
                "; {} material(s) bind no picture from a sampler their own programs read only for lighting ({} drew the constant colour the unlit program names, {} of them 2048's answer carried over where the record matches)",
                self.lighting_only_materials,
                self.constant_colour_materials,
                self.inherited_lighting_only
            )
        };
        let alpha_product = if self.alpha_product_materials == 0 {
            String::new()
        } else {
            format!(
                "; {} material(s) draw Constant1 times their normal map's alpha",
                self.alpha_product_materials
            )
        };
        let (e, d) = (
            self.emissive_colour_submeshes,
            self.constant_diffuse_materials,
        );
        let flat_constants = if e + d == 0 {
            String::new()
        } else {
            format!(
                "; {e} submesh(es) draw an EmissiveColour unlit, {d} material(s) a constant diffuse"
            )
        };
        let uv_keys = if self.uv_keyed_meshes == 0 {
            String::new()
        } else {
            format!(
                "; {} mesh(es) ride the UV keys their .vex node authors",
                self.uv_keyed_meshes
            )
        };
        let panel = self.panel.describe();
        let hull_colour = self.hull_colour.describe();
        format!(
            "{} triangle(s) over {} submesh(es), {texture}, {} authored \
             normal(s) (rest off face normals); {} unaccounted GPU pointer(s){poisoned}{placeholders}{zone_colours}{track_shapes}{scrolls}{twins}{holograms}{tangents}{flipped}{lightmaps}{see_through}{nodes}{gnf}{lighting_only}{alpha_product}{flat_constants}{emissive_add}{pad_bars}{tarmac}{crowd}{snow}{rock}{vertex_blend}{alpha_emissive}{lightning}{refraction}{uv_keys}{panel}{hull_colour}",
            self.triangles, self.submeshes, self.authored_normals, self.unpaired
        )
    }
}
