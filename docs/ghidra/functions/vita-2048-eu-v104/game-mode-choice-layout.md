# `GameModeChoice_Screen`: the code overrides the file's tile positions

Functions in `eboot.elf` (WipEout 2048, Vita, `PCSF00007` patch v1.04,
`/2048/eboot-vita-2048-eu-v104.elf`), image base `0x81000000`. **No function is
renamed** (all `FUN_` names, read from a decompile, none traced live), so
nothing is added to `names.tsv`. Wired in `crates/ui/src/frontend/mode_grid.rs`
and `touch.rs`; the pair `2048/menu-game-mode` went from mean grey 36.2 to 6.4
(see "Result").

## The class

The type string `GameModeChoice_Screen` is at `0x8146a50c`; its registration
(`0x811252a0`) stores vtable `0x8150fbb4 + 8`. Slots read:

| Slot | Address | What | Confidence |
| --- | --- | --- | --- |
| 3 | `0x811241c2` (`FUN_811241c2`) | on-enter handler: hides, recolours, goes to `newFEShell` when already seen | 85 |
| 11 | `0x81123a26` (`FUN_81123a26`) | constructor: looks the six tiles and the tick up by widget name, stores them at `this+0xec..0x104` | 85 |
| - | `0x811237c0` (`FUN_811237c0`) | **the layout**: called as `(this, g_bDlc1Mounted \|\| g_bDlc2Mounted)` from the constructor; overwrites each tile's `x` (`+0xa8`) and `y` (`+0xac`) | 85 |

## The layout (`FUN_811237c0`)

Floats read from the hex immediates (`0x42be0000` = 95.0, `0x43a00000` = 320.0,
`0x433e0000` = 190.0). Widgets are found by name: `offline` (+0xec), `offlinehd`
(+0xf0), `offlinefury` (+0xf4), `multiplayer` (+0xf8), `adhoc` (+0xfc),
`crossplay` (+0x100), `confirmButton` (+0x104).

| Mounted | Top row | Bottom row |
| --- | --- | --- |
| neither | one row of four at `y=190`: `offline` 167, `multiplayer` 329, `adhoc` 491, `crossplay` 653 | none |
| one | `offline` x=320 and the mounted tile x=500, `y=95` | `multiplayer` 228, `adhoc` 410, `crossplay` 592, `y=320` |
| both | 228, 410, 592, `y=95` | 228, 410, 592, `y=320` |

The v1.04 `Definition.xml` authors the top row at `y=110`: never drawn. The
Vita3K capture's top row is at `y=95` (measured on the base frame, the pair
frame and two more). The five- and six-tile branches the function also holds
need a count the caller never produces and were not used.

## Hidden and recoloured (`FUN_811241c2`)

- `*(+0x104 -> +0x30) &= ~6` on every enter: **`confirmButton` (the tick) is
  hidden**, so a tap on a mode has to leave on its own (consistent with the
  2026-10-26 one-tap measurement). Confidence 85.
- The same mask clears `offlinehd` when `g_bDlc1Mounted` is false and
  `offlinefury` when `g_bDlc2Mounted` is false: **an unmounted campaign's tile
  is hidden, not greyed**. Confidence 85.
- `Grey2048` (`+0x110`) is written into `multiplayer` and `crossplay` (`+0xdc`,
  `+0xe4`) unless the online check passes (`FUN_81236fb6`, a read of
  `[obj+0x153]`, and two siblings); **`adhoc` is never touched, so it is always
  the enabled blue**. Confidence 80 (the check's exact meaning is not read).
- The tiles' own `+0xfd` (a "focused" byte) is zeroed at construction; the
  captures show no focus on any tile. Neither pad nor pointer focus is drawn
  on entry.

## Read versus chosen

- **Read:** positions, the hidden tick, the hidden unmounted tiles, which two
  tiles take `Grey2048`.
- **Chosen, not measured:** that this build is always "offline" (so
  `multiplayer` and `crossplay` are always grey); that "mounted" is read from
  the campaign maps the caller gave (`Frontend::has_campaign_tier`) rather than
  `g_bDlc1Mounted`/`g_bDlc2Mounted`; that ADHOC, drawn enabled as the original
  does, answers a tap with a note that this build has no ad hoc session; the
  pad ring (shown only after the pad moved, the original draws none); the
  RACEBOX, REMIX and MULTIPLAYER tiles (this build's own, left column).

## Result

`menu-game-mode` mean abs grey (PIL `L`), whole frame / left column / check
button / top row / bottom row (region means):

| | whole | left column | check button | top row | bottom row |
| --- | --- | --- | --- | --- | --- |
| before | 36.2 | 152 | 137 | 53 | 19.9 |
| after, compare shot (`--original-menu`) | 6.4 | 2.3 | 2.1 | 9.7 | 8.8 |
| after, as a player sees it (with our three tiles) | 19.6 | 152 | 2.1 | 9.7 | 8.8 |

The remaining top and bottom row difference is glyph shapes (the original's
labels are narrower and bolder than our `NEOSANS` stand-in scale) and the header
rule under the title; neither is wired here.

## Omega

Omega's `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`) carries `GameModeChoice_Screen`
(`0x01824d05`), `offlinefury` (`0x01824bf2`) and the three `mini_*campaign` icons,
so the class code ships. Omega's front end is HD's `<HorizMenu>`
(`crates/raceplay/tests/main_menu_ground_truth.rs`), so the screen is not
reached. **Checked, differs:** the layout law was not re-read on the PS4 code.
HD has no such screen; not applicable.
