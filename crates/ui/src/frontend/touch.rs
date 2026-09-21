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
//! below the tile's bottom edge, wrapped, 22px between lines.
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
//! own shape, kept rather than folded into one tap. `Home`'s five tiles each
//! carry their own `redirect` and leave on the tap.
//!
//! Only `FE_SP_CAMPAIGN` can be confirmed here: the other three modes are
//! online, ad-hoc and cross-play, and the screens behind them are network
//! sessions this build does not have. Confirming one of those says so in a
//! note and stays.

use crate::pointer::{Pointer, contains};
use crate::screen::{TouchButton, argb_to_rgba, parse_argb};

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
const LABEL_SCALE: f32 = 16.0 / 27.0;

/// Where the touch front end is: which tile the pad is on, which mode has
/// been chosen, and what a confirmed tap asked the composition root for.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TouchState {
    /// Index into the current screen's `touch_buttons`, in document order.
    pub selected: usize,
    /// The `idstring` of the toggle tile last tapped on `GameModeChoice`.
    pub chosen_mode: Option<String>,
    /// The campaign event a tap on the map asked to race, once one has.
    /// Read by the composition root through [`Frontend::launch`].
    pub launch: Option<String>,
}

impl Frontend {
    /// The campaign event the front end has asked to race, if any. `Some`
    /// only once [`Frontend::is_finished`] is, on a title whose front end
    /// fires `Launch 2048` rather than `Launch Game`.
    #[must_use]
    pub fn launch(&self) -> Option<&str> {
        self.touch.launch.as_deref()
    }

    /// The mode chosen on `GameModeChoice`, by its `idstring`.
    #[must_use]
    pub fn chosen_mode(&self) -> Option<&str> {
        self.touch.chosen_mode.as_deref()
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
        }
        let count = self
            .screens
            .by_name(&current)
            .map_or(0, |screen| screen.touch_buttons.len());
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

    /// A tap or a press on button `at` of screen `current`.
    fn activate_touch(&mut self, current: &str, at: usize) {
        let Some(button) = self
            .screens
            .by_name(current)
            .and_then(|screen| screen.touch_buttons.get(at))
            .cloned()
        else {
            return;
        };
        let label = button
            .idstring
            .clone()
            .or_else(|| button.name.clone())
            .unwrap_or_else(|| "confirm".to_string());
        if button.toggle {
            // Two taps: the first chooses, the second confirms through the
            // screen's own tick - the language picker's rule, see the
            // module docs.
            if self.touch.chosen_mode.as_deref() == button.idstring.as_deref()
                && let Some(tick) = self.screens.by_name(current).and_then(|screen| {
                    screen
                        .touch_buttons
                        .iter()
                        .position(|b| !b.toggle && b.idstring.is_none() && b.redirect.is_some())
                })
            {
                self.notes
                    .push(format!("{current}: {label} chosen again, confirming"));
                self.activate_touch(current, tick);
                return;
            }
            self.touch.chosen_mode = button.idstring.clone();
            self.notes.push(format!("{current}: {label} chosen"));
            return;
        }
        let Some(target) = button.redirect.clone() else {
            self.notes
                .push(format!("{current}: {label} names no redirect"));
            return;
        };
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
        // `Home`'s five destinations are in includes this build does not
        // load (`oag_2048::frontend::includes::FOLLOWED`), so their names
        // resolve to no state; the machine would ignore the fire silently,
        // and a tap that does nothing should at least say why.
        if !self.machine.contains(&target) {
            self.notes.push(format!(
                "{current}: {label} tapped, but {target} is a screen this build does not load"
            ));
            return;
        }
        self.notes
            .push(format!("{current}: {label} tapped, firing {target}"));
        self.touch.selected = 0;
        self.on_screen_for = 0.0;
        self.machine.fire(&target);
    }

    /// The pointer on a touch grid: hovering selects, a click activates, a
    /// second click on a chosen toggle confirms through the tick.
    ///
    /// Returns whether a touch grid was on screen to take it.
    pub(super) fn touch_pointer(&mut self, pointer: &Pointer) -> bool {
        let Some(current) = self.machine.current().map(str::to_string) else {
            return false;
        };
        if !matches!(
            current.as_str(),
            w2048::GAME_MODE_CHOICE | w2048::HOME | w2048::NEW_FE_SHELL
        ) {
            return false;
        }
        if pointer.is_idle() {
            return true;
        }
        let Some(at) = pointer.at else {
            return true;
        };
        let hit = self.screens.by_name(&current).and_then(|screen| {
            screen
                .touch_buttons
                .iter()
                .position(|button| contains(tile_rect(button), at))
        });
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
        for (at, button) in screen.touch_buttons.iter().enumerate() {
            // The three network modes are `Grey2048` on the capture, taken
            // with no PSN session - the state this build is always in.
            let colour = if name == w2048::GAME_MODE_CHOICE
                && button.toggle
                && button.idstring.as_deref() != Some("FE_SP_CAMPAIGN")
            {
                gated
            } else {
                tile
            };
            self.draw_tile(button, colour, at == self.touch.selected, cursor, out);
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
    fn underlay_header_blocks(
        &self,
        screen: &crate::screen::Screen,
        tile: [f32; 4],
        out: &mut Vec<Draw>,
    ) {
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
        button: &TouchButton,
        tile: [f32; 4],
        selected: bool,
        cursor: [f32; 4],
        out: &mut Vec<Draw>,
    ) {
        let rect = tile_rect(button);
        // Every button is its own `Blue2048` box, labelled or not: the
        // bare `cross.gtf` back button on `10-adhoc-game-list.png` is a
        // 122x96 box - its authored size - with the glyph centred in it.
        out.push(Draw::Fill { rect, color: tile });
        if selected {
            let [x, y, w, h] = rect;
            let r = CURSOR_RING;
            for ring in [
                [x - r, y - r, w + 2.0 * r, r],
                [x - r, y + h, w + 2.0 * r, r],
                [x - r, y, r, h],
                [x + w, y, r, h],
            ] {
                out.push(Draw::Fill {
                    rect: ring,
                    color: cursor,
                });
            }
        }
        if let Some(placed) = button.src.as_deref().and_then(|src| {
            self.placements
                .iter()
                .find(|(name, _)| name == src)
                .map(|(_, placed)| *placed)
        }) {
            // The texture's own size, centred - measured, see the module
            // docs. `ImageSize` scales it where the widget authors one.
            let scale = button.image_size.unwrap_or(1.0);
            let w = placed.width as f32 * scale;
            let h = placed.height as f32 * scale;
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
                color: argb_to_rgba(button.color),
            });
        }
        let label = button
            .idstring
            .as_deref()
            .map(|id| self.strings.get_or_id(id).to_string())
            .or_else(|| button.string.clone());
        if let Some(label) = label {
            out.push(Draw::Text {
                x: rect[0] + rect[2] * 0.5,
                y: rect[1] + rect[3] + LABEL_GAP - self.label_ascent(),
                scale: LABEL_SCALE,
                color: tile,
                border: None,
                align: Align::Centre,
                text: label,
                wrap_width: button.string_width_limit,
            });
        }
    }

    /// How far above the cap line the `Default` face's pen sits, at the
    /// label's scale: the header's own `y="30"` puts its caps at 40 on the
    /// capture, ten units down at scale 1.
    fn label_ascent(&self) -> f32 {
        10.0 * LABEL_SCALE
    }

    /// A skin colour by its global name, or white with a note-free fallback:
    /// the two names this reads (`Blue2048`, `Orange2048`) are declared in
    /// `NEWGUI/Skin.xml` and resolved into every include as fallbacks.
    fn global_colour(&self, name: &str) -> [f32; 4] {
        self.screens
            .globals
            .get(name)
            .and_then(|value| parse_argb(value))
            .map_or([1.0, 1.0, 1.0, 1.0], argb_to_rgba)
    }
}

/// A button's own square.
fn tile_rect(button: &TouchButton) -> [f32; 4] {
    [button.x, button.y, button.width, button.height]
}
