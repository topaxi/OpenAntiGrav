# The race HUD's message lines: `Info1` to `Info4`

**Binary:** `pulse-psp-usa` `BOOT.BIN`, image base `0x08804000`. Read in Ghidra
2026-10-06 for the "does the original show a medal mid-race" question
(`hud-medal` lane). **Measured negative: it does not**, by static census, with
the live-frame half open (see the end).

## The two functions, confidence 80

| Address | Name | What it is |
| --- | --- | --- |
| `0x0881b52c` | `Hud_PushMessage` | `(hud, string_id, good)`: `strlen`, find the first of four slots whose busy byte (`hud+0x130+16*i`) is clear, store the id, set the slot's timer to `100.0` (a "waiting" sentinel) and its good byte (`hud+0x131`) to `good`. All four busy: dropped. |
| `0x0881f148` | `Hud_UpdateMessages` | Called once a tick from `Hud_Update` (`0x0881c5ec`). Drains the flag word `*(*(hud+0x3c)+0xe0)` into pushes, then runs the four slots. |

`Hud_BindWidgets` (`0x08820ab8`) binds the four slot widgets: `Info1` to
`hud+0x134`, `Info2` to `+0x144`, `Info3` to `+0x154`, `Info4` to `+0x164`. The
layouts author them at `x=240`, `y=80/95/110/125`, `font="HUD"`, `scale=0.6`,
centred (`TimeTrial_HUD.xml`, and `Zone_HUD.xml` the same; Speed Lap reads the
Time Trial layout; the other layouts and Pure's three were not all
counted line by line). `docs/ui/hud.md` had them as "debug, inferred from
the names, confidence 40".

## The flag bits - and no medal among them

| Bit | String id | Good byte |
| --- | --- | --- |
| `0x001` | `IG_HUD_PLAP` | 1 |
| `0x002` | `IG_HUD_NEWLAP_REC` | 1 |
| `0x004` | `IG_HUD_FIN_LAP` | 1 |
| `0x008` | `IG_HUD_CONT_ELIM` | 1 |
| `0x010` | `IG_HUD_CONT_ELIM` | 0 |
| `0x020` | `IG_HUD_PERF_ZONE` | 1 |
| `0x040` | `IG_HUD_PERF_BOOST` | 1 |
| `0x080` | `IG_HUD_NEW_ZONE_RECORD` | 1 |
| `0x100` | `IG_HUD_NEW_SCORE_RECORD` | 1 |
| `0x200` | pops a string off a queue at `DAT_08b32d40` | queue's own |

That is **the whole list of things the original raises in these lines, and a
medal is not one of them.** `0x200` is a generic queue; its pushers were not
read (the xref set on `DAT_08b32d40` is the whole HUD's, ~80 sites), so a
medal arriving by that route is not excluded by this page alone - the census
below is what excludes it.

## The slot law (decompiled; not yet seen on a live frame)

- A slot whose timer is `100.0` waits until the shared gate (`param_2+0x124`)
  is below zero, then shows: it plays the `MESSAGE` cue
  (`Sound_PlayNamedInSlot`, `0x400`), resolves the id through the language
  table (`FUN_088938ec`, falling back to the id), sets the gate to `0.8`
  (`0x3f4ccccd`) and the timer to `4.0` (`8.0` when `g_game_mode >= 0xe`).
- The gate counts down by the frame time only while it is zero or above.
- Each tick a shown slot sets its alpha to `sin(pi * (1 - (t/life)^6))` (times
  255), `t` the time left: full in the first half second, then a slow fade.
- Colour `0x30ff30` (green) when the good byte is set, else `0xff3030` (red);
  the border colour's alpha follows (`FUN_088cd3a4`).
- At `t < 0` the slot clears.

`oag_hud::messages` implements exactly this, and nothing of the triggers.

## Why there is no medal here: the census, confidence 85

- `Cell_EvaluateMedal` (`0x088bf620`) is called from `Race_RecordResult` (six
  sites, `0x0880afe4`-`0x0880b32c`) and from `Cell_BestMedal` (`0x088bf600`).
  `Cell_BestMedal`'s callers are `Cell_MedalPoints`, `Grid_CountMedalsAtLeast`
  and two sites in `CellSelection_PopulateGrid` (`0x088d5ee4`, `0x088d600c`):
  the end-of-race record, and menus. **No race-HUD function reaches it.**
- The in-race medal-shaped readout is the one `hud.md` already closed: the live
  target tier `PlayerStatus_Update` computes for Time Trial and Speed Lap and
  `Hud_UpdateTimeCluster` shows as `IG_HUD_BRONZE`/`SILVER`/`GOLD`/`RECORD` in
  `TotalTimeTxt`. It is "the tier you are on pace for this lap", re-evaluated
  every tick against the cell's three targets - not an award, and in Speed Lap
  it resets each lap.
- `Zone_HUD.xml` authors no medal or target widget (`Zone`, `Score`, the
  `Zone_Bar_*` family and the four `Info` lines only), and the Zone mode code
  reads no cell target while the race runs (`zone-mode.md`: the target is
  "progression data").
- The medal phrases the disc does carry (`ER_GMA`/`ER_SMA`/`ER_BMA`) and the
  jingles (`gold_med`/`silver_med`/`bronze_med`) are the end-of-race screen's
  (`endrace-screens.md`).

## What this page does not close

A live PPSSPP capture of a campaign Zone or Speed Lap cell across a medal
threshold was **not obtained**: a fresh profile has only `grid0`'s centre cell
unlocked, and the Zone and Speed Lap cells are behind medals (a campaign walk
screenshot is in the lane's scratch, `data/scratch/hud-medal/shots/cells.png`).
The static census above is what stands in for it. To close it: unlock a Zone
cell on a PPSSPP profile (or write the cell record) and watch `hud+0xe0` and
the four slot busy bytes across the threshold; the prediction is that none
moves.

HD composes the same four `Info` widgets into its Zone, Speed Lap and Time
Trial layouts (`InfoTextParent`), checked 2026-10-06. Pure authors them in all
three layouts and ships
`HUD_Perfect Zone!` and `HUD_New Zone Record` strings beside them, so its
message system is the same shape. Its `speech_zone.bnk` carries
`zone_bronze`/`silver`/`gold` voice lines, which Pulse's bank lacks; that is
the one lead that a Pure Zone race *does* announce a medal, unread.
