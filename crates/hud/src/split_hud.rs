//! A split screen's HUD as data: where each group of a title's own widgets goes
//! inside one view.
//!
//! A title authors its HUD for the whole screen. Where it authors a split-screen
//! layout of its own ([`oag_title::hud::SplitLayouts`], HD's two views) that one
//! is used. Everywhere else - Pulse and Pure at any count, HD at three to eight,
//! every aspect the originals never drew - the layout comes from a TOML file of
//! this project's (`schemas/v1/split-hud.tosd`), read through the asset layers so
//! a player's override beats it and the disc's authored one sits between.
//!
//! # What a file says
//!
//! ```toml
//! [[layout]]
//! views = [4]                    # any of 1..=8; absent = every count
//! aspect = [1.2, 2.2]            # a view's width / height, [min, max); absent = any
//! modes = ["single_race"]        # `oag_race::Mode::name`; absent = every mode
//!
//! [[layout.group]]
//! widgets = ["SpeedBar*", "ShieldBar*"]   # the title's own widget names, a trailing `*` is a prefix
//! anchor = "bottom-right"        # a corner, an edge midpoint or the centre
//! offset = [-0.02, -0.03]        # fractions of the view's width and height
//! scale = 0.9                    # 1.0 = the whole-screen HUD scaled to fit the view
//! ```
//!
//! **Only the title's own widgets are placed, never new art.** A group is the
//! widgets its patterns match; the point of their authored bounding box that
//! `anchor` names is pinned to the same point of the view, moved by `offset` and
//! drawn at `scale`. A widget no group claims is not drawn, which is how a crowded
//! view drops its secondary readouts. The first rule matching the view's count,
//! aspect and mode answers; the first group to claim a widget keeps it.
//!
//! Every number is **chosen, not measured**: no capture of the original's split
//! screen has been read for a layout like this one, so a file carries no
//! confidence score.
//!
//! The grid is never named: an offset is a fraction of the view and a scale is
//! relative to the fit, so one file serves the PSP's 480x272 and the PS2's 640x448.

use serde::Deserialize;

use crate::draw::top_edge;
use crate::{Layout, Placement};

/// The schema kind a split-HUD file declares in its `[toml-schema]` header.
pub const SCHEMA_KIND: &str = "split-hud";
/// The one schema major this build reads.
pub const SCHEMA_MAJOR: u32 = 1;

/// Most views a race shows, `oag_render::viewport::MAX_VIEWPORTS`.
const MAX_VIEWS: u8 = 8;

/// Why a file was refused. Every variant names what it found.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Not TOML, or a key or shape the schema does not have.
    #[error("not a split-hud file: {0}")]
    Parse(#[from] toml::de::Error),
    /// The `[toml-schema]` header names another kind, another major, or is malformed.
    #[error("{0}")]
    Header(#[from] oag_schema::Error),
    /// A value outside what a layout can mean.
    #[error("layout {rule}: {what}")]
    Invalid {
        /// The rule's `name`, or its position when it has none.
        rule: String,
        /// What is wrong.
        what: String,
    },
}

/// A point of a rectangle: where a group is pinned.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Anchor {
    /// Top left corner.
    TopLeft,
    /// Middle of the top edge.
    Top,
    /// Top right corner.
    TopRight,
    /// Middle of the left edge.
    Left,
    /// Centre.
    Centre,
    /// Middle of the right edge.
    Right,
    /// Bottom left corner.
    BottomLeft,
    /// Middle of the bottom edge.
    Bottom,
    /// Bottom right corner.
    BottomRight,
}

impl Anchor {
    /// The point as fractions of a rectangle's width and height.
    #[must_use]
    pub const fn fraction(self) -> [f32; 2] {
        match self {
            Self::TopLeft => [0.0, 0.0],
            Self::Top => [0.5, 0.0],
            Self::TopRight => [1.0, 0.0],
            Self::Left => [0.0, 0.5],
            Self::Centre => [0.5, 0.5],
            Self::Right => [1.0, 0.5],
            Self::BottomLeft => [0.0, 1.0],
            Self::Bottom => [0.5, 1.0],
            Self::BottomRight => [1.0, 1.0],
        }
    }
}

/// Exactly two numbers: a three-element array deserialises into `[f32; 2]` without
/// complaint, which the schema rightly refuses.
fn pair<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<[f32; 2]>, D::Error> {
    match Option::<Vec<f32>>::deserialize(deserializer)? {
        None => Ok(None),
        Some(numbers) => match numbers.as_slice() {
            &[a, b] => Ok(Some([a, b])),
            other => Err(serde::de::Error::invalid_length(
                other.len(),
                &"two numbers",
            )),
        },
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    #[serde(rename = "toml-schema")]
    toml_schema: Option<oag_schema::Header>,
    #[serde(default)]
    layout: Vec<RuleFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleFile {
    name: Option<String>,
    #[serde(default)]
    views: Vec<u8>,
    #[serde(default, deserialize_with = "pair")]
    aspect: Option<[f32; 2]>,
    #[serde(default)]
    modes: Vec<String>,
    #[serde(default)]
    group: Vec<GroupFile>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GroupFile {
    widgets: Vec<String>,
    anchor: Anchor,
    #[serde(default, deserialize_with = "pair")]
    offset: Option<[f32; 2]>,
    scale: Option<f32>,
}

/// One group of widgets and where it goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    /// Widget names; one ending in `*` matches by prefix.
    pub widgets: Vec<String>,
    /// The point of the group's bounding box, and of the view, that meet.
    pub anchor: Anchor,
    /// Added to the view's anchor point, as fractions of the view's width and height.
    pub offset: [f32; 2],
    /// Relative to the whole-screen HUD scaled to fit the view.
    pub scale: f32,
}

impl Group {
    /// Whether `name` is one of the widgets this group names.
    #[must_use]
    pub fn matches(&self, name: &str) -> bool {
        self.widgets.iter().any(|pattern| {
            pattern
                .strip_suffix('*')
                .map_or(pattern == name, |prefix| name.starts_with(prefix))
        })
    }
}

/// One layout: the views it is for and the groups it places.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    /// The rule's own name, for a load report.
    pub name: String,
    /// The view counts it answers; empty is every count.
    pub views: Vec<u8>,
    /// A view's width over height, `[min, max)`; `None` is any shape.
    pub aspect: Option<[f32; 2]>,
    /// `oag_race::Mode::name`s it answers; empty is every mode.
    pub modes: Vec<String>,
    /// In paint order.
    pub groups: Vec<Group>,
}

impl Rule {
    /// Whether this rule answers a view of `aspect` in a race of `count` views.
    #[must_use]
    pub fn answers(&self, mode: oag_race::Mode, count: usize, aspect: f32) -> bool {
        self.covers(mode, count)
            && self
                .aspect
                .is_none_or(|[low, high]| aspect >= low && aspect < high)
    }

    /// Whether this rule is for a race of `count` views in `mode`, whatever the
    /// shape of a view.
    #[must_use]
    pub fn covers(&self, mode: oag_race::Mode, count: usize) -> bool {
        (self.views.is_empty() || self.views.iter().any(|&v| usize::from(v) == count))
            && (self.modes.is_empty() || self.modes.iter().any(|m| m == mode.name()))
    }
}

/// A parsed split-HUD file.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitHud {
    /// In file order; the first to answer a view wins.
    pub rules: Vec<Rule>,
}

impl SplitHud {
    /// Reads a file.
    ///
    /// A file with no `[toml-schema]` table loads with a warning; one naming
    /// another kind or major is refused by name.
    ///
    /// # Errors
    ///
    /// [`Error`]: the text is not a split-hud file, its header is another kind
    /// or major, or a value is out of range.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let file: File = toml::from_str(text)?;
        match oag_schema::check(file.toml_schema.as_ref(), SCHEMA_KIND, SCHEMA_MAJOR) {
            Ok(()) => {}
            Err(missing @ oag_schema::Error::Missing { .. }) => log::warn!("split HUD: {missing}"),
            Err(refused) => return Err(refused.into()),
        }
        let rules = file
            .layout
            .into_iter()
            .enumerate()
            .map(|(at, rule)| Rule::from_file(at, rule))
            .collect::<Result<_, _>>()?;
        Ok(Self { rules })
    }

    /// The first rule that answers a view of `aspect` in a race of `count` views.
    #[must_use]
    pub fn rule_for(&self, mode: oag_race::Mode, count: usize, aspect: f32) -> Option<usize> {
        self.rules
            .iter()
            .position(|rule| rule.answers(mode, count, aspect))
    }

    /// Every rule that could answer some view of a race of `count` views in
    /// `mode`, whatever the view's shape.
    pub fn rules_for(
        &self,
        mode: oag_race::Mode,
        count: usize,
    ) -> impl Iterator<Item = usize> + '_ {
        self.rules
            .iter()
            .enumerate()
            .filter(move |(_, rule)| rule.covers(mode, count))
            .map(|(at, _)| at)
    }
}

impl Rule {
    fn from_file(at: usize, file: RuleFile) -> Result<Self, Error> {
        let name = file.name.unwrap_or_else(|| format!("#{}", at + 1));
        let invalid = |what: String| Error::Invalid {
            rule: name.clone(),
            what,
        };
        if let Some(bad) = file.views.iter().find(|v| !(1..=MAX_VIEWS).contains(v)) {
            return Err(invalid(format!("`views` has {bad}, not 1 to {MAX_VIEWS}")));
        }
        if let Some([low, high]) = file.aspect
            && !(low.is_finite() && high.is_finite() && 0.0 <= low && low < high)
        {
            return Err(invalid(format!(
                "`aspect` [{low}, {high}] is not an increasing range from 0 or more"
            )));
        }
        let mut groups = Vec::new();
        for group in file.group {
            let offset = group.offset.unwrap_or([0.0, 0.0]);
            let scale = group.scale.unwrap_or(1.0);
            if group.widgets.is_empty() {
                return Err(invalid("a group names no widget".into()));
            }
            if group.widgets.iter().any(|w| w.is_empty() || w == "*") {
                return Err(invalid("a widget pattern is empty or `*` alone".into()));
            }
            if !(scale.is_finite() && scale > 0.0 && scale <= 8.0) {
                return Err(invalid(format!("`scale` {scale} is not in (0, 8]")));
            }
            if !offset.iter().all(|o| o.is_finite() && o.abs() <= 1.0) {
                return Err(invalid(format!(
                    "`offset` {offset:?} is outside one view's width or height"
                )));
            }
            groups.push(Group {
                widgets: group.widgets,
                anchor: group.anchor,
                offset,
                scale,
            });
        }
        Ok(Self {
            name,
            views: file.views,
            aspect: file.aspect,
            modes: file.modes,
            groups,
        })
    }
}

/// The line heights of the three HUD faces, as loaded: how tall a label's text
/// is, for a group's bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineHeights {
    /// The `HUD` face.
    pub hud: f32,
    /// The `HUDSmall` face.
    pub small: f32,
    /// The `Default` face.
    pub default: f32,
}

/// One group's widgets cut out of the title's layout, with where its anchor
/// point is in the layout's own grid.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// The widgets this group claimed, in the layout's document order.
    pub layout: Layout,
    /// The group's anchor point in the layout's grid, from its widgets' bounds.
    pub source: [f32; 2],
    /// How the group is pinned into a view.
    pub group: Group,
}

impl Piece {
    /// Where this piece goes in `view`, a view's rectangle in the layout's grid
    /// (`[x, y, width, height]`), for a HUD whose whole screen is `screen` big.
    #[must_use]
    pub fn placement(&self, view: [f32; 4], screen: (f32, f32)) -> Placement {
        let fit = (view[2] / screen.0).min(view[3] / screen.1);
        let scale = self.group.scale * fit;
        let [fx, fy] = self.group.anchor.fraction();
        let destination = [
            view[0] + view[2] * (fx + self.group.offset[0]),
            view[1] + view[3] * (fy + self.group.offset[1]),
        ];
        Placement {
            scale,
            offset: [
                destination[0] - scale * self.source[0],
                destination[1] - scale * self.source[1],
            ],
            clip: view,
        }
    }
}

/// Cuts `layout` into the pieces `rule` places, in group order.
///
/// A group that claims no widget of this layout (a pickup widget in a mode that
/// has none) yields no piece. A widget claimed by an earlier group is not claimed
/// again.
#[must_use]
pub fn arrange(rule: &Rule, layout: &Layout, heights: LineHeights) -> Vec<Piece> {
    let mut claimed = Claimed::new(layout);
    let mut pieces = Vec::new();
    for group in &rule.groups {
        let mut piece = Layout {
            constants: layout.constants.clone(),
            ..Layout::default()
        };
        let mut bounds = Bounds::default();
        for (at, sprite) in layout.sprites.iter().enumerate() {
            if claimed.take(Kind::Sprite, at, group, &sprite.name) {
                bounds.add_rect(turned(sprite.rect, sprite.rotation));
                piece.sprites.push(sprite.clone());
            }
        }
        for (at, fill) in layout.fills.iter().enumerate() {
            if claimed.take(Kind::Fill, at, group, &fill.name) {
                bounds.add_rect(fill.rect);
                piece.fills.push(fill.clone());
            }
        }
        for (at, label) in layout.labels.iter().enumerate() {
            if claimed.take(Kind::Label, at, group, &label.name) {
                let line = match label.font {
                    crate::Font::Hud => heights.hud,
                    crate::Font::Small => heights.small,
                    crate::Font::Default => heights.default,
                };
                let top = top_edge(label, line);
                bounds.add_rect([label.x, top, 0.0, line * label.scale]);
                piece.labels.push(label.clone());
            }
        }
        for (at, model) in layout.models.iter().enumerate() {
            if claimed.take(Kind::Model, at, group, &model.name) {
                bounds.add_rect([model.position[0], model.position[1], 0.0, 0.0]);
                piece.models.push(model.clone());
            }
        }
        if let Some(source) = bounds.point(group.anchor) {
            pieces.push(Piece {
                layout: piece,
                source,
                group: group.clone(),
            });
        }
    }
    pieces
}

/// Where a split-HUD file came from, which decides where it sits in the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A player's override: it beats the title's authored layout.
    Override,
    /// This project's own, behind the disc: the title's authored layout beats it.
    Fallback,
}

/// A race's split-HUD file, parsed and already cut into pieces for the layout
/// this race draws.
///
/// The pieces are cut once at load for every rule that could answer this race's
/// mode and view count, so a frame only picks one by the shape of a view.
#[derive(Debug, Clone)]
pub struct SplitHudAssets {
    /// The entry name it was read under, for the load report.
    pub entry: String,
    /// Which layer answered.
    pub origin: Origin,
    /// The parsed file.
    pub file: SplitHud,
    /// Per rule of [`Self::file`], its pieces; `None` for a rule that cannot
    /// answer this race.
    pub arranged: Vec<Option<Vec<Piece>>>,
}

impl SplitHudAssets {
    /// Cuts `file` for a race of `count` views in `mode`, drawn from `layout`.
    #[must_use]
    pub fn new(
        entry: &str,
        origin: Origin,
        file: SplitHud,
        layout: &Layout,
        heights: LineHeights,
        mode: oag_race::Mode,
        count: usize,
    ) -> Self {
        let arranged = file
            .rules
            .iter()
            .map(|rule| {
                rule.covers(mode, count)
                    .then(|| arrange(rule, layout, heights))
            })
            .collect();
        Self {
            entry: entry.to_owned(),
            origin,
            file,
            arranged,
        }
    }

    /// The rule answering a view of `aspect`, and its pieces.
    #[must_use]
    pub fn answer(
        &self,
        mode: oag_race::Mode,
        count: usize,
        aspect: f32,
    ) -> Option<(&Rule, &[Piece])> {
        let at = self.file.rule_for(mode, count, aspect)?;
        Some((&self.file.rules[at], self.arranged[at].as_deref()?))
    }
}

/// What lays out one view of a split screen.
#[derive(Debug, Clone, Copy)]
pub enum Source<'a> {
    /// The title's own authored layouts: the disc's data.
    Authored(&'a crate::SplitAssets),
    /// A split-HUD file's rule and its pieces.
    File(&'a Rule, &'a [Piece], Origin),
    /// The whole-screen layout scaled to fit the view, the last resort.
    Scaled,
}

impl Source<'_> {
    /// For the loader report.
    #[must_use]
    pub fn describe(&self) -> String {
        match self {
            Self::Authored(_) => "the title's authored split-screen layout".to_owned(),
            Self::File(rule, _, Origin::Override) => {
                format!("the player's override, layout {:?}", rule.name)
            }
            Self::File(rule, _, Origin::Fallback) => {
                format!("this project's fallback file, layout {:?}", rule.name)
            }
            Self::Scaled => "the whole-screen layout scaled into the view".to_owned(),
        }
    }
}

/// Which layout answers a view of `aspect` in a race of `count` views in `mode`:
/// a player's override file, then the disc's authored layout, then this project's
/// fallback file, then the whole-screen layout scaled to fit.
///
/// A file answers only where one of its rules does, so an override that covers
/// four views leaves two to the authored layout.
#[must_use]
pub fn source<'a>(
    authored: Option<&'a crate::SplitAssets>,
    file: Option<&'a SplitHudAssets>,
    mode: oag_race::Mode,
    count: usize,
    aspect: f32,
) -> Source<'a> {
    let from_file = file.and_then(|file| {
        file.answer(mode, count, aspect)
            .map(|(rule, pieces)| Source::File(rule, pieces, file.origin))
    });
    let authored = authored.filter(|_| count == 2).map(Source::Authored);
    match file.map(|file| file.origin) {
        Some(Origin::Override) => from_file.or(authored),
        _ => authored.or(from_file),
    }
    .unwrap_or(Source::Scaled)
}

#[derive(Clone, Copy)]
enum Kind {
    Sprite,
    Fill,
    Label,
    Model,
}

/// Which widgets an earlier group already took.
struct Claimed {
    taken: [Vec<bool>; 4],
}

impl Claimed {
    fn new(layout: &Layout) -> Self {
        Self {
            taken: [
                vec![false; layout.sprites.len()],
                vec![false; layout.fills.len()],
                vec![false; layout.labels.len()],
                vec![false; layout.models.len()],
            ],
        }
    }

    fn take(&mut self, kind: Kind, at: usize, group: &Group, name: &str) -> bool {
        let slot = &mut self.taken[kind as usize][at];
        if *slot || !group.matches(name) {
            return false;
        }
        *slot = true;
        true
    }
}

/// `rect` as it lies on screen: a quarter turn about its centre swaps its width and
/// height, which is what stands HD's Zone ladder up as a column. Other angles are the
/// small tilts of a row of lights and leave the box alone.
fn turned(rect: [f32; 4], rotation: f32) -> [f32; 4] {
    let quarters = (rotation / std::f32::consts::FRAC_PI_2).round();
    let off = (rotation - quarters * std::f32::consts::FRAC_PI_2).abs();
    if off < 0.05 && quarters as i32 % 2 != 0 {
        let (cx, cy) = (rect[0] + rect[2] * 0.5, rect[1] + rect[3] * 0.5);
        [cx - rect[3] * 0.5, cy - rect[2] * 0.5, rect[3], rect[2]]
    } else {
        rect
    }
}

/// The bounding box of what a group claimed.
#[derive(Default)]
struct Bounds(Option<[f32; 4]>);

impl Bounds {
    fn add_rect(&mut self, rect: [f32; 4]) {
        let (right, bottom) = (rect[0] + rect[2], rect[1] + rect[3]);
        self.0 = Some(match self.0 {
            None => [rect[0], rect[1], right, bottom],
            Some([l, t, r, b]) => [l.min(rect[0]), t.min(rect[1]), r.max(right), b.max(bottom)],
        });
    }

    fn point(&self, anchor: Anchor) -> Option<[f32; 2]> {
        let [l, t, r, b] = self.0?;
        let [fx, fy] = anchor.fraction();
        Some([l + (r - l) * fx, t + (b - t) * fy])
    }
}

#[cfg(test)]
mod tests;
