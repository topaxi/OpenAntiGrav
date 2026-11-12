//! Wipeout 2048's campaign map: `SP.xml`'s events, on the `newFEshell`
//! screen the disc's own `<TouchCampaign>` and `<FE3DCanvas>` live on, each
//! a tap away from its card (`event_card`), and from `Launch 2048` past that.
//!
//! # Where a tile sits
//!
//! The original's map is its `Canvas3D` triangle lattice with cells painted:
//! an event is the hexagon of six cells at a column and row its `M_X`/`M_Y`
//! scale to, raised into a prism, and its place on screen is the canvas
//! camera's (`docs/ghidra/functions/vita-2048-eu-v104/campaign-map-projection.md`,
//! "The law"). [`MapEvent::x`]/[`MapEvent::y`] carry that cell and
//! [`super::canvas_camera`] the camera, the way the shell's scroll moves it and
//! the two centring laws: the view the map opens on
//! ([`CampaignMap::open`]) and the view a selection scrolls to
//! ([`CampaignMap::follow`]). The lattice itself is drawn beneath the tiles
//! from the picture the title page bakes ([`super::canvas_lattice`]), shifted
//! by whole lattice periods to the view's phase.
//!
//! A tile is the prism the canvas's own programs draw ([`super::canvas_prism`]):
//! top face, walls and the shade at their foot, in the colours the canvas
//! paints a state with. An event not reached yet is not a prism but its six cells
//! flat in a tint, and every event with a role ([`MapRole`]) has the ring of
//! striped or dotted cells the canvas paints around it ([`super::canvas_cells`]);
//! both are drawn between the lattice and the prisms. What stands in for the
//! original here: the season cards and the ground relief are not drawn, and the
//! side challenges the original hides until the player's rank reaches
//! `m_RankRequired` ([`MapEvent::rank_gated`]) stay hidden, with no tile, ring or
//! cursor (this build has no rank). The view's
//! travel limits are **chosen, not measured**: any event can be centred, with the
//! original's 650 px of margin past the outermost ones.
//!
//! # The pad's cursor and the mouse
//!
//! The right stick moves an arrow over the map and pushing it past the view's
//! edge pans, so a pad reaches every event, including ones the d-pad's steps
//! skip ([`super::map_cursor`]). A held left mouse button drags the view
//! (`oag_game`'s pointer window turns it into a finger's drag while this map is
//! up) and the wheel pans a row a detent ([`CampaignMap::pan`]).
//!
//! # The card
//!
//! A tap on a tile opens the event's card (`event_card`,
//! `docs/ghidra/functions/vita-2048-eu-v104/campaign-event-card.md`); the map
//! launches nothing by itself. [`Frontend::launch_selected_event`] applies the
//! craft restriction the original's card applies
//! (`docs/formats/2048-campaign.md`, "Craft choice").
//!
//! # Progression: locked, open, passed, elite
//!
//! [`MapEvent::requires`] is authored data: the single event
//! `oag_2048::campaign::unlock_gates` names as this one's prerequisite. What
//! tier a finished attempt earned is a player's save, read by whatever calls
//! [`Frontend::refresh_campaign_progress`] (this crate carries no persistence,
//! so the medal lookup arrives as a closure). [`ProgressState`] folds the two
//! together. A tile's colour is one the canvas paints a state with
//! ([`canvas_prism::colours`]).

use crate::kinetic::{Bounds, Extent, Kinetic};
use crate::pointer::{Pointer, contains};

use oag_title::MapRole;

use super::canvas_camera::{self, Camera};
use super::canvas_cells::{self, CanvasButton, Tile};
use super::canvas_lattice;
use super::canvas_prism;
use super::touch::{LABEL_SCALE, Launch};
use super::*;
use oag_2048::frontend::states as w2048;
use oag_2048::race::TEAM_VARIANTS;

/// How far past the outermost events the view may travel, in viewport pixels:
/// the margin the original widens its scroll bounds by.
const TRAVEL_MARGIN: f32 = 650.0;
/// Where the hexagon sits in the 128 x 128 hex art, as fractions of the
/// texture: left, top, right, bottom of its opaque texels (2..116 across,
/// 12..115 down, measured on the decoded `hex_filled.gxt`). The selection ring
/// is drawn in the rect that puts this hexagon over a tile's top face.
const HEX_ART: [f32; 4] = [2.0 / 128.0, 12.0 / 128.0, 116.0 / 128.0, 115.0 / 128.0];
/// The mode glyph's width over the top face's: the original's flag spans 67
/// of the face's 120 px in the reference frame, and the glyph fills 60 of its
/// art's 64 texels.
const ICON_SHARE: f32 = 0.6;

/// The disc's own selection ring (`data/FE/Images/hex_select.gxt`), a thicker
/// glow ring. No front-end widget names it, so `oag_game::boot::sprites::load`
/// asks for it through its `extra` list. Drawn only on the selected marker, in
/// `Orange2048`, in place of [`Frontend::draw_cursor_ring`]'s rectangle ring
/// when it decoded.
pub const HEX_SELECT: &str = r"Data\FE\Images\hex_select.gtf";

/// Which of the disc's four campaign-event icons an event's kind draws,
/// `Data\FE\NewImages\callout\*_mode.gtf`: a chequered flag, a stopwatch, a
/// radar glyph and a crosshair. This crate's copy of
/// `oag_2048::campaign::EventKind` (plus the `laps == 0` Speed Lap split), so
/// `oag-ui` does not depend on the campaign tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventIcon {
    /// A lap race with one or more laps.
    Race,
    /// A lap race with `laps == Some(0)`, Speed Lap's own sentinel.
    SpeedLap,
    /// A Zone survival event.
    Zone,
    /// An Elimination event.
    Elimination,
}

impl EventIcon {
    /// The disc's own texture name, spelled the way every other front-end
    /// `Src=` is (`.gtf`, respelled `.gxt` by
    /// `oag_game::boot::sprites::read_front_end_first`).
    #[must_use]
    pub fn texture_name(self) -> &'static str {
        match self {
            Self::Race => r"Data\FE\NewImages\callout\race_mode.gtf",
            Self::SpeedLap => r"Data\FE\NewImages\callout\speed_mode.gtf",
            Self::Zone => r"Data\FE\NewImages\callout\zone_mode.gtf",
            Self::Elimination => r"Data\FE\NewImages\callout\combat_mode.gtf",
        }
    }
}

/// One campaign event, in the terms the map draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct MapEvent {
    /// The `SP.xml` instance name - what `Launch::Event` carries.
    pub name: String,
    /// The canvas cell the event's tile starts at, column then row: the first
    /// of the six lattice cells its hexagon covers
    /// (`oag_title::NodeMap::nudged` on the title's map nodes, or a
    /// downloadable campaign's `oag_title::NodeMap::node`).
    pub x: f32,
    pub y: f32,
    /// What to say about it: the circuit's display name, the kind, the
    /// class and the laps, already resolved to text by the caller.
    pub detail: String,
    /// The single other event whose completion opens this one -
    /// `oag_2048::campaign::unlock_gates`'s own name for it, `None` for an
    /// event open from the start. Authored data, read once at boot; see
    /// this module's own "Progression" section.
    pub requires: Option<String>,
    /// Which of the disc's own four mode icons this event draws - see
    /// [`EventIcon`].
    pub kind: EventIcon,
    /// `oag_2048::campaign::craft::forced_craft`'s own team id
    /// (e.g. `"Qirex2048\1"`) when this event authors
    /// `M_PPLAYERSHIPMODELDATA` - `None` for the 127 of 141 that leave the
    /// player's own craft alone. Forces the launch onto this craft
    /// unconditionally; [`Self::refused_craft`] is never consulted when this
    /// is `Some`, the same precedence `race::load_event` already applies.
    pub forced_craft: Option<String>,
    /// `oag_2048::campaign::craft::refused_craft`'s own set - every native
    /// team id this event's own `M_bPrevent*Ships` flags forbid. Empty for
    /// every unforced, unrestricted event (135 of 141) and always empty
    /// alongside [`Self::forced_craft`], which never coexists with a
    /// restriction on the real file.
    pub refused_craft: Vec<String>,
    /// What the event's own card says - see [`super::event_card`].
    pub card: super::event_card::EventCard,
    /// How the original's chain walk treats it, which decides the ring of cells
    /// painted around its tile.
    pub role: MapRole,
    /// The event authors the `m_options` bit the original tests as "locked"
    /// (`oag_2048::campaign::OPTION_LOCKED`): it has no tile, no ring and no cursor
    /// until a pass is earned on it. The original raises its progress by player
    /// rank (`m_RankRequired`, hypothesis); this build has no rank, so it stays
    /// hidden.
    pub rank_gated: bool,
}

/// The ring mode of an event's button, the fill its 18 ring cells take
/// (`canvas_cells::SHIFTS`): the chain's dark stripes (4) once the event is
/// reached, light stripes (3) before; a branch's dark dots (2) and light dots (1);
/// a speed lap's light stripes once reached. 0 is none. `0x810ef5fe`, `0x810ef536`
/// and the loop over the name table.
fn ring_mode(role: MapRole, reached: bool) -> u8 {
    match (role, reached) {
        (MapRole::Chain, true) => 4,
        (MapRole::Chain, false) => 3,
        (MapRole::Branch, true) => 2,
        (MapRole::Branch, false) => 1,
        (MapRole::SpeedLap, true) => 3,
        (MapRole::SpeedLap, false) | (MapRole::Side, _) => 0,
    }
}

/// A finished attempt's own two-tier result - this crate's copy of
/// `oag_2048::campaign::Tier`, kept separate so `oag-ui` never depends on
/// `oag_2048` or `oag_game::records`. See
/// [`Frontend::refresh_campaign_progress`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EarnedTier {
    /// Cleared the event's own pass bar.
    Pass,
    /// Cleared the harder elite bar.
    Elite,
}

/// Where an event sits once its own gate and a player's own save have both
/// been folded in - see this module's own "Progression" section.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProgressState {
    /// [`MapEvent::requires`] names an event that has not been passed yet.
    /// Refuses a launch.
    Locked,
    /// Open, not yet finished with at least a pass. The starting state for
    /// an event with no gate, before any progress has been read in at all.
    #[default]
    Open,
    /// Finished with at least [`EarnedTier::Pass`].
    Passed,
    /// Finished with [`EarnedTier::Elite`].
    Elite,
}

/// Where the map is: which event the cursor is on and how far the view has
/// scrolled.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CampaignMap {
    pub(super) events: Vec<MapEvent>,
    /// [`EarnedTier`] per event, indexed the same as `events` - `None`
    /// until [`Frontend::refresh_campaign_progress`] runs, and for every
    /// event a save has no result for yet. Absence here reads exactly like
    /// "never played", which is correct both before the first refresh and
    /// after it.
    pub(super) earned: Vec<Option<EarnedTier>>,
    pub(super) selected: usize,
    /// Which map is on screen: the `GameModeChoice` tile that opened it, `None`
    /// for the single-player map. See [`super::campaign_tiers`].
    pub(super) tile: Option<String>,
    /// Events a caller has closed, indexed the same as `events` - a
    /// downloadable campaign's grids stay shut until the one before them has
    /// earned its points (`oag_tables::race_campaign::grid_points_met`). Empty
    /// closes nothing.
    pub(super) closed: Vec<bool>,
    /// The maps not on screen. See [`super::campaign_tiers`].
    pub(super) parked: Vec<super::campaign_tiers::Parked>,
    /// The view's scroll over the canvas, x then y, in grid units.
    pub(super) scroll: [Kinetic; 2],
    /// The craft a fresh boot is about to race, before the player ever
    /// touches `Team` this session - `settings.race.team`/`variant`,
    /// combined the same way a campaign launch eventually loads with
    /// (`crate::main::session::menus::combine_variant`'s own return shape,
    /// e.g. `"Qirex2048\3"`). `Frontend::team_choice` stays `None` until the
    /// player actually moves on `Team` - `Session::frame` relies on that to
    /// tell "never touched" from "touched, first tile" - so this is the
    /// *only* place the untouched default reaches the front end at all. Set
    /// once, by [`Frontend::seed_craft`], next to
    /// [`Frontend::refresh_campaign_progress`] (`Session::finish_loading`'s
    /// own call site) since both need the save/settings a fresh boot has
    /// just read.
    pub(super) craft_seed: Option<String>,
    /// The per-event card, when one is open over the map.
    pub(super) card: Option<super::event_card::CardState>,
    /// The pad's pointer on the map. See [`super::map_cursor`].
    pub(super) cursor: super::map_cursor::MapCursor,
    /// The music wave round the current event. See [`super::campaign_wave`].
    pub(super) wave: super::canvas_wave::Wave,
    /// Seconds the wave has not stepped yet.
    pub(super) wave_owed: f32,
    /// Band 1 of the playing track's spectrum, when the music path has one.
    pub(super) band1: Option<f32>,
}

impl CampaignMap {
    /// How far the view sits right and down of the canvas's reset camera, in
    /// viewport pixels: the negative of the original's scroll position.
    pub(super) fn scroll(&self) -> (f32, f32) {
        (self.scroll[0].offset(), self.scroll[1].offset())
    }

    /// The canvas camera of the current view.
    fn camera(&self) -> Camera {
        Camera::scrolled(self.scroll())
    }

    pub(super) fn cell(event: &MapEvent) -> (f32, f32) {
        (event.x, event.y)
    }

    /// The box of an event's top face, in viewport pixels: what a pointer
    /// hits and what the hex art's hexagon is stretched over.
    fn face(camera: &Camera, event: &MapEvent) -> [f32; 4] {
        let centre = camera.project(canvas_camera::tile_top(Self::cell(event)));
        let (width, height) = canvas_camera::tile_face();
        [
            centre.0 - width * 0.5,
            centre.1 - height * 0.5,
            width,
            height,
        ]
    }

    /// The rect the hex art is drawn in for its hexagon to cover `face`.
    fn art(face: [f32; 4]) -> [f32; 4] {
        let width = face[2] / (HEX_ART[2] - HEX_ART[0]);
        let height = face[3] / (HEX_ART[3] - HEX_ART[1]);
        [
            face[0] - HEX_ART[0] * width,
            face[1] - HEX_ART[1] * height,
            width,
            height,
        ]
    }

    /// How far the view may travel along one axis: far enough to centre the
    /// outermost events, and [`TRAVEL_MARGIN`] past them.
    pub(super) fn limits(&self, axis: usize) -> (f32, f32) {
        let reset = Camera::scrolled((0.0, 0.0));
        let (low, high) = (0..self.events.len())
            .filter(|&at| !self.hidden(at))
            .map(|at| {
                let event = &self.events[at];
                let at = reset.project(canvas_camera::cell_ground(Self::cell(event)));
                match axis {
                    0 => at.0 - canvas_camera::CENTRE.0,
                    _ => at.1 - canvas_camera::CENTRE.1,
                }
            })
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), at| {
                (low.min(at), high.max(at))
            });
        if low > high {
            return (0.0, 0.0);
        }
        (low - TRAVEL_MARGIN, high + TRAVEL_MARGIN)
    }

    /// One axis of the view's travel: its limits, a tile's size as the pitch,
    /// and a rubber band half the view long past either end. Unsnapped: the
    /// map is a free canvas with no item to come to rest on.
    pub(super) fn extent(&self, axis: usize) -> Extent {
        let face = canvas_camera::tile_face();
        let (pitch, view) = match axis {
            0 => (face.0, canvas_camera::VIEWPORT.0),
            _ => (face.1, canvas_camera::VIEWPORT.1),
        };
        let (min, max) = self.limits(axis);
        Extent {
            pitch,
            max_fling: crate::kinetic::MAX_FLING,
            friction: crate::kinetic::FRICTION,
            snap: false,
            bounds: Bounds::Clamp {
                min,
                max,
                band: view * 0.5,
            },
        }
    }

    /// Scrolls the selected event to the middle of the view the way a tap on
    /// its tile does on the original: one application of the canvas's centring
    /// law from where the view is (`canvas_camera::centred_on`). The original
    /// eases there; this arrives at once.
    pub(super) fn follow(&mut self) {
        let Some(event) = self.events.get(self.selected) else {
            return;
        };
        let to = canvas_camera::centred_on(Self::cell(event), self.scroll());
        self.scroll[0].set(to.0);
        self.scroll[1].set(to.1);
    }

    /// Puts the view where the map opens for the selected event
    /// (`canvas_camera::opening_offset`).
    pub(super) fn open(&mut self) {
        self.scroll = Default::default();
        let Some(event) = self.events.get(self.selected) else {
            return;
        };
        let to = canvas_camera::opening_offset(Self::cell(event));
        self.scroll[0].set(to.0);
        self.scroll[1].set(to.1);
    }

    /// Pans the view by a finger (the canvas follows it, coasts after a
    /// flick and bounces off the edges, through [`crate::kinetic`]) or a
    /// wheel (one tile row per detent), and returns the pointer as the
    /// map should read it - without the tap that caught a coasting map.
    ///
    /// A drag moves the scroll by the finger's travel, as the original's
    /// `<TouchScroll>` does, so the picture follows a hair faster than the
    /// finger across (the camera's shear, `canvas_camera::span`). **Chosen,
    /// not measured**: the coast and bounce, and how a mouse wheel moves the
    /// view. The selection stays where it is - a pan reveals the map, it
    /// does not pick an event.
    fn pan(&mut self, pointer: &Pointer, view: (f32, f32)) -> Pointer {
        let scale = (
            canvas_camera::VIEWPORT.0 / view.0,
            canvas_camera::VIEWPORT.1 / view.1,
        );
        let extents = [self.extent(0), self.extent(1)];
        if pointer.scroll != 0 {
            let (min, max) = self.limits(1);
            let wheel = pointer.scroll as f32 * canvas_camera::tile_face().1;
            let to = (self.scroll[1].offset() + wheel).clamp(min, max);
            self.scroll[1].set(to);
            self.scroll[1].settle(&extents[1]);
        }
        let seen = self.scroll[0].gesture(pointer, -pointer.drag.0 * scale.0, &extents[0]);
        self.scroll[1].gesture(&seen, -pointer.drag.1 * scale.1, &extents[1])
    }

    /// The box of event `index`'s top face where the view is now.
    pub(super) fn face_of(&self, index: usize) -> Option<[f32; 4]> {
        Some(Self::face(&self.camera(), self.events.get(index)?))
    }

    /// The event whose top face holds `at` (the canvas viewport's coordinates),
    /// never one the map hides.
    pub(super) fn hit_at(&self, at: (f32, f32)) -> Option<usize> {
        let camera = self.camera();
        (0..self.events.len()).find(|&index| {
            !self.hidden(index) && contains(Self::face(&camera, &self.events[index]), at)
        })
    }

    /// Advances a coast or a bounce by one tick.
    pub(super) fn tick(&mut self, dt: f64) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a tick is milliseconds; f32 holds it exactly"
        )]
        let dt = dt as f32;
        let extents = [self.extent(0), self.extent(1)];
        for (scroll, extent) in self.scroll.iter_mut().zip(&extents) {
            scroll.tick(dt, extent);
        }
    }

    /// Whether the map shows nothing for the event at `index`: a rank-gated event
    /// ([`MapEvent::rank_gated`]) that has not been passed.
    pub(super) fn hidden(&self, index: usize) -> bool {
        self.events.get(index).is_some_and(|event| event.rank_gated)
            && !matches!(
                self.state_of(index),
                ProgressState::Passed | ProgressState::Elite
            )
    }

    /// [`ProgressState`] for the event at `index` - `Locked` when
    /// [`MapEvent::requires`] names an event that has not been passed yet
    /// (an unknown name, e.g. one filtered out of the map entirely, reads as
    /// not passed rather than as open by default - see this module's own
    /// "Never invent" rule), the tier `earned` carries otherwise.
    pub(super) fn state_of(&self, index: usize) -> ProgressState {
        let Some(event) = self.events.get(index) else {
            return ProgressState::Locked;
        };
        if self.closed.get(index).copied().unwrap_or(false) {
            return ProgressState::Locked;
        }
        let open = event.requires.as_deref().is_none_or(|gate| {
            self.events
                .iter()
                .position(|other| other.name == gate)
                .and_then(|at| self.earned.get(at).copied())
                .flatten()
                .is_some()
        });
        if !open {
            return ProgressState::Locked;
        }
        match self.earned.get(index).copied().flatten() {
            Some(EarnedTier::Elite) => ProgressState::Elite,
            Some(EarnedTier::Pass) => ProgressState::Passed,
            None => ProgressState::Open,
        }
    }
}

impl Frontend {
    /// Gives the map its events, in the order the file lists them. The
    /// cursor starts on the unlock graph's root: the first event with no
    /// [`MapEvent::requires`] that another event requires (2048's side
    /// challenges are open from the start too, but gate nothing). Failing
    /// that, the first ungated event, and failing that the first event. No
    /// name is read, so a downloadable or mod campaign opens on its own
    /// start.
    ///
    /// Every event reads [`ProgressState::Open`] or [`ProgressState::Locked`]
    /// (off its own [`MapEvent::requires`] alone, nothing earned yet) until
    /// [`Self::refresh_campaign_progress`] is called - a caller with a save
    /// to read should call it once, straight after this.
    pub fn set_campaign(&mut self, events: Vec<MapEvent>) {
        let ungated = |event: &MapEvent| event.requires.is_none() && !event.rank_gated;
        let gates_another = |event: &MapEvent| {
            events
                .iter()
                .any(|other| other.requires.as_deref() == Some(event.name.as_str()))
        };
        self.campaign.selected = events
            .iter()
            .position(|event| ungated(event) && gates_another(event))
            .or_else(|| events.iter().position(ungated))
            .unwrap_or(0);
        self.campaign.earned = vec![None; events.len()];
        self.campaign.closed = Vec::new();
        self.campaign.events = events;
        self.campaign.open();
    }

    /// Folds a player's own save into the map: `earned(name)` is asked once
    /// per event and answers the tier that event's own best-ever result
    /// earned, or `None` for one never finished with at least a pass.
    ///
    /// A plain closure rather than a `Store` reference, on purpose - see
    /// this module's own "Progression" section for why `oag-ui` cannot
    /// depend on `oag_game::records` at all.
    ///
    /// Every map gets it, the one on screen and the parked ones
    /// ([`super::campaign_tiers`]): a downloadable campaign's cells are asked
    /// for by their own names, which no single-player event shares.
    pub fn refresh_campaign_progress(&mut self, earned: impl Fn(&str) -> Option<EarnedTier>) {
        self.campaign.each_map(|events, tiers, _| {
            *tiers = events.iter().map(|event| earned(&event.name)).collect();
        });
    }

    /// Tells the map what craft a launch will fly before the player ever
    /// touches `Team` this session - see [`CampaignMap::craft_seed`]'s own
    /// doc for why this is the only path that reaches the front end at all,
    /// and why it is a separate call from [`Self::refresh_campaign_progress`]
    /// rather than folded into it (a caller with no save still boots with a
    /// real craft, the two are independent facts).
    pub fn seed_craft(&mut self, team_id: String) {
        self.campaign.craft_seed = Some(team_id);
    }

    /// The events on the map, in the order they were given.
    #[must_use]
    pub fn campaign_events(&self) -> &[MapEvent] {
        &self.campaign.events
    }

    /// The event the cursor is on.
    #[must_use]
    pub fn selected_event(&self) -> Option<&MapEvent> {
        self.campaign.events.get(self.campaign.selected)
    }

    /// Where `name` sits right now - `None` when it names no event on the
    /// map at all. A caller that only wants to check a save's own effect
    /// (a ground-truth test, most plausibly) can read this directly rather
    /// than driving the pad or the pointer to find out.
    #[must_use]
    pub fn campaign_event_state(&self, name: &str) -> Option<ProgressState> {
        let index = self
            .campaign
            .events
            .iter()
            .position(|event| event.name == name)?;
        Some(self.campaign.state_of(index))
    }

    /// Whether the campaign map is the screen and no event card is over it.
    pub(super) fn on_open_map(&self) -> bool {
        self.machine.is(w2048::NEW_FE_SHELL) && self.campaign.card.is_none()
    }

    /// The pad on the map: the d-pad moves to the nearest event that way,
    /// cross or start opens the card of the one under the cursor.
    pub(super) fn update_campaign_map(&mut self, input: &mut Input) {
        if self.campaign.events.is_empty() {
            return;
        }
        for (button, direction) in [
            (Button::Right, (1.0, 0.0)),
            (Button::Left, (-1.0, 0.0)),
            (Button::Down, (0.0, 1.0)),
            (Button::Up, (0.0, -1.0)),
        ] {
            if input.is_pressed(button) {
                input.consume_press(button);
                self.campaign.cursor.hide();
                if let Some(next) = self.nearest_event(direction) {
                    self.campaign.selected = next;
                    self.campaign.follow();
                }
                return;
            }
        }
        if input.is_pressed(Button::Cross) || input.is_pressed(Button::Start) {
            input.consume_press(Button::Cross);
            input.consume_press(Button::Start);
            // With the cursor up, Confirm opens what it points at, which the
            // view is not moved for (the cursor would leave the tile).
            if let Some(at) = self.campaign.cursor.at {
                if let Some(hit) = self.campaign.hit_at(at) {
                    self.campaign.selected = hit;
                    self.open_event_card();
                }
                return;
            }
            self.open_event_card();
        }
    }

    /// The event nearest the cursor in `direction`, by distance on the
    /// lattice, among those that lie at least one unit that way (neighbouring
    /// tiles are one and a half to two units apart).
    fn nearest_event(&self, direction: (f32, f32)) -> Option<usize> {
        let from = self.campaign.events.get(self.campaign.selected)?;
        self.campaign
            .events
            .iter()
            .enumerate()
            .filter(|(at, _)| *at != self.campaign.selected && !self.campaign.hidden(*at))
            .filter_map(|(at, event)| {
                let dx = (event.x - from.x) * canvas_lattice::CELL_DX;
                let dy = (event.y - from.y) * canvas_lattice::CELL_DY;
                let along = dx * direction.0 + dy * direction.1;
                if along < 1.0 {
                    return None;
                }
                // Distance, weighted against drifting sideways.
                let across = dx * direction.1 - dy * direction.0;
                Some((at, along + 2.0 * across.abs()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(at, _)| at)
    }

    /// The current craft when `event` forbids it, `None` when it may fly.
    /// Nothing to check reads as allowed - see [`Self::launch_selected_event`].
    pub(super) fn craft_refused_for(&self, event: &MapEvent) -> Option<String> {
        if event.forced_craft.is_some() || event.refused_craft.is_empty() {
            return None;
        }
        let current = self
            .team_choice()
            .map(|(team, suffix)| TEAM_VARIANTS.join.combine(team, suffix))
            .or_else(|| self.campaign.craft_seed.clone())?;
        event.refused_craft.contains(&current).then_some(current)
    }

    pub(super) fn launch_selected_event(&mut self) {
        let Some(event) = self.campaign.events.get(self.campaign.selected) else {
            return;
        };
        if matches!(
            self.campaign.state_of(self.campaign.selected),
            ProgressState::Locked
        ) {
            self.notes.push(format!(
                "{}: {:?} is locked, refusing to launch",
                w2048::NEW_FE_SHELL,
                event.name
            ));
            return;
        }
        // `event.forced_craft` always wins over a restriction on the real
        // file (`MapEvent::forced_craft`'s own doc), so a forced event never
        // consults `refused_craft` at all - it launches unconditionally,
        // same as before this check existed. A restricted event with
        // neither `team_choice()` (untouched `Team` this session) nor a
        // `craft_seed` (a caller that never called `Self::seed_craft`, e.g.
        // a unit test building its own fixture) has no craft to check at
        // all, so it launches rather than refusing on missing data - the
        // same "never guess" rule [`MapEvent::refused_craft`]'s own
        // construction already follows.
        if let Some(current) = self.craft_refused_for(event) {
            self.notes.push(format!(
                "{}: {:?} forbids {current:?}, refusing to launch - change craft at Team",
                w2048::NEW_FE_SHELL,
                event.name
            ));
            return;
        }
        self.notes.push(format!(
            "{}: {:?} tapped, firing {}",
            w2048::NEW_FE_SHELL,
            event.name,
            w2048::LAUNCH_2048
        ));
        self.touch.launch = Some(if self.campaign.tile.is_some() {
            Launch::Cell(event.name.clone())
        } else {
            Launch::Event(event.name.clone())
        });
        self.machine.fire(w2048::LAUNCH_2048);
    }

    /// The pointer on the map: hovering selects, a click on the selected
    /// event opens its card, a click elsewhere selects.
    pub(super) fn campaign_map_pointer(&mut self, pointer: &Pointer) -> bool {
        if self.campaign.card.is_some() {
            return self.event_card_pointer(pointer);
        }
        if pointer.is_idle() {
            return true;
        }
        let pointer = &self.campaign.pan(pointer, self.space.size);
        let Some(at) = pointer.at else {
            return true;
        };
        let (width, height) = self.space.size;
        let at = (
            at.0 * canvas_camera::VIEWPORT.0 / width,
            at.1 * canvas_camera::VIEWPORT.1 / height,
        );
        let hit = self.campaign.hit_at(at);
        // Against the selection *before* this tick's hover moved it: a tap
        // arrives with `moved` and `clicked` set together, and reading the
        // click against the hover it came with would launch on one tap.
        let was = self.campaign.selected;
        if pointer.moved
            && let Some(hit) = hit
        {
            self.campaign.selected = hit;
        }
        if pointer.clicked
            && let Some(hit) = hit
        {
            if hit == was {
                self.open_event_card();
            } else {
                self.campaign.selected = hit;
                self.campaign.follow();
            }
        }
        true
    }

    /// Where `src` sits in the sheet, or `None` when it never decoded (or
    /// never loaded - every `oag-ui` unit test in this crate boots with an
    /// empty sheet, which is what makes the hex art in [`Self::draw_campaign_map`]
    /// optional rather than assumed).
    pub(super) fn placed(&self, src: &str) -> Option<Placed> {
        self.placements
            .iter()
            .find(|(name, _)| name == src)
            .map(|(_, placed)| *placed)
    }

    /// A sprite of `placed`'s own texture, stretched to `rect` and tinted
    /// `color` - the shared shape [`Self::draw_campaign_map`]'s three hex
    /// layers and its mode icon all draw with.
    pub(super) fn sprite_at(rect: [f32; 4], placed: Placed, color: [f32; 4]) -> Draw {
        Draw::Sprite {
            rect,
            uv: placed.sheet_rect([0.0, 0.0, placed.width as f32, placed.height as f32]),
            color,
        }
    }

    /// The canvas's lattice at the map's view: the picture the title page
    /// bakes, drawn at the view's phase. The picture is one camera's frame of a
    /// pattern that repeats by the two lattice vectors, so it is placed by the
    /// displacement between its camera and this one, reduced to one period,
    /// and repeated by the whole lattice vectors nearest its own size until
    /// the screen is covered. Nothing is drawn when the sheet has no picture.
    fn draw_map_lattice(&self, out: &mut Vec<Draw>) {
        let Some(picture) = self.placed(canvas_lattice::TILE) else {
            return;
        };
        let (width, height) = self.space.size;
        let scale = (
            width / canvas_camera::VIEWPORT.0,
            height / canvas_camera::VIEWPORT.1,
        );
        for (x, y) in self.lattice_copies() {
            // One texel in from the picture's edge, so a filtered draw
            // never reads the sheet's neighbour along a seam.
            out.push(Draw::Sprite {
                rect: [
                    (x + 1.0) * scale.0,
                    (y + 1.0) * scale.1,
                    width - 2.0 * scale.0,
                    height - 2.0 * scale.1,
                ],
                uv: picture.sheet_rect([
                    1.0,
                    1.0,
                    picture.width as f32 - 2.0,
                    picture.height as f32 - 2.0,
                ]),
                color: [1.0; 4],
            });
        }
    }

    /// Where the lattice picture's four copies sit, their top left corners in
    /// viewport pixels, whole pixels: what [`Self::draw_map_lattice`] draws and
    /// what a raised cell takes its face from ([`super::campaign_wave`]).
    pub(super) fn lattice_copies(&self) -> [(f32, f32); 4] {
        let (across, down) = Camera::lattice_period();
        let here = self.campaign.camera().project([0.0; 3]);
        let baked = Camera::scrolled(canvas_lattice::BAKED_VIEW).project([0.0; 3]);
        let mut phase = (here.0 - baked.0, here.1 - baked.1);
        let rows = (phase.1 / down.1).floor();
        phase = (phase.0 - rows * down.0, phase.1 - rows * down.1);
        phase.0 -= (phase.0 / across.0).floor() * across.0;
        let origin = (phase.0 - across.0 - down.0, phase.1 - down.1);
        // The picture's own repeat: whole lattice vectors just inside its size.
        let columns = (canvas_camera::VIEWPORT.0 / across.0).floor();
        let lines = (canvas_camera::VIEWPORT.1 / down.1).floor();
        let slip = lines * down.0 - (lines * down.0 / across.0).round() * across.0;
        let mut copies = [(0.0, 0.0); 4];
        for (at, copy) in copies.iter_mut().enumerate() {
            let (line, column) = ((at / 2) as f32, (at % 2) as f32);
            *copy = (
                (origin.0 + column * columns * across.0 + line * slip).round(),
                (origin.1 + line * lines * down.1).round(),
            );
        }
        copies
    }

    /// A tile's prism: the baked picture ([`canvas_prism`]) with its anchor on
    /// the tile's projected top-face centre, tinted the state's colour. Not
    /// rounded to a whole pixel: the picture is soft shading and edges, and a
    /// rounded place would move a tile up to half a pixel off the camera's.
    fn prism_sprite(
        camera: &Camera,
        event: &MapEvent,
        prism: Placed,
        colour: [f32; 3],
        scale: (f32, f32),
    ) -> Draw {
        let layout = canvas_prism::layout();
        let at = camera.project(canvas_camera::tile_top(CampaignMap::cell(event)));
        let origin = (at.0 - layout.anchor.0, at.1 - layout.anchor.1);
        // One texel in from the picture's edge, which the bake leaves clear, so
        // a filtered draw never reads the sheet's neighbour along it.
        Draw::Sprite {
            rect: [
                (origin.0 + 1.0) * scale.0,
                (origin.1 + 1.0) * scale.1,
                (layout.size.0 as f32 - 2.0) * scale.0,
                (layout.size.1 as f32 - 2.0) * scale.1,
            ],
            uv: prism.sheet_rect([
                1.0,
                1.0,
                prism.width as f32 - 2.0,
                prism.height as f32 - 2.0,
            ]),
            color: canvas_prism::tint(colour),
        }
    }

    /// The flat tint of a tile that is not reached yet: the chain walk's colour for
    /// its walk (`0x810ef5fe` the chain's `0x817327c4`, `0x810ef536` the branch's
    /// `0x817327c8`).
    fn unreached_colour(event: &MapEvent) -> [f32; 3] {
        match event.role {
            MapRole::Branch => canvas_prism::colours::UNREACHED_BRANCH,
            _ => canvas_prism::colours::UNREACHED,
        }
    }

    /// Every visible event as the canvas button it paints.
    pub(super) fn canvas_buttons(&self) -> Vec<CanvasButton> {
        (0..self.campaign.events.len())
            .filter(|&at| !self.campaign.hidden(at))
            .map(|at| {
                let event = &self.campaign.events[at];
                let reached = self.campaign.state_of(at) != ProgressState::Locked;
                let (col, row) = CampaignMap::cell(event);
                CanvasButton {
                    cell: (col.round() as i32, row.round() as i32),
                    tile: if reached {
                        Tile::Raised
                    } else {
                        Tile::Flat(Self::unreached_colour(event))
                    },
                    ring: ring_mode(event.role, reached),
                }
            })
            .collect()
    }

    /// The canvas's ground cells that are neither lattice nor a raised tile: every
    /// ring cell and every unreached event's flat tint, by the canvas's priority
    /// rule ([`canvas_cells::resolve`]), drawn over the lattice and under the
    /// tiles. A sheet without the cell pictures draws none.
    fn draw_ground_cells(&self, camera: &Camera, out: &mut Vec<Draw>) {
        let (width, height) = self.space.size;
        let scale = (
            width / canvas_camera::VIEWPORT.0,
            height / canvas_camera::VIEWPORT.1,
        );
        let buttons = self.canvas_buttons();
        for painted in canvas_cells::resolve(&buttons) {
            let up = canvas_cells::is_up(painted.cell);
            let Some(picture) = self.placed(canvas_cells::name(painted.fill, up)) else {
                continue;
            };
            let layout = canvas_cells::layout(up);
            // Whole pixels, as the lattice picture beneath is placed.
            let origin = canvas_cells::origin(camera, painted.cell);
            let (x, y) = (origin.0.round(), origin.1.round());
            if x + layout.size.0 as f32 <= 0.0
                || y + layout.size.1 as f32 <= 0.0
                || x >= canvas_camera::VIEWPORT.0
                || y >= canvas_camera::VIEWPORT.1
            {
                continue;
            }
            // One texel in from the picture's edge, which the bake leaves clear.
            out.push(Draw::Sprite {
                rect: [
                    (x + 1.0) * scale.0,
                    (y + 1.0) * scale.1,
                    (layout.size.0 as f32 - 2.0) * scale.0,
                    (layout.size.1 as f32 - 2.0) * scale.1,
                ],
                uv: picture.sheet_rect([
                    1.0,
                    1.0,
                    picture.width as f32 - 2.0,
                    picture.height as f32 - 2.0,
                ]),
                color: if painted.fill == 0 {
                    canvas_prism::tint(painted.colour)
                } else {
                    [1.0; 4]
                },
            });
        }
    }

    /// The map: the lattice, then every tile in view with its mode glyph, and
    /// the cursor ring on the selected one.
    ///
    /// A tile is the canvas's prism ([`canvas_prism`]) on the top face
    /// [`CampaignMap::face`] names, in the colour of its [`ProgressState`]. A
    /// sheet without the baked prism draws a flat rectangle, and without the
    /// ring art a rectangle ring.
    pub(super) fn draw_campaign_map(&self, out: &mut Vec<Draw>) {
        let (width, height) = self.space.size;
        let scale = (
            width / canvas_camera::VIEWPORT.0,
            height / canvas_camera::VIEWPORT.1,
        );
        let on_screen = |rect: [f32; 4]| {
            [
                rect[0] * scale.0,
                rect[1] * scale.1,
                rect[2] * scale.0,
                rect[3] * scale.1,
            ]
        };
        let open = self.global_colour("Blue2048");
        let cursor = self.global_colour("Orange2048");
        let white = [1.0, 1.0, 1.0, 1.0];
        let hex_select = self.placed(HEX_SELECT);
        self.draw_map_lattice(out);
        let camera = self.campaign.camera();
        self.draw_ground_cells(&camera, out);
        self.draw_wave(&camera, out);
        // Far to near, as the canvas's depth buffer would: a nearer tile's wall
        // covers a farther tile's shade.
        let mut order: Vec<usize> = (0..self.campaign.events.len()).collect();
        order.sort_by(|&a, &b| {
            self.campaign.events[a]
                .y
                .total_cmp(&self.campaign.events[b].y)
        });
        for at in order.into_iter().filter(|&at| !self.campaign.hidden(at)) {
            let event = &self.campaign.events[at];
            let face = CampaignMap::face(&camera, event);
            let rect = on_screen(CampaignMap::art(face));
            if rect[0] + rect[2] < 0.0
                || rect[1] + rect[3] < 0.0
                || rect[0] > width
                || rect[1] > height
            {
                continue;
            }
            let state = self.campaign.state_of(at);
            let reached = state != ProgressState::Locked;
            if reached {
                let colour = match state {
                    ProgressState::Passed => canvas_prism::colours::PASSED,
                    ProgressState::Elite => canvas_prism::colours::ELITE,
                    ProgressState::Open | ProgressState::Locked => canvas_prism::colours::OPEN,
                };
                match self.placed(canvas_prism::TILE) {
                    Some(prism) => {
                        out.push(Self::prism_sprite(&camera, event, prism, colour, scale))
                    }
                    None => out.push(Draw::Fill {
                        rect: on_screen(face),
                        color: canvas_prism::tint(colour),
                    }),
                }
            } else if self.placed(canvas_cells::name(0, true)).is_none() {
                // No cell pictures in the sheet: the flat tint as a rectangle.
                out.push(Draw::Fill {
                    rect: on_screen(face),
                    color: canvas_prism::tint(Self::unreached_colour(event)),
                });
            }
            // The icon writer runs for a raised tile only (`0x8106fbd8`).
            if let Some(icon) = self.placed(event.kind.texture_name()).filter(|_| reached) {
                // The glyph lies on the tilted face, so it is as much shorter
                // than wide as the face is against a regular hexagon.
                let side = face[2] * ICON_SHARE;
                let tall = side * (face[3] / face[2]) / canvas_lattice::CELL_DY;
                out.push(Self::sprite_at(
                    on_screen([
                        face[0] + (face[2] - side) * 0.5,
                        face[1] + (face[3] - tall) * 0.5,
                        side,
                        tall,
                    ]),
                    icon,
                    white,
                ));
            }
            if at == self.campaign.selected {
                match hex_select {
                    Some(select) => out.push(Self::sprite_at(rect, select, cursor)),
                    None => self.draw_cursor_ring(on_screen(face), cursor, out),
                }
            }
        }
        if self.campaign.events.is_empty() {
            out.push(Draw::Text {
                x: width * 0.5,
                y: height * 0.5,
                scale: LABEL_SCALE,
                color: open,
                border: None,
                align: Align::Centre,
                text: "no campaign events were loaded".to_string(),
                wrap_width: None,
            });
        }
    }
}
