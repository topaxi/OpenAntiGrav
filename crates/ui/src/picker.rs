//! The race box's two selection screens: **Track Select** and **Ship Select**.
//!
//! Wipeout Pulse authors these as `Track Creation` and `Team Selection` in
//! `Data\Plugins\PI001\GUI\Selection_Definition.xml`, and both are the same
//! shape: a wrapping list moved with up/down, a counter under an up and a
//! down arrow, a preview on the left, and an info panel on the right whose
//! rows are the disc's own widgets - see
//! `docs/formats/race-setup.md` for the widget inventory and
//! `docs/ui/selection-screens.md` for the capture this was drawn against.
//!
//! **The tree is ours, the presentation is the disc's** - the same line
//! `docs/architecture/menus.md` draws for the menus. What is ours here is
//! *where the screens sit* (the RACE page's START row opens the first, the
//! first's confirm opens the second) and *what they list* (whatever the
//! source offers, packs included). Everything drawn comes off the screen the
//! disc authors: the arrows, the counter, the hex-grid panel, the rules, the
//! stat rows and the bars are all read out of the XML by [`Layout::read`]
//! with their authored positions, colours and texture sub-rects, and this
//! module only decides what each named widget *says*.
//!
//! The previews are 3D and are not drawn by this crate - it emits no mesh.
//! [`Layout::preview`] is where one goes, and the composition root draws it
//! there with `oag_game::preview`. Two things the original draws are left
//! out rather than invented: the circuit flythrough behind the hexagonal
//! window (the real track scene, not a front-end asset - see
//! `race-setup.md`), and the `Loyalty` bar, which reads a per-team counter
//! this build does not keep.

use crate::frontend::{Align, Draw, Placed};
use crate::language::StringTable;
use crate::menu::{Backdrop, Frame, Layers, Skin};
use crate::screen::{Fill, Image, Screen, Screens, Text, argb_to_rgba};
use oag_gameplay::input::{Button, Input};

pub mod slideshow;

/// The grid the selection screens are authored in on the PSP, which every
/// number this module carries of its own was read against. A PS2 layout is
/// the same screens scaled onto its 640x448 grid, so a constant measured
/// here is scaled the same way - see [`Layout::read`]'s `grid`.
const PSP_GRID: [f32; 2] = [480.0, 272.0];

/// Which of the two screens a [`Picker`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `Track Creation`: circuits.
    Track,
    /// `Team Selection`: teams, with a livery axis.
    Ship,
}

/// A team's four authored ratings, `0..=10`, as `Definition.xml` states them
/// on each `PI_Team`'s own `<FE speed=".." thrust=".." handling=".."
/// shield="..">` element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rating {
    pub speed: u8,
    pub thrust: u8,
    pub handling: u8,
    pub shield: u8,
}

impl Rating {
    /// The four in the order the screen draws them, each with the name of
    /// the widget pair (`<name> Bar`, `<name>`) that shows it.
    fn bars(self) -> [(&'static str, u8); 4] {
        [
            ("Speed", self.speed),
            ("Thrust", self.thrust),
            ("Handling", self.handling),
            ("Shield", self.shield),
        ]
    }

    /// How full a ten-segment bar is for `value`, clamped to the bar.
    #[must_use]
    pub fn fraction(value: u8) -> f32 {
        f32::from(value.min(10)) / 10.0
    }
}

/// What one list entry knows beyond its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Details {
    /// A circuit: the three info rows, in the order the screen's `Info1`,
    /// `Info2`, `Info3` widgets draw them - distance, lap record, race
    /// record. Already formatted; an unknown value is whatever the caller
    /// chose to say for one (the original's own is `-.--.--`).
    Track { info: [String; 3] },
    /// A team: its ratings, when the definition authors them, and its
    /// selectable liveries as `(id, label)` pairs - empty for a team that
    /// offers none, which draws no livery row rather than a fixed word.
    Ship {
        rating: Option<Rating>,
        variants: Vec<(String, String)>,
    },
}

/// One thing a player can pick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// What a setting stores - a `PI_Track` id or a team folder.
    pub id: String,
    /// The disc's own name for it, in the chosen language.
    pub label: String,
    pub details: Details,
}

/// What a tick of input did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The selection moved to another entry.
    Moved,
    /// The livery moved, on a [`Kind::Ship`] picker with more than one.
    VariantChanged,
    /// The player confirmed the selection.
    Confirmed,
    /// The player backed out.
    Back,
}

/// The screen's state: which entry, which livery, and how long it has been
/// open - the last for the turntable, which is the composition root's to
/// spin but this model's to time, so a capture at `--ticks N` is a fixed
/// pose.
#[derive(Debug, Clone)]
pub struct Picker {
    kind: Kind,
    entries: Vec<Entry>,
    index: usize,
    variant: usize,
    seconds: f32,
    /// Seconds since the selection last moved - the slideshow's clock,
    /// which the original restarts on every selection (`TrackSelection_ApplySelection`
    /// transitions the new circuit's own state machine to `Info` afresh).
    since_selection: f32,
}

impl Picker {
    /// A picker over `entries`, opened on `selected` when that id is one of
    /// them and on the first otherwise, with `variant` chosen the same way
    /// among the selected entry's liveries.
    #[must_use]
    pub fn new(
        kind: Kind,
        entries: Vec<Entry>,
        selected: Option<&str>,
        variant: Option<&str>,
    ) -> Self {
        let index = selected
            .and_then(|id| entries.iter().position(|entry| entry.id == id))
            .unwrap_or(0);
        let mut out = Self {
            kind,
            entries,
            index,
            variant: 0,
            seconds: 0.0,
            since_selection: 0.0,
        };
        out.variant = variant
            .and_then(|id| out.variants().iter().position(|(v, _)| v == id))
            .unwrap_or(0);
        out
    }

    #[must_use]
    pub fn kind(&self) -> Kind {
        self.kind
    }

    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    #[must_use]
    pub fn index(&self) -> usize {
        self.index
    }

    #[must_use]
    pub fn selected(&self) -> Option<&Entry> {
        self.entries.get(self.index)
    }

    /// The selected entry's liveries; empty for a circuit or a team with none.
    #[must_use]
    pub fn variants(&self) -> &[(String, String)] {
        match self.selected().map(|entry| &entry.details) {
            Some(Details::Ship { variants, .. }) => variants,
            _ => &[],
        }
    }

    /// The chosen livery, when the selected team offers any.
    #[must_use]
    pub fn variant(&self) -> Option<&(String, String)> {
        self.variants().get(self.variant)
    }

    /// Seconds since the screen opened, off the fixed tick.
    #[must_use]
    pub fn seconds(&self) -> f32 {
        self.seconds
    }

    /// Seconds since the selection last moved, or since the screen opened.
    /// See [`slideshow::Slideshow::at`].
    #[must_use]
    pub fn since_selection(&self) -> f32 {
        self.since_selection
    }

    /// Advances the screen's own clock by one fixed tick.
    pub fn tick(&mut self, dt: f32) {
        self.seconds += dt;
        self.since_selection += dt;
    }

    /// Rewrites one info row of a circuit entry - how a distance measured on
    /// a worker after the screen opened reaches the panel. A ship entry, or
    /// a row the panel has no widget for, is left alone.
    pub fn set_track_info(&mut self, index: usize, row: usize, value: String) {
        if let Some(Entry {
            details: Details::Track { info },
            ..
        }) = self.entries.get_mut(index)
            && let Some(slot) = info.get_mut(row)
        {
            *slot = value;
        }
    }

    /// Consumes this tick's presses and reports what they did.
    ///
    /// **Up/down wrap**, which is measured: the original's three-entry track
    /// list walked round with `down` alone lands back at `1/3` on the fourth
    /// press (`race-setup.md`). Left/right move the livery and are inert on
    /// a circuit, where the original opens `Track Help` instead - a screen
    /// this build does not have. Cross or Start confirms; Circle backs out.
    pub fn update(&mut self, input: &mut Input) -> Vec<Event> {
        let mut out = Vec::new();
        let count = self.entries.len();
        if input.take(Button::Down) && count > 0 {
            self.index = (self.index + 1) % count;
            self.variant = 0;
            self.since_selection = 0.0;
            out.push(Event::Moved);
        }
        if input.take(Button::Up) && count > 0 {
            self.index = (self.index + count - 1) % count;
            self.variant = 0;
            self.since_selection = 0.0;
            out.push(Event::Moved);
        }
        let variants = self.variants().len();
        if input.take(Button::Right) && variants > 1 {
            self.variant = (self.variant + 1) % variants;
            out.push(Event::VariantChanged);
        }
        if input.take(Button::Left) && variants > 1 {
            self.variant = (self.variant + variants - 1) % variants;
            out.push(Event::VariantChanged);
        }
        if input.take(Button::Cross) || input.take(Button::Start) {
            out.push(Event::Confirmed);
        }
        if input.take(Button::Circle) {
            out.push(Event::Back);
        }
        out
    }
}

/// Which face a widget's `font` role draws in, as a multiple of the menu
/// face's own size.
///
/// The menu renderer draws one face - the title's `menu` - and the disc's
/// selection screens author three roles on top of it: `Menu` for the name,
/// `default` for the stat rows and `small` for the footer. A role is drawn as
/// the menu face scaled by the ratio of the two faces' line heights, which
/// keeps every row at the height the disc states even though the glyphs are
/// the bigger face's. Measured at boot from the faces actually loaded, where
/// both are; see [`Layout::read`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceScales {
    /// `default` over `menu`.
    pub default: f32,
    /// `small` over `menu`.
    pub small: f32,
}

impl Default for FaceScales {
    /// Pulse's own three faces: `menu` is 22 px tall, `small` 17 and
    /// `default` 13 (`docs/ui/menus-original.md`, `docs/formats/fnt.md`).
    fn default() -> Self {
        Self {
            default: 13.0 / 22.0,
            small: 17.0 / 22.0,
        }
    }
}

/// The disc's own layout for one selection screen, with every string
/// already looked up.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The screen title, resolved - `RB_TRACK_SEL` or `RC_SHIPSEL`.
    pub title: String,
    /// The screen's widgets at their screen positions.
    pub screen: Screen,
    /// Where the 3D preview goes, in the screen's own grid.
    ///
    /// **Ours.** Neither screen authors a widget for its preview - the
    /// original places each in code (`race-setup.md`) - so this is read off
    /// the capture instead: the craft fills the left of `Team Selection`
    /// between the arrows and the panel, and the circuit outline sits on
    /// `Track Creation`'s panel between the name block and the first stat
    /// row.
    pub preview: [f32; 4],
    /// The info panel's own rect - the `Infogradient` widget - which the
    /// name block and the counter are laid out against.
    pub panel: [f32; 4],
    pub faces: FaceScales,
    /// How much larger this screen's grid is than the PSP's, per axis -
    /// `[1, 1]` on the PSP, `[4/3, 448/272]` on the PS2. Every number this
    /// module measured off the PSP capture is multiplied by it.
    pub scale: [f32; 2],
}

impl Layout {
    /// Reads `Track Creation` or `Team Selection` off the parsed front-end
    /// XML, resolving every `idstring` through `strings`.
    ///
    /// `None` when the screen is not in `screens` at all - a title that does
    /// not author one, or a Pulse boot whose `Selection_Definition.xml`
    /// failed to load - which leaves the caller with no picker rather than
    /// an empty one.
    ///
    /// `grid` is the screen space the XML is authored in - the PSP's 480x272
    /// or the PS2's 640x448. The widgets carry their own positions either
    /// way; what it scales is the handful of numbers this module measured
    /// off the PSP capture rather than read out of a widget.
    #[must_use]
    pub fn read(
        screens: &Screens,
        kind: Kind,
        strings: &StringTable,
        faces: FaceScales,
        grid: [f32; 2],
    ) -> Option<Self> {
        // Two names for the track screen because the titles disagree and
        // neither is a guess: Pulse's own is "Track Creation", Pure's is
        // "Track Selection" (`docs/formats/race-setup.md`) - tried in order,
        // the same shape `oag_game::race::shield_entry_names` uses for a
        // title-divergent file name. Every title so far names its ship
        // screen "Team Selection" alike, so that one needs no list.
        let names: &[&str] = match kind {
            Kind::Track => &["Track Creation", "Track Selection"],
            Kind::Ship => &["Team Selection"],
        };
        let name = names.iter().find(|n| screens.by_name(n).is_some())?;
        let scale = [grid[0] / PSP_GRID[0], grid[1] / PSP_GRID[1]];
        let mut screen = screens.by_name(name)?.clone();
        strip_player_suffix(&mut screen);
        for text in &mut screen.texts {
            if let Some(id) = text.idstring.as_deref()
                && let Some(resolved) = strings.get(id)
            {
                text.string = Some(resolved.to_string());
            }
        }
        // The screen's own title is its one `Title`-face text - unnamed,
        // placed at the skin's `TitleXOffset`/`TitleYOffset` like every
        // other screen's, and drawn as chrome rather than as a body widget.
        let title = screen
            .texts
            .iter()
            .find(|text| is_title(text))
            .and_then(|text| text.string.clone())
            .unwrap_or_else(|| name.to_string());
        // `Infogradient` is the panel's own translucent backing, and the one
        // fill on either screen that carries a name.
        let panel = screen
            .fills
            .iter()
            .find(|fill| fill.name.as_deref() == Some("Infogradient"))
            .map_or(
                [
                    290.0 * scale[0],
                    25.0 * scale[1],
                    170.0 * scale[0],
                    200.0 * scale[1],
                ],
                |fill| {
                    [
                        fill.x,
                        fill.y,
                        fill.width.unwrap_or(170.0 * scale[0]),
                        fill.height.unwrap_or(200.0 * scale[1]),
                    ]
                },
            );
        // The first rule under the name block is the top of the stat rows;
        // the outline sits between the two. Measured on the capture at
        // panel y+40 to the first `line bg`, and the craft on the left
        // between the arrow column and the panel.
        let first_rule = screen
            .fills
            .iter()
            .filter(|fill| {
                fill.gradient.is_some()
                    && fill
                        .height
                        .is_some_and(|height| (height - 14.0 * scale[1]).abs() < 1.5)
            })
            .map(|fill| fill.y)
            .fold(f32::INFINITY, f32::min);
        let preview = match kind {
            Kind::Track => {
                // Under the tallest name block the panel authors - three
                // lines of the menu face from `Info Track 3.1`'s own `y` -
                // and above the first stat rule.
                let name_bottom = screen
                    .texts
                    .iter()
                    .filter(|text| {
                        text.name
                            .as_deref()
                            .is_some_and(|n| n.starts_with("Info Track "))
                    })
                    .map(|text| text.y)
                    .fold(panel[1], f32::max)
                    + 24.0 * scale[1];
                let bottom = if first_rule.is_finite() {
                    first_rule
                } else {
                    panel[1] + panel[3] * 0.6
                };
                [
                    panel[0],
                    name_bottom,
                    panel[2],
                    (bottom - name_bottom).max(1.0),
                ]
            }
            Kind::Ship => [
                10.0 * scale[0],
                40.0 * scale[1],
                panel[0] - 20.0 * scale[0],
                190.0 * scale[1],
            ],
        };
        Some(Self {
            title,
            screen,
            preview,
            panel,
            faces,
            scale,
        })
    }

    /// The scale a widget's `font` role draws at in the menu face.
    fn face_scale(&self, font: &str) -> f32 {
        match font.to_ascii_lowercase().as_str() {
            "default" => self.faces.default,
            "small" => self.faces.small,
            _ => 1.0,
        }
    }
}

/// The screen, as layers: the frame's backdrops, the title, and everything
/// the picker itself draws.
///
/// The same shape [`crate::menu::draw_list`] returns for a menu page, so the
/// composition root composites both the same way. `sprites` answers where an
/// image's `src` sits in the sheet; an image the sheet does not hold is left
/// out rather than drawn as a box.
#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "the same eight facts of a frame `menu::draw_list` takes, plus the sheet"
)]
pub fn draw_list(
    picker: &Picker,
    layout: &Layout,
    skin: &Skin,
    frame: &Frame,
    backdrop: Option<Backdrop>,
    race_behind: bool,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    measure: &dyn Fn(&str) -> f32,
) -> Layers {
    let (title_x, title_y, title_scale) = skin.title_at();
    let video = backdrop.map(|backdrop| Draw::Video {
        rect: backdrop.rect,
        frame: backdrop.frame,
        position: backdrop.position,
        source: crate::frontend::Video::Backdrop,
    });
    let mut layers = Layers {
        backdrop: frame.backdrops(skin.space(), skin.background(), video, race_behind),
        ..Layers::default()
    };
    // See `crate::menu::draw_list`'s own title push - `Draw::title` is shared
    // with it for exactly this pair of call sites.
    layers.chrome.push(Draw::title(
        skin.title_font(),
        title_x,
        title_y,
        title_scale,
        skin.title_color(frame.ink),
        layout.title.clone(),
    ));
    layers.body = body(picker, layout, skin, sprites, measure);
    layers
}

/// The entry list, on a screen that lists rather than shows one at a time.
///
/// **The two titles disagree on the shape of a selection screen.** Pulse
/// shows the *selected* entry alone, in named `<Text>` widgets its
/// `Selection_Definition.xml` authors (`Info Track 1.1`, `Info1`, the stat
/// bars) - so it has no `<Menu>` and this draws nothing. Pure authors one
/// `<Menu name="Track">` / `<Menu name="Team">` per screen and no named text
/// at all, and lists every entry down the left the way its own `Language
/// Selection` lists languages.
///
/// The widget states its own x, y, scale, colour and alignment; **it states
/// no row pitch**, so the step is one line of the widget's own font at the
/// widget's own scale - exactly the rule `oag_ui::frontend::draw`'s
/// `DisplayLanguages` block already infers for this same widget on this same
/// title, and deliberately not [`Skin::row_pitch`], which is a menu *page*'s
/// step and carries that page's leading and `MenuScale`. The selected row takes the title's own
/// measured selected ink and no highlight band: a real capture of Pure's
/// `Language Selection` has no band at all, and inventing one here would
/// contradict a measurement already in the tree.
///
/// Coordinates are the widget's own and are **not** put through
/// [`Layout::scale`]: that factor exists for this build's own constants, and
/// a screen's widgets already arrive in the grid being drawn in - which is
/// why the PS2 pressing's panel reads `387` where the PSP's reads `290`.
/// Every other widget in [`body`] follows the same rule.
///
/// **This does not scroll.** The `<Menu>`'s `allocate` is a capacity, not a
/// count - `16` on both of Pure's screens - and a source that offers more
/// entries than fit runs off the bottom rather than paging. Pure's disc
/// offers 12 circuits and 10 teams; **its downloadable packs add more**, and
/// nothing here has counted a fully-packed roster against that 16.
fn entry_rows(picker: &Picker, layout: &Layout, skin: &Skin) -> Vec<Draw> {
    let Some(menu) = layout.screen.menu.as_ref() else {
        return Vec::new();
    };
    let scale = layout.face_scale(&menu.font) * menu.scale;
    let pitch = skin.line_height() * scale;
    let align = Align::parse(&menu.align);
    // The title's own measured pair, not the widget's `color` attribute -
    // see `oag_ui::frontend::draw`'s `DisplayLanguages` block for why: on
    // Pure that attribute resolves to `TextColor`, which is the *selected*
    // row's ink, and using it for both loses the selection cue.
    let normal = skin.normal();
    picker
        .entries()
        .iter()
        .enumerate()
        .map(|(index, entry)| Draw::Text {
            x: menu.x,
            y: menu.y + index as f32 * pitch,
            scale,
            color: if index == picker.index() {
                skin.selected()
            } else {
                normal
            },
            border: None,
            align,
            text: entry.label.clone(),
            wrap_width: None,
        })
        .collect()
}

/// Everything under the title: the panel, its rows, the arrows and the
/// counter, in the XML's own paint order - fills, then images, then texts.
fn body(
    picker: &Picker,
    layout: &Layout,
    skin: &Skin,
    sprites: &dyn Fn(&str) -> Option<Placed>,
    measure: &dyn Fn(&str) -> f32,
) -> Vec<Draw> {
    let mut out = Vec::new();
    let screen = &layout.screen;
    // The loyalty block - its title row and its bar row - is left out
    // whole: nothing here keeps the counter it would show. Both rows are
    // found by their named widget and everything sharing that row goes.
    let loyalty = screen
        .images
        .iter()
        .find(|image| image.name.as_deref() == Some("Loyalty Bar"))
        .map(|image| [image.x, image.y]);
    let loyalty_rows: Vec<f32> = loyalty
        .map(|[_, y]| y)
        .into_iter()
        .chain(
            screen
                .texts
                .iter()
                .filter(|text| text.name.as_deref() == Some("Loyalty"))
                .map(|text| text.y),
        )
        .collect();
    let on_loyalty_row = |y: f32| loyalty_rows.iter().any(|row| (row - y).abs() < 1.0);

    for fill in &screen.fills {
        out.push(fill_draw(fill));
    }

    let rating = match picker.selected().map(|entry| &entry.details) {
        Some(Details::Ship { rating, .. }) => *rating,
        _ => None,
    };
    let variants = picker.variants().len();
    for image in &screen.images {
        // The loyalty bar and its own dim backing share a rect; neither is
        // drawn, since nothing here keeps the counter it would show.
        if on_loyalty_row(image.y) {
            continue;
        }
        let Some(placed) = sprites(&image.src) else {
            continue;
        };
        let mut fraction = 1.0;
        let mut color = argb_to_rgba(image.color);
        match image.name.as_deref() {
            Some(name) if name.ends_with(" Bar") => {
                let Some(rating) = rating else { continue };
                let stat = name.trim_end_matches(" Bar");
                let bars = rating.bars();
                let Some((_, value)) = bars.iter().find(|(bar, _)| *bar == stat) else {
                    continue;
                };
                fraction = Rating::fraction(*value);
            }
            Some("skin left arrow" | "skin right arrow") => {
                if variants == 0 {
                    continue;
                }
                // Dim with nothing to cycle to, the way the capture shows a
                // fresh profile's single livery.
                if variants < 2 {
                    color[3] *= 0.35;
                }
            }
            _ => {}
        }
        // A bar backing with no rating to draw beside it is left out too:
        // an empty ten-segment backing reads as a rating of zero.
        if image.name.is_none()
            && rating.is_none()
            && screen.images.iter().any(|bar| {
                bar.name.as_deref().is_some_and(|n| n.ends_with(" Bar"))
                    && bar.y == image.y
                    && bar.x == image.x
            })
        {
            continue;
        }
        let width = image.width.unwrap_or(placed.width as f32);
        let height = image.height.unwrap_or(placed.height as f32);
        let sampled = [
            image.texture_width.unwrap_or(placed.width as f32),
            image.texture_height.unwrap_or(placed.height as f32),
        ];
        // A sub-rect larger than the texture is the texture repeated:
        // `Infohexgrid` samples 340x120 of a 32x16 tile. Anything else is
        // the ordinary one-patch sprite.
        if sampled[0] > placed.width as f32 + 0.5 || sampled[1] > placed.height as f32 + 0.5 {
            out.push(Draw::TiledSprite {
                rect: [image.x, image.y, width * fraction, height],
                uv: [
                    placed.x as f32,
                    placed.y as f32,
                    placed.width as f32,
                    placed.height as f32,
                ],
                repeat: [
                    sampled[0] * fraction / placed.width.max(1) as f32,
                    sampled[1] / placed.height.max(1) as f32,
                ],
                color,
            });
            continue;
        }
        out.push(Draw::Sprite {
            rect: [image.x, image.y, width * fraction, height],
            uv: [
                placed.x as f32 + image.u.unwrap_or(0.0),
                placed.y as f32 + image.v.unwrap_or(0.0),
                sampled[0] * fraction,
                sampled[1],
            ],
            color,
        });
    }

    out.extend(entry_rows(picker, layout, skin));

    let Some(entry) = picker.selected() else {
        return out;
    };
    let name_lines = match picker.kind() {
        Kind::Track => wrap_name(&entry.label, layout, measure),
        Kind::Ship => vec![entry.label.clone()],
    };
    for text in &screen.texts {
        if on_loyalty_row(text.y) {
            continue;
        }
        // The title is chrome - see [`Layout::read`] - not a body widget.
        if is_title(text) {
            continue;
        }
        let name = text.name.as_deref().unwrap_or("");
        let content: Option<String> = match name {
            "honey" => Some(format!(
                "{} / {}",
                picker.index() + 1,
                picker.entries().len()
            )),
            // `Info Track <lines>.<line>`: the one block whose line count
            // matches the wrapped name is drawn, and only its own lines.
            _ if name.starts_with("Info Track ") => {
                let mut parts = name["Info Track ".len()..].split('.');
                let lines: usize = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                let line: usize = parts.next().and_then(|n| n.parse().ok()).unwrap_or(0);
                match picker.kind() {
                    Kind::Track if lines == name_lines.len() && line >= 1 => {
                        name_lines.get(line - 1).cloned()
                    }
                    _ => None,
                }
            }
            "Info1" | "Info2" | "Info3" => match &entry.details {
                Details::Track { info } => {
                    let row = usize::from(name.as_bytes()[4] - b'1');
                    info.get(row).cloned()
                }
                Details::Ship { .. } => None,
            },
            "Speed" | "Thrust" | "Handling" | "Shield" => rating.and_then(|rating| {
                rating
                    .bars()
                    .iter()
                    .find(|(bar, _)| *bar == name)
                    .map(|(_, value)| value.to_string())
            }),
            "skin" => picker.variant().map(|(_, label)| label.clone()),
            // Only ever shown for a forced or suggested craft, and for a
            // team with nothing to cycle - neither of which this build has
            // a rule for yet.
            "skin fixed" | "Suggest" | "Loyalty" => None,
            _ => text.string.clone(),
        };
        let Some(content) = content else { continue };
        // A stat title beside a row this entry has no value for is left
        // out with it, or the panel reads as a row with a blank.
        if name.is_empty()
            && rating.is_none()
            && screen.texts.iter().any(|value| {
                value.y == text.y
                    && matches!(
                        value.name.as_deref(),
                        Some("Speed" | "Thrust" | "Handling" | "Shield")
                    )
            })
        {
            continue;
        }
        out.push(text_draw(text, &content, layout));
    }
    // The team's own name sits where the disc's `List` puts its rows - the
    // panel's inner left edge, three down - which no `Text` widget authors.
    if picker.kind() == Kind::Ship {
        let color = screen
            .texts
            .iter()
            .find(|text| text.name.as_deref() == Some("skin"))
            .map_or([1.0, 1.0, 1.0, 1.0], |text| argb_to_rgba(text.color));
        out.push(Draw::Text {
            x: layout.panel[0] + 14.0 * layout.scale[0],
            y: layout.panel[1] + 3.0 * layout.scale[1],
            scale: 1.0,
            color,
            border: None,
            align: Align::Left,
            text: entry.label.clone(),
            wrap_width: None,
        });
    }
    out
}

/// A circuit's name split into the lines the panel has blocks for.
///
/// Greedy by word against the panel's inner width, capped at the three
/// lines `Info Track 3.x` authors. The two names the capture shows agree:
/// `Talon's Junction White` broke as three lines and `Moa Therma White` as
/// two, which is what a greedy wrap at the panel's 142 usable units in the
/// `Menu` face produces.
fn wrap_name(label: &str, layout: &Layout, measure: &dyn Fn(&str) -> f32) -> Vec<String> {
    let inner = layout.panel[2] - 2.0 * 14.0 * layout.scale[0];
    let mut lines: Vec<String> = Vec::new();
    for word in label.split_whitespace() {
        let full = lines.len() == 3;
        match lines.last_mut() {
            Some(line) if full || measure(&format!("{line} {word}")) <= inner => {
                line.push(' ');
                line.push_str(word);
            }
            _ => lines.push(word.to_string()),
        }
    }
    if lines.is_empty() {
        lines.push(label.to_string());
    }
    lines
}

/// Drops the PS2's player index off every widget name.
///
/// The PS2 pressing authors `Team Selection` for split screen, so every
/// widget on it is `honey0`/`honey1`, `Speed Bar0`/`Speed Bar1` and so on -
/// one set per player - where the PSP's are bare (`docs/formats/race-setup.md`).
/// This build draws one player and has no split screen, so the `0` set is
/// read under the PSP's own names; the `1` set only exists on
/// `Team SelectionSplit`, a screen nothing opens yet. When split screen
/// arrives this becomes "read set N" rather than "strip the zero".
/// Keyed on the counter: a screen with a `honey0` and no `honey` is one
/// authored that way, and a screen with a bare `honey` (`Track Creation` on
/// both discs) is left alone, `Info Track 1.1` and `line bg10` included.
fn strip_player_suffix(screen: &mut Screen) {
    let suffixed = |name: &Option<String>, want: &str| {
        name.as_deref()
            .is_some_and(|n| n.len() == want.len() + 1 && n.starts_with(want) && n.ends_with('0'))
    };
    let has_counter = screen
        .texts
        .iter()
        .any(|text| suffixed(&text.name, "honey"));
    let has_bare = screen
        .texts
        .iter()
        .any(|text| text.name.as_deref() == Some("honey"));
    if !has_counter || has_bare {
        return;
    }
    let strip = |name: &mut Option<String>| {
        if let Some(n) = name
            && n.len() > 1
            && n.ends_with('0')
        {
            n.pop();
        }
    };
    screen
        .texts
        .iter_mut()
        .for_each(|text| strip(&mut text.name));
    screen
        .images
        .iter_mut()
        .for_each(|image| strip(&mut image.name));
    screen
        .fills
        .iter_mut()
        .for_each(|fill| strip(&mut fill.name));
}

/// Whether a widget is the screen's title: the one text in the `Title` face.
fn is_title(text: &Text) -> bool {
    text.font.eq_ignore_ascii_case("Title")
}

fn fill_draw(fill: &Fill) -> Draw {
    let rect = [
        fill.x,
        fill.y,
        fill.width.unwrap_or(0.0),
        fill.height.unwrap_or(0.0),
    ];
    match fill.gradient {
        Some([c1, c2, c3, c4]) => Draw::GradientFill {
            rect,
            left: mean(argb_to_rgba(c1), argb_to_rgba(c2)),
            right: mean(argb_to_rgba(c3), argb_to_rgba(c4)),
        },
        None => Draw::Fill {
            rect,
            color: argb_to_rgba(fill.color),
        },
    }
}

fn mean(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        (a[0] + b[0]) * 0.5,
        (a[1] + b[1]) * 0.5,
        (a[2] + b[2]) * 0.5,
        (a[3] + b[3]) * 0.5,
    ]
}

fn text_draw(text: &Text, content: &str, layout: &Layout) -> Draw {
    Draw::Text {
        x: text.x,
        y: text.y,
        scale: text.scale * layout.face_scale(&text.font),
        color: argb_to_rgba(text.color),
        border: None,
        align: match text.align.to_ascii_lowercase().as_str() {
            "right" => Align::Right,
            "centre" | "center" => Align::Centre,
            _ => Align::Left,
        },
        text: content.to_string(),
        wrap_width: text.wrap_width,
    }
}

/// An image widget's sheet sub-rect, for a caller drawing one outside this
/// module's own walk.
#[must_use]
pub fn image_uv(image: &Image, placed: Placed) -> [f32; 4] {
    [
        placed.x as f32 + image.u.unwrap_or(0.0),
        placed.y as f32 + image.v.unwrap_or(0.0),
        image.texture_width.unwrap_or(placed.width as f32),
        image.texture_height.unwrap_or(placed.height as f32),
    ]
}

#[cfg(test)]
mod tests;
