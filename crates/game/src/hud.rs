//! The in-race HUD: the disc's own layout, drawn through the existing quad
//! renderer.
//!
//! # The layout is authored data, not code
//!
//! Pulse ships five HUD layouts in `Data.wad`, one per race mode, in the same
//! shortened front-end XML the front end uses - `Arcade_HUD.xml`,
//! `Elimination_HUD.xml`, `TimeTrial_HUD.xml`, `Zone_HUD.xml` and
//! `MPTag_HUD.xml`. They carry exact pixel rectangles, atlas sub-rectangles,
//! named colour constants, font roles and alignment, so **none of the geometry
//! here is measured, fitted or guessed**: it is read off the player's own disc.
//! `oag-wad cat --expand <image>:...Data.wad 'Data\XML\TimeTrial_HUD.xml'`
//! prints one.
//!
//! That is worth stating because it was got wrong first: the absence of a HUD
//! element from [`oag_formats::fexml`]'s known-element table was read as
//! evidence that the HUD was drawn from code, and a whole plan was built on
//! recovering rectangles out of the decompiler. The layouts were there the
//! whole time. **Negative evidence from a table of what a parser happens to
//! model is not evidence about the format.**
//!
//! # What this module does and does not decide
//!
//! [`Layout`] is geometry, straight from the file. [`draw_list`] decides only
//! *which* widgets are live this frame and *what text* they carry, from a
//! [`Readout`]. Keeping those apart is what lets the whole HUD be tested
//! without a GPU, the way [`crate::perf`] is - and it means a layout change is a
//! data change rather than a code change.
//!
//! # Screen space
//!
//! Everything is in the PSP's 480x272 space ([`SCREEN`]), which is what the XML's
//! coordinates are in, and the renderer letterboxes that into whatever the window
//! is. A widget inside an `<Item>` is positioned relative to that group's
//! `OffsetX`/`OffsetY`; this module resolves those to absolute coordinates at
//! parse time so nothing downstream has to carry the offset around.

use std::collections::HashMap;

use oag_formats::fexml::{self, Node};

use crate::frontend::{Align, Draw, SCREEN};
use crate::screen::{argb_to_rgba, parse_argb};

/// The atlas and the five layout entries: `oag_pulse::hud`.
///
/// Which files Pulse ships is a title fact and moved there under [ADR-0021];
/// everything that reads them - the widget model, the `<Item>` offset handling,
/// the draw list - is this module and is title-blind.
///
/// [ADR-0021]: ../../../docs/architecture/adr/0021-title-packages.md
pub use oag_pulse::hud::{ATLAS, layouts};

/// Which of the disc's fonts a widget draws in.
///
/// The role names are the language plugin's, and the mapping to `.fnt` files is
/// in [`crate::frontend`]'s line-height table and
/// [`oag_pulse::names::fonts`]: `HUD` is `PulseHud.fnt` at 25 px,
/// `HUDSmall` is `small.fnt` at 10 px, `Default` is `pulse_text.fnt` at 13 px.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Font {
    /// `font="HUD"`. Every value the HUD displays - speed, lap, times.
    #[default]
    Hud,
    /// `font="HUDSmall"`. Every label beside those values.
    Small,
    /// `font="Default"`.
    ///
    /// Used by exactly eight widgets across all five layouts, `PosTag0` to
    /// `PosTag7` - the floating opponent name tags. Nothing draws those yet,
    /// since a race has one ship, but the variant exists so the parser does not
    /// silently fold them into a font they are not.
    Default,
}

impl Font {
    fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "hudsmall" => Self::Small,
            "default" => Self::Default,
            _ => Self::Hud,
        }
    }
}

/// Vertical alignment of a text widget about its `y`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VertAlign {
    /// No `vertalign`: `y` is the top edge, which is what
    /// [`crate::render::Renderer`] draws text at natively.
    #[default]
    Top,
    /// `vertalign="middle"` or `vertalign="centre"`.
    ///
    /// **Both spellings ship**, 37 and 16 times respectively, and they mean the
    /// same thing - unlike `align`, where `centre` is the only spelling used.
    Middle,
    /// `vertalign="bottom"`, the commonest at 41 uses: `y` is the baseline-ish
    /// bottom edge, so a value grows upward from a fixed line.
    Bottom,
}

impl VertAlign {
    fn parse(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "middle" | "centre" | "center" => Self::Middle,
            "bottom" => Self::Bottom,
            _ => Self::Top,
        }
    }
}

/// One `<Image>`: a rectangle cut out of [`ATLAS`] and drawn somewhere.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    /// The `name` attribute, which is how [`draw_list`] finds a widget.
    pub name: String,
    /// Destination in screen space, `[x, y, width, height]`, absolute - the
    /// enclosing `<Item>`'s offset and any `Centred="true"` are already applied.
    pub rect: [f32; 4],
    /// Source rectangle in atlas pixels, `[U, V, TxtrWidth, TxtrHeight]`.
    ///
    /// Separate from [`Self::rect`] because they genuinely differ: `TimeDiffIcon`
    /// samples a 28x23 patch and draws it at 14x12, so the sprite is
    /// half-size rather than 1:1.
    pub uv: [f32; 4],
    /// Modulating colour, already resolved through the constant table.
    pub color: [f32; 4],
    /// The `Src` entry name. Almost always [`ATLAS`].
    pub src: String,
}

/// One `<Text>`: a string, a font, and where it goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// The `name` attribute.
    pub name: String,
    /// Anchor x in screen space, absolute; what it anchors depends on
    /// [`Self::align`].
    pub x: f32,
    /// Anchor y in screen space, absolute; what it anchors depends on
    /// [`Self::vertalign`].
    pub y: f32,
    /// Which disc font.
    pub font: Font,
    /// Scale multiplier on the font's own cell. Ranges 0.5 to 1.5 in the shipped
    /// layouts.
    pub scale: f32,
    /// Text colour, resolved.
    pub color: [f32; 4],
    /// The `BorderColor`, when the widget has one.
    ///
    /// An outline colour, and 54 widgets carry one. **Not drawn yet** - the
    /// renderer has no outline pass, and faking one with four offset copies
    /// would quadruple the glyph count for something nobody has compared against
    /// the original. Parsed rather than dropped because the attribute is real.
    pub border: Option<[f32; 4]>,
    /// Horizontal alignment about [`Self::x`].
    pub align: Align,
    /// Vertical alignment about [`Self::y`].
    pub vertalign: VertAlign,
    /// A localisation key, resolved against the language plugin's string table.
    pub idstring: Option<String>,
    /// A literal string, used instead of `idstring` by 11 widgets - `"0 kmh"`,
    /// `"+0"`, `"/"`.
    pub string: Option<String>,
}

/// One `<Image>` with **no** `Src`: a solid rectangle rather than a cut-out.
///
/// The same convention the front end's screens use, where
/// [`crate::screen::Screen::fills`] holds the colour-only kind. Exactly one HUD
/// widget is of this kind, `HeadToHeadBar` in the arcade and eliminator layouts,
/// and it is authored `width="5" height="0"` - a zero-height bar, so its length
/// is supplied at runtime and the authored height is a floor rather than a size.
/// That is recorded rather than acted on: nothing here drives it.
#[derive(Debug, Clone, PartialEq)]
pub struct Fill {
    /// The `name` attribute.
    pub name: String,
    /// Destination in screen space, `[x, y, width, height]`, absolute.
    pub rect: [f32; 4],
    /// Colour, resolved.
    pub color: [f32; 4],
}

/// One `<Mode3D><Model>`: a `.vex` model drawn in the 3D overlay layer.
///
/// Recorded and **not drawn**. The five referenced models are the countdown
/// (`Pulse_Ready_Go`, `Cockpit_321GO`) and the weapon sights
/// (`missile_sight_inner`/`_outer`, `leachbeam_sight`), all of which need a
/// second pass with a projection of their own. See `docs/ui/hud.md`.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// The `name` attribute.
    pub name: String,
    /// The `.vex` entry name.
    pub src: String,
    /// Position in the 3D overlay's own space.
    pub position: [f32; 3],
}

/// A parsed HUD layout: pure geometry, straight off the disc.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Layout {
    /// Every `<Image>` with a `Src`, in document order - which is back-to-front
    /// paint order.
    pub sprites: Vec<Sprite>,
    /// Every `<Image>` without one, in document order.
    pub fills: Vec<Fill>,
    /// Every `<Text>`, in document order.
    pub labels: Vec<Label>,
    /// Every `<Mode3D><Model>`, recorded but not drawn.
    pub models: Vec<Model>,
    /// The `<Variable global=>` table, by name, before `FEConst->` resolution.
    pub constants: HashMap<String, String>,
    /// Anything the parser met and did not model, for the load report.
    ///
    /// A layout that silently drops a widget looks identical to one that draws it
    /// wrong, so the count is surfaced rather than swallowed.
    pub skipped: Vec<String>,
}

/// Resolves `FEConst->Name` and `FEGlobals->Name` against a constant table.
///
/// Both prefixes appear in the shipped data - the HUD layouts use `FEConst->`
/// throughout and the front end's own screens use `FEGlobals->` - and
/// `docs/formats/fexml.md` records them as a load-time snapshot versus a live
/// binding. Nothing here is live, so both are read the same way.
///
/// The real XML has newlines inside these values, from being written across two
/// source lines, hence the trim.
fn resolve<'a>(constants: &'a HashMap<String, String>, value: &'a str) -> Option<&'a str> {
    let key = value
        .strip_prefix("FEConst->")
        .or_else(|| value.strip_prefix("FEGlobals->"));
    match key {
        Some(key) => constants.get(key.trim()).map(String::as_str),
        None => Some(value),
    }
}

/// A colour attribute, resolved and split into RGBA. White when absent.
fn colour(constants: &HashMap<String, String>, raw: Option<&str>) -> [f32; 4] {
    raw.and_then(|v| resolve(constants, v))
        .and_then(parse_argb)
        .map_or([1.0, 1.0, 1.0, 1.0], argb_to_rgba)
}

/// A numeric attribute, resolved. `None` when absent or unparseable.
fn number(constants: &HashMap<String, String>, raw: Option<&str>) -> Option<f32> {
    resolve(constants, raw?)?.trim().parse().ok()
}

impl Layout {
    /// Reads one of the disc's HUD layouts.
    ///
    /// Takes the **expanded** XML - the shortened form has to go through
    /// [`oag_formats::fexml::expand`] first, which the archive reader does.
    ///
    /// Deliberately forgiving in the same way [`oag_formats::fexml::parse`] is:
    /// a widget missing an attribute takes a documented default rather than
    /// failing the whole layout, because a HUD that loses one readout is far
    /// better than a race with no HUD. What it will not do is lose a widget
    /// *silently* - see [`Self::skipped`].
    #[must_use]
    pub fn from_xml(xml: &str) -> Self {
        let root = fexml::parse(xml);
        let mut out = Self::default();

        // Constants first: a widget's colour can reference one declared anywhere
        // in the file, including after the widget itself.
        collect_constants(&root, &mut out.constants);
        out.collect(&root, 0.0, 0.0);
        out
    }

    /// Walks the tree, carrying the enclosing `<Item>`'s offset down.
    fn collect(&mut self, node: &Node, offset_x: f32, offset_y: f32) {
        for child in &node.children {
            let name = child.name.as_str();

            if name.eq_ignore_ascii_case("Item") {
                // An `<Item>` is a translation group, and its offset is absolute
                // rather than relative to an enclosing one: nothing in the
                // shipped data nests them, so composing would be untested code.
                let x = number(&self.constants, child.attr("OffsetX")).unwrap_or(0.0);
                let y = number(&self.constants, child.attr("OffsetY")).unwrap_or(0.0);
                self.collect(child, x, y);
                continue;
            }

            if name.eq_ignore_ascii_case("Image") {
                if let Some(sprite) = self.sprite_from(child, offset_x, offset_y) {
                    self.sprites.push(sprite);
                }
                continue;
            }

            if name.eq_ignore_ascii_case("Text") {
                if let Some(label) = self.label_from(child, offset_x, offset_y) {
                    self.labels.push(label);
                }
                continue;
            }

            if name.eq_ignore_ascii_case("Model") {
                if let Some(model) = self.model_from(child) {
                    self.models.push(model);
                }
                continue;
            }

            // `Screen`, `Mode3D`, `Variable` and `Values` are containers or
            // already-handled carriers; everything else is walked into so a
            // widget nested somewhere unexpected is still found.
            self.collect(child, offset_x, offset_y);
        }
    }

    fn sprite_from(&mut self, node: &Node, offset_x: f32, offset_y: f32) -> Option<Sprite> {
        let Some(name) = node.attr("name").map(str::to_string) else {
            self.skipped.push("an <Image> with no name".to_string());
            return None;
        };
        // No `Src` means a solid rectangle, not a broken widget - see [`Fill`].
        let Some(src) = node.value("Src").map(str::to_string) else {
            let width = number(&self.constants, node.value("width")).unwrap_or(0.0);
            let height = number(&self.constants, node.value("height")).unwrap_or(0.0);
            self.fills.push(Fill {
                name,
                rect: [
                    offset_x + number(&self.constants, node.value("x")).unwrap_or(0.0),
                    offset_y + number(&self.constants, node.value("y")).unwrap_or(0.0),
                    width,
                    height,
                ],
                color: colour(&self.constants, node.value("Color")),
            });
            return None;
        };

        let width = number(&self.constants, node.value("width")).unwrap_or(0.0);
        let height = number(&self.constants, node.value("height")).unwrap_or(0.0);
        let mut x = offset_x + number(&self.constants, node.value("x")).unwrap_or(0.0);
        let mut y = offset_y + number(&self.constants, node.value("y")).unwrap_or(0.0);

        // `Centred="true"` makes x/y the middle of the rectangle rather than its
        // top-left corner. 54 widgets use it, all of them screen-centre pieces
        // like the pickup box at x=240 - half of 480.
        if node.flag("Centred").unwrap_or(false) {
            x -= width / 2.0;
            y -= height / 2.0;
        }

        Some(Sprite {
            name,
            rect: [x, y, width, height],
            uv: [
                number(&self.constants, node.value("U")).unwrap_or(0.0),
                number(&self.constants, node.value("V")).unwrap_or(0.0),
                number(&self.constants, node.value("TxtrWidth")).unwrap_or(width),
                number(&self.constants, node.value("TxtrHeight")).unwrap_or(height),
            ],
            color: colour(&self.constants, node.value("Color")),
            src,
        })
    }

    fn label_from(&mut self, node: &Node, offset_x: f32, offset_y: f32) -> Option<Label> {
        let Some(name) = node.attr("name").map(str::to_string) else {
            self.skipped.push("a <Text> with no name".to_string());
            return None;
        };

        Some(Label {
            name,
            x: offset_x + number(&self.constants, node.value("x")).unwrap_or(0.0),
            y: offset_y + number(&self.constants, node.value("y")).unwrap_or(0.0),
            font: node.value("font").map(Font::parse).unwrap_or_default(),
            scale: number(&self.constants, node.value("scale")).unwrap_or(1.0),
            color: colour(&self.constants, node.value("Color")),
            border: node
                .value("BorderColor")
                .and_then(|v| resolve(&self.constants, v))
                .and_then(parse_argb)
                .map(argb_to_rgba),
            align: node.value("align").map(Align::parse).unwrap_or(Align::Left),
            vertalign: node
                .value("vertalign")
                .map(VertAlign::parse)
                .unwrap_or_default(),
            idstring: node.value("idstring").map(str::to_string),
            string: node.value("string").map(str::to_string),
        })
    }

    fn model_from(&mut self, node: &Node) -> Option<Model> {
        let name = node.attr("name").unwrap_or("").to_string();
        let Some(src) = node.value("Src").map(str::to_string) else {
            self.skipped.push(format!("<Model {name}> has no Src"));
            return None;
        };
        Some(Model {
            name,
            src,
            position: [
                number(&self.constants, node.value("x")).unwrap_or(0.0),
                number(&self.constants, node.value("y")).unwrap_or(0.0),
                number(&self.constants, node.value("z")).unwrap_or(0.0),
            ],
        })
    }

    /// A sprite by widget name.
    #[must_use]
    pub fn sprite(&self, name: &str) -> Option<&Sprite> {
        self.sprites.iter().find(|s| s.name == name)
    }

    /// A label by widget name.
    #[must_use]
    pub fn label(&self, name: &str) -> Option<&Label> {
        self.labels.iter().find(|l| l.name == name)
    }

    /// The outline colour a widget with no `BorderColor` takes.
    ///
    /// The layout's own `HudBGColour`, falling back to opaque black. See
    /// [`BORDER_CONSTANT`].
    #[must_use]
    pub fn default_border(&self) -> [f32; 4] {
        self.constants
            .get(BORDER_CONSTANT)
            .and_then(|v| parse_argb(v))
            .map_or(FALLBACK_BORDER, argb_to_rgba)
    }

    /// A fill by widget name.
    #[must_use]
    pub fn fill(&self, name: &str) -> Option<&Fill> {
        self.fills.iter().find(|f| f.name == name)
    }

    /// How many widgets this layout draws in screen space.
    #[must_use]
    pub fn widget_count(&self) -> usize {
        self.sprites.len() + self.fills.len() + self.labels.len()
    }
}

/// Widget-name prefixes whose authored position is **not** a screen coordinate.
///
/// `PosTag0`-`PosTag7` and `PlrTag0`-`PlrTag7` are the floating opponent tags:
/// each is anchored to a rival's projected screen position at runtime, and its
/// authored `x`/`y` plus the enclosing `<Item>` offset are a *nudge* from that
/// anchor. The arcade layout puts them inside `<Item OffsetX="-40">`, so they
/// resolve to negative coordinates - correct for an offset, nonsense for a
/// position.
///
/// This exists because the on-screen check is otherwise the sharpest test of the
/// `<Item>` handling, and these eight would force it to be dropped entirely.
pub const RUNTIME_ANCHORED: &[&str] = &["PosTag", "PlrTag"];

/// Whether a widget's authored position is a screen coordinate at all.
///
/// See [`RUNTIME_ANCHORED`].
#[must_use]
pub fn is_screen_positioned(name: &str) -> bool {
    !RUNTIME_ANCHORED
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

fn collect_constants(node: &Node, out: &mut HashMap<String, String>) {
    if node.name.eq_ignore_ascii_case("Variable")
        && let Some(name) = node.attr("global")
        && let Some(value) = node.value("String")
    {
        out.insert(name.to_string(), value.to_string());
    }
    for child in &node.children {
        collect_constants(child, out);
    }
}

/// Ticks per second the race clock counts at.
///
/// The simulation is fixed at 60 Hz ([ADR-0007]), so a tick is 1/60 s here even
/// though the original integrates a measured frame duration. **The original's own
/// race clock does not count emulated frames at all** - two laps of 3,069 and
/// 3,087 ticks were timed by the game at `0.50.25` and `1.11.08` - so this is
/// our clock keeping our time, not a reproduction of theirs.
///
/// [ADR-0007]: ../../../docs/architecture/adr/0007-fixed-timestep-vs-original.md
const TICKS_PER_SECOND: f64 = 60.0;

/// What the HUD needs to know about the race this frame.
///
/// A plain snapshot, deliberately: [`draw_list`] takes this rather than a
/// `&Race`, so the whole HUD is testable without a world, a track or a GPU. Every
/// field is a value the simulation already has or a documented gap.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Readout {
    /// Speed in km/h - `|velocity| * 3.6`, using the recovered factor in
    /// [`oag_render::exhaust::SPEED_TO_KMH`].
    pub speed_kmh: f32,
    /// The speed the bar reads full at, in km/h.
    ///
    /// **Ours, not recovered.** The original's bar has some maximum and nothing
    /// has established what sets it - a speed class's top speed, a constant, or
    /// the craft's `<Engine>` figures. [`DEFAULT_SPEED_FULL_KMH`] explains the
    /// placeholder.
    pub speed_full_kmh: f32,
    /// Shield energy remaining.
    pub shield: f32,
    /// The pool it started from, from `<Misc shield/>`.
    pub shield_max: f32,
    /// Lap the ship is on, 1-based. Zero means unknown.
    pub lap: u32,
    /// Laps in the race. Zero means unknown.
    pub laps: u32,
    /// Race position, 1-based. Zero means unknown.
    pub place: u32,
    /// Ships in the race.
    pub ships: u32,
    /// Ticks since the race began.
    pub race_ticks: u64,
    /// Ticks since the current lap began.
    pub lap_ticks: u64,
    /// The best lap so far, in ticks.
    pub best_lap_ticks: Option<u32>,
    /// Whether the ship is pointing back down the track.
    pub wrong_way: bool,
    /// The zone number, Zone mode only.
    ///
    /// Zero outside Zone mode and before the first step, which is also what the
    /// original starts it at: the mode object zeroes its counter in the
    /// constructor and the first step happens ten seconds in.
    pub zone: u32,
    /// Zone mode's score.
    pub score: i32,
}

/// What [`Readout::speed_full_kmh`] defaults to.
///
/// **A placeholder, and labelled one so it is never mistaken for a recovered
/// value.** The one bounded piece of evidence about the original's speed range is
/// the exhaust's own ramp, which saturates at **600 km/h**
/// (`oag_render::exhaust::RAMP_FLOOR_KMH` + `RAMP_SPAN_KMH`, both recovered from
/// the instruction stream). That is a ramp for a visual effect, not a bar
/// maximum, so using it here is an inference and not a small one.
///
/// What would retire it: a reference frame at a known speed, read off the bar's
/// fill fraction. Until then the bar's *shape* is right and its *scale* is ours.
pub const DEFAULT_SPEED_FULL_KMH: f32 = 600.0;

impl Readout {
    /// A readout with nothing known, for a test or a race that has not started.
    #[must_use]
    pub fn blank() -> Self {
        Self {
            speed_full_kmh: DEFAULT_SPEED_FULL_KMH,
            ..Self::default()
        }
    }

    /// How full the speed bar is, `0..=1`.
    #[must_use]
    pub fn speed_fraction(&self) -> f32 {
        fraction(self.speed_kmh, self.speed_full_kmh)
    }

    /// How full the shield bar is, `0..=1`.
    #[must_use]
    pub fn shield_fraction(&self) -> f32 {
        fraction(self.shield, self.shield_max)
    }
}

/// `value / full`, clamped, and zero rather than `NaN` when `full` is zero.
///
/// The zero case is not hypothetical: a shield pool is zero until a ship's
/// handling stats are loaded, and `0.0 / 0.0` would put a `NaN` into a vertex
/// buffer, which draws nothing at all rather than an empty bar.
fn fraction(value: f32, full: f32) -> f32 {
    if full <= 0.0 {
        return 0.0;
    }
    (value / full).clamp(0.0, 1.0)
}

/// How many fractional digits a time readout carries.
///
/// **The original uses both, on the same screen.** Read off a reference frame of a
/// time trial: `best` shows `0.00.00` and `current` shows `1.27.9`, so the digit
/// count differs between two widgets a few pixels apart. The running clocks show
/// tenths and the recorded best shows hundredths - which also squares with the
/// `best 1.11.08` that `docs/tools/oag-trace.md` recorded from a lap boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    /// `m.ss.h`, for a clock that is still running.
    Tenths,
    /// `m.ss.hh`, for a time that has been set.
    Hundredths,
}

/// Formats a tick count the way the original writes a time.
///
/// Minutes are not zero-padded, seconds and the fraction are, and the separator is
/// a full stop rather than a colon - observed as `1.11.08`, `0.50.25`, `1.27.9` and
/// `0.29.0`.
#[must_use]
pub fn format_lap_time(ticks: u64, precision: Precision) -> String {
    let total = ticks as f64 / TICKS_PER_SECOND;
    let minutes = (total / 60.0).floor();
    let seconds = total - minutes * 60.0;
    let whole = seconds.floor();
    let fraction = seconds - whole;
    // Truncated rather than rounded: a lap timed at 1.11.089 has not reached
    // 1.11.09, and a clock that rounds up can show a best lap it never ran.
    match precision {
        Precision::Tenths => {
            let tenths = (fraction * 10.0).floor();
            format!("{minutes:.0}.{whole:02.0}.{tenths:.0}")
        }
        Precision::Hundredths => {
            let hundredths = (fraction * 100.0).floor();
            format!("{minutes:.0}.{whole:02.0}.{hundredths:02.0}")
        }
    }
}

/// The colour a glyph's baked outline takes when a widget names none.
///
/// The layouts declare `HudBGColour` and every widget that *does* carry a
/// `BorderColor` points at it, so using it for the 57 that do not is a
/// data-driven default rather than an invented one. Resolved from the layout;
/// [`FALLBACK_BORDER`] covers a layout that declares no such constant.
///
/// **Why a dark outline at all**: a reference frame of the original shows one on
/// every HUD-font widget, including `CurrentTime` and `BestTime`, which carry no
/// `BorderColor`. An earlier revision drew those with a transparent outline on the
/// grounds that it invented nothing; the frame says the original does draw one.
const BORDER_CONSTANT: &str = "HudBGColour";

/// What the outline falls back to when the layout declares no `HudBGColour`.
///
/// Opaque black. Every shipped layout declares the constant, so this is a
/// belt-and-braces default rather than a value in use.
const FALLBACK_BORDER: [f32; 4] = [0.0, 0.0, 0.0, 1.0];

/// Whether a widget is one of the bar fills, whose width tracks a value.
///
/// The two are recognised by name because the layout gives no other signal: a
/// fill and its background are identical in every attribute but colour. See
/// `each_bar_exactly_overlays_its_own_background` in the ground-truth tests.
fn bar_fraction(name: &str, readout: &Readout) -> Option<f32> {
    match name {
        "SpeedBar" => Some(readout.speed_fraction()),
        "ShieldBar" => Some(readout.shield_fraction()),
        _ => None,
    }
}

/// Crops a sprite horizontally to `fraction` of its width.
///
/// **The fill model, and it is confidence 80 rather than higher.** A bar and its
/// background share rectangle *and* source rectangle exactly, differing only in
/// colour, which leaves a horizontal crop of the same art as the only way the
/// fill can work - there is no second piece of geometry for it to be. What is not
/// established is that the crop is *linear* in the value, or that it grows from
/// the left. A reference frame at a known speed settles both.
fn crop_horizontally(sprite: &Sprite, fraction: f32) -> Sprite {
    let mut out = sprite.clone();
    out.rect[2] = sprite.rect[2] * fraction;
    out.uv[2] = sprite.uv[2] * fraction;
    out
}

/// The text a widget shows, or `None` when it shows nothing this frame.
///
/// **An allow-list, deliberately.** A widget this function does not name is not
/// drawn, so a layout carrying 35 text widgets - most of them for modes,
/// opponents and progression that do not exist yet - produces only the handful
/// that have a real source. The alternative, drawing every widget and letting the
/// empty ones be invisible, hides the difference between "no value" and "not
/// wired up".
fn text_for(
    label: &Label,
    readout: &Readout,
    strings: &crate::language::StringTable,
) -> Option<String> {
    // A literal in the XML wins for the widgets that have one: `LapOf`'s "/" is
    // the separator between lap and total, and it is authored rather than
    // computed.
    let literal =
        |fallback: &str| -> String { label.string.clone().unwrap_or_else(|| fallback.to_string()) };

    match label.name.as_str() {
        // Values.
        "SpeedBarText" => Some(format!("{:.0} kmh", readout.speed_kmh.max(0.0))),
        // A **percentage**, not the raw pool: the reference frame reads `100%` on
        // an undamaged craft, and `<Misc shield/>` is a few hundred units.
        "ShieldBarText" => Some(format!("{:.0}%", readout.shield_fraction() * 100.0)),
        "Lap" => (readout.lap > 0).then(|| readout.lap.to_string()),
        "Lap Outof" => (readout.laps > 0).then(|| readout.laps.to_string()),
        "Position" => (readout.place > 0).then(|| readout.place.to_string()),
        "Position Outof" => (readout.ships > 0).then(|| readout.ships.to_string()),
        // Running clocks: tenths. See [`Precision`].
        "CurrentTime" => Some(format_lap_time(readout.lap_ticks, Precision::Tenths)),
        "TotalTime" => Some(format_lap_time(readout.race_ticks, Precision::Tenths)),
        // A time that has been set: hundredths. With none set the original shows
        // zeros rather than a dash placeholder - `best 0.00.00` on the reference
        // frame - so an absent best formats as zero rather than as its own string.
        "BestTime" => Some(format_lap_time(
            u64::from(readout.best_lap_ticks.unwrap_or(0)),
            Precision::Hundredths,
        )),

        // Zone mode. The number and the score are the two widgets with a real
        // source; `Zone_Bar_*` is the mode's own graphic and nothing was found
        // that writes it, so it stays off the list. See
        // `docs/ghidra/functions/psp-pulse-usa/zone-mode.md`.
        //
        // Drawn from zone 1 onward: the counter is zero for the first ten
        // seconds of a run, and a HUD reading `ZONE 0` would be reporting a zone
        // the player is not in yet.
        "Zone" => (readout.zone > 0).then(|| readout.zone.to_string()),
        "Score" => (readout.zone > 0).then(|| readout.score.to_string()),

        // Separators, authored as literals. Drawn only when both sides have a
        // value: a lone `/` between two blanks reads as a rendering fault, and
        // with lap counting unrecovered that is the common case rather than a
        // corner one.
        "LapOf" => (readout.lap > 0 && readout.laps > 0).then(|| literal("/")),
        "PositionOf" => (readout.place > 0 && readout.ships > 0).then(|| literal("/")),

        // Conditional.
        "WrongWay" => readout.wrong_way.then(|| {
            label.idstring.as_deref().map_or_else(
                || "WRONG WAY".to_string(),
                |id| strings.get_or_id(id).to_string(),
            )
        }),

        // Static labels: anything with an `idstring` that this function has not
        // already claimed. These are the `IG_HUD_*` captions beside the values.
        _ => label
            .idstring
            .as_deref()
            .map(|id| strings.get_or_id(id).to_string()),
    }
}

/// Sprite widgets drawn whenever the HUD is up.
///
/// The bars, their backgrounds and the time icon. Everything else in the layouts
/// is a weapon icon, a warning, an opponent tag or a mode-specific piece, none of
/// which has anything driving it - see `docs/ui/hud.md`.
const LIVE_SPRITES: &[&str] = &[
    "SpeedBarBg",
    "ShieldBarBg",
    "SpeedBar",
    "ShieldBar",
    "SpeedBarMark",
    "ShieldBarMark",
    "TimeIcon",
];

/// Everything the HUD needs that does not change from frame to frame.
///
/// Bundled rather than passed as four arguments because the caller assembles it
/// once at race load and holds it for the whole race.
#[derive(Debug, Clone, Copy)]
pub struct Context<'a> {
    /// The parsed layout for this race's mode.
    pub layout: &'a Layout,
    /// The language's string table, for `idstring` captions.
    pub strings: &'a crate::language::StringTable,
    /// Where [`ATLAS`] sits in the shared sprite sheet, in sheet pixels.
    pub atlas_origin: (f32, f32),
    /// Line height of the `HUD` font, **as actually loaded**.
    ///
    /// Not the 25 the XML role table implies: if the disc font is missing the
    /// caller falls back to the 5x7 built-in set, whose line height is 8, and
    /// vertical alignment has to follow what is really on screen or every
    /// bottom-aligned value drifts by the difference.
    pub hud_line_height: f32,
    /// Line height of the `HUDSmall` font, as actually loaded.
    pub small_line_height: f32,
    /// What a widget with no `BorderColor` outlines its glyphs in.
    ///
    /// 57 of the 84 HUD-font widgets are in that position, and the original draws
    /// an outline on them regardless - see [`BORDER_CONSTANT`].
    pub default_border: [f32; 4],
}

/// One frame's worth of HUD, split by which pass draws it.
///
/// Three lists rather than one because a [`crate::render::Renderer`] binds
/// exactly one font atlas, so the two text fonts cannot share a pass. Draw them
/// in field order: sprites, then `HUD` text, then `HUDSmall` text.
///
/// Splitting text across two passes means paint order is no longer strictly the
/// layout's document order *between* fonts. That is safe here because no label
/// overlaps another in any shipped layout - asserted by
/// `no_two_live_labels_overlap` rather than assumed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
    /// Bars, backgrounds and icons, in paint order.
    pub sprites: Vec<Draw>,
    /// Text in the `HUD` font: the values.
    pub hud_text: Vec<Draw>,
    /// Text in the `HUDSmall` font: the captions.
    pub small_text: Vec<Draw>,
}

impl Frame {
    /// Whether there is nothing at all to draw.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sprites.is_empty() && self.hud_text.is_empty() && self.small_text.is_empty()
    }

    /// How many quads' worth of work this frame is, for a log line.
    #[must_use]
    pub fn len(&self) -> usize {
        self.sprites.len() + self.hud_text.len() + self.small_text.len()
    }
}

/// Turns a widget's anchored `y` into the top edge the renderer draws text at.
///
/// [`crate::render::Renderer::render`] positions a line by its top edge, and the
/// layout anchors 94 of its 127 text widgets by their bottom or middle instead.
/// Ignoring that puts a bottom-anchored value a whole line below where it belongs:
/// on the lap counter, the difference between sitting on its caption and sitting
/// under the screen edge.
fn top_edge(label: &Label, line_height: f32) -> f32 {
    let height = line_height * label.scale;
    match label.vertalign {
        VertAlign::Top => label.y,
        VertAlign::Middle => label.y - height / 2.0,
        VertAlign::Bottom => label.y - height,
    }
}

/// Builds the frame's draw list from a layout and a readout.
///
/// Paint order within each list is the layout's own document order, which is
/// back-to-front - the same rule [`crate::render::Renderer`] applies to every
/// other draw list in the game.
#[must_use]
pub fn draw_list(cx: &Context<'_>, readout: &Readout) -> Frame {
    let mut frame = Frame::default();

    for sprite in &cx.layout.sprites {
        if !LIVE_SPRITES.contains(&sprite.name.as_str()) {
            continue;
        }
        match bar_fraction(&sprite.name, readout) {
            // A bar at zero is not drawn at all: a zero-width quad is a
            // degenerate triangle pair, and asking the rasteriser to do nothing
            // is worse than not asking.
            Some(fraction) if fraction <= 0.0 => {}
            Some(fraction) => frame.sprites.push(sprite_draw(
                &crop_horizontally(sprite, fraction),
                cx.atlas_origin,
            )),
            None => frame.sprites.push(sprite_draw(sprite, cx.atlas_origin)),
        }
    }

    for label in &cx.layout.labels {
        if !is_screen_positioned(&label.name) {
            continue;
        }
        let Some(text) = text_for(label, readout, cx.strings) else {
            continue;
        };
        if text.is_empty() {
            continue;
        }
        let small = label.font == Font::Small;
        let line_height = if small {
            cx.small_line_height
        } else {
            cx.hud_line_height
        };
        let draw = Draw::Text {
            x: label.x,
            y: top_edge(label, line_height),
            scale: label.scale,
            color: label.color,
            // The layout's own `BorderColor`, which is what makes the HUD fonts'
            // baked outline visible as an outline rather than as more glyph. The
            // 57 widgets that name none still get one - the original draws it -
            // from the layout's `HudBGColour`. See `crate::font::Atlas::luma`.
            border: Some(label.border.unwrap_or(cx.default_border)),
            align: label.align,
            text,
        };
        if small {
            frame.small_text.push(draw);
        } else {
            frame.hud_text.push(draw);
        }
    }

    frame
}

/// Everything the HUD needs off the disc.
///
/// Grouped rather than spread across [`crate::race::Loaded`] because it is a
/// unit: without the layout none of the rest is usable, and the whole thing
/// degrades together.
#[derive(Debug)]
pub struct Assets {
    /// The mode's parsed layout, or `None` when it could not be read.
    ///
    /// `None` means **no HUD is drawn at all**, and that is the honest outcome:
    /// the geometry is the disc's and there is nothing of ours to stand in for it.
    /// A HUD invented on the spot would be a worse failure than none, because it
    /// would look like a working HUD in the wrong place.
    pub layout: Option<Layout>,
    /// The `PulseHUD.mip` atlas, packed into a sheet with its placement known.
    pub sheet: crate::sprite::Sheet,
    /// The `HUD` font, or the built-in 5x7 set when the disc's is unreadable.
    pub font: crate::font::Atlas,
    /// The `HUDSmall` font, likewise.
    pub small_font: crate::font::Atlas,
    /// The language's string table, for the `IG_HUD_*` captions.
    pub strings: crate::language::StringTable,
}

impl Assets {
    /// Where [`ATLAS`] sits in [`Self::sheet`], in sheet pixels.
    ///
    /// `(0, 0)` when the atlas is not in the sheet, which pairs with a `None`
    /// layout - nothing is drawn either way, so the value never reaches a shader.
    #[must_use]
    pub fn atlas_origin(&self) -> (f32, f32) {
        self.sheet
            .get(ATLAS)
            .map_or((0.0, 0.0), |placed| (placed.x as f32, placed.y as f32))
    }
}

/// The HUD's two renderers and the data they draw.
///
/// # Why two renderers
///
/// [`crate::render::Renderer`] binds exactly one font atlas into an immutable
/// bind group and picks its sampler filter once from `Atlas::is_real`. The HUD
/// uses two fonts - `HUD` for values and `HUDSmall` for captions - so it needs two
/// of them and two passes. The alternative, a second atlas binding plus a third
/// `mode` value in `ui.wgsl`, touches the bind group every existing screen depends
/// on; two renderers touch nothing. See `docs/ui/hud.md`.
///
/// Both are built with `video: None`. A video pipeline that is built and never
/// filled draws a **green** rectangle rather than nothing, the planes being zeroed
/// rather than absent - see `crate::render`.
pub struct Overlay {
    layout: Layout,
    strings: crate::language::StringTable,
    atlas_origin: (f32, f32),
    hud_line_height: f32,
    small_line_height: f32,
    /// Draws the values, in `PulseHud.fnt`.
    values: crate::render::Renderer,
    /// Draws the captions, in `small.fnt`.
    captions: crate::render::Renderer,
}

impl std::fmt::Debug for Overlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Overlay")
            .field("widgets", &self.layout.widget_count())
            .field("strings", &self.strings.len())
            .field("atlas_origin", &self.atlas_origin)
            .finish()
    }
}

impl Overlay {
    /// Builds the two renderers, or `None` when there is no layout to draw.
    ///
    /// # Errors
    ///
    /// Propagates pipeline creation.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        assets: &Assets,
    ) -> anyhow::Result<Option<Self>> {
        let Some(layout) = assets.layout.clone() else {
            return Ok(None);
        };

        let values = crate::render::Renderer::new(
            device,
            queue,
            format,
            None,
            assets.font.clone(),
            &assets.sheet,
        )?;
        // The captions renderer never draws a sprite, but it binds the sheet
        // anyway: `Renderer::new` takes one unconditionally, and a second copy of
        // a 256x256 atlas is cheaper than making the parameter optional.
        let captions = crate::render::Renderer::new(
            device,
            queue,
            format,
            None,
            assets.small_font.clone(),
            &assets.sheet,
        )?;

        Ok(Some(Self {
            hud_line_height: assets.font.line_height,
            small_line_height: assets.small_font.line_height,
            atlas_origin: assets.atlas_origin(),
            strings: assets.strings.clone(),
            layout,
            values,
            captions,
        }))
    }

    /// The context [`draw_list`] takes.
    #[must_use]
    pub fn context(&self) -> Context<'_> {
        Context {
            layout: &self.layout,
            strings: &self.strings,
            atlas_origin: self.atlas_origin,
            hud_line_height: self.hud_line_height,
            small_line_height: self.small_line_height,
            default_border: self.layout.default_border(),
        }
    }

    /// Draws the HUD **over** whatever is already in `view`.
    ///
    /// Two passes, both `LoadOp::Load`: sprites and values through the `HUD`
    /// renderer, then captions through the `HUDSmall` one. The sprites go with the
    /// values because they share a renderer and the sprite sheet is bound in both.
    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        readout: &Readout,
        viewport: (f32, f32, f32, f32),
    ) {
        let frame = draw_list(&self.context(), readout);
        if frame.is_empty() {
            return;
        }

        // Sprites first so text sits over the bars, and both in one pass because
        // one renderer holds both the sheet and the value font.
        let mut first = frame.sprites;
        first.extend(frame.hud_text);
        if !first.is_empty() {
            self.values
                .overlay(device, queue, encoder, view, &first, viewport);
        }
        if !frame.small_text.is_empty() {
            self.captions
                .overlay(device, queue, encoder, view, &frame.small_text, viewport);
        }
    }
}

/// Whether a rectangle lies wholly inside the PSP's screen.
///
/// Used by the layout tests rather than by the renderer: a widget that lands
/// outside is a parser bug, and the parser is the thing under test.
///
/// **PSP only, and measurably so.** The PS2 release authors the same layouts in
/// its own 640x448 grid - `Data\XML\Arcade_HUD.xml` on `pulse-ps2-eu.chd` is
/// the PSP file's coordinates scaled by 640/480 and 448/272 to four digits
/// (`x` reaches 321 against 241, `y` 435 against 264, ratios 1.3320 and 1.6477),
/// the same scaling `crate::frontend::Space`'s PS2 evidence records for
/// `Skin.xml`. So a PS2 layout checked against [`SCREEN`] fails on nearly every
/// widget, which would say nothing about the parser. A PS2 sweep wants
/// `Space::PS2.size` here and has not been done; the runtime draw path is
/// unaffected, since it takes its `screen` uniform from the source's own space.
#[must_use]
pub fn inside_screen(rect: [f32; 4]) -> bool {
    rect[0] >= 0.0
        && rect[1] >= 0.0
        && rect[0] + rect[2] <= SCREEN.0
        && rect[1] + rect[3] <= SCREEN.1
}

/// Draws a sprite widget at its authored geometry.
///
/// Trivial, and it exists so that no caller has to remember that `rect` is the
/// destination and `uv` the source: getting those the wrong way round produces a
/// picture rather than an error.
#[must_use]
pub fn sprite_draw(sprite: &Sprite, atlas_origin: (f32, f32)) -> Draw {
    Draw::Sprite {
        rect: sprite.rect,
        // The atlas is packed into a shared sheet, so its own pixel coordinates
        // are offset by wherever the packer put it. See `crate::sprite::Sheet`.
        uv: [
            atlas_origin.0 + sprite.uv[0],
            atlas_origin.1 + sprite.uv[1],
            sprite.uv[2],
            sprite.uv[3],
        ],
        color: sprite.color,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of the real `TimeTrial_HUD.xml`, cut to what this module reads.
    /// Every attribute spelling, the `FEConst->` indirection, the `<Item>`
    /// offsets and the mixed `centre`/`middle` vertical alignment are as they
    /// appear on the disc.
    const SAMPLE: &str = r#"
<Screen>
<Screen name="HUD">
<Variable global="HudColour1"><Values String="0xFFFFFFFF"></Values></Variable>
<Variable global="HudColour2"><Values String="0xFF7DEFC0"></Values></Variable>
<Variable global="HudColour3A"><Values String="0x60B5D7C8"></Values></Variable>
<Variable global="HudBGColour"><Values String="0x40000000"></Values></Variable>

<Mode3D>
<Values FirstPass="yes" OriginX="0.0" OriginY="35.0"></Values>
<Model name="ReadyGo" Enabletransition="0" Delay="0">
<Values Src="Data\HUD\Pulse_Ready_Go.vex" x="0.0" y="0.0" z="-70.0" ztest="0"></Values>
</Model>
</Mode3D>

<Item OffsetX="300" OffsetY="210">
<Image name="SpeedBarBg">
<Values x="6" y="10" width="168" height="26" U="6" V="0" TxtrWidth="168" TxtrHeight="26" CalcBlur="1" Color="FEConst->HudColour3A" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
<Text name="SpeedBarText">
<Values string="0 kmh" font="HUDSmall" align="right" scale="1.0" x="0" y="0" CalcBlur="1" Color="FEConst->HudColour1" BorderColor="FEConst->HudBGColour"></Values>
</Text>
</Item>

<Item OffsetX="5" OffsetY="5">
<Text name="LapTxt">
<Values idstring="IG_HUD_LAP" font="HUDSmall" scale="1.0" x="4" y="0" CalcBlur="1" Color="FEConst->HudColour2"></Values>
</Text>
<Text name="Lap">
<Values scale="1.0" font="HUD" align="left" vertalign="bottom" x="3" y="32" CalcBlur="1"/>
</Text>
</Item>

<Image name="PickupBackground">
<Values x="240" y="35" Centred="true" width="66" height="60" U="3" V="111" TxtrWidth="66" TxtrHeight="60" CalcBlur="1" Color="FEConst->HudColour1" Src="Data\HUD\Textures\PulseHUD.mip"></Values>
</Image>
<Text name="TimeDiffText">
<Values font="HUD" scale="0.6" align="centre" vertalign="middle" x="0" y="5"/>
</Text>
</Screen>
</Screen>
"#;

    #[test]
    fn the_sample_yields_every_widget_it_declares() {
        let layout = Layout::from_xml(SAMPLE);
        assert_eq!(layout.sprites.len(), 2);
        assert_eq!(layout.labels.len(), 4);
        assert_eq!(layout.models.len(), 1);
        assert_eq!(layout.widget_count(), 6);
        assert!(
            layout.skipped.is_empty(),
            "nothing should be skipped: {:?}",
            layout.skipped
        );
    }

    /// The single most important thing this parser does. A child of an `<Item>`
    /// is positioned relative to the group, and reading `x`/`y` as absolute puts
    /// the entire HUD in the top-left corner - a failure that looks like a
    /// layout bug rather than a parsing one.
    #[test]
    fn an_item_offsets_the_widgets_inside_it() {
        let layout = Layout::from_xml(SAMPLE);
        let bar = layout.sprite("SpeedBarBg").expect("SpeedBarBg");
        // 300 + 6, 210 + 10, and the size untouched.
        assert_eq!(bar.rect, [306.0, 220.0, 168.0, 26.0]);

        let lap = layout.label("Lap").expect("Lap");
        assert_eq!((lap.x, lap.y), (8.0, 37.0));
    }

    /// A widget outside any `<Item>` keeps its own coordinates.
    #[test]
    fn a_widget_outside_an_item_is_not_offset() {
        let layout = Layout::from_xml(SAMPLE);
        let diff = layout.label("TimeDiffText").expect("TimeDiffText");
        assert_eq!((diff.x, diff.y), (0.0, 5.0));
    }

    #[test]
    fn centred_makes_the_position_the_middle_of_the_rectangle() {
        let layout = Layout::from_xml(SAMPLE);
        let box_ = layout.sprite("PickupBackground").expect("PickupBackground");
        // x=240 is the screen's centre line, so a 66-wide box starts at 207.
        assert_eq!(box_.rect, [207.0, 5.0, 66.0, 60.0]);
        // The centring must not disturb the source rectangle.
        assert_eq!(box_.uv, [3.0, 111.0, 66.0, 60.0]);
    }

    #[test]
    fn feconst_colours_resolve_through_the_variable_table() {
        let layout = Layout::from_xml(SAMPLE);
        assert_eq!(
            layout.constants.get("HudColour2").map(String::as_str),
            Some("0xFF7DEFC0")
        );

        let bar = layout.sprite("SpeedBarBg").expect("SpeedBarBg");
        // 0x60B5D7C8 -> alpha 0x60, and the channels in RGBA order.
        let [r, g, b, a] = bar.color;
        assert!((a - 96.0 / 255.0).abs() < 1e-6, "alpha was {a}");
        assert!((r - 181.0 / 255.0).abs() < 1e-6, "red was {r}");
        assert!((g - 215.0 / 255.0).abs() < 1e-6, "green was {g}");
        assert!((b - 200.0 / 255.0).abs() < 1e-6, "blue was {b}");
    }

    /// An unresolvable colour must not silently become black: a black HUD on a
    /// dark track is invisible, and invisible reads as "not implemented".
    #[test]
    fn an_unknown_constant_leaves_the_colour_white() {
        let layout = Layout::from_xml(
            r#"<Screen><Image name="X"><Values x="0" y="0" width="4" height="4"
               Color="FEConst->NoSuchThing" Src="a.mip"/></Image></Screen>"#,
        );
        assert_eq!(layout.sprite("X").expect("X").color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn both_spellings_of_vertical_centring_mean_the_same_thing() {
        assert_eq!(VertAlign::parse("middle"), VertAlign::Middle);
        assert_eq!(VertAlign::parse("centre"), VertAlign::Middle);
        assert_eq!(VertAlign::parse("center"), VertAlign::Middle);
        assert_eq!(VertAlign::parse("bottom"), VertAlign::Bottom);
        // Anything else is the renderer's own native behaviour.
        assert_eq!(VertAlign::parse(""), VertAlign::Top);
    }

    #[test]
    fn the_three_font_roles_are_distinguished() {
        assert_eq!(Font::parse("HUD"), Font::Hud);
        assert_eq!(Font::parse("HUDSmall"), Font::Small);
        assert_eq!(Font::parse("hudsmall"), Font::Small);
        assert_eq!(Font::parse("Default"), Font::Default);
    }

    /// `TxtrWidth`/`TxtrHeight` are the *source* size and `width`/`height` the
    /// *destination* size, and they really do differ in the shipped data.
    #[test]
    fn a_sprite_can_sample_larger_than_it_draws() {
        let layout = Layout::from_xml(
            r#"<Screen><Image name="TimeDiffIcon"><Values x="-35" y="0" width="14" height="12"
               U="52" V="86" TxtrWidth="28" TxtrHeight="23" Src="a.mip"/></Image></Screen>"#,
        );
        let icon = layout.sprite("TimeDiffIcon").expect("TimeDiffIcon");
        assert_eq!(icon.rect, [-35.0, 0.0, 14.0, 12.0]);
        assert_eq!(icon.uv, [52.0, 86.0, 28.0, 23.0]);
    }

    /// A missing `U`/`V` pair falls back to the destination size rather than to
    /// zero, so a full-texture sprite draws the whole texture instead of nothing.
    #[test]
    fn a_sprite_with_no_source_rectangle_samples_its_own_size() {
        let layout = Layout::from_xml(
            r#"<Screen><Image name="Plain"><Values x="1" y="2" width="8" height="9" Src="a.mip"/></Image></Screen>"#,
        );
        assert_eq!(
            layout.sprite("Plain").expect("Plain").uv,
            [0.0, 0.0, 8.0, 9.0]
        );
    }

    /// `HeadToHeadBar`, the one widget of this kind on the disc, verbatim.
    ///
    /// It has a `Color` and no `Src`, so it is a solid rectangle - and reading it
    /// as a broken sprite loses it silently, which is what the parser did first.
    #[test]
    fn an_image_with_no_src_is_a_solid_fill_rather_than_a_broken_sprite() {
        let layout = Layout::from_xml(
            r#"<Screen>
            <Variable global="HudColour2"><Values String="0xFF7DEFC0"/></Variable>
            <Image name="HeadToHeadBar">
            <Values x="15" y="30" width="5" height="0" Color="FEConst->HudColour2" CalcBlur="1"></Values>
            </Image></Screen>"#,
        );
        assert!(layout.sprites.is_empty());
        assert!(
            layout.skipped.is_empty(),
            "a colour-only Image is not an error: {:?}",
            layout.skipped
        );
        let bar = layout.fill("HeadToHeadBar").expect("HeadToHeadBar");
        // The authored height really is zero - the bar's length is a runtime
        // quantity, so this must survive parsing rather than being defaulted.
        assert_eq!(bar.rect, [15.0, 30.0, 5.0, 0.0]);
        assert_eq!(bar.color, argb_to_rgba(0xFF7D_EFC0));
        assert_eq!(layout.widget_count(), 1);
    }

    /// The floating opponent tags are anchored at runtime, so their authored
    /// position is an offset and may legitimately be negative.
    #[test]
    fn the_opponent_tags_are_not_screen_positioned() {
        assert!(!is_screen_positioned("PosTag0"));
        assert!(!is_screen_positioned("PlrTag7"));
        assert!(is_screen_positioned("SpeedBar"));
        assert!(is_screen_positioned("Lap"));
    }

    #[test]
    fn a_nameless_widget_is_reported_rather_than_dropped_quietly() {
        let layout = Layout::from_xml(
            r#"<Screen><Image><Values x="0" y="0" width="1" height="1" Src="a.mip"/></Image></Screen>"#,
        );
        assert!(layout.sprites.is_empty());
        assert_eq!(layout.skipped.len(), 1);
    }

    #[test]
    fn the_atlas_offset_is_added_to_the_source_rectangle() {
        let sprite = Sprite {
            name: "X".to_string(),
            rect: [1.0, 2.0, 3.0, 4.0],
            uv: [10.0, 20.0, 30.0, 40.0],
            color: [1.0; 4],
            src: ATLAS.to_string(),
        };
        let Draw::Sprite { rect, uv, .. } = sprite_draw(&sprite, (100.0, 200.0)) else {
            panic!("expected a sprite draw");
        };
        assert_eq!(rect, [1.0, 2.0, 3.0, 4.0], "the destination must not move");
        assert_eq!(uv, [110.0, 220.0, 30.0, 40.0]);
    }

    #[test]
    fn inside_screen_rejects_what_falls_off_the_edge() {
        assert!(inside_screen([0.0, 0.0, 480.0, 272.0]));
        assert!(!inside_screen([0.0, 0.0, 481.0, 272.0]));
        assert!(!inside_screen([-1.0, 0.0, 10.0, 10.0]));
    }

    fn strings() -> crate::language::StringTable {
        crate::language::StringTable::default()
    }

    fn context<'a>(layout: &'a Layout, strings: &'a crate::language::StringTable) -> Context<'a> {
        Context {
            default_border: layout.default_border(),
            layout,
            strings,
            atlas_origin: (0.0, 0.0),
            hud_line_height: 25.0,
            small_line_height: 10.0,
        }
    }

    /// Every reading of this format taken off the running original.
    #[test]
    fn a_lap_time_is_formatted_the_way_the_original_writes_it() {
        // `best 1.11.08`, from a lap boundary - see docs/tools/oag-trace.md.
        let ticks = (71.08 * TICKS_PER_SECOND).round() as u64;
        assert_eq!(format_lap_time(ticks, Precision::Hundredths), "1.11.08");

        // `0.50.25` - under a minute, so the minute field is a bare zero.
        let ticks = (50.25 * TICKS_PER_SECOND).round() as u64;
        assert_eq!(format_lap_time(ticks, Precision::Hundredths), "0.50.25");

        // `current 1.27.9` and `record 0.29.0`, both off the 2026-07-30 reference
        // frame: the running clocks carry **one** fractional digit, not two.
        let ticks = (87.9 * TICKS_PER_SECOND).round() as u64;
        assert_eq!(format_lap_time(ticks, Precision::Tenths), "1.27.9");
        let ticks = (29.0 * TICKS_PER_SECOND).round() as u64;
        assert_eq!(format_lap_time(ticks, Precision::Tenths), "0.29.0");
    }

    #[test]
    fn a_lap_time_truncates_rather_than_rounding_up() {
        // 59/60 of a second is 0.983s: it has not reached 0.99, and must not be
        // shown as a whole second either.
        assert_eq!(format_lap_time(59, Precision::Hundredths), "0.00.98");
        assert_eq!(format_lap_time(60, Precision::Hundredths), "0.01.00");
        assert_eq!(format_lap_time(0, Precision::Hundredths), "0.00.00");
        // The same at one digit: 0.983s shows as .9, not 1.0.
        assert_eq!(format_lap_time(59, Precision::Tenths), "0.00.9");
    }

    /// A clock that rolls a minute over correctly is worth one assertion.
    #[test]
    fn the_minute_field_is_not_capped() {
        let ten_minutes = (600.0 * TICKS_PER_SECOND) as u64;
        assert_eq!(
            format_lap_time(ten_minutes, Precision::Hundredths),
            "10.00.00"
        );
    }

    #[test]
    fn a_bar_fraction_never_leaves_zero_to_one() {
        let mut readout = Readout::blank();
        readout.speed_full_kmh = 600.0;

        readout.speed_kmh = 0.0;
        assert!((readout.speed_fraction() - 0.0).abs() < 1e-6);
        readout.speed_kmh = 300.0;
        assert!((readout.speed_fraction() - 0.5).abs() < 1e-6);
        // Over the maximum clamps rather than overflowing the bar.
        readout.speed_kmh = 900.0;
        assert!((readout.speed_fraction() - 1.0).abs() < 1e-6);
        // Reverse reads as empty, not as a negative-width quad.
        readout.speed_kmh = -50.0;
        assert!((readout.speed_fraction() - 0.0).abs() < 1e-6);
    }

    /// A zero pool must give zero rather than `NaN`. A `NaN` width draws nothing
    /// at all, which looks exactly like the HUD not being wired up.
    #[test]
    fn an_empty_pool_gives_zero_rather_than_a_nan() {
        let readout = Readout {
            shield: 0.0,
            shield_max: 0.0,
            ..Readout::blank()
        };
        let f = readout.shield_fraction();
        assert!(f.is_finite(), "shield fraction was {f}");
        assert!((f - 0.0).abs() < 1e-6);
    }

    #[test]
    fn a_bar_is_cropped_horizontally_and_keeps_its_height() {
        let sprite = Sprite {
            name: "SpeedBar".to_string(),
            rect: [306.0, 220.0, 168.0, 26.0],
            uv: [6.0, 0.0, 168.0, 26.0],
            color: [1.0; 4],
            src: ATLAS.to_string(),
        };
        let half = crop_horizontally(&sprite, 0.5);
        assert_eq!(half.rect, [306.0, 220.0, 84.0, 26.0]);
        // The source has to be cropped by the same factor, or the art is squashed
        // into the shorter bar rather than clipped by it.
        assert_eq!(half.uv, [6.0, 0.0, 84.0, 26.0]);
    }

    #[test]
    fn a_bottom_anchored_label_is_lifted_by_its_own_line_height() {
        let label = Label {
            name: "Lap".to_string(),
            x: 8.0,
            y: 37.0,
            font: Font::Hud,
            scale: 1.0,
            color: [1.0; 4],
            border: None,
            align: Align::Left,
            vertalign: VertAlign::Bottom,
            idstring: None,
            string: None,
        };
        assert!((top_edge(&label, 25.0) - 12.0).abs() < 1e-6);

        // Scale multiplies the lift, because the drawn line is that much taller.
        let scaled = Label {
            scale: 0.6,
            ..label.clone()
        };
        assert!((top_edge(&scaled, 25.0) - 22.0).abs() < 1e-6);

        let middle = Label {
            vertalign: VertAlign::Middle,
            ..label.clone()
        };
        assert!((top_edge(&middle, 25.0) - 24.5).abs() < 1e-6);

        let top = Label {
            vertalign: VertAlign::Top,
            ..label
        };
        assert!((top_edge(&top, 25.0) - 37.0).abs() < 1e-6);
    }

    #[test]
    fn the_two_fonts_go_to_separate_passes() {
        let layout = Layout::from_xml(SAMPLE);
        let strings = strings();
        let readout = Readout {
            speed_kmh: 300.0,
            lap: 2,
            ..Readout::blank()
        };
        let frame = draw_list(&context(&layout, &strings), &readout);

        // `SpeedBarText` and `LapTxt` are HUDSmall; `Lap` is HUD.
        let hud: Vec<&str> = frame
            .hud_text
            .iter()
            .filter_map(|d| match d {
                Draw::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(hud, ["2"], "the lap value belongs in the HUD font");

        assert_eq!(frame.small_text.len(), 2, "{:?}", frame.small_text);
        assert!(!frame.is_empty());
    }

    /// A blank readout must still produce a valid frame rather than panicking or
    /// emitting a degenerate quad: this is what the first frame of a race is.
    #[test]
    fn a_blank_readout_draws_the_furniture_and_no_bars() {
        let layout = Layout::from_xml(SAMPLE);
        let strings = strings();
        let frame = draw_list(&context(&layout, &strings), &Readout::blank());

        for draw in &frame.sprites {
            let Draw::Sprite { rect, .. } = draw else {
                panic!("expected sprites only");
            };
            assert!(
                rect[2] > 0.0 && rect[3] > 0.0,
                "a zero-area quad reached the draw list: {rect:?}"
            );
        }
        // The background is furniture and is always there; the fill is not.
        assert!(
            !frame.sprites.is_empty(),
            "the bar background should still draw"
        );
        // Lap is unknown at 0, so the value is omitted rather than shown as "0".
        assert!(
            frame.hud_text.is_empty(),
            "nothing should claim a value it does not have: {:?}",
            frame.hud_text
        );
    }

    /// With no best lap set the original shows **zeros**, not a dash placeholder:
    /// `best 0.00.00` on the reference frame. An earlier revision here invented
    /// `-.--.--`, which is the kind of thing that looks deliberate forever.
    #[test]
    fn a_missing_best_lap_shows_zeros_at_hundredths() {
        let layout = Layout::from_xml(
            r#"<Screen><Text name="BestTime"><Values font="HUD" x="0" y="0"/></Text></Screen>"#,
        );
        let strings = strings();
        let frame = draw_list(&context(&layout, &strings), &Readout::blank());
        let Some(Draw::Text { text, .. }) = frame.hud_text.first() else {
            panic!("expected the best-time widget");
        };
        assert_eq!(text, "0.00.00");
    }

    /// The outline is what makes a pre-outlined glyph read as a glyph, and 57 of
    /// the 84 HUD-font widgets name no colour for it, so the default is doing real
    /// work rather than covering a corner case.
    #[test]
    fn a_widget_with_no_border_colour_still_gets_the_layouts_own() {
        let layout = Layout::from_xml(
            r#"<Screen>
            <Variable global="HudBGColour"><Values String="0x40000000"/></Variable>
            <Text name="CurrentTime"><Values font="HUD" x="0" y="0"/></Text>
            </Screen>"#,
        );
        assert_eq!(layout.default_border(), argb_to_rgba(0x4000_0000));
        // The widget itself declares none...
        assert_eq!(
            layout.label("CurrentTime").expect("CurrentTime").border,
            None
        );

        // ...and the draw list supplies one anyway.
        let strings = strings();
        let frame = draw_list(&context(&layout, &strings), &Readout::blank());
        let Some(Draw::Text { border, .. }) = frame.hud_text.first() else {
            panic!("expected the current-time widget");
        };
        assert_eq!(*border, Some(argb_to_rgba(0x4000_0000)));
    }

    /// A layout with no `HudBGColour` must still outline rather than fall through
    /// to "no border", which is what drew filled boxes.
    #[test]
    fn a_layout_with_no_background_constant_falls_back_to_black() {
        let layout = Layout::from_xml(
            r#"<Screen><Text name="CurrentTime"><Values font="HUD" x="0" y="0"/></Text></Screen>"#,
        );
        assert_eq!(layout.default_border(), FALLBACK_BORDER);
    }

    /// The shield readout is a percentage, not the raw `<Misc shield/>` pool -
    /// `100%` on the reference frame against a pool of several hundred units.
    #[test]
    fn the_shield_readout_is_a_percentage() {
        let layout = Layout::from_xml(
            r#"<Screen><Text name="ShieldBarText"><Values font="HUDSmall" x="0" y="0"/></Text></Screen>"#,
        );
        let strings = strings();
        let readout = Readout {
            shield: 300.0,
            shield_max: 300.0,
            ..Readout::blank()
        };
        let frame = draw_list(&context(&layout, &strings), &readout);
        let Some(Draw::Text { text, .. }) = frame.small_text.first() else {
            panic!("expected the shield readout");
        };
        assert_eq!(text, "100%");

        let readout = Readout {
            shield: 150.0,
            shield_max: 300.0,
            ..Readout::blank()
        };
        let frame = draw_list(&context(&layout, &strings), &readout);
        let Some(Draw::Text { text, .. }) = frame.small_text.first() else {
            panic!("expected the shield readout");
        };
        assert_eq!(text, "50%");
    }

    #[test]
    fn the_wrong_way_warning_only_appears_when_it_applies() {
        let layout = Layout::from_xml(
            r#"<Screen><Text name="WrongWay"><Values font="HUD" x="240" y="70"/></Text></Screen>"#,
        );
        let strings = strings();

        let quiet = draw_list(&context(&layout, &strings), &Readout::blank());
        assert!(quiet.is_empty(), "{quiet:?}");

        let readout = Readout {
            wrong_way: true,
            ..Readout::blank()
        };
        let warned = draw_list(&context(&layout, &strings), &readout);
        assert_eq!(warned.hud_text.len(), 1);
    }

    /// The opponent tags are runtime-anchored, so drawing them at their authored
    /// position would put text at negative coordinates.
    #[test]
    fn the_opponent_tags_are_never_drawn() {
        let layout = Layout::from_xml(
            r#"<Screen><Item OffsetX="-40" OffsetY="0">
            <Text name="PosTag0"><Values idstring="X" font="Default" x="0" y="20"/></Text>
            </Item></Screen>"#,
        );
        let strings = strings();
        let frame = draw_list(&context(&layout, &strings), &Readout::blank());
        assert!(frame.is_empty(), "{frame:?}");
    }

    /// Splitting text into two passes gives up strict document paint order between
    /// fonts, which is only safe if no two live labels overlap. Checked here on the
    /// sample and on the shipped layouts by the ground-truth suite.
    #[test]
    fn no_two_live_labels_overlap() {
        let layout = Layout::from_xml(SAMPLE);
        let strings = strings();
        let readout = Readout {
            speed_kmh: 300.0,
            lap: 2,
            laps: 3,
            ..Readout::blank()
        };
        let frame = draw_list(&context(&layout, &strings), &readout);

        let mut anchors: Vec<(f32, f32)> = Vec::new();
        for draw in frame.hud_text.iter().chain(frame.small_text.iter()) {
            let Draw::Text { x, y, .. } = draw else {
                continue;
            };
            assert!(
                !anchors.contains(&(*x, *y)),
                "two labels share the anchor ({x}, {y}), so paint order between the \
                 two font passes would decide which is visible"
            );
            anchors.push((*x, *y));
        }
    }
}
