//! Flag groups `clap` flattens into [`super::Cli`], each its own struct so
//! `cli.rs` stays under the size ratchet.

use std::path::PathBuf;

/// Writing the window icon.
#[derive(clap::Args, Debug)]
pub(crate) struct IconArgs {
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
/// `records.toml` and never writes it (see `oag_game::race_capture::CaptureOptions::
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
    ///     --ticks 9000 --record-ghost data/scratch/lap.oagr --screenshot /dev/null
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
    #[arg(long, value_name = "PROMPT", requires = "menu_page")]
    pub(crate) menu_prompt: Option<String>,
}

/// Capturing a wrecked craft in a headless `--race --screenshot` run.
#[derive(clap::Args, Debug)]
pub(crate) struct WreckArgs {
    /// With `--race --screenshot`, put one craft into the destroyed sequence at
    /// the end of a tick: `TICK:SLOT`, e.g. `60:3` and then `--ticks 100` for a
    /// wreck about half a second old.
    ///
    /// A verification aid, there because a wreck is hard to produce at a chosen
    /// place and moment. It enters the state `Ship_Damage`'s depletion does, so
    /// the sequence that follows is the simulation's own. See
    /// [`oag_raceplay::Race::force_destroy`].
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_wreck: Option<String>,

    /// With `--race --screenshot`, land a weapon hit on one craft at the end of
    /// a tick: `TICK:SLOT`, e.g. `10:0`. A verification aid for what a struck
    /// hull shows (hit sparks, HD's damage smoke), there because a headless run
    /// has no rival whose weapon reaches the player. The hit spends no shield
    /// and takes none; pair it with `--force-shield` for the shield it should
    /// find. See [`oag_raceplay::Race::force_weapon_hit`].
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_hit: Option<String>,

    /// With `--race --screenshot`, put a locked LeachBeam from the player onto
    /// a rival at the end of a tick: `TICK:SLOT`, e.g. `300:1`. A verification
    /// aid for the LeachBall's own draw, there because a headless run has no
    /// rival in the beam's cone. Writes the world the way `--force-wreck`
    /// does, from outside `Race::tick`.
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_leach_lock: Option<String>,

    /// With `--race --screenshot`, put craft `SLOT` on a Bomb the player has laid
    /// at the end of a tick: `TICK:SLOT`, e.g. `600:1`. A verification aid for
    /// the Bomb's detonation, there because a headless run has no rival that
    /// drives over it. The next tick's own trip test does the rest. See
    /// [`oag_raceplay::Race::force_bomb_trip`].
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_bomb_trip: Option<String>,

    /// With `--race --screenshot`, put craft `SLOT` on a Missile the player has
    /// fired at the end of a tick: `TICK:SLOT`, e.g. `330:1`. The same aid
    /// `--force-bomb-trip` is, for the Missile's explosion: a headless run's
    /// rivals drive away from the shot. The next tick's own hit test does the
    /// rest. See [`oag_raceplay::Race::force_missile_hit`].
    #[arg(long, value_name = "TICK:SLOT", requires = "race")]
    pub(crate) force_missile_hit: Option<String>,

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
    /// `oag_game::race_capture::CaptureOptions::motion_blur`.
    #[arg(long)]
    pub(crate) motion_blur: Option<oag_display::display::MotionBlur>,

    /// The blur's gather resolution: full or half.
    ///
    /// Overrides `[render_profiles.<title>] motion_blur_resolution` for every
    /// title, for this run only, the way `--motion-blur` does.
    #[arg(long)]
    pub(crate) motion_blur_resolution: Option<oag_display::display::BlurResolution>,
}
