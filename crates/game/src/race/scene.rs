//! [`Scene`]: the GPU-side composition of a race - the pipelines, the textures
//! and the per-slot drawables one frame is assembled from.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Drawing a
//! frame with it is `race/frame.rs`; its tests are `race/tests/scene.rs`.

use super::*;
use log::warn;

mod absorb_overlay;
mod absorb_shell;
mod beam;
mod clouds;
mod frame;
mod hd_chain;
mod motion;
mod per_slot;
mod queries;
mod scratch;
mod weapon_models;
mod weapon_quads;

use frame::{depth_texture, msaa_color_texture};
use motion::Attachments;
pub use motion::TemporalFrame;
pub(super) use scratch::Scratch;

/// The track and the ship on the GPU, drawn from a chase camera.
#[derive(Debug)]
pub struct Scene {
    track: Drawable,
    /// The circuit's own light rig, where it authors one.
    ///
    /// [`mesh_render::Light::stand_in`] for every title whose rig has not been
    /// recovered, which is all of them but Wipeout HD; HD states a sun
    /// direction, a sun colour and a constant ambient in the `.envsettings`
    /// beside its `track.vex`, and `CLAUDE.md`'s rule about not inventing what
    /// the assets already author is why that is read rather than approximated.
    /// See [`oag_tables::envsettings`].
    light: mesh_render::Light,
    /// The track's authored fog volumes, sampled at the camera each frame.
    ///
    /// Empty for a track that authors no `fogCube` - four of the forty - and the
    /// race then renders unfogged, which is what the original does too.
    fog_volumes: Vec<oag_vex::fog::FogVolume>,
    /// Wipeout HD's authored distance fog, static for the whole race - what
    /// binds when no `fogCube` volume covers the camera, which on HD is
    /// always, since HD authors no `fogCube` at all. See
    /// [`crate::race::Loaded::authored_fog`].
    authored_fog: Option<mesh_render::Fog>,
    /// The Zone colour grade laid over the two above, stage by stage -
    /// `None` outside Zone and on a title shipping no stage table. Held here
    /// rather than in `oag_gameplay::World` deliberately: nothing in the
    /// simulation reads it, no force depends on it, and `World` is
    /// exhaustively destructured into the determinism hash - so a field there
    /// would have to enter that hash and move its committed reference
    /// constants, which `CLAUDE.md` forbids outright. A colour grade is
    /// presentation, and this is where the rest of the presentation lives.
    /// See [`crate::race::zone_grade::ZoneGrade`].
    zone_grade: Option<crate::race::zone_grade::ZoneGrade>,
    /// The track's `Skycube`, drawn camera-centred before anything else -
    /// `None` when the file authors no sky, which is every Pure track and every
    /// non-track `.vex`. Its own [`mesh_render::Depth::Sky`] pipelines compare
    /// `Always` and write no depth, so it fills the frame and everything drawn
    /// after covers it - see [`Scene::render`] for why it is not scaled to the
    /// far plane instead.
    sky: Option<Drawable>,
    /// The track's `Speedup Pad` geometry, drawn with the track.
    ///
    /// Same pipeline, same textures, same fog, same world matrix as
    /// [`Self::track`]; separate only because a pad belongs to no visibility
    /// `section`, so it is offered frustum culling but not the PVS. `None` when
    /// the track authors none.
    pads: Option<Drawable>,
    /// The track's `Weapon Pad` geometry, drawn beside [`Self::pads`].
    weapon_pads: Option<Drawable>,
    /// The start gantry, standing on the mount this circuit's own track
    /// geometry authors - see `race::gantry`. `None` on a circuit that
    /// authors no mount, which draws no gantry rather than a placed guess.
    gantry: Option<gantry::Gantry>,
    /// The track's authored visibility partition, when it decoded.
    ///
    /// `None` for a track with no `section` nodes - every Pure track - and the
    /// first tier is then skipped entirely rather than approximated.
    visibility: Option<TrackVisibility>,
    /// One drawable per craft, the player's first.
    ///
    /// Separate drawables over a cloned mesh rather than instancing: a
    /// `Drawable` owns its uniform buffer, and eight craft need eight matrices a
    /// frame. See `oag_render::mesh::Model`'s note on the trade.
    ships: Vec<Drawable>,
    /// One boost plume per craft: ordinary `Drawable`s with their blend pipeline
    /// overridden to [`exhaust::BLEND`] instead of
    /// [`mesh_render::TRANSPARENT_BLEND`].
    ///
    /// **Empty** when the source carries no `shipboost.vex` under this team's
    /// name - see `Loaded::boost_model`. Each is drawn only while its craft's
    /// [`Exhaust::plume_visible`] is true, with that craft's own model matrix,
    /// since the original parents the plume to the craft rather than to the
    /// flare.
    ///
    /// Eight copies of the mesh for the same reason [`Self::ships`] holds eight:
    /// a `Drawable` owns the uniform buffer its model matrix and its UV transform
    /// are written into, and two craft boosting at once need two of each. Empty
    /// rather than `Option<Vec<_>>` so the no-plume source and the iteration read
    /// the same way `ships` does.
    boost: Vec<Option<Drawable>>,
    /// One always-on engine flame per craft, for a title that authors its flare
    /// as geometry rather than as a sprite.
    ///
    /// **Empty of everything but `None` on Pulse, Pure and the PS2 port**,
    /// whose flare is the six camera-facing vertices `exhaust::Pipeline`
    /// draws: a different mechanism, not a missing model. See
    /// [`oag_title::flare::Flare`] and `crate::livery::flare`.
    ///
    /// Indexed by slot for the reason [`Self::boost`] is: eight craft need
    /// eight model matrices a frame, and the draw loop indexes rather than
    /// zips.
    flares: Vec<Option<Drawable>>,
    /// One shield shell per craft, on the same additive pipeline as
    /// [`Self::boost`] and for the same reason: both textures carry the artists'
    /// `_ADD` suffix.
    ///
    /// `None` for a slot whose source carries neither the per-team
    /// `shipshield.vex` nor the shared `Data\Weapons\shield.vex` - see
    /// `crate::livery::Livery::shield`, which reports which of the two answered.
    /// Drawn only while that craft's `oag_render::shield::ShipShield` is
    /// visible, with the craft's own model matrix scaled by the shell's swell.
    ///
    /// Eight copies for the reason [`Self::ships`] holds eight, plus one this
    /// one has on its own: the shell's colour is uploaded into its *vertex*
    /// buffer every frame (`Drawable::tint`), so two craft with shields up at
    /// different points of their fade need two buffers.
    shield: Vec<Option<Drawable>>,
    /// The shield seen from **inside** the cockpit: one sphere, not one per
    /// craft, because only the player's camera can ever be inside one.
    ///
    /// Drawn *instead of* the player's entry in [`Self::shield`] while
    /// `Race::draws_own_ship` is false, which is exactly the branch
    /// `ShipShield_Update` takes on `craft+0x6d`. `None` on a source that does
    /// not carry `Data\Weapons\vr_shield_cockpit.vex`.
    shield_cockpit: Option<Drawable>,
    /// The absorb hull overlay per slot - see [`absorb_overlay`].
    absorb_overlay: Vec<Option<Drawable>>,
    /// HD's absorb shell per slot - see [`absorb_shell`].
    absorb_shell: Vec<Option<Drawable>>,
    /// One drawable per projectile slot, for rockets drawn as their own model.
    ///
    /// **Empty** when `Data\Weapons\Rocket.vex` did not load, and the sprite
    /// fallback in [`Race::projectile_sprites`] carries the whole effect then.
    /// Sized to `MAX_PROJECTILES` and clone-per-slot for the same reason
    /// [`Self::ships`] is clone-per-craft: a `Drawable` owns the uniform buffer
    /// its model matrix goes in, and three rockets in the air at once need three
    /// matrices. Eighty-four vertices apiece makes sixteen copies cheap.
    rockets: Vec<Drawable>,
    /// The Mine's own drawables, one per pool slot, on the same terms as
    /// [`Self::rockets`]: empty when `Data\Weapons\Pulse_Mine.vex` did not
    /// load. See [`weapon_models::build`].
    mines: Vec<Drawable>,
    /// The Bomb's own, one size up from the Mine's model, same terms.
    bombs: Vec<Drawable>,
    /// A Cannon round's own, same terms: empty when
    /// `Data\Weapons\pulse_muzzleflash.vex` did not load, and a round then
    /// draws as nothing rather than as an invented stand-in.
    cannon_rounds: Vec<Drawable>,
    /// The Plasma's own drawables - the blast's three plus the bolt's own
    /// head, [`blast_models::PlasmaBlastDrawables::ball`]. See
    /// [`blast_models::PlasmaBlastModels`]'s own doc comment for why the
    /// bolt rides in this container rather than a field of its own.
    plasma_blast: blast_models::PlasmaBlastDrawables,
    /// Each slot's own plume's authored texture-transform keyframes, sampled
    /// per frame and applied to that plume's authored UVs - the recovered
    /// mechanism (`TEXMAPMODE` 0 plus the animated `TEXOFFSET` u-scroll; see
    /// `crate::livery::Livery::boost_uv`). `None` falls back to the engine's
    /// own identity default.
    ///
    /// **Per slot, because the plumes are.** Every team read so far carries
    /// the identical track, so sharing one would be invisible today and wrong
    /// the moment a team did not.
    boost_uv_transforms: Vec<Option<oag_vex::vex::TexTransform>>,
    /// The collision soup overlay, present only when `Options::collision` asked
    /// for it. Drawn with the identity transform, same as the track: the
    /// collision geometry is already in world space.
    collision: Option<Drawable>,
    /// The engine flare and the boost plume: the frame's blended pipelines.
    ///
    /// `RefCell` because its per-frame upload needs `&mut` while [`Scene::render`]
    /// stays `&self`. That signature is worth keeping: the alternative threads
    /// `&mut` through `RaceStage::render` and `race::capture` for a buffer write
    /// that `queue` already accepts through a shared reference. The borrow is
    /// taken and released inside `render` with nothing re-entrant in between.
    exhaust: std::cell::RefCell<exhaust::Pipeline>,
    weapon_quads: std::cell::RefCell<oag_render::weapon_quads::Pipeline>,
    clouds: std::cell::RefCell<clouds::Clouds>,
    /// The LeachBeam's own ribbon, `None` on an undecoded texture - see `beam`.
    beam: Option<std::cell::RefCell<oag_render::beam::Pipeline>>,
    /// Collision sparks. `RefCell` for the same reason [`Self::exhaust`] is.
    sparks: std::cell::RefCell<sparks::Pipeline>,
    /// The `blob` shadow tier: one ground-aligned quad per craft, drawn after
    /// the track and before the hulls.
    ///
    /// Built with the scene whatever `[render_profiles.<title>] shadows` says,
    /// for the reason [`Self::motion_blur`] is: the tier is read fresh every
    /// frame so the setting applies live, and at `off` nothing is uploaded and
    /// the draw returns immediately. `RefCell` for the reason
    /// [`Self::exhaust`] is one. See `oag_render::shadow` and
    /// [`crate::race::shadow`], which split what is the disc's from what is
    /// ours.
    shadow: std::cell::RefCell<oag_render::shadow::Pipeline>,
    /// The `original` tier's shadow map on Wipeout HD: what the craft cast
    /// into and the track samples. `RefCell` for the reason [`Self::exhaust`]
    /// is - its pass is encoded inside `render`'s `&self`.
    ///
    /// Built for every race and bound by every drawable, because the setting
    /// applies live; the strength in `Scene::shadow` is what turns it on. See
    /// `oag_render::shadow::map`.
    shadow_map: std::cell::RefCell<oag_render::shadow::map::Map>,
    /// The `original` tier's two per-craft maps on Wipeout HD: the track
    /// under the craft drawn from the sun as its own baked sun-occlusion mask,
    /// and the craft's own depth from the sun, which together gate the hull's
    /// sun term. Built for every race and bound by
    /// every drawable like [`Self::shadow_map`]; a hull that names no layer
    /// never samples it. See `oag_render::shadow::occlusion`.
    sun_occlusion: std::cell::RefCell<oag_render::shadow::occlusion::Maps>,
    /// The `original` tier's geometry, one authored hull per grid slot where
    /// the craft's model carries one. CPU-side: it is projected afresh every
    /// frame against the surface under the craft, so there is nothing to
    /// upload until then. See [`crate::race::shadow::hulls`].
    shadow_hulls: Vec<Option<oag_vex::shadow_occluder::Occluder>>,
    /// The four vertex lists [`Scene::render`] gathers each frame, kept so
    /// their capacity is.
    ///
    /// They were four `Vec::new()`s a frame, and the ribbon is what made that
    /// cost real: `Exhaust::trail_vertices` produces a fixed 648 vertices per
    /// craft, so a full grid grew the trail list from empty to 5,184 by
    /// doubling - eight appends over four reallocations, every one copying
    /// what it had so far. Measured on Talon's Junction, gathering the exhaust
    /// allocated 1.18 MB a frame with a full grid; cleared and refilled they
    /// reach their high-water mark once and stay there.
    ///
    /// `RefCell` for the reason [`Self::exhaust`] is one: [`Scene::render`]
    /// takes `&self`, and this is per-frame scratch rather than scene state -
    /// nothing reads it between frames, and [`Scratch::clear`] is the first
    /// thing done with it.
    scratch: std::cell::RefCell<Scratch>,
    /// The recovered bloom, run after the scene pass over whatever the frame
    /// stamped into its alpha channel. `None` when the pipelines would not
    /// build, which costs the glow and nothing else.
    ///
    /// **Not exhaust-specific**, even though the exhaust is currently its only
    /// writer: it blooms the glow mask, and any surface that opts into the mask
    /// is handled by the same three passes. See `oag_render::post::bloom`.
    bloom: Option<oag_render::post::bloom::Bloom>,
    /// Wipeout HD's post chain: the linear float scene target the whole race
    /// draws into, the read `FunkLayerBloom` passes over it, and the encode
    /// into the caller's own view. `None` for every other title, where the
    /// scene draws straight into the caller's target as it always has. See
    /// `oag_render::post::hd_bloom` and [`Scene::render`].
    hd: Option<oag_render::post::hd_bloom::Chain>,
    /// Per-object motion blur, run last over the finished frame - an
    /// enhancement of this project's, not a reading of the original. Built
    /// with the scene whatever `[graphics] motion_blur` says, because that
    /// setting is a *strength* [`Scene::render`] reads fresh every frame -
    /// the row applies live, per `docs/rendering/motion-blur.md`'s design -
    /// and at `off` the pass simply never encodes anything. `RefCell` for
    /// the reason [`Self::exhaust`] is: its scratch targets resize inside
    /// `render`'s `&self`.
    ///
    /// It reads [`Self::velocity`] and the depth attachment; under MSAA both
    /// are multisampled and the pass's prepare stage reads sample 0, so
    /// unlike the camera-reprojection tier this replaced there is no MSAA
    /// gate. See `oag_render::post::motion_blur` and ADR-0030.
    motion_blur: Option<std::cell::RefCell<oag_render::post::motion_blur::MotionBlur>>,
    /// The scene's velocity attachment: every draw's screen-space motion
    /// since the previous tick, in uv units -
    /// `oag_render::mesh_render::VELOCITY_FORMAT`, at the scene's own sample
    /// count. **Always written in the game path**, whatever the blur setting
    /// says, per the design: the buffer is an FSR 3.1/TAA prerequisite as
    /// much as a blur input, and gating it on a setting would make it a
    /// sometimes-there artefact nothing downstream could rely on.
    velocity: wgpu::Texture,
    /// The previous tick's camera and model matrices, for the `prev_mvp`
    /// every drawable's velocity is measured against - see
    /// [`motion::MotionState`]. `RefCell` for the reason [`Self::exhaust`]
    /// is: promoting a tick's snapshot is per-frame state `render`'s `&self`
    /// has to move.
    motion: std::cell::RefCell<Option<motion::MotionState>>,
    /// The camera-jitter phase: frames, not ticks. See [`Scene::jittered`].
    frame_index: std::cell::Cell<u32>,
    /// What the last [`Scene::render`] drew with, for a temporal upscaler that
    /// runs *after* it and cannot see any of it.
    ///
    /// **Recorded rather than recomputed.** The jitter offset, the sequence
    /// length and the camera are all decided inside `render` - the offset from
    /// a counter this scene owns, the camera from the tick's own speed-widened
    /// field of view - and a caller re-deriving them would be a second answer
    /// to the same question, wrong in exactly the way that produces a
    /// plausible picture. `None` until the first frame.
    last_frame: std::cell::Cell<Option<TemporalFrame>>,
    /// The camera view and player respawn count as of the last recorded
    /// frame, for detecting a camera cut mid-race - see `motion::CutWatch`.
    cut_watch: std::cell::Cell<motion::CutWatch>,
    depth: wgpu::Texture,
    /// The colour attachment every pipeline here actually draws into, and its
    /// sample count.
    ///
    /// `Some` for `[graphics] anti_aliasing`'s two MSAA levels: every pipeline
    /// above is built at `sample_count`, and [`Scene::render`] draws them into
    /// this multisampled target and resolves it into the caller's own view at
    /// the end of the one pass. `None` at sample count 1, where there is
    /// nothing to resolve and the pipelines draw straight into the caller's
    /// view - see [`Scene::render`].
    ///
    /// Baked in at [`Scene::new`] rather than read from settings each frame:
    /// every pipeline's `multisample` state is fixed at the moment it is
    /// built, so changing this setting mid-race would need every pipeline
    /// above rebuilt, not just this texture. See
    /// `oag_display::display::AntiAliasing::msaa_samples`.
    msaa_color: Option<wgpu::Texture>,
    /// The three attachment views the race pass binds, held rather than made
    /// each frame.
    ///
    /// A `create_view` is a driver object - a `VkImageView` or its equivalent -
    /// and the three textures behind these move only in [`Self::new`] and
    /// [`Self::resize`], so a per-frame rebuild was three allocations and three
    /// driver calls producing the same three handles every time. They are also
    /// what lets `oag_render::post::motion_blur` cache its own bind groups:
    /// a group is only reusable while the views inside it are, and views made
    /// fresh each frame are never the same views twice.
    attachment_views: Attachments,
    /// What this scene's pipelines were actually built with, for the
    /// GRAPHICS menu's restart note - see `Session::open_menus` in `main.rs`.
    msaa: oag_display::display::Msaa,
    /// Where the far plane goes, from the track's own extent.
    far: f32,
}

impl Scene {
    /// Builds both pipelines and a depth buffer for a viewport of `size`.
    ///
    /// # Errors
    ///
    /// Propagates a pipeline or geometry upload failure from `oag-render`.
    // Three models plus how to draw them (surface format, viewport, texture
    // filtering); a wrapper struct for one call site would name the grouping
    // without clarifying it.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        track_model: Model,
        liveries: &[Livery],
        collision_model: Option<Model>,
        sky_model: Option<Model>,
        pad_model: Option<Model>,
        weapon_pad_model: Option<Model>,
        gantry: Option<gantry::Placed>,
        mode: Mode,
        rocket_model: Option<Model>,
        mine_model: Option<Model>,
        bomb_model: Option<Model>,
        cannon_model: Option<Model>,
        plasma_blast_models: blast_models::PlasmaBlastModels,
        shield_cockpit: Option<Model>,
        flare: Option<FlareTexture>,
        leach_beam_texture: Option<FlareTexture>,
        noise: Option<FlareTexture>,
        trail_blend: Option<wgpu::BlendState>,
        trail_shape: Option<FlareTexture>,
        cannon_quad_textures: (Option<FlareTexture>, Option<FlareTexture>),
        cloud_layer: Option<(oag_render::cloud::Layer, FlareTexture)>,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        anisotropy: Anisotropy,
        bloom_enabled: bool,
        visibility: Option<TrackVisibility>,
        msaa: oag_display::display::Msaa,
        fog_volumes: Vec<oag_vex::fog::FogVolume>,
        light: mesh_render::Light,
        authored_fog: Option<mesh_render::Fog>,
        hd_bloom: Option<oag_render::post::hd_bloom::Params>,
        zone_grade: Option<crate::race::zone_grade::ZoneGrade>,
        shadows: Vec<oag_render::shadow::Silhouette>,
        shadow_hulls: Vec<Option<oag_vex::shadow_occluder::Occluder>>,
    ) -> Result<Self> {
        // The far plane comes from the track's own bounding sphere: a track is
        // hundreds of units across, and a fixed guess would either clip it away or
        // waste the depth range on empty space.
        let far = track_model.radius * 4.0;
        let sample_count = msaa.samples();
        let scene_depth = mesh_render::Depth::Scene;
        // See `hd_chain::build`'s own doc comment for what this chain is and
        // why `format` comes back shadowed.
        let (hd, format, caller_format) =
            hd_chain::build(device, format, size, hd_bloom, bloom_enabled);
        // **The Zone stage's own texture, bound once per model.** The showing
        // stage's `zoneModeTrack<n>.gtf`, which `mesh.wgsl` samples at
        // `zoneColourTint.xy * (1 - meshUV)` wherever the material's albedo is
        // pure black - see `mesh_render::Zone`. `None` outside a Zone race, or
        // on a title with no located set, and then the shader's own
        // `zone.enabled` is zero too and nothing is added.
        //
        // **Bound at build time, so it does not follow a stage change.** HD
        // does step stages mid-race now, and the original publishes a
        // `zoneTexInner`/`zoneTexOuter` pair the way it publishes the colour
        // sets - but re-binding every drawable's scene group mid-race is
        // machinery this has no caller for yet, so a stage step moves the
        // two colour sets over the opening stage's texture. Stated on
        // `docs/rendering/hd-zone-recolour.md` as open.
        let zone_art = zone_grade
            .as_ref()
            .map_or(mesh_render::zone::StageArt::NONE, |grade| {
                grade.stage_art_pair()
            });
        let zone_art = &zone_art;
        // **Built for every race, whatever the setting says**, and bound by
        // every drawable: `Scene::shadow.strength` at zero is what makes it
        // inert, so the shadow row applies live without rebuilding a pipeline.
        // A caller that never casts pays one megabyte and a cleared pass.
        let shadow_map = oag_render::shadow::map::Map::new(device);
        let sun_occlusion = oag_render::shadow::occlusion::Maps::new(
            device,
            &mesh_render::material_bind_group_layout(device),
        );
        let shadow_maps = mesh_render::ShadowMaps {
            coverage: Some(shadow_map.view()),
            depth: Some(shadow_map.depth_view()),
            occlusion: Some(sun_occlusion.view()),
            self_shadow: Some(sun_occlusion.self_shadow().view()),
        };
        let sky = sky_model
            .filter(|model| !model.indices.is_empty())
            .map(|model| {
                Drawable::new(
                    device,
                    queue,
                    model,
                    format,
                    anisotropy,
                    sample_count,
                    mesh_render::Depth::Sky,
                    mesh_render::TRANSPARENT_BLEND,
                    mesh_render::GlowMask::Protected,
                    zone_art,
                    shadow_maps,
                    // The sky reads no map at all: it is drawn at infinity with the
                    // depth test disabled, so a shadow on it is a dark patch hanging
                    // in the air.
                    mesh_render::ShadowReceiver::Never,
                )
            })
            .transpose()?;
        let track = Drawable::new(
            device,
            queue,
            track_model,
            format,
            anisotropy,
            sample_count,
            scene_depth,
            mesh_render::TRANSPARENT_BLEND,
            mesh_render::GlowMask::Protected,
            zone_art,
            shadow_maps,
            // **The one receiver of the coverage map**, which is what Wipeout
            // HD's own materials say: the track surface declares
            // `shadowMapTex` and a craft's does not. It reads the `mapped`
            // tier's depth map too, hence `Both`.
            mesh_render::ShadowReceiver::Both,
        )?;
        // One per grid slot, each drawing **its own team's hull**. Built up
        // front rather than on demand, because a `Drawable` needs the device
        // and the pass does not have it. A grid shorter than `GRID_SLOTS`
        // liveries repeats the last one rather than panicking - `livery::load`
        // returns one per slot, so that is a defensive floor and not a path
        // any caller takes.
        let mut ships = Vec::with_capacity(GRID_SLOTS as usize);
        for slot in 0..GRID_SLOTS as usize {
            let livery = &liveries[slot.min(liveries.len().saturating_sub(1))];
            ships.push(Drawable::new(
                device,
                queue,
                livery.hull.clone(),
                format,
                anisotropy,
                sample_count,
                scene_depth,
                mesh_render::TRANSPARENT_BLEND,
                mesh_render::GlowMask::Protected,
                zone_art,
                shadow_maps,
                mesh_render::ShadowReceiver::Mapped,
            )?);
        }
        let collision = collision_model
            .map(|model| {
                Drawable::new(
                    device,
                    queue,
                    model,
                    format,
                    anisotropy,
                    sample_count,
                    scene_depth,
                    mesh_render::TRANSPARENT_BLEND,
                    mesh_render::GlowMask::Protected,
                    zone_art,
                    shadow_maps,
                    mesh_render::ShadowReceiver::Mapped,
                )
            })
            .transpose()?;
        let pad_drawable = |model: Option<Model>| {
            model
                .filter(|model| !model.indices.is_empty())
                .map(|model| {
                    Drawable::new(
                        device,
                        queue,
                        model,
                        format,
                        anisotropy,
                        sample_count,
                        scene_depth,
                        mesh_render::TRANSPARENT_BLEND,
                        mesh_render::GlowMask::Protected,
                        zone_art,
                        shadow_maps,
                        mesh_render::ShadowReceiver::Mapped,
                    )
                })
                .transpose()
        };
        let pads = pad_drawable(pad_model)?;
        // `World_CollectNodeLists` clears the node's own visibility bit
        // whenever `g_weapons_enabled` is `0`, which the front end forces for
        // time trial, speed lap and Zone alike - see `Mode::weapons_enabled`.
        // **The same predicate gates the trigger** (`Race::test_weapon_pads`),
        // deliberately: in the original both come from that one global, so a
        // pad that draws is a pad that can be crossed and there is no mode
        // where one is true and the other is not.
        // The geometry is still decoded and still in
        // `Loaded` either way (`the_weapon_pads_are_drawn_where_they_trigger`
        // checks the decode, not the mode), so this is the one place that
        // decision turns into "never uploaded to the GPU at all" rather than
        // "decoded, then hidden" - closer to what clearing a visibility bit
        // before the submit pass actually does than a flag checked at draw
        // time would be.
        let weapon_pads = if mode.weapons_enabled() {
            pad_drawable(weapon_pad_model)?
        } else {
            None
        };
        // The start gantry, already measured onto its mount by `race::gantry`.
        let gantry = gantry
            .map(|placed| {
                gantry::Gantry::new(
                    device,
                    queue,
                    placed,
                    format,
                    anisotropy,
                    sample_count,
                    shadow_maps,
                )
            })
            .transpose()?;
        // The boost plume, drawn additively. It is real geometry (a `.vex`
        // mesh, not a camera-facing quad), so it needs depth testing and a
        // model matrix, which is what `Drawable` already gives every other
        // mesh here rather than a third thing beside `exhaust::Pipeline`.
        // An additive blend in place of `TRANSPARENT_BLEND` is the one thing
        // that has to differ - the model's texture is named `_ADD`, and
        // drawing it with the ordinary lerp blend looks plausible and is
        // wrong.
        //
        // **Which additive blend changed on 2026-08-08, from the ribbon's
        // `TRAIL_BLEND` to the flare's `exhaust::BLEND`, and it is a bug fix
        // rather than a retuning.** The two differ only in the source factor -
        // `One` against `SrcAlpha` - and the plume is the one model here whose
        // authored vertex alpha is not constant: it is bimodal, `0` on the
        // orange rim and `255` in the white core (pinned by
        // `boost_plume_ground_truth.rs`). Under `TRAIL_BLEND` alpha cannot
        // reach the picture at all, so the load site compensated by
        // premultiplying it into the RGB *per vertex* - and that is simply the
        // wrong arithmetic. Premultiplying at a vertex and then interpolating
        // is not interpolating and then multiplying: it maps the rim to
        // **black**, so what interpolates across the fin is black-to-white and
        // the authored orange never exists anywhere on it. `SrcAlpha` does the
        // same multiply *per fragment*, after the rasteriser has interpolated
        // colour and alpha separately, which is what real hardware does and
        // what leaves a dimmed orange fringe fading out along the fin.
        //
        // Measured against the first matched-pose pad capture (boost age
        // `0.367` s, the original's own recorded camera and its measured
        // entry intensity), over the magenta signature `r > 110, b > 110,
        // r - g > 25, b - g > 20` in a crop around the craft:
        //
        // | build | pixels | mean colour | `b - r` |
        // | --- | ---: | --- | ---: |
        // | the original | 9,217 | `(223, 164, 224)` | +1 |
        // | premultiplied, `TRAIL_BLEND` | 493 | `(180, 141, 228)` | +48 |
        // | raw, `BLEND` (this) | 1,993 | `(159, 118, 175)` | +16 |
        //
        // Extent 4x better and the red/blue balance three times closer to the
        // original's neutral. The premultiplied build was blue-dominant; raw
        // alpha alone recovers the hue but draws the fins as hard-edged solid
        // wedges, which is the 2026-08-07 and 2026-08-08 result reproduced
        // exactly, and `SrcAlpha` is what supplies the falloff those wedges are
        // missing. Still 22% of the original's extent - but that comparison is
        // against the original's frame, so it is one of the readings the
        // 2026-08-09 projection finding invalidates: our whole image is zoomed
        // 1.12-1.26x depending on speed, because the original widens its fov
        // with speed and we do not. Re-take it with `--camera-fov` before
        // leaning on the number. The missing bright-pass is unaffected and
        // still real; "a craft drawn too large" was the wrong half and is
        // withdrawn - see `docs/rendering/projection-vs-the-original.md`.
        //
        // **This is an empirical approximation and the real mechanism is
        // something else.** The GE state for the transparent mesh pass is now
        // read: `GU_ALPHA_TEST` is *disabled*, `GU_BLEND` is *enabled* with
        // `GU_FIX` white on both sides, and nothing in the pass scales fragment
        // RGB but vertex colour x texture x the blend - so no source-alpha
        // weight exists on the hardware at all. What supplies the original's
        // falloff is the *texture*: the pass sets `TEXMAPMODE` uvgen 2,
        // environment mapping from the vertex normal and lights 0/1, and
        // `pulse_boost2_ADD`'s own bright-to-dark gradient varies across the
        // fin. Every plume batch declares normals (`vtype = 0x013d` on all 32
        // across 8 teams - `cargo run -p oag-assets --example
        // boost_vertex_type`), so that generation has something to vary with.
        //
        // **Environment-mapped UV generation was implemented here 2026-08-09
        // and replaced 2026-08-10**: the plume's compiled list is replayed
        // under `TEXMAPMODE` 0 (settled by reading the recorded frame stream
        // in GE order - mesh-draw.md, "The plume is replayed under
        // `TEXMAPMODE` 0"), so the original samples the *authored* UVs
        // through the keyframed `TEXOFFSET` u-scroll authored in the file
        // itself. `Drawable::apply_uv_transform` now does exactly that;
        // `oag_render::texgen` remains correct for batches genuinely inside
        // the transparent-pass bracket, which the plume is not.
        //
        // **`SrcAlpha` stays, and it is no longer a stand-in - it is a
        // measured choice that beat the recovered alternative.** The argument
        // for going back to `TRAIL_BLEND` was strong on paper: the recovered
        // GE state really is `GU_FIX` white on both sides, `SrcAlpha` was only
        // ever introduced as a substitute for the missing texture falloff, and
        // that falloff now exists. So it was tried, at the original's own pose
        // (`data/traces/pad0-boost.csv` tick 62), against the original's own
        // frame (`data/shots/pad0-boost/tick00062.png`), over the capture's own
        // plume mask (`min(r, b) - g > 25` and `luma > 60`):
        //
        // | build | plume px | mean | `b - r` | orange px |
        // | --- | ---: | --- | ---: | ---: |
        // | the original | 9,435 | `(220, 165, 237)` | +17.6 | 230 |
        // | ours, authored UVs (what shipped before) | 4,480 | `(184, 143, 203)` | +19.0 | 2,679 |
        // | **ours, texgen + `SrcAlpha` (this)** | **5,996** | `(197, 150, 210)` | +13.3 | **2,041** |
        // | ours, texgen + `TRAIL_BLEND` (the recovered blend) | 1,958 | `(243, 181, 227)` | -15.6 | 7,030 |
        //
        // **The measurements live in
        // `docs/ghidra/functions/psp-pulse-usa/exhaust.md`**, not here. This
        // table was duplicated across three files and re-measured four times -
        // wrong pose age, a boost accumulator charging 100x too slowly,
        // ADR-0020 moving background luminance 54 %, and finally a mask that
        // gated on absolute brightness - and every duplicate drifted. One home.
        //
        // What survives all four, and is why this line reads `exhaust::BLEND`:
        // the recovered `One`/`One` blend is the worst row on every column,
        // restoring the authored `(255, 98, 5)` rim at full strength where the
        // original's plume and ribbon are one violet family with nothing near
        // that orange. Generated coordinates beat authored ones on every
        // column too.
        //
        // **So something in the recovered blend chain is still incomplete**,
        // and that is worth stating rather than papering over: the GE state
        // was read exhaustively - all five setters to their command byte,
        // `Gu_TexFunc` confirmed `MODULATE`/`TCC_RGBA` - and it does not
        // reproduce the picture, while an unrecovered source-alpha weight does.
        // `SrcAlpha` is kept because it measures better, not because it is
        // understood.
        //
        // **Do not read any column stated against the original as calibrated.**
        // This comment used to blame a craft that "renders about 1.4x too
        // large"; that is refuted - the mesh is correct to 0.15 % and the whole
        // frame is zoomed, because the original widens its fov with speed and
        // we do not. At this capture's 148.9 units/s that is roughly 1.25x, and
        // it shifts which of the original's pixels fall inside the mask. The
        // ours-versus-ours results above are unaffected: all three builds
        // render at the same pose, so they share one zoom and a ratio between
        // them cancels it. See `docs/rendering/projection-vs-the-original.md`,
        // `docs/ghidra/functions/psp-pulse-usa/exhaust.md` and `mesh-draw.md`.
        //
        // One per grid slot, cloned the way the hulls above are: every craft can
        // be on a speed pad at once, and eight plumes need eight model matrices
        // and eight sampled UV transforms a frame.
        //
        // **Each slot's own team's plume, sampled through its own team's
        // keyframes.** Eight per-team plumes played through one team's UV
        // track is the kind of wrong that renders plausibly, so the transform
        // is carried per slot beside the model. A slot whose team ships no
        // plume simply has none, which is why this is a `Vec` of `Option`
        // rather than a shorter `Vec` - the draw loop indexes by slot.
        let mut boost = Vec::new();
        let mut boost_uv_transforms = Vec::new();
        let mut flares = Vec::new();
        for slot in 0..GRID_SLOTS as usize {
            let livery = &liveries[slot.min(liveries.len().saturating_sub(1))];
            boost_uv_transforms.push(livery.boost_uv.clone());
            // **The material's own equation reaches the pipeline by itself.**
            // Every draw of an HD flare carries the factor pair its
            // `.rcsmaterial` authors (`SrcAlpha`/`One`), and
            // `mesh_render::TransparentPipelines::select` prefers that over any
            // class - so this argument is the fallback for a model that
            // authored none, and it is the same equation either way.
            //
            // **`GlowMask::Protected`, unlike the plume's.** The plume's
            // `Written` is a *recovered* override - the original's draw path
            // opens the alpha channel there. Nothing has been read out of HD's
            // executable about its flare, so it takes the default every
            // authored batch gets. It still reaches HD's bloom through that
            // chain's luminance term, which is not a substitute for the
            // reading and is not being treated as one.
            flares.push(
                match livery
                    .flare
                    .as_ref()
                    .filter(|model| !model.indices.is_empty())
                {
                    Some(model) => Some(Drawable::new(
                        device,
                        queue,
                        model.clone(),
                        format,
                        anisotropy,
                        sample_count,
                        scene_depth,
                        mesh_render::ADDITIVE_BLEND,
                        mesh_render::GlowMask::Protected,
                        zone_art,
                        shadow_maps,
                        mesh_render::ShadowReceiver::Mapped,
                    )?),
                    None => None,
                },
            );
            let Some(model) = livery
                .boost
                .as_ref()
                .filter(|model| !model.indices.is_empty())
            else {
                boost.push(None);
                continue;
            };
            {
                boost.push(Some(Drawable::new(
                    device,
                    queue,
                    model.clone(),
                    format,
                    anisotropy,
                    sample_count,
                    scene_depth,
                    exhaust::BLEND,
                    // **The plume feeds the bloom.** Its draw path in the
                    // original opens the alpha channel unconditionally, unlike
                    // the hull's - see `mesh_render::GlowMask`. Without this the
                    // boost's brightest surface contributes nothing to the glow
                    // mask, which is the shape of the effect a player notices.
                    mesh_render::GlowMask::Written,
                    zone_art,
                    shadow_maps,
                    mesh_render::ShadowReceiver::Mapped,
                )?));
            }
        }
        // **A PS2 plume reaches none of that**: its four batches carry no
        // `0x0700` class bit, so they land in `Model::draws` and `draw` would
        // submit them through the opaque pipeline. `frame` calls
        // `Drawable::draw_additive` for the plume instead, which puts every
        // list on `ADDITIVE_BLEND` - the same equation the PSP plume's own
        // `0x200` batches already select, so the two discs share one blend.
        // That is a **model-scoped override, not a decode**, and the reference
        // frame that justifies it plus the PS2 dispatch still unfollowed are
        // both on `draw_additive`. The `pass_mask & 0xc0` glow-mask stamp is a
        // separate and still-standing read, and it is why `GlowMask::Written`
        // below is right on both discs. See
        // `docs/ghidra/functions/ps2-pulse-eu/batch-draw-state.md`.

        // The shield shell, on the plume's own additive pipeline. Both models'
        // textures carry the artists' `_ADD` suffix - `pulse_shield_test_ADD`
        // here, `pulse_boost2_ADD` there - which is what says they share a
        // blend, and the shell's whole look is a bright surface over the hull
        // rather than a surface that occludes it.
        //
        // **Glow-mask written, like the plume and unlike the hull.** A raised
        // shield is one of the brightest things in the frame and the effect a
        // player is meant to notice; leaving it out of the mask would keep it
        // out of the bloom, which is where most of its presence comes from.
        // Unlike the plume, that is a reading rather than a recovered draw
        // path - the shell's own GE state has not been read.
        let slots = per_slot::Build {
            device,
            queue,
            liveries,
            format,
            anisotropy,
            sample_count,
            zone_art,
            shadow_maps,
        };
        let shield = slots.drawables(
            |l| l.shield.clone(),
            exhaust::BLEND,
            scene_depth,
            mesh_render::ShadowReceiver::Mapped,
        )?;
        // The cockpit sphere, on the shell's own pipeline: same additive blend,
        // same glow mask, same argument. One rather than eight - a craft whose
        // camera is inside it is the player's, and there is one of those.
        let shield_cockpit = match shield_cockpit.filter(|model| !model.indices.is_empty()) {
            Some(model) => Some(Drawable::new(
                device,
                queue,
                model,
                format,
                anisotropy,
                sample_count,
                scene_depth,
                exhaust::BLEND,
                mesh_render::GlowMask::Written,
                zone_art,
                shadow_maps,
                mesh_render::ShadowReceiver::Mapped,
            )?),
            None => None,
        };
        // The absorb hull overlay: the hull again, unshadowed, tested
        // `LessEqual` against its own depth - see `absorb_overlay`.
        let overlay = |l: &crate::livery::Livery| l.absorb_overlay.clone();
        let never = mesh_render::ShadowReceiver::Never;
        let absorb_overlay = slots.drawables(
            overlay,
            oag_render::hull_overlay::BLEND,
            mesh_render::Depth::Overlay,
            never,
        )?;
        let absorb_shell = slots.drawables(
            |l| l.absorb_shell.clone(),
            exhaust::BLEND,
            scene_depth,
            never,
        )?;
        // One per projectile slot: the Rocket's, the Mine's, the Bomb's, the
        // Cannon round's and the Plasma blast's, all built the same way - see
        // `weapon_models::build_all`. A laid mine's hull is opaque too - a
        // painted shell, not a glow - so every one takes the ordinary
        // transparent blend and protected glow mask the ships take, and any
        // flare stays in the additive pass with the exhaust where it belongs.
        let (rockets, mines, bombs, cannon_rounds, plasma_blast) = weapon_models::build_all(
            device,
            queue,
            rocket_model,
            mine_model,
            bomb_model,
            cannon_model,
            plasma_blast_models,
            format,
            anisotropy,
            sample_count,
            scene_depth,
            zone_art,
            shadow_maps,
        )?;
        // 64 is a stand-in size only, and only when the disc's own texture did not
        // decode; `load` has already reported that when it happens.
        let flare = flare.unwrap_or_else(|| FlareTexture::placeholder(64));
        let noise = noise.unwrap_or_else(|| FlareTexture::placeholder(64));
        // The ribbon's own blend where the title authors one, and the recovered
        // PSP preset's otherwise - see `Loaded::trail_blend`.
        let trail_blend = trail_blend.unwrap_or(exhaust::TRAIL_BLEND);
        let exhaust = std::cell::RefCell::new(exhaust::Pipeline::new(
            device,
            queue,
            format,
            &flare,
            &noise,
            trail_shape.as_ref(),
            trail_blend,
            sample_count,
            // The race pass carries the velocity attachment, so these
            // pipelines carry its (write-masked) second target - see
            // `mesh_render::Velocity`.
            mesh_render::Velocity::Write,
        ));
        let sparks = std::cell::RefCell::new(sparks::Pipeline::new(
            device,
            format,
            sample_count,
            mesh_render::Velocity::Write,
        ));
        let weapon_quads =
            weapon_quads::build(device, queue, format, cannon_quad_textures, sample_count);
        let beam = beam::build(
            device,
            queue,
            format,
            leach_beam_texture.as_ref(),
            sample_count,
        );
        // One silhouette per grid slot, in the same slot order the liveries
        // are in - `race::shadow::silhouettes` built them, and the load report
        // already said which slots got the disc's own image and which got the
        // generated falloff.
        let shadow = std::cell::RefCell::new(oag_render::shadow::Pipeline::new(
            device,
            queue,
            format,
            &shadows,
            sample_count,
            mesh_render::Velocity::Write,
        ));
        // A failure here is reported and dropped rather than propagated: a race
        // without a bloom is a dimmer race, not a broken one. The HD chain
        // replaces this pass outright - its read gate consumes the same glow
        // mask as one of its two terms - so the two never run together.
        let bloom = match (bloom_enabled && hd.is_none())
            .then(|| oag_render::post::bloom::Bloom::new(device, format))
            .transpose()
        {
            Ok(bloom) => bloom,
            Err(e) => {
                warn!("bloom unavailable ({e}) - the frame draws without it");
                None
            }
        };

        // Built against the caller's own format: whichever chain runs, the
        // frame this pass reads is the one already encoded into the caller's
        // view - after the HD chain's encode, after the PSP bloom's
        // composite. A failure is reported and dropped the way the bloom's
        // is: a race without motion blur is a sharper race, not a broken one.
        // See `Self::motion_blur` for why this is not gated on the setting.
        let motion_blur =
            match oag_render::post::motion_blur::MotionBlur::new(device, caller_format) {
                Ok(pass) => Some(std::cell::RefCell::new(pass)),
                Err(e) => {
                    warn!("motion blur unavailable ({e}) - the frame draws without it");
                    None
                }
            };

        let depth = depth_texture(device, size, sample_count);
        let velocity = motion::velocity_texture(device, size, sample_count);
        let msaa_color = msaa_color_texture(device, format, size, sample_count);
        let attachment_views = Attachments::new(&depth, &velocity, msaa_color.as_ref());
        Ok(Self {
            bloom,
            hd,
            motion_blur,
            velocity,
            motion: std::cell::RefCell::new(None),
            frame_index: std::cell::Cell::new(0),
            last_frame: std::cell::Cell::new(None),
            cut_watch: std::cell::Cell::new(motion::CutWatch::default()),
            track,
            visibility,
            ships,
            boost,
            flares,
            shield,
            shield_cockpit,
            absorb_overlay,
            absorb_shell,
            rockets,
            mines,
            bombs,
            cannon_rounds,
            plasma_blast,
            boost_uv_transforms,
            collision,
            sky,
            pads,
            weapon_pads,
            gantry,
            fog_volumes,
            light,
            authored_fog,
            zone_grade,
            exhaust,
            weapon_quads,
            clouds: clouds::Clouds::build(device, queue, format, sample_count, cloud_layer),
            beam,
            sparks,
            shadow,
            shadow_map: std::cell::RefCell::new(shadow_map),
            sun_occlusion: std::cell::RefCell::new(sun_occlusion),
            shadow_hulls,
            scratch: std::cell::RefCell::default(),
            depth,
            msaa_color,
            attachment_views,
            msaa,
            far,
        })
    }
}
