//! The command line: every flag `oag-game` takes, and nothing else.
//!
//! Its own module because it is a third of what `main.rs` used to be, and it
//! is pure declaration - `clap` derives the parser, and every consumer
//! (`main`, [`crate::headless`], [`crate::args`]) reads fields off it.

use clap::Parser;

use oag_game::display;
use oag_gameplay::ControlScheme;
use oag_render::mesh_render::Anisotropy;

use crate::args::DEFAULT_SIZE;

#[derive(Parser, Debug)]
#[command(
    name = "oag-game",
    about = "Run Wipeout Pulse from a disc image",
    version
)]
pub(crate) struct Cli {
    /// A disc image, or a directory extracted with `oag-unpack`.
    ///
    /// Either release: the archives are found by name, so
    /// `data/images/pulse-ps2-eu.chd` works as well as the PSP default. See
    /// `oag_assets::Layout`.
    ///
    /// Left out, it is searched for: `data/images/` in the current directory,
    /// then beside the AppImage, then `<data dir>/oag/images`. `oag_game::source`
    /// documents the whole order, and `$OAG_IMAGE` short-circuits it. Finding
    /// more than one there opens the chooser - see `--launcher`.
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
    /// next run picks up whatever finished. See `oag_game::prefetch`.
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

    /// With `--screenshot`, the image's size as `WIDTHxHEIGHT`.
    ///
    /// A window is not always given the size it asks for - a tiling compositor
    /// hands out whatever its layout has - and the field of view is derived from
    /// the viewport, so this is how a capture can show what a differently shaped
    /// window would have drawn.
    #[arg(long, default_value = DEFAULT_SIZE)]
    pub(crate) size: String,

    /// With `--screenshot`, run until this state is current before capturing.
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

    /// Load the menu tree from this file instead of the one built into the
    /// binary.
    ///
    /// For editing `assets/ui/menu.toml` without a rebuild. Checked the same
    /// way the built-in one is, so a mistake in it is a startup error.
    #[arg(long, value_name = "FILE")]
    pub(crate) menu: Option<std::path::PathBuf>,

    /// With `--screenshot`, draw one page of our own menus instead of the
    /// sequence: a page id from `assets/ui/menu.toml`.
    ///
    /// For looking at a layout without launching the game and walking to it.
    /// Like `--screen`, it takes no input and runs no state machine.
    #[arg(long, value_name = "PAGE")]
    pub(crate) menu_page: Option<String>,

    /// With `--menu-page`, draw that page part-way through arriving.
    ///
    /// `0` is the instant a page change starts and `1` is the end of it. A
    /// still cannot otherwise show a transition at all: `--menu-page` runs no
    /// clock, and `--ticks` does nothing alongside it, so without this the only
    /// way to look at the effect is to play the game and watch.
    ///
    /// Shows the *arriving* half only. The page being left is whatever the
    /// player came from, which a one-page capture has no way to know.
    #[arg(long, value_name = "0..1")]
    pub(crate) menu_anim_phase: Option<f32>,

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

    /// Report what was loaded and exit, without rendering anything.
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
    /// See `oag_game::remix::Remix`.
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

    /// A directory holding downloadable content: a pack's `.edat` files, or the
    /// `.zip` they were downloaded as. Repeatable.
    ///
    /// Left out, it is searched for: `data/dlc/` in the current directory, then
    /// beside the AppImage, then `<data dir>/oag/dlc`. `$OAG_DLC` short-circuits
    /// that, and `oag_game::source::resolve_dlc` documents the order.
    ///
    /// **Any pack works against any release.** The original locked a pack to
    /// its own territory's disc; this does not. See `docs/formats/dlc-pack.md`.
    #[arg(long)]
    pub(crate) dlc: Vec<String>,

    /// The speed class: venom, flash, rapier or phantom.
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

    /// Development override: force the Zone colour grade to a stage.
    ///
    /// **The only way to see HD/Fury's Zone look at all today**, because what
    /// advances the stage during a race is recovered on 2048 and not on HD, so
    /// an HD Zone race otherwise rests where its loader left it. The original
    /// is visibly *not* on stage 0 at a start line - Moa Therma's opens on
    /// `Sub Venom`'s cyan - so a frame compared against it needs this.
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
    /// `race::CaptureOptions::zone_spectrum_test`'s own doc comment.
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
    /// volley; see `oag_game::race::Race::rocket_model_matrices`).
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

    /// What an authored `LodGroup` draws: `both` children, the way the original
    /// does, or only the higher-detail `single` one.
    ///
    /// Overrides `[graphics] lod` in the settings file (`settings::path`) for
    /// this run only; the file on disk is not changed. Here for the same reason
    /// `--anisotropy` is: `both` means two differently-tessellated copies of the
    /// same surface occupy the same space, and two captures differing only by
    /// this flag are how you see what that costs.
    #[arg(long)]
    pub(crate) lod: Option<oag_render::mesh::Lod>,

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

    /// Which resampler carries the frame onto the surface: bilinear or fsr1.
    ///
    /// Overrides `[render_profiles.<title>] upscaler` for *every* title, for
    /// this run only; the file on disk is not changed. Every title rather than
    /// one, because no title is open yet this early to single one out - see
    /// `crate::main::render_scale` in `main.rs`. Here for the same reason
    /// `--anisotropy` is, and for one more: two `--presented` captures
    /// differing only by this flag are how the resamplers get compared, and
    /// asking somebody to edit a settings file between them is how a comparison
    /// ends up differing by something else as well.
    #[arg(long)]
    pub(crate) upscaler: Option<crate::display::Upscaler>,

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
    /// jittered. See `oag_render::jitter`.
    #[arg(long)]
    pub(crate) camera_jitter: bool,

    /// Anti-aliasing mode: off, fxaa, smaa or msaa4x.
    ///
    /// Overrides `[render_profiles.<title>] anti_aliasing` for *every* title,
    /// for this run only; the file on disk is not changed - see `--upscaler`'s
    /// own doc for why every title rather than one. Here for the same reason
    /// `--upscaler` is: two `--presented` captures differing only by this flag
    /// are how the modes get compared.
    #[arg(long)]
    pub(crate) anti_aliasing: Option<crate::display::AntiAliasing>,

    /// Motion blur strength: off, low, medium or high.
    ///
    /// Overrides `[render_profiles.<title>] motion_blur` for *every* title, for
    /// this run only; the file on disk is not changed - see `--upscaler`'s own
    /// doc for why every title rather than one. Here for the reason
    /// `--anti-aliasing` is: two captures differing only by this flag are how
    /// the blur gets compared against itself off, and a capture honours it by
    /// rendering a primer frame at the tick-before-last camera first - see
    /// `race::CaptureOptions::motion_blur`.
    #[arg(long)]
    pub(crate) motion_blur: Option<crate::display::MotionBlur>,

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

    /// With `--race --screenshot`: drive the run from a committed `.inputs`
    /// script (see `scripts/input_script.py` for the format) instead of
    /// `--hold`/`--press` - the same file `scripts/psp-trace.py --script`
    /// feeds the emulator, so one authored input produces both sides of a
    /// visual comparison. Ticks past the script's end hold its last state.
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
    /// Combining it with `--screenshot` is refused rather than ignored: the
    /// screenshot route renders through `race::capture`, and threading a writer
    /// into that loop is a change to a file this flag deliberately does not
    /// touch. Run the two separately.
    #[arg(long, value_name = "FILE.csv", conflicts_with = "screenshot")]
    pub(crate) trace_out: Option<std::path::PathBuf>,

    /// Rasterise `assets/icons/64x64.svg` to a PNG at `--icon-size` and exit.
    ///
    /// Touches no disc, no settings file and no window - only `oag_game::icon`.
    /// What `just install-desktop-file` and `scripts/build-appimage.sh` both
    /// call, so the icon on disk and the one `main/window.rs` hands winit at
    /// startup come from the same rasterizer and never drift apart:
    ///
    /// ```sh
    /// cargo run -p oag-game -- --write-icon /tmp/oag-game.png --icon-size 256
    /// ```
    #[arg(long, value_name = "FILE.png")]
    pub(crate) write_icon: Option<std::path::PathBuf>,

    /// Edge length in pixels for `--write-icon`.
    ///
    /// The source is a vector, so any size re-renders cleanly rather than
    /// upscaling a raster; 256 is what the freedesktop hicolor icon theme
    /// wants for `apps/`.
    #[arg(long, requires = "write_icon", default_value_t = 256)]
    pub(crate) icon_size: u32,
}

impl Cli {
    /// Applies `--render-scale`, `--upscaler`, `--anti-aliasing` and
    /// `--motion-blur` to one render profile.
    ///
    /// **One function because there are two callers and they disagreed.**
    /// `main.rs` applies these by walking `settings.render_profiles`, which is
    /// keyed by title - and a headless `--race` capture has no title in hand,
    /// so it built a `RenderProfile::default()` instead and every one of these
    /// four flags was silently discarded. That made `just compare-upscalers`
    /// produce three byte-identical images, which is exactly the failure a
    /// comparison instrument cannot survive: it reported "these upscalers look
    /// the same" and nobody could tell that from "the flag did nothing".
    ///
    /// `render_scale` is parsed by the caller because it is the one of the four
    /// that can fail.
    pub(crate) fn apply_render_overrides(
        &self,
        profile: &mut crate::settings::RenderProfile,
        render_scale: Option<crate::display::Scale>,
    ) {
        if let Some(render_scale) = render_scale {
            profile.render_scale = render_scale;
        }
        if let Some(upscaler) = self.upscaler {
            profile.upscaler = upscaler;
        }
        if let Some(anti_aliasing) = self.anti_aliasing {
            profile.anti_aliasing = anti_aliasing;
        }
        if let Some(motion_blur) = self.motion_blur {
            profile.motion_blur = motion_blur;
        }
    }
}
