//! [`Scene`]: the GPU-side composition of a race - the pipelines, the textures
//! and the per-slot drawables one frame is assembled from.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Drawing a
//! frame with it is `race/frame.rs`; its tests are `race/tests/scene.rs`.

use super::*;
use log::{debug, warn};

mod absorb_overlay;
mod absorb_shell;
mod beam;
pub(crate) mod behind_glass;
mod bloom;
mod boost_flare;
mod clouds;
mod effects;
mod frame;
mod ghost;
mod hd_chain;
mod magstrip_wake;
mod mist;
mod motion;
mod per_slot;
mod queries;
mod scratch;
mod shine;
mod weapon_models;
mod weapon_quads;
mod wreck;

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
    /// [`crate::Loaded::authored_fog`].
    authored_fog: Option<mesh_render::Fog>,
    /// The Zone colour grade laid over the two above, stage by stage -
    /// `None` outside Zone and on a title shipping no stage table. Held here
    /// rather than in `oag_gameplay::World` deliberately: nothing in the
    /// simulation reads it, no force depends on it, and `World` is
    /// exhaustively destructured into the determinism hash - so a field there
    /// would have to enter that hash and move its committed reference
    /// constants, which `CLAUDE.md` forbids outright. A colour grade is
    /// presentation, and this is where the rest of the presentation lives.
    /// See [`crate::zone_grade::ZoneGrade`].
    zone_grade: Option<crate::zone_grade::ZoneGrade>,
    /// The track's `Skycube`, drawn camera-centred behind the solid circuit -
    /// `None` when the file authors no sky, which is every Pure track and every
    /// non-track `.vex`. Its own [`mesh_render::Depth::Sky`] pipelines write no
    /// depth and land only where nothing solid has - see `Scene::draw_track`,
    /// and [`mesh_render::Depth`] for why it is not scaled to the far plane
    /// instead.
    sky: Option<Drawable>,
    /// The track's `Speedup Pad` geometry, drawn with the track. Same
    /// pipeline, same textures, same fog, same world matrix as
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
    /// The billboard adverts, drawn once a frame into the targets the track's
    /// placeholder quads show - see [`crate::adverts`]. `None` with no card.
    adverts: Option<crate::adverts::Cards>,
    /// Vineta K's behind-the-glass target, drawn before the main view - see
    /// [`behind_glass`]. `None` on every other circuit.
    behind_glass: Option<behind_glass::Target>,
    /// The track's authored visibility partition, when it decoded. `None`
    /// for a track with no `section` nodes - every Pure track - and the
    /// first tier is then skipped entirely rather than approximated.
    visibility: Option<TrackVisibility>,
    /// One drawable per craft, the player's first.
    ///
    /// Separate drawables over a cloned mesh rather than instancing: a
    /// `Drawable` owns its uniform buffer, and eight craft need eight matrices a
    /// frame. See `oag_mesh::mesh::Model`'s note on the trade.
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
    /// Eight copies for the same reason [`Self::ships`] holds eight: a
    /// `Drawable` owns the uniform buffer its model matrix and UV transform
    /// write into, and two craft boosting at once need two. Empty rather
    /// than `Option<Vec<_>>` so the no-plume source and the iteration read
    /// the same way `ships` does.
    boost: Vec<Option<Drawable>>,
    /// One always-on engine flame per craft, for a title that authors its flare
    /// as geometry rather than as a sprite.
    ///
    /// **Empty of everything but `None` on Pulse, Pure and the PS2 port**,
    /// whose flare is the six camera-facing vertices `exhaust::Pipeline`
    /// draws: a different mechanism, not a missing model. See
    /// [`oag_title::flare::Flare`] and `oag_livery::flare`.
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
    /// `oag_livery::Livery::shield`, which reports which of the two answered.
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
    /// The absorb, then the LeachBeam hull overlay per slot - see [`absorb_overlay`].
    absorb_overlay: [Vec<Option<Drawable>>; 2],
    /// The hull's environment-mapped extra pass per slot - see [`shine`].
    shine: Vec<Option<Drawable>>,
    /// Every slot's wreck, drawn instead of its hull once the craft is out of
    /// the race - see [`wreck`].
    wrecks: wreck::Wrecks,
    /// The circuit's own extra pass - see [`shine::TrackShine`].
    track_shine: Option<shine::TrackShine>,
    /// This frame's animation clock, in seconds: set where the frame reads it,
    /// read where the circuit's extra pass samples its animated nodes.
    anim_clock: std::cell::Cell<f32>,
    /// HD's absorb shell per slot - see [`absorb_shell`].
    absorb_shell: Vec<Option<Drawable>>,
    /// One drawable per projectile slot, for rockets drawn as their own
    /// model. **Empty** when `Data\Weapons\Rocket.vex` did not load, and the sprite
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
    /// `Data\Weapons\pulse_muzzleflash.vex` did not load - a round then
    /// draws as nothing rather than an invented stand-in.
    cannon_rounds: Vec<Drawable>,
    /// The Plasma's own drawables - the blast's three plus the bolt's own
    /// head, [`blast_models::PlasmaBlastDrawables::ball`]; see that type's
    /// own doc comment for why the bolt rides in this container.
    plasma_blast: blast_models::PlasmaBlastDrawables,
    /// The Bomb's own - a hemisphere and a shockwave; see [`bomb_blast::BombBlastDrawables`].
    bomb_blast: bomb_blast::BombBlastDrawables,
    /// The LeachBeam's own ball, HD only - `None` when
    /// `Data\Weapons\hd_leachbeam_ball_bloomring.vex` did not load. Drawn
    /// at [`oag_fx::beam::hd_ball::position`] each tick a beam is locked
    /// (see [`Race::leach_ball_model_matrix`] and `scene/frame/beam.rs`),
    /// through its own material's program, `mesh::rcs::rim_glow`'s
    /// `RIM_GLOW` (docs/rendering/hd-unlit-programs.md).
    leach_ball: Option<Drawable>,
    /// Each slot's own plume's authored texture-transform keyframes, sampled
    /// per frame and applied to that plume's authored UVs - the recovered
    /// mechanism (`TEXMAPMODE` 0 plus the animated `TEXOFFSET` u-scroll; see
    /// `oag_livery::Livery::boost_uv`). `None` falls back to the engine's
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
    weapon_quads: weapon_quads::Cannon,
    clouds: std::cell::RefCell<clouds::Clouds>,
    /// The weather's mist overlay, once [`Scene::attach_mist`] built it.
    mist: std::cell::RefCell<Option<oag_fx::mist::Pipeline>>,
    /// The LeachBeam's own ribbon, `None` on an undecoded texture - see `beam`.
    beam: Option<std::cell::RefCell<oag_fx::beam::Pipeline>>,
    /// The HD-lineage magstrip arc wake, `None` off a title that builds it.
    magstrip: Option<magstrip_wake::Magstrip>,
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
    /// [`crate::shadow`], which split what is the disc's from what is
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
    /// How many `blob` silhouettes this project generated for want of the
    /// disc's own, and whether the log has said so yet - see
    /// `Scene::shadow_geometry`.
    generated_silhouettes: usize,
    shadow_silhouette_count: usize,
    generated_silhouettes_said: std::cell::Cell<bool>,
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
    /// upload until then. See [`crate::shadow::hulls`].
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
    /// This frame's front-to-back order of the circuit's opaque draws, under
    /// HD's chain only - see `Scene::draw_track`. Kept apart from
    /// [`Self::scratch`], which `render` holds borrowed across the pass.
    opaque_order: std::cell::RefCell<Vec<(f32, u32)>>,
    /// The recovered bloom, run after the scene pass over whatever the frame
    /// stamped into its alpha channel. `None` when the pipelines would not
    /// build, which costs the glow and nothing else.
    ///
    /// **Not exhaust-specific**, even though the exhaust is currently its only
    /// writer: it blooms the glow mask, and any surface that opts into the mask
    /// is handled by the same three passes. See `oag_post::bloom`.
    bloom: Option<oag_post::bloom::Bloom>,
    /// Pulse PS2's own bloom, in place of [`Self::bloom`] over a PS2 race's
    /// glow mask. See `oag_post::ps2_bloom`.
    ps2_bloom: Option<oag_post::ps2_bloom::Ps2Bloom>,
    /// Whether [`Self::bloom`] or [`Self::ps2_bloom`] was prepared this frame and still owes its
    /// composite: [`Scene::composite_bloom`], after the HUD.
    bloom_pending: std::cell::Cell<bool>,
    /// Wipeout HD's post chain: the linear float scene target the whole race
    /// draws into, the read `FunkLayerBloom` passes over it, and the encode
    /// into the caller's own view. `None` for every other title, where the
    /// scene draws straight into the caller's target as it always has. See
    /// `oag_post::hd_bloom` and [`Scene::render`].
    hd: Option<oag_post::hd_bloom::Chain>,
    /// Omega's tone map: the same linear float scene target, then the
    /// executable's adaptive exposure and cubic curve, and the encode. `None`
    /// for every other title. See `oag_post::omega_tonemap`.
    omega: Option<oag_post::omega_tonemap::Chain>,
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
    /// gate. See `oag_post::motion_blur` and ADR-0030.
    motion_blur: Option<std::cell::RefCell<oag_post::motion_blur::MotionBlur>>,
    /// Whether that blur gathers at half resolution - the profile's
    /// `motion_blur_resolution`, handed in before each frame by
    /// [`Scene::set_blur_resolution`].
    blur_half: std::cell::Cell<bool>,
    /// The scene's velocity attachment: every draw's screen-space motion
    /// since the previous tick, in uv units -
    /// `oag_gpu::formats::VELOCITY_FORMAT`, at the scene's own sample
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
    /// what lets `oag_post::motion_blur` cache its own bind groups:
    /// a group is only reusable while the views inside it are, and views made
    /// fresh each frame are never the same views twice.
    attachment_views: Attachments,
    /// What this scene's pipelines were actually built with, for the
    /// GRAPHICS menu's restart note - see `Session::open_menus` in `main.rs`.
    msaa: oag_display::display::Msaa,
    /// Where the far plane goes, from the track's own extent.
    far: f32,
    /// The ghost ship - see [`ghost`].
    ghost: ghost::Ghosts,
    /// How many of [`Self::new`]'s render pipelines were asked for, reused and
    /// built, from `mesh_render::BuildCacheScope`. Kept so a test can tell a
    /// pool of 128 drawables sharing one pipeline set from one that compiled
    /// 128 of them.
    build_cache: BuildCacheCounts,
}

/// One scene build's pipeline cache: `(asked for, reused, distinct built)`.
type BuildCacheCounts = (u32, u32, usize);

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
        billboards: crate::adverts::Billboards,
        weapons_on: bool,
        rocket_model: Option<Model>,
        mine_model: Option<Model>,
        bomb_model: Option<Model>,
        cannon_model: Option<Model>,
        plasma_blast_models: blast_models::PlasmaBlastModels,
        bomb_blast_models: bomb_blast::BombBlastModels,
        leach_ball_model: Option<Model>,
        shield_cockpit: Option<Model>,
        flare: Option<FlareTexture>,
        leach_beam_texture: Option<FlareTexture>,
        magstrip_wake_textures: Option<[FlareTexture; 2]>,
        noise: Option<FlareTexture>,
        trail_blend: Option<wgpu::BlendState>,
        trail_shape: Option<FlareTexture>,
        cannon: crate::CannonAssets,
        cloud_layer: Option<(oag_fx::cloud::Layer, FlareTexture)>,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        anisotropy: Anisotropy,
        visibility: Option<TrackVisibility>,
        msaa: oag_display::display::Msaa,
        fog_volumes: Vec<oag_vex::fog::FogVolume>,
        light: mesh_render::Light,
        authored_fog: Option<mesh_render::Fog>,
        hd_bloom: Option<oag_post::hd_bloom::Params>,
        omega_tonemap: Option<oag_post::omega_tonemap::Params>,
        zone_grade: Option<crate::zone_grade::ZoneGrade>,
        shadows: Vec<oag_render::shadow::Silhouette>,
        shadow_hulls: Vec<Option<oag_vex::shadow_occluder::Occluder>>,
        behind_glass: behind_glass::BehindGlassModels,
    ) -> Result<Self> {
        // **Opened here, not by the caller.** Every drawable below shares this
        // `device`, so `mesh_render::build` parses `mesh.wgsl` once and reuses
        // a pipeline whenever two drawables ask for the descriptor-identical
        // one. A caller that opened it itself is a caller that can forget: the
        // `--screenshot` path did, and compiled 9,859 pipelines for 53
        // distinct ones - about 7 MiB of resident memory per weapon drawable,
        // 8.4 GiB for a Pulse race. See `pipeline_cache`.
        let crate::adverts::Billboards { gantry, adverts } = billboards;
        let cache_scope = mesh_render::BuildCacheScope::open();
        cache_scope.lit_by(&light);
        // The far plane comes from the track's own bounding sphere: a track is
        // hundreds of units across, and a fixed guess would either clip it away or
        // waste the depth range on empty space.
        let far = track_model.radius * 4.0;
        let sample_count = msaa.samples();
        let scene_depth = mesh_render::Depth::Scene;
        // See `hd_chain::build`'s own doc comment for what this chain is and
        // why `format` comes back shadowed.
        let (hd, format, caller_format) = hd_chain::build(device, format, size, hd_bloom);
        let (omega, format) =
            hd_chain::build_omega(device, format, caller_format, size, omega_tonemap);
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
            behind_glass: None,
        };
        let behind_glass = behind_glass::Target::build(
            device,
            queue,
            behind_glass,
            sky_model.as_ref(),
            format,
            anisotropy,
            zone_art,
            shadow_maps,
        )?;
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
        let measured_mask = track_model.stamps_glow; // see `bloom` below
        let ps2_mask = measured_mask && track_model.glow_by_texel;
        let track_shine = shine::TrackShine::build(
            device,
            queue,
            &track_model,
            format,
            anisotropy,
            sample_count,
            zone_art,
            shadow_maps,
        )?;
        let placeholders = oag_render::gantry::placeholder_texture_slots(&track_model);
        let mut track = Drawable::new_with(
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
            // The circuit's glass reads the behind-the-glass target.
            mesh_render::ShadowMaps {
                behind_glass: behind_glass.as_ref().map(behind_glass::Target::colour),
                ..shadow_maps
            },
            // **The one receiver of the coverage map**, which is what Wipeout
            // HD's own materials say: the track surface declares
            // `shadowMapTex` and a craft's does not. It reads the `mapped`
            // tier's depth map too, hence `Both`.
            mesh_render::ShadowReceiver::Both,
            mesh_render::Texcoords::Interleaved,
            // HD's circuit alone: its opaque list shades about two fragments
            // a pixel, where Pulse's shades 1.2 - see `mesh_render::Prepass`.
            hd.is_some(),
            false,
        )?;
        let adverts = if adverts.is_empty() {
            None
        } else {
            let mut cards = crate::adverts::Cards::new(device, queue, adverts, anisotropy)?;
            cards.bind_into(device, queue, &mut track, &placeholders);
            Some(cards)
        };
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
        let weapon_pads = if weapons_on {
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
        let (boost, boost_uv_transforms, flares) = boost_flare::build(
            device,
            queue,
            liveries,
            format,
            anisotropy,
            sample_count,
            scene_depth,
            zone_art,
            shadow_maps,
        )?;
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
        // **`Written` is a reading, not a recovered draw path**, and on Pulse
        // PSP it does not apply: the shell is `Model::stamps_glow` there, so its
        // own batches decide, and a transparent batch without `0xc0` writes no
        // mask - the stencil rule measured on docs/rendering/glow-mask.md.
        // Other sources keep it in the mask, as a shield is one of the frame's
        // brightest things; the shell's own GE state has not been read.
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
        let absorb_overlay = [slots.absorb_overlays()?, slots.leach_overlays()?];
        let shine = slots.shines()?;
        let wrecks = wreck::Wrecks::build(&slots, scene_depth, zone_art)?;
        let shell = |l: &oag_livery::Livery| l.absorb_shell.clone();
        let never = mesh_render::ShadowReceiver::Never;
        let absorb_shell = slots.drawables(shell, exhaust::BLEND, scene_depth, never)?;
        // One per projectile slot - the Rocket's, the Mine's, the Bomb's,
        // the Cannon round's, the Plasma blast's and the Bomb blast's -
        // built the same way, see `weapon_models::build_all`. A laid mine's
        // hull is opaque too, so every one takes the ordinary transparent
        // blend and protected glow mask the ships take.
        let (rockets, mines, bombs, cannon_rounds, plasma_blast, bomb_blast) =
            weapon_models::build_all(
                device,
                queue,
                rocket_model,
                mine_model,
                bomb_model,
                cannon_model,
                plasma_blast_models,
                bomb_blast_models,
                format,
                anisotropy,
                sample_count,
                scene_depth,
                zone_art,
                shadow_maps,
            )?;
        // The LeachBeam's own ball, HD only - one instance, not a pool: unlike
        // the Rocket/Mine/Bomb/Cannon above there is never more than one live
        // beam at once (`oag_gameplay::World::leach_beam` is a single
        // `Option`, not a slotted pool), so this takes the same
        // one-drawable shape `Self::shield_cockpit` does rather than
        // `weapon_models::build`'s per-slot one. Same transparent blend and
        // protected glow mask every other weapon body above takes - see
        // `oag_fx::beam::hd_ball`'s own doc comment for what is measured
        // about this model's placement and what is chosen.
        let leach_ball = match leach_ball_model.filter(|model| !model.indices.is_empty()) {
            Some(model) => Some(Drawable::new(
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
            )?),
            None => None,
        };
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
            !ps2_mask,
        ));
        let sparks = std::cell::RefCell::new(sparks::Pipeline::new(
            device,
            format,
            sample_count,
            mesh_render::Velocity::Write,
        ));
        let weapon_quads =
            weapon_quads::build(device, queue, format, cannon, liveries, sample_count);
        let beam = beam::build(
            device,
            queue,
            format,
            leach_beam_texture.as_ref(),
            sample_count,
        );
        let magstrip = magstrip_wake::build(
            device,
            queue,
            format,
            magstrip_wake_textures.as_ref(),
            sample_count,
        );
        // One silhouette per grid slot, in the same slot order the liveries
        // are in - `race::shadow::silhouettes` built them, and the load report
        // already said which slots got the disc's own image and which got the
        // generated falloff.
        let shadow_silhouette_count = shadows.len();
        let generated_silhouettes = shadows.iter().filter(|s| s.generated).count();
        let shadow = std::cell::RefCell::new(oag_render::shadow::Pipeline::new(
            device,
            queue,
            format,
            &shadows,
            sample_count,
            mesh_render::Velocity::Write,
        ));
        // A failure here is reported and dropped: a race without a bloom is
        // dimmer, not broken. The HD chain replaces this pass outright, and it
        // runs only over a mask stamped as Pulse PSP's is measured to be -
        // Pure's and the PS2's are not (docs/rendering/glow-mask.md).
        let bloom = match (hd.is_none() && omega.is_none() && measured_mask && !ps2_mask)
            .then(|| oag_post::bloom::Bloom::new(device, format))
            .transpose()
        {
            Ok(bloom) => bloom,
            Err(e) => {
                warn!("bloom unavailable ({e}) - the frame draws without it");
                None
            }
        };
        let ps2_bloom = match (hd.is_none() && omega.is_none() && ps2_mask)
            .then(|| oag_post::ps2_bloom::Ps2Bloom::new(device, format))
            .transpose()
        {
            Ok(bloom) => bloom,
            Err(e) => {
                warn!("PS2 bloom unavailable ({e}) - the frame draws without it");
                None
            }
        };

        // Built against the caller's own format: whichever chain runs, the
        // frame this pass reads is the one already encoded into the caller's
        // view - after the HD chain's encode, after the PSP bloom's
        // composite. A failure is reported and dropped the way the bloom's
        // is: a race without motion blur is a sharper race, not a broken one.
        // See `Self::motion_blur` for why this is not gated on the setting.
        let motion_blur = match oag_post::motion_blur::MotionBlur::new(device, caller_format) {
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
        let (shader_calls, shader_hits) = cache_scope.shader_counts();
        let (texture_calls, texture_hits) = cache_scope.texture_counts();
        let build_cache = cache_scope.pipeline_counts();
        debug!(
            "race scene build cache: shader {shader_hits}/{shader_calls} reused, pipeline \
             {}/{} reused ({} distinct built), texture upload {texture_hits}/{texture_calls} reused",
            build_cache.1, build_cache.0, build_cache.2
        );
        Ok(Self {
            build_cache,
            bloom,
            ps2_bloom,
            bloom_pending: std::cell::Cell::new(false),
            hd,
            omega,
            motion_blur,
            blur_half: std::cell::Cell::new(false),
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
            shine,
            wrecks,
            track_shine,
            anim_clock: std::cell::Cell::new(0.0),
            absorb_shell,
            rockets,
            mines,
            bombs,
            cannon_rounds,
            plasma_blast,
            bomb_blast,
            leach_ball,
            boost_uv_transforms,
            collision,
            sky,
            pads,
            weapon_pads,
            gantry,
            adverts,
            behind_glass,
            fog_volumes,
            light,
            authored_fog,
            zone_grade,
            exhaust,
            weapon_quads,
            clouds: clouds::Clouds::build(device, queue, format, sample_count, cloud_layer),
            mist: std::cell::RefCell::new(None),
            beam,
            magstrip,
            sparks,
            shadow,
            shadow_map: std::cell::RefCell::new(shadow_map),
            generated_silhouettes,
            shadow_silhouette_count,
            generated_silhouettes_said: std::cell::Cell::new(false),
            sun_occlusion: std::cell::RefCell::new(sun_occlusion),
            shadow_hulls,
            scratch: std::cell::RefCell::default(),
            opaque_order: std::cell::RefCell::default(),
            depth,
            msaa_color,
            attachment_views,
            msaa,
            far,
            ghost: ghost::Ghosts::default(),
        })
    }
}
