//! [`Scene`]: the GPU-side composition of a race - the pipelines, the textures
//! and the per-slot drawables one frame is assembled from.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Drawing a
//! frame with it is `race/frame.rs`; its tests are `race/tests/scene.rs`.

use super::*;

mod frame;

use frame::{depth_texture, msaa_color_texture};

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
    /// See [`oag_formats::envsettings`].
    light: mesh_render::Light,
    /// The track's authored fog volumes, sampled at the camera each frame.
    ///
    /// Empty for a track that authors no `fogCube` - four of the forty - and the
    /// race then renders unfogged, which is what the original does too.
    fog_volumes: Vec<oag_formats::fog::FogVolume>,
    /// Wipeout HD's authored distance fog, static for the whole race.
    ///
    /// What binds when no `fogCube` volume covers the camera - which on HD is
    /// always, since HD authors no `fogCube` at all. See
    /// [`crate::race::Loaded::authored_fog`].
    authored_fog: Option<mesh_render::Fog>,
    /// The track's `Skycube`, drawn camera-centred before anything else.
    ///
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
    /// One drawable per projectile slot, for rockets drawn as their own model.
    ///
    /// **Empty** when `Data\Weapons\Rocket.vex` did not load, and the sprite
    /// fallback in [`Race::projectile_sprites`] carries the whole effect then.
    /// Sized to `MAX_PROJECTILES` and clone-per-slot for the same reason
    /// [`Self::ships`] is clone-per-craft: a `Drawable` owns the uniform buffer
    /// its model matrix goes in, and three rockets in the air at once need three
    /// matrices. Eighty-four vertices apiece makes sixteen copies cheap.
    rockets: Vec<Drawable>,
    /// Each slot's own plume's authored texture-transform keyframes, sampled
    /// per frame and applied to that plume's authored UVs - the recovered
    /// mechanism (`TEXMAPMODE` 0 plus the animated `TEXOFFSET` u-scroll; see
    /// `crate::livery::Livery::boost_uv`). `None` falls back to the engine's
    /// own identity default.
    ///
    /// **Per slot, because the plumes are.** Every team read so far carries
    /// the identical track, so sharing one would be invisible today and wrong
    /// the moment a team did not.
    boost_uv_transforms: Vec<Option<oag_formats::vex::TexTransform>>,
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
    /// Collision sparks. `RefCell` for the same reason [`Self::exhaust`] is.
    sparks: std::cell::RefCell<sparks::Pipeline>,
    /// The recovered bloom, run after the scene pass over whatever the frame
    /// stamped into its alpha channel. `None` when the pipelines would not
    /// build, which costs the glow and nothing else.
    ///
    /// **Not exhaust-specific**, even though the exhaust is currently its only
    /// writer: it blooms the glow mask, and any surface that opts into the mask
    /// is handled by the same three passes. See `oag_render::post::bloom`.
    bloom: Option<oag_render::post::bloom::Bloom>,
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
    /// [`crate::display::AntiAliasing::msaa_samples`].
    msaa_color: Option<wgpu::Texture>,
    /// What this scene's pipelines were actually built with, for the
    /// GRAPHICS menu's restart note - see `Session::open_menus` in `main.rs`.
    anti_aliasing: crate::display::AntiAliasing,
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
        mode: Mode,
        rocket_model: Option<Model>,
        flare: Option<FlareTexture>,
        noise: Option<FlareTexture>,
        format: wgpu::TextureFormat,
        size: (u32, u32),
        anisotropy: Anisotropy,
        bloom_enabled: bool,
        visibility: Option<TrackVisibility>,
        anti_aliasing: crate::display::AntiAliasing,
        fog_volumes: Vec<oag_formats::fog::FogVolume>,
        light: mesh_render::Light,
        authored_fog: Option<mesh_render::Fog>,
    ) -> Result<Self> {
        // The far plane comes from the track's own bounding sphere: a track is
        // hundreds of units across, and a fixed guess would either clip it away or
        // waste the depth range on empty space.
        let far = track_model.radius * 4.0;
        let sample_count = anti_aliasing.msaa_samples();
        let scene_depth = mesh_render::Depth::Scene;
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
        for slot in 0..GRID_SLOTS as usize {
            let livery = &liveries[slot.min(liveries.len().saturating_sub(1))];
            boost_uv_transforms.push(livery.boost_uv.clone());
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
                )?));
            }
        }
        // One per projectile slot, cloned the way the hulls and plumes above are.
        // A rocket's hull is opaque - it is a painted dart, not a glow - so this
        // takes the ordinary transparent blend and the protected glow mask the
        // ships take, and the flare around it stays in the additive pass with
        // the exhaust where it belongs.
        let mut rockets = Vec::new();
        if let Some(model) = rocket_model.filter(|model| !model.indices.is_empty()) {
            for _ in 0..oag_gameplay::projectile::MAX_PROJECTILES {
                rockets.push(Drawable::new(
                    device,
                    queue,
                    model.clone(),
                    format,
                    anisotropy,
                    sample_count,
                    scene_depth,
                    mesh_render::TRANSPARENT_BLEND,
                    mesh_render::GlowMask::Protected,
                )?);
            }
        }
        // 64 is a stand-in size only, and only when the disc's own texture did not
        // decode; `load` has already reported that when it happens.
        let flare = flare.unwrap_or_else(|| FlareTexture::placeholder(64));
        let noise = noise.unwrap_or_else(|| FlareTexture::placeholder(64));
        let exhaust = std::cell::RefCell::new(exhaust::Pipeline::new(
            device,
            queue,
            format,
            &flare,
            &noise,
            sample_count,
        ));
        let sparks = std::cell::RefCell::new(sparks::Pipeline::new(device, format, sample_count));
        // A failure here is reported and dropped rather than propagated: a race
        // without a bloom is a dimmer race, not a broken one.
        let bloom = match bloom_enabled
            .then(|| oag_render::post::bloom::Bloom::new(device, format))
            .transpose()
        {
            Ok(bloom) => bloom,
            Err(e) => {
                eprintln!("bloom unavailable ({e}) - the frame draws without it");
                None
            }
        };

        Ok(Self {
            bloom,
            track,
            visibility,
            ships,
            boost,
            rockets,
            boost_uv_transforms,
            collision,
            sky,
            pads,
            weapon_pads,
            fog_volumes,
            light,
            authored_fog,
            exhaust,
            sparks,
            depth: depth_texture(device, size, sample_count),
            msaa_color: msaa_color_texture(device, format, size, sample_count),
            anti_aliasing,
            far,
        })
    }

    /// Rebuilds the depth buffer, and the MSAA colour target if there is one,
    /// for a new viewport size.
    ///
    /// A colour or depth attachment whose size does not match the others is a
    /// validation error, so this is not optional on resize.
    pub fn resize(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat, size: (u32, u32)) {
        let sample_count = self.anti_aliasing.msaa_samples();
        self.depth = depth_texture(device, size, sample_count);
        self.msaa_color = msaa_color_texture(device, format, size, sample_count);
    }

    /// What this scene's pipelines were actually built with, for the restart
    /// note - see [`Self::msaa_color`].
    #[must_use]
    pub fn anti_aliasing(&self) -> crate::display::AntiAliasing {
        self.anti_aliasing
    }
}
