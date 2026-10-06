//! Wipeout 2048's touch-icon grids - `GameModeChoice` and `Home` - answering
//! a pad and a pointer, and drawn off their own `<TouchButton>` widgets.
//!
//! # What is authored, what is measured, what is chosen
//!
//! **Authored** (`NEWGUI/Definition.xml`, confidence 92): every tile's
//! position, size, icon texture, label id, `toggle` flag, `redirect` target
//! and `StringWidthLimit`; the header icon and title; `Blue2048`, `Grey2048`
//! and `Orange2048` as colours the skin declares.
//!
//! **Measured** off `data/reference/2048-frontend/08-game-mode-grid-clean.png`
//! (one Vita3K frame, confidence 80): an enabled tile is a solid square of
//! `Blue2048` (`0xff19295d` - the capture's tile pixels are exactly
//! `(25, 41, 93)`), a network-gated tile is `Grey2048` (`0xff717b96`, exactly
//! `(113, 123, 150)`), the icon is drawn at its texture's own size centred on
//! the tile (a 92px-wide glyph in a 128px texture lands 90px wide on screen),
//! and the label sits under the tile in the tile colour, its cap line 18px
//! below the tile's bottom edge, wrapped, 22px between lines. Off
//! `10-adhoc-game-list.png`: an unlabelled button (`cross.gtf`) is a box of
//! its authored 122x96 with the glyph centred, and a box that carries text
//! carries it inside, white, centred.
//!
//! **Chosen, not measured - no confidence score**: the pad cursor. The
//! capture shows no selected state at all (it was taken between taps), so
//! the tile the pad is on is outlined in `Orange2048`, a colour the skin
//! declares for something, drawn as a 4-unit ring. A player needs to see
//! where the cursor is; what the original draws there is unread.
//!
//! # Two taps on a toggle
//!
//! `GameModeChoice`'s four mode tiles are `toggle="true"` with no `redirect`;
//! a fifth, unlabelled button (`Icon_Tick.gtf`, `redirect="newFEshell"`) is
//! the confirm. So a tap on a mode marks it and the tick leaves - the file's
//! own shape, kept rather than folded into one tap; a second tap on the
//! chosen mode is read as the tick, the language picker's own rule. `Home`'s
//! five tiles each carry their own `redirect` and leave on the tap.
//!
//! Only `FE_SP_CAMPAIGN` can be confirmed here: the other three modes are
//! online, ad-hoc and cross-play, and the screens behind them are network
//! sessions this build does not have. Confirming one of those says so in a
//! note and stays.
//!
//! # Two tiles the disc does not author: RACE BOX and REMIX
//!
//! **This build's own, not on the disc**, and the one place this module
//! adds to what the file draws. 2048 ships no race box and no
//! `CellMode_Definition.xml` - its front end is the campaign map and the
//! network modes - and every other title in this project reaches this
//! build's own race box and RACE REMIX pages (`assets/ui/menu.toml`,
//! ADR-0034) from its own front end. So the two are appended **after** the
//! four authored tiles, never mixed into `Screen::touch_buttons`, labelled
//! with this project's own `OAG_MENU_RACEBOX`/`OAG_MENU_REMIX` strings (the
//! same ids the menu tree uses), and drawn as the labelled 122x96 box the
//! disc's own game-list screen authors for a text button - an authored
//! shape holding an unauthored destination. **Their positions are chosen,
//! not measured**: the bottom-left corner at `(16, 432)` and `(158, 432)`,
//! the confirm tick's own row, where nothing drawn sits. The parent shell
//! (`GameModeChoice` is nested inside `newFEshell`) authors a `<TouchNews>`
//! at `(16, 432)`, but that widget carries no size and this build draws
//! nothing for it, so the rect is empty in the picture. See
//! [`EXTRA_TILES`]. No disc icon exists for either, so neither draws one.

use crate::pointer::{Pointer, contains};
use crate::screen::{Screen, TouchButton, argb_to_rgba, parse_argb};

use super::*;
use oag_2048::frontend::states as w2048;

/// The pad cursor's ring, in grid units. Chosen - see the module docs.
const CURSOR_RING: f32 = 4.0;
/// From the tile's bottom edge to the label's cap line. Measured - see the
/// module docs. The 22 units between a wrapped label's lines is not a
/// constant here: it is the face's own 37-unit line height at
/// [`LABEL_SCALE`], which the renderer's wrapping already steps by.
const LABEL_GAP: f32 = 18.0;
/// The label's scale against the `Default` face. `NEOSANS_BOLD_LARGE.fnt`
/// is 37 units tall per line; the capture's label caps are 16 units and its
/// header caps (`scale="1.0"`, the same face) 27, so the label draws at
/// their ratio. Measured, one capture.
pub(super) const LABEL_SCALE: f32 = 16.0 / 27.0;
/// How far above the cap line the `Default` face's pen sits at scale 1:
/// the header's own `y="30"` puts its caps at 40 on the capture. Measured.
pub(super) const PEN_ABOVE_CAPS: f32 = 10.0;

/// What a confirmed tap asked the composition root for - see
/// [`Frontend::launch`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// A campaign event by its `SP.xml` instance name: the map's own
    /// `redirect="Launch 2048"`, resolved by `oag_raceplay::load_event`.
    Event(String),
    /// This build's own race box - the `race` page of `assets/ui/menu.toml`.
    RaceBox,
    /// This build's own RACE REMIX page (ADR-0034).
    Remix,
}

/// One of this build's own tiles on `GameModeChoice`, in the terms the
/// composition root fills it: a label already resolved through the
/// project's own string table, and where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtraTile {
    pub label: String,
    pub launch: Launch,
}

/// Where the two extra tiles sit, in the order [`Frontend::set_extra_tiles`]
/// was given them. **Chosen, not measured** - see the module docs. The
/// 122x96 is the tick's own authored size; the 20-unit gap between the two
/// is this build's.
pub const EXTRA_TILES: [[f32; 4]; 2] = [[16.0, 432.0, 122.0, 96.0], [158.0, 432.0, 122.0, 96.0]];

/// Where the touch front end is: which tile the pad is on, which mode has
/// been chosen, and what a confirmed tap asked the composition root for.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TouchState {
    /// Index into the current screen's [`Frontend::tiles`], in that order.
    pub selected: usize,
    /// The `idstring` of the toggle tile last tapped on `GameModeChoice`.
    pub chosen_mode: Option<String>,
    /// What a confirmed tap asked for, once one has. Read by the
    /// composition root through [`Frontend::launch`].
    pub launch: Option<Launch>,
    /// This build's own tiles, see [`ExtraTile`]. Empty until the
    /// composition root supplies them, which draws the grid as the file
    /// authors it and nothing more.
    pub extra: Vec<ExtraTile>,
    /// Screens left through [`Frontend::redirect_touch`], most recent last -
    /// what a `redirect="PreviousScreen"` tap pops. `Manual3D`'s own tick and
    /// `Team_Definition.xml`'s `select_button` both author that literal
    /// target rather than a screen name, which is Wipeout 2048's own "back"
    /// gesture and not one this build had a stack for until `Home`'s five
    /// destinations needed to leave the way they arrived. Never pushed by
    /// `newFEshell`'s own pad-driven Home/GameModeChoice hop, which fires
    /// directly and does not go through `redirect_touch` - so it never grows
    /// on ordinary campaign play, only while a player is under `Home`.
    pub history: Vec<String>,
    /// `Team_Definition.xml`'s own grid: `(team index, craft-slot index)`
    /// into [`oag_2048::race::NATIVE_TEAMS`]/`SHIP_TYPES`. `None` until the
    /// player moves off the screen's own starting point - see
    /// [`Frontend::team_choice`]'s own doc for why that is the contract.
    pub team_choice: Option<(usize, usize)>,
    /// `teamskin_touch`'s own highlighted entry. Drawn and cycled; not
    /// wired to any setting - see `frontend::team`'s module doc.
    pub skin_index: usize,
    /// `OptionsCamera`'s own choice - see [`Frontend::camera_choice`].
    pub camera_choice: Option<u8>,
    /// `OptionsAudio`'s `Music Volume` - see [`Frontend::music_choice`].
    pub music_choice: Option<u32>,
    /// `OptionsAudio`'s `SFX Volume` - see [`Frontend::sfx_choice`].
    pub sfx_choice: Option<u32>,
    /// Which of `OptionsAudio`'s two sliders Up/Down last focused: `0` music,
    /// `1` SFX.
    pub audio_slider: usize,
    /// `OptionsControls`' `Motion Sensor` list. Drawn and cycled; not wired
    /// to any setting - see `frontend::options2048`'s module doc.
    pub controls_index: usize,
    /// `OptionsPilot`'s `Pilot Assist` list. Drawn and cycled; not wired to
    /// any setting - see `frontend::options2048`'s module doc.
    pub pilot_index: usize,
}

/// What a tap on a tile does.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    /// Marks the tile chosen; the screen's tick leaves. `GameModeChoice`.
    Toggle(String),
    /// Leaves for a screen by name.
    Redirect(String),
    /// Leaves the front end with a request for the composition root.
    Launch(Launch),
}

/// One tile as drawn and hit-tested: an authored `<TouchButton>` or one of
/// this build's own, in one shape so the cursor cannot land on one list and
/// the tap fire on another.
#[derive(Debug, Clone, PartialEq)]
struct Tile {
    rect: [f32; 4],
    /// The icon's texture name, as the widget spells it.
    src: Option<String>,
    /// The label, already resolved to text.
    label: Option<String>,
    /// `StringWidthLimit`.
    wrap: Option<f32>,
    /// Whether the label is drawn inside the box (a text button) rather
    /// than under it (an icon tile).
    label_inside: bool,
    /// The icon's modulating colour.
    color: [f32; 4],
    action: Action,
    /// What to say in a note about this tile.
    name: String,
}

impl Frontend {
    /// What the front end has asked the composition root to open, if
    /// anything. `Some` only once [`Frontend::is_finished`] is, on a title
    /// whose front end fires `Launch 2048` rather than `Launch Game`.
    #[must_use]
    pub fn launch(&self) -> Option<&Launch> {
        self.touch.launch.as_ref()
    }

    /// The mode chosen on `GameModeChoice`, by its `idstring`.
    #[must_use]
    pub fn chosen_mode(&self) -> Option<&str> {
        self.touch.chosen_mode.as_deref()
    }

    /// Gives `GameModeChoice` this build's own tiles - see the module docs.
    /// At most [`EXTRA_TILES`]`.len()` are placed; the rest are dropped with
    /// a note rather than drawn on top of each other.
    pub fn set_extra_tiles(&mut self, tiles: Vec<ExtraTile>) {
        if tiles.len() > EXTRA_TILES.len() {
            self.notes.push(format!(
                "{} extra tiles offered, {} placed: only that many positions are chosen",
                tiles.len(),
                EXTRA_TILES.len()
            ));
        }
        self.touch.extra = tiles.into_iter().take(EXTRA_TILES.len()).collect();
    }

    /// Every tile on screen `name`: the authored buttons, then this build's
    /// own on `GameModeChoice`.
    fn tiles(&self, name: &str) -> Vec<Tile> {
        let Some(screen) = self.screens.by_name(name) else {
            return Vec::new();
        };
        let mut tiles: Vec<Tile> = screen
            .touch_buttons
            .iter()
            .map(|button| self.authored_tile(button))
            .collect();
        if name == w2048::GAME_MODE_CHOICE {
            for (extra, rect) in self.touch.extra.iter().zip(EXTRA_TILES) {
                tiles.push(Tile {
                    rect,
                    src: None,
                    label: Some(extra.label.clone()),
                    wrap: Some(rect[2] - 8.0),
                    label_inside: true,
                    color: [1.0; 4],
                    action: Action::Launch(extra.launch.clone()),
                    name: extra.label.clone(),
                });
            }
        }
        tiles
    }

    fn authored_tile(&self, button: &TouchButton) -> Tile {
        let label = button
            .idstring
            .as_deref()
            .map(|id| self.strings.get_or_id(id).to_string())
            .or_else(|| button.string.clone());
        let name = button
            .idstring
            .clone()
            .or_else(|| button.name.clone())
            .unwrap_or_else(|| "confirm".to_string());
        let action = match (&button.idstring, &button.redirect) {
            (Some(id), _) if button.toggle => Action::Toggle(id.clone()),
            (_, Some(target)) => Action::Redirect(target.clone()),
            _ => Action::Toggle(name.clone()),
        };
        Tile {
            rect: tile_rect(button),
            src: button.src.clone(),
            label,
            wrap: button.string_width_limit,
            label_inside: false,
            color: argb_to_rgba(button.color),
            action,
            name,
        }
    }

    /// The pad on a touch grid.
    pub(super) fn update_touch(&mut self, input: &mut Input) {
        let Some(current) = self.machine.current().map(str::to_string) else {
            return;
        };
        // The shell's own `<TouchHomeButton>` (`redirect="Home"`,
        // `moderedirect="GameModeChoice"`) is a 44-unit icon at `(16, 16)`
        // with no authored size, so it has no tile to point at yet; its
        // two targets are on the pad instead. **Which buttons is chosen,
        // not measured**: the widget names no button at all.
        if current == w2048::NEW_FE_SHELL {
            if self.event_card_open() {
                self.update_event_card(input);
                return;
            }
            for (button, target) in [
                (Button::Triangle, w2048::HOME),
                (Button::Circle, w2048::GAME_MODE_CHOICE),
            ] {
                if input.is_pressed(button) {
                    input.consume_press(button);
                    self.notes
                        .push(format!("{current}: {button:?} pressed, firing {target}"));
                    self.touch.selected = 0;
                    self.on_screen_for = 0.0;
                    self.machine.fire(target);
                    return;
                }
            }
            self.update_campaign_map(input);
            return;
        }
        let count = self.tiles(&current).len();
        if count == 0 {
            return;
        }
        self.touch.selected = self.touch.selected.min(count - 1);
        if input.is_pressed(Button::Right) {
            input.consume_press(Button::Right);
            self.touch.selected = (self.touch.selected + 1) % count;
        } else if input.is_pressed(Button::Left) {
            input.consume_press(Button::Left);
            self.touch.selected = (self.touch.selected + count - 1) % count;
        } else if input.is_pressed(Button::Cross) || input.is_pressed(Button::Start) {
            input.consume_press(Button::Cross);
            input.consume_press(Button::Start);
            self.activate_touch(&current, self.touch.selected);
        }
    }

    /// A tap or a press on tile `at` of screen `current`.
    fn activate_touch(&mut self, current: &str, at: usize) {
        let tiles = self.tiles(current);
        let Some(tile) = tiles.get(at) else {
            return;
        };
        let label = tile.name.clone();
        match tile.action.clone() {
            Action::Toggle(id) => {
                // Two taps: the first chooses, the second confirms through
                // the screen's own tick - see the module docs. **Only on
                // `GameModeChoice`**: every other screen this build reaches
                // generically may carry its own label-less `Action::Redirect`
                // tiles for an unrelated purpose - `Team_Definition.xml`'s
                // `replay_unlock`/`replay_unlock_2` are exactly that, and
                // without this guard a second tap on any of its toggle
                // buttons would silently auto-fire one of them instead of
                // simply re-choosing.
                if current == w2048::GAME_MODE_CHOICE
                    && self.touch.chosen_mode.as_deref() == Some(id.as_str())
                    && let Some(tick) = tiles
                        .iter()
                        .position(|t| t.label.is_none() && matches!(t.action, Action::Redirect(_)))
                {
                    self.notes
                        .push(format!("{current}: {label} chosen again, confirming"));
                    self.activate_touch(current, tick);
                    return;
                }
                self.touch.chosen_mode = Some(id);
                self.notes.push(format!("{current}: {label} chosen"));
            }
            Action::Redirect(target) => self.redirect_touch(current, &label, &target),
            Action::Launch(launch) => {
                self.notes.push(format!(
                    "{current}: {label} tapped, firing {} for {launch:?}",
                    w2048::LAUNCH_2048
                ));
                self.touch.launch = Some(launch);
                self.machine.fire(w2048::LAUNCH_2048);
            }
        }
    }

    /// A tile whose tap leaves for `target`, with the checks a target needs
    /// on these grids.
    ///
    /// `pub(super)` rather than private: [`super::team`] and
    /// [`super::options2048`] both leave their own screens through this same
    /// path rather than duplicating the `PreviousScreen`/history handling
    /// below.
    pub(super) fn redirect_touch(&mut self, current: &str, label: &str, target: &str) {
        // The confirm tick on the mode grid: only a chosen mode this build
        // can follow leaves. See the module docs.
        if current == w2048::GAME_MODE_CHOICE {
            match self.touch.chosen_mode.as_deref() {
                Some("FE_SP_CAMPAIGN") => {}
                Some(mode) => {
                    self.notes.push(format!(
                        "{current}: {mode} needs a network session this build does not have"
                    ));
                    return;
                }
                None => {
                    self.notes.push(format!("{current}: no mode chosen yet"));
                    return;
                }
            }
        }
        // `redirect="PreviousScreen"` is Wipeout 2048's own back gesture -
        // `Team_Definition.xml`'s `select_button` and `manual3D`'s tick both
        // author this literal target rather than a screen name. Popped from
        // `TouchState::history` rather than looked up in the state machine:
        // there is no `"PreviousScreen"` state to find.
        if target == "PreviousScreen" {
            let Some(previous) = self.touch.history.pop() else {
                self.notes.push(format!(
                    "{current}: {label} tapped, but nothing is on the back stack"
                ));
                return;
            };
            self.notes.push(format!(
                "{current}: {label} tapped, returning to {previous}"
            ));
            self.touch.selected = 0;
            self.on_screen_for = 0.0;
            self.machine.fire(&previous);
            return;
        }
        // `Home`'s five destinations are in includes this build does not
        // load (`oag_2048::frontend::includes::FOLLOWED`), so their names
        // resolve to no state; the machine would ignore the fire silently,
        // and a tap that does nothing should at least say why.
        if !self.machine.contains(target) {
            self.notes.push(format!(
                "{current}: {label} tapped, but {target} is a screen this build does not load"
            ));
            return;
        }
        self.notes
            .push(format!("{current}: {label} tapped, firing {target}"));
        self.touch.history.push(current.to_string());
        self.touch.selected = 0;
        self.on_screen_for = 0.0;
        self.machine.fire(target);
    }

    /// The pointer on a touch grid: hovering selects, a click activates, a
    /// second click on a chosen toggle confirms through the tick.
    ///
    /// Returns whether a touch grid was on screen to take it.
    pub(super) fn touch_pointer(&mut self, pointer: &Pointer) -> bool {
        let Some(current) = self.machine.current().map(str::to_string) else {
            return false;
        };
        match current.as_str() {
            w2048::NEW_FE_SHELL => return self.campaign_map_pointer(pointer),
            w2048::TEAM => return self.team_pointer(pointer),
            w2048::OPTIONS_CAMERA
            | w2048::OPTIONS_AUDIO
            | w2048::OPTIONS_CONTROLS
            | w2048::OPTIONS_PILOT => return self.options_panel_pointer(pointer),
            w2048::GAME_MODE_CHOICE
            | w2048::HOME
            | w2048::PROFILE
            | w2048::PROFILE_STATS
            | w2048::OPTIONS
            | w2048::COMMUNITY_ADHOC_CHECK
            | w2048::EXTRAS
            | w2048::EXTRAS_MANUAL
            | w2048::EXTRAS_CREDITS => {}
            _ => return false,
        }
        if pointer.is_idle() {
            return true;
        }
        let Some(at) = pointer.at else {
            return true;
        };
        let hit = self
            .tiles(&current)
            .iter()
            .position(|tile| contains(tile.rect, at));
        if pointer.moved
            && let Some(hit) = hit
        {
            self.touch.selected = hit;
        }
        if pointer.clicked
            && let Some(hit) = hit
        {
            self.touch.selected = hit;
            self.activate_touch(&current, hit);
        }
        true
    }

    /// The tiles of screen `name`, over its own header widgets already in
    /// `out`.
    pub(super) fn draw_touch(&self, name: &str, out: &mut Vec<Draw>) {
        let Some(screen) = self.screens.by_name(name) else {
            return;
        };
        let tile = self.global_colour("Blue2048");
        let gated = self.global_colour("Grey2048");
        let cursor = self.global_colour("Orange2048");
        self.underlay_header_blocks(screen, tile, out);
        for (at, drawn) in self.tiles(name).iter().enumerate() {
            // The three network modes are `Grey2048` on the capture, taken
            // with no PSN session - the state this build is always in.
            let colour = match &drawn.action {
                Action::Toggle(id) if name == w2048::GAME_MODE_CHOICE && id != "FE_SP_CAMPAIGN" => {
                    gated
                }
                _ => tile,
            };
            self.draw_tile(drawn, colour, at == self.touch.selected, cursor, out);
        }
    }

    /// The `Blue2048` block under a screen's white header icon.
    ///
    /// `GameModeChoice` authors `Icon_Mode_HomeBut.gtf` at `(15, 15)`,
    /// `70x70`, `color="FEGlobals->White2048"` - a white glyph, which on
    /// the capture sits on a `Blue2048` square at exactly that rect
    /// (`08-game-mode-grid-clean.png`, x 16-85, y 16-85). The square is
    /// not a widget; it is what the screen type draws under its header.
    /// Measured, one capture, confidence 80. The 4-unit rule the capture
    /// runs from the square to x=425 is not drawn: its right end is not
    /// derivable from anything the file authors.
    ///
    /// Slotted in under the images rather than appended, since the draw
    /// list is drawn in order and the glyph has to land on top.
    fn underlay_header_blocks(&self, screen: &Screen, tile: [f32; 4], out: &mut Vec<Draw>) {
        let white = self
            .screens
            .globals
            .get("White2048")
            .and_then(|value| parse_argb(value));
        let blocks: Vec<Draw> = screen
            .images
            .iter()
            .filter(|image| white.is_some_and(|white| image.color == white))
            .filter_map(|image| {
                Some(Draw::Fill {
                    rect: [image.x, image.y, image.width?, image.height?],
                    color: tile,
                })
            })
            .collect();
        // After the base fills, before the first sprite.
        let at = out
            .iter()
            .position(|draw| matches!(draw, Draw::Sprite { .. }))
            .unwrap_or(out.len());
        out.splice(at..at, blocks);
    }

    fn draw_tile(
        &self,
        tile: &Tile,
        colour: [f32; 4],
        selected: bool,
        cursor: [f32; 4],
        out: &mut Vec<Draw>,
    ) {
        let rect = tile.rect;
        // Every button is its own `Blue2048` box, labelled or not: the
        // bare `cross.gtf` back button on `10-adhoc-game-list.png` is a
        // 122x96 box - its authored size - with the glyph centred in it.
        out.push(Draw::Fill {
            rect,
            color: colour,
        });
        if selected {
            self.draw_cursor_ring(rect, cursor, out);
        }
        if let Some(placed) = tile.src.as_deref().and_then(|src| {
            self.placements
                .iter()
                .find(|(name, _)| name == src)
                .map(|(_, placed)| *placed)
        }) {
            // The texture's own size, centred - measured, see the module
            // docs.
            let w = placed.width as f32;
            let h = placed.height as f32;
            out.push(Draw::Sprite {
                rect: [
                    rect[0] + (rect[2] - w) * 0.5,
                    rect[1] + (rect[3] - h) * 0.5,
                    w,
                    h,
                ],
                uv: [
                    placed.x as f32,
                    placed.y as f32,
                    placed.width as f32,
                    placed.height as f32,
                ],
                color: tile.color,
            });
        }
        let Some(label) = &tile.label else {
            return;
        };
        let (y, color) = if tile.label_inside {
            // Centred in the box, white on the tile - the game-list
            // screen's text buttons. The line count is the wrap's: a
            // label wider than the box breaks once, and this face's
            // glyphs average nine units at the label scale.
            let lines = if tile.wrap.is_some_and(|w| label.len() as f32 * 9.0 > w) {
                2.0
            } else {
                1.0
            };
            let line = self.default_line_height.unwrap_or(37.0) * LABEL_SCALE;
            (
                rect[1] + (rect[3] - lines * line) * 0.5,
                [1.0, 1.0, 1.0, 1.0],
            )
        } else {
            (
                rect[1] + rect[3] + LABEL_GAP - PEN_ABOVE_CAPS * LABEL_SCALE,
                colour,
            )
        };
        out.push(Draw::Text {
            x: rect[0] + rect[2] * 0.5,
            y,
            scale: LABEL_SCALE,
            color,
            border: None,
            align: Align::Centre,
            text: label.clone(),
            wrap_width: tile.wrap,
        });
    }

    /// The pad cursor: a ring outside `rect`. Chosen - see the module docs.
    pub(super) fn draw_cursor_ring(&self, rect: [f32; 4], cursor: [f32; 4], out: &mut Vec<Draw>) {
        out.extend(cursor_ring(rect, cursor));
    }

    /// A skin colour by its global name, or white: the names this reads
    /// (`Blue2048`, `Grey2048`, `Orange2048`) are declared in
    /// `NEWGUI/Skin.xml` and resolved into every include as fallbacks.
    pub(super) fn global_colour(&self, name: &str) -> [f32; 4] {
        self.screens
            .globals
            .get(name)
            .and_then(|value| parse_argb(value))
            .map_or([1.0, 1.0, 1.0, 1.0], argb_to_rgba)
    }
}

/// The pad cursor's ring around `rect`, four bars [`CURSOR_RING`] thick.
/// Shared with the screens that reuse this front end's tile idiom
/// (`oag_ui_screens::endrace::touch`). Chosen, not measured.
#[must_use]
pub fn cursor_ring(rect: [f32; 4], color: [f32; 4]) -> [Draw; 4] {
    let [x, y, w, h] = rect;
    let r = CURSOR_RING;
    [
        [x - r, y - r, w + 2.0 * r, r],
        [x - r, y + h, w + 2.0 * r, r],
        [x - r, y, r, h],
        [x + w, y, r, h],
    ]
    .map(|rect| Draw::Fill { rect, color })
}

/// A button's own square.
fn tile_rect(button: &TouchButton) -> [f32; 4] {
    [button.x, button.y, button.width, button.height]
}
