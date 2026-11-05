//! The `Campaign Selection` screen ahead of `Grid Selection`: Wipeout HD/Fury's
//! two campaigns and Omega's three (2048, HD, Fury). See
//! `docs/ui/campaign-screens.md`'s "Wipeout HD/Fury: `Campaign Selection`" and
//! "Omega: `Campaign Selection`" sections for the full read of the disc's own
//! XML, the measurements this model is built from, and what is chosen rather
//! than measured.
//!
//! **Read, not declared.** Which campaigns it offers, in which order, where its
//! `<Redirect>` sends each and which grids each target screen pages are all
//! read off the screen's own XML ([`Chooser::read`]); this module names no
//! title. `oag_game::campaign::load` returns a chooser for a title whose
//! `CellMode_Definition.xml` authors the screen (HD's `DATA06` copy and Omega's
//! only), and `crate::campaign_stage::CampaignStage` opens on
//! [`crate::campaign_stage::Screen::Selection`] only when it has one.
//!
//! # The screen's own `<Redirect>` is not read generically
//!
//! `Campaign Selection`'s `<Redirect>` carries `<Entry item="campaignList"
//! equals="..." goto="...">` children - a conditional table keyed on the
//! `campaignList`'s own selection - which `oag_ui::screen::Redirect` does not
//! model at all: that type only ever reads a `<Default goto="...">` (this
//! screen's own `Default` is `"To Be Done"`, an unfinished fallback this
//! build never reaches). So [`Chooser::read`] reads the mapping off the raw
//! XML rather than through the generic screen parser.

use std::ops::Range;

use oag_gameplay::input::{Button, Input};

use oag_ui::frontend::{Align, Draw, Placed};
use oag_ui::language::StringTable;
use oag_ui::menu::{Frame, Layers, Picture, Skin};
use oag_ui::pointer::{Pointer, contains};
use oag_ui::screen::{Node, Screens, argb_to_rgba, parse, parse_argb};

use super::draw::{fill_draw, image_draw, text_draw};
use super::{Event, Layout};

#[cfg(test)]
mod tests;

/// The screen's name in `CellMode_Definition.xml`.
pub const SCREEN: &str = "Campaign Selection";

/// `ScreenTitle`'s own idstring on this screen - `FE_RC_SELECT`,
/// `"CAMPAIGN SELECT"` on an RPCS3 frame. Distinct from `Grid
/// Selection`/`Cell Selection`'s own `FE_RC` (`oag_ui_screens::campaign::hd::TITLE_ID`),
/// the same "each screen names its own title idstring" shape that pair
/// already has.
///
/// **`pub`, unlike that sibling constant** - a caller building this screen's
/// own `strings` needs it (and [`SUBTITLE_ID`]) to overlay the one archive
/// that actually resolves either: see
/// `oag_game::main::session::campaign::open_campaign`'s own doc on why the
/// general string table this crate is handed does not carry either id.
pub const TITLE_ID: &str = "FE_RC_SELECT";

/// The subtitle `MiniText`'s own idstring - `FE_CAMPSEL_MODES`,
/// `"CAMPAIGN MODES"` on an RPCS3 frame. See [`TITLE_ID`]'s own doc for why
/// this is `pub` too.
pub const SUBTITLE_ID: &str = "FE_CAMPSEL_MODES";

/// One of `campaignList`'s entries, by position in the list: the first is the
/// measured default (`Fury` on HD, `2048` on Omega). Which campaign it is
/// belongs to the [`Chooser`] that was read off the same screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Campaign(usize);

impl Campaign {
    /// The entry at `index` in `campaignList`.
    #[must_use]
    pub fn new(index: usize) -> Self {
        Self(index)
    }

    /// Its position in `campaignList`.
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

/// One `<Entry String="...">` of `campaignList`, with where the screen's
/// `<Redirect>` sends it: the campaign's own `Grid Selection` screen, its
/// layout, and the grids that screen's own `flyerlist` pages.
#[derive(Debug, Clone)]
pub struct Entry {
    /// `campaignList`'s own `String` - `FE_RC_HD`, `FE_RC_FURY`, `FE_RC`
    /// (2048): also the idstring of the campaign's name.
    pub id: String,
    /// The `goto` of the `<Redirect>` entry that `equals` [`Self::id`].
    pub screen: String,
    /// The grids that screen's `flyerlist` names (`grid0`..`grid7`), as
    /// indices into the title's grid list.
    pub grids: Range<usize>,
    /// [`Self::screen`]'s own layout.
    pub layout: Layout,
}

/// What a widget's name calls an entry by (`NumMedalsTextHD`,
/// `FuryCampaignFlyerModel`): the screen names its per-campaign widgets with
/// it, and nothing in the XML ties the two together but the spelling.
const TAGS: [(&str, &str); 3] = [
    ("FE_RC_HD", "HD"),
    ("FE_RC_FURY", "Fury"),
    ("FE_RC", "2048"),
];

/// The `campaignList` entries this build has met: the idstrings that name a
/// campaign on the screen.
pub fn known_entry_ids() -> impl Iterator<Item = &'static str> {
    TAGS.iter().map(|(id, _)| *id)
}

impl Entry {
    /// The word the screen's own widgets name this campaign by, or `None` for
    /// an entry this build has not met.
    #[must_use]
    pub fn tag(&self) -> Option<&'static str> {
        TAGS.iter()
            .find(|(id, _)| *id == self.id)
            .map(|(_, tag)| *tag)
    }

    /// The `<Flyer>` widget that stands for this campaign.
    #[must_use]
    pub fn card_widget(&self) -> Option<String> {
        self.tag().map(|tag| format!("{tag}CampaignFlyerModel"))
    }

    /// The `Data/FE/Flyers/<name>/` folder of its card: the lower-cased tag and
    /// `_campaign` (`fury_campaign`, `2048_campaign`).
    #[must_use]
    pub fn card_flyer(&self) -> Option<String> {
        self.tag()
            .map(|tag| format!("{}_campaign", tag.to_ascii_lowercase()))
    }
}

/// A `<MiniText>` of the screen: they are not collected generically (see
/// [`Chooser::read`]), so the ones this screen authors are kept here.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// The widget's `name`, when it has one (`HDGoldMedalsMiniText`).
    pub name: Option<String>,
    /// Its idstring.
    pub idstring: String,
    /// Where it starts.
    pub position: (f32, f32),
    /// ARGB, `FEGlobals` resolved.
    pub color: u32,
}

/// `Campaign Selection`, read off the disc's own XML: the entries of its
/// `campaignList` in order, where its `<Redirect>` sends each, the grids that
/// screen pages, and the screen's `<MiniText>` labels.
#[derive(Debug, Clone)]
pub struct Chooser {
    entries: Vec<Entry>,
    labels: Vec<Label>,
    bracket: [f32; 4],
}

impl Chooser {
    /// Reads the screen named [`SCREEN`] out of `xml`. `grid_layout` gives the
    /// layout of a campaign's own grid screen by name.
    ///
    /// **Read straight off the raw tree, not through [`Screens::collect`]**:
    /// `oag_ui::screen::Redirect` models only a `<Default goto>`, never the
    /// conditional `<Entry item="campaignList" equals="..." goto="...">` rows
    /// this screen's redirect is made of, and `<MiniText>` is on
    /// `MainMenu_Definition.xml`, `RaceBox_Definition.xml` and
    /// `Additional_Definition.xml` too, so collecting it generically would draw
    /// widgets on screens nobody has measured.
    ///
    /// `None` when the screen, an entry's redirect, its grid screen's
    /// `flyerlist` or its layout is missing: the caller then has no chooser
    /// and opens straight on a grid.
    #[must_use]
    pub fn read(
        xml: &str,
        screens: &Screens,
        grid_layout: &dyn Fn(&str) -> Option<Layout>,
    ) -> Option<Self> {
        let root = parse(xml);
        let screen = find_screen(&root, SCREEN)?;
        let list = descendants(screen).into_iter().find(|node| {
            node.name.eq_ignore_ascii_case("List") && node.attr("name") == Some("campaignList")
        })?;
        let redirect = screen.children_named("Redirect").next()?;
        let mut entries = Vec::new();
        for entry in list.children_named("Entry") {
            let id = entry.attr("String")?.to_string();
            let goto = redirect
                .children_named("Entry")
                .find(|row| {
                    row.attr("item") == Some("campaignList") && row.attr("equals") == Some(&id)
                })
                .and_then(|row| row.attr("goto"))?;
            let grids = flyer_grids(find_screen(&root, goto)?)?;
            entries.push(Entry {
                id,
                screen: goto.to_string(),
                grids,
                layout: grid_layout(goto)?,
            });
        }
        if entries.is_empty() {
            return None;
        }
        let number = |node: &Node, key: &str| screens.number(node.value(key));
        let labels = descendants(screen)
            .into_iter()
            .filter(|node| node.name.eq_ignore_ascii_case("MiniText"))
            .filter_map(|node| {
                Some(Label {
                    name: node.attr("name").map(str::to_string),
                    idstring: node.value("idstring")?.to_string(),
                    position: (number(node, "x")?, number(node, "y")?),
                    color: node
                        .value("color")
                        .and_then(|value| screens.resolve(value))
                        .and_then(parse_argb)
                        .unwrap_or(0xffff_ffff),
                })
            })
            .collect();
        let bracket = descendants(screen)
            .into_iter()
            .find(|node| node.name.eq_ignore_ascii_case("Bracket"))
            .and_then(|node| {
                Some([
                    number(node, "x")?,
                    number(node, "y")?,
                    number(node, "Width")?,
                    number(node, "Height")?,
                ])
            })?;
        Some(Self {
            entries,
            labels,
            bracket,
        })
    }

    /// The entries, in `campaignList` order.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The entry `campaign` names.
    #[must_use]
    pub fn entry(&self, campaign: Campaign) -> Option<&Entry> {
        self.entries.get(campaign.index())
    }

    /// The campaign whose grids include `grid`.
    #[must_use]
    pub fn campaign_of(&self, grid: usize) -> Option<Campaign> {
        self.entries
            .iter()
            .position(|entry| entry.grids.contains(&grid))
            .map(Campaign)
    }

    /// The `<Bracket>` both sides of the screen sit inside.
    #[must_use]
    pub fn bracket(&self) -> [f32; 4] {
        self.bracket
    }

    /// The idstrings the screen draws by its own widgets: title, subtitle, the
    /// medal and rank labels and each campaign's name.
    #[must_use]
    pub fn string_ids(&self) -> Vec<&str> {
        [TITLE_ID, SUBTITLE_ID]
            .into_iter()
            .chain(self.labels.iter().map(|label| label.idstring.as_str()))
            .chain(self.entries.iter().map(|entry| entry.id.as_str()))
            .collect()
    }

    /// The `i`th of `count` equal columns of the bracket.
    fn column(&self, index: usize) -> [f32; 4] {
        let [x, y, width, height] = self.bracket;
        let count = self.entries.len().max(1);
        #[allow(clippy::cast_precision_loss, reason = "a handful of entries")]
        let (index, count) = (index as f32, count as f32);
        let each = width / count;
        [x + each * index, y, each, height]
    }
}

fn find_screen<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    if node.name.eq_ignore_ascii_case("Screen") && node.attr("name") == Some(name) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_screen(child, name))
}

fn descendants(node: &Node) -> Vec<&Node> {
    let mut out = Vec::new();
    let mut stack: Vec<&Node> = node.children.iter().rev().collect();
    while let Some(next) = stack.pop() {
        out.push(next);
        stack.extend(next.children.iter().rev());
    }
    out
}

/// The grids a `FlyerSelection` screen's `flyerlist` names, `grid8`..`grid15`
/// as `8..16`; `None` when it names none or not a contiguous run.
fn flyer_grids(screen: &Node) -> Option<Range<usize>> {
    let list = descendants(screen).into_iter().find(|node| {
        node.name.eq_ignore_ascii_case("List") && node.attr("name") == Some("flyerlist")
    })?;
    let indices: Vec<usize> = list
        .children_named("Entry")
        .filter_map(|entry| entry.attr("String")?.strip_prefix("grid")?.parse().ok())
        .collect();
    let first = *indices.first()?;
    indices
        .iter()
        .enumerate()
        .all(|(offset, grid)| *grid == first + offset)
        .then(|| first..first + indices.len())
}

/// The selected column's outline thickness and colour - the bracket split
/// into one column per entry. The screen authors no per-entry
/// `Selector`/highlight widget at all, unlike `Grid Selection`/`Cell
/// Selection`'s own `<Image name="Selector">`; the real game's equivalent is
/// the 3D flyer's own scale/emphasis animation (an RPCS3 frame shows the
/// selected card larger and centred), which is drawn when the cards load. A
/// plain four-sided outline stands in for it where they do not. **Chosen, not
/// measured.**
const SELECTOR_BORDER: f32 = 6.0;
const SELECTOR_COLOR: u32 = 0xffff_ffff;

/// Where each entry's name is drawn when no card stands in the column:
/// the list's own position is off screen by design (`<!-- -500 for y so it's
/// off screen!! -->`), so the name sits at the column's centre, above the
/// medal labels. **Chosen, not measured.**
const ENTRY_NAME_Y: f32 = 610.0;

/// The selected column, as a plain four-sided outline - see
/// [`SELECTOR_BORDER`]'s own doc.
fn selector_outline(chooser: &Chooser, selected: Campaign) -> [Draw; 4] {
    let [rx, ry, rw, rh] = chooser.column(selected.index());
    let color = argb_to_rgba(SELECTOR_COLOR);
    let t = SELECTOR_BORDER;
    [
        Draw::Fill {
            rect: [rx, ry, rw, t],
            color,
        },
        Draw::Fill {
            rect: [rx, ry + rh - t, rw, t],
            color,
        },
        Draw::Fill {
            rect: [rx, ry, t, rh],
            color,
        },
        Draw::Fill {
            rect: [rx + rw - t, ry, t, rh],
            color,
        },
    ]
}

/// `Campaign Selection`'s own model: a flat list of the chooser's entries,
/// the same shape [`super::GridSelection`]'s own pager is, stepped by
/// `left`/`right` (measured on HD: `right` moves `Fury` -> `Wipeout HD`)
/// rather than up/down.
#[derive(Debug, Clone)]
pub struct CampaignSelection {
    index: usize,
    chooser: Chooser,
}

impl CampaignSelection {
    /// On the list's first entry, the measured default.
    #[must_use]
    pub fn new(chooser: Chooser) -> Self {
        Self { index: 0, chooser }
    }

    /// Starts on `campaign` already selected - what a caller returning to
    /// this screen from a grid uses, so a player who chose `Wipeout HD` and
    /// backs out lands on `Campaign Selection` still showing it selected, not
    /// reset to the screen's own default.
    #[must_use]
    pub fn at(chooser: Chooser, campaign: Campaign) -> Self {
        let index = campaign
            .index()
            .min(chooser.entries.len().saturating_sub(1));
        Self { index, chooser }
    }

    #[must_use]
    pub fn selected(&self) -> Campaign {
        Campaign(self.index)
    }

    #[must_use]
    pub fn chooser(&self) -> &Chooser {
        &self.chooser
    }

    /// `right` (`1`) is the measured step. **Clamped, not wrapping** - unlike
    /// [`super::GridSelection::step`]'s own sixteen-tier pager, this is
    /// measured directly rather than chosen for consistency with it: `left`
    /// (`-1`) at the list's own first entry is observed to leave the screen
    /// where it was, not wrap to the last. See `docs/ui/campaign-screens.md`,
    /// "The toggle".
    fn step(&mut self, step: i32) {
        let last = i32::try_from(self.chooser.entries.len().saturating_sub(1)).unwrap_or(0);
        let index = i32::try_from(self.index).unwrap_or(0) + step;
        self.index = usize::try_from(index.clamp(0, last)).unwrap_or(0);
    }

    /// The four directions and the two buttons this screen answers to -
    /// `up`/`down` unbound, since neither was independently isolated as a
    /// toggle or a no-op.
    pub fn update(&mut self, input: &mut Input) -> Vec<Event> {
        let mut out = Vec::new();
        if input.take(Button::Right) {
            self.step(1);
            out.push(Event::Moved);
        }
        if input.take(Button::Left) {
            self.step(-1);
            out.push(Event::Moved);
        }
        if input.take(Button::Cross) || input.take(Button::Start) {
            out.push(Event::Confirmed);
        }
        if input.take(Button::Circle) {
            out.push(Event::Back);
        }
        out
    }

    /// One click target per entry, the bracket split into equal columns: the
    /// screen authors no per-flyer clickable rect to ground a target in more
    /// precisely than that, the same "grounded in a real rect, not
    /// free-standing" reasoning [`super::hd::hd_grid_targets`] already uses
    /// for `Grid Selection`'s own confirm region. **Chosen, not measured.**
    /// Hover selects, a second click on the already-selected column confirms -
    /// the same two-tap idiom every other campaign screen in this crate uses.
    pub fn pointer(&mut self, pointer: &Pointer) -> Vec<Event> {
        let mut out = Vec::new();
        if pointer.is_idle() {
            return out;
        }
        let hit = pointer.at.and_then(|at| {
            (0..self.chooser.entries.len()).find(|&index| contains(self.chooser.column(index), at))
        });
        let was = self.index;
        if pointer.moved
            && let Some(index) = hit
            && index != was
        {
            self.index = index;
            out.push(Event::Moved);
        }
        if pointer.clicked
            && let Some(index) = hit
        {
            if index == self.index {
                out.push(Event::Confirmed);
            } else {
                self.index = index;
                out.push(Event::Moved);
            }
        }
        if pointer.back {
            out.push(Event::Back);
        }
        out
    }
}

/// Which entry a per-campaign widget belongs to, by the tag its name carries
/// (`NumMedalsTextHD`, `FuryGoldMedalsMiniText`) or, for 2048's rank block
/// (`RankText`, `TotalXPBg`, `TotalXPFill`, `Rank_Icon_Endrace`), the entry
/// tagged `2048`. `None` for a widget of the screen itself.
fn owner(chooser: &Chooser, name: &str) -> Option<usize> {
    let rank = name.starts_with("Rank") || name.starts_with("TotalXP");
    chooser.entries.iter().position(|entry| {
        let tag = entry.tag().unwrap_or("");
        if rank {
            tag == "2048"
        } else {
            !tag.is_empty() && name.contains(tag)
        }
    })
}

/// `Campaign Selection`'s draw list.
///
/// `cards` is whether the flyer cards are drawn behind it. **When they are,
/// the stand-ins for them are not**: the selected column's white outline and
/// the campaign names ([`ENTRY_NAME_Y`]) were this build's own invention for a
/// screen with nothing in the space the cards fill, and an RPCS3 frame draws
/// neither - the selected card faces the camera and the others are turned,
/// which is the indication, and each card carries its own name.
///
/// `gold` is one `(earned, total)` per entry, in `campaignList` order -
/// `earned` is `0` on a fresh profile, the same "player-progress source is
/// optional" reading the rest of this module gives; `total` is the
/// denominator: the campaign's own total cell count, which is also its total
/// possible gold medals (every cell carries exactly one `<Gold>` target). An
/// RPCS3 frame reads `"0 / 80"` beside `Fury`'s own flyer and `"0 / 87"`
/// beside `HD`'s: two different numbers, one per campaign, off
/// `oag_game::main::campaign_stage::CampaignStage::campaign_gold_medals`,
/// which sums `grid.cells.len()` over the campaign's own grids. The entry of
/// 2048 draws a rank where the others draw a medal count: `rank` is its
/// number (`0` on a fresh profile) and the experience bar under it is empty.
///
/// `footer_overlay` is [`super::hd::hd_cell_draw_list`]'s own parameter,
/// unchanged: an RPCS3 frame of this screen
/// shows the same `NAVIGATION`/`CONFIRM`/`BACK` footer row `Cell
/// Selection`'s own frame does.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same shape every other screen's own *_draw_list takes, plus the counts this \
              screen alone needs and the footer overlay hd_cell_draw_list already takes"
)]
pub fn draw_list(
    model: &CampaignSelection,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    strings: &StringTable,
    gold: &[(u32, u32)],
    rank: u32,
    backdrop: Option<Picture>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    footer_overlay: &[Draw],
    cards: bool,
) -> Layers {
    let chooser = model.chooser();
    let mut layers = Layers {
        backdrop: frame.backdrops(
            skin.space(),
            skin.background(),
            backdrop.map(Picture::draw),
            race_behind,
        ),
        ..Layers::default()
    };
    let (title_x, title_y, title_scale) = skin.title_at();
    layers.chrome.push(Draw::title(
        skin.title_font(),
        title_x,
        title_y,
        title_scale,
        skin.title_color(frame.ink),
        strings.get_or_id(TITLE_ID).to_string(),
    ));
    layers.chrome.extend_from_slice(footer_overlay);

    let screen = &layout.screen;
    let mut out = Vec::new();
    // With the cards drawn, only the selected campaign's counter shows: an
    // RPCS3 frame with Fury selected has none beside the turned `HD` card and
    // one with `HD` selected none beside Fury's.
    let shown = |name: &str| {
        owner(chooser, name).is_none_or(|entry| !cards || entry == model.selected().index())
    };
    for fill in &screen.fills {
        let name = fill.name.as_deref().unwrap_or("");
        if !shown(name) {
            continue;
        }
        // The experience bar's fill is as wide as the player's progress: none
        // on a fresh profile.
        if name == "TotalXPFill" {
            continue;
        }
        out.push(fill_draw(fill));
    }
    for image in &screen.images {
        let name = image.name.as_deref().unwrap_or("");
        // Both crop no sub-rect of `Hexmedal_HD.mip`'s own shared
        // multi-colour atlas, unlike `Target0/1/2 Medal`'s use of the same
        // atlas elsewhere on this title (`u`/`v`/`TxtrWidth`) - drawing
        // either as authored would show the whole atlas, not a clean glyph.
        // Not drawn rather than guessed at; see the module doc.
        if name.starts_with("MedalImage") || !shown(name) {
            continue;
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        out.push(image_draw(image, placed));
    }
    for text in &screen.texts {
        let name = text.name.as_deref().unwrap_or("");
        if !shown(name) {
            continue;
        }
        let counter = |tag: &str| {
            chooser
                .entries
                .iter()
                .position(|entry| entry.tag() == Some(tag))
                .and_then(|entry| gold.get(entry))
                .map(|(earned, total)| format!("{earned} / {total}"))
        };
        let content = match name {
            // `ScreenTitle` is drawn above through `skin.title_font()`,
            // the same idiom `oag_ui_screens::campaign::hd` already uses for its
            // own two screens' titles.
            "ScreenTitle" => None,
            // `RB_EVENT_TYPE` is a reused/generic idstring this build does
            // not trust as content - see [`draw_list`]'s own doc.
            "NumMedalsTextFury" => counter("Fury"),
            "NumMedalsTextHD" => counter("HD"),
            // Authored `99`, the shield's placeholder: the number is the player's.
            "RankText" => Some(rank.to_string()),
            _ => text.string.clone(),
        };
        let Some(content) = content else { continue };
        // A literal the face cannot spell is not drawn: the three `Caption*`
        // widgets author Chinese strings and the frame shows none of them.
        // **Chosen, not measured**: that the whole string is dropped rather than
        // its Latin characters (`2048`) is read off that frame alone.
        if !content.chars().all(|c| u32::from(c) < 0x250) {
            continue;
        }
        out.push(text_draw(text, &content, layout));
    }

    // The screen's `<MiniText>` labels: the subtitle, the medal and rank
    // captions, drawn as the pickers' headings are - a square bullet and small
    // text. Placed from the raw tree, see [`Chooser::read`].
    let headings: Vec<crate::picker::hd::MiniText> = chooser
        .labels
        .iter()
        .filter(|label| shown(label.name.as_deref().unwrap_or("")))
        .map(|label| crate::picker::hd::MiniText {
            x: label.position.0,
            y: label.position.1,
            text: strings.get_or_id(&label.idstring).to_string(),
            color: label.color,
        })
        .collect();
    crate::picker::hd::draw_labels(&headings, &mut out);
    if !cards {
        let grey = screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some("ScreenTitle"))
            .map_or(0xffff_ffff, |text| text.color);
        for (index, entry) in chooser.entries.iter().enumerate() {
            let [x, _, width, _] = chooser.column(index);
            out.push(Draw::Text {
                x: x + width / 2.0,
                y: ENTRY_NAME_Y,
                scale: 1.0,
                color: argb_to_rgba(grey),
                border: None,
                align: Align::Centre,
                text: strings.get_or_id(&entry.id).to_string(),
                wrap_width: None,
            });
        }
        // The selected column's outline - see [`SELECTOR_BORDER`]'s own doc on
        // why this is drawn rather than a `Selector` widget the screen does
        // not author.
        out.extend(selector_outline(chooser, model.selected()));
    }

    layers.body = out;
    layers
}
