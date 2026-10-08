//! The menu stage, and the looping picture drawn behind it.

use anyhow::{Result, bail};

use oag_game::input::Button;
use oag_game::movie;
use oag_game::render::{Renderer, letterbox_in};
use oag_raceplay::pilots;
use oag_ui::frontend::{self, Draw};
use oag_ui::{font, menu};
use oag_ui_screens::marquee;

use crate::frontend_stage::HeldFrame;
use crate::gpu::Gpu;

/// The footer's `Confirm`/`Back` legend and tip ticker - split out under the 1,000-line rule.
#[path = "menu_stage/footer.rs"]
mod footer;

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
    /// The `Default`-role face `oag_ui_screens::campaign::footer`'s own prompts and
    /// tips draw through when their own `font` routes there (see
    /// `Draw::in_role`) - kept a second time for the same reason
    /// [`Self::text_atlas`] is: measuring `FE_CONFIRM`'s own shrink-to-fit
    /// width needs the atlas it will actually render in, not
    /// [`Self::text_atlas`]'s `menu`-role one. Not `Option`: every title has
    /// a `Default`-role atlas (`boot::load_font`'s own fallback is the
    /// built-in 5x7 set, never nothing), so this is always something to
    /// measure against, even on a title with no campaign screens at all.
    pub(crate) default_atlas: font::Atlas,
    /// The disc's own frame around every page: what the screen clears to and
    /// the rules it draws. Read once, off the front-end XML this source shipped
    /// - see `menu::read_frame` - and empty for a title whose frame is unread.
    pub(crate) frame: menu::Frame,
    /// The front-end root's own `Confirm`/`Back` legend, carried from
    /// `Shell::nav_legend` - `None` on a source whose root authors no
    /// `NavigationController` this build reads. Drawn on every ordinary
    /// menu page (not the picker, the campaign screens or a frozen race),
    /// `Confirm` unconditionally and `Back` only past the tree's own root -
    /// see [`Self::render`]'s own call site for why that boundary and not
    /// a disc-measured one.
    pub(crate) nav_legend: Option<oag_ui_screens::campaign::footer::NavigationLegend>,
    /// The footer ticker layout, from `Shell::ticker`, and its free-running
    /// clock (advanced by [`Self::tick`]). See `footer::ticker_overlay`.
    pub(crate) ticker: Option<oag_ui_screens::campaign::footer::TickerLayout>,
    pub(crate) ticker_elapsed: f32,
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
    /// The style's animated backdrop, when this source has one - the model,
    /// ticked here off the same fixed step the movie is; its GPU side is in
    /// [`Self::renderer`]. `None` on every other source, and then the rows sit
    /// on the movie or on the page's clear.
    pub(crate) styled: Option<oag_game::boot::backdrop::Live>,
    /// The race box's selection screen over this page, when one is open -
    /// see [`crate::picker_stage`]. Like [`Self::prompt`], it takes the
    /// tick's input whole while it is `Some`, and it is drawn instead of
    /// the rows rather than over them.
    pub(crate) picker: Option<crate::picker_stage::PickerStage>,
    /// The Race Campaign's `Grid Selection`/`Cell Selection`, when one is
    /// open - the same "takes the tick whole, draws instead of the rows"
    /// shape [`Self::picker`] is. See `crate::campaign_stage` and
    /// `crate::session::campaign`.
    pub(crate) campaign: Option<crate::campaign_stage::CampaignStage>,
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
    tween: oag_ui::anim::Tween,
    /// How far each half travels.
    shape: menu::Transition,
}

/// The looping menu picture: a player, where it goes, and which frame is up.
pub(crate) struct Backdrop {
    pub(crate) player: frontend::Player,
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
/// [`frontend::Player`] here instead put the picture back at frame zero on that
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
    carried: Option<frontend::Player>,
    frames: usize,
    frame_rate: (u64, u64),
) -> frontend::Player {
    carried.unwrap_or_else(|| {
        // `repeat`, which is the whole difference between this movie and the
        // intro: `FE Screen` sits under it for as long as a player is in the
        // menus, so it wraps rather than finishing on its last frame. See
        // `frontend::Player`.
        frontend::Player::new(frames, true, frame_rate)
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
        // One clip frame per tick: the original counts its clip up once per
        // rendered frame at sixty, and the stage's step is the same sixty.
        if let Some(styled) = &mut self.styled {
            styled.tick(self.menu.depth() == 1);
        }
        // The same fixed `dt` the backdrop is stepped with, and for the same
        // reason: nothing on this stage reads the wall clock, so two runs of
        // the same `--ticks` produce the same picture. See `oag_ui::anim`.
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
        // This stage's own footer ticker clock, free-running alongside the marquee's.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a tick is milliseconds; f32 holds it exactly"
        )]
        {
            self.marquee.tick(dt as f32, marquee::focus(&self.menu));
            self.ticker_elapsed += dt as f32;
        }
        // The campaign footer's own ticker clock - free-running, the same
        // "no reset on a screen change" choice the marquee's own pulse
        // above is documented making, since nothing measured says the
        // original restarts it either. See `CampaignStage::tick_ticker`.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a tick is milliseconds; f32 holds it exactly"
        )]
        if let Some(campaign) = &mut self.campaign {
            campaign.tick_ticker(dt as f32);
        }
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
        // The blocks behind HD's entries ease toward their selected width
        // at the executable's own rate, one step per tick - `Menu` keeps the
        // fraction and the skin says what it is worth. A title with no
        // blocks ticks a fraction nothing reads.
        self.menu
            .tick_focus(self.skin.blocks().map_or(1.0, |blocks| blocks.ease));
        // The picker's own clock - the turntable - off the same fixed tick.
        if let Some(picker) = &mut self.picker {
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a tick is milliseconds; f32 holds it exactly"
            )]
            picker.model.tick(dt as f32);
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
            tween: oag_ui::anim::Tween::new(self.skin.transition_secs()),
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
        // **The gap this used to leave is closed**: a `<ScreenClear>`, a
        // `MenuSkin::background` and any mark covering the whole screen are
        // one group in `menu::Frame::backdrops` - everything that sits under
        // the movie - and `race_behind` drops the group rather than filtering
        // its members one at a time. So none of the three can paint over the
        // parked race any more, which the two tests in
        // `menu/tests/frame.rs` pin directly.
        frozen_race: bool,
        // The CONTROLS page's key-capture prompt, already resolved to text
        // and off `Session::awaiting_binding` - `Session::draw`'s call site
        // again, for the same reason `bound_keys` and `frozen_race` both
        // are: this stage holds no session state of its own. `Some` draws
        // `oag_ui_screens::prompt::message_draw` over everything else, last, so a
        // capture reads as the page being frozen rather than broken - the
        // gap `docs/architecture/menus.md`'s Rebinding section names as this
        // project's own, not the disc's.
        binding_prompt: Option<&str>,
        // What the `AXIS` row on the AI PILOTS page currently means, off
        // `Session::draw`'s call site - this stage holds no `StringTable` of
        // its own to read `pilots::axis_preview_for` with. `Some` draws one
        // more line under the rows, in the same slot a warning or a restart
        // note would use; see `oag_ui_screens::prompt::axis_preview_draw`'s own
        // doc for why that is not `message_draw`'s modal shape.
        axis_preview: Option<&str>,
        // The RECORDS page's own per-class table, off `Session::draw`'s call
        // site the same way `axis_preview` is - this stage holds no
        // `records::Store` either. Empty off any page but RECORDS; see
        // `crate::records_page::table_for`'s own doc for why this is several
        // lines rather than the single one `axis_preview` occupies, and
        // `oag_ui_screens::prompt::record_row_draw` for how each pair is drawn.
        records_table: &[(String, String)],
        // This source's honest tip rotation - see `oag_game::records::ticker_tips`.
        ticker_tips: &[String],
        // `view`'s own full size, which a pillarboxed window makes different
        // from `viewport` - the picker's preview pass needs it for its depth
        // attachment, the same reason `RaceStage::draw_hud` takes it.
        target_size: (u32, u32),
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
            // Frames that could not be opened are no backdrop: drawing the empty
            // planes would be a green rectangle rather than a missing picture.
            _ => None,
        };
        // The movie where there is one, else the style's backdrop for this tick
        // - sized to the viewport, for the root (`Main Menu`) or any other page.
        let shown = shown.map(menu::Picture::from).or_else(|| {
            Some(
                self.styled
                    .as_ref()?
                    .picture(viewport, self.menu.depth() == 1),
            )
        });
        // A selection screen replaces the rows outright, in the same frame; its
        // preview goes on last, a 3D pass over the finished picture. No marquee,
        // page tween or prompt applies. A circuit's stills live on a sheet of
        // the screen's own, handed to the renderer once per selection.
        let picker_footer = self.picker_footer(ticker_tips);
        if let Some(picker) = self.picker.as_mut() {
            if let Some(sheet) = picker.take_sheet() {
                self.renderer.set_sprites(&gpu.device, &gpu.queue, sheet);
            }
            let mut layers = oag_ui_screens::picker::draw_list(
                &picker.model,
                &picker.layout,
                &self.skin,
                &self.frame,
                if frozen_race { None } else { shown },
                frozen_race,
                &|src| picker.placed(src),
                &|text| font::measure(&self.text_atlas, text),
            );
            layers.body.extend(picker.slideshow_draws());
            layers.chrome.extend(footer::hd_nav(
                &self.nav_legend,
                &self.default_atlas,
                picker,
            ));
            let (flat, ticker_clip) = footer::append(layers.flatten(), picker_footer);
            let list: Vec<Draw> = if frozen_race {
                std::iter::once(Draw::Fill {
                    rect: overlay_rect(self.skin.space(), viewport),
                    color: PAUSE_OVERLAY,
                })
                .chain(flat)
                .collect()
            } else {
                flat
            };
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
                footer::resolve_clip(None, ticker_clip, frozen_race),
            );
            let (orbit, seconds, rect) = (
                picker.orbit(),
                picker.model.seconds(),
                picker.layout.preview,
            );
            // `Track Creation`'s own `<Mode3D>` camera, when authored.
            let mode3d_model = picker.mode3d_model().cloned();
            if let Some(preview) = picker.preview.as_mut() {
                preview.draw_auto(
                    &gpu.device,
                    &gpu.queue,
                    encoder,
                    view,
                    viewport,
                    target_size,
                    self.skin.space(),
                    mode3d_model.as_ref(),
                    rect,
                    orbit,
                    seconds,
                );
            }
            return Ok(());
        }
        // The Race Campaign's own screens - no preview mesh, no slideshow:
        // both are plain 2D widget draws off `CellMode_Definition.xml`, the
        // same pass a picker's own body uses without the 3D tail.
        if let Some(campaign) = self.campaign.as_ref() {
            // Set by the non-HD arm below, when this frame draws one at
            // all - what the clip lookup after `flatten` searches for.
            // HD/Fury draws no ticker of its own yet, so this stays `None`
            // on that branch.
            let mut ticker_draw: Option<Draw> = None;
            // Wipeout HD/Fury draws a completely different screen behind the
            // same two names - see `oag_ui_screens::campaign::hd`'s own module doc.
            // `default_measure` is the footer legend's fit-to-gap measure.
            let default_measure = |text: &str| font::measure(&self.default_atlas, text);
            let layers = if campaign.is_hd() {
                match &campaign.screen {
                    // **HD only** - `Campaign Selection` ahead of `Grid
                    // Selection`. See `crate::campaign_stage`'s own module
                    // doc. `selection_layout` is `None` only when
                    // `Screen::Selection` itself could never have been
                    // built (`CampaignStage::new`'s own invariant), so the
                    // fallback below is never actually reached - kept
                    // rather than a panic in a render path regardless.
                    crate::campaign_stage::Screen::Selection(model) => {
                        match campaign.selection_layout() {
                            Some(layout) => {
                                let (fury_gold, hd_gold) = campaign.campaign_gold_medals();
                                let footer_overlay =
                                    campaign.nav_legend_draw(&layout.faces, &default_measure);
                                oag_ui_screens::campaign::selection::draw_list(
                                    model,
                                    layout,
                                    &self.skin,
                                    &self.frame,
                                    &campaign.strings,
                                    fury_gold,
                                    hd_gold,
                                    if frozen_race { None } else { shown },
                                    frozen_race,
                                    &|src| campaign.sprites.get(src),
                                    &footer_overlay,
                                    !campaign.flyer_shows().is_empty(),
                                )
                            }
                            None => oag_ui::menu::Layers::default(),
                        }
                    }
                    crate::campaign_stage::Screen::Grid(model) => {
                        let footer_overlay = campaign
                            .nav_legend_draw(&campaign.grid_layout().faces, &default_measure);
                        oag_ui_screens::campaign::hd::hd_grid_draw_list(
                            model,
                            campaign.grid_layout(),
                            &self.skin,
                            &self.frame,
                            &campaign.strings,
                            if frozen_race { None } else { shown },
                            frozen_race,
                            &|src| campaign.sprites.get(src),
                            &footer_overlay,
                        )
                    }
                    crate::campaign_stage::Screen::Cell { model, .. } => {
                        let (grid_index, grid_count, grid_summary, next_flyer) =
                            campaign.cell_grid_summary().unwrap_or_else(|| {
                                (0, 1, oag_ui_screens::campaign::GridSummary::empty(), None)
                            });
                        // No ticker: HD's shared `Skin.xml` has no `TextInfoIsAlwaysLast` viewport.
                        let footer_overlay = campaign
                            .nav_legend_draw(&campaign.cell_layout().faces, &default_measure);
                        oag_ui_screens::campaign::hd::hd_cell_draw_list(
                            model,
                            campaign.cell_layout(),
                            &self.skin,
                            &self.frame,
                            &campaign.strings,
                            campaign.circuit_names(),
                            grid_index,
                            grid_count,
                            &grid_summary,
                            &oag_ui_screens::campaign::hd::CellArt {
                                next_flyer,
                                fury: campaign.fury_open(),
                                track_emblem: &|id| campaign.circuit_emblem(id),
                            },
                            if frozen_race { None } else { shown },
                            frozen_race,
                            &|src| campaign.sprites.get(src),
                            &footer_overlay,
                        )
                    }
                }
            } else {
                // Both the legend's `FE_CONFIRM` shrink-to-fit and the ticker's
                // tips draw in the `Default` role, so one measure serves.
                let default_measure = |text: &str| font::measure(&self.default_atlas, text);
                match &campaign.screen {
                    // Never built for a non-HD title - see
                    // `crate::campaign_stage::Screen::Selection`'s own doc.
                    crate::campaign_stage::Screen::Selection(_) => oag_ui::menu::Layers::default(),
                    crate::campaign_stage::Screen::Grid(model) => {
                        let ticker = campaign.ticker_draw(&campaign.grid_layout().faces, &default_measure);
                        ticker_draw = ticker.clone();
                        let footer_overlay: Vec<Draw> = ticker.into_iter().collect();
                        oag_ui_screens::campaign::grid_draw_list(
                            model,
                            campaign.grid_layout(),
                            &self.skin,
                            &self.frame,
                            &campaign.strings,
                            if frozen_race { None } else { shown },
                            frozen_race,
                            &|src| campaign.sprites.get(src),
                            &footer_overlay,
                        )
                    }
                    crate::campaign_stage::Screen::Cell { model, .. } => {
                        let faces = &campaign.cell_layout().faces;
                        let mut footer_overlay = campaign.nav_legend_draw(faces, &default_measure);
                        let ticker = campaign.ticker_draw(faces, &default_measure);
                        ticker_draw = ticker.clone();
                        footer_overlay.extend(ticker);
                        // `Cell Help`'s own static overlay - drawn last, over
                        // everything else, while `triangle` has it open. See
                        // `oag_ui_screens::campaign::draw::cell_help_draw`'s own doc
                        // for why it is static rather than scripted.
                        if model.help_open()
                            && let Some(cell_help) = campaign.cell_help_layout()
                        {
                            footer_overlay.extend(oag_ui_screens::campaign::cell_help_draw(
                                cell_help,
                                &|src| campaign.sprites.get(src),
                            ));
                        }
                        oag_ui_screens::campaign::cell_draw_list(
                            model,
                            campaign.cell_layout(),
                            &self.skin,
                            &self.frame,
                            &campaign.strings,
                            if frozen_race { None } else { shown },
                            frozen_race,
                            &|src| campaign.sprites.get(src),
                            &footer_overlay,
                        )
                    }
                }
            };
            let split = layers.backdrop.len();
            let flat = layers.flatten();
            // The ticker's own clip: `Renderer::render_with`'s `clip` is
            // keyed on a draw's index in the *flattened* list
            // (`crate::marquee`'s own row-value scroll works the same way),
            // so this finds `ticker_draw`'s value again by equality rather
            // than carrying an index computed before `flatten` reordered
            // nothing - a Fury/HD frame has no ticker at all, and
            // `ticker_draw` is `None` there, so `position` never runs.
            let clip = ticker_draw.as_ref().and_then(|draw| {
                let index = flat.iter().position(|d| d == draw)?;
                let (left, right) = campaign.ticker_clip_bounds()?;
                Some((index, left, right))
            });
            let list: Vec<Draw> = if frozen_race {
                std::iter::once(Draw::Fill {
                    rect: overlay_rect(self.skin.space(), viewport),
                    color: PAUSE_OVERLAY,
                })
                .chain(flat)
                .collect()
            } else {
                flat
            };
            // The pause overlay's `Fill` above shifts every index by one -
            // `clip` was found against the unshifted list, so it has to
            // move with it.
            let clip = clip.map(|(index, left, right)| {
                (if frozen_race { index + 1 } else { index }, left, right)
            });
            let load = if frozen_race {
                wgpu::LoadOp::Load
            } else {
                wgpu::LoadOp::Clear(wgpu::Color::BLACK)
            };
            // The flyer cards go between the backdrop and the widgets -
            // `Grid Selection`'s one, `Campaign Selection`'s two; every other
            // campaign screen has none, which makes this exactly
            // `Renderer::render_with`.
            let shows = campaign.flyer_shows();
            let cards = campaign
                .flyers
                .as_ref()
                .map(|flyers| (flyers, shows.as_slice()));
            oag_game::flyer::render_list(
                &mut self.renderer,
                load,
                (&gpu.device, &gpu.queue, gpu.config.format),
                encoder,
                view,
                (&list, split),
                (viewport, target_size, self.skin.space()),
                cards.filter(|_| !frozen_race),
                clip,
            );
            return Ok(());
        }
        // Refreshed every frame, off whichever page is current, rather than
        // once when the menus opened: a player can navigate to a page with a
        // different reservation need (AI PILOTS today, see
        // `pilots::page_reserves_axis_preview`) without anything else in the
        // frame loop calling back into `menu::visible_rows` for it. Keeping
        // `self.menu`'s own cached figure in step with the page on screen is
        // what lets `rows::draw` read it straight off `Menu::visible_rows`
        // instead of recomputing a second, possibly-disagreeing answer of
        // its own - see that method's doc.
        let reserve_note = pilots::page_reserves_axis_preview(self.menu.page());
        self.menu
            .set_visible_rows(menu::visible_rows(&self.skin, &self.frame, reserve_note));
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
            frozen_race,
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
        // The AI PILOTS page's own live line, off `Session::draw`'s call
        // site. Skipped mid-transition (`self.change.is_some()`), the same
        // guard the marquee above uses and for the same reason: the row
        // list it is positioned against is a zoomed, fading picture during a
        // tween, not the still one `oag_ui_screens::prompt::axis_preview_draw`
        // reads `self.menu`'s scroll and visible-row count off.
        let list: Vec<Draw> = match (axis_preview, self.change.is_none()) {
            (Some(text), true) => list
                .into_iter()
                .chain(std::iter::once(oag_ui_screens::prompt::axis_preview_draw(
                    &self.menu, &self.skin, text,
                )))
                .collect(),
            _ => list,
        };
        // The RECORDS page's own per-class table, off `Session::draw`'s call
        // site - see `records_table`'s own doc. Skipped mid-transition for
        // the same reason `axis_preview` above is: the row list it lines up
        // against is a zoomed, fading picture during a tween.
        let list: Vec<Draw> = if self.change.is_none() {
            list.into_iter()
                .chain(
                    records_table
                        .iter()
                        .enumerate()
                        .flat_map(|(index, (label, value))| {
                            oag_ui_screens::prompt::record_row_draw(
                                &self.menu, &self.skin, index, label, value,
                            )
                        }),
                )
                .collect()
        } else {
            list
        };
        // The footer's own draws, skipped mid-transition too - see `footer.rs`.
        let (list, ticker_clip) = if self.change.is_none() {
            self.footer_overlay(list, ticker_tips)
        } else {
            (list, None)
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
        // `oag_ui_screens::prompt::message_draw` is the shared drawing code -
        // `--menu-page --menu-prompt binding`'s own headless capture calls
        // the same function, off `crate::capture::menu_page::prompt_draws`,
        // so this and that flag cannot draw two different pictures for the
        // same state.
        let list: Vec<Draw> = match binding_prompt {
            Some(text) => list
                .into_iter()
                .chain(oag_ui_screens::prompt::message_draw(&self.skin, text))
                .collect(),
            None => list,
        };
        // See [`footer::resolve_clip`] for the marquee/ticker trade-off.
        let clip = footer::resolve_clip(clip, ticker_clip, frozen_race);
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
fn overlay_rect(space: oag_display::space::Space, viewport: (f32, f32, f32, f32)) -> [f32; 4] {
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
        let space = oag_display::space::Space::PSP;
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
