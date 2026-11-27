//! Flag groups `clap` flattens into [`super::Cli`], each its own struct so
//! `cli.rs` stays under the size ratchet.

use anyhow::Context as _;
use std::path::PathBuf;

/// Writing the window icon.
#[derive(clap::Args, Debug)]
pub(crate) struct IconArgs {
    /// Rasterise `assets/icons/64x64.svg` to a PNG at `--icon-size` and exit.
    ///
    /// Touches no disc, no settings file and no window - only `crate::icon`.
    /// What `just install-desktop-file` and `scripts/build-appimage.sh` both
    /// call, so the icon on disk and the one `app/window.rs` hands winit at
    /// startup come from the same rasterizer and never drift apart:
    ///
    /// ```sh
    /// cargo run -p oag-game -- --write-icon /tmp/oag-game.png --icon-size 256
    /// ```
    #[arg(long, value_name = "FILE.png")]
    pub(crate) write_icon: Option<PathBuf>,

    /// Edge length in pixels for `--write-icon`.
    ///
    /// The source is a vector, so any size re-renders cleanly rather than
    /// upscaling a raster; 256 is what the freedesktop hicolor icon theme
    /// wants for `apps/`.
    #[arg(long, requires = "write_icon", default_value_t = 256)]
    pub(crate) icon_size: u32,
}

/// Racing and recording a ghost in a headless `--race --screenshot` capture.
///
/// **Explicit files, never the config directory.** A capture reads
/// `records.toml` and never writes it (see `crate::race_capture::CaptureOptions::
/// previous_best`); a ghost is the same: `--ghost` names the file to race and
/// `--record-ghost` the file the run's best lap goes to, so a scripted capture
/// cannot overwrite a player's own best. See ADR-0055.
#[derive(clap::Args, Debug)]
pub(crate) struct GhostArgs {
    /// Race the ghost lap stored in this replay file.
    ///
    /// The file a session writes under `<config dir>/oag/ghosts/`, or one an
    /// earlier `--record-ghost` wrote. Its key is not checked against this
    /// race's - naming a file is the intent.
    #[arg(long, value_name = "FILE.oagr")]
    pub(crate) ghost: Option<PathBuf>,

    /// Record this run and write its best lap, as a ghost, to this file.
    ///
    /// Written once, after the last tick, when the run completed a lap -
    /// whatever is already there.
    ///
    /// ```sh
    /// cargo run -p oag-game -- --race --mode time_trial --autopilot \
    ///     --ticks 9000 --record-ghost lap.oagr --screenshot /dev/null
    /// ```
    #[arg(long, value_name = "FILE.oagr")]
    pub(crate) record_ghost: Option<PathBuf>,
}

/// Timing the loading screen's hand-off to a race, in a window.
#[derive(clap::Args, Debug)]
pub(crate) struct MeasureArgs {
    /// Skip the boot sequence, launch this many races from the menus one
    /// after another, print every main-thread frame of each load-to-race
    /// transition that went over a 60 Hz frame's budget, and quit.
    ///
    /// The windowed route a player takes - the menus' own `LAUNCH RACE`, the
    /// loading screen, the hand-off - rather than `--race`, which loads before
    /// the window opens and never shows the loading screen at all. The first
    /// run pays a cold disc cache and the second does not, so asking for two
    /// separates the two. Measure a release build: a debug one is several
    /// times slower on exactly the work this times.
    ///
    /// ```sh
    /// cargo run --release -p oag-game -- data/images/pulse-psp-eu.chd \
    ///     --no-audio --measure-race-load 2
    /// ```
    #[arg(long, value_name = "RUNS", num_args = 0..=1, default_missing_value = "1")]
    pub(crate) measure_race_load: Option<u32>,
}

/// Looking at one of our own menus without launching the game and walking
/// to it: `--menu`, `--menu-page`, `--menu-anim-phase`, `--menu-picker-seconds`
/// and `--menu-prompt`.
#[derive(clap::Args, Debug)]
pub(crate) struct MenuArgs {
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

    /// Which glyphs the button prompts draw: `auto`, `original`, `playstation`,
    /// `xbox`, `nintendo` or `keyboard`.
    ///
    /// Overrides `[controls] prompt_style` for this run without writing it
    /// back. `auto` follows the device last used.
    #[arg(long)]
    pub(crate) prompt_style: Option<String>,

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

    /// With `--menu-page track-select`/`ship-select`, draw the screen this
    /// many seconds after it opened, instead of settled.
    ///
    /// The race box's own two selection screens are not `assets/ui/menu.toml`
    /// pages, so `--menu-anim-phase` does not reach them - they run neither
    /// the page tween nor `--ticks`. This is their equivalent: `0.0` is the
    /// instant the screen opens (the info panel and the hexagonal window's
    /// stills not yet faded in), and by `0.5` both are settled - the
    /// `LeftLayer transition` the screen's own XML authors
    /// (`docs/ui/selection-screens.md`). `None` draws it settled, the same
    /// rule `--menu-anim-phase` follows.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) menu_picker_seconds: Option<f32>,

    /// With `--menu-page ship-select`/`track-select`/`grid-select`/`cell-select` on HD/Fury or
    /// Omega, draw the page this many seconds after it became current, for its `vwipe` images.
    ///
    /// Each waits its authored `delay` from the page's entry and then ramps over its in-time
    /// (`docs/ghidra/functions/ps3-hdfury-eu/image-widget.md`, "Wired: page entry"), so `0.6`
    /// shows Team Selection's logo (0.5 s delay) a slab into its wipe. `None` draws every one
    /// finished, which is what a still of the settled page is. Apart from
    /// `--menu-picker-seconds`, which a pair script sets to the seconds that turn the craft to
    /// the original's turntable angle: that is not the page's age.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) menu_entry_seconds: Option<f32>,

    /// With `--menu-page ship-select` on a title that draws the race hull,
    /// draw the screen this many seconds after a selection step.
    ///
    /// HD/Fury cuts the old craft, shows an empty frame for about 60 ms and
    /// cuts the new one in, which then eases to rest over about 0.5 s
    /// (`docs/ui/selection-screens.md`, "The craft's swap"). A still has no
    /// step to be after, so this stands for one: the screen is otherwise
    /// settled, or `--menu-picker-seconds` after it opened. `None` draws no step.
    ///
    /// Also `--menu-page track-select` (the circuit `Emblem`'s wipe restarts at a step) and, on
    /// HD/Fury and Omega, `--menu-page cell-select`, where it is the seconds since the cell or the
    /// rung last changed, which re-arms the four emblems' wipes; the page is then
    /// `--menu-entry-seconds` old, or 10 s old with none.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) menu_picker_step_seconds: Option<f32>,

    /// With `--menu-page track-select`/`ship-select`, the previewed model's
    /// own animation time in seconds, where it is not the time since the screen
    /// opened: Pulse's `ship_FE.vex` sways on an `Anim Transform` that
    /// started when the model loaded, not when the screen did. `None` takes
    /// `--menu-picker-seconds`.
    ///
    /// With `--menu-page grid-select`/`cell-select` on HD/Fury it is the flyer
    /// card's: its arrows and layers slide on the card's own clock, which
    /// the widget keeps in `[3, 6)` seconds (a time reaching `6.0` goes back by
    /// 3, `Flyer_Item`'s render at `0x001a2510`), so no two grabs of the original
    /// show the same one. `scripts/rpcs3-hd-menu-pair.py` reads the clock out of
    /// RAM at the grab and passes it; `None` draws the card at 3.0.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) menu_model_seconds: Option<f32>,

    /// With `--menu-page` on a title with a footer ticker, draw this one message instead of the
    /// rotation: `ITEM@X`, the `PI_News` item's id (`TKR_FAVMUS`) and where the message's head
    /// is, in pixels from the ticker's left edge (the original's own coordinate: a message
    /// enters at the ticker's width and moves left a pixel a frame). A still has no moment in
    /// a rotation drawn at random, so this states the one the original's frame caught; the
    /// message is drawn from the profile's own lifetime statistics.
    #[arg(long, value_name = "ITEM@X")]
    pub(crate) menu_ticker: Option<String>,

    /// With `--menu-page grid-select`/`cell-select`/`track-select` on Pulse, the seconds the card title's
    /// (or the circuit name panel's) glow ladder (`pulse="true"` texts, 1 s period) has run, where it is
    /// not `--menu-picker-seconds`: the text's counter and the `Selector`'s are separate in the original,
    /// so a pair script pins each to what it read. `None` follows `--menu-picker-seconds`.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) menu_title_seconds: Option<f32>,

    /// With `--menu-page ship-select`/`track-select` on HD/Fury or Omega, where the hex grid's ring
    /// pulse stands, in seconds of the widget's own clock (the phase runs 3 a second and wraps at
    /// 2), where it is not `--menu-picker-seconds`: `HexSelection` counts its own updates from
    /// its first build and the original never resets the count between visits
    /// (`docs/ghidra/functions/ps3-hdfury-eu/ship-select-ring.md`, "The pulse's zero point"), so a
    /// pair script reads the phase out of RAM at the grab and passes it. `None` follows
    /// `--menu-picker-seconds`.
    #[arg(long, value_name = "SECONDS")]
    pub(crate) menu_ring_seconds: Option<f32>,

    /// With `--menu-page`, where the focused row's colour pulse stands, `0` at the unselected ink
    /// and `1` at the peak: a still has no clock, so the pulse rests at `0`, and a pair script reads
    /// the level the original's frame caught off the focused row's own ink. `None` rests at `0`.
    #[arg(long, value_name = "LEVEL")]
    pub(crate) menu_focus_level: Option<f32>,

    /// With `--menu-page`, the frame of the disc's looping menu backdrop movie
    /// to draw behind the page, counting from zero; past the end it wraps.
    ///
    /// The movie runs from boot and never restarts, so where a reference still
    /// caught it depends on how long the walk took: the streaks and the craft
    /// that fly past are bright for about 0.3 s of the nine-second loop.
    /// `scripts/compare-menu-movie-phase.py` finds the frame a still shows.
    /// `None` draws frame zero. No effect on a title with no backdrop movie, except on
    /// HD's `track-select`, where it is the frame of the selected circuit's fly-by clip
    /// (`scripts/compare-menu-movie-phase.py --fly-by-clip` finds it).
    #[arg(long, value_name = "FRAME")]
    pub(crate) menu_movie_frame: Option<usize>,

    /// With `--menu-page`, draw a modal prompt over it: `rename`,
    /// `rename-note`, `delete`, `delete-built-in` or `binding`.
    ///
    /// The same argument `--menu-anim-phase` makes, one step stronger. A
    /// prompt is on screen because a row was **activated**, and this path runs
    /// no state machine and calls no `Menu::update` - so the on-screen
    /// keyboard can never appear here on its own, and without this flag its
    /// layout is reviewable only by playing the game on a machine that has a
    /// display. The models drawn are the live ones (`oag_ui_screens::prompt`) and
    /// the labels come out of the same string table `session::pilot_editor`
    /// resolves, so this is the real screen rather than a mock-up of it.
    ///
    /// `rename-note` and `delete-built-in` are the two worst cases for the
    /// layout: the live note under the buffer, and the longest message
    /// anything asks a player to read. `binding` is the CONTROLS page's own
    /// key-capture prompt - `--menu-page controls --menu-prompt binding`
    /// draws it over whichever row's `button` its own page opens on first,
    /// since a still has no selected row to prefer over another.
    ///
    /// `tag-entry`/`tag-entry-typed` draw Pulse's own `TagInput` cell row
    /// instead of `rename`'s grid - the shape `session::pilot_editor` opens
    /// when the disc's own alphabet can spell the pilot's name, see
    /// `docs/formats/fexml.md`'s `TagInput` section. Needs a real `--race`
    /// source open, since it reads the row's geometry live off the disc;
    /// `-typed` is the same screen after a few glyph changes.
    ///
    /// `ffmpeg` draws the offer to download `ffmpeg` and `ffmpeg-progress` the status line over
    /// the menus while it runs (`session::ffmpeg`).
    ///
    /// `player-name` and `player-tag` draw the first-run entries on Pulse's own `TagInput` row
    /// (needs a Pulse source); `player-name-grid` and `player-tag-grid` draw the grid every other
    /// title gets.
    #[arg(long, value_name = "PROMPT", requires = "menu_page")]
    pub(crate) menu_prompt: Option<String>,
}

/// Capturing a wrecked craft in a headless `--race --screenshot` run.
#[derive(clap::Args, Debug)]
#[command(group(clap::ArgGroup::new("forced_rocket").multiple(true)))]
pub(crate) struct WreckArgs {
    /// With `--race --screenshot`, put one craft into the destroyed sequence at
    /// the end of a tick: `TICK:SLOT`, e.g. `60:3` and then `--ticks 100` for a
    /// wreck about half a second old.
    ///
    /// A verification aid, there because a wreck is hard to produce at a chosen
    /// place and moment. It enters the state `Ship_Damage`'s depletion does, so
    /// the sequence that follows is the simulation's own. See
    /// [`oag_raceplay::RaceSim::force_destroy`].
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_wreck: Option<String>,

    /// With `--race --screenshot`, land a weapon hit on one craft at the end of
    /// a tick: `TICK:SLOT`, e.g. `10:0`. A verification aid for what a struck
    /// hull shows (hit sparks, HD's damage smoke), there because a headless run
    /// has no rival whose weapon reaches the player. The hit spends no shield
    /// and takes none; pair it with `--force-shield` for the shield it should
    /// find. On a craft with a running Shield pickup it is swallowed: the shell
    /// flashes and no sparks are thrown. See
    /// [`oag_raceplay::Race::force_weapon_hit`].
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_hit: Option<String>,

    /// With `--race --mode detonator --screenshot`, set the Detonator score at the end of a
    /// tick: `TICK:SCORE`, e.g. `60:200000`. A verification aid for what the HUD shows as the
    /// score passes a campaign cell's targets (with `--campaign-cell`), there because a headless
    /// run cannot shoot a campaign cell's tens of thousands of points.
    #[arg(long, value_name = "TICK:SCORE", requires = "race")]
    pub(crate) force_detonator_score: Option<String>,

    /// With `--race --screenshot`, throw one craft sideways to its right at
    /// 600 units a second at the end of a tick: `TICK:SLOT`, e.g. `60:1`. A verification aid
    /// for a seat's wall sparks, shake and contact sound, there because a
    /// headless run flies no person into a wall (it meets one where the circuit has a wall on
    /// that side). The wall contact that follows
    /// is the simulation's own. See [`oag_raceplay::RaceSim::force_wall_slide`].
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_wall_slide: Option<String>,

    /// With `--race --screenshot`, put a locked LeachBeam from the player onto
    /// a rival at the end of a tick: `TICK:SLOT`, e.g. `300:1`. A verification
    /// aid for the LeachBall's own draw, there because a headless run has no
    /// rival in the beam's cone. Writes the world the way `--force-wreck`
    /// does, from outside `Race::tick`.
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_leach_lock: Option<String>,

    /// With `--force-leach-lock`, draw HD's LeachBeam strip along a path read
    /// off the original instead of the one this race walks: a text file of
    /// `x,y,z` lines, the walked (target) end first, as the original's strip
    /// object stores its samples (`scripts/rpcs3-hd-whiteout.py --leach`
    /// dumps them). With `--pose-from`/`--camera-pose` and `--camera-fov` set
    /// to the dumped frame's own, the frame isolates this renderer's draw of
    /// the strip from the path. A verification aid only.
    #[arg(long, value_name = "FILE", requires = "force_leach_lock")]
    pub(crate) leach_path: Option<std::path::PathBuf>,

    /// With `--race --screenshot`, put craft `SLOT` on a Bomb the player has laid
    /// at the end of a tick: `TICK:SLOT`, e.g. `600:1`. A verification aid for
    /// the Bomb's detonation, there because a headless run has no rival that
    /// drives over it. The next tick's own trip test does the rest. See
    /// [`oag_raceplay::RaceSim::force_bomb_trip`].
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_bomb_trip: Option<String>,

    /// With `--race`, leave the weapons' point lights (Missile explosion, Rocket,
    /// Bomb blast) out of Wipeout HD's SPU vertex-light list, so a picture can be
    /// read for the model alone. See [`oag_raceplay::Race::set_weapon_lights`].
    #[arg(long, requires = "race")]
    pub(crate) no_weapon_lights: bool,

    /// With `--race`, start the particle stage's random stream from `SEED`, so a test
    /// can read an effect over several arrangements. See
    /// [`oag_raceplay::Race::reseed_stage`].
    #[arg(long, value_name = "SEED", requires = "race")]
    pub(crate) stage_seed: Option<u64>,

    /// With `--race --screenshot`, hand craft `SLOT` to a scripted remote person
    /// at a tick: `TICK:SLOT[:WINDOW[:contact][:fire][:own[=UNITS]]]`, e.g. `480:7:20:contact`. The client guesses its
    /// input straight on for `WINDOW` ticks (default 6, 100 ms) while the true
    /// taps its airbrake twice toward the player (a sideshift; with `contact` the player dashes at it too), then the server's state arrives and the
    /// client reconciles; the frame at `--ticks` is drawn on the client. A
    /// verification aid for what a correction looks like. With `fire` the true
    /// person instead fires a Rocket on the second tick, which the guess never sees. With `own[=UNITS]` (3 units by default) the server's copy of the player's own craft is that far to its right, a contact the guess missed on the craft the camera follows. See
    /// [`oag_raceplay::correction::Scene`].
    #[arg(
        long,
        value_name = "TICK:SLOT[:WINDOW[:contact][:fire][:own[=UNITS]]]",
        requires = "race"
    )]
    pub(crate) force_remote_correction: Option<String>,

    /// With `--force-remote-correction`, draw the correction where it lands
    /// instead of gliding it out. See [`oag_raceplay::Race::set_correction_smoothing`].
    #[arg(long, requires = "force_remote_correction")]
    pub(crate) no_correction_smoothing: bool,

    /// With `--race --screenshot`, put craft `SLOT` on a Missile the player has
    /// fired at the end of a tick: `TICK:SLOT`, e.g. `330:1`. The same aid
    /// `--force-bomb-trip` is, for the Missile's explosion: a headless run's
    /// rivals drive away from the shot. The next tick's own hit test does the
    /// rest. See [`oag_raceplay::RaceSim::force_missile_hit`].
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_missile_hit: Option<String>,

    /// With `--race --screenshot`, put craft `SLOT` on a Rocket the player has
    /// fired at the end of a tick: `TICK:SLOT`, e.g. `294:1`. The same aid
    /// `--force-missile-hit` is, for the Rocket's craft explosion. See
    /// [`oag_raceplay::RaceSim::force_rocket_hit`].
    #[arg(
        long,
        value_name = "TICK:SLOT",
        requires = "race",
        group = "forced_rocket"
    )]
    pub(crate) force_rocket_hit: Option<String>,

    /// With `--race --screenshot`, move the player's first weapon out (a laid Bomb, a
    /// Missile in flight) to a world point at the start of a tick, before
    /// `--force-bomb-trip` or `--force-missile-hit` on the same tick sets it off:
    /// `TICK:X,Y,Z`. The blast then stands where a capture of the original had it. A
    /// verification aid. See [`oag_raceplay::RaceSim::force_shot_at`]. Dev only.
    #[cfg(feature = "dev-cli")]
    #[arg(
        long,
        value_name = "TICK:X,Y,Z",
        requires = "race",
        allow_hyphen_values = true
    )]
    pub(crate) force_shot_at: Option<String>,

    /// With `--race --screenshot`, at the end of a tick play `COUNT` Rocket craft blasts (1 by
    /// default) at a world point and take as many of the player's Rockets out of the air:
    /// `TICK:X,Y,Z[:COUNT]`, e.g. `709:525.15,-28.26,-34.80`. Repeat it for a volley that lands
    /// in waves. The blasts stand where a capture of the original put them, which
    /// `--force-rocket-hit` cannot: it lands every Rocket at one point. No craft is struck. A
    /// verification aid. See [`oag_raceplay::Race::force_rocket_hit_at`]. Dev only.
    #[cfg(feature = "dev-cli")]
    #[arg(
        long,
        value_name = "TICK:X,Y,Z[:COUNT]",
        requires = "race",
        group = "forced_rocket"
    )]
    pub(crate) force_rocket_hit_at: Vec<String>,

    /// With `--force-rocket-hit`, give the struck craft this velocity as it is put on the Rocket;
    /// with `--force-rocket-hit-at`, have its blasts carry it. In world units a second. The craft
    /// blast carries the struck craft's velocity, so a comparison with a capture needs the
    /// captured craft's. See [`oag_raceplay::Race::force_struck_velocity`]. Dev only.
    #[cfg(feature = "dev-cli")]
    #[arg(
        long,
        value_name = "VX,VY,VZ",
        requires = "forced_rocket",
        allow_hyphen_values = true
    )]
    pub(crate) force_rocket_hit_velocity: Option<String>,

    /// With `--pose-hold-ticks`, hold the craft on this pose instead of its start pose until the
    /// end of tick `TICK`, then on its start pose: `TICK:X,Y,Z,LX,LY,LZ,UX,UY,UZ,FX,FY,FZ`, the
    /// position and the left, up and forward rows as `--field-from` reads them. For a capture
    /// that fired while its craft moved: a Rocket fired before `TICK` leaves from this pose. See
    /// [`crate::race_capture::pose_hold::Launch`]. Dev only.
    #[cfg(feature = "dev-cli")]
    #[arg(
        long,
        value_name = "TICK:X,Y,Z,LX,LY,LZ,UX,UY,UZ,FX,FY,FZ",
        requires = "pose_hold_ticks",
        allow_hyphen_values = true
    )]
    pub(crate) pose_hold_launch: Option<String>,

    /// With `--race --screenshot`, launch craft `SLOT`'s Quake so its bump
    /// reaches the player at the end of a tick: `TICK:SLOT`, e.g. `120:1`. The
    /// rival's wave a headless run would otherwise never send, for what a Quake
    /// hit shows and sounds: the cue pair, the shake, the sparks and the cockpit
    /// static. See [`oag_raceplay::SimRace::force_quake`].
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_quake: Option<String>,

    /// With `--race --screenshot`, set the player's shield to a percentage of
    /// its maximum at the end of a tick: `TICK:PERCENT`, e.g. `0:15` for a
    /// craft at 15 % from the first frame. Repeat the flag to script a drop:
    /// `--force-shield 0:60.9 --force-shield 40:60.2` is a fall inside one
    /// whole percent, `--force-shield 40:59.9` one across it.
    ///
    /// A verification aid, written from outside `Race::tick` the way `--give`
    /// writes the pickup slot, so nothing here reaches a determinism hash. It
    /// exists to look at the shield readout's flash against the original at a
    /// chosen shield without driving a craft into a wall for it.
    #[arg(long, value_name = "TICK:PERCENT", requires = "race")]
    pub(crate) force_shield: Vec<String>,

    /// With `--race --screenshot`, raise the "medal awarded" HUD message at the
    /// end of a tick: `TICK:TIER`, `TIER` being `gold`, `silver` or `bronze`,
    /// e.g. `10:gold`. A verification aid for the message lines, which a real
    /// race raises only from a campaign Zone or Speed Lap cell - a state a
    /// headless capture has no way to reach. Written from outside `Race::tick`.
    #[arg(long, value_name = "TICK:TIER", requires = "race")]
    pub(crate) force_medal: Vec<String>,

    /// With `--race --screenshot`, judge the race against a campaign cell's own
    /// medal targets (`grid0_3_2`), so the real medal lines - the four-second
    /// banner and the standing one - show as a Zone run or Speed Lap crosses a
    /// tier. Only the targets are taken from the cell; the mode and circuit
    /// stay whatever `--mode` and `--track` say.
    #[arg(long, value_name = "CELL", requires = "race")]
    pub(crate) campaign_cell: Option<String>,

    /// Keep a destroyed craft's hull instead of swapping in its
    /// `shipwreck.vex`: the headless way to measure what the wreck adds, by
    /// rendering the same frame with and without it.
    #[arg(long)]
    pub(crate) no_hull_wreck: bool,
}

/// The pre-race flyby: skipping it, and photographing a tick of it.
#[derive(clap::Args, Debug)]
pub(crate) struct IntroArgs {
    /// Skip the pre-race flyby: go straight to the countdown, as the original does when the
    /// button is held through it.
    ///
    /// On Pulse (PSP) a race opens with the circuit's own camera animation, `AnimEnd` long
    /// (25 s on most circuits). It ends on its own, or on a held Cross (thrust) once its first
    /// second has gone; this flag is for a scripted run that has no one to hold it.
    #[arg(long)]
    pub(crate) no_intro: bool,

    /// With `--screenshot`: play this many ticks of the pre-race flyby first and photograph that
    /// tick of it, the world held at the grid's first tick. A capture aid for the flyby.
    #[arg(long, value_name = "TICKS", default_value_t = 0)]
    pub(crate) intro_ticks: u32,

    /// With `--race --screenshot`, draw the on-screen touch controls over the
    /// frame, in a pose: `idle`, `idle-buttons` (zones off), `go-left`, `go-right`, `stick-go`, `go-fire`, `go-left-past`, `easy-idle`, `easy-zones-off`, `easy-brake` or `easy-bar-brake`.
    ///
    /// A headless run has no fingers; this builds the same `Touches` a finger
    /// would. Chosen, not measured: the controls are this project's own.
    #[arg(long, value_name = "POSE")]
    pub(crate) touch_overlay: Option<String>,
}

/// The motion blur's two overrides.
#[derive(clap::Args, Debug)]
pub(crate) struct BlurArgs {
    /// Motion blur strength: off, low, medium or high.
    ///
    /// Overrides `[render_profiles.<title>] motion_blur` for *every* title, for
    /// this run only; the file on disk is not changed - see `--reconstruction`'s own
    /// doc for why every title rather than one. Here for the reason
    /// `--msaa` is: two captures differing only by this flag are how
    /// the blur gets compared against itself off, and a capture honours it by
    /// rendering a primer frame at the tick-before-last camera first - see
    /// `crate::race_capture::CaptureOptions::motion_blur`.
    #[arg(long)]
    pub(crate) motion_blur: Option<oag_display::display::MotionBlur>,

    /// The blur's gather resolution: full or half.
    ///
    /// Overrides `[render_profiles.<title>] motion_blur_resolution` for every
    /// title, for this run only, the way `--motion-blur` does.
    #[arg(long)]
    pub(crate) motion_blur_resolution: Option<oag_display::display::BlurResolution>,
}

/// Who flies the player's craft in a headless run: an opponent's driver, and Pilot Assist.
#[derive(clap::Args, Debug)]
pub(crate) struct DriverArgs {
    /// Let an opponent's driver fly the player's craft.
    ///
    /// A verification aid - see `race::RaceSim::set_autopilot` - and the only way
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
    ///
    /// In a window it also presses the skip of a pre-race flyby that only a
    /// press ends (HD's repeats until one), as a hand holding Cross would;
    /// a flyby that ends by itself (Pulse's) still plays out. `--no-intro`
    /// drops every flyby.
    #[arg(long)]
    pub(crate) autopilot: bool,

    /// Pilot Assist for the player's craft in a headless run (`--screenshot`,
    /// `--trace-out`): `off`, `normal`, `extreme` or `on` (Extreme, the old switch's
    /// spelling), default off. The settings file's
    /// `[controls] pilot_assist` drives the windowed game; a capture or trace
    /// takes only this flag so it never depends on a machine's settings.
    #[arg(long, value_parser = parse_pilot_assist)]
    pub(crate) pilot_assist: Option<oag_physics::pilot_assist::Level>,

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

    /// Whose driver flies the opponents: `original` (the default), the
    /// recovered original driver in every race that has opponents, or `ours`,
    /// this project's own driver, everywhere. A developer's switch for testing
    /// our driver; it is not a setting and no menu row reaches it. See
    /// [`crate::ai_choice`].
    #[arg(long, value_enum, default_value_t)]
    pub(crate) ai: crate::ai_choice::AiChoice,

    /// Split screen: this many local players in one window, each with their own
    /// view of the one race (1 to 8, default 1 = no split).
    ///
    /// Slots `0..N` become local players and the screen is cut into `N` views,
    /// two stacked top and bottom, four as quadrants, up to eight - **chosen, not
    /// measured**. The race starts with `N` craft on the grid at least. Each view
    /// draws its own slot's HUD. See [`crate::split_screen`].
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=8))]
    pub(crate) players: u8,

    /// Which way `--players 2` is cut: `horizontal` (one view above the other, the default)
    /// or `vertical` (side by side). Means nothing for any other count. **Chosen, not
    /// measured**; on Wipeout HD/Fury the race box's own `SPLIT SCREEN` row sets it. See
    /// [`oag_render::viewport::Cut`].
    #[arg(long, default_value = "horizontal", value_parser = ["horizontal", "vertical"])]
    pub(crate) split_cut: String,

    /// Multi-window local play: this many OS windows (2 to 8), the people dealt across them as
    /// evenly as they divide - `--windows 2` is two people, a window each; `--windows 2 --players 6`
    /// is three and three, each window cut up like a split screen.
    ///
    /// Window 1 is the front end's and the one whose close ends the run; the others show a
    /// holding frame until a race starts, and closing one turns its seats over to the AI. Seated
    /// the way `--players N` seats: the keyboard and the first pad on slot 0, each further pad on
    /// the next slot. Desktop only. **Chosen, not measured.** See `docs/rendering/multi-window.md`.
    #[arg(long, value_parser = clap::value_parser!(u8).range(2..=8), conflicts_with = "window_seats")]
    pub(crate) windows: Option<u8>,

    /// Multi-window local play with the seats per window spelled out: a comma-separated list, one
    /// entry per window, each a count of 1 to 8 and optionally `h` or `v` for the way two seats are
    /// cut. `--window-seats 4,2v` is two windows: four seats cut in quarters in the first, two side
    /// by side in the second. A count without a letter takes `--split-cut`. At most eight seats in
    /// all. Not with `--windows` or `--players`. See `docs/rendering/multi-window.md`.
    #[arg(long, value_name = "COUNTS", conflicts_with_all = ["windows", "players"])]
    pub(crate) window_seats: Option<String>,

    /// With `--windows` or `--window-seats`: each window borderless fullscreen on a monitor of its
    /// own, in the order the platform lists them (window 1 on the first; they share when there are
    /// fewer). Works under Wayland, which cannot place a plain window on a monitor.
    #[arg(long)]
    pub(crate) windows_fullscreen: bool,
}

impl DriverArgs {
    /// The people dealt across windows, window 1 first: `--window-seats` as spelled, or
    /// `--windows` with `--players` (one person per window without it) dealt evenly. Empty for a
    /// run with one window. **An error where a window cannot be opened** - the browser, Android
    /// and iOS have the one surface the platform gives - rather than a flag silently ignored.
    pub(crate) fn window_layout(&self) -> anyhow::Result<Vec<crate::split_screen::WindowSeats>> {
        Ok(crate::app::window::one_window_under_gamescope(
            self.requested_layout()?,
            crate::app::window::under_gamescope(None),
        ))
    }

    /// The one log line that says `--windows` or `--window-seats` was folded into a single window
    /// because gamescope shows one window at a time; `None` when nothing was folded. The caller
    /// logs it once, since [`Self::window_layout`] is asked for several times.
    pub(crate) fn gamescope_fold_note(&self) -> Option<String> {
        let requested = self.requested_layout().ok()?;
        let seats: usize = requested.iter().map(|window| window.count).sum();
        (requested.len() >= 2 && crate::app::window::under_gamescope(None)).then(|| {
            format!(
                "gamescope shows one window at a time: the {} requested windows are folded into one window holding all {seats} seats",
                requested.len()
            )
        })
    }

    /// The layout as the command line asks for it, before any compositor folds it.
    fn requested_layout(&self) -> anyhow::Result<Vec<crate::split_screen::WindowSeats>> {
        use crate::split_screen::{deal, parse_window_seats};
        let layout = match (&self.window_seats, self.windows) {
            (Some(text), _) => parse_window_seats(text, self.cut())
                .map_err(|why| anyhow::anyhow!("--window-seats: {why}"))?,
            (None, Some(windows)) => {
                let people = if self.players > 1 {
                    usize::from(self.players)
                } else {
                    usize::from(windows)
                };
                deal(people, usize::from(windows), self.cut())
            }
            (None, None) => return Ok(Vec::new()),
        };
        if cfg!(any(
            target_arch = "wasm32",
            target_os = "android",
            target_os = "ios"
        )) {
            anyhow::bail!(
                "--windows and --window-seats need a desktop: this platform has the one window it gives"
            );
        }
        Ok(layout)
    }

    /// How many people the run seats at the start: the windows' seats, or `--players`.
    pub(crate) fn people(&self) -> u8 {
        let seated: usize = self
            .window_layout()
            .map_or(0, |layout| layout.iter().map(|window| window.count).sum());
        if seated > 0 {
            u8::try_from(seated).unwrap_or(u8::MAX)
        } else {
            self.players
        }
    }

    /// `--split-cut` as the type the renderer takes.
    pub(crate) fn cut(&self) -> oag_render::viewport::Cut {
        oag_render::viewport::Cut::parse(&self.split_cut).unwrap_or_default()
    }
}

/// `--pilot-assist`'s value: a level's token, or `on` for Extreme as the switch spelled it.
fn parse_pilot_assist(text: &str) -> Result<oag_physics::pilot_assist::Level, String> {
    use oag_physics::pilot_assist::Level;
    if text == "on" {
        return Ok(Level::Extreme);
    }
    Level::from_name(text)
        .ok_or_else(|| format!("expected off, normal, extreme or on, not {text:?}"))
}

/// `--connect` and its companions: this game as a client of an `oag-server`. See
/// `crate::net_play` and `docs/networking/README.md`, "Running a local server".
#[derive(clap::Args, Debug, Default)]
pub(crate) struct NetArgs {
    /// Race on the `oag-server` at HOST:PORT instead of locally. Implies `--race`:
    /// the track, team, class, mode, seed and grid are the server's. Needs
    /// `--cert-hash`.
    #[arg(long, value_name = "HOST:PORT", requires = "cert_hash")]
    pub(crate) connect: Option<String>,

    /// The server certificate's SHA-256, as `oag-server` prints it on its
    /// `cert-hash` line (64 hex digits), or `@FILE` to read it from the file
    /// `oag-server --hash-out` wrote. Only that certificate is accepted.
    #[arg(long, value_name = "HEX|@FILE", requires = "connect")]
    pub(crate) cert_hash: Option<String>,

    /// Fly this machine's craft with the player's own driver, sent to the server as
    /// input like a pad's. A verification aid for a whole race with nobody at the keys; on a
    /// LAN race from the menus (HOST GAME or JOIN GAME) it flies that machine's craft too.
    #[arg(long)]
    pub(crate) net_bot: bool,

    /// Add a round trip and a datagram loss rate on this end of the connection,
    /// `RTT_MS[:LOSS_PERCENT]` (e.g. `100` or `100:2`): a verification aid for judging
    /// a run on localhost at a real network's numbers.
    #[arg(long, value_name = "RTT_MS[:LOSS]", requires = "connect")]
    pub(crate) net_shim: Option<String>,

    /// Race through the online lobby at HOST:PORT (`oag-server lobby`): make or join a room,
    /// pick a ship, and race on the server it hands this machine to. Implies `--race`, and
    /// then goes the way `--connect` does. Needs `--lobby-hash`. A command-line path for the
    /// first spike; the menus' ONLINE screens are open work.
    #[arg(
        long,
        value_name = "HOST:PORT",
        requires = "lobby_hash",
        conflicts_with = "connect"
    )]
    pub(crate) online: Option<String>,

    /// The lobby certificate's SHA-256 (`lobby cert-hash` line), or `@FILE`.
    #[arg(long, value_name = "HEX|@FILE", requires = "online")]
    pub(crate) lobby_hash: Option<String>,

    /// The name this player goes by in the lobby.
    #[arg(long, requires = "online")]
    pub(crate) online_name: Option<String>,

    /// Make a room (with `--online-track`, `--online-class`, `--online-mode`,
    /// `--online-max`) and press START once `--online-start-at` people are ready.
    /// Without it, join `--online-room`, or the first joinable room.
    #[arg(long, requires = "online")]
    pub(crate) online_create: bool,

    /// With `--online-create`: this machine hosts the room's race (a player-hosted room). The
    /// lobby checks `--online-host-port` from its side before it lists the room.
    #[arg(long, requires = "online_create")]
    pub(crate) online_host: bool,

    /// The UDP port a hosted race listens on, the one to open or forward to this machine
    /// (default: the game's own, 4435); 0 lets the system pick (a test on one machine).
    #[arg(long, requires = "online_host")]
    pub(crate) online_host_port: Option<u16>,

    /// The room to join, from the lobby's list.
    #[arg(long, requires = "online", conflicts_with = "online_create")]
    pub(crate) online_room: Option<u32>,

    /// The team to fly, as `--team` spells it. Required with `--online`: picking a ship is
    /// what readies a person in the lobby, and a default would be a title's guess.
    #[arg(long, requires = "online")]
    pub(crate) online_team: Option<String>,

    /// The circuit of a room made with `--online-create`, as `--track` spells it.
    #[arg(long, requires = "online", default_value = "")]
    pub(crate) online_track: String,

    /// The speed class of a room made with `--online-create`.
    #[arg(long, requires = "online", default_value = "VENOM")]
    pub(crate) online_class: String,

    /// The race mode of a room made with `--online-create`.
    #[arg(long, requires = "online", default_value = "single_race")]
    pub(crate) online_mode: String,

    /// The most people in a room made with `--online-create`.
    #[arg(long, requires = "online", default_value_t = 2)]
    pub(crate) online_max: u8,

    /// Laps of a room made with `--online-create`; 0 is the mode's own count.
    #[arg(long, requires = "online", default_value_t = 0)]
    pub(crate) online_laps: u8,

    /// A room made with `--online-create` switches barrel rolls off.
    #[arg(long, requires = "online")]
    pub(crate) online_no_barrel_rolls: bool,

    /// A room made with `--online-create` disallows Pilot Assist (it allows it otherwise).
    #[arg(long, requires = "online")]
    pub(crate) online_no_pilot_assist: bool,

    /// A room made with `--online-create` switches weapons off (Eliminator keeps them).
    #[arg(long, requires = "online")]
    pub(crate) online_no_weapons: bool,

    /// The zone a Zone room made with `--online-create` ends at (15 to 55); 0 never ends.
    #[arg(long, requires = "online", default_value_t = 0)]
    pub(crate) online_zone_target: u8,

    /// The Pilot Assist level this client flies with where the room allows it
    /// (`off`, `normal`, `extreme`).
    #[arg(long, requires = "online", default_value = "off")]
    pub(crate) online_assist: String,

    /// A room made with `--online-create` presses START when this many are ready.
    #[arg(long, requires = "online", default_value_t = 2)]
    pub(crate) online_start_at: usize,
}

impl NetArgs {
    /// The connection asked for, if any, its hash and shim parsed.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn connect(&self, source: &str) -> anyhow::Result<Option<crate::net_play::Connect>> {
        if let Some(plan) = self.online_plan(source)? {
            return crate::online::run(&plan).map(|connect| {
                Some(crate::net_play::Connect {
                    shim: self.shim().ok().flatten(),
                    bot: self.net_bot,
                    ..connect
                })
            });
        }
        let Some(addr) = &self.connect else {
            return Ok(None);
        };
        let given = self.cert_hash.as_deref().unwrap_or_default();
        let hex = match given.strip_prefix('@') {
            Some(path) => std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("--cert-hash @{path}: {e}"))?,
            None => given.to_string(),
        };
        Ok(Some(crate::net_play::Connect {
            addr: addr.clone(),
            cert_hash: oag_net::wt::parse_hash(hex.trim())?,
            shim: self.shim()?,
            bot: self.net_bot,
            name: format!("oag-game {}", std::process::id()),
            token: String::new(),
            lobby: None,
            params: [0; 32],
            host_key: [0; 32],
            signer: None,
        }))
    }

    /// `--net-shim`, parsed.
    #[cfg(not(target_arch = "wasm32"))]
    fn shim(&self) -> anyhow::Result<Option<oag_net::link::Conditions>> {
        self.net_shim
            .as_deref()
            .map(oag_net::link::Conditions::parse)
            .transpose()
            .map_err(anyhow::Error::msg)
    }

    /// The lobby flow `--online` asked for, if any.
    #[cfg(not(target_arch = "wasm32"))]
    fn online_plan(&self, source: &str) -> anyhow::Result<Option<crate::online::Plan>> {
        use crate::online::{Action, Plan};
        let Some(addr) = &self.online else {
            return Ok(None);
        };
        let given = self.lobby_hash.as_deref().unwrap_or_default();
        let hex = match given.strip_prefix('@') {
            Some(path) => std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("--lobby-hash @{path}: {e}"))?,
            None => given.to_string(),
        };
        let opened = oag_source::title::open_source(source, Vec::new(), Vec::new())
            .map_err(|e| anyhow::anyhow!("{source} is not a disc this build knows: {e:#}"))?;
        let title = opened.title.name;
        let edition = oag_net::wire::online::Edition {
            title: title.to_string(),
            platform: opened.archives.layout.platform.to_string(),
            content: crate::mods::Mods::load_for_profile()
                .ok()
                .map_or(0, |mods| {
                    let picks = crate::settings::load().unwrap_or_default().mod_settings;
                    mods.content_fingerprint(&picks)
                }),
            build: oag_net::BUILD_VERSION.to_string(),
        };
        let action = if self.online_create {
            Action::Create(oag_net::wire::online::RoomSettings {
                title: title.to_string(),
                track: self.online_track.clone(),
                class: self.online_class.clone(),
                mode: self.online_mode.clone(),
                max_players: self.online_max,
                opponents: false,
                laps: self.online_laps,
                no_barrel_rolls: self.online_no_barrel_rolls,
                pilot_assist: !self.online_no_pilot_assist,
                weapons: !self.online_no_weapons,
                zone_target: self.online_zone_target,
            })
        } else {
            Action::Join(self.online_room)
        };
        Ok(Some(Plan {
            addr: addr.clone(),
            cert_hash: oag_net::wt::parse_hash(hex.trim())?,
            name: self
                .online_name
                .clone()
                .unwrap_or_else(|| format!("oag-game {}", std::process::id())),
            titles: vec![edition],
            action,
            team: self
                .online_team
                .clone()
                .filter(|team| !team.is_empty())
                .ok_or_else(|| {
                    anyhow::anyhow!("--online needs --online-team (the ship to pick)")
                })?,
            assist: oag_physics::pilot_assist::Level::from_name(&self.online_assist)
                .ok_or_else(|| anyhow::anyhow!("--online-assist is off, normal or extreme"))?,
            start_at: self.online_start_at,
            host: self.online_host.then(|| {
                let port = self.online_host_port.unwrap_or(oag_net::lan::DEFAULT_PORT);
                (source.to_string(), port)
            }),
        }))
    }

    /// The browser build has no native transport (`oag_net::wt` is native only), so
    /// `--connect` is refused there.
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn connect(
        &self,
        _source: &str,
    ) -> anyhow::Result<Option<crate::net_play::Connect>> {
        match (&self.connect, &self.online) {
            (None, None) => Ok(None),
            _ => anyhow::bail!("--connect and --online are not available in the browser build"),
        }
    }
}

/// Switches for measuring the front end without a person at it (`crate::devrun`).
#[derive(clap::Args, Debug)]
pub(crate) struct DevArgs {
    /// With `--players`, `--windows` or `--window-seats` and no `--race`: seat those people
    /// on placeholder keyboard devices (no pads, no virtual input) and open the WINDOW LAYOUT
    /// screen as soon as the menus are up, pre-filled from the flags.
    ///
    /// **Explicit, not implied by the flags** (chosen, not measured): `--windows 2` has always
    /// opened its windows and waited for a person at the join screen. Real pads can still
    /// take a free seat on the join screen after BACK; with every seat dealt they cannot
    /// until a seat is freed. Pair with `--no-intro` to reach the menus sooner.
    #[arg(long)]
    pub(crate) preseat: bool,

    /// A timed button script for a windowed run: `SECONDS:BUTTON,...`, e.g.
    /// `5:down,5.5:down,6:right,12:quit`; `+3:right` counts from where `--preseat` landed. Buttons are the `--press` names; `quit` ends the run.
    ///
    /// Seconds count from the first frame. Each entry is one tap. Unlike `--press` it needs no
    /// `--screenshot`. Dev only.
    #[arg(long, value_name = "SCRIPT")]
    pub(crate) script: Option<String>,

    /// Play one rumble pattern on the connected pad and exit: a disc pattern by name
    /// (`Missile_Fired_Rumble`, read from `--source`) or the path of a `.toml` pattern.
    ///
    /// Opens the pad subsystem with no window, steps the pattern at 60 Hz through the
    /// same player and `gilrs` backend a race uses, prints every level it sends, and
    /// silences the motors. The law is `[controls] rumble` (`ps3` or `ps4`; `original`
    /// is the booted title's own). A looping pattern is held for `--rumble-hold` seconds.
    /// Dev only.
    #[cfg(feature = "dev-cli")]
    #[arg(long, value_name = "NAME|FILE.toml")]
    pub(crate) rumble_pattern: Option<String>,

    /// Seconds `--rumble-pattern` holds a looping pattern before finishing it.
    #[cfg(feature = "dev-cli")]
    #[arg(long, default_value_t = 2.0, requires = "rumble_pattern")]
    pub(crate) rumble_hold: f32,

    /// Log one line a second on every stage, menus included: stage, fps, frame/CPU/GPU/present
    /// time, target rate, window, surface and render size. `OAG_FRAME_LOG=1` does the same.
    /// To the terminal and the log file. Dev only.
    #[arg(long)]
    pub(crate) frame_log: bool,

    /// Run our own menus with no window, driven by this script, and exit non-zero when a step
    /// times out or names a row that is not there. Steps are separated by `;` or newlines:
    /// `until page=race`, `select race.track`, `set race.mode=time_trial`, `press cross`,
    /// `ticks 60`, `screenshot out.png`, `expect stage=race`, `quit`. The game reports its state
    /// as JSON lines (`--state-log`); the script reads that, never a picture. Needs `--source`
    /// or one disc image in the search path, or `--launcher` to script the chooser itself. See
    /// docs/tools/oag-game.md, "Driving the menus".
    #[arg(
        long,
        value_name = "STEPS",
        conflicts_with_all = ["drive_file", "race", "screenshot", "dry_run", "trace_out"]
    )]
    pub(crate) drive: Option<String>,

    /// `--drive`, with the script read from a file (`#` starts a comment).
    #[arg(
        long,
        value_name = "FILE",
        conflicts_with_all = ["race", "screenshot", "dry_run", "trace_out"]
    )]
    pub(crate) drive_file: Option<std::path::PathBuf>,

    /// Where `--drive` writes its state, one JSON line per change. Stderr when absent.
    #[arg(long, value_name = "FILE")]
    pub(crate) state_log: Option<std::path::PathBuf>,
}

impl DevArgs {
    /// What a run carries of these switches. `people` is `DriverArgs::people`.
    pub(crate) fn dev(&self, people: u8) -> anyhow::Result<crate::devrun::Dev> {
        Ok(crate::devrun::Dev {
            preseat: (self.preseat && people > 1).then_some(usize::from(people)),
            script: match &self.script {
                Some(text) => crate::devrun::Script::parse(text)
                    .map_err(|why| anyhow::anyhow!("--script: {why}"))?,
                None => crate::devrun::Script::default(),
            },
            frame_log: self.frame_log
                || std::env::var_os("OAG_FRAME_LOG").is_some_and(|v| v != "0"),
            lockstep: self.drive.is_some() || self.drive_file.is_some(),
            waiting: false,
        })
    }

    /// The script `--drive` or `--drive-file` names, parsed, or `None` for a run that drives nothing.
    pub(crate) fn drive(&self) -> anyhow::Result<Option<crate::drive::Script>> {
        let text = match (&self.drive, &self.drive_file) {
            (Some(text), _) => text.clone(),
            (None, Some(path)) => std::fs::read_to_string(path)
                .with_context(|| format!("--drive-file {}", path.display()))?,
            (None, None) => {
                anyhow::ensure!(
                    self.state_log.is_none(),
                    "--state-log needs --drive or --drive-file"
                );
                return Ok(None);
            }
        };
        crate::drive::Script::parse(&text)
            .map(Some)
            .map_err(|why| anyhow::anyhow!("--drive: {why}"))
    }
}

/// The attract demo as a run of its own.
#[derive(clap::Args, Debug)]
pub(crate) struct DemoArgs {
    /// Start in an all-AI demo race and chain the next when one ends; any input returns to the menu.
    ///
    /// With `--dry-run` or `--screenshot` no window opens: `--dry-run` flies demos back to back
    /// (`--ticks` shortens each) and logs each pick; `--screenshot` captures one. See
    /// docs/ui/attract-demo.md.
    #[arg(long, conflicts_with_all = ["race", "launcher"])]
    pub(crate) demo: bool,

    /// Like `--demo`, as a wallpaper: input is ignored (Escape quits) and the HUD is hidden.
    /// On a Wayland compositor with `wlr-layer-shell` (niri, sway, Hyprland) it is the desktop
    /// background of every output, a camera each; SIGINT or SIGTERM ends it.
    #[arg(long, conflicts_with_all = ["race", "launcher"])]
    pub(crate) wallpaper: bool,

    /// The most frames a second a `--wallpaper` draws (a rate, or `unlimited`), whatever the
    /// VSYNC and FRAME LIMIT settings say, so a background does not run the GPU flat out.
    #[arg(long, value_name = "FPS", default_value = "30", requires = "wallpaper")]
    pub(crate) wallpaper_fps: oag_present::perf::FrameLimit,
}

impl DemoArgs {
    /// `Some(wallpaper)` when the run is a demo.
    pub(crate) fn mode(&self) -> Option<bool> {
        (self.demo || self.wallpaper).then_some(self.wallpaper)
    }

    /// `--wallpaper-fps`, for a wallpaper only.
    pub(crate) fn limit(&self) -> Option<oag_present::perf::FrameLimit> {
        self.wallpaper.then_some(self.wallpaper_fps)
    }
}

/// `--png-info` and `--from-png`: reading a screenshot's own description back.
#[derive(clap::Args, Debug, Default)]
pub(crate) struct ShotArgs {
    /// Print the description a screenshot of ours carries (the `oag` text chunk: title, race,
    /// camera, render settings, adapter, mods, build and the flags that draw it again) and
    /// stop. Reads the file only; opens no disc and no window.
    #[arg(long, value_name = "FILE.png")]
    pub(crate) png_info: Option<PathBuf>,

    /// Make a screenshot of ours again: rebuild its race from the input stream it carries,
    /// re-simulate to its tick, say whether the state hash matches, and draw the frame to
    /// `--screenshot` (default `<file>-rerender.png`). Needs the same title's disc, found as
    /// every other run finds it.
    #[arg(long, value_name = "FILE.png")]
    pub(crate) from_png: Option<PathBuf>,

    /// Open a screenshot of ours, or a replay file (`.oagr`), in the replay viewer: the game
    /// starts, and once the main menu is up the race the file carries is loaded. A replay plays
    /// from the start; a screenshot opens paused on the tick it was taken at (press CROSS to play
    /// it from the beginning). A file that is neither, has no input stream, was taken on another title or needs a
    /// simulation mod this machine lacks is told to the player over the main menu. The same
    /// opens by dropping the file on the window over the main menu or the attract demo. Needs
    /// the same title's disc, found as every other run finds it; use `--from-png` to draw the
    /// frame again headless instead.
    #[arg(
        long,
        value_name = "FILE",
        visible_alias = "watch",
        conflicts_with_all = ["from_png", "png_info", "race", "screenshot", "dry_run"]
    )]
    pub(crate) replay_png: Option<PathBuf>,
}
