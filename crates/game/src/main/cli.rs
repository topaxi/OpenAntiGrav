//! The command line: every flag `oag-game` takes, and nothing else.
//!
//! Its own module because it is a third of what `main.rs` used to be, and it
//! is pure declaration - `clap` derives the parser, and every consumer
//! (`main`, [`crate::headless`], [`crate::args`]) reads fields off it.

use clap::Parser;

use oag_display::display;
use oag_gameplay::ControlScheme;
use oag_mesh::mesh_render::Anisotropy;

use crate::args::DEFAULT_SIZE;

// `#[path]` for the reason `main.rs` gives its own children: this module is
// itself loaded through one, so a bare `mod` would look beside `main.rs`.
#[path = "cli/extra.rs"]
pub(crate) mod extra;
#[path = "cli/render_overrides.rs"]
mod render_overrides;

#[derive(Parser, Debug)]
#[command(
    name = "oag-game",
    about = "Run a Wipeout game (Pulse, Pure, HD/Fury, 2048 or Omega) from your own copy",
    // The commit too, so a deployed build can be told from the last one.
    version = oag_game::BUILD_VERSION
)]
pub(crate) struct Cli {
    /// Your game: a disc image (`.chd` or `.iso`, for Pulse, Pure or HD/Fury),
    /// or an unpacked folder (2048 or Omega, from their `.pkg`s), for example
    /// `data/images/pulse-psp-eu.chd`. See `oag_assets::Layout`.
    ///
    /// Left out, it is searched for: `data/images/` in the current directory,
    /// then beside the program (AppImage or executable), then
    /// `<data dir>/oag/images`. Europe is opened before the USA; name the USA
    /// one here to play it. `$OAG_IMAGE` short-circuits the search, and finding
    /// more than one title opens the chooser - see `--launcher`.
    pub(crate) source: Option<String>,

    /// Show the disc chooser, even when a source would resolve on its own.
    ///
    /// Without this the chooser appears only when nothing named a source and
    /// the search path holds more than one image; one image boots straight in,
    /// as it always did. With it the screen is shown for a single image too.
    ///
    /// **It lists the search path, and only the search path.** A source named
    /// on the command line, in `$OAG_IMAGE` or in `settings.toml` is not a row:
    /// this is how to reach something *other* than the one configured, not a
    /// list of everything reachable. An empty search path is still the ordinary
    /// "no disc image found" error rather than an empty screen.
    ///
    /// Needs a window: `--race`, `--dry-run` and `--screenshot` all name their
    /// own source and are refused.
    #[arg(long)]
    pub(crate) launcher: bool,

    /// Which movie to play: an archive entry name, or `hash:XXXXXXXX` for one of
    /// the reels whose name is not recovered.
    ///
    /// Defaults to `Data\Movies\Intro.PMF`, which is what the `LogoFMV` screen
    /// plays and the only movie the disc's own boot opens, or to the European
    /// cut of the dev/pub reel under `--reel`.
    #[arg(long)]
    pub(crate) movie: Option<String>,

    /// Boot into `Intro Screen->IntroMovie1` instead of `LogoFMV`.
    ///
    /// The code-side state with the frame-counted holds at 144, 231 and 260,
    /// playing the 260-frame dev/pub reel those counters describe. The disc's
    /// own boot never enters it - see `docs/architecture/frontend-boot.md` - so
    /// this is a way to watch a real, evidenced code path, not the boot order.
    #[arg(long)]
    pub(crate) reel: bool,

    /// Convert only the first this many frames of the movie, rather than all of
    /// them.
    ///
    /// The whole 40-second intro is 33 MiB of cache and about 80 seconds of
    /// `ffmpeg`, once. `--movie-frames 261` is enough for the reel leg.
    #[arg(long)]
    pub(crate) movie_frames: Option<usize>,

    /// Do not convert the movie at all. The sequence still plays, without a
    /// picture.
    #[arg(long)]
    pub(crate) no_video: bool,

    /// Where converted frames are cached.
    #[arg(long)]
    pub(crate) cache: Option<std::path::PathBuf>,

    /// Convert every movie again even when the cache already holds it,
    /// overwriting the cached file.
    ///
    /// **For a cache written by a build whose conversion has since changed.**
    /// The movie cache is keyed by the *source* bytes - the WAD name hash and
    /// the entry size - so a fixed transcode produces the same key as the wrong
    /// one it replaces, and every later run would reuse the stale file forever.
    /// This is the way out, and clearing the cache directory by hand is the
    /// other.
    ///
    /// Only affects movies that are transcoded at all: on a `native-video`
    /// build the H.264 reels decode through GStreamer and write no cache file,
    /// so there is nothing there for this to ignore. It converts nothing that
    /// would not otherwise have been converted - it only refuses the shortcut.
    ///
    /// The sounds are untouched. `--prefetch` still skips every ATRAC3+ stream
    /// it finds cached, which is five seconds of the wait rather than ten
    /// minutes of it.
    #[arg(long)]
    pub(crate) refresh_video: bool,

    /// Build the AV1 cache for the boot's movies rather than letting this
    /// build's platform decoder handle them.
    ///
    /// **You need this once.** A cache file that already exists is preferred
    /// over the platform decoder with no flag at all, so this is the run that
    /// creates one; every run after it picks the file up by itself. `--prefetch`
    /// does the same thing for every movie on the disc rather than just the
    /// boot's.
    ///
    /// Only affects the PSP's H.264 `.PMF` reels on a Linux `native-video`
    /// build; the PS2's `.PSS` and `.IPF` have no platform decoder and always
    /// went through the cache. For an uncached first run the platform decoder
    /// stays the default - see
    /// [ADR-0017](../../docs/architecture/adr/0017-gstreamer-native-video.md).
    ///
    /// The trade is a one-off transcode against a faster start every time after
    /// it. GStreamer decodes every wanted frame into memory before it returns;
    /// the cache is opened lazily and decoded a frame at a time. Measured on
    /// this workspace against the EU PSP disc, both reels:
    ///
    /// | | Media phase |
    /// | --- | --- |
    /// | GStreamer, nothing cached | **4.80 s**, and again on every boot |
    /// | This flag, nothing cached | **36.08 s** once (intro 30.33, backdrop 5.74) |
    /// | A cache file present - no flag needed | **0.05 s** |
    ///
    /// So it pays for itself after about eight boots.
    #[arg(long)]
    pub(crate) prefer_av1_cache: bool,

    /// Convert every movie and every sound on the disc up front, on a
    /// background thread, instead of one at a time on first use.
    ///
    /// **Off by default and deliberately so.** Converting lazily is what makes
    /// a cold boot half a second rather than ten minutes; this is the opposite
    /// trade, taken once, so that nothing afterwards ever waits. Measured on
    /// this workspace: 5.5 s for all 93 ATRAC3+ streams and about ten minutes
    /// for the ~13,000 movie frames, of which `Data\Movies\Intro.PMF` alone is
    /// 57 s.
    ///
    /// The game boots and runs normally while it happens, and progress goes to
    /// stdout. Interrupting is safe: both caches are keyed by content, so the
    /// next run picks up whatever finished. With `--dry-run` it is the whole run, no window: `oag_game::prefetch`.
    #[arg(long)]
    pub(crate) prefetch: bool,

    /// Render one frame to a PNG and exit, without opening a window.
    #[arg(long)]
    pub(crate) screenshot: Option<std::path::PathBuf>,

    /// With `--screenshot`, draw the loading screen at a stated conversion
    /// state instead of the sequence: `DONE/TOTAL`, e.g. `37/115`.
    ///
    /// The same kind of debugging view `--screen` and `--menu-page` are, and it
    /// exists for the same reason plus one of its own: a warm cache passes
    /// through a real mid-conversion state in a fraction of a second and a cold
    /// one takes ten minutes to leave it, so neither is a way to look at this
    /// screen. What is drawn is the window's own layout with a stated input.
    ///
    /// `--ticks` chooses the frame, which is what picks the wave's beat and the
    /// tip on show: the heartbeat peaks at 5 and 11 of its 24 frames.
    #[arg(long, value_name = "DONE/TOTAL")]
    pub(crate) loading_screen: Option<String>,

    /// With `--loading-screen`, draw a stated phase instead of `--prefetch`'s:
    /// `race`, or the boot's own media phase at a load step - `cached`,
    /// `decoding`, or `transcoding:DONE/TOTAL` (e.g. `transcoding:340/1200`).
    ///
    /// `race` is the screen between the menus and the grid, which is the
    /// title's own rather than this build's: Wipeout HD's full-screen still and
    /// its `FE_LOADINGDOT` caption, Pulse's wave and one of its tips. It counts
    /// nothing, so the `DONE/TOTAL` given to `--loading-screen` is ignored and
    /// `--track` supplies the circuit name instead.
    ///
    /// **Pair it with `--ticks 0`.** Any `DONE/TOTAL` where the two are equal
    /// reads as finished, and a finished screen is fading - so a later tick
    /// captures this phase part-way through its fade rather than at full
    /// opacity.
    ///
    /// The counts above stay the loads - `2/5` is two of five movie loads done -
    /// and this is what the one in flight is doing. Stated for the same reason
    /// they are: a cache hit is over in milliseconds and a frame counter holds
    /// one value for about a second, so neither is a state a capture can be
    /// timed to catch.
    #[arg(long, value_name = "STEP", requires = "loading_screen")]
    pub(crate) loading_step: Option<String>,

    /// With `--loading-screen` and `--loading-step race`, run a real race load
    /// of `--track` on a worker, paced at 60 Hz, and draw the bar at the stage
    /// the load has reached by `--ticks`. How a capture sees a stage-driven bar
    /// step, which no stated state can show.
    #[arg(long, requires = "loading_screen")]
    pub(crate) loading_live: bool,

    /// Write everything the mixer produced to a WAV instead of to a device.
    ///
    /// The audio counterpart of `--screenshot`, and for the same reason: this
    /// project's runs are headless and CI has no sound card, so a file is the
    /// only end-to-end evidence available that a sound was made at all.
    ///
    /// **Forces the null backend.** With a device attached `cpal`'s callback is
    /// already draining the mixer from its own thread, and pulling the same
    /// samples here would race it - so a run that dumps is silent on the
    /// speakers by construction rather than by accident.
    ///
    /// Length is a function of the tick count alone: `--ticks 600` is ten
    /// seconds whatever the machine's frame rate. **Requires `--screenshot`**:
    /// the buffer grows for as long as the run does, and a capture is the only
    /// route with an end.
    #[arg(long, value_name = "FILE")]
    pub(crate) dump_audio: Option<std::path::PathBuf>,

    /// Force the null audio backend: no device, no dump, nothing played.
    ///
    /// The same forced-null trick `--dump-audio` uses, without the WAV: a
    /// `--screenshot --until` run that has to sit through a movie leg needs the
    /// mixer to be tick-clocked rather than paced by a real device, and a
    /// device attached to the machine otherwise makes that a wall-clock race a
    /// headless run always loses - it finishes in far less real time than the
    /// movie takes to play, so `--until` spends its tick ceiling on a picture
    /// that has barely moved. See `oag_sound::Audio::movie_playhead` and
    /// ADR-0019. `--dump-audio` forces the same backend and works too, but also requires
    /// `--screenshot` and grows a sample buffer for the run; this is the plain way to ask for the tick-clocked movie leg alone. Conflicts with `--tap-audio`, which needs a real stream.
    #[arg(long, conflicts_with_all = ["dump_audio", "tap_audio"])]
    pub(crate) no_audio: bool,

    /// Ours: offer every circuit in the Race Box, whatever is cleared (`oag_game::unlock`).
    #[arg(long)]
    pub(crate) unlock_all: bool,

    /// Record what the **device** is handed to a WAV, while it plays.
    ///
    /// Not `--dump-audio`, which forces the null backend so that it can pull
    /// from the mixer without racing a callback that is already draining it.
    /// This one taps the callback itself, so it is the only way to see the
    /// signal a real sound card received - which is what a fault that only
    /// happens with hardware attached needs.
    ///
    /// Recording starts when the stream opens, so pair it with `--race` to skip
    /// the front end. The file is written once it is full, from the frame loop,
    /// so a run killed at the terminal still leaves it behind.
    #[arg(long, value_name = "FILE")]
    pub(crate) tap_audio: Option<std::path::PathBuf>,

    /// How many seconds `--tap-audio` records. Preallocated, so it costs about
    /// 384 KB a second.
    #[arg(
        long,
        value_name = "SECONDS",
        default_value_t = 90.0,
        requires = "tap_audio"
    )]
    pub(crate) tap_seconds: f32,

    /// Pin the animation clock, in seconds, instead of deriving it from the
    /// tick.
    ///
    /// Three mechanisms run off one clock, which a race takes from the tick:
    /// the two trackside ones - the per-material texture transform and the
    /// `Anim Transform` node motion - and, since 2026-08-24, Wipeout HD's
    /// engine-flame surface scroll, which the original drives from the same
    /// kind of global clock (`time`, engine shader parameter slot 0). This
    /// overrides it, for a comparison harness that needs our phase matched to a
    /// still of the original rather than left where the tick put it. Two runs
    /// at two values are also the headless way to show that an animated surface
    /// moves at all - and two runs exactly one period apart are how the flame's
    /// scroll rate was checked, since they must come back identical. Same flag,
    /// same meaning as `oag-view --anim-seconds`.
    ///
    /// **This is deliberately not a settings-file option.** It was one -
    /// `[graphics] animated_textures`, a boolean - and it was a footgun: it
    /// dated from when the animation was a guess, it grew to freeze moving
    /// scenery as well as scrolling surfaces without its name saying so, and a
    /// `false` persisted from before the default flipped left circuits silently
    /// static. A harness knob belongs on the command line.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) anim_seconds: Option<f32>,

    /// Plays this static path of the Fury menu backdrop on a `--menu-page`
    /// capture instead of the picker's roll, `--anim-seconds` being the time
    /// into it. The original's own debug `Force Path`. This is how a capture
    /// is lined up with an RPCS3 frame whose path and clock were read off the
    /// clip (`scripts/hd-fury-backdrop-break.py`).
    #[arg(long, value_name = "INDEX")]
    pub(crate) fury_path: Option<usize>,

    /// With `--screenshot`, the image's size as `WIDTHxHEIGHT`.
    ///
    /// A window is not always given the size it asks for - a tiling compositor
    /// hands out whatever its layout has - and the field of view is derived from
    /// the viewport, so this is how a capture can show what a differently shaped
    /// window would have drawn.
    #[arg(long, default_value = DEFAULT_SIZE)]
    pub(crate) size: String,

    /// With `--screenshot`, run until this state is current before capturing.
    ///
    /// Wipeout 2048 also takes `card`, `card:N` or `card:N:EVENT NAME`: stop
    /// with the campaign map's event card open on page `N` (default `0`), of
    /// the named event if one is given. The pulsed `--press` buttons open the
    /// card, then the right button turns its pages. A locked event's card
    /// never opens, so name an unlocked one.
    #[arg(long)]
    pub(crate) until: Option<String>,

    /// With `--screenshot`, draw one named screen out of the front-end XML and
    /// stop, instead of running the sequence.
    ///
    /// A debugging view of a screen the boot order does not reach yet, e.g.
    /// `--screen "Show Logo"`. Names are the XML's own, and a `Parent->Child`
    /// path works too.
    #[arg(long)]
    pub(crate) screen: Option<String>,

    /// With `--screen`, draw it this many seconds after it appeared, instead
    /// of settled.
    ///
    /// The live boot order's own clock - `Show Logo`'s `pulse="true"` throb
    /// and `Title Screen`'s `<Animation><Key>` wipe both need it, since
    /// `--screen` otherwise calls the public `draw_screen`, which reads as
    /// `f64::INFINITY` and freezes every one of those at its own settled
    /// end state. `None` keeps that settled behaviour, the same rule
    /// `--menu-anim-phase`/`--menu-picker-seconds` follow for the menu
    /// pages `--screen` does not reach.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) screen_seconds: Option<f32>,

    /// With `--screenshot`, capture the frame the way a window presents it:
    /// through the render scale, the upscaler, the brightness/gamma grade and
    /// the aspect bars.
    ///
    /// Off by default, because an ungraded capture at exactly `--size` is the
    /// right thing for a bug report and a graded one would make every capture
    /// disagree with every other. Turn it on to see what a player sees - which
    /// is the only way to see `[graphics] upscaler` do anything at all, since
    /// the ordinary capture never reaches the blit.
    #[arg(long)]
    pub(crate) presented: bool,

    /// With `--screenshot`, run this many ticks before capturing.
    ///
    /// A capture that reaches `Launch Game` spends what is left of them on the
    /// race the front end hands off to.
    #[arg(long, default_value_t = 0)]
    pub(crate) ticks: u32,

    /// With `--screenshot`, hold these buttons on every tick.
    ///
    /// Comma-separated abstract button names: `start`, `cross`, `circle`, `up`,
    /// `down`, `activate`, `cancel`.
    #[arg(long)]
    pub(crate) hold: Option<String>,

    /// With `--screenshot`, press and release these buttons on alternating
    /// ticks.
    ///
    /// `--press start,cross` is how the capture reaches `Launch Game`: START
    /// skips the intro, then cross picks a language. A held button only ever
    /// produces one rising edge, so two presses need two edges.
    ///
    /// It reaches the race too, not only the front end, which is what makes an
    /// edge-triggered manoeuvre testable headlessly: `--race --hold cross
    /// --press l` double-taps the left airbrake and sideshifts, repeatedly - a
    /// tap every other tick is well inside the `0.25 s` window. Expect the ship
    /// to be slow as well as displaced, because `q` is one key for both the
    /// airbrake *axis* and the sideshift *button*, so pulsing it also pulses the
    /// aerodynamic brake. That is a real pilot's gesture, not an artefact.
    #[arg(long)]
    pub(crate) press: Option<String>,

    /// Control scheme: `veteran` or `novice`.
    ///
    /// Overrides `[controls] scheme` for this run without writing it back. The
    /// two differ only in how a sideshift is asked for - veteran double-taps an
    /// airbrake, novice holds the sideshift button and flicks the stick - and
    /// they are the original's own `Control_Type` values. See
    /// `docs/ghidra/functions/psp-pulse-usa/input-bindings.md`.
    #[arg(long)]
    pub(crate) scheme: Option<ControlScheme>,

    /// Draw a frame counter over the intro.
    ///
    /// On by default when there is no picture, since a black screen for eight
    /// seconds is otherwise indistinguishable from a hang.
    #[arg(long)]
    pub(crate) overlay: bool,

    /// `--menu`, `--menu-page`, `--menu-anim-phase`, `--menu-picker-seconds`
    /// and `--menu-prompt` - see [`extra::MenuArgs`].
    #[command(flatten)]
    pub(crate) menu_args: extra::MenuArgs,

    /// Show the language picker even when a language is already chosen.
    ///
    /// Without this the picker is skipped once `settings.toml` names a
    /// language, which is what a player wants and what makes the second run
    /// shorter than the first.
    #[arg(long)]
    pub(crate) pick_language: bool,

    /// Print every state transition as it happens, exits included.
    #[arg(long)]
    pub(crate) trace: bool,

    /// Report what was loaded and exit, without rendering. With `--prefetch`: convert everything, write the cache manifest, then exit.
    #[arg(long)]
    pub(crate) dry_run: bool,

    /// Skip the front end and go straight to a ship on a track.
    ///
    /// Arrow keys steer, X or Return thrusts, Q and E are the airbrakes. The
    /// same race the front end's `Launch Game` starts.
    #[arg(long)]
    pub(crate) race: bool,

    /// A second disc image or extracted directory the craft, HUD and grid
    /// roster load from instead of `source` - a Race Remix from the command
    /// line. Left out, the craft comes from `source` like every other race.
    /// See `oag_source::remix::Remix`.
    #[arg(long)]
    pub(crate) craft_source: Option<String>,

    /// The track's `.vex` entry name, for either way into a race. The same name
    /// on both **Pulse** releases, PSP and PS2.
    ///
    /// **Left out, it comes from whichever title the source turns out to be**,
    /// which is why there is no `default_value` here: the two titles share no
    /// circuit directory at all, so a constant default would name a path that is
    /// not on a Pure disc and fail at the archive with a message about a missing
    /// entry rather than about a missing circuit. See [`default_track`].
    #[arg(long)]
    pub(crate) track: Option<String>,

    /// The team, which selects both the handling stats and the model.
    ///
    /// A team **id**, which is the folder under `Data\Ships\` and not always
    /// the name on screen - the Mirage pack's team is `Mantis`. Teams a mounted
    /// pack adds are accepted here like any other; see `--dlc`.
    /// Left out, it comes from the source's own title, the same way `--track`
    /// does: Pulse's default team is not on a Pure disc under that spelling.
    #[arg(long)]
    pub(crate) team: Option<String>,

    /// The player's own hull file, for a title whose
    /// `oag_title::race::HullVariant` axis offers one - `"extra"` for
    /// Pulse's Concept model, left out for the baseline `Ship.vex`. A team
    /// with no such axis, or a stem the team does not offer, races the
    /// baseline instead and says so in the load report.
    #[arg(long)]
    pub(crate) variant: Option<String>,

    /// The player's own alternate paint job: a `PI_ModelSkin` name the team
    /// declares - `Alternative` or `Eliminator` on both Pulse releases, left
    /// out for the hull's own textures.
    ///
    /// A **skin**, not a hull: the same geometry with four texture slots
    /// swapped out of a `.dat` the definition names. A team that declares no
    /// such skin races the baseline and says so in the load report.
    ///
    /// **Which skin a race flies is this project's choice, not the
    /// original's**, and no unlock is checked - see
    /// `oag_livery`'s `ship_skin` module docs.
    #[arg(long)]
    pub(crate) skin: Option<String>,

    /// A directory holding downloadable content: a pack's `.edat` files, or the
    /// `.zip` they were downloaded as. Repeatable.
    ///
    /// Left out, it is searched for: `data/dlc/` in the current directory, then
    /// beside the AppImage, then `<data dir>/oag/dlc`. `$OAG_DLC` short-circuits
    /// that, and `oag_source::source::resolve_dlc` documents the order.
    ///
    /// **Any pack works against any release.** The original locked a pack to
    /// its own territory's disc; this does not. See `docs/formats/dlc-pack.md`.
    #[arg(long)]
    pub(crate) dlc: Vec<String>,

    /// The speed class: venom, flash, rapier or phantom - or vector, which
    /// only Wipeout Pure authors.
    ///
    /// Checked here against every ladder measured so far, which is
    /// spell-checking rather than availability: whether *this* source's files
    /// author the rung is settled when the race loads, and a source that does
    /// not names the ladder it does carry.
    #[arg(long, default_value = "venom")]
    pub(crate) class: String,

    /// The race mode: time_trial, speed_lap, zone or single_race.
    ///
    /// Needed on the `--race` path in particular, which skips the menus and so
    /// has no other way to pick one. `single_race` is the only one that races
    /// with weapons, so it is the only one where a `Weapon Pad` draws or hands
    /// anything out.
    #[arg(long, default_value = "time_trial")]
    pub(crate) mode: String,
    /// A Wipeout 2048 campaign event by name - see `oag_raceplay::load_event`.
    #[arg(long, value_name = "NAME")]
    pub(crate) event: Option<String>,

    /// Development override: pin the Zone colour grade to a stage for the
    /// whole run.
    ///
    /// **Both titles' ladders are recovered now** - 2048's own zone-number
    /// table and, since 2026-08-31, HD/Fury's (`crates/hd/src/race.rs`'s
    /// `ZONE_STAGES`) - so a race normally escalates the grade on its own,
    /// re-deriving the stage from the zone counter every frame
    /// (`Scene::sync_zone_grade`). This flag exists for a **capture** rather
    /// than for seeing the look at all: without it, a stage 0 (`Start`) frame
    /// is not reachable at all, since even a zero-tick capture lands after
    /// the race's own opening-stage logic has already stepped past it (see
    /// `oag_raceplay::load::environment::zone_grade`), and there is otherwise
    /// no way to hold the grade still on a chosen rung long enough to compare
    /// it against the original frame for frame.
    ///
    /// Pins, not just sets once:
    /// [`oag_raceplay::zone_grade::ZoneGrade::pin_stage`] holds the stage
    /// against every later `show_zone` call, so the ladder does not
    /// overwrite it on the next frame the way a plain commit would.
    ///
    /// Clamped to the stages the loaded file names. Has no effect outside
    /// `--mode zone`, or on a title that ships no stage table.
    #[arg(long, value_name = "N")]
    pub(crate) zone_stage: Option<u32>,

    /// Development override: draw the Zone visualiser glow from a fixed
    /// synthetic ramp instead of this project's own live audio spectrum.
    ///
    /// **The only way to see the glow in a capture at all.** `--screenshot`
    /// runs on the null audio backend, which never publishes a spectrum -
    /// so an ordinary capture shows no glow whatever the scene authors, and
    /// a real device's spectrum is live and non-deterministic, the wrong
    /// input for a comparison two runs are meant to agree on. See
    /// `oag_game::race_capture::CaptureOptions::zone_spectrum_test`'s own doc comment.
    #[arg(long)]
    pub(crate) zone_spectrum_test: bool,

    /// Development view: draw the driveable ribbon instead of the track's art
    /// meshes.
    ///
    /// The ribbon is the geometry the simulation actually spawns on and queries,
    /// so ship-plus-ribbon shows directly whether the ship is where the physics
    /// thinks it is - useful for a physics comparison and misleading about
    /// everything else. A race draws the map by default.
    #[arg(long)]
    pub(crate) ribbon: bool,

    /// Overlay the collision soup - the geometry the physics world is actually
    /// made of, the same view `oag-view --collision` draws - on top of the
    /// track model chosen above.
    #[arg(long)]
    pub(crate) collision: bool,

    /// Spawn the rest of the grid even though `--mode` races solo in the
    /// original.
    ///
    /// A verification aid - see `race::Options::opponents` - for looking at
    /// the measured grid layout without a mode that actually fields one.
    /// `time_trial`, `speed_lap` and `zone` all race with `AI DIFFICULTY`
    /// greyed to `N/A` on the real Custom Race screen, and this flag exists
    /// despite that rather than because of it.
    #[arg(long)]
    pub(crate) opponents: bool,

    /// Play the trail-hit sparks on the player continuously, whatever the
    /// trigger says.
    ///
    /// **A verification aid for the *drawing*, not for the trigger** - the two
    /// fail differently and this separates them. `WO_TRAIL_HITSHIP` fires when
    /// a craft flies into another's engine trail, which under `--autopilot`
    /// happens a handful of times in a whole race and almost never in front of
    /// the camera, so "I saw nothing" cannot tell a burst that never played
    /// from one that played and drew nothing. With this on, a burst re-ignites
    /// at the player's nozzle every half second: if the screen stays empty the
    /// fault is in the draw path, and if it does not, the trigger is what to
    /// look at. See `Race::advance_trail_hits`.
    #[arg(long)]
    pub(crate) trail_sparks: bool,

    /// Let an opponent's driver fly the player's craft.
    ///
    /// A verification aid - see `race::Race::set_autopilot` - and the only way
    /// to reach the **end** of a race without a human at the keyboard for four
    /// minutes: a race finishes when the player crosses the line for the last
    /// time, so nothing that holds the throttle in a straight line ever gets
    /// there. It is what makes a screenshot of the results table possible:
    ///
    /// ```sh
    /// cargo run -p oag-game -- --race --mode single_race --autopilot \
    ///     --ticks 8000 --screenshot /tmp/scoreboard.png
    /// ```
    ///
    /// The capture stops on the tick the race ends, so a `--ticks` past the
    /// flag lands on the board rather than overshooting it.
    #[arg(long)]
    pub(crate) autopilot: bool,

    /// Pilot Assist for the player's craft in a headless run (`--screenshot`,
    /// `--trace-out`): `on` or `off`, default off. The settings file's
    /// `[controls] pilot_assist` drives the windowed game; a capture or trace
    /// takes only this flag so it never depends on a machine's settings.
    #[arg(long, value_parser = ["on", "off"])]
    pub(crate) pilot_assist: Option<String>,

    /// With `--autopilot`, fly the player with a named pilot instead of the
    /// neutral baseline `Driver::default` otherwise leaves slot 0 at.
    ///
    /// One of the same characters a real single race deals from
    /// [`oag_raceplay::pilots::load`]: the four built-ins - `balanced`, `aggressive`,
    /// `passive`, `shy` - or a name out of `<config dir>/oag/pilots/`. This is
    /// what makes `--autopilot` useful for checking *a character*, not only
    /// for reaching the finish line unattended - the roster a player edits in
    /// the pilot screen is the same one this flag can fly.
    ///
    /// Tempered by `--autopilot-skill`, or by `[ai] difficulty` when that flag
    /// is not given - the same temper an opponent's own entry gets, so naming
    /// `aggressive` here is exactly the character a grid slot could draw, not
    /// a stronger or weaker claim.
    #[arg(long, value_name = "PILOT", requires = "autopilot")]
    pub(crate) autopilot_pilot: Option<String>,

    /// With `--autopilot`, fly the player at a stated AI skill instead of the
    /// race's own: `novice`, `skilled`, `elite` or `ace`. See
    /// [`oag_ai::Difficulty`].
    ///
    /// **Independent of `[ai] difficulty`** - the opponents still race at
    /// whatever that resolves to, only the autopiloted craft moves. That is
    /// the point: it is what lets a claim like "Ace corners a third faster
    /// than Novice" be checked by flying both across the same field, without
    /// restarting the race between them and losing the comparison.
    #[arg(long, value_name = "SKILL", requires = "autopilot")]
    pub(crate) autopilot_skill: Option<oag_ai::Difficulty>,

    /// With `--race --screenshot`, arm the camera's impact shake at the end of
    /// a tick as a wall hit of that severity would: `TICK:SEVERITY`, severity
    /// `0..=1`, e.g. `120:1.0` and then `--ticks 121` for the first frame of a
    /// hard hit.
    ///
    /// A verification aid, there because a hard hit is hard to produce on
    /// demand and the first shaken frame is where the shake meets the motion
    /// blur and the temporal upscaler. See [`Race::force_shake`].
    #[arg(long, value_name = "TICK:SEVERITY", requires = "race")]
    pub(crate) force_shake: Option<String>,

    /// Seed the world generator, instead of `race::SEED`.
    ///
    /// A verification aid - see `race::Options::seed`. The one thing in a race
    /// that draws from the generator today is which pickup a weapon pad hands
    /// over, so this is how a capture is made to show a chosen weapon.
    #[arg(long)]
    pub(crate) seed: Option<u64>,

    /// In a race, print a telemetry line every this many ticks. Zero prints
    /// none.
    #[arg(long, default_value_t = 60)]
    pub(crate) log_every: u32,

    /// Keep handing the player this weapon whenever their pickup slot is empty.
    ///
    /// **A debug affordance, and worth saying why it exists.** A weapon can only
    /// be *seen* once something fires it, and `--race` holds the throttle
    /// without steering, so it never crosses a `Weapon Pad` and never receives a
    /// pickup. That made every visual change to a projectile unverifiable except
    /// by playing the game by hand - and two rocket changes shipped blind before
    /// this existed, one of them wrong (three pieces of track scenery read as a
    /// volley; see `oag_raceplay::Race::rocket_model_matrices`).
    ///
    /// Spelled as the `type` attribute the weapon table uses - `Rocket`,
    /// `Missile`, `Cannon` - and matched case-insensitively.
    ///
    /// It writes the player's pickup slot from **outside** the simulation, in
    /// the frame loop rather than in `Race::tick`, so nothing here can reach a
    /// determinism hash. Combine with `--press square` to fire repeatedly:
    ///
    /// ```sh
    /// cargo run -p oag-game -- --race --mode single_race --give rocket \
    ///     --hold cross --press square --ticks 900 --screenshot /tmp/shot.png
    /// ```
    #[arg(long, value_name = "WEAPON")]
    pub(crate) give: Option<String>,

    /// Anisotropic filtering level for track and ship textures: off, 2x, 4x,
    /// 8x or 16x.
    ///
    /// Overrides `[graphics] anisotropy` in the settings file
    /// (`settings::path`) for this run only; the file on disk is not changed.
    #[arg(long)]
    pub(crate) anisotropy: Option<Anisotropy>,

    /// How far out an authored `LodGroup` switches to its coarser tiers:
    /// `original`, `high` or `maximum`, overriding
    /// `[render_profiles.<title>] model_detail` for this run only (the file
    /// on disk is not changed). See `oag_mesh::mesh::ModelDetail`.
    #[arg(long)]
    pub(crate) lod: Option<oag_mesh::mesh::ModelDetail>,

    /// How far out a PSP `.vex` model keeps its finer texture levels:
    /// `original`, `high` or `maximum`, overriding
    /// `[render_profiles.<title>] texture_detail` for this run only (the file
    /// on disk is not changed). See `oag_mesh::mesh_render::TextureDetail`.
    #[arg(long)]
    pub(crate) texture_detail: Option<oag_mesh::mesh_render::TextureDetail>,

    /// Whether the track's authored visibility set culls this run: `true` or
    /// `false`.
    ///
    /// Overrides `[graphics] pvs_culling` in the settings file
    /// (`settings::path`) for this run only; the file on disk is not changed.
    /// Here for the same reason `--lod` is, and for one more: **when geometry
    /// goes missing, the first question is whether the PVS took it**, and two
    /// captures differing only by this flag answer it without editing a
    /// settings file between them - which is how a comparison ends up
    /// differing by something else as well.
    #[arg(long)]
    pub(crate) pvs: Option<bool>,

    /// What resolves the frame onto the surface: off, fxaa, smaa, fsr1 or
    /// fsr3.
    ///
    /// Overrides `[render_profiles.<title>] reconstruction` for *every* title,
    /// for this run only; the file on disk is not changed. Every title rather
    /// than one, because no title is open yet this early to single one out -
    /// see `crate::main::render_scale` in `main.rs`. Here for the same reason
    /// `--anisotropy` is, and for one more: two `--presented` captures
    /// differing only by this flag are how these get compared, and asking
    /// somebody to edit a settings file between them is how a comparison ends
    /// up differing by something else as well.
    #[arg(long)]
    pub(crate) reconstruction: Option<oag_display::display::Reconstruction>,

    /// What percentage of the displayed size the game is rendered at, 25 to
    /// 200.
    ///
    /// Overrides `[render_profiles.<title>] render_scale` for *every* title,
    /// for this run only - see `--upscaler`'s own doc for why every title
    /// rather than one. The companion to `--upscaler`: a resampler can only be
    /// judged at a scale where it has something to resample, and the two
    /// flags together are what let one command produce one image of a
    /// comparison.
    #[arg(long)]
    pub(crate) render_scale: Option<u32>,

    /// Offset the camera by a sub-pixel each frame, from a 16-phase Halton
    /// sequence.
    ///
    /// **This makes the picture worse, and is here anyway.** Camera jitter is
    /// the last renderer-side thing FSR 3.1 needs
    /// (`docs/overview/modern-features.md`), and nothing consumes it yet - with
    /// a spatial resolve or none, all it does is move every edge a fraction of
    /// a pixel a frame, which reads as a shimmer with no reconstruction behind
    /// it. It is built ahead of its consumer because the constraint it has to
    /// satisfy - not perturbing the culling frustum or the velocity buffer - is
    /// a property of *where* the offset is applied, and that is far cheaper to
    /// get right now than to retrofit under a temporal upscaler that is already
    /// ghosting.
    ///
    /// A flag rather than a `[graphics]` key deliberately: a settings key
    /// implies a choice a player should be making, and there is no version of
    /// this that a player wants until something reconstructs from it. The real
    /// gate, when it exists, is "is a temporal upscaler selected".
    ///
    /// Two `--screenshot` runs differing only by this differ by well under a
    /// pixel - invisible in a window, obvious in a byte diff, which is how to
    /// check it is live. Only the race's 3D scene moves: the HUD, the reticle
    /// and the menus are all drawn at presentation resolution and are not
    /// jittered. See `oag_post::jitter`.
    #[arg(long)]
    pub(crate) camera_jitter: bool,

    /// How many samples the rasterizer takes: off or 4x.
    ///
    /// Overrides `[render_profiles.<title>] msaa` for *every* title, for this
    /// run only; the file on disk is not changed - see `--reconstruction`'s own
    /// doc for why every title rather than one. Here for the same reason
    /// `--reconstruction` is: two `--presented` captures differing only by this
    /// flag are how the levels get compared.
    #[arg(long)]
    pub(crate) msaa: Option<oag_display::display::Msaa>,

    /// `--motion-blur` and `--motion-blur-resolution` - see [`extra::BlurArgs`].
    #[command(flatten)]
    pub(crate) blur: extra::BlurArgs,

    /// What casts a shadow: off or blob.
    ///
    /// Overrides `[render_profiles.<title>] shadows` for *every* title, for
    /// this run only; the file on disk is not changed - see
    /// `--reconstruction`'s own doc for why every title rather than one. Here
    /// for the reason `--motion-blur` is, and for one more: the tier has no
    /// menu row yet, so until it has one this flag is the only way to select
    /// it, and two captures differing only by it are how it gets compared
    /// against itself off.
    #[arg(long)]
    pub(crate) shadows: Option<oag_display::display::Shadows>,

    /// Which screen filter draws the finished frame: `off`, a built-in such
    /// as `psp-3000` or `crt-interlaced`, or the stem of a `.wgsl` file in
    /// `<config dir>/oag/shaders/`.
    ///
    /// Overrides `[render_profiles.<title>] screen_filter` for *every* title,
    /// for this run only, the way `--shadows` does. **Only visible under
    /// `--presented`** in a capture, for the reason the grade is: an ordinary
    /// `--screenshot` is the scene as drawn. See
    /// `docs/rendering/screen-filters.md`.
    #[arg(long, value_name = "ID")]
    pub(crate) screen_filter: Option<String>,

    /// Start the craft at this world position instead of on its grid slot:
    /// `x,y,z` or `x,y,z,yaw`, `yaw` in degrees off the track's own direction
    /// there.
    ///
    /// **A capture aid, not a spawn.** Two circuits photographed from the same
    /// place, or one of our frames lined up with a position read out of an
    /// emulator's debugger, instead of running a guessed number of ticks and
    /// comparing whatever comes up. The position is used exactly as given -
    /// nothing lifts or snaps it - while attitude comes from the nearest spline
    /// sample, so a banked corner or a loop reads right rather than leaving the
    /// craft flat inside the geometry.
    #[arg(long, value_name = "X,Y,Z[,YAW]")]
    pub(crate) pose: Option<String>,

    /// Start the craft at a captured trace row's exact pose: position and full
    /// basis from the CSV, nothing recomputed from the spline.
    ///
    /// The other half of the comparison `--pose` was built for: an emulator
    /// frame and one of ours, from the same state. When the capture carries
    /// camera columns (`psp-trace.py --camera`), the frame is also rendered
    /// from the recorded camera pose; without them the chase camera frames the
    /// shot as usual.
    #[arg(long, value_name = "TRACE.CSV", conflicts_with = "pose")]
    pub(crate) pose_from: Option<std::path::PathBuf>,

    /// Which tick of `--pose-from` to take the pose off.
    #[arg(long, default_value_t = 0, requires = "pose_from")]
    pub(crate) pose_tick: u64,

    /// With `--pose-from`: ignore the capture's camera columns and keep the
    /// chase camera.
    #[arg(long, requires = "pose_from")]
    pub(crate) no_camera: bool,

    /// With `--pose-from`: render the recorded camera at this fov instead of
    /// the disc's authored value, in **vertical degrees** - the authored
    /// value's own unit, settled at confidence 94 on 2026-08-09.
    ///
    /// It was the calibration knob for settling that unit. It is now a
    /// *measurement instrument* rather than a leftover: `place_at` resets the
    /// body, so a posed craft has zero velocity and never receives the
    /// original's speed-dependent widen. **Every matched-pose comparison needs
    /// this flag**, set to `60 + 0.075 * dot(fwd, vel)` for the captured tick.
    /// See `docs/rendering/projection-vs-the-original.md`.
    #[arg(long)]
    pub(crate) camera_fov: Option<f32>,

    /// Leave out a Pulse hull's `0x2000` environment-mapped extra pass
    /// (`oag_render::shine`): the headless way to measure what the pass adds,
    /// by rendering the same frame with and without it.
    #[arg(long)]
    pub(crate) no_hull_shine: bool,

    /// Leave out a Pulse circuit's own extra pass: its `*_shinemap` batches
    /// under their chrome map, which `oag_render::shine::build_track` draws.
    #[arg(long)]
    pub(crate) no_track_shine: bool,

    /// Render from a camera given outright, as nine comma-separated numbers:
    /// `eye_x,eye_y,eye_z,fwd_x,fwd_y,fwd_z,up_x,up_y,up_z`.
    ///
    /// **The consumer of an RPCS3 capture.** `scripts/rpcs3-drive.py capture`
    /// pairs a screenshot with the `viewProj` the original's own shaders were
    /// fed, and `scripts/ps3_pose.py` decomposes that matrix into exactly these
    /// nine numbers plus a field of view. Passing them here puts this renderer
    /// at the original's camera, so the two frames can be laid over each other
    /// instead of eyeballed at approximately the same place.
    ///
    /// `--camera-fov` is what carries the tenth number, and a matched-pose
    /// comparison wants both. Unlike the other pose flags this one needs no
    /// `--pose-from`: a captured camera is a complete statement on its own.
    #[arg(long, value_name = "EYE,FWD,UP", conflicts_with = "pose_from")]
    pub(crate) camera_pose: Option<String>,

    /// With `--screenshot`: force the exhaust into the state it would hold this
    /// many seconds after entering a speed pad, at saturated intensity.
    ///
    /// The frame-comparison knob for the boost visuals: a posed capture
    /// (`--pose-from --ticks 0`) never crosses a pad, so without this the flare
    /// renders cold and the plume not at all, and there is nothing to compare
    /// against an emulator shot taken mid-boost. `0.0` is the entry tick.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) pose_boost: Option<f32>,

    /// With `--pose-boost`: the intensity the exhaust had when the pad was
    /// entered, instead of saturating it.
    ///
    /// **Saturated is the wrong default for a comparison against a teleported
    /// capture, and this is what fixes it.** `Exhaust_Update`'s intensity ramp
    /// climbs at only `0.25`/s, so it needs four seconds of thrust to reach
    /// `1.0`; a craft put on a pad's approach by `psp-drive.py place` has had
    /// far less than that, and the first real pad capture measured `0.1334` at
    /// the entry tick. Intensity is not cosmetic there - it sets the flare's
    /// resting half-size through `(i * 0.6 + 0.4) * 2.5` (`1.2` at `0.13`
    /// against `2.5` saturated) and all three of the ribbon's staggered layer
    /// alphas - so comparing a saturated render against an unsaturated capture
    /// measures the difference in *state*, not in rendering.
    ///
    /// Take the value from a `psp-trace.py --flare` capture's `intensity`
    /// column at the entry tick. The ramp then keeps climbing through the
    /// posed age at the same rate the original's does, so one number matches
    /// the whole curve rather than just its start.
    #[arg(long, value_name = "0..1", requires = "pose_boost")]
    pub(crate) pose_intensity: Option<f32>,

    /// With `--pose-boost`: the speed in units/s to advance the exhaust at,
    /// instead of the default racing `120`.
    ///
    /// Only reaches the picture through `Exhaust_Update`'s speed ramp, which
    /// floors the boost accumulator at `clamp((kmh - 100) / 500) * 0.6` - so it
    /// matters when a capture's speed is far from `120`. Take it from the
    /// capture's `speed` column.
    #[arg(long, value_name = "UNITS_PER_S", requires = "pose_boost")]
    pub(crate) pose_speed: Option<f32>,

    /// With `--race --screenshot` or `--race --trace-out`: drive the run from
    /// a committed `.inputs` script (see `scripts/input_script.py` for the
    /// format) instead of `--hold`/`--press` - the same file
    /// `scripts/psp-trace.py --script` feeds the emulator, so one authored
    /// input produces both sides of a visual or CSV comparison. Ticks past
    /// the script's end hold its last state.
    ///
    /// **`--trace-out` did not read this at all before 2026-09-10** - a run
    /// combining the two produced a plausible-looking CSV of a craft that
    /// never moved, with no error. `HeldButtons::advance` is where both legs
    /// now make the same choice, so they cannot diverge on it again.
    #[arg(long, value_name = "FILE.inputs", requires = "race")]
    pub(crate) input_script: Option<std::path::PathBuf>,

    /// Which of the three in-race camera perspectives to fly with, overriding
    /// `[graphics] camera_view` for this run: `internal`, `close` or `far`.
    ///
    /// In-game the choice is cycled with SELECT (TAB on a keyboard) and persisted,
    /// so this flag exists for the case a keypress cannot reach: a **headless
    /// `--screenshot`**, which is the only way to get a frame of the cockpit view
    /// on a machine that never opens a window. Unlike the settings row, it does not
    /// persist.
    ///
    /// Gated on `--screenshot` the way `--camera-fov` is gated on `--pose-from`,
    /// and for the same reason: a windowed run reads the view from the settings
    /// file and cycles it from the button, so a flag accepted there would be
    /// silently dropped. Refusing it says so instead.
    #[arg(long, value_name = "VIEW", requires = "screenshot")]
    pub(crate) camera_view: Option<display::CameraView>,

    /// With `--race`: write our per-tick state to a CSV in the capture's own
    /// columns, so it can be differenced against one from
    /// `scripts/psp-trace.py`.
    ///
    /// **Simulation only** - no window, no GPU, no screenshot. The point is to
    /// stop verifying the exhaust by looking at pictures: `oag-trace compare
    /// data/traces/pad0-boost.csv ours.csv` puts a number on every column
    /// instead. Writes the eight `--flare` columns beside the ship state, so a
    /// capture taken with `psp-trace.py --flare` compares on all of them.
    ///
    /// Combine with `--input-script` to drive it from an authored `.inputs`
    /// file instead of `--hold`/`--press` - a second, non-rendering way to
    /// get a scripted run's full per-tick basis, more convenient than
    /// `oag-trace run` for anyone already working against this binary's own
    /// load path rather than a captured reference trace.
    ///
    /// Combining it with `--screenshot` is refused rather than ignored: the
    /// screenshot route renders through `oag_game::race_capture::capture`, and threading a writer
    /// into that loop is a change to a file this flag deliberately does not
    /// touch. Run the two separately.
    #[arg(long, value_name = "FILE.csv", conflicts_with = "screenshot")]
    pub(crate) trace_out: Option<std::path::PathBuf>,

    /// `--write-icon` and `--icon-size` - see [`extra::IconArgs`].
    #[command(flatten)]
    pub(crate) icon: extra::IconArgs,

    /// `--ghost` and `--record-ghost` - see [`extra::GhostArgs`].
    #[command(flatten)]
    pub(crate) ghost: extra::GhostArgs,

    /// `--measure-race-load` - see [`extra::MeasureArgs`].
    #[command(flatten)]
    pub(crate) measure: extra::MeasureArgs,

    /// `--force-wreck` and `--no-hull-wreck` - see [`extra::WreckArgs`].
    #[command(flatten)]
    pub(crate) wreck: extra::WreckArgs,
    #[command(flatten)]
    pub(crate) intro: extra::IntroArgs,
    #[command(flatten)]
    pub(crate) log: oag_log::tool::LogArgs,
}
