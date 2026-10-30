# Pure's pickup icons: authored colour, shown and hidden, never recoloured

Live read of Pure USA (`pure-psp-usa.chd`, `UCUS98612_2.00`, PPSSPP 1.20.4 software, the saved
`time-trial-vineta-vector-feisar-restart` state) on 2026-10-30, lane `pickup-fade-titles`. Addresses
are the USA binary's; the EU twin (`/pure/BOOT-psp-pure-eu.BIN`) was read for strings and the update.

| Address (USA) | Name | Confidence |
| --- | --- | ---: |
| `0x08821838` | `Hud_BindWidgets` (EU `0x0882166c`) | 75 |

`Hud_BindWidgets` looks up `HUD->ROCKET_icon` ... `HUD->BOMB_icon` (strings `0x08a46468`.., table of 11
name pointers at `0x08a8ca40`, slot 0 empty) and `weapon_icon_grid`, storing the widgets at
`hud+0x124 + 4*i` and `hud+0x150`. There is **no `PickupBackground` string in either Pure binary** (`search_strings` on `/pure/BOOT-psp-pure-usa.BIN` and `-eu.BIN`, 0 matches; the
brief's question has the answer "none"); there is no backdrop widget to colour.

## What the widgets hold

At the binder's call site (`0x0881e6d4`, `a0` = the HUD, `0x0975DD50` in the run) the ten icon widgets
(index = the row order Rocket 1, Missile 2, Quake 3, Disruptor 4, Turbo 5, Shield 6, Autopilot 7,
Plasma 8, Mine 9, Bomb 10) hold at `widget+0xbc` the layout's colour: `0xFF40ACFF` (1, 2, 4),
`0xFF0000FF` (3, 8), `0xFF40FF40` (5, 6, 7), `0xFFFFC040` (9, 10), and `0xFFFFFFFF` at `+0x38`, `+0x118`
and `+0x1A4`. Exactly the `colour=` words of `Arcade_HUD.xml` that `crates/pure/src/hud.rs` documents.
The `weapon_icon_grid` widget reads `0xFFFFFFFF`.

## What a grant changes

A halting breakpoint on the per-frame HUD update `FUN_0892195c` (EU `0x08921248`, `PlayerStatus_Update`'s
twin, whose EU jump table at `0x08a74a70` maps the held id `pickup+0x1d8` to the icon index `status+0x90`),
with `pickup+0x1d8` poked to 4, 5 and 6, diffing the first `0x200` bytes of all ten icon widgets and
the grid every update for 100 updates:

- the new icon widget's flag word `+0x10c` goes `0x0105B026 -> 0x0105F026` (a bit set: visible), and
  the previous one's `0x0105F026 -> 0x0105F022` nine updates later (the poke is at update 5, the show at 6-7, the hide at 15);
- the new widget's scale words `+0x120`, `+0x134`, `+0x148` take `0x3F7FFD84` (0.99996) once;
- **`+0xbc` and every other colour word never change** in 100 updates, in any of the three runs.

Pure draws its icons as the layout authors them, with a visibility flag. **No per-weapon table, no
runtime tint and no fade of the colour.** Confidence **75**: three grants of three weapons on one
circuit, the first `0x200` bytes of each widget; a writer outside that window or a different mode was
not seen. A pickup grant by pad and a use were not reproduced (the id was poked); the `+0x10c` bits are
the show/hide, unmapped past that.

Cross-check against Pulse ([pickup-colour.md](../psp-pulse-usa/pickup-colour.md)): Pulse recolours one
shared backdrop from a 13-word table with an eased fade; Pure shows ten icons in their own authored
colours. **checked, differs.** What to wire on Pure: nothing new (`pickup_backdrop_colour:
Some("HudBGColour")` in `crates/pure/src/hud.rs` is moot, there is no backdrop).
