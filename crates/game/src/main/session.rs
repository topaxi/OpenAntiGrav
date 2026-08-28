//! One window's run: the state that outlives whatever stage is on screen.
//!
//! [`Session`] owns the device, the mixer, the input devices and the
//! settings, and swaps [`Stage`]s under them. Its methods are split across
//! the modules below, which are children rather than siblings so that each
//! still reaches the private fields it always could.

use oag_core::TickClock;

use oag_game::render::Renderer;
use oag_game::{audio, catalogue, loading, menu, movie, perf, prefetch, race, settings, upscale};
use oag_gameplay::ControlScheme;
use oag_input::Controls;
use oag_render::mesh_render::Anisotropy;

use crate::gpu::Gpu;
use crate::stage::Stage;

#[path = "session/apply.rs"]
mod apply;
#[path = "session/frame.rs"]
mod frame;
#[path = "session/load.rs"]
mod load;
#[path = "session/menus.rs"]
pub(crate) mod menus;
#[path = "session/placeholder.rs"]
mod placeholder;

/// Everything that only exists once there is a window.
pub(crate) struct Session {
    pub(crate) gpu: Gpu,
    pub(crate) stage: Stage,
    /// One set of devices for both stages: they belong to the window rather than
    /// to what is on screen, so key state carries across the handoff and a focus
    /// loss releases everything whichever stage is running.
    pub(crate) controls: Controls,
    /// The mixer, and the device behind it when this run has one.
    ///
    /// Beside `controls` because it is the same kind of thing: a device that
    /// belongs to the run rather than to whatever is on screen, so a race
    /// starting does not restart the music. Stepped from inside the fixed
    /// timestep and never from the frame - see [`audio::Audio::tick`].
    pub(crate) audio: audio::Audio,
    /// Which Pulse releases this machine has. See [`App::music_discs`].
    pub(crate) music_discs: audio::MusicDiscs,
    pub(crate) clock: TickClock,
    pub(crate) last: std::time::Instant,
    /// Recent frame times, for the performance overlay.
    ///
    /// Fed from the same `elapsed` the fixed timestep is driven by, which is
    /// the only wall clock in the process. Nothing the simulation reads comes
    /// back out of it - see [`perf`].
    pub(crate) meter: perf::Meter,
    /// Draws the overlay over whatever the stage drew.
    ///
    /// A renderer of its own because a race has none: `race::Scene` draws
    /// meshes and knows nothing about text.
    pub(crate) overlay: Renderer,
    /// Set whenever the loop is about to stall on a load, so the frame that
    /// carries it is dropped from [`Session::meter`] rather than measured.
    ///
    /// Starts `true`: the first frame's `elapsed` reaches back to before the
    /// window existed.
    pub(crate) stalled: bool,
    /// Toggled by `Start` while a race is running: freezes the tick, nothing
    /// else. There is deliberately no pause screen behind it - see
    /// [`Session::frame`], which is the only place this is read.
    pub(crate) paused: bool,
    /// When the next frame is due, under a frame limit.
    ///
    /// A schedule rather than a stopwatch - see
    /// [`Session::schedule_next_frame`], which is where the difference between
    /// asking for 240 and getting it lives.
    pub(crate) next_frame: std::time::Instant,
    /// What `Launch Game` starts, kept because the front end is loaded long
    /// before anyone knows whether a race will be asked for.
    ///
    /// `None` only while the chooser is on screen: it names a `source`, and
    /// which source is precisely what has not been decided yet. See
    /// [`Session::finish_launcher`].
    pub(crate) race_options: Option<race::Options>,
    /// What the command line decided, until a pick turns it into a boot.
    ///
    /// Taken by [`Session::finish_launcher`], which is what makes that
    /// once-only: a second pick would find `None` and do nothing.
    pub(crate) pending: Option<crate::prepare::Pending>,
    /// `--trace`, kept because the stage that reads it may be built here.
    ///
    /// Every other stage is built in `App::open`, which has the flag in hand;
    /// the loading screen a pick opens is built in [`Session::finish_launcher`]
    /// long after that.
    pub(crate) trace: bool,
    pub(crate) log_every: u32,
    /// `--give`, carried into the race loop. See the CLI field.
    pub(crate) give: Option<oag_formats::weapons::Weapon>,
    /// `--autopilot`, carried the same way and applied to every race this
    /// session starts - including one launched from the menus, which is how
    /// the results table is reached without driving. See
    /// `race::Race::set_autopilot`.
    pub(crate) autopilot: bool,
    pub(crate) anisotropy: Anisotropy,
    /// The control scheme every race this session starts is driven with.
    ///
    /// Resolved once, at startup, from `--scheme` over `[controls] scheme`, and
    /// not re-read afterwards. Deliberate: a scheme changed mid-race would leave
    /// a half-finished gesture armed in `ShipState`, so the `CONTROLS` row says
    /// the change applies to the next race the way `BOOST FOV KICK` does.
    pub(crate) scheme: ControlScheme,
    /// Set the first time `Launch Game` starts a race, so a load that fails is
    /// reported once rather than on every frame.
    pub(crate) launched: bool,
    /// The persisted settings, kept because the menus change them and every
    /// change is written straight back.
    pub(crate) settings: settings::Settings,
    /// `--anim-seconds`: pins the trackside animation clock instead of deriving
    /// it from the tick.
    ///
    /// **Not in [`Self::settings`], on purpose.** It is a harness knob, and the
    /// boolean it replaced (`[graphics] animated_textures`) was persisted -
    /// which is how a value written before the default flipped went on
    /// freezing circuits long after the animation stopped being a guess.
    pub(crate) anim_seconds: Option<f32>,
    /// `--pvs`: overrides `[graphics] pvs_culling` for this run.
    ///
    /// **Not in [`Self::settings`], on purpose**, for the reason
    /// [`Self::anim_seconds`] is not: it is a harness knob for answering "did
    /// the visibility set take that geometry, or something else?", and a value
    /// persisted into a settings file would keep answering it long after the
    /// question was closed.
    pub(crate) pvs_culling: Option<bool>,
    /// Where every stage draws, before it is stretched onto the surface.
    ///
    /// On the session rather than on a stage because it outlives them: a race
    /// taking the window over does not want a fresh target, and the size it
    /// should be is a property of the window and the settings rather than of
    /// what happens to be on screen.
    pub(crate) framebuffer: upscale::Framebuffer,
    /// How many races this run has launched, which is what varies the loading
    /// screen's feature draw.
    ///
    /// **A counter rather than a clock**, so a run is reproducible: the seed a
    /// screen draws with is this, and two runs that launch the same races in
    /// the same order see the same features. See `loading::Screen::new`.
    pub(crate) races_launched: u64,
    /// Set when a menu asks to quit, read by the event loop.
    pub(crate) quit: bool,
    /// What the menus need, when this run has menus at all.
    pub(crate) shell: Option<Shell>,
    /// What the loading screen draws, kept for the *second* time it goes up.
    ///
    /// The boot's own screen used to consume these on the way past, which was
    /// enough while the only wait they covered was the one before the front
    /// end. A race load puts the same screen up again, and the title's own
    /// backdrop, tips and caption were read once with the archives open - so
    /// they are held here rather than read again with a menu on screen. See
    /// `Session::launch_race`.
    pub(crate) loading_assets: loading::Assets,
    /// The looping picture the menus are drawn on, when this source has one.
    ///
    /// On the session and not in [`Shell`], which is `Clone`d into every stage
    /// that needs it: a movie is a decoder and a multi-megabyte blob, held once.
    /// The menu stage borrows the feed for the one upload it needs and owns the
    /// playhead. See [`MenuStage::backdrop`].
    ///
    /// **It outlives the menu stage on purpose.** A player walking menu -> race
    /// -> escape -> menu would otherwise pay a fresh decoder and a fresh read of
    /// the cache file every time; instead the stage is rebuilt and the feed is
    /// only [`movie::Feed::restart`]ed, which is one decoder flush. It also means
    /// the worker is alive while a race is on screen - parked on a full ring,
    /// costing nothing, because nothing is taking frames out of it.
    ///
    /// **It also outlives the boot sequence, and that is the whole reason there
    /// is one movie here rather than two.** The front end draws out of this same
    /// feed under `Show Logo` and hands its playhead to the menus when it ends -
    /// see [`menu_playhead`] - so nothing is restarted on that path. The
    /// [`movie::Feed::restart`] above is for the race path only, where there is
    /// no playhead left to continue.
    pub(crate) backdrop: Option<movie::Feed>,
    /// The backdrop's frame rate and where on screen it goes, kept because the
    /// feed carries pixels and not presentation.
    pub(crate) backdrop_shape: Option<BackdropShape>,
    /// The menu backdrop's last shown picture, stashed here - beside the
    /// playhead above - the moment a race starts.
    ///
    /// `launch_race` moves it out of the outgoing `MenuStage`'s
    /// [`crate::menu_stage::Backdrop::held`], because `open_menus` needs it
    /// later and the stage holding it is dropped in the same call. **Not the
    /// playhead**, which `menu_playhead` restarts deliberately on this path -
    /// only the picture, so `escape` back to the menus does not show black for
    /// the one to three frames the restarted feed takes to produce its own
    /// first one. `None` before the first race; overwritten, not cleared, by
    /// every later one, the same way `stage.frontend.held_backdrop` is simply
    /// left behind rather than reset.
    pub(crate) held_menu_backdrop: Option<crate::frontend_stage::HeldFrame>,
    /// The `--prefetch` worker, when this run started one.
    ///
    /// **On the session and not on the loading stage.** The stage is replaced
    /// the moment the wait is over, and a handle owned by it would be dropped by
    /// that replacement - `Prefetch`'s `Drop` sets the stop flag, so a run whose
    /// loading screen finished planning first would silently abandon everything
    /// still to convert. Here it outlives every stage and is joined once, at the
    /// exit, by [`App::finish_prefetch`].
    pub(crate) prefetch: Option<prefetch::Prefetch>,
    /// What `--prefetch` asked for, until it is safe to start. See
    /// [`App::prefetch`] and [`Session::start_prefetch`].
    pub(crate) prefetch_pending: Option<prefetch::Options>,
    /// `--overlay`, applied when the sequence is assembled. Not to be confused
    /// with [`Self::overlay`] above, which is the performance overlay's own
    /// renderer: this one draws the intro's frame counter.
    pub(crate) boot_overlay: bool,
    /// `--pick-language`, read at the same moment.
    pub(crate) pick_language: bool,
    /// When the race scene finished building, kept until the first race frame
    /// is presented.
    ///
    /// Diagnostic only - it exists to answer "where did the wait actually go"
    /// with a timestamp rather than a guess, while the fade-to-race hand-off
    /// is still being tuned. See [`Session::advance_race_build`],
    /// [`Session::finish_race_loading`] and [`Session::frame`], which log the
    /// three points a stall could be hiding in: the build itself, the hand-off,
    /// and the first frame drawn with the new scene - the last of which is
    /// where a driver that defers pipeline compilation to first use would show
    /// up, since building the pipeline object earlier does not force that.
    pub(crate) race_ready_at: Option<std::time::Instant>,
}

/// What the menus need to know about the backdrop besides its pixels.
///
/// Read off the [`movie::Movie`] before its frames moved onto a decode thread,
/// because that is the last moment both are in one place.
#[derive(Clone, Copy)]
pub(crate) struct BackdropShape {
    /// The rate this cut presents frames at. A `.PMF` is 30000/1001; the PS2's
    /// `.IPF` cuts are 25/1 or 30000/1001 depending on which one it is.
    frame_rate: (u64, u64),
    /// Where the picture goes on the 480x272 screen, pillarboxed if the cut's
    /// display aspect is not the screen's.
    rect: [f32; 4],
}

/// Everything the menus need, gathered where it is loaded.
///
/// `None` on the `--race` path, which opens a window straight onto a track and
/// never loads a front end - so there is no font, no sprite sheet, and nothing
/// to draw a menu with. That is a real state rather than an oversight, and
/// [`Session::open_menus`] says so rather than unwrapping.
#[derive(Clone)]
pub(crate) struct Shell {
    pub(crate) definition: menu::Definition,
    /// Every raceable circuit and the name to show for it.
    pub(crate) tracks: Vec<(catalogue::Track, String)>,
    /// Every circuit a Zone race can be picked from, on the same terms as
    /// [`Self::tracks`] - what the CIRCUIT row shows once MODE is Zone. See
    /// `boot::Shell::zone_tracks` and [`Self::tracks_for`].
    pub(crate) zone_tracks: Vec<(catalogue::Track, String)>,
    /// Every team, valued by its id and labelled off the disc's string table.
    ///
    /// A pack's teams are in here too, and indistinguishable from the disc's -
    /// which is the point: the front end has no concept of downloadable
    /// content, only of what this source offers.
    pub(crate) teams: Vec<menu::Choice>,
    /// Every language this source offers, valued by its English name and
    /// labelled in itself.
    pub(crate) languages: Vec<menu::Choice>,
    /// The chosen language's table, kept for the one thing that re-reads the
    /// disc after boot: swapping the loading screen's styling needs the
    /// feature's strings again, and they were resolved once with the archives
    /// open. See `Session::reload_loading_assets`.
    pub(crate) strings: oag_game::language::StringTable,
    /// The chosen language's table entry, for the same reload. See
    /// `boot::Shell::entries`.
    pub(crate) entries: Option<String>,
    /// The front-end stylings this source ships art for, valued and labelled by
    /// the disc's own names for them.
    ///
    /// One row on every title but Wipeout HD, which ships two - see
    /// `oag_title::loading::FeatureStyle`. Resolved once at boot for the reason
    /// the circuits are: it is a property of the source and does not change
    /// while the game runs.
    pub(crate) front_end_styles: Vec<menu::Choice>,
    /// The race modes, valued by their token and labelled off the disc.
    ///
    /// Resolved once here rather than each time the menus open, the same way the
    /// circuits are: the strings do not change while the game runs, and the
    /// string table is not kept past boot.
    pub(crate) modes: Vec<menu::Choice>,
    pub(crate) font: oag_game::font::Atlas,
    pub(crate) sprites: oag_game::sprite::Sheet,
    /// How this title lays its menus out and colours them, carried from the
    /// serial that identified the source. See `boot::Shell::menu_skin`.
    pub(crate) menu_skin: &'static oag_title::MenuSkin,
    /// The grid that skin's numbers are in, and the one the rows are drawn in.
    ///
    /// Carried beside the skin for the reason the skin is carried at all: it is
    /// a property of the source, settled while the archives were still open, and
    /// re-deriving it when the menus open would mean asking the disc a question
    /// it has already answered. See `boot::Shell::space`.
    pub(crate) space: oag_game::frontend::Space,
    /// The face menu rows are drawn in, which is a bigger one than the rest
    /// of the front end uses. `None` draws them in `font`.
    pub(crate) menu_font: Option<oag_game::font::Atlas>,
    /// The disc's own frame around every menu page, read off the front-end XML
    /// while it was still in hand.
    ///
    /// Built here rather than when the menus open for the reason `space` is
    /// carried: it is a property of the source, it does not change while the
    /// game runs, and building it needs the parsed screens - which the boot
    /// shell has and a running session does not. Empty for a title whose frame
    /// is unread, which draws the menus exactly as they were drawn before this
    /// existed. See `oag_game::menu::read_frame`.
    pub(crate) frame: menu::Frame,
}

impl Shell {
    /// The list the CIRCUIT row shows for `mode` - [`Self::zone_tracks`] under
    /// Zone, [`Self::tracks`] otherwise. The one place that dispatch is made,
    /// so the row drawn, the row seeded and the circuit a launched race
    /// resolves against cannot disagree about which list is current.
    pub(crate) fn tracks_for(&self, mode: oag_race::Mode) -> &[(catalogue::Track, String)] {
        if mode == oag_race::Mode::Zone {
            &self.zone_tracks
        } else {
            &self.tracks
        }
    }

    /// Which circuit a stored `race.track` names, if this source has it on
    /// the list `mode` shows.
    fn track(&self, mode: oag_race::Mode, id: &str) -> Option<&catalogue::Track> {
        self.tracks_for(mode)
            .iter()
            .map(|(track, _)| track)
            .find(|track| track.id == id)
    }

    /// Whether a stored `race.team` is one this source offers, and its id.
    ///
    /// The teams are already `menu::Choice`s here rather than
    /// `catalogue::Team`s, and the value side of a choice *is* the id - see
    /// where the shell is built.
    fn team(&self, id: &str) -> Option<&str> {
        self.teams
            .iter()
            .find(|choice| choice.value == id)
            .map(|choice| choice.value.as_str())
    }
}

impl Session {
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.gpu.config.width = width;
        self.gpu.config.height = height;
        self.gpu
            .surface
            .configure(&self.gpu.device, &self.gpu.config);
        // The depth attachment follows the *framebuffer*, not the window, and
        // `frame` resizes both together - so a resize only has to invalidate,
        // which configuring the surface above already did.
    }

    /// How long one frame is allowed to take at the least, or `None` when
    /// nothing is limiting.
    ///
    /// `None` under `vsync = "on"` alone, whatever the limit says: there the
    /// display is deciding, and a second limiter underneath it does not halve
    /// the frame rate, it beats against the refresh and turns an even 60 into
    /// an uneven one. That is why the menu greys the row out under that one
    /// value and not the other two - `off` and `smooth` both leave the loop
    /// free to run ahead, and under `smooth` the limit is the only thing
    /// stopping the GPU rendering frames that are then discarded.
    fn frame_period(&self) -> Option<std::time::Duration> {
        if self.settings.display.vsync.paces_itself() {
            return None;
        }
        self.settings.display.frame_limit.period()
    }

    /// The earliest the next frame may start, or `None` for as soon as
    /// possible.
    pub(crate) fn next_frame_at(&self) -> Option<std::time::Instant> {
        self.frame_period().map(|_| self.next_frame)
    }

    /// What a frame time is measured against in the overlay: the rate this
    /// build is actually trying to present at.
    ///
    /// **Not the tick rate**, which is what this used to pass and which stopped
    /// being right the moment a frame limiter existed. Against 60 Hz a 4 ms
    /// frame is a 4-pixel sliver and every column is green, so a 240-limited
    /// run - the default - drew a pacing graph that conveyed nothing. The rule
    /// line means "one target frame" and the target has to be the one in force.
    ///
    /// With vsync on the display is the target and this build cannot ask a
    /// surface what its refresh is, so it falls back to the simulation's own
    /// rate. On a 144 Hz panel that reads pessimistically - the numbers are
    /// still measured, only the graph's scale is off - and getting it right
    /// needs a refresh rate off the monitor, which is work of its own.
    fn presentation_hz(&self) -> u32 {
        self.frame_period()
            .and_then(|_| self.settings.display.frame_limit.hz())
            .unwrap_or_else(|| self.clock.rate().hz())
    }

    /// Puts the next frame on the schedule, one period after the last one was
    /// *due* rather than one period after now.
    ///
    /// **This is the difference between a 240 limit delivering 240 and
    /// delivering 220.** A timer wakes at or after its deadline, never before,
    /// and the platform's granularity is around a millisecond - which at a
    /// 4.17 ms period is a quarter of it. Measuring the next deadline from when
    /// the loop actually woke banks that overshoot into every frame and the
    /// error compounds into a systematically low frame rate; measuring it from
    /// the previous deadline puts the frames on a fixed grid, so a late wake-up
    /// is followed by an early-relative one and the *average* is the rate that
    /// was asked for.
    ///
    /// The schedule is resynchronised whenever it is more than one period away
    /// from now, in either direction: behind means the loop stalled on a load
    /// and must not pay it back as a burst of frames, ahead means the limit
    /// itself just changed and the old period is still on the clock.
    fn schedule_next_frame(&mut self, now: std::time::Instant) {
        let Some(period) = self.frame_period() else {
            self.next_frame = now;
            return;
        };
        self.next_frame += period;
        if self.next_frame < now || self.next_frame > now + period {
            self.next_frame = now + period;
        }
    }
}

impl Session {
    /// Whether the authored visibility set culls this frame: `--pvs` when it
    /// was given, `[graphics] pvs_culling` otherwise.
    pub(crate) fn pvs_culling(&self) -> bool {
        self.pvs_culling
            .unwrap_or(self.settings.graphics.pvs_culling)
    }
}
