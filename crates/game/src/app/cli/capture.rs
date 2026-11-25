//! A headless `--screenshot`: its size, the state it waits for and the buttons it presses.

use crate::app::args::DEFAULT_SIZE;

/// A headless `--screenshot`: its size, the state it waits for and the buttons it presses.
#[derive(clap::Args, Debug)]
pub(crate) struct CaptureArgs {
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
    /// never opens, so name an unlocked one. `map:EVENT NAME` stops on the
    /// map with that event under the cursor and no card. A mounted package's
    /// cell (`grid0_3_1`) brings its own campaign's map up.
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

    /// With `--press` and `--until`, stop pressing once the `--until` state is
    /// current, so the remaining `--ticks` run on that screen untouched.
    ///
    /// Without it the buttons keep pulsing after the state is reached, which
    /// is what `--until Home --press ... --ticks 40` relies on. A screen that
    /// a button press leaves (2048's `TitleScreen`, whose intro movie only a
    /// press ends) needs this to be captured settled: `--until TitleScreen
    /// --press start --press-until-reached --ticks 300`.
    #[arg(long, requires_all = ["until", "press"])]
    pub(crate) press_until_reached: bool,

    /// With `--until map:EVENT`, press these buttons one after another on the
    /// map once it is up, one press every 30 ticks, e.g. `right,right,down`;
    /// `--ticks` then picks the frame after the last of them.
    #[arg(long, requires = "until")]
    pub(crate) map_presses: Option<String>,
}
