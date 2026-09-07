//! The menu stage, and the looping picture drawn behind it.

use anyhow::{Result, bail};

use oag_game::frontend::Draw;
use oag_game::input::Button;
use oag_game::render::{Renderer, letterbox_in};
use oag_game::{font, marquee, menu, movie};

use crate::frontend_stage::HeldFrame;
use crate::gpu::Gpu;

/// The pause overlay's tint, over a parked race's picture.
///
/// **Chosen, not authored.** Neither PSP title's own front-end XML defines a
/// pause screen to read one off: `Skin.xml`'s own `LoadXML` list is
/// exhaustively 22 files - see
/// [fe-menu-definitions.md](../../../../docs/formats/fe-menu-definitions.md) -
/// and none of them is a `Pause` definition. The one HUD string that names
/// pausing, `IG_PAUSE_QUIT`, is drawn by nothing:
/// [hud.md](../../../../docs/ui/hud.md) records "There is no pause" against
/// it, because leaving a race used to drop the `World` outright. So there is
/// no colour or alpha on the disc for this to recover, and this one is
/// this project's own rather than a reading of one.
const PAUSE_OVERLAY: [f32; 4] = [0.0, 0.0, 0.0, 0.55];

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
    /// The same face `renderer` draws rows in, kept a second time so a value
    /// column's width can be measured without reaching into the renderer's
    /// own, private atlas. Cheap to hold twice: [`font::Atlas`] is a small
    /// glyph table, not a GPU resource. See [`marquee::apply`].
    pub(crate) text_atlas: font::Atlas,
    /// The disc's own frame around every page: what the screen clears to and
    /// the rules it draws. Read once, off the front-end XML this source shipped
    /// - see `menu::read_frame` - and empty for a title whose frame is unread.
    pub(crate) frame: menu::Frame,
    /// The value marquee's clock: which row it is timing, and for how long.
    pub(crate) marquee: marquee::Timer,
    /// The modal prompt on screen, if one is: an on-screen keyboard or a
    /// yes/no. See [`crate::overlay`].
    ///
    /// **On the stage rather than on `Session`, and carrying its own
    /// purpose.** It is part of what is drawn, the same argument
    /// [`Self::backdrop`] makes one paragraph up - and while it is `Some` the
    /// menus behind it take no input at all, which is a fact about this
    /// stage's tick rather than about the session.
    pub(crate) prompt: Option<crate::overlay::Prompt>,
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
    /// menu-on-black that produced were the flicker on the START press. **It
    /// also starts `Some` on the `escape` -> menus path now**: the playhead
    /// still restarts there deliberately - see [`menu_playhead`], which is
    /// where that judgement call is argued - but the picture does not have to,
    /// and carrying it is what [`Backdrop::held`] is for.
    pub(crate) shown: Option<usize>,
    /// The picture behind [`Self::shown`], kept past the moment it was taken
    /// from the feed - the same reason `FrontendStage::held_backdrop` exists,
    /// and for the same handoff, one stage later. `take_upto` pops, so without
    /// this the frame the menus were last drawing would already be gone from
    /// the ring by the time a race ends and `Session::open_menus` wants to seed
    /// the next `MenuStage` with it. Moved onto [`crate::session::Session`] in
    /// `launch_race`, not read again here - once a race starts nothing in this
    /// stage runs until it is rebuilt.
    pub(crate) held: Option<HeldFrame>,
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
///
/// **This paragraph is about the playhead only.** The *picture* it restarts
/// on **is** carried across a race now, through [`Backdrop::held`] and
/// `Session::held_menu_backdrop` - that closed the one to three black frames
/// `escape` used to show while this fresh player waited for the restarted
/// feed's first frame, which is a different bug from the one this section
/// argues about.
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
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a tick is milliseconds; f32 holds it exactly"
        )]
        self.marquee.tick(dt as f32, marquee::focus(&self.menu));
        // The selected row's own pulse, measured on Pulse - see
        // `oag_title::MenuSkin::selected_pulse_period_secs`. Free-running like
        // the marquee's clock is reset-on-focus-change: unconditional here
        // because nothing measured suggests the original resets phase on a
        // selection change.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a tick is milliseconds; f32 holds it exactly"
        )]
        self.skin.tick_pulse(dt as f32);
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
    #[expect(
        clippy::too_many_arguments,
        reason = "frozen_race is one more fact the caller already has"
    )]
    pub(crate) fn render(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        feed: Option<&mut movie::Feed>,
        // The **live** table, off `Session::controls` - not
        // `oag_input::keys::bound_keys`'s default one. This stage holds no
        // input state of its own to read it from; see `Session::draw`'s call
        // site for why the closure has to be built there instead.
        bound_keys: &dyn Fn(Button) -> Vec<&'static str>,
        // Whether `Session::suspended_race` holds a parked race, off
        // `Session::draw` - this stage holds no session state of its own to
        // read it from either. Changes two things: the disc's own looping
        // backdrop is left out (see `shown` below - the parked race's own
        // picture is what shows through instead), and this draws with
        // `LoadOp::Load` plus a translucent [`PAUSE_OVERLAY`] rather than
        // clearing to black. `Session::draw` has already resolved the
        // parked race's scene into `view` by the time this runs, past
        // `has_scene`, which is what there is to load rather than clear -
        // and `resolve_scene` clears the *whole* target to black before it
        // draws that scene into its own rect, per `Framebuffer::present`, so
        // the aspect bars outside it are exactly as black as they would be
        // from this stage's own clear on an ordinary frame.
        //
        // **A known gap this does not cover**: a title whose frame authors a
        // `<ScreenClear>`, or whose `MenuSkin` carries a `background` (only
        // Pure's does, per `Skin::background`'s own doc), still draws that
        // as an opaque `Draw::Fill` first in `menu::draw_list`'s own
        // `backdrops` layer regardless of `frozen_race` - hiding the parked
        // race outright rather than dimming it. Both PSP titles' frames are
        // unread and neither authors a `MenuSkin::background`, so this is
        // invisible on the two titles this build actually plays, but it is
        // real and undocumented anywhere else; see this thread's `## Open`.
        frozen_race: bool,
        // The CONTROLS page's key-capture prompt, already resolved to text
        // and off `Session::awaiting_binding` - `Session::draw`'s call site
        // again, for the same reason `bound_keys` and `frozen_race` both
        // are: this stage holds no session state of its own. `Some` draws
        // `oag_game::prompt::message_draw` over everything else, last, so a
        // capture reads as the page being frozen rather than broken - the
        // gap `docs/architecture/menus.md`'s Rebinding section names as this
        // project's own, not the disc's.
        binding_prompt: Option<&str>,
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
                    backdrop.held = Some(HeldFrame {
                        index: frame.index,
                        picture: frame.picture,
                    });
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
        let arriving = menu::draw_list(
            &self.menu,
            &self.skin,
            bound_keys,
            &|text| font::measure(&self.text_atlas, text),
            // `None` over a parked race even when `shown` just decoded a
            // fresh frame above: the decode keeps running so the loop has
            // not drifted by the time a player resumes, but the picture
            // behind the menus is the race's own, not the disc's backdrop
            // movie playing underneath it.
            if frozen_race { None } else { shown },
            &self.frame,
        );
        let (list, clip) = match &self.change {
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
                (list, None)
            }
            // No transition in flight, so this is the picture staying on
            // screen rather than one mid-fade - the only time a marquee runs.
            // A fading, scaling row is not where a player is reading, so
            // `change.is_some()` skips this rather than scrolling a row
            // nobody can make out anyway.
            None => marquee::apply(
                arriving.flatten(),
                &self.menu,
                &self.skin,
                &|text| font::measure(&self.text_atlas, text),
                self.marquee.elapsed(),
            ),
        };
        // The pause overlay, drawn under the rows and over everything else:
        // first in the list, since the list paints back to front. Ahead of
        // `list` rather than pushed onto it, so it sits under the frame's own
        // marks and the transition tween too, not only under the rows -
        // `list` already carries every layer flattened together by this
        // point and there is no later seam to insert behind just the body.
        // See [`overlay_rect`] for why it is not simply `space.size`.
        let list: Vec<Draw> = if frozen_race {
            std::iter::once(Draw::Fill {
                rect: overlay_rect(self.skin.space(), viewport),
                color: PAUSE_OVERLAY,
            })
            .chain(list)
            .collect()
        } else {
            list
        };
        // The prompt last, so it is over everything: its own scrim is what
        // says the rows behind it are not the thing taking input, and a
        // marquee or a page tween running underneath it is still correct -
        // neither is being driven while a prompt is open, so what shows
        // through is a still picture of where the player will land back.
        let list: Vec<Draw> = match &self.prompt {
            Some(prompt) => list.into_iter().chain(prompt.draw(&self.skin)).collect(),
            None => list,
        };
        // The key-capture prompt, last of all - even over a page mid-tween or
        // a pilot-editor prompt, neither of which can be open at the same
        // time as a capture in practice (a capture only opens from the
        // CONTROLS page's own confirm, off the raw `Input` `Session::
        // maybe_begin_binding` reads ahead of `Menu::update`), but there is
        // no invariant enforcing that here, so "last" is the same safe
        // default the pilot-editor prompt above picked for itself.
        // `oag_game::prompt::message_draw` is the shared drawing code -
        // `--menu-page --menu-prompt binding`'s own headless capture calls
        // the same function, off `crate::capture::menu_page::prompt_draws`,
        // so this and that flag cannot draw two different pictures for the
        // same state.
        let list: Vec<Draw> = match binding_prompt {
            Some(text) => list
                .into_iter()
                .chain(oag_game::prompt::message_draw(&self.skin, text))
                .collect(),
            None => list,
        };
        // `LoadOp::Load` over a parked race - `Session::draw` already
        // resolved its scene into `view` this frame, and clearing here would
        // erase it. `Renderer::render`'s own black clear is right the rest
        // of the time, when this stage owns the frame outright.
        let load = if frozen_race {
            wgpu::LoadOp::Load
        } else {
            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
        };
        self.renderer.render_with(
            load,
            &gpu.device,
            &gpu.queue,
            encoder,
            view,
            &list,
            viewport,
            clip,
        );
        Ok(())
    }
}

/// A grid-space rectangle for the pause overlay's [`Draw::Fill`], sized so it
/// covers the whole of `viewport` once `Renderer::render_with` letterboxes
/// it - **not** `space.size` itself.
///
/// Every `Draw` this stage emits is fit into `viewport` through the same
/// `letterbox_in` scale the rows are, but the parked race behind the overlay
/// is not: `Framebuffer::resolve_scene` fills the whole viewport rectangle at
/// whatever aspect the player's ASPECT setting picked, with no notion of this
/// title's own `display_aspect` at all. A fill sized to exactly `space.size`
/// would then be letterboxed *again* on top of an already-correct picture,
/// leaving undimmed bands of live race wherever the two aspects disagree -
/// reachable any time ASPECT is not this title's own shape, `Aspect::Free`
/// included, which asks for whatever the window is and so has no bound on
/// how far the two can drift.
///
/// Dividing by the same scale the row layer is about to be shrunk by is what
/// cancels that shrink **exactly**: `a_mismatched_aspect_still_covers_the_
/// whole_viewport` below re-derives `Renderer`'s own clip-space mapping
/// independently and checks the four edges land at exactly `-1.0`/`1.0`,
/// not merely close.
fn overlay_rect(space: oag_game::frontend::Space, viewport: (f32, f32, f32, f32)) -> [f32; 4] {
    let (width, height) = space.size;
    let [scale_x, scale_y] =
        letterbox_in((viewport.2 as u32, viewport.3 as u32), space.display_aspect);
    let (full_width, full_height) = (width / scale_x, height / scale_y);
    [
        (width - full_width) * 0.5,
        (height - full_height) * 0.5,
        full_width,
        full_height,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Renderer`'s own clip-space mapping, reimplemented independently of
    /// `crate::render`'s `ui.wgsl`/`to_clip` rather than calling it, so this
    /// cannot pass by sharing a mistake with the code under test.
    fn to_clip_x(x: f32, screen_width: f32, scale_x: f32) -> f32 {
        (2.0 * (x / screen_width) - 1.0) * scale_x
    }

    fn to_clip_y(y: f32, screen_height: f32, scale_y: f32) -> f32 {
        (1.0 - 2.0 * (y / screen_height)) * scale_y
    }

    /// The bug this pins: a fill sized to exactly `space.size` is correct
    /// only when `viewport`'s own aspect happens to match the title's
    /// `display_aspect`. `overlay_rect` has to keep covering the whole
    /// viewport - both of the parked race's picture and the aspect bars
    /// `Framebuffer::resolve_scene` clears to black around it - at any
    /// aspect a player's ASPECT row (`Free` included) can produce.
    #[test]
    fn a_mismatched_aspect_still_covers_the_whole_viewport() {
        let space = oag_game::frontend::Space::PSP;
        for viewport in [
            // Matched: `viewport`'s own aspect equals the PSP's own.
            (0.0, 0.0, 480.0, 272.0),
            // Pillarboxed: much taller than the PSP's own 480/272 shape.
            (0.0, 0.0, 480.0, 1000.0),
            // Letterboxed the other way: much wider.
            (0.0, 0.0, 3000.0, 272.0),
            // An extreme `Aspect::Free` window with nothing to bound it.
            (0.0, 0.0, 7680.0, 200.0),
        ] {
            let rect = overlay_rect(space, viewport);
            let [scale_x, scale_y] =
                letterbox_in((viewport.2 as u32, viewport.3 as u32), space.display_aspect);
            let left = to_clip_x(rect[0], space.size.0, scale_x);
            let right = to_clip_x(rect[0] + rect[2], space.size.0, scale_x);
            let top = to_clip_y(rect[1], space.size.1, scale_y);
            let bottom = to_clip_y(rect[1] + rect[3], space.size.1, scale_y);
            assert!(
                (left - -1.0).abs() < 1e-4,
                "{viewport:?}: left edge {left}, wanted -1.0 (rect {rect:?})"
            );
            assert!(
                (right - 1.0).abs() < 1e-4,
                "{viewport:?}: right edge {right}, wanted 1.0 (rect {rect:?})"
            );
            assert!(
                (bottom - -1.0).abs() < 1e-4,
                "{viewport:?}: bottom edge {bottom}, wanted -1.0 (rect {rect:?})"
            );
            assert!(
                (top - 1.0).abs() < 1e-4,
                "{viewport:?}: top edge {top}, wanted 1.0 (rect {rect:?})"
            );
        }
    }
}
