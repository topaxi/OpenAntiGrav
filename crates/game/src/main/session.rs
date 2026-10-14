//! One window's run: the state that outlives whatever stage is on screen.
//!
//! [`Session`] owns the device, the mixer, the input devices and the
//! settings, and swaps [`Stage`]s under them. Its methods are split across
//! the modules below, which are children rather than siblings so that each
//! still reaches the private fields it always could.

use oag_core::TickClock;

use oag_game::render::Renderer;
use oag_game::{boot, loading, movie, prefetch, records, settings};
use oag_gameplay::ControlScheme;
use oag_input::Controls;
use oag_mesh::mesh_render::Anisotropy;
use oag_present::{drs, perf, upscale};
use oag_raceplay::{self as race, catalogue, pilots};
use oag_ui::menu;

use crate::gpu::Gpu;
use crate::{race_stage::RaceStage, stage::Stage};

#[path = "session/apply.rs"]
mod apply;
#[path = "session/campaign.rs"]
mod campaign;
#[path = "session/draw.rs"]
mod draw;
#[path = "session/endrace.rs"]
mod endrace;
// `Session::escape` and `Session::resume_race` - split out of `menus.rs`
// under the 1,000-line rule; see `escape.rs`'s own doc for why this pair is
// the seam.
#[path = "session/escape.rs"]
mod escape;
#[path = "session/frame.rs"]
mod frame;
#[path = "session/frame_timing.rs"]
pub(crate) mod frame_timing;
#[path = "session/lifecycle.rs"]
pub(crate) mod lifecycle;
#[path = "session/language.rs"]
mod language;
#[path = "session/launch2048.rs"]
mod launch2048;
#[path = "session/load.rs"]
mod load;
#[path = "session/load_probe.rs"]
mod load_probe;
#[path = "session/menu_sound.rs"]
mod menu_sound;
#[path = "session/music.rs"]
mod music;
pub(crate) use load_probe::LoadProbe;
#[path = "session/menus.rs"]
pub(crate) mod menus;
#[path = "session/pacing.rs"]
mod pacing;
#[path = "session/pause.rs"]
pub(crate) mod pause;
#[path = "session/picker.rs"]
mod picker;
// Named `pilot_editor`, not `pilots` - `oag_raceplay::pilots` is already imported
// unaliased above, and a sibling module of the same name would shadow it,
// the same reason `remix_menu` below is not called `remix`.
#[path = "session/pilot_editor.rs"]
mod pilot_editor;
#[path = "session/custom_tracks.rs"]
mod custom_tracks;
#[path = "session/join.rs"]
mod join;
#[cfg(not(target_arch = "wasm32"))]
#[path = "session/lan.rs"]
mod lan;
#[cfg(target_arch = "wasm32")]
#[path = "session/lan_web.rs"]
mod lan;
#[path = "session/multiplayer.rs"]
mod multiplayer;
#[path = "session/seat_assign.rs"]
mod seat_assign;
#[path = "session/window_layout.rs"]
mod window_layout;
#[path = "session/pure_chain.rs"]
mod pure_chain;
#[path = "session/placeholder.rs"]
mod placeholder;
#[path = "session/pointer.rs"]
mod pointer;
#[path = "session/timing.rs"]
pub(crate) mod timing;
#[path = "session/windows.rs"]
mod windows;
#[path = "session/tournament.rs"]
mod tournament;
// Named `remix_menu`, not `remix` - `oag_game::remix` is already imported
// unaliased throughout `session::menus`, and a sibling module of the same
// name would shadow it at every one of those call sites.
#[path = "session/remix.rs"]
mod remix_menu;

/// Everything that only exists once there is a window.
pub(crate) struct Session {
    pub(crate) gpu: Gpu,
    pub(crate) stage: Stage,
    /// One set of devices for both stages: they belong to the window rather than
    /// to what is on screen, so key state carries across the handoff and a focus
    /// loss releases everything whichever stage is running.
    pub(crate) controls: Controls,
    /// The d-pad direction pressed on this tick, read before a screen consumes
    /// it, for a menu sound whose screen did not say which way it moved. See
    /// [`menu_sound::pad_direction`].
    pub(crate) pad_dir: Option<oag_ui::menu::nav::Dir>,
    /// The mouse and the touchscreen, latched between ticks the way
    /// `controls` latches a key. See `crate::pointer`.
    pub(crate) pointer: crate::pointer::Window,
    /// The on-screen racing controls and the Android pad's stick. See
    /// `crate::touch`.
    pub(crate) touch_overlay: crate::touch::Overlay,
    /// The mixer, and the device behind it when this run has one.
    ///
    /// Beside `controls` because it is the same kind of thing: a device that
    /// belongs to the run rather than to whatever is on screen, so a race
    /// starting does not restart the music. Stepped from inside the fixed
    /// timestep and never from the frame - see [`oag_sound::Audio::tick`].
    pub(crate) audio: oag_sound::Audio,
    /// Which Pulse releases this machine has. See [`App::music_discs`].
    pub(crate) music_discs: oag_sound::MusicDiscs,
    /// The race playlist's saved place: whether this run has already put it back,
    /// what was last written and the ticks since a race last wrote it. See
    /// `oag_game::music_state` and `session/music.rs`.
    pub(crate) music_place: oag_game::music_state::Run,
    pub(crate) clock: TickClock,
    pub(crate) last: web_time::Instant,
    /// Recent frame times, for the performance overlay.
    ///
    /// Fed from the same `elapsed` the fixed timestep is driven by, which is
    /// the only wall clock in the process. Nothing the simulation reads comes
    /// back out of it - see [`perf`].
    pub(crate) meter: perf::Meter,
    /// What the race's scene pass cost on the **GPU**, in the same seconds
    /// [`Session::meter`] is fed.
    ///
    /// A second meter rather than a second number in the first one, because it
    /// measures a different thing: `meter` is the interval between loop
    /// iterations, which under `Vsync::On` is the refresh and under any
    /// `FrameLimit` is the limit. This one is work, and it is the only signal a
    /// resolution controller could be built on - see
    /// [`oag_gpu::timing::PassTimer`] and
    /// [dynamic-resolution.md](../../../../docs/rendering/dynamic-resolution.md).
    ///
    /// It feeds the `dev` overlay's own reading and, since the controller
    /// landed, sits beside the reading [`Session::drs`] is given - the same
    /// number, kept as a window here and as a policy input there.
    pub(crate) scene_cost: perf::Meter,
    /// The dynamic-resolution controller, fed the same GPU reading
    /// [`Session::scene_cost`] is.
    ///
    /// Runs only while `[graphics] target_fps` names a rate; off, it
    /// is asked for the ceiling every frame and returns it. See
    /// [`oag_present::drs`], which owns the whole policy and reads nothing.
    pub(crate) drs: drs::Controller,
    /// The timer behind [`Session::scene_cost`], or `None` on a device that
    /// cannot be asked - in which case dynamic resolution has no signal at all
    /// and never moves the extent, whatever the menu row says. The row cannot
    /// be greyed for it: `disabled_by` names a *setting* and an adapter
    /// capability is not one. See `docs/architecture/menus.md`.
    pub(crate) pass_timer: Option<oag_gpu::timing::PassTimer>,
    /// What FSR 3.1's six **render-resolution** dispatches cost on the GPU, in
    /// the seconds [`Session::scene_cost`] is fed.
    ///
    /// A second meter because it measures a second thing and one a player
    /// choosing an upscaler actually has to weigh: the temporal resolve runs
    /// *after* the scene pass and is not in its reading, so a render scale
    /// that halved the scene can still cost more overall. Empty on every frame
    /// no temporal upscaler ran, which is most of them.
    ///
    /// **Six dispatches and not eight since
    /// [ADR-0045](../../../../docs/architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md)**:
    /// the two that resolve a pixel are [`Session::upscale_presented_cost`],
    /// because they are the half of the chain a resolution controller cannot
    /// make cheaper.
    pub(crate) upscale_cost: perf::Meter,
    /// The timer behind [`Session::upscale_cost`].
    ///
    /// **Its own ring rather than a share of [`Session::pass_timer`]'s**: that
    /// one feeds the dynamic-resolution controller, and a slot spent on the
    /// upscaler is a frame the controller does not get a scene reading for.
    /// Four slots each is eight tiny buffers - see `PassTimer`'s own note on
    /// what a ring costs.
    pub(crate) upscale_timer: Option<oag_gpu::timing::PassTimer>,
    /// What FSR 3.1's `accumulate` and `rcas` cost on the GPU, in the same
    /// seconds - the half of the chain that runs at presentation resolution
    /// and does not move when the extent does.
    ///
    /// See [ADR-0045](../../../../docs/architecture/adr/0045-fsr3-splits-into-a-scaled-and-a-presented-reading.md)
    /// for why this is measured apart from [`Session::upscale_cost`] rather
    /// than summed with it: it is the only part of the chain that belongs in
    /// [`drs::Cost::fixed`], and counting the whole chain there made the
    /// budget smaller than the truth.
    pub(crate) upscale_presented_cost: perf::Meter,
    /// The timer behind [`Session::upscale_presented_cost`], on its own ring
    /// for the reason [`Session::upscale_timer`] has one.
    ///
    /// **Claimed, abandoned and resolved in lockstep with
    /// [`Session::upscale_timer`]**, never independently.
    /// `Framebuffer::resolve_scene` returns one `bool` for both because
    /// `Fsr3::render` encodes both compute passes or neither, and two rings
    /// that disagree by a frame are two rings `Session::feed_drs` can never
    /// match again - it pairs readings by frame index and treats a missing
    /// one as "not measured yet", which stalls the controller for the run.
    pub(crate) upscale_presented_timer: Option<oag_gpu::timing::PassTimer>,
    /// What the motion-blur chain costs on the **GPU**, in the same seconds.
    ///
    /// **The third meter, and the one that made the budget honest.** Motion
    /// blur draws through the render extent, so its cost falls with the
    /// resolution exactly as the scene pass's does - and it was entirely
    /// invisible: measured 2026-09-03 on an HD/Fury circuit at 2560x1440,
    /// `high` costs about 1.7 ms against a 9.3 ms frame, with the scene pass
    /// and the FSR 3.1 chain both unmoved. A controller that could not see it
    /// was budgeting against a fifth of the work it controls. See
    /// [`oag_present::drs`].
    pub(crate) blur_cost: perf::ChainCost,
    /// The timer behind [`Session::blur_cost`], on its own ring for the reason
    /// [`Session::upscale_timer`] has one.
    ///
    /// **Bracketed across six passes with one pair**, which is what
    /// `PassTimer::half_writes` exists for: the chain is prepare, two tile
    /// reductions, a neighbour-max, a gather and a copy, and timing any single
    /// one of them would measure a fraction of the cost.
    pub(crate) blur_timer: Option<oag_gpu::timing::PassTimer>,
    /// What Wipeout HD/Fury's read bloom chain costs on the **GPU**, in the
    /// same seconds.
    ///
    /// **The fourth meter, and the fix for a gap ADR-0042 recorded rather than
    /// closed.** `oag_post::hd_bloom::Chain::run` draws through the
    /// render extent - the same viewport every level of its ladder follows -
    /// so its cost belongs beside the scene pass and the motion-blur chain in
    /// [`drs::Cost::scalable`], not folded into `drs::RESIDUAL_SHARE` as an
    /// assumed constant. See
    /// [ADR-0043](../../../../docs/architecture/adr/0043-hd-bloom-joins-the-scalable-budget.md).
    ///
    /// `None` on Pulse, Pure, and any HD circuit whose `track.envsettings`
    /// carries no `HDR and Bloom` block - `race::scene::frame::Scene::hd` is
    /// `None` there too, and `stats.hd_bloom_encoded` is how the frame loop
    /// tells the two "nothing measured" cases (no chain, ring full) apart.
    pub(crate) hd_bloom_cost: perf::ChainCost,
    /// The timer behind [`Session::hd_bloom_cost`], on its own ring for the
    /// reason [`Session::upscale_timer`] has one.
    ///
    /// **Bracketed the same way [`Session::blur_timer`] is**, with one
    /// simplification: `Chain::run` has no early return, so the only way a
    /// claim goes unwritten is a scene with no `Chain` at all, never a
    /// mid-chain bail-out.
    pub(crate) hd_bloom_timer: Option<oag_gpu::timing::PassTimer>,
    /// How long [`Session::frame`] itself took, in the same seconds
    /// [`Session::meter`] is fed - the **fifth** meter, and the first one that
    /// is not a GPU timestamp at all. Measured from the swapchain acquire near
    /// its top (`Gpu::acquire`) to the present, so both blocking swapchain
    /// calls are inside it and inside [`Session::present_cost`] alike.
    ///
    /// **What the `OTHER` row could never say.** That row is
    /// `frame - (every timed pass)`, and a large one has three possible
    /// causes - idle sleep under a frame limiter or vsync, CPU work on this
    /// thread, and GPU passes nothing brackets - which no amount of further
    /// device timing can tell apart, because the first of them is not GPU
    /// work and not work at all. Two `Instant`s do tell them apart, and they
    /// need no adapter feature: a machine with no `TIMESTAMP_QUERY` gets this
    /// breakdown when it gets no other. See [`perf::CpuCost`].
    ///
    /// Deliberately **display-only**. It is not fed to [`Session::drs`] and
    /// must not be: `drs::Cost` splits GPU cost by whether the render extent
    /// moves it, and a CPU reading answers a different question - see
    /// `race::scene::frame::Scene::render`'s own note on which passes may be
    /// timestamped for the controller.
    pub(crate) cpu_cost: perf::Meter,
    /// How much of [`Session::cpu_cost`] went into the two swapchain calls
    /// that may block - `Surface::get_current_texture` and `Queue::present`.
    ///
    /// The sixth meter, and the one that separates a loop *waiting* from a
    /// loop *working*: under [`perf::Vsync::On`] the wait for the refresh
    /// happens inside the frame rather than in the limiter's own sleep, so
    /// without this a vsync-bound run reads as CPU-bound.
    pub(crate) present_cost: perf::Meter,
    /// Where each window frame's input sample sat against its swapchain
    /// acquire, summed a second at a time and logged at `debug`. See
    /// [`frame_timing`].
    pub(crate) frame_timing: frame_timing::FrameTiming,
    /// Whether the "this target is out of reach" line has already been said
    /// for the spell the controller is currently in.
    ///
    /// The condition is per-frame and would otherwise be a log line per frame;
    /// what a reader wants is the edge. Cleared when the controller stops
    /// reporting it, so a target that becomes reachable again and then does
    /// not says so twice.
    pub(crate) drs_unreachable_said: bool,
    /// Window 1's cost of each frame of a multi-window race, until every further window's
    /// rings have come back for it and the sum is fed to [`Session::drs`]
    /// ([`Session::feed_windows`]). Empty with one window.
    pub(crate) window_ahead: std::collections::BTreeMap<u64, drs::Cost>,
    /// Which frame the loop is on, counted rather than timed.
    ///
    /// Exists because a GPU reading arrives a frame or more after the frame it
    /// describes, so it has to name one: see
    /// [`oag_gpu::timing::Reading::frame`] and [`Session::stall_frame`].
    pub(crate) frame_index: u64,
    /// The most recent frame that carried a load.
    ///
    /// A reading from that frame or earlier is thrown away rather than
    /// recorded, which is [`Session::stalled`]'s guard applied to a
    /// measurement that arrives late: `meter.clear()` cannot reach a reading
    /// that has not come back yet, and a track load measured as a frame would
    /// drive a controller to the floor and take seconds to climb back.
    pub(crate) stall_frame: u64,
    /// The `Dev` overlay's memory line, throttled - see [`perf::memory::Probe`].
    pub(crate) memory: perf::memory::Probe,
    /// Draws the overlay over whatever the stage drew.
    ///
    /// A renderer of its own because a race has none: `race::Scene` draws
    /// meshes and knows nothing about text.
    pub(crate) overlay: Renderer,
    /// The cursor's own sprite, on `overlay`'s sheet - see
    /// `oag_game::cursor`. Rebuilt whenever the title changes, which is
    /// once: the chooser's own until a disc is picked, the title's after.
    pub(crate) cursor_sheet: oag_hud::sprite::Sheet,
    /// Which title `cursor_sheet` was built for; `None` is the chooser's.
    pub(crate) cursor_title: Option<&'static str>,
    /// Set whenever the loop is about to stall on a load, so the frame that
    /// carries it is dropped from [`Session::meter`] rather than measured.
    ///
    /// Starts `true`: the first frame's `elapsed` reaches back to before the
    /// window existed.
    pub(crate) stalled: bool,
    /// Moved by `Start` while a race is running: freezes the tick, nothing else,
    /// and counts a resume down in a race with more than one person. There is
    /// deliberately no pause screen behind it, only the line under it - see
    /// `session::pause`, which owns the rules and the one gate
    /// (`pause::State::ticks`).
    pub(crate) pause: oag_game::pause::State,
    /// The session's wall clock in seconds, read once a frame: what a pause's
    /// owner timeout and the resume countdown are measured on. A race that is
    /// frozen has no ticks to count, so this is not simulation time and is never
    /// hashed.
    pub(crate) wall: f64,
    /// When [`Self::wall`] last had the race move again out of a resume
    /// countdown, so `GO` can linger on screen for a moment.
    pub(crate) go_at: Option<f64>,
    /// Where [`Self::wall`] counts from.
    pub(crate) wall_origin: web_time::Instant,
    /// When the next frame is due, under a frame limit.
    ///
    /// A schedule rather than a stopwatch - see
    /// [`Session::schedule_next_frame`], which is where the difference between
    /// asking for 240 and getting it lives.
    pub(crate) next_frame: web_time::Instant,
    /// The limit [`Session::schedule_next_frame`] last logged, so a switch
    /// between the front end's cap and the race's is one line in the log and a
    /// steady run is none.
    pub(crate) logged_limit: Option<perf::FrameLimit>,
    /// What `Launch Game` starts, kept because the front end is loaded long
    /// before anyone knows whether a race will be asked for.
    ///
    /// `None` only while the chooser is on screen: it names a `source`, and
    /// which source is precisely what has not been decided yet. See
    /// [`Session::finish_launcher`].
    pub(crate) race_options: Option<race::Options>,
    /// The track the command line named (`--ballisticng-track`/`--track-file`), which
    /// every launch that does not pick its own sets on `race_options`; a CUSTOM
    /// remix sets its file there instead, and RACE AGAIN keeps whatever was
    /// raced. See [`Session::inherit_cli_foreign_track`].
    pub(crate) cli_foreign_track: Option<std::path::PathBuf>,
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
    pub(crate) give: Option<oag_tables::weapons::Weapon>,
    /// `--no-intro`, carried into the race loop. See the CLI field.
    pub(crate) no_intro: bool,
    /// `--connect`: the race is stepped through the server's protocol instead of
    /// `Race::tick`. See `oag_game::net_play`.
    pub(crate) net: Option<oag_game::net_play::NetPlay>,
    /// `--net-bot`, carried so a LAN race started from the menus is flown by the driver too.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code, reason = "LAN play is native only"))]
    pub(crate) net_bot: bool,
    /// Ticks a network race has stood ended with nobody finished (`session::frame`).
    pub(crate) net_over_ticks: u32,
    /// `--autopilot`, carried the same way and applied to every race this
    /// session starts - including one launched from the menus, which is how
    /// the results table is reached without driving. See
    /// `race::Race::set_autopilot`.
    pub(crate) autopilot: bool,
    /// `--autopilot-pilot`, carried the same way. See `race::Race::set_autopilot_pilot`.
    pub(crate) autopilot_pilot: Option<oag_ai::Pilot>,
    /// `--autopilot-skill`, carried the same way. See `race::Race::set_autopilot_tuning`.
    pub(crate) autopilot_skill: Option<oag_ai::Difficulty>,
    /// `--ai`, carried the same way. See `oag_game::ai_choice`.
    pub(crate) ai_choice: oag_game::ai_choice::AiChoice,
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
    /// The Wipeout 2048 campaign event the next [`Session::launch_race`]
    /// starts, by its `SP.xml` name - set by the front end's own
    /// `Launch 2048` and taken by that one launch. `None` is an ordinary
    /// race from the menus' own options. See `oag_raceplay::load_event`.
    pub(crate) pending_event: Option<String>,
    /// The persisted settings, kept because the menus change them and every
    /// change is written straight back.
    pub(crate) settings: settings::Settings,
    /// Best lap, best total time and the last result, per circuit/mode/class -
    /// loaded once at [`Session`] construction and rewritten whenever a race
    /// finishes or is left. See `oag_game::records`'s own module doc for why
    /// this exists at all, and `Session::frame`/`Session::escape` for the two
    /// places that ever call [`records::Store::record`].
    pub(crate) records: records::Store,
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
    /// `--camera-jitter`: offsets the camera by a sub-pixel each frame.
    ///
    /// **Not in [`Self::settings`], on purpose**, and for a stronger reason
    /// than [`Self::anim_seconds`] and [`Self::pvs_culling`] are not: those are
    /// harness knobs, where this is infrastructure for an upscaler that does
    /// not exist yet. On its own it strictly worsens the picture, so there is
    /// no version of it a player should be offered a row for - and a value
    /// persisted now would still be set when the real gate ("is a temporal
    /// upscaler selected") arrives to replace it. See `oag_post::jitter`.
    pub(crate) camera_jitter: bool,
    /// The Zone visualiser's per-band peak-hold.
    ///
    /// On the session rather than on the race stage because it is ballistics
    /// over time: a hold rebuilt when a stage is is a hold that restarts from
    /// silence, and the original keeps its own sixteen floats in a struct
    /// that outlives a frame. See
    /// `oag_mesh::mesh_render::zone::Hold` for the recovered rule.
    pub(crate) zone_hold: oag_mesh::mesh_render::zone::Hold,
    /// Where every stage draws, before it is stretched onto the surface.
    ///
    /// On the session rather than a stage because it outlives them: a race
    /// taking the window over does not want a fresh target, and its size is a
    /// property of the window and the settings, not of what is on screen.
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
    pub(crate) prompt: oag_game::prompts::PromptState,
    /// Every screen filter this run can offer: the built-ins and the player's
    /// own `shaders/` directory, polled once a second by the frame loop so a
    /// saved edit shows without a restart. See `oag_game::screen`.
    pub(crate) screen_filters: oag_game::screen::Catalogue,
    /// The last `screen_filter` id the profile named that the catalogue could
    /// not find, so the frame loop says so once rather than sixty times a
    /// second. See `Session::note_missing_screen_filter`.
    pub(crate) missing_screen_filter: Option<String>,
    /// The title `--race` opened - see `App::race_title`. Read only when
    /// `shell` is `None`, which is exactly the `--race` route.
    pub(crate) race_title: Option<&'static oag_title::Title>,
    /// The platform that same source is for - see `App::race_platform`.
    pub(crate) race_platform: Option<oag_disc::Platform>,
    /// Every title this machine can currently open a source for, one row per
    /// distinct title - what the RACE REMIX page's TRACK TITLE and CRAFT
    /// TITLE rows offer, and how `Action::LaunchRemix` turns a picked title's
    /// name back into a source path.
    ///
    /// Surveyed once here rather than reused from whatever decided the disc
    /// chooser in `main.rs`: the two run in different processes' worth of
    /// state (this is `App`/`Session`, the chooser's own decision is local to
    /// `main.rs`) and threading one through costs more than a second survey,
    /// which happens once per windowed boot, not per frame. See
    /// `oag_game::launcher::{survey, distinct_titles}`.
    pub(crate) titles: Vec<oag_game::launcher::Candidate>,
    /// RACE REMIX's CUSTOM tracks. See [`oag_game::custom_tracks`].
    pub(crate) custom_tracks: oag_game::custom_tracks::Scan,
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
    pub(crate) race_ready_at: Option<web_time::Instant>,
    /// `--measure-race-load`, when this run was asked to time its race
    /// loads. See `session::load_probe`.
    pub(crate) load_probe: Option<load_probe::LoadProbe>,
    /// A race `escape` parked rather than discarded, waiting for
    /// [`Self::resume_race`].
    ///
    /// **Not part of [`Self::stage`].** The menus are what is on screen while
    /// this is `Some`, and a `Stage` can only ever be one thing - carrying the
    /// parked race as a second field is what lets both exist at once. `None`
    /// whenever the last race ran to completion, or was never escaped from at
    /// all - see [`Self::open_menus`], which is the only place that fills it,
    /// and only when the outgoing race had not finished.
    pub(crate) suspended_race: Option<Box<RaceStage>>,
    /// The campaign cell a launch in progress is racing, from the moment
    /// `Cell Selection`'s own confirm is handled
    /// ([`Session::launch_campaign_cell`]) to the moment the race stage is
    /// actually built, which drains it via [`Option::take`] into
    /// [`RaceStage::campaign_cell`] - see `crate::main::session::load`'s
    /// `finish_loading`. The mirror of `DAT_08b30ffc`,
    /// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`'s "current
    /// campaign cell" - not carried through `race::Options`/`race::Setup`
    /// because nothing in `oag_race`/`oag_gameplay` may know a campaign
    /// exists, only the composition root.
    ///
    /// `None` on every path but a campaign launch: an ordinary RACE-page or
    /// RACE REMIX launch clears it defensively (`Session::launch_from_settings`,
    /// `Session::launch_remix`) so a cell abandoned by backing out of `Team
    /// Selection` cannot leak into the next unrelated race.
    pub(crate) campaign_cell: Option<oag_tables::race_campaign::Cell>,
    /// Wipeout Pure's campaign list: the class it was left on, as an index into
    /// `PureClass::ALL`, so reopening it lands where the player was.
    pub(crate) pure_chain_class: usize,
    /// The split-screen lobby Start left, from the join screen's START until the
    /// race it asked for is built (`oag_game::split_screen::seat_for_launch`, which drains it)
    /// or the player backs out of the race box's screens (which drop it). `None`
    /// is an ordinary race.
    pub(crate) split_lobby: Option<oag_input::lobby::Lobby>,
    /// Which way a two-person race is cut: `--split-cut` at the start, then the race box's
    /// `SPLIT SCREEN` row on a title that authors it, set when the join screen starts the
    /// race. Kept across a restart from the end screens, which keeps the seats.
    pub(crate) split_cut: oag_render::viewport::Cut,
    /// The windows after the first of a `--windows N` run (`crate::windows`): empty until
    /// [`Session::open_screens`] has opened them, and then one per seat still open.
    pub(crate) screens: Vec<crate::windows::Screen>,
    /// Whether this run has opened seats: from then on an event for a window that is
    /// neither window 1 nor an open seat is dropped, and a lost focus means every window's.
    /// Stays set when the last seat closes, so a late event of a closed window is still
    /// not window 1's. Never set on a run with one window.
    pub(crate) screens_opened: bool,
    /// How many windows `--windows` asked for, in all, until the seats are opened (or
    /// refused for the title); `0` for a run with one window. See [`Session::screens_to_open`].
    #[cfg_attr(
        any(target_arch = "wasm32", target_os = "android", target_os = "ios"),
        allow(dead_code)
    )]
    pub(crate) windows_wanted: u8,
    /// What `--windows` / `--window-seats` asked for, kept for the run: a race the menus launch is
    /// laid out by it ([`oag_game::split_screen::layout_for`]). Empty for one window.
    pub(crate) window_layout: Vec<oag_game::split_screen::WindowSeats>,
    /// `--windows-fullscreen`: each window borderless fullscreen on a monitor of its own.
    #[cfg_attr(
        any(target_arch = "wasm32", target_os = "android", target_os = "ios"),
        allow(dead_code)
    )]
    pub(crate) windows_fullscreen: bool,
    /// Whether [`Self::window_layout`] is the layout screen's choice for the split race it started
    /// (rather than `--windows`/`--window-seats`, which seat everyone a menu race fields): a
    /// later one-person race from the menus stays one person in window 1
    /// ([`Session::seat_the_windows`]), whatever windows are still open.
    pub(crate) window_layout_chosen: bool,
    /// The monitor each further window was chosen to sit on, by name, window 2 first: the
    /// layout screen's choice (`Session::apply_window_layout`). Empty for a `--windows` run,
    /// whose windows sit beside window 1.
    #[cfg_attr(
        any(target_arch = "wasm32", target_os = "android", target_os = "ios"),
        allow(dead_code)
    )]
    pub(crate) window_monitors: Vec<String>,
    /// Which of the run's windows have focus. Used only while there are seats: with one
    /// window a lost focus is acted on at once, as it always was.
    pub(crate) focus: crate::windows::Focus,
    /// Windows (bit `i` is window `i + 1`, window 1 being 0) a left click landed on since
    /// the last tick. Only the further windows set one: window 1's is `pointer`'s. Read by
    /// the pause as that window's first seat's Start, and nothing else.
    pub(crate) seat_clicks: u8,
    /// Set while the RACE page is the LOCAL MULTIPLAYER flow's settings step
    /// (`Session::open_multiplayer_settings`): the lobby the join screen made is kept through
    /// the page's changes and the Track and Ship Select that follow. Cleared, with the lobby,
    /// when the menu leaves the page (`Session::sync_multiplayer`).
    pub(crate) multiplayer: bool,
    /// LAN play, from HOST GAME or JOIN GAME until the menus reopen after the race
    /// (`session::lan`). It owns the in-process host and the connection while they run.
    pub(crate) lan: Option<lan::Lan>,
    /// The ship this machine flies in a LAN race: its own pick, sent in the hello.
    #[cfg_attr(target_arch = "wasm32", allow(dead_code, reason = "LAN play is native only"))]
    pub(crate) lan_team: String,
    /// This machine as the host list compares it with a host (`session::lan`).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code, reason = "LAN play is native only"))]
    pub(crate) lan_facts: Option<lan::LanFacts>,
    /// Who sits in each pane of the layout, the panes counted across the windows from 0: the
    /// seat assignment screen's answer, kept for the race, a race again and a RETURN TO MENU
    /// race until the flow ends ([`Session::sync_multiplayer`]). Empty deals the slots in
    /// order ([`oag_game::split_screen::window_slots`]).
    pub(crate) pane_people: Vec<usize>,
    /// The devices the join screen last seated this session, seated again the next time it
    /// opens where they are still connected. **In memory only**: a restart joins again.
    pub(crate) mp_devices: Vec<oag_input::lobby::Device>,
    /// What the people seated after the first picked on their own Ship Select,
    /// in seat order, until the launch moves them into
    /// `oag_raceplay::Options::seat_picks`. Empty outside a split-screen race box.
    pub(crate) split_picks: Vec<picker::SeatChoice>,
    /// Where the `Classic` circuits a Pure campaign event races are, by
    /// `PI_Track` name: the shell's catalogue lists only the ordinary ones.
    /// Filled when the list opens, read by `launch_campaign_cell`.
    pub(crate) pure_classic_entries: std::collections::BTreeMap<String, String>,
    pub(crate) campaign_cursor: crate::campaign_stage::CellCursor, // kept across campaign openings
    /// [`Self::campaign_cell`]'s own difficulty rung, draining into
    /// [`RaceStage::campaign_difficulty`] the identical way and at the
    /// identical moment - see [`Session::launch_campaign_cell`]'s own doc
    /// for when this is `None` even with a cell in play.
    pub(crate) campaign_difficulty: Option<oag_tables::race_campaign::Difficulty>,
    /// [`Self::campaign_cell`]'s own resolved AI skill scale - `AI_ResolveSkillScale`
    /// (`docs/ghidra/functions/psp-pulse-usa/race-campaign.md`, `0x08834df4`)
    /// against [`Self::campaign_difficulty`]'s own rung and the launched
    /// track's `stats.xml` - draining into [`RaceStage::campaign_ai_skill_scale`]
    /// the identical way [`Self::campaign_difficulty`] does, and applied to
    /// the race's AI tuning there
    /// (`crate::main::session::load::advance_race_build`). Computed once, in
    /// [`Session::launch_campaign_cell`], rather than read again at drain
    /// time - see that function's own `resolve_campaign_ai_skill_scale`.
    /// `None` whenever [`Self::campaign_difficulty`] is, or the resolution
    /// itself falls back (see that function's own doc for when).
    pub(crate) campaign_ai_skill_scale: Option<f32>,
    /// A Tournament cell's own leg list and running standings, from the
    /// moment `Session::launch_campaign_cell` starts one to the moment its
    /// last leg's `EndRace Menu` is left (`RETURN TO GRID`/`RETURN TO MENU`).
    /// See `crate::main::session::tournament` and
    /// `race::tournament::Progress`'s own doc.
    ///
    /// **Unlike [`Self::campaign_cell`], this survives across a leg's own
    /// relaunch rather than draining into the race stage** - every mode
    /// this engine ran before Tournament reset its state fresh per race,
    /// because every one of them ended there; a tournament's own points
    /// total is the first fact that has to outlive one. `None` outside a
    /// Tournament cell, and cleared the moment its `EndRace Menu` is left by
    /// any option, finished or abandoned - this engine implements no
    /// save/resume (`Tournament_SaveProgress`/`_LoadProgress`), so leaving
    /// mid-tournament loses progress rather than parking it. **Chosen, not
    /// measured.**
    pub(crate) tournament: Option<race::tournament::Progress>,
    /// A CONTROLS binding row a player just confirmed, waiting for the key
    /// that will replace it.
    ///
    /// Set by [`Session::maybe_begin_binding`] off a raw `Input::take` on the
    /// selected row - `Entry::Binding` stays a `Menu::activate` no-op, so this
    /// is what makes confirming one mean something - and read by `app.rs`'s
    /// `KeyboardInput` handler, which is where the *raw* key a rebind needs
    /// actually lives: by the time a key reaches `Input`, it has already gone
    /// through `Controls::set_key`'s abstract-button mapping, and an unbound
    /// key never reaches it at all. `Some` freezes `Stage::Menu`'s own
    /// navigation for the tick - see [`Session::frame`] - and diverts every
    /// keyboard event away from `Controls::set_key` until [`crate::rebind::decide`]
    /// resolves it one way or the other.
    pub(crate) awaiting_binding: Option<oag_gameplay::input::Button>,
    /// The in-game pilot editor's own roster: the four built-ins, then
    /// whatever `pilots::directory` holds - see [`pilot_editor`].
    ///
    /// **Not what a race resolves pilots from.** `race::start` calls
    /// `pilots::load` fresh every time a race launches, deliberately never
    /// caching it - a race already in progress must not change out from under
    /// itself because a player saved an edit mid-race. This copy is purely
    /// for the AI PILOTS page to list and edit; the next race to launch reads
    /// the file, not this field.
    pub(crate) pilot_roster: pilots::Roster,
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
    /// The mods the MODS page lists, built with [`Self::definition`].
    pub(crate) mods: Vec<oag_game::mods::Listing>,
    /// The front end's navigation sounds, decoded at boot from the title's
    /// front-end bank. `None` where the boot never read one; a title without
    /// the bank holds an empty [`oag_sound::sfx::MenuSfx`] and plays nothing.
    pub(crate) menu_sfx: Option<oag_sound::sfx::MenuSfx>,
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
    pub(crate) strings: oag_ui::language::StringTable,
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
    /// The `KILLS` row's values, off the disc's own `Eliminations` list.
    pub(crate) kill_targets: Vec<menu::Choice>,
    /// The `WEAPONS` row's two states, valued by the disc's `On`/`Off` and
    /// labelled by its own `FE_ON`/`FE_OFF` strings.
    pub(crate) weapons: Vec<menu::Choice>,
    pub(crate) font: oag_ui::font::Atlas,
    pub(crate) sprites: oag_hud::sprite::Sheet,
    /// The front end's own `FEGlobals` table, carried from `boot::Shell`'s
    /// parsed screens for the one thing that re-reads the disc's XML after
    /// boot: a selection screen's per-entity `screen.xml` declares no globals
    /// and still names them for its stills' colour. See
    /// `oag_ui_screens::picker::slideshow::Slideshow::read`.
    pub(crate) globals: Vec<(String, String)>,
    /// Wipeout HD/Fury's own circuit-name fold, carried from
    /// `boot::Shell::circuit_names` for
    /// `crate::main::session::campaign::open_campaign`, the one place after
    /// boot that resolves a track id to a name outside the RACE page's own
    /// rows (which use [`Self::tracks`]/[`Self::zone_tracks`], already
    /// folded above). `CircuitNames::default()` on every other title, which
    /// is free to carry and a no-op to consult - see
    /// `oag_ui_screens::campaign::draw::track_line`'s own doc.
    pub(crate) circuit_names: oag_ui::language::CircuitNames,
    /// How this title lays its menus out and colours them, carried from the
    /// serial that identified the source. See `boot::Shell::menu_skin`.
    pub(crate) menu_skin: &'static oag_title::MenuSkin,
    /// Which title this is, carried the same way `menu_skin` is - from
    /// `boot::Shell::title`, not re-derived. Lets the RACE page's own
    /// VARIANT row ask this title's own
    /// [`oag_title::RaceDefaults::team_variants`] for whichever team
    /// `race.team` names, the same question RACE REMIX's craft-side VARIANT
    /// row asks of a *picked* title instead of the booted one.
    pub(crate) title: &'static oag_title::Title,
    /// Which console this source is for, carried the same way `title` is -
    /// from `boot::Shell::platform`, not re-derived. See
    /// `settings::profile_key`, which is why this is carried at all: a
    /// PS2-sourced Pulse race and a PSP-sourced one now read and write
    /// different `[render_profiles.<key>]` rows.
    pub(crate) platform: oag_disc::Platform,
    /// The grid that skin's numbers are in, and the one the rows are drawn in.
    ///
    /// Carried beside the skin for the reason the skin is carried at all: it is
    /// a property of the source, settled while the archives were still open, and
    /// re-deriving it when the menus open would mean asking the disc a question
    /// it has already answered. See `boot::Shell::space`.
    pub(crate) space: oag_display::space::Space,
    /// The face menu rows are drawn in, which is a bigger one than the rest
    /// of the front end uses. `None` draws them in `font`.
    pub(crate) menu_font: Option<oag_ui::font::Atlas>,
    pub(crate) face_scales: Vec<(String, f32)>,
    /// The face the screen title is drawn in, when this title names a role
    /// for it. `None` draws it in whichever face the frame is already bound
    /// to - see `boot::Shell::title_font` and
    /// `oag_title::MenuSkin::title_font`.
    pub(crate) title_font: Option<oag_ui::font::Atlas>,
    /// The PlayStation button-glyph face, when this source's language
    /// plugins name one - see `boot::Shell::buttons_font` and
    /// `render::Renderer::set_buttons_atlas`.
    pub(crate) buttons_font: Option<oag_ui::font::Atlas>,
    /// The disc's own frame around every menu page, read off the front-end XML
    /// while it was still in hand.
    ///
    /// Built here rather than when the menus open for the reason `space` is
    /// carried: it is a property of the source, it does not change while the
    /// game runs, and building it needs the parsed screens - which the boot
    /// shell has and a running session does not. Empty for a title whose frame
    /// is unread, which draws the menus exactly as they were drawn before this
    /// existed. See `oag_ui::menu::read_frame`.
    pub(crate) frame: menu::Frame,
    /// The front-end root's own `Confirm`/`Back` legend, carried from
    /// `boot::Shell::nav_legend` for the same reason [`Self::frame`] is - a
    /// property of the source, read once while the archives were open.
    /// `None` for a source whose root authors no `NavigationController`
    /// with either half [`oag_ui_screens::campaign::footer::NavigationLegend::read`]
    /// reads.
    pub(crate) nav_legend: Option<oag_ui_screens::campaign::footer::NavigationLegend>,
    /// The front-end root's own footer ticker layout, carried from
    /// `boot::Shell::ticker` on the same terms `nav_legend` is - a property
    /// of the source, read once while the archives were open. `None` for a
    /// source whose root authors no `TextInfoIsAlwaysLast` viewport, which is
    /// every title but Pulse today.
    pub(crate) ticker: Option<oag_ui_screens::campaign::footer::TickerLayout>,
    /// The style's menu backdrop, read at boot - see `oag_game::boot::backdrop`.
    /// `None` where the source has none, and then the menus sit on the movie or
    /// the page's clear.
    pub(crate) fury_backdrop: Option<std::sync::Arc<oag_game::boot::backdrop::MenuBackdrop>>,
    /// The race box's two selection screens, read at boot - see
    /// `oag_game::boot::Shell::track_select`. `None` launches straight from
    /// the RACE page.
    pub(crate) track_select: Option<oag_ui_screens::picker::Layout>,
    pub(crate) ship_select: Option<oag_ui_screens::picker::Layout>,
    /// The two halves of `Team SelectionSplit`, where the source authors it.
    pub(crate) ship_select_split: Option<[oag_ui_screens::picker::Layout; 2]>,
    /// Every team with what the ship picker shows for it - its ratings and
    /// its skins - where [`Self::teams`] is only the id and the label the
    /// RACE page's own row needs.
    pub(crate) team_details: Vec<catalogue::Team>,
}

impl Shell {
    /// Builds every field a booted [`boot::Shell`] settles, keeping
    /// `definition` as the caller's own rather than re-deriving it.
    ///
    /// **The one place this literal is written**, so a first boot
    /// ([`crate::prepare::Pending::windowed`]) and a live LANGUAGE-row switch
    /// ([`Session::resupply_language`]) cannot drift onto two different
    /// readings of the same `boot::Shell` - which is exactly the shape of bug
    /// this project has already hit once: [`oag_raceplay::hud::load_hud`]'s
    /// own `None` language stayed correct for months because nothing forced
    /// its one caller to agree with the picker.
    ///
    /// `definition` stays the caller's rather than being read off `title`
    /// again: the row *structure* a title offers - which rows exist, which
    /// are dropped by [`menu::Definition::drop_unavailable_race_variant`]/
    /// [`menu::Definition::drop_rows_picked_on_screen`] - does not depend on
    /// which language draws their labels, so a live switch passes in the
    /// menus' own current definition unchanged and only the fields below
    /// move.
    pub(crate) fn from_boot(
        title: &'static oag_title::Title,
        definition: menu::Definition,
        boot_shell: &boot::Shell,
    ) -> Self {
        Self {
            definition,
            mods: Vec::new(),
            menu_sfx: None,
            title,
            platform: boot_shell.platform,
            circuit_names: boot_shell.circuit_names.clone(),
            strings: boot_shell.strings.clone(),
            entries: boot_shell.entries.clone(),
            modes: menu::mode_choices(&boot_shell.strings),
            kill_targets: (boot_shell.race_setup.kill_targets.iter())
                .map(menu::Choice::plain)
                .collect(),
            weapons: boot_shell.race_setup.weapon_choices(&boot_shell.strings),
            // The disc's own names for its stylings, so the row offers what the
            // source has rather than a list this build holds.
            front_end_styles: boot_shell
                .loading
                .map(|loading| {
                    loading
                        .features
                        .iter()
                        .map(|style| menu::Choice::plain(style.name))
                        .collect()
                })
                .unwrap_or_default(),
            teams: boot_shell
                .teams
                .iter()
                .map(|team| menu::Choice::labelled(&team.id, team.label(&boot_shell.strings)))
                .collect(),
            tracks: boot_shell
                .tracks
                .iter()
                .map(|track| {
                    (
                        track.clone(),
                        catalogue::label(
                            track,
                            &boot_shell.circuit_names,
                            &boot_shell.strings,
                            &boot_shell.tracks,
                        ),
                    )
                })
                .collect(),
            zone_tracks: boot_shell
                .zone_tracks
                .iter()
                .map(|track| {
                    (
                        track.clone(),
                        catalogue::label(
                            track,
                            &boot_shell.circuit_names,
                            &boot_shell.strings,
                            &boot_shell.zone_tracks,
                        ),
                    )
                })
                .collect(),
            languages: boot_shell
                .languages
                .iter()
                .map(|language| menu::Choice::labelled(&language.name, &language.native_name))
                .collect(),
            font: boot_shell.font.clone(),
            menu_skin: boot_shell.menu_skin,
            space: boot_shell.space,
            menu_font: boot_shell.menu_font.clone(),
            face_scales: boot_shell.face_scales.clone(),
            title_font: boot_shell.title_font.clone(),
            buttons_font: boot_shell.buttons_font.clone(),
            sprites: boot_shell.sprites.clone(),
            globals: boot_shell
                .screens
                .globals
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect(),
            // The disc's own chrome, read by the boot itself - which is where
            // the parsed XML, the sheet and the grid were all in hand.
            frame: boot_shell.frame.clone(),
            nav_legend: boot_shell.nav_legend.clone(),
            ticker: boot_shell.ticker.clone(),
            fury_backdrop: boot_shell.fury_backdrop.clone(),
            track_select: boot_shell.track_select.clone(),
            ship_select: boot_shell.ship_select.clone(),
            ship_select_split: boot_shell.ship_select_split.clone(),
            team_details: boot_shell.teams.clone(),
        }
    }

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
        self.gpu.output.config.width = width;
        self.gpu.output.config.height = height;
        self.gpu.output.configure(&self.gpu.device);
        // The depth attachment follows the *framebuffer*, not the window, and
        // `frame` resizes both together - so a resize only has to invalidate,
        // which configuring the surface above already did.
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
