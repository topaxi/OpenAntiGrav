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
//! element from [`oag_tables::fexml`]'s known-element table was read as
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
//! the whole HUD be tested without a GPU, the way [`oag_present::perf`] is - and it
//! means a layout change is a data change rather than a code change.
//!
//! Which widgets those are, out of which texture, and with which colour
//! substituted is a **per-title** table rather than this module's - three rows
//! that Pulse and HD/Fury measurably disagree on. See [`oag_title::HudArt`].
//!
//! # Screen space
//!
//! Everything is in the PSP's 480x272 space ([`oag_display::space::SCREEN`]), which is what the XML's
//! coordinates are in, and the renderer letterboxes that into whatever the window
//! is. A widget inside an `<Item>` is positioned relative to that group's
//! `OffsetX`/`OffsetY`; this module resolves those to absolute coordinates at
//! parse time so nothing downstream has to carry the offset around.

use std::collections::HashMap;

use oag_tables::fexml::{self, Node};

mod assets;
pub mod assist;
mod compose;
mod dialect_2048;
mod draw;
mod head_to_head;
pub mod kill_tags;
mod lap_splits;
pub mod messages;
mod pickup;
mod runtime;
mod shield_bar;
mod sight_draw;
pub mod sprite;
mod time_trial_pace;
mod widget;

pub use assets::Assets;
pub use compose::{Composed, compose};
pub use draw::{Context, Frame, draw_list, sprite_draw};
pub use head_to_head::HeadToHead;
pub use kill_tags::KillTag;
pub use pickup::pickup_icon_name;
pub use time_trial_pace::{PaceTier, RecordTarget, TimeTrialPace, pace_for};
pub use widget::{Fill, Font, Label, Model, Sprite, VertAlign};

// The two the outline default is built from. Everything else `draw` decides is
// private to it; `tests` reaches those through `super::draw::*`.
use draw::{BORDER_CONSTANT, FALLBACK_BORDER};

use oag_ui::frontend::{Align, Draw};
use oag_ui::screen::{argb_to_rgba, parse_argb};

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
    /// [`oag_tables::fexml::expand`] first, which the archive reader does.
    ///
    /// Deliberately forgiving in the same way [`oag_tables::fexml::parse`] is:
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
        // The default outside any `<Mode3D>` matches the orthographic
        // dialect's own defaults - unreachable in shipped data (every
        // `<Model>` lives inside a `<Mode3D>`), kept as the safe fallback
        // rather than an invented one.
        out.collect(root, 0.0, 0.0, true, [0.0, 0.0]);
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
    ///
    /// `orthographic`/`origin` carry the enclosing `<Mode3D>` block's own
    /// dialect the same way `offset_x`/`offset_y` carry `<Item>`'s - see
    /// [`Model::orthographic`]'s doc for what the two dialects are and why a
    /// `<Model>` needs to know which one it was authored in.
    fn collect(
        &mut self,
        node: &Node,
        offset_x: f32,
        offset_y: f32,
        orthographic: bool,
        origin: [f32; 2],
    ) {
        for child in &node.children {
            let name = child.name.as_str();

            // Read off the element rather than through `Node::value`: no shipped
            // layout on either disc puts an offset in a `<Values>` carrier, and
            // reaching into one would let a child's carrier answer for its
            // parent.
            let x = offset_x + number(&self.constants, child.attr("OffsetX")).unwrap_or(0.0);
            let y = offset_y + number(&self.constants, child.attr("OffsetY")).unwrap_or(0.0);

            if name.eq_ignore_ascii_case("Mode3D") {
                // `mode`/`OriginX`/`OriginY` live on this element's own
                // `<Values>` carrier - `Node::value` reaches through to it,
                // `Node::attr` would not. Every shipped block is exactly one
                // of two shapes: `mode="orthographic"` with real screen
                // pixels on every child `x`/`y`, or `FirstPass`/`OriginX`/
                // `OriginY` with `x="0" y="0"` on every child - never both,
                // never neither.
                let mode_orthographic = child
                    .value("mode")
                    .is_some_and(|m| m.eq_ignore_ascii_case("orthographic"));
                let mode_origin = [
                    number(&self.constants, child.value("OriginX")).unwrap_or(0.0),
                    number(&self.constants, child.value("OriginY")).unwrap_or(0.0),
                ];
                self.collect(child, x, y, mode_orthographic, mode_origin);
                continue;
            }

            if name.eq_ignore_ascii_case("Item") {
                self.collect(child, x, y, orthographic, origin);
                continue;
            }

            if name.eq_ignore_ascii_case("Image") {
                if let Some(sprite) = self.sprite_from(child, x, y) {
                    self.sprites.push(sprite);
                }
                self.collect(child, x, y, orthographic, origin);
                continue;
            }

            if name.eq_ignore_ascii_case("Text") {
                if let Some(label) = self.label_from(child, x, y) {
                    self.labels.push(label);
                }
                self.collect(child, x, y, orthographic, origin);
                continue;
            }

            if name.eq_ignore_ascii_case("Model") {
                if let Some(model) = self.model_from(child, orthographic, origin) {
                    self.models.push(model);
                }
                continue;
            }

            // `Screen`, `Variable` and `Values` are containers or
            // already-handled carriers; everything else is walked into so a
            // widget nested somewhere unexpected is still found.
            self.collect(child, x, y, orthographic, origin);
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
                origin_y: offset_y,
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
            // `RotationTheta`, in radians about the rectangle's own centre. See
            // [`Sprite::rotation`]; `Node::value` folds the two spellings the
            // Zone layout ships.
            rotation: number(&self.constants, node.value("RotationTheta")).unwrap_or(0.0),
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

    fn model_from(&mut self, node: &Node, orthographic: bool, origin: [f32; 2]) -> Option<Model> {
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
            // Pure's own spelling, `colour` rather than `Color` - see
            // `Model::colour`'s doc for why this is read as an `Option` rather
            // than through the `colour()` helper's white-when-absent default.
            colour: node
                .value("colour")
                .and_then(|v| resolve(&self.constants, v))
                .and_then(parse_argb)
                .map(argb_to_rgba),
            orthographic,
            origin,
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

/// Widget-name prefixes this crate does not check the position of.
///
/// `PlrTag0`-`PlrTag7` (`MPTag_HUD.xml`, multiplayer only) are the real
/// runtime anchor this constant's name describes: their `<Values>` carries
/// no `x`/`y` at all. `PosTag0`-`PosTag7` used to be listed here and are not
/// any more: they resolve to a fixed on-screen column, `(405, 25..165)` on
/// the arcade layout and `(460, 25..165)` on the eliminator one, both
/// measured and pinned by `postag_is_a_fixed_column_not_a_runtime_anchor`
/// (`crates/game/tests/hud_layout_ground_truth.rs`), and the Eliminator's
/// kill column now draws in them - see [`kill_tags`]. See `docs/ui/hud.md`.
///
/// This exists because the on-screen check is otherwise the sharpest test of the
/// `<Item>` handling, and these eight would force it to be dropped entirely.
pub const RUNTIME_ANCHORED: &[&str] = &["PlrTag"];

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
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Readout {
    /// Speed in km/h - `|velocity| * 3.6`, using the recovered factor in
    /// [`oag_fx::exhaust::SPEED_TO_KMH`].
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
    /// Ticks of racing clock: since the start-line countdown released the
    /// craft, zero throughout it - [`oag_race::race_clock_ticks`]. It is the
    /// clock the `TotalTime` readout and the medal pace read, **not** the raw
    /// tick the start gantry and the thrust gate ride.
    pub race_ticks: u64,
    /// Ticks since the current lap began, likewise never counting the
    /// countdown - see [`oag_race::RaceState::lap_ticks`].
    pub lap_ticks: u64,
    /// The best lap so far, in ticks.
    pub best_lap_ticks: Option<u32>,
    /// Each completed lap's own time, in ticks, indexed by `lap - 1` - the same
    /// history [`oag_race::RaceState::lap_splits`] carries, for HD/Fury's
    /// `Lap1Image`-`Lap4Image` rows. `None` in a slot means that lap has not
    /// been completed and its row draws nothing, not a zero.
    pub lap_splits: [Option<u32>; oag_race::MAX_RECORDED_LAPS],
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
    /// Which rung of the Zone escalation ladder the race is on.
    ///
    /// The same index `crate::race::zone_grade::ZoneGrade` shows and
    /// `--zone-stage` selects, read against
    /// [`oag_title::HudArt::zone_speed_classes`] to name the speed class. Zero
    /// outside Zone mode and on a title whose ladder is unrecovered, which is
    /// where HD still rests: its own loader zeroes the stage and nothing
    /// recovered advances it.
    ///
    /// **Deliberately not derived from [`Self::zone`] here.** What zone sits on
    /// what rung is a per-title table (`oag_title::RaceDefaults::zone_stages`),
    /// recovered on 2048 and not on HD, and the maintainer's own play says the
    /// obvious guess - one rung per zone - is wrong.
    pub zone_stage: u32,
    /// How many rows down the ladder the **next** speed class begins, counted
    /// from the current zone's row.
    ///
    /// `None` outside Zone mode, on the top rung, and on a title whose ladder is
    /// unrecovered. HD's `NextSpeedClassBG` is authored on top of the current
    /// row and moved at runtime to the row of the zone its class starts at - the
    /// maintainer's own observation of the original - so this is what places it.
    /// Filled from `crate::race::Scene::zones_to_next_stage` at the two draw
    /// sites, for the same reason [`Self::zone_stage`] is.
    pub zone_next_in: Option<u32>,
    /// What the craft is carrying, if anything.
    ///
    /// `None` on every tick of a race with weapons off, which is every mode but
    /// the single race - see `oag_race::Mode::weapons_enabled`.
    pub pickup: Option<oag_tables::weapons::Weapon>,
    /// The lock-on reticle, or `None` for a caller that has none to draw.
    ///
    /// `Copy` state off `Race`, passed whole rather than reduced to five
    /// rectangles here, so [`draw_list`] can resolve each piece against the
    /// layout's own `<Mode3D>` models - which is where the art comes from and
    /// what makes the four brackets one model drawn four ways. See
    /// [`oag_race::sight`].
    pub sight: Option<oag_race::sight::Sight>,
    /// Whether the shield bar is inside its ~1 s post-hit flash window this
    /// frame.
    ///
    /// `Hud_UpdateEnergyBar`'s own `hud+0x11c` timer - a one-shot accumulator
    /// armed the instant the pool *drops* and running for about a second,
    /// re-read every frame rather than derived from [`Self::shield_fraction`]
    /// alone, because "just dropped" is not a pure function of the current
    /// value. The memory lives on [`crate::race::Race`], not here: a
    /// `Readout` is a fresh snapshot built every tick and has nowhere of its
    /// own to keep it. See
    /// `docs/ghidra/functions/psp-pulse-usa/shield.md#hud_updateenergybar-0x0881c638-tints-the-bar-from-a-20-threshold-not-a-gradient`.
    ///
    /// `false` on [`Self::blank`], the same as every other "nothing has run
    /// yet" field.
    pub shield_flashing: bool,
    /// HD's [`Self::shield_flashing`]: the post-hit window armed by a drop of
    /// the truncated whole percentage, `hud+0x110 > 0` in
    /// `Hud_UpdateShieldReadout`. See `Race::advance_shield_flash_whole`.
    pub shield_flashing_whole: bool,
    /// Whether the player is inside the one-second window after absorbing a
    /// pickup this frame - `Race::absorb_window_active(player_slot)`, the
    /// same state [`oag_fx::hull_overlay`] draws off.
    ///
    /// `Hud_UpdateEnergyBar` (`0x0881c638`) calls
    /// `HullOverlay_AbsorbWindowActive` on the player's own craft every
    /// frame and reads it twice: it **suppresses** the forced-red branch
    /// entirely (see [`Self::shield_forced_red`]) and it writes a flat
    /// `0xff`/`0x00` alpha into a second field on the `ShieldBar` widget
    /// (`+0xf4`) that stays off on every other frame this function ever
    /// touches. Confidence 84 for the call itself - this page's own ceiling
    /// for "decompilation only" per `docs/reverse-engineering/confidence-rubric.md`,
    /// a direct decompile with no VFPU, `program=BOOT.BIN` at
    /// `/pulse/BOOT-psp-pulse-usa.BIN`, plus the standing live corroboration
    /// that the call itself fires every frame (`cannon-quake-leachbeam.md`'s
    /// "2026-09-23 (later)" probe) - see
    /// `docs/ghidra/functions/psp-pulse-usa/shield.md#hud_updateenergybar-the-absorb-flash-2026-09-25`.
    /// **What `+0xf4` itself drives is not chased past the field write**:
    /// the widget's own draw method (through its vtable at `+0x38`) was not
    /// decompiled, so which texture/quad it gates is inferred - see that
    /// section's own confidence note - rather than read off the draw call.
    ///
    /// `false` on [`Self::blank`], the same as every other "nothing has run
    /// yet" field.
    pub shield_absorbing: bool,
    /// The shield bar's blink accumulator - `Hud_UpdateEnergyBar`'s own
    /// `hud+0x1dc`, in seconds, ported by
    /// `crate::race::Race::advance_shield_blink`.
    ///
    /// Not itself a "is blinking" flag - see [`Self::shield_blinking`] for
    /// that - just the raw accumulator, so [`Self::shield_blink_phase_on`]
    /// can apply the original's own `floor(t * 8.0)` parity test to
    /// whatever this holds, including a value frozen from an earlier blink
    /// that has since stopped. `0.0` on [`Self::blank`], which reads as the
    /// window's own start - **assumed for a fresh race**, see
    /// [`crate::race::view::View::shield_blink_timer`]'s own doc comment for
    /// what that assumption rests on.
    pub shield_blink_phase: f32,
    /// HD's [`Self::shield_blink_phase`], `hud+0x1e8`: the accumulator the
    /// original runs `floor(t * 8)` parity on, gated on the whole-percent
    /// window. See `Race::advance_shield_flash_whole`.
    pub shield_blink_phase_whole: f32,
    /// 2048's `EnergyBarDelay`: an exponentially-smoothed trail behind
    /// [`Self::shield_fraction`], `0..=1`.
    ///
    /// `Hud_UpdateEnergyBar` (`0x811957a2`, vita-2048-eu-v104) applies
    /// `lagging += (target - lagging) * 0.1` every tick, where `target`
    /// resolves to the current shield fraction on every path through the
    /// branch that sets it (a rise snaps it up immediately; a fall lowers
    /// it to the current fraction the same tick) - so this is a plain
    /// one-pole low-pass toward [`Self::shield_fraction`], not a filter that
    /// only runs one direction. Ported by
    /// `crate::race::Race::advance_energy_bar_delay`, the same
    /// advance-then-read shape [`Self::shield_blink_phase`] uses. `0.0` on
    /// [`Self::blank`] - the pool itself reads `0.0` before `<Misc>` loads
    /// (see [`Self::shield_fraction`]'s own zero-guard and
    /// `crate::race::start`'s `shield_flash_prev: 0.0` choice), so seeding
    /// this at the shield's own pre-load value avoids a false catch-up
    /// animation from an assumed "full" start that never happened.
    pub energy_bar_delay_fraction: f32,
    /// 2048's `ThrustBar` fill, in percent: the HUD's own chase toward the
    /// ship's thrust level, `Hud_Update` (`0x81195cdc`, dirty bit `0x80`).
    ///
    /// The original keeps `hud+0x1c8` and moves it toward the thrust level
    /// at 140 per second when below it and 100 per second when above, clamped
    /// onto the target, then crops `ThrustBar` to `value * 0.01` of its
    /// width. Ported by `Race::advance_thrust_chase`. **The target is chosen,
    /// not measured**: the original reads the ship's `+0x578`, a thrust
    /// level behind per-class rates this project has not recovered, and this
    /// port feeds the raw thrust input (`0` or `100`) instead.
    pub thrust_chase_percent: f32,
    /// Whether 2048's Pilot Assist setting is on, which lights `PilotAssist`.
    ///
    /// The original reads one global byte (`DAT_81545468 + 0x3260e`, the
    /// Extreme setting); a race sets it from the player's assist switch.
    pub pilot_assist: bool,
    /// The assist indicator HD authors (`AssistIndicator*`), see [`assist`].
    pub assist: assist::AssistReadout,
    /// The race's mode. Read by a title's runtime HUD rules, which the
    /// original keys on its own mode id - see `hud::runtime`.
    pub mode: oag_race::Mode,
    /// The Time Trial/Speed Lap "pacing" indicator -
    /// `Hud_UpdateTimeCluster`'s tier/target/redden triple
    /// (`*(hud+0x3c)+0x34`/`+0x30`/`+0x38`): a campaign cell's
    /// gold/silver/bronze ladder, or `RECORD` against the stored best and the
    /// track's authored time. `None` outside those two modes, and whenever the
    /// track's `stats.xml` did not load, which leaves the plain elapsed clock.
    /// See [`pace_for`] and
    /// `docs/ghidra/functions/psp-pulse-usa/hud-time-caption-substitution.md`.
    pub time_trial_pace: Option<TimeTrialPace>,
    /// Head2Head's gap to the opponent, `None` outside a two-craft Head2Head
    /// race. See [`HeadToHead`].
    pub head_to_head: Option<HeadToHead>,
    /// The Eliminator's kill column, top row first: one row per craft, already
    /// in the order the original draws them. Empty in every other mode. See
    /// [`kill_tags`].
    pub kill_tags: Vec<KillTag>,
    /// The Eliminator's kill target, resolved: the number in `KILLS (5)`.
    /// Zero in every other mode. See [`kill_tags::header`].
    pub kill_target: u32,
    /// The message slots this frame - `Info1` to `Info4`. See [`messages`].
    pub messages: messages::Lines,
}

/// What [`Readout::speed_full_kmh`] defaults to.
///
/// **A placeholder, and labelled one so it is never mistaken for a recovered
/// value.** The one bounded piece of evidence about the original's speed range is
/// the exhaust's own ramp, which saturates at **600 km/h**
/// (`oag_fx::exhaust::RAMP_FLOOR_KMH` + `RAMP_SPAN_KMH`, both recovered from
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

    /// Whether the shield bar draws forced solid red this frame rather than
    /// its own authored colour.
    ///
    /// `Hud_UpdateEnergyBar`'s threshold-plus-flash rule, confidence 82: red
    /// at or under [`oag_physics::damage::CRITICAL_PERCENT`] of the pool, or
    /// during [`Self::shield_flashing`]'s window, whichever fires first -
    /// **never a blend between the two colours by percentage**. Reusing
    /// `CRITICAL_PERCENT` rather than a second `20.0` here is deliberate:
    /// `Ship_Damage`'s energy-critical warning, the low-shield icon and this
    /// bar tint all read the same literal out of `.rodata`, and the point of
    /// naming it once is that the three cannot drift apart.
    ///
    /// **`iVar1` is identified and reproduced, 2026-09-25**: it is
    /// `HullOverlay_AbsorbWindowActive(player)`, [`Self::shield_absorbing`],
    /// and the original skips the forced-red branch *entirely* while it is
    /// set, even under the pool at 20% or a fresh hit. So absorbing always
    /// wins over a genuine drop for the colour, though not for the blink,
    /// see [`Self::shield_blinking`]. See shield.md's
    /// "`Hud_UpdateEnergyBar`: the absorb flash" section, confidence 82.
    #[must_use]
    pub fn shield_forced_red(&self) -> bool {
        !self.shield_absorbing
            && (self.shield_flashing
                || self.shield_fraction() * 100.0 <= oag_physics::damage::CRITICAL_PERCENT)
    }

    /// Whether the shield bar is inside `Hud_UpdateEnergyBar`'s shared blink
    /// accumulator this tick: the pool at or under
    /// [`oag_physics::damage::CRITICAL_PERCENT`], mid
    /// [`Self::shield_flashing`]'s post-hit window, or
    /// [`Self::shield_absorbing`] - the same three conditions ORed together
    /// that used to read as one before `iVar1` was identified (see
    /// [`Self::shield_forced_red`]). Absorbing enters this loop even though
    /// it is never forced red: the two reads are independent in the
    /// original. Confidence 80, shield.md's own reading of `hud+0x1dc`.
    #[must_use]
    pub fn shield_blinking(&self) -> bool {
        self.shield_absorbing
            || self.shield_flashing
            || self.shield_fraction() * 100.0 <= oag_physics::damage::CRITICAL_PERCENT
    }

    /// Whether [`Self::shield_blink_phase`]'s cycle is in its "on" (visible)
    /// phase this tick - `floor(t * 8.0)` even, the parity test
    /// `Hud_UpdateEnergyBar` runs on `hud+0x1dc` directly, ported rather
    /// than approximated: [`Self::shield_blink_phase`] already carries the
    /// original's own freeze-not-reset accumulator
    /// (`crate::race::Race::advance_shield_blink`), so this is the same
    /// arithmetic the decompile runs on it. The rate (8 transitions/second,
    /// a 4 Hz on/off cycle) and the 50% duty cycle are read directly off the
    /// `* 8.0` and `& 1` in the decompile.
    #[must_use]
    pub fn shield_blink_phase_on(&self) -> bool {
        let phase = (self.shield_blink_phase * 8.0).floor() as i64;
        phase.rem_euclid(2) == 0
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

/// Whether a rectangle lies wholly inside a screen of the given size.
///
/// Used by the layout tests rather than by the renderer: a widget that lands
/// outside is a parser bug, and the parser is the thing under test. `screen`
/// is the source's own grid - [`oag_display::space::Space::PSP`]`.size` for a PSP
/// layout, [`oag_display::space::Space::PS2`]`.size` for a PS2 one - because the
/// PS2 release authors the same five layouts in its own 640x448 grid rather
/// than the PSP's 480x272 ([`oag_display::space::SCREEN`]).
///
/// # The PS2 grid is not a flat scaling of the raw attributes, and that turns
/// out not to matter here
///
/// Checked coordinate by coordinate rather than by the extremes, the same way
/// `crates/game/tests/frontend_grid_ground_truth.rs` checked `Skin.xml`:
/// `Data\XML\Arcade_HUD.xml` carries 141 raw `x`/`y` attribute values (69 `x`,
/// 72 `y`) on both discs, keyed by tree path rather than by document order so
/// a widget is compared against its own counterpart and not merely the next
/// value in the file. **119 of the 141 are the PSP's own value scaled by
/// 640/480 or 448/272 and rounded to an integer; 22 are not**, and this
/// re-measurement corrects an earlier one that used a weaker key and found 20
/// - see below. The 22 split cleanly:
///
/// - **18 are the nine `<Mode3D><Model>` placements** (`x` and `y` each),
///   every one of them at the PSP's own `x="-240" y="136"`, byte-identical on
///   the PS2. `Mode3D` declares `mode="orthographic"`: a projection this
///   codebase does not drive, whose own parameters (near/far, ortho extent)
///   are unrecovered, so there is no known conversion from this pair into a
///   640x448 or 480x272 screen position at all. **Moot for this function
///   regardless**: [`Model`] is not [`Sprite`] or [`Label`], so nothing here
///   ever iterates one - a model was never in scope of an on-screen check
///   before this ratio was even a question, on either console.
/// - **4 are small integer nudges that stay byte-identical across consoles**:
///   `LapTxt`'s `y="-2"`, `RearWarningMissileIcon`'s `y="-4"`,
///   `RearWarningRocketIcon`'s `y="-2"`, and `TimeDiffIcon`'s `x`, which
///   *does* move (`-35` to `-45`) but only 1.67px short of the 640/480 ratio's
///   `-46.67`, just outside this measurement's one-pixel tolerance. All four
///   sit inside an `<Item>` whose own `OffsetX`/`OffsetY` **does** scale by
///   the ratio - `RearWarningIcons` goes `OffsetX="270" OffsetY="61"` to
///   `OffsetX="360" OffsetY="100"`, exactly 640/480 and closer than a pixel to
///   448/272 - and [`Layout::from_xml`] composes the offset into the widget's
///   `rect`/`x,y` before this function ever sees it. A pixel or two of
///   uncorrected nudge on a widget whose enclosing offset already moved it
///   most of the way is not enough to push any of the four off a screen nearly
///   a third larger; [`crate::tests`] and the ground-truth sweep below
///   confirm none does.
///
/// So, measured against the **composed** rect every [`Sprite`] and [`Label`]
/// actually carries - not the raw per-attribute values above - every
/// screen-positioned widget across all five layouts lands inside both the PSP's
/// 480x272 and the PS2's 640x448, and this function needed nothing beyond the
/// `screen` parameter to check either. See
/// `crates/game/tests/hud_layout_ground_truth.rs`'s PS2 sweep.
///
/// **What the previous version of this comment got wrong.** It attributed a
/// `y="-4"` "predicted -7" to `TimeDiffIcon`, which authors no such value in
/// any shipped layout - `TimeDiffIcon`'s own `y` is `0` on both discs, and the
/// `-4` belongs to `RearWarningMissileIcon` instead. That is a second, later
/// instance of exactly the failure mode `frontend_grid_ground_truth.rs`'s own
/// header warns about: a positional diff without a keyed path can pair a
/// widget with the wrong counterpart and blame the space for what is really a
/// bookkeeping error. Corrected 2026-09-05 by walking the tree with the same
/// [path, `#n`-suffixed]-keyed scheme `frontend_grid_ground_truth.rs` uses for
/// `Skin.xml`, rather than reading the two files' coordinates in parallel by
/// eye.
///
/// `Skin.xml`'s own `<Animation><Key>` carries `x`/`y` that are a *travel* and
/// are byte-identical on both consoles for the same reason the `<Mode3D>`
/// numbers are: a value authored against something other than the widget grid
/// has no reason to move when the grid does. **`x` does not mean one thing**
/// in this dialect, and anything reading a coordinate has to know which kind
/// it holds - but for the on-screen check specifically, every non-grid `x`/`y`
/// this file has found also happens to live somewhere [`inside_screen`]'s two
/// callers never look: inside a `<Model>` this module does not iterate, or
/// swamped by an `<Item>` offset that does scale.
#[must_use]
pub fn inside_screen(rect: [f32; 4], screen: (f32, f32)) -> bool {
    rect[0] >= 0.0
        && rect[1] >= 0.0
        && rect[0] + rect[2] <= screen.0
        && rect[1] + rect[3] <= screen.1
}

#[cfg(test)]
mod pickup_model_tests;
#[cfg(test)]
mod readout_tests;
#[cfg(test)]
mod reticle_tests;
#[cfg(test)]
mod shield_tests;
#[cfg(test)]
mod state_2048_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod zone_tests;
