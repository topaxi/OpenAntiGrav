# Button prompts

A prompt is a string with a button in it: Pulse's "Press ε to continue", HD's
"ε CONFIRM  γ BACK". The disc's faces draw a PlayStation glyph for that
codepoint. This build draws **the disc's glyph whenever the player holds a
PlayStation pad** (or forces `original`), and a substitute for the controller
they actually hold otherwise.

Nothing here replaces the disc's art when it is the right art: with the style
`original`, or `auto` and nothing but a PlayStation pad (or no input yet), the
draw is byte-identical to before this feature (a `--menu-page` still is the
same PNG with and without it).

## Setting

`[controls] prompt_style`, a token, config only - there is no menu row, and
`--prompt-style <token>` overrides it for one run without writing it back:

| Token | Draws |
| --- | --- |
| `auto` (default) | the family of the **last device used** |
| `original` | the disc's own glyphs, whatever is held |
| `playstation`, `xbox`, `nintendo`, `keyboard` | that family, forced |

`auto` answers: a key was pressed last, so the keyboard; a pad was last, so its
family (the disc's own when the family is unknown); nothing touched yet, so the
disc's own. A PlayStation pad under `auto`
is the disc's own glyphs, not PromptFont's PlayStation set (`playstation`
forced gives that). An unknown token is reported and `auto` used.

## What is measured and what is chosen

| Thing | Status | Evidence |
| --- | --- | --- |
| Pulse's `ε γ δ β Λ Ν` are cross, circle, square, triangle, L, R | **measured** | the rendered cells of `pulse_text.fnt`, `Pulse_14.fnt`, `Pulse_20.fnt` on the EU disc, read as pictures; pinned by hash in `crates/ui/tests/prompt_glyphs_ground_truth.rs` |
| HD's `ε γ δ` in `PS_BUTTONS.fnt` are cross, circle, square | **measured** | the same probe on the HD EU disc |
| HD's `Δ Γ Β Α` | **left alone** | four arrow-like navigation marks; the action each stands for was not established, so they draw as the disc draws them |
| Which PromptFont glyph stands for a control on a pad | **chosen, not measured** | `crates/game/src/prompts.rs`; PromptFont ships no Nintendo-coloured face glyphs, so lettered discs stand in |
| Which family a pad is | **chosen** (see below) | `crates/input/src/prompt.rs` |

The table is by **action**: a codepoint maps to the control it depicts
(`oag_title::prompts::Prompt`), per title (`Title::prompts`, with `Scope`:
Pulse's glyphs live in every text face, HD's only in its `Buttons` face).
Nothing branches on a title's name. A region's confirm button is whatever its
string table carries (a Japanese table writes `γ` for confirm); no JP disc is
in this project to check, so that is a reading of the mechanism, not a
measurement.

The pad layer maps the **south** button to cross whatever the pad prints
(`oag_input::pad::map_button`), so the substitute follows position: Xbox
south is `A`, Nintendo south is `B` (and east `A`), PlayStation south is cross.
The keyboard family shows the **key bound** to the control, read from the live
bindings (`ENTER` for cross by default, `BACKSPACE` for circle).

## How a glyph is swapped

`oag_ui::prompt` adds to every loaded face that carries a stand-in one extra
cell per substitute glyph, **scaled into the disc glyph's own box and advance**
(PromptFont's alpha, area-averaged), under private-use codepoints. The
renderer rewrites a string's stand-ins to those codepoints just before drawing
(`Renderer::set_prompt_substitution`), so measurement, centring and word-wrap
see the same advances as before. Switching family at run time is a string
rewrite; no texture is re-uploaded. A wide key (the keyboard's backspace) is
fitted into the box, so on a 12 px Pulse face it is legible as a key but its
label is tiny: a known limit, see the handover thread.

Art: **PromptFont** by Shinmera, SIL Open Font Licence 1.1
([licence](../../licences/PromptFont-OFL.txt), no Reserved Font Name clause).
`scripts/gen-prompt-glyphs.py` rasterises 59 glyphs from the pinned
`promptfont.ttf` (SHA-256 in the script) into `assets/ui/prompts/`. Not game
content.

## How the device is told apart

`oag_input::prompt`, plain functions over a descriptor, an environment lookup
and an injected file reader, so tests need no device and no Steam.

1. **Last used** (`Detector`): a key down switches to the keyboard; a pad press
   or a stick past 0.5 switches to that pad's family. A resting stick does not
   flip it. A pad attached and untouched seeds the family until something is
   used.
2. **Is it a pad at all** (`is_gamepad`, 2026-10-06): the OS lists a
   keyboard's consumer-control / system-control HID interface as a joystick
   (a Steam launch with no controller logged `Keychron Keychron K2 Pro System
   Control`, UUID `03000000-3434-0000-2102-000011010000`, "No mapping found",
   and the prompts flipped to Xbox). A device counts only with a known SDL
   mapping, or at least two real face buttons plus either both left-stick axes
   or a d-pad (four d-pad buttons, or a hat / axis pair; plenty of controllers
   have no stick), and never when its name is a known non-pad interface (`system control`,
   `consumer control`, `keyboard`, `mouse`, `touchpad`, `motion sensors`,
   `power button`). A device that is not a pad is skipped by `Pad::poll_players`
   (so it neither drives a craft nor moves `last used`), by `names` and by
   `first_family`; the game logs it once as `not a gamepad, ignored`.
   Pure-function tests with the Keychron descriptor verbatim, an Xbox pad, a
   DualSense, an unknown pad with real axes and the Steam virtual pad are in
   `crates/input/src/prompt/tests.rs`; the gilrs read of the capabilities
   (`pad_caps`) needs a device and is not covered by a test.
3. **Pad family** (`classify_pad`): USB vendor id (Sony `054c`, Nintendo
   `057e`, Microsoft `045e`, Valve `28de`) from gilrs, then name keywords. A
   pad nothing recognises has **no family**, and `auto` draws the disc's own
   glyphs for it rather than guessing Xbox (it was Xbox before 2026-10-06).
4. **Under Steam**, a pad is usually a Steam Input *virtual* Xbox 360 pad, so
   its own name and ids say Xbox. Steam names the real controller in a file
   whose path is in the `SteamVirtualGamepadInfo` environment variable. SDL
   parses it in
   [`SDL_steam_virtual_gamepad.c`](https://github.com/libsdl-org/SDL/blob/main/src/joystick/SDL_steam_virtual_gamepad.c):
   `[slot N]` sections with `name=`, `VID=`, `PID=` and `type=`, where `type`
   is an `SDL_GamepadType` string (`ps3 ps4 ps5 switchpro joyconleft joyconright
   joyconpair gamecube xbox360 xboxone standard steam`, listed in
   [`SDL_gamepad.c`](https://github.com/libsdl-org/SDL/blob/main/src/joystick/SDL_gamepad.c)).
   `parse_virtual_info` reads the same file; the pad's slot is the trailing
   number of its name (`Microsoft X-Box 360 pad 0`). **No Steamworks SDK is
   needed**: `ISteamInput::GetInputTypeForHandle` would give the same answer
   and cost a dependency.
   A Steam launch hides stderr, so the game's log file records which of these
   variables it saw and whether the file exists: see
   [`logging.md`](../architecture/logging.md), "The log file".
5. **The Deck's own controls**: `SteamDeck=1` (set by Steam for a game started
   on a Deck) with a Steam or Microsoft vendor pad and no file is Xbox-shaped,
   as is SDL's `steam` type.

**Deferred**: the live Steam Input pad check waits until the maintainer has a
controller; the 2026-10-06 launch log had none.

**Not verified here**: this sandbox has no Steam client, so the Steam path is
tested against the file format SDL's source defines and a hand-written sample,
not against a real launch. `SteamDeck`, `SteamAppId` and `SteamGameId` are the
variables Steam is known to set for a launched game; their contents were not
read from a live Steam. First check on a Deck: launch through Steam with
`--prompt-style auto` and read the first line of the log, which names the family
(see the handover thread).

## Where it applies

Every screen drawn through the front end's renderer: the menus, the campaign
footers and the language picker (menu stage and boot sequence), a finished
race's EndRace screens, and the pause menu (a `Stage::Menu` over the parked
race). **The in-race HUD, the resume countdown line and the race loading
screen draw no stand-in at all**, so there is nothing for them to swap:
a string census over the English tables of Pulse PSP and PS2, Pure and HD
found stand-ins only in front-end prompts (`FE_PRESS_TO_*`, `FE_*_BUTTON`,
`MAN_*`, `MSC_*`, `ONL_FILT_MSG`), the end-race photo screen (`ER_PHOTO_*`,
already wired), Pure's confirm dialogs, HD's `BOOT_PRESS_X` (no reader) and
the PS2's controller-unplugged messages (not drawn). No `IG_HUD_*` caption
carries one, and a Pulse race frame under `--prompt-style xbox` shows none.

**Pulse PS2 shares Pulse's table but not its faces.** `Pulse_14.fnt` (21x21) and
`Pulse_20.fnt` (24x24) carry the six at their own sizes, and on this pressing `Λ Ν`
are lettered `L1` and `R1`; `pulse_text.fnt`, `PulseHud.fnt` and `small.fnt` carry
none. Its race frames under `--prompt-style xbox` and `original` are byte-identical.
Its menu face is outlined, and an outlined face takes opacity from the body mask
alone (`crates/game/shaders/ui.wesl`), so an added cell's mask has to follow its ink
(`Atlas::push_glyph`); a solid mask draws a filled box.
The HUD's own atlases are built without the stand-in cells
(`oag_raceplay::hud::hud_font`), so a HUD string that did carry one would
need `with_prompts` there and a fan-out on `hud_overlay::Overlay` first.

## Other titles

- **Pure**: `ε γ δ β Λ Ν` in `FX300ANG.fnt` and `LTe50325.fnt` are the same
  six pictures as Pulse's (14x17 face buttons, 25x17 and 26x17 shoulder
  boxes), **measured**, table in `oag_pure::prompts`, pinned by hash in
  `prompt_glyphs_ground_truth`. The Pure front end's own footer glyph is a
  sprite, not a stand-in, so the menus do not change under a forced family
  today; the table serves the strings that carry the codepoints
  (`Press ε to Continue`, `Press β to Cancel`).
- **Omega**: `PS_BUTTONS.fnt` is HD's face at twice the size: `ε γ δ β` are
  96x107 circled cross, circle, square and triangle, `Λ Ν` 168x109 boxes
  lettered `L2` and `R2`. **Measured**, table in `oag_omega::prompts`
  (`Scope::ButtonsFace`), pinned by hash. A forced `xbox` footer draws the `A`
  disc where the disc draws the cross (read as a picture).
- **HD**: the same probe found `β Λ Ν` in `PS_BUTTONS.fnt` beside `ε γ δ`
  (the table listed three); all six are now in `oag_hd::prompts`. `Λ Ν` are
  drawn as `L2`/`R2` and map to the shoulder stand-ins (**chosen**).
  Omega's six are HD's six at twice the size (**ported**).
- **2048**: **checked, differs.** None of its five language faces
  (`NEOSANS*.fnt`, `2048_hud.fnt`) holds a stand-in cell and there is no
  `PS_BUTTONS.fnt`: a touch front end with its own icons, `UNREAD`.
