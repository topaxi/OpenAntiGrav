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
//! [`Layout`] is geometry, straight from the file, and is this file.
//! [`draw_list`] decides only *which* widgets are live this frame and *what
//! text* they carry, from a [`Readout`], and is [`draw`] - its own file since
//! 2026-08-25, because the division is the point and a reader should be able to
//! see which side of it a given rule is on. Keeping the two apart is what lets
//! the whole HUD be tested without a GPU, the way [`crate::perf`] is - and it
//! means a layout change is a data change rather than a code change.
//!
//! Which widgets those are, out of which texture, and with which colour
//! substituted is a **per-title** table rather than this module's - three rows
//! that Pulse and HD/Fury measurably disagree on. See [`oag_title::HudArt`].
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

mod assets;
mod compose;
mod draw;
mod overlay;
mod widget;

pub use assets::Assets;
pub use compose::{Composed, compose};
pub use draw::{Context, Frame, draw_list, pickup_icon_name, sprite_draw};
pub use overlay::Overlay;
pub use widget::{Fill, Font, Label, Model, Sprite, VertAlign};

// The two the outline default is built from. Everything else `draw` decides is
// private to it; `tests` reaches those through `super::draw::*`.
use draw::{BORDER_CONSTANT, FALLBACK_BORDER};

use crate::frontend::{Align, Draw, SCREEN};
use crate::screen::{argb_to_rgba, parse_argb};

/// The atlas and the five layout entries: `oag_pulse::hud`.
///
/// Which files Pulse ships is a title fact and moved there under [ADR-0022];
/// everything that reads them - the widget model, the `<Item>` offset handling,
/// the draw list - is this module and is title-blind.
///
/// [ADR-0022]: ../../../docs/architecture/adr/0022-title-packages.md
pub use oag_pulse::hud::layouts;

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
        Self::from_tree(&fexml::parse(xml))
    }

    /// Reads an already-parsed tree.
    ///
    /// The entry point for a layout that is **more than one file**: HD/Fury
    /// composes each mode's HUD out of a shell plus a dozen `<LoadXML>`
    /// fragments, and [`compose`] splices those into one tree before this runs.
    /// Doing it on the tree rather than on text is what lets a fragment's
    /// `<Variable>` constants reach a widget in a different file, since the
    /// constant sweep below sees the whole composed document at once.
    #[must_use]
    pub fn from_tree(root: &Node) -> Self {
        let mut out = Self::default();

        // Constants first: a widget's colour can reference one declared anywhere
        // in the file, including after the widget itself.
        collect_constants(root, &mut out.constants);
        out.collect(root, 0.0, 0.0);
        out
    }

    /// Walks the tree, carrying the enclosing translation down.
    ///
    /// # `OffsetX`/`OffsetY` translate; `x`/`y` place
    ///
    /// Two coordinate attributes with two different meanings, and telling them
    /// apart is the whole rule. `OffsetX`/`OffsetY` move the element **and
    /// everything under it**; `x`/`y` place the element itself inside whatever
    /// translation it inherited. So the offsets **compose** down the tree and the
    /// positions do not.
    ///
    /// This paragraph said the opposite until 2026-08-17 - an `<Item>`'s offset
    /// was read as absolute, "because nothing in the shipped data nests them, so
    /// composing would be untested code". True of Pulse and false of the
    /// lineage: measured across Pulse's five layouts and HD/Fury's 73 HUD files,
    ///
    /// | | Pulse | HD/Fury |
    /// | --- | ---: | ---: |
    /// | elements carrying an offset | 26, **all `<Item>`** | 218 |
    /// | of those, widgets rather than `<Item>` | 0 | 116 |
    /// | widgets holding a widget | **0** | 242 |
    /// | offsets carried inside `<Values>` rather than on the element | 0 | 0 |
    ///
    /// so composing is a **strict no-op on Pulse** - nothing there ever inherits
    /// a non-zero offset - and is required to read HD at all. `Data\XML\
    /// HUD_Elim_info_text.xml` is the case that settles the second half:
    /// `<Text name="Info1" OffsetX="600" OffsetY="330">` carries `x="0" y="20"`
    /// of its own and holds three `<Image>` frame pieces at `x="-160"`, `0` and
    /// `160`, all `centred`. The text lands in the middle of its own frame only
    /// if the element's `x`/`y` and its children's are read in the *same*
    /// translated space - which is to say an element's own offset applies to
    /// itself too, not only to what it contains.
    fn collect(&mut self, node: &Node, offset_x: f32, offset_y: f32) {
        for child in &node.children {
            let name = child.name.as_str();

            // Read off the element rather than through `Node::value`: no shipped
            // layout on either disc puts an offset in a `<Values>` carrier, and
            // reaching into one would let a child's carrier answer for its
            // parent.
            let x = offset_x + number(&self.constants, child.attr("OffsetX")).unwrap_or(0.0);
            let y = offset_y + number(&self.constants, child.attr("OffsetY")).unwrap_or(0.0);

            if name.eq_ignore_ascii_case("Item") {
                self.collect(child, x, y);
                continue;
            }

            if name.eq_ignore_ascii_case("Image") {
                if let Some(sprite) = self.sprite_from(child, x, y) {
                    self.sprites.push(sprite);
                }
                self.collect(child, x, y);
                continue;
            }

            if name.eq_ignore_ascii_case("Text") {
                if let Some(label) = self.label_from(child, x, y) {
                    self.labels.push(label);
                }
                self.collect(child, x, y);
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
            self.collect(child, x, y);
        }
    }

    fn sprite_from(&mut self, node: &Node, offset_x: f32, offset_y: f32) -> Option<Sprite> {
        let name = anonymous_or(node);
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
        Some(Label {
            name: anonymous_or(node),
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

/// A widget's `name`, or the empty string when it has none.
///
/// **A nameless widget is drawn, not broken**, and this used to drop it: both
/// widget readers required a `name` and pushed "an `<Image>` with no name" onto
/// [`Layout::skipped`] otherwise. Pulse authors none - all five layouts name
/// every widget, which is why the rule survived - and HD/Fury authors **106**
/// against 2,214 named across its eighteen composed HUDs, most of them the
/// `BackgroundLayer="1"` panels sitting behind a named readout. Dropping those
/// leaves the numbers with nothing behind them.
///
/// The empty name is not a placeholder to be matched on: nothing in
/// [`draw_list`]'s tables is named `""`, so an anonymous widget carries geometry
/// and is never selected by name - which is exactly what an unaddressable widget
/// should do.
fn anonymous_or(node: &Node) -> String {
    node.attr("name").unwrap_or_default().to_string()
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
    /// What the craft is carrying, if anything.
    ///
    /// `None` on every tick of a race with weapons off, which is every mode but
    /// the single race - see `oag_race::Mode::weapons_enabled`.
    pub pickup: Option<oag_formats::weapons::Weapon>,
    /// The lock-on reticle, or `None` for a caller that has none to draw.
    ///
    /// `Copy` state off `Race`, passed whole rather than reduced to five
    /// rectangles here, so [`draw_list`] can resolve each piece against the
    /// layout's own `<Mode3D>` models - which is where the art comes from and
    /// what makes the four brackets one model drawn four ways. See
    /// [`crate::race::sight`].
    pub sight: Option<crate::race::sight::Sight>,
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

/// Whether a rectangle lies wholly inside the PSP's screen.
///
/// Used by the layout tests rather than by the renderer: a widget that lands
/// outside is a parser bug, and the parser is the thing under test.
///
/// **PSP only, and measurably so.** The PS2 release authors the same layouts in
/// its own 640x448 grid. Checked coordinate by coordinate rather than by the
/// extremes: `Data\XML\Arcade_HUD.xml` carries 141 coordinate values (69 `x`,
/// 72 `y`) on both discs in the same order, and **121 of the 141 are the PSP's
/// own value scaled by 640/480 or 448/272 and rounded to an integer**.
/// `Skin.xml` behaves the same way and was measured the same way - 30 of 43,
/// with its own exceptions - in `crates/game/tests/frontend_grid_ground_truth.rs`.
///
/// The 20 that are not divide cleanly, and neither is a counter-example:
///
/// - **18 are the nine `<Mode3D><Model>` placements**, every one of them at the
///   PSP's own `x="-240" y="136"` unchanged. Those sit in an `orthographic` 3D
///   mode rather than in the widget grid, and nothing here places them.
/// - **2 are `TimeDiffIcon`'s** `x` (-35 against a predicted -47) and `y` (-4
///   against -7), small negatives inside an `<Item>` - a nudge from an anchor
///   rather than a screen position, which is the same distinction
///   [`RUNTIME_ANCHORED`] exists for.
///
/// So a PS2 layout checked against [`SCREEN`] fails on nearly every widget -
/// `y` reaches 435 - which would say nothing about the parser. **A PS2 sweep
/// needs more than swapping in `Space::PS2.size` here**: the unscaled offsets
/// above mean it also has to agree with the [`RUNTIME_ANCHORED`] skip about
/// which coordinates are positions at all.
///
/// That is now a fact about this XML dialect rather than about one file.
/// `Skin.xml`'s `<Animation><Key>` carries `x`/`y` that are a *travel* and are
/// byte-identical on both consoles, exactly as `TimeDiffIcon`'s are here - two
/// files, two element names, one confusion. **`x` does not mean one thing**,
/// and anything that reads a coordinate has to know which kind it holds before
/// it can compare it to a screen. Not done. The runtime draw path is unaffected
/// either way, since it takes its `screen` uniform from the source's own space.
#[must_use]
pub fn inside_screen(rect: [f32; 4]) -> bool {
    rect[0] >= 0.0
        && rect[1] >= 0.0
        && rect[0] + rect[2] <= SCREEN.0
        && rect[1] + rect[3] <= SCREEN.1
}

#[cfg(test)]
mod reticle_tests;
#[cfg(test)]
mod tests;
