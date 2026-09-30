# The `PosTag0`-`PosTag7` rows: the Eliminator's kill column

**Binary:** `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`.

`Elimination_HUD.xml` and `Arcade_HUD.xml` both author eight text rows,
`PosTag0` to `PosTag7`, with no `idstring` and no `string`
([hud.md](../../../ui/hud.md)). Their text is runtime-supplied. This page reads
who supplies it, in which modes, and what it says. Evidence: two decompiles, the
mode constructors that set the gating flags, and a live PPSSPP frame.

## Where the rows come from

`Hud_BindWidgets` (`0x0881fbec`) binds them: a loop over `i` in `0..8` formats
`"PosTag%d"` (`0x08a7a100`, the only reference to it), looks the widget up, stores
it at **`hud + 0x254 + 4*i`**, and sets its text to the empty string
(`0x08a79c38` is a NUL). It does so when `hud+0x40 & 0x20 == 0`, the same flag
gating the clock cluster's bind. So every row of every non-Zone layout exists and
starts empty.

`hud+0x40` is the mode word the mode constructors OR their features into. The
`search_instructions` scan over the binary for the three bits that matter:

| Bit | Set by | Read by |
| --- | --- | --- |
| `0x40` | `ArcadeRace_Construct` (`0x0882c29c`), `Tournament_Construct`, `MPRace`, `MPTimeTrial`, `MPTournament` | `FUN_0881d458`: the place readout |
| `0x200` | `Elimination_Construct` (`0x0882cb18`), `MPElimination_Construct` | `FUN_0881af38`: the kill column |
| `0x800` | `MPRace_Construct` (`0x088220e8` region), `MPTournament_Construct` | `FUN_0881d458`: the multiplayer place list |

`Hud_Update` (`0x0881bf50`) calls `FUN_0881d458` on `0x40`, and
`FUN_0881af38` on `0x200` **only when `hud+0x120` is set** (a "dirty" byte it
clears after the call), so the column is rewritten when something changed rather
than every tick.

## `FUN_0881af38` (`0x0881af38`): the kill column, confidence 85

Renamed `Hud_UpdateKillColumn`. Reads the grid (`FUN_08826d80` copies
`manager+0x78+4n`, `n < DAT_08b30f90`, so **grid-array order**) and for each craft:

```c
rank = (count - 1) - #{ j : craft_j->0x8d8 < craft_k->0x8d8 };   // 0x8d8 = kills
while (row_taken[rank]) rank -= 1;                                 // move up
row_taken[rank] = true;
widget = *(hud + 0x254 + 4*rank);
```

so rows are ordered by kills, most first, and among equal kills **the craft later
in the grid array is higher**. The row text, for the solo modes (`DAT_08b30f90+0xb8 < 14`):

- the player (`craft+0x48 == 0`): `"%s  %d"` (`0x08a79ccc`, two spaces) of
  the profile's tag (`DAT_08b31774 + 0x457`) and the kills; scale `1.0`;
  widget flag `+0x2c |= 0x200`.
- an opponent: `"%s %d"` (`0x08a79cc4`) of the team's display name
  (`FUN_088938ec(DAT_08ab1160, craft+0x798, 0)`, a string-table lookup of the
  team id) and the kills; scale `0.8` (`0x3f4ccccd`), `+0x2c &= ~0x200`.

The scale is written to widget `+0xa4` and `+0xa8`, the pair `Hud_BindWidgets` writes
`0x3f4ccccd` to on `PositionOf` as Head2Head's second row's scale (`head_to_head.rs`'s
`SECOND_ROW_SCALE`); a crop of the live frame shows the player's row taller than its
neighbours at the same brightness. An earlier revision of this page read them as alpha. Modes `>= 14` (multiplayer)
take the name from `FUN_08966848(DAT_08b32d40, craft+0x364)` instead.

## `FUN_0881d458` (`0x0881d458`): the place readout, confidence 78

Renamed `Hud_UpdatePositionCluster`. Three shapes, by `hud+0x40` and the mode at `hud+0x284`:

- **`hud+0x40 & 0x800` (multiplayer):** the place list in the same rows. Row
  `i < PLAYER_HUD+0x18` is visible (`+0x2c |= 4`) and reads `"<place digit> <name>"`
  from the race manager's place array (`manager+0x98+4i`), the player's own row
  taking the profile tag at full size and the flag `0x200`, the others the team
  name at scale `0.8`.
- **mode 9 or 15 (Head2Head):** the two ordinal rows and the gap label,
  [head2head.md](head2head.md).
- **otherwise (a solo race):** the two digits of `PLAYER_HUD+0x10` and `+0x18` into
  the `Position` and `PositionOf` widgets, `+0x240`/`+0x244`. **Nothing is written
  to any `PosTag`.**

## The `KillsText` header

`Hud_BindWidgets` formats `KillsText` once, at bind time, as `"%s (%d)"`
(`0x08a7a0c8`) of the `IG_HUD_KILLS` string and the kill target (a campaign
cell's gold, else the race box's kill row). See
[eliminator-kill-target.md](eliminator-kill-target.md).

## Measured live, 2026-09-30

Own PPSSPP, `pulse-psp-usa.chd`, Venom Eliminator on Talon's Junction:
`g_game_mode` `8`, `hud+0x40` **`0x229f`** (`0x200` set; `0x40` and `0x800`
clear), `hud+0x284` `8`. The frame reads `KILLS (5)` over
`AG Systems 1`, `Feisar 1`, `Qirex 1`, `AAA 0`, `Triakis 0`, `Goteki 45 0`,
`Piranha 0`, `EG-X 0`, top to bottom, right-aligned at `x=460`, `y` stepping by
20, in the default face (`pulse_text.fnt`); the player's row is the larger one.
That is the sort rule above (three craft on one kill above five on none, and the
zero-kill craft in descending array order), with the profile tag `AAA` typed at
the first-boot name entry. A solo single race reads `pos 8/8` and no rows.

## What this build does with it

[`oag_game::hud::kill_tags`](../../../../crates/game/src/hud/kill_tags.rs):
`ranked` is the row rule, `Readout::kill_tags` carries the rows, and `draw.rs`
puts them in a new `Default`-face text bucket (`Frame::default_text`, a third
`Renderer` in `Overlay`, the face loaded only for a layout that names it).
`KillsText` now reads `KILLS (5)`. **Pulse only**
(`oag_title::HudArt::kill_column`): Wipeout HD authors a `KillsText` and six `PosTag`
slots of its own and nothing has measured what it writes in them.

**Chosen, not measured:** this build has no profile tag, so the player's row
carries the player's team's display name; the grid array's order is this
build's own, so the tie-break can differ from a given original race; and the
`0x200` flag on the player's row, which Head2Head's player row also carries and
which throbs on the original, is not drawn. The multiplayer place list is not a
mode this build runs.
