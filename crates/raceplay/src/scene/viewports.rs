//! [`Scene::render_viewports`]: split screen, one scene draw per local player.
//!
//! One target, `N` rectangles of it ([`oag_render::viewport::layout`]), each drawn
//! by an ordinary [`Scene::render`] from its own camera
//! ([`Race::set_viewpoint`]). Five things make that more than a loop, and each
//! is a way to get a plausible picture that is wrong:
//!
//! - **One encoder and one submit per view.** `Scene::render` writes every
//!   drawable's uniforms with `queue.write_buffer`, and a queue write lands
//!   before the *submission's* commands, not at its place in the encoder. Two
//!   views recorded into one encoder would both draw with the last view's
//!   matrices: every rectangle the same camera. A batched [`super::Recording`] keeps
//!   that order by keeping a later view's writes in a log of its own and replaying
//!   it just before that view's submission (`split_record`).
//! - **Only the first view clears.** The scene pass clears the whole attachment
//!   whatever the viewport is, so the later views load what is there
//!   ([`Layer::Next`]). Depth is cleared each time; the views do not
//!   overlap and nothing reads one view's depth in another. Under MSAA the
//!   resolve rewrites the whole target from the multisampled attachment, which
//!   is why the colour attachment's load is switched, not only the resolve's.
//! - **What does not depend on the camera is done once, by the first view.** The
//!   two shadow maps, the shadow geometry's gather (`Scene::upload_shadow_geometry`
//!   keeps the lists; each view uploads them with its own matrix), the scenery's
//!   animation clocks, the advert cards and the craft shine passes' environment-mapped
//!   vertices (`Scene::write_shine`) are functions of the craft and the race
//!   clock, so a later view ([`Layer::Next`]) reads what the first left in the
//!   buffers and textures. Measured on Pulse, the gather alone was half of a view's
//!   CPU, and a split is CPU-bound (`docs/rendering/split-screen.md`, "Cost"). The
//!   picture of every view is byte-identical to drawing it all per view. **One
//!   exception: the `mapped` tier's depth map is fitted on the craft its view
//!   follows, so each later view redraws it** in its own encoder
//!   (`Scene::render_shadow_passes`).
//! - **Pulse's bloom runs once per view, after the last one.** A view's composite
//!   cannot land on the resolved target as the view is drawn, because the next
//!   view's multisample resolve rewrites the whole target from the multisampled
//!   colour. So every view is drawn first, and then [`Scene::prepare_split_bloom`]
//!   reads each view's own rectangle of the finished picture (`oag_post::bloom`
//!   honours the frame's origin since 2026-10-15; it read from the target's
//!   top-left before, so a view anywhere else glowed from the first view's pixels)
//!   into a scratch lane of its own. [`Scene::composite_bloom`] adds each back after
//!   the HUD, as a single view's is. **Chosen, not measured**: each lane's size
//!   keeps the glow's reach in proportion to the view (`oag_post::bloom::lane_extent`).
//! - **Motion blur runs once per view, after the last one** (`split_blur`). Each view
//!   measures its velocity against a camera history of its own ([`motion::split_history`]),
//!   writes it into its rectangle of the velocity and depth attachments (a later view
//!   *loads* them, as it loads the colour), and [`Scene::blur_split_views`] then runs the
//!   blur over each rectangle, one submission each. **Camera jitter and the temporal
//!   upscaler per view too** (2026-10-16): each view jitters on its seat's own sequence and
//!   records its frame in its seat's history, and [`Scene::temporal`] answers a frame per view
//!   so the upscaler resolves each with a chain of its own; the window's own record is kept
//!   cleared. **Chosen, not measured.**
//! - **HD's and Omega's post chains run once per view, after the last one**
//!   ([`Scene::run_split_post`], `split_exposure`). Both draw the scene into a linear
//!   float target and then encode it into `view` with a pass that *clears* `view`
//!   (HD's). Run as each view drew, each encode wiped every earlier view (a black
//!   top view in a two-player HD race) and every later view was graded from the first
//!   view's pixels. The views now only fill the linear target, and the chain then
//!   runs over each view's own rectangle of it, with an adaptation per seat: a view in
//!   a dark tunnel and one in daylight each have their own exposure, and HD's glow
//!   stops at the view's edge. **Chosen, not measured.** The boost zoom streak (HD's own ring) is not drawn under a
//!   split and Omega's distortion offsets are not drawn: the first holds the
//!   primary craft's pulse; the second reads the depth attachment, which since the blur
//!   (a later view loads it) holds every view's and could be wired. Both open.

use super::*;

/// Which split-screen view a [`Scene::render`] call is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Layer {
    /// No split: the one view, clearing the target and blooming as always.
    Whole,
    /// The first view of a split: clears the target.
    First,
    /// A later view: loads what the earlier ones drew.
    Next,
    /// A view that fills a target of its own (a window): clears like [`Layer::First`] and
    /// does all the camera-independent work itself, but runs the post chains as
    /// [`Layer::Whole`] does, because it starts at the origin and fills the target.
    Window,
}

/// What a window draws on top of the scene itself, read fresh off the settings each
/// frame as [`Scene::render`]'s arguments are: the motion blur, the camera jitter a temporal
/// upscaler wants and the timestamp pairs a dynamic-resolution timer is read from. A one-seat
/// window uses all of them on the window's own history ([`Scene::render_window`]). A split
/// (several views in one target) uses the blur and its timestamp pair only, once per view
/// ([`Scene::blur_split_views`]); the jitter and the other timers are one camera's.
#[derive(Default)]
pub struct WindowEffects<'a> {
    /// The blur's strength; `Off` encodes nothing.
    pub motion_blur: oag_display::display::MotionBlur,
    /// The jitter sequence's length, `None` for no jitter. See [`Scene::render`].
    pub camera_jitter: Option<u32>,
    /// Brackets the window's scene pass.
    pub timestamps: Option<wgpu::RenderPassTimestampWrites<'a>>,
    /// Brackets the motion-blur chain.
    pub blur_timestamps: Option<oag_post::motion_blur::ChainTimestamps<'a>>,
    /// Brackets HD's bloom chain.
    pub hd_bloom_timestamps: Option<oag_post::hd_bloom::ChainTimestamps<'a>>,
}

impl std::fmt::Debug for WindowEffects<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowEffects")
            .field("motion_blur", &self.motion_blur)
            .field("camera_jitter", &self.camera_jitter)
            .finish_non_exhaustive()
    }
}

impl Scene {
    /// Whether the view being drawn does the frame's camera-independent work itself: the two
    /// shadow maps, the shadow gather, the scenery clocks and the advert cards. **Not a later
    /// view of a split** ([`Layer::Next`]), and not a window drawing after another window of
    /// the same frame ([`Scene::render_window`]'s `shared`): both read what the view that
    /// did it left in the buffers and textures.
    pub(super) fn does_frame_work(&self) -> bool {
        self.layer.get() != Layer::Next && !self.reuse_frame_work.get()
    }

    /// Draws slot `slot`'s view into the whole of `area` of `view`, recorded into
    /// `encoder`: one window of a multi-window race. The caller submits `encoder`
    /// before it draws another window, **one submission per window for the whole
    /// chain** (scene, resolve, HUD, composite), for the reason the module doc gives.
    ///
    /// Everything [`Scene::render`] draws is drawn: the bloom, HD's and Omega's chains,
    /// the shadows, and what `effects` asks for (the motion blur and the camera jitter).
    /// **Each window has a camera history of its own** (`window` picks it,
    /// [`Scene::history`]): the previous tick's matrices the blur measures against, the
    /// jitter phase and the temporal upscaler's frame record, and the cut watch, so the
    /// windows alternating cameras within a frame never read each other's. The viewpoint is
    /// put back to the player's own before returning.
    #[allow(clippy::too_many_arguments)]
    pub fn render_view(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        race: &Race,
        window: usize,
        slot: usize,
        area: (f32, f32, f32, f32),
        fov: oag_display::display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
        shadows: oag_display::display::Shadows,
        zone_spectrum: &[f32],
        shared: bool,
        effects: WindowEffects<'_>,
    ) -> SceneStats {
        race.set_viewpoint(slot);
        self.layer.set(Layer::Window);
        self.reuse_frame_work.set(shared);
        self.begin_split_frame(shared);
        self.note_split_views(window, &[]);
        self.history.set(window);
        let stats = self.render(
            device,
            queue,
            encoder,
            view,
            race,
            area,
            fov,
            cull,
            pvs_cull,
            anim_seconds,
            effects.motion_blur,
            shadows,
            effects.camera_jitter,
            zone_spectrum,
            effects.timestamps,
            effects.blur_timestamps,
            effects.hd_bloom_timestamps,
        );
        self.layer.set(Layer::Whole);
        self.reuse_frame_work.set(false);
        self.history.set(0);
        race.reset_viewpoint();
        stats
    }

    /// Draws `race` into the rectangles `race.views_drawn()` cuts `area` into,
    /// and returns the sum of what each drew. **That is one view once the race is
    /// over**: the end screens sit over the spectator camera, and it is drawn
    /// here, with no blur, jitter or upscaler history, rather than by
    /// [`Scene::render`], whose history this path never wrote. Submits its own command buffers,
    /// so everything the caller has already encoded must be submitted before
    /// this, and the caller's own encoder after it.
    ///
    /// `view_timers` brackets each view's scene pass, one claimed timer per view
    /// (the caller `begin`s them); empty everywhere but the render bench, which
    /// is the only caller that reads them. A view past the slice's end is untimed.
    ///
    /// For a race that was never split ([`Race::viewport_count`] of one) this is
    /// not the call to make: use [`Scene::render`], which is what keeps a race
    /// without a split exactly as it was.
    #[allow(clippy::too_many_arguments)]
    pub fn render_viewports(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        race: &Race,
        area: (f32, f32, f32, f32),
        fov: oag_display::display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
        shadows: oag_display::display::Shadows,
        zone_spectrum: &[f32],
        effects: WindowEffects<'_>,
        view_timers: &mut [Option<oag_gpu::timing::PassTimer>],
    ) -> SceneStats {
        let rects = race.view_rects(area);
        let slots: Vec<usize> = (0..rects.len()).collect();
        self.render_slots(
            device,
            queue,
            view,
            race,
            &slots,
            &rects,
            area,
            fov,
            cull,
            pvs_cull,
            anim_seconds,
            shadows,
            zone_spectrum,
            false,
            0,
            effects,
            view_timers,
        )
    }

    /// Window `window` of a multi-window race, whatever it holds: nothing for a window with
    /// no seat, [`Scene::render_view`] for one seat (recorded into `encoder`, which the caller
    /// submits before it draws another window) and [`Scene::render_window_views`] for several
    /// (which submits its own buffers, so `encoder` keeps only what the caller records after
    /// it). The one place that choice is made; the window's frame loop and the headless
    /// capture both draw through it.
    ///
    /// `effects` is the window's blur, jitter and timers; a window of several seats is a
    /// split and draws the blur once per view ([`Scene::blur_split_views`]) and the jitter on
    /// each seat's own sequence (**chosen, not measured**). `view_timers` brackets each of
    /// the window's seats' scene passes, as [`Scene::render_viewports`]'s does: empty
    /// everywhere but the render bench, where it stands in for `effects.timestamps`.
    ///
    /// **`shared` is whether an earlier window of this same frame already drew**, and with
    /// it did the camera-independent work (the shadow maps and their gather, the scenery
    /// clocks, the advert cards): this window reads what that one left, as a later view of
    /// a split does, so a frame does it once however many windows draw. The earlier window's
    /// encoder must be submitted before this window's (the maps are textures it wrote). The
    /// `mapped` tier's depth map is fitted on the craft each view follows, so every window
    /// still redraws that one. The caller says it: `false` for the first window that draws a
    /// frame (window 1 can draw nothing, and then window 2 is the first).
    #[allow(clippy::too_many_arguments)]
    pub fn render_window(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        race: &Race,
        window: usize,
        area: (f32, f32, f32, f32),
        fov: oag_display::display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
        shadows: oag_display::display::Shadows,
        zone_spectrum: &[f32],
        shared: bool,
        effects: WindowEffects<'_>,
        view_timers: &mut [Option<oag_gpu::timing::PassTimer>],
    ) -> SceneStats {
        match race.window_drawn(window).as_slice() {
            [] => SceneStats::default(),
            &[slot] => {
                let timer = view_timers.first_mut().and_then(Option::as_mut);
                let effects = WindowEffects {
                    timestamps: effects.timestamps.or_else(|| {
                        timer
                            .as_deref()
                            .and_then(oag_gpu::timing::PassTimer::writes)
                    }),
                    ..effects
                };
                let stats = self.render_view(
                    device,
                    queue,
                    encoder,
                    view,
                    race,
                    window,
                    slot,
                    area,
                    fov,
                    cull,
                    pvs_cull,
                    anim_seconds,
                    shadows,
                    zone_spectrum,
                    shared,
                    effects,
                );
                if let Some(timer) = timer {
                    timer.resolve(encoder);
                }
                stats
            }
            _ => self.render_window_views(
                device,
                queue,
                view,
                race,
                window,
                area,
                fov,
                cull,
                pvs_cull,
                anim_seconds,
                shadows,
                zone_spectrum,
                shared,
                effects,
                view_timers,
            ),
        }
    }

    /// Draws window `window`'s views ([`Race::window_drawn`]) into the rectangles its cut
    /// divides `area` into ([`Race::window_rects`]): a window that holds several seats, cut
    /// up the way a split screen cuts its views. The same sequence as
    /// [`Scene::render_viewports`] - it submits its own command buffers, so the caller's
    /// encoder goes after it - for the slots the window holds instead of `0..n`. A window
    /// of one seat is [`Scene::render_view`] instead.
    #[allow(clippy::too_many_arguments)]
    pub fn render_window_views(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        race: &Race,
        window: usize,
        area: (f32, f32, f32, f32),
        fov: oag_display::display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
        shadows: oag_display::display::Shadows,
        zone_spectrum: &[f32],
        shared: bool,
        effects: WindowEffects<'_>,
        view_timers: &mut [Option<oag_gpu::timing::PassTimer>],
    ) -> SceneStats {
        let slots = race.window_drawn(window);
        let rects = race.window_rects(window, area);
        self.render_slots(
            device,
            queue,
            view,
            race,
            &slots,
            &rects,
            area,
            fov,
            cull,
            pvs_cull,
            anim_seconds,
            shadows,
            zone_spectrum,
            shared,
            window,
            effects,
            view_timers,
        )
    }

    /// `slots[i]`'s view drawn into `rects[i]` of `area`, then the post chains once per view: what [`Scene::render_viewports`] and [`Scene::render_window_views`] are.
    #[allow(clippy::too_many_arguments)]
    fn render_slots(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        race: &Race,
        slots: &[usize],
        rects: &[(f32, f32, f32, f32)],
        area: (f32, f32, f32, f32),
        fov: oag_display::display::Fov,
        cull: bool,
        pvs_cull: bool,
        anim_seconds: Option<f32>,
        shadows: oag_display::display::Shadows,
        zone_spectrum: &[f32],
        shared: bool,
        window: usize,
        effects: WindowEffects<'_>,
        view_timers: &mut [Option<oag_gpu::timing::PassTimer>],
    ) -> SceneStats {
        let mut stats = SceneStats::default();
        self.reuse_frame_work.set(shared);
        // Each view jitters and records its frame in its seat's history, and the upscaler
        // resolves each view from that (`Scene::temporal`); the window's own record is not
        // one of them, so it is cleared.
        self.histories[window.min(motion::MAX_HISTORIES - 1)]
            .last_frame
            .set(None);
        let noted: Vec<motion::SplitView> = slots
            .iter()
            .zip(rects)
            .map(|(&slot, rect)| {
                (
                    slot,
                    (rect.0 as u32, rect.1 as u32, rect.2 as u32, rect.3 as u32),
                )
            })
            .collect();
        self.note_split_views(window, &noted);
        self.split_live.set(true);
        self.begin_split_frame(shared);
        for &slot in slots {
            self.keep_split_seat(slot);
        }
        let recording = self.split_recording();
        if recording == super::Recording::Interleaved {
            for (index, (&slot, &rect)) in slots.iter().zip(rects).enumerate() {
                self.enter_view(race, index, slot);
                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("race viewport"),
                });
                let timer = view_timers.get_mut(index).and_then(Option::as_mut);
                stats.add(
                    self.render(
                        device,
                        queue,
                        &mut encoder,
                        view,
                        race,
                        rect,
                        fov,
                        cull,
                        pvs_cull,
                        anim_seconds,
                        // The blur runs after the last view: `Scene::blur_split_views`.
                        oag_display::display::MotionBlur::Off,
                        shadows,
                        effects.camera_jitter,
                        zone_spectrum,
                        timer
                            .as_deref()
                            .and_then(oag_gpu::timing::PassTimer::writes),
                        None,
                        None,
                    ),
                );
                if let Some(timer) = timer {
                    timer.resolve(&mut encoder);
                }
                let commands = encoder.finish();
                oag_gpu::perfprobe::mark("encoder-finish");
                queue.submit(Some(commands));
                oag_gpu::perfprobe::mark("submit");
            }
        } else {
            let settings = super::frame::ViewSettings {
                fov,
                cull,
                pvs_cull,
                anim_seconds,
                shadows,
                camera_jitter: effects.camera_jitter,
                zone_spectrum,
            };
            stats = self.render_slots_batched(
                device,
                queue,
                view,
                race,
                slots,
                rects,
                &settings,
                view_timers,
                recording,
            );
        }
        self.layer.set(Layer::Whole);
        self.reuse_frame_work.set(false);
        self.history.set(0);
        race.reset_viewpoint();
        // HD's and Omega's chains, one run per view with its own adaptation
        // (`split_exposure`).
        let post_ran = self.run_split_post(device, queue, view, slots, rects, area);
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("race viewports post"),
        });
        // Pulse's glow, one lane per view, read from the finished picture: every view is in
        // `view` now, and the composite waits for the HUD as a single view's does.
        let glow = self.prepare_split_bloom(device, queue, &mut encoder, view, rects, area);
        if glow {
            queue.submit(Some(encoder.finish()));
        }
        stats.hd_bloom_encoded = post_ran && self.hd.is_some();
        // Motion blur, last, as a single view's is: after the glow's bright pass (which the
        // composite adds back over the blurred frame after the HUD) and after HD's and
        // Omega's chain, before the HUD. See `split_blur`.
        stats.blur_encoded = self.blur_split_views(
            device,
            queue,
            view,
            slots,
            rects,
            effects.motion_blur.shutter(),
            effects.blur_timestamps,
        );
        stats
    }
}
