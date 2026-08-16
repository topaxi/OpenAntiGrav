//! The menu stage, and the looping picture drawn behind it.

use anyhow::{Result, bail};

use oag_game::keys;
use oag_game::render::Renderer;
use oag_game::{menu, movie};

use crate::gpu::Gpu;

/// The front end, and everything only it needs.
/// The menus, and a renderer of their own.
///
/// Built from the same font atlas and sprite sheet the front end draws with,
/// rather than taken over from it: the menus have to be openable from somewhere
/// that is not the front end - a pause menu, eventually - and a stage that can
/// only exist downstream of another one cannot be. It costs one pipeline build
/// at a moment already spent loading.
pub(crate) struct MenuStage {
    pub(crate) renderer: Renderer,
    pub(crate) menu: menu::Menu,
    /// Where every row goes and what colour it is. Built once, when the menus
    /// open, from the title's own table and the line height of the face that
    /// will draw the rows - the two halves `menu::Skin` exists to join.
    pub(crate) skin: menu::Skin,
    /// The page change in flight, if one is.
    ///
    /// **The page the player left is kept as a finished draw list, not as a
    /// `Menu` to re-draw.** The transition only ever scales and fades what was
    /// already on screen, so a snapshot is both cheaper and more honest than a
    /// second model that would keep answering input.
    pub(crate) change: Option<PageChange>,
    /// Where the disc's looping backdrop has got to, and where it goes on
    /// screen. `None` when this source has no backdrop, and then the rows are
    /// drawn on black exactly as they were before it existed.
    ///
    /// The player lives here rather than in `Session` because it is part of what
    /// is on screen: a stage that is not running should not be advancing a
    /// movie, and putting it here makes that structural instead of remembered.
    /// **It is usually not this stage's own player**: coming out of the boot
    /// sequence it is the one `Show Logo` was running, moved across rather than
    /// rebuilt, so the loop never restarts on the handoff. See
    /// [`menu_playhead`].
    pub(crate) backdrop: Option<Backdrop>,
}

/// A page change part-way through.
///
/// Held for as long as the tween runs and then dropped. See
/// [`menu::Layers::zoomed`] for the effect and `docs/ui/menus-original.md` for
/// the capture it came off.
pub(crate) struct PageChange {
    /// The page being left, as it looked on its last frame.
    leaving: menu::Layers,
    /// The clock both halves share.
    tween: oag_game::anim::Tween,
    /// How far each half travels.
    shape: menu::Transition,
}

/// The looping menu picture: a player, where it goes, and which frame is up.
pub(crate) struct Backdrop {
    pub(crate) player: movie::Player,
    pub(crate) rect: [f32; 4],
    /// Which frame is **actually in the renderer's planes**, or `None` while none
    /// is.
    ///
    /// Not "which frame we would like": the decode happens on another thread, so
    /// there can genuinely be no picture yet, and `None` is what stops the video
    /// quad being drawn over zeroed planes - which is a green rectangle, not a
    /// black one. It is also what the draw list reports, so the list names the
    /// frame on screen rather than one that may not have arrived. See
    /// [`MenuStage::render`].
    ///
    /// **Coming out of the boot sequence this starts `Some`**, seeded by
    /// `Session::open_menus` from the picture the front end had on screen. It
    /// used to start `None` on every path, and the one to three frames of
    /// menu-on-black that produced were the flicker on the START press. It still
    /// starts `None` on the `escape` -> menus path, which carries no playhead and
    /// no picture and so shows black until the restarted feed produces frame 0 -
    /// see [`menu_playhead`], which is where that whole path's judgement call is
    /// argued.
    pub(crate) shown: Option<usize>,
}

/// The playhead the menus open on: the one already running, when there is one.
///
/// # Coming out of the boot sequence, there always is
///
/// `Show Logo` and the menus are two screens in front of **one** playback of
/// `Data\Movies\Backdrop`, not two playbacks of it: `Show Logo` is a child of
/// the `FE Screen` that owns the movie, so on hardware pressing START changes
/// which widgets are drawn over a loop that never stops. Building a
/// [`movie::Player`] here instead put the picture back at frame zero on that
/// press, which is the jump a player who has run the original reported. So the
/// front end hands its playhead over - see
/// [`frontend::Frontend::take_backdrop`] - and this passes it straight through,
/// unmodified and un-rewound. There is one [`movie::Feed`] for the whole
/// session already, so nothing else has to move.
///
/// # Leaving a race is the other way in, and it does start over
///
/// A race replaces the menu stage, and its playhead goes with it: nothing
/// advances the backdrop while a race is on screen, and nothing takes frames
/// out of the feed either, so both stop where the menus left them. Rather than
/// resume mid-loop from a stage that no longer exists, `escape` gets a fresh
/// playhead and `Session::open_menus` restarts the feed to match - which is
/// what the original does too, `FE Screen` being torn down for a race and
/// rebuilt after it, with its `autostart` movie starting again. It is also the
/// cheap end of the trade: one decoder flush against carrying a position
/// through a stage that has no use for it.
///
/// **This is a judgement call and the docs do not settle it.** What is
/// confirmed against the original is the boot handoff above; nobody has
/// measured the loop's phase across a race. If it turns out to continue there
/// too, the change is to stash the playhead on [`Session`] when a race starts
/// and pass it back in here - the feed needs no restart for that, because it
/// parks at most four frames past where the menus stopped taking from it.
pub(crate) fn menu_playhead(
    carried: Option<movie::Player>,
    frames: usize,
    frame_rate: (u64, u64),
) -> movie::Player {
    carried.unwrap_or_else(|| {
        // `repeat`, which is the whole difference between this movie and the
        // intro: `FE Screen` sits under it for as long as a player is in the
        // menus, so it wraps rather than finishing on its last frame. See
        // `movie::Player`.
        movie::Player::new(frames, true, frame_rate)
    })
}

impl MenuStage {
    /// Advances the backdrop by one tick.
    ///
    /// Driven from the same fixed `dt` the simulation is, and for the same
    /// reason the front end's movie is: nothing here reads the wall clock. The
    /// player wraps rather than finishing - it is a loop, which is the whole
    /// difference between this movie and the intro.
    pub(crate) fn tick(&mut self, dt: f64) {
        if let Some(backdrop) = &mut self.backdrop {
            backdrop.player.update(dt);
        }
        // The same fixed `dt` the backdrop is stepped with, and for the same
        // reason: nothing on this stage reads the wall clock, so two runs of
        // the same `--ticks` produce the same picture. See `oag_game::anim`.
        if let Some(change) = &mut self.change {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a tick is milliseconds; f32 holds it exactly"
            )]
            change.tween.advance(dt as f32);
            if change.tween.done() {
                self.change = None;
            }
        }
    }

    /// Starts a page change, snapshotting the page being left.
    ///
    /// Called after the model has already moved, so `leaving` is passed in
    /// rather than drawn here - the page it holds no longer exists as far as
    /// the model is concerned.
    ///
    /// A transition already in flight is **replaced**, not queued: a player
    /// holding a direction moves faster than half a second a page, and queueing
    /// would run the menus behind the input by however long the player kept
    /// going.
    pub(crate) fn begin_change(&mut self, leaving: menu::Layers) {
        self.change = Some(PageChange {
            leaving,
            tween: oag_game::anim::Tween::new(self.skin.transition_secs()),
            shape: menu::Transition::default(),
        });
    }

    /// Draws the page, uploading whatever backdrop frame the decode thread has
    /// ready.
    ///
    /// # The decode used to be on this thread, and it cost up to 30 ms a frame
    ///
    /// **Measured on the PSP backdrop, release build: `FrameStore::read_frame`
    /// took 0.03 ms to 30 ms for one frame**, depending on where in the movie the
    /// loop had got to - about 5 ms through the quiet half, 12-30 ms through the
    /// busy one, with every other call nearly free because the decoder's frame
    /// delay is 1. The upload was 0.06 ms and never the problem. At 30 frame
    /// changes a second that was **roughly a quarter of every second spent
    /// decoding on the thread that also draws**, which capped a 240-limited loop
    /// near 180 and, unlimited, hid inside an average that looked fine while
    /// individual frames were tens of milliseconds long.
    ///
    /// It is now a [`movie::Feed`]: a worker thread decodes ahead into a small
    /// ring and this asks for the newest frame at or before the playhead. What is
    /// left on this thread is the upload, and only on a frame that changed. See
    /// [ADR-0010](../../docs/architecture/adr/0010-movie-decode-thread.md).
    ///
    /// # A frame that has not arrived is not a frame
    ///
    /// `take_upto` returning `None` is ordinary and means *keep what is on
    /// screen*. It happens for the first frame or two after the menus open, and
    /// it would happen again if the worker ever fell behind. Two rules follow,
    /// and both are load-bearing rather than defensive:
    ///
    /// - **Nothing is uploaded**, so the planes keep the last good picture. A
    ///   frame held one frame longer is invisible at 30 Hz.
    /// - **With no picture at all, the video draw is left out entirely.** Zeroed
    ///   I420 planes are not black - `Y=0, U=0, V=0` is green - so drawing the
    ///   quad before the first frame lands would flash green over the menu. The
    ///   rows draw on black instead, exactly as they do on a source with no
    ///   backdrop.
    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        feed: Option<&mut movie::Feed>,
    ) -> Result<()> {
        let shown = match (&mut self.backdrop, feed) {
            (Some(backdrop), Some(feed)) => {
                // Surfaced before the frame it would have been, and once: the
                // feed hands a decode failure over exactly one time.
                if let Some(reason) = feed.take_error() {
                    bail!("decoding the menu backdrop: {reason}");
                }
                if let Some(frame) = feed.take_upto(backdrop.player.position()) {
                    self.renderer.upload_frame(&gpu.queue, &frame.picture)?;
                    backdrop.shown = Some(frame.index);
                }
                // The frame on screen, not the one the playhead names.
                backdrop.shown.map(|frame| menu::Backdrop {
                    rect: backdrop.rect,
                    frame,
                    position: backdrop.player.position(),
                })
            }
            // A backdrop whose frames could not be opened is no backdrop: the
            // planes hold nothing, and drawing them would be a green rectangle
            // over the menu rather than a missing picture.
            _ => None,
        };
        let arriving = menu::draw_list(&self.menu, &self.skin, &keys::bound_keys, shown);
        let list = match &self.change {
            // The page being left grows and fades out; the one arriving grows
            // into place from smaller and fades in. Both run off one tween, so
            // they cannot drift apart. The backdrop is drawn once, by the page
            // arriving, because it is the same looping movie either way.
            Some(change) => {
                let t = change.tween.eased();
                let shape = &change.shape;
                let going = change.leaving.clone().zoomed(
                    shape.origin,
                    1.0 + (shape.out_scale - 1.0) * t,
                    1.0 - t,
                );
                let coming =
                    arriving.zoomed(shape.origin, shape.in_scale + (1.0 - shape.in_scale) * t, t);
                let mut list = coming.backdrop.clone();
                list.extend(going.chrome);
                list.extend(going.body);
                list.extend(coming.chrome);
                list.extend(coming.body);
                list
            }
            None => arriving.flatten(),
        };
        self.renderer
            .render(&gpu.device, &gpu.queue, encoder, view, &list, viewport);
        Ok(())
    }
}
