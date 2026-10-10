//! Wipeout 2048's campaign map: `SP.xml`'s events, on the `newFEshell`
//! screen the disc's own `<TouchCampaign>` and `<FE3DCanvas>` live on, each
//! a tap away from its card (`event_card`), and from `Launch 2048` past that.
//!
//! # What is authored, and what is not
//!
//! **Authored**: every event and everything said about it - its name, its
//! circuit, its kind, class and lap count, and its `M_X`/`M_Y` map cell -
//! comes off `Data\xml\SP.xml` through `oag_2048::campaign`
//! (`docs/formats/2048-campaign.md`); the screen it sits on, and the
//! `redirect="Launch 2048"` a tap fires, are `NEWGUI/Definition.xml`'s
//! (`docs/formats/2048-frontend.md`). The scrollable area is the shell's own
//! `<TouchScroll>`: a 960x544 view over `MaxScrollX="960"
//! maxscrolly="544"`, so a canvas twice the screen each way.
//!
//! **Chosen, not measured - no confidence score**: how a cell maps to a
//! pixel. `M_X`/`M_Y` are real `GameModeBase` fields the executable does
//! deserialise (`docs/ghidra/functions/vita-2048-eu-v104/
//! frontend-campaign-map.md`'s 2026-09-21 section: struct offsets `0x2c4`/
//! `0x2c8`, found after an earlier pass the same day missed two
//! non-auto-stringified `FUN_812dab08` registrations), but they land in a
//! *different* struct offset from the one the DLC tiers' own hotspot
//! functions read (`+0x15c`/`+0x160`) - some unfound conversion between the
//! two is where the real formula lives, not a bare read of `m_x`/`m_y`
//! itself. A live Vita3K capture the same pass
//! (`data/reference/2048-frontend/README.md`, frames `12`-`14`) confirms the
//! real map is not this grid at all: a **hexagonal** tessellation of
//! icon-on-hexagon markers (chequered flag / stopwatch, coloured by
//! locked/passed state), with a season title card
//! (`A·G·R·C 2048`/`2049`/`2050`) drawn inline on the same scrollable canvas
//! at each season's own cluster - not a `MenuSkin`-style corner widget, and
//! not any of the `<CanvasLabel>`s `2048-frontend.md` already accounts for.
//! Neither the exact per-event anchor formula nor the season-card asset was
//! recovered this pass (`M_BUTTONSHAPE`'s own `CanvasButtonShape` enum is a
//! plausible icon selector, `M_CANVASTWEAK_X`/`_Y` a plausible small pixel
//! nudge, neither confirmed - no consuming function found for either), so
//! this build still lays cells out on an even grid - [`PITCH`] units per
//! cell from [`ORIGIN`] - and draws each event as a plain [`MARKER`]-sized
//! square, coloured by its own progress (see this module's "Progression"
//! section below) rather than a hexagon shape nothing here has a decoded
//! source for. The city behind the real map does not exist as a 3D backdrop
//! either way: the Vita3K capture's own tiles sit on a flat light
//! triangle-outline background, corroborating `2048-frontend.md`'s "not a
//! real 3D scene" finding from the opposite direction.
//!
//! **2026-09-25: [`CANVAS`]/[`PITCH`]/[`ORIGIN`]/[`MARKER`] are now scaled
//! against a direct pixel measurement of frames `12`/`13`, not the raw
//! `MaxScrollX="960" maxscrolly="544"` reading.** That reading treats the
//! `<TouchScroll>`'s declared scroll range as the *whole* canvas
//! (view + scroll = 1920x1088), which is what the shell's own XML states -
//! but the two live frames' own scrollbar thumbs measure far smaller than a
//! 2x-view canvas would draw: the vertical thumb spans `y` 41-143 of a
//! 544-tall view (103 px, a 5.28x canvas/view ratio) and the horizontal
//! thumb spans `x` 84-260 in frame `12` versus `142`-`318` in frame `13`
//! (177 px of a 960-wide view, a 5.42x ratio, and the two frames' thumb
//! positions moving together with the scroll a tapped event caused is
//! itself corroboration this is a scrollbar and not another widget). That
//! puts the *real* canvas at roughly 5207x2873 - 2.71x/2.64x bigger than the
//! 1920x1088 this build assumed - so [`CANVAS`], [`ORIGIN`] and [`PITCH`]
//! are scaled up by the same factor, keeping the file's own `x` 1-34/`y`
//! 1-25 cells filling the corrected canvas the same way the old, smaller
//! numbers filled the wrong one. [`MARKER`] is not scaled by that same
//! factor - it is set directly off a second, independent measurement: a
//! flood-filled bounding box around one clean, unclipped green hex tile in
//! frame `13` (`x` 352-474, `y` 127-244 - 122x117 px), landing this build's
//! square between that figure and the canvas-derived scale-up of the old
//! 36 px marker (36 * 2.71 ≈ 97.6). Both measurements are pixel counts off
//! the committed reference frames, not a decompile, so this is real
//! evidence rather than a tuned-to-look-right guess - but the grid itself
//! is still a square standing in for a hexagon nothing here draws, so the
//! whole scheme stays **chosen, not measured**, now with a better-fitted
//! scale rather than a wrong one.
//!
//! # 2026-09-27: the marker itself is the disc's own hex art, not a flat square
//!
//! **The grid's own scale and the per-event placement formula are still
//! exactly as the section above leaves them - chosen, not measured, same
//! even grid.** What changed is what each cell draws *with*: this pass
//! decoded every plausible hex/icon filename the 2026-09-25 pass's own
//! survey had only grepped for **by name against XML text**, never opened as
//! pixels (`data/FE/Images/hex_{filled,outline,select}.gxt`,
//! `data/FE/NewImages/callout/{race,speed,zone,combat}_mode.gxt`, all in the
//! base package, none named by any `NEWGUI/*.xml` widget - the same
//! "native-code-driven, unlocated in a widget" situation `canvasTexture.gxt`
//! is already in). All seven decode to real, unambiguous art:
//! `hex_filled`/`hex_outline`/`hex_select` are a gloss-filled hexagon, a thin
//! hexagonal outline and a thicker glow ring; `race_mode` is a chequered
//! flag, `speed_mode` a stopwatch, `zone_mode` a radar/target glyph and
//! `combat_mode` a crosshair.
//!
//! **Confirmed against the same committed frames the grid's own scale was
//! measured off, not just visually plausible in isolation.** Frames `12`
//! and `13` (`data/reference/2048-frontend/`) show green hexagon tiles
//! carrying a chequered-flag glyph and a stopwatch glyph, grey ones carrying
//! the same two glyphs unlit - exactly `race_mode`'s flag and `speed_mode`'s
//! stopwatch, on exactly the green/grey split [`ProgressState::Passed`]/
//! [`ProgressState::Locked`] already draw with `Pass2048`/`Grey2048`. This
//! is a stronger correction than the 2026-09-25 pass could make with a name
//! search alone: `hex_filled.gxt`, `hex_outline.gxt` and the four
//! `*_mode.gxt` icons all decode to pixels a live capture already showed
//! sitting on the map, not merely names that were never ruled out.
//!
//! **Still not the real thing, honestly**: the real tile is lit with a 3D
//! bevel and drop shadow this build's flat colour-tinted sprite does not
//! reproduce; the striped/dotted "reachable path" texture connecting a
//! season's own tiles (visible behind the hexagons in both frames) has no
//! decoded source and is not drawn; the season title card
//! (`A·G·R·C 2048`/`2049`/`2050`, frame `12`'s own left edge) and the header
//! (home button, top-right badge cluster) remain undrawn, same gaps the
//! 2026-09-25 pass already named. [`EventIcon`] itself is measured at the
//! same confidence `EventKind` already carries (78-82,
//! `docs/formats/2048-campaign.md`), since it is a direct re-reading of that
//! same field - not a new guess.
//!
//! Wired through `oag_game::boot::sprites::load`'s own `extra` mechanism
//! (the same one the menu blocks' nine-patch already used) - see
//! [`HEX_FILLED`] and [`EventIcon::texture_name`]'s own doc comments. A
//! sheet with none of these seven (every `oag-ui` unit test in this crate,
//! which boots with an empty sheet) falls back to the original flat square
//! and rectangle cursor ring exactly as before - see [`Frontend::placed`]
//! and [`Frontend::draw_campaign_map`].
//!
//! # The bottom panel is gone, and no event-card stand-in replaces it
//!
//! **2026-09-25.** Until this pass, tapping (pad or pointer) drew a
//! full-width bar under the map naming the selected event's name/circuit/
//! mode/laps/weapons - this build's own invented chrome, already labelled
//! "chosen" in an earlier revision of this doc comment. It covered roughly
//! 15% of the screen; the live capture shows nothing of the kind on the base
//! map. What the real game shows on a tap is a completely different screen
//! (`data/reference/2048-frontend/14-campaign-map-event-card-unity-square.png`):
//! a photo backdrop, the event's name and kind, a `PASS`/objective line, a
//! pair of lap-count arrows, three pagination dots and three bottom buttons -
//! and per that frame's own caption, it opens for a **locked** node too, not
//! only an open one, so it is a browsing/preview surface as much as a launch
//! confirmation.
//!
//! That screen was searched for, this pass, as XML the way [`PITCH`] and
//! [`MARKER`] were measured as pixels - and it is not there. Every lead
//! traces to a dead end: `Definition.xml`'s own `<TouchCampaign>` carries
//! exactly one child, `redirect="Launch 2048"`, in both the base package and
//! the `v1.04` patch (`data/plugins/frontend/NEWGUI/InGame_Definition.xml`'s
//! `Launch 2048` screen is byte-identical between the two); that screen is
//! nothing but a `<BackendController task="Launch">` straight through to
//! `InGame2048`, with no confirm step of any kind in between. The two
//! screens that share the tick/cross vocabulary a confirm dialog would use -
//! `StartEventConfirm`/`FriendStartEventConfirm`
//! (`data/plugins/frontend/NEWGUI/Community_Definition.xml`) - are a small
//! 660x192 online-only "join this friend's event?" box with two buttons and
//! one line of text, nothing like frame `14`'s three-button, photo-backed
//! card; nothing else in any of the 25 `NEWGUI/` documents across the base
//! package, the patch's two archives or either DLC package names a screen
//! shaped like it. Reproduced with `cargo run -p oag-tools --example
//! psarc_grep -- <data.psarc> <needle>` against `TouchCampaign`, `Launch
//! 2048` and every plausible screen name pulled from the archive's own
//! `Screen name="..."` list, and confirmed with the added `dump_screen_scratch`
//! scratch tool (dumps any one named `<Screen>` block, the same crude
//! nesting-depth walk `dump_newfeshell.rs` already used for `newFEshell`
//! alone). The strings the card shows (`ER_FINISH_5TH` and its siblings in
//! `data/plugins/languages/*/entries.xml`) are real and already shared with
//! `EndRace_Definition.xml`'s own post-race objective screen, so the card's
//! *text* is authored - but its layout, its per-event photo backdrop and its
//! button chrome are not, the same "native-code-driven, unlocated in any
//! XML" conclusion this module's own hex-tile/background/header findings
//! already reached.
//!
//! Per this project's own "never invent" rule, an unlocated screen is not
//! stood in for - so nothing replaces the removed bar. The gap this leaves:
//! a player who taps or pads onto an event today sees only its marker
//! recolour under the cursor ring ([`Frontend::draw_cursor_ring`]) - no
//! name, no circuit, no objective, nothing that says what pressing confirm
//! would start. [`Frontend::selected_event`] and [`MapEvent::detail`] still
//! carry the real, disc-authored text (`crates/game/src/boot/campaign2048.rs`
//! builds it from `SP.xml`) for whenever the card itself is recovered - nothing
//! about removing the bar touches how that text is built, only where it was
//! drawn.
//!
//! **2026-09-28: the card's own *code* is located, not its layout.**
//! `docs/ghidra/functions/vita-2048-eu-v104/campaign-event-card.md` -
//! `CampaignEventCard_HandleInput`/`Draw` (`0x810f2164`/`0x810f1196`), found
//! chasing the user's own play observation that 2048 restricts craft choice
//! on some events. Its own three bottom buttons are Launch (refused when the
//! current craft fails `GameModeBase_IsShipTypeAllowed`), Back, and Change
//! Craft (redirected to `Team_Definition.xml`'s `team` screen, unfiltered -
//! the same screen [`super::team`] already implements). The photo backdrop,
//! per-event art and pagination dots this doc comment's own paragraphs above
//! describe are still unlocated - only the button logic was decompiled -
//! so the "never invent" conclusion for the *layout* is unchanged.
//! [`Frontend::launch_selected_event`] below now applies the same
//! craft-restriction gate the original's card does, on this map screen
//! rather than waiting on the card - see [`MapEvent::refused_craft`] and
//! `docs/formats/2048-campaign.md`'s "Craft choice" section.
//!
//! # Progression: locked, open, passed, elite
//!
//! **Resolved 2026-09-21.** [`MapEvent::requires`] is authored data - the
//! single event `oag_2048::campaign::unlock_gates` names as this one's own
//! prerequisite, folding its `M_PNEXTEVENT`/`M_PBRANCHEVENT` chain edge and
//! its `M_PEVENTREQUIRED` field into one name (see that function's own doc
//! for why one name is enough for every case `SP.xml` authors). What tier a
//! finished attempt at an event earned is not authored here at all - a
//! player's own save, read by whatever calls
//! [`Frontend::refresh_campaign_progress`] once at boot (this crate carries
//! no persistence of its own, and must not: nothing gameplay- or
//! session-facing may depend on `oag-ui`, so the medal lookup arrives as a
//! plain closure rather than a `Store` reference). [`ProgressState`] folds
//! the two together: an event whose own [`MapEvent::requires`] has not been
//! passed draws [`ProgressState::Locked`] and refuses a launch
//! ([`Frontend::launch_selected_event`]); everything open from the start, or
//! opened by a since-passed prerequisite, draws [`ProgressState::Open`],
//! [`ProgressState::Passed`] or [`ProgressState::Elite`] off whatever the
//! closure answers for its own name.
//!
//! The four colours this draws with are the disc's own -
//! `Skin.xml`'s `Grey2048`/`Blue2048`/`Pass2048`/`ElitePass2048` globals,
//! already reachable through [`Frontend::global_colour`] - never an invented
//! palette. `HardcorePass2048`, the fifth global that file declares, is not
//! used: nothing in `SP.xml` authors a third objective tier alongside
//! `M_PASSOBJECTIVE`/`M_ELITEOBJECTIVE` (measured - see
//! `oag_2048::campaign::objective_type`'s own doc comment), so that colour
//! most plausibly belongs to a Hardcore *difficulty* flag this pass found no
//! authored data for, not a rung this map ever needs to draw.

use crate::kinetic::{Bounds, Extent, Kinetic};
use crate::pointer::{Pointer, contains};

use super::touch::{LABEL_SCALE, Launch};
use super::*;
use oag_2048::frontend::states as w2048;
use oag_2048::race::TEAM_VARIANTS;

/// The real scrollable canvas - not the shell's own declared
/// `MaxScrollX="960" maxscrolly="544"` (view + that = 1920x1088), but that
/// figure scaled by the two live frames' own scrollbar-thumb measurement
/// (~5.3x-5.4x the view, not 2x) - see this module's own doc comment for
/// the pixel ranges.
const CANVAS: (f32, f32) = (5207.0, 2873.0);
/// Units per map cell, scaled off [`CANVAS`]'s own correction so the file's
/// cells still fill the canvas the way the old, smaller numbers did.
const PITCH: (f32, f32) = (146.0, 111.0);
/// Where cell `(1, 1)` lands, scaled the same way as [`PITCH`].
const ORIGIN: (f32, f32) = (65.0, 63.0);
/// A marker's side - not scaled with the rest, but set directly off a real
/// hex tile's own measured bounding box in frame `13`. See this module's own
/// doc comment.
const MARKER: f32 = 108.0;

/// The disc's own hex tile art (`data/FE/Images/hex_{filled,outline,select}.gxt`),
/// decoded and visually confirmed 2026-09-27: a gloss-filled hexagon, a thin
/// hexagonal outline and a thicker glow ring, all matching the hexagonal
/// silhouette the live Vita3K capture already showed. **Not named by any
/// front-end widget**, the same "native-code-driven, no authoring `<Image>`"
/// situation `canvasTexture.gxt` is already in, so
/// `oag_game::boot::sprites::load` asks for them by name through its own
/// `extra` list, the same mechanism the menu-block art already uses. Drawn
/// in place of the flat colour square whenever the sheet decoded them
/// (`Frontend::placed` returning `Some`); falls back to the old flat square
/// otherwise, so a build with no sprite sheet (every `oag-ui` unit test in
/// this crate) draws exactly what it always did rather than a blank hole.
/// Confidence 80: the shape and the file names are a direct pixel read, not
/// a guess, but which exact draw call composites them (still unfound, see
/// this module's own doc comment above) is not, so the per-event pixel
/// formula and the hex tessellation itself stay **chosen, not measured**.
pub const HEX_FILLED: &str = r"Data\FE\Images\hex_filled.gtf";
/// See [`HEX_FILLED`].
pub const HEX_OUTLINE: &str = r"Data\FE\Images\hex_outline.gtf";
/// See [`HEX_FILLED`]. Drawn only on the selected marker, in `Orange2048`,
/// in place of [`Frontend::draw_cursor_ring`]'s rectangle ring when it
/// decoded.
pub const HEX_SELECT: &str = r"Data\FE\Images\hex_select.gtf";

/// Which of the disc's own four campaign-event icons an event's own kind
/// draws, `Data\FE\NewImages\callout\*_mode.gtf` - decoded and visually
/// confirmed 2026-09-27: `race_mode` is a chequered flag, `speed_mode` a
/// stopwatch, `zone_mode` a radar/target glyph and `combat_mode` a
/// crosshair, exactly the "chequered flag / stopwatch" icon this module's
/// doc comment already speculated existed on the real map. **This crate's
/// own copy of `oag_2048::campaign::EventKind`** (plus the `laps == 0`
/// Speed Lap split `oag_2048::campaign::engine_mode` already makes), kept
/// separate so `oag-ui` never depends on `oag_2048` or `oag_tables` - the
/// same reason [`EarnedTier`] is its own copy of `oag_2048::campaign::Tier`.
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEvent {
    /// The `SP.xml` instance name - what `Launch::Event` carries.
    pub name: String,
    /// `M_X`/`M_Y`, the file's own cell.
    pub x: i32,
    pub y: i32,
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
    earned: Vec<Option<EarnedTier>>,
    pub(super) selected: usize,
    /// The view's scroll over the canvas, x then y, in grid units.
    scroll: [Kinetic; 2],
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
}

impl CampaignMap {
    fn marker(event: &MapEvent) -> [f32; 4] {
        [
            ORIGIN.0 + (event.x - 1) as f32 * PITCH.0,
            ORIGIN.1 + (event.y - 1) as f32 * PITCH.1,
            MARKER,
            MARKER,
        ]
    }

    /// The view's scroll over the canvas, in grid units.
    pub(super) fn scroll(&self) -> (f32, f32) {
        (self.scroll[0].offset(), self.scroll[1].offset())
    }

    /// One axis of the canvas: its range, its marker pitch, and a rubber
    /// band half the view long past either end. Unsnapped - the map is a
    /// free canvas with no item to come to rest on.
    fn extent(axis: usize, view: (f32, f32)) -> Extent {
        let (canvas, view, pitch) = match axis {
            0 => (CANVAS.0, view.0, PITCH.0),
            _ => (CANVAS.1, view.1, PITCH.1),
        };
        Extent {
            pitch,
            max_fling: crate::kinetic::MAX_FLING,
            friction: crate::kinetic::FRICTION,
            snap: false,
            bounds: Bounds::Clamp {
                min: 0.0,
                max: (canvas - view).max(0.0),
                band: view * 0.5,
            },
        }
    }

    /// Scrolls so the selected marker is as close to the middle of the
    /// view as the canvas allows.
    fn follow(&mut self, view: (f32, f32)) {
        let Some(event) = self.events.get(self.selected) else {
            return;
        };
        let [x, y, w, h] = Self::marker(event);
        self.scroll[0].set((x + w * 0.5 - view.0 * 0.5).clamp(0.0, (CANVAS.0 - view.0).max(0.0)));
        self.scroll[1].set((y + h * 0.5 - view.1 * 0.5).clamp(0.0, (CANVAS.1 - view.1).max(0.0)));
    }

    /// Pans the view by a finger (the canvas follows it, coasts after a
    /// flick and bounces off the edges, through [`crate::kinetic`]) or a
    /// wheel (one marker row per detent), and returns the pointer as the
    /// map should read it - without the tap that caught a coasting map.
    ///
    /// **Chosen, not measured**: the original's `<TouchScroll>` is a Vita
    /// touch widget whose feel was not measured, and how a mouse wheel
    /// moves it is ours. The selection stays where it is - a pan reveals
    /// the map, it does not pick an event, and the next pad move re-centres
    /// on the selection.
    fn pan(&mut self, pointer: &Pointer, view: (f32, f32)) -> Pointer {
        if pointer.scroll != 0 {
            let y = Self::extent(1, view);
            let wheel = pointer.scroll as f32 * PITCH.1;
            let to = (self.scroll[1].offset() + wheel).clamp(0.0, (CANVAS.1 - view.1).max(0.0));
            self.scroll[1].set(to);
            self.scroll[1].settle(&y);
        }
        let seen = self.scroll[0].gesture(pointer, -pointer.drag.0, &Self::extent(0, view));
        self.scroll[1].gesture(&seen, -pointer.drag.1, &Self::extent(1, view))
    }

    /// Advances a coast or a bounce by one tick.
    pub(super) fn tick(&mut self, dt: f64, view: (f32, f32)) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a tick is milliseconds; f32 holds it exactly"
        )]
        let dt = dt as f32;
        for (axis, scroll) in self.scroll.iter_mut().enumerate() {
            scroll.tick(dt, &Self::extent(axis, view));
        }
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
    /// cursor starts on the first one named `2048 - Event 1` when there is
    /// one - the first season's first event by name, which is a reading of
    /// the names and not of the unlock graph - and on the first event
    /// otherwise.
    ///
    /// Every event reads [`ProgressState::Open`] or [`ProgressState::Locked`]
    /// (off its own [`MapEvent::requires`] alone, nothing earned yet) until
    /// [`Self::refresh_campaign_progress`] is called - a caller with a save
    /// to read should call it once, straight after this.
    pub fn set_campaign(&mut self, events: Vec<MapEvent>) {
        self.campaign.selected = events
            .iter()
            .position(|event| event.name == "2048 - Event 1")
            .unwrap_or(0);
        self.campaign.earned = vec![None; events.len()];
        self.campaign.events = events;
        self.campaign.scroll = Default::default();
        self.campaign.follow(self.space.size);
    }

    /// Folds a player's own save into the map: `earned(name)` is asked once
    /// per event and answers the tier that event's own best-ever result
    /// earned, or `None` for one never finished with at least a pass.
    ///
    /// A plain closure rather than a `Store` reference, on purpose - see
    /// this module's own "Progression" section for why `oag-ui` cannot
    /// depend on `oag_game::records` at all.
    pub fn refresh_campaign_progress(&mut self, earned: impl Fn(&str) -> Option<EarnedTier>) {
        self.campaign.earned = self
            .campaign
            .events
            .iter()
            .map(|event| earned(&event.name))
            .collect();
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
                if let Some(next) = self.nearest_event(direction) {
                    self.campaign.selected = next;
                    self.campaign.follow(self.space.size);
                }
                return;
            }
        }
        if input.is_pressed(Button::Cross) || input.is_pressed(Button::Start) {
            input.consume_press(Button::Cross);
            input.consume_press(Button::Start);
            self.open_event_card();
        }
    }

    /// The event nearest the cursor in `direction`, by cell distance,
    /// among those that lie at least one cell that way.
    fn nearest_event(&self, direction: (f32, f32)) -> Option<usize> {
        let from = self.campaign.events.get(self.campaign.selected)?;
        self.campaign
            .events
            .iter()
            .enumerate()
            .filter(|(at, _)| *at != self.campaign.selected)
            .filter_map(|(at, event)| {
                let dx = (event.x - from.x) as f32;
                let dy = (event.y - from.y) as f32;
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
        self.touch.launch = Some(Launch::Event(event.name.clone()));
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
        let (sx, sy) = self.campaign.scroll();
        let at = (at.0 + sx, at.1 + sy);
        let hit = self
            .campaign
            .events
            .iter()
            .position(|event| contains(CampaignMap::marker(event), at));
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
                self.campaign.follow(self.space.size);
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
            uv: [
                placed.x as f32,
                placed.y as f32,
                placed.width as f32,
                placed.height as f32,
            ],
            color,
        }
    }

    /// The map: every marker in view, the cursor ring, and the panel.
    ///
    /// Each marker's own colour is one of the disc's own four -
    /// [`ProgressState::Locked`] draws `Grey2048`, [`ProgressState::Open`]
    /// the same `Blue2048` this always drew, [`ProgressState::Passed`]
    /// `Pass2048` and [`ProgressState::Elite`] `ElitePass2048` - see this
    /// module's own "Progression" section for why no fifth colour is drawn.
    ///
    /// **Each marker is the disc's own hex tile art since 2026-09-27**, when
    /// the sheet has it - see [`HEX_FILLED`]'s own doc comment - tinted by
    /// the same four colours a flat square always was, with
    /// [`EventIcon::texture_name`]'s mode glyph centred on top. A sheet with
    /// none of it falls back to the original flat square and rectangle
    /// cursor ring, unchanged.
    pub(super) fn draw_campaign_map(&self, out: &mut Vec<Draw>) {
        let (width, height) = self.space.size;
        let open = self.global_colour("Blue2048");
        let cursor = self.global_colour("Orange2048");
        let white = [1.0, 1.0, 1.0, 1.0];
        let (sx, sy) = self.campaign.scroll();
        let hex_filled = self.placed(HEX_FILLED);
        let hex_outline = self.placed(HEX_OUTLINE);
        let hex_select = self.placed(HEX_SELECT);
        for (at, event) in self.campaign.events.iter().enumerate() {
            let [x, y, w, h] = CampaignMap::marker(event);
            let rect = [x - sx, y - sy, w, h];
            if rect[0] + w < 0.0 || rect[1] + h < 0.0 || rect[0] > width || rect[1] > height {
                continue;
            }
            let color = match self.campaign.state_of(at) {
                ProgressState::Locked => self.global_colour("Grey2048"),
                ProgressState::Open => open,
                ProgressState::Passed => self.global_colour("Pass2048"),
                ProgressState::Elite => self.global_colour("ElitePass2048"),
            };
            match hex_filled {
                Some(filled) => {
                    out.push(Self::sprite_at(rect, filled, color));
                    if let Some(outline) = hex_outline {
                        out.push(Self::sprite_at(rect, outline, white));
                    }
                }
                None => out.push(Draw::Fill { rect, color }),
            }
            if let Some(icon) = self.placed(event.kind.texture_name()) {
                // The mode glyph, centred, at a fraction of the marker's own
                // side - **chosen**, since nothing measured the real map's
                // own icon-to-hex size ratio.
                let side = w * 0.42;
                out.push(Self::sprite_at(
                    [
                        rect[0] + (w - side) * 0.5,
                        rect[1] + (h - side) * 0.5,
                        side,
                        side,
                    ],
                    icon,
                    white,
                ));
            }
            if at == self.campaign.selected {
                match hex_select {
                    Some(select) => out.push(Self::sprite_at(rect, select, cursor)),
                    None => self.draw_cursor_ring(rect, cursor, out),
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
