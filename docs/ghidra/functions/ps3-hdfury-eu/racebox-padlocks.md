# What the padlocks on HD's race box are

Wipeout HD / Fury PS3 EU (`/hdfury/EBOOT-ps3-hdfury-eu.elf`), 2026-10-13, lane `hd-authored-split-setup`. Static
reading through the Ghidra bridge (decompile and xrefs, no live run). One name recovered:
`RaceBox_UpdateTrialPadlocks`.

**The question.** `racebox_definition.xml`'s `Single Player` screen authors a `*Padlock` image beside the
`Mode`, `Class`, `Weapons`, `Difficulty` and `NumberOfPlayers` lists (`ModePadlock` ... `NumPlayersPadlock`) and
a big `Padlock` over the preview icon ([hd-frontend.md](../../../formats/hd-frontend.md), "RACEBOX authors local
split screen"). Is the padlock on `NumberOfPlayers = 2` a progression unlock, a trial restriction or something
else? Everything below is about that one row; the rest of the routine is read only as far as it shares the rule.

**Answer, confidence 72: a trial-build restriction, not a progression unlock.** The padlock logic runs only
while the content flag `g_FuryContentFlags` (`0x00b979fc`, a byte; the label is already in the database, it is
not recovered here) is non-zero, and that flag is the one the front end reads to decide whether to draw the
trial's own `TrialText` and the `PurchaseGameRedirect` (`FUN_0016cb40`, `0x0016cb40`). With it zero - the full
game - the routine returns before any padlock is shown or any entry refused, so **two players are available from
the first launch**, with no save-game input. This build races a full game, so it offers two players unlocked.

## The routine

`RaceBox_UpdateTrialPadlocks` (`0x002497d8`, `.opd` `FUN_002497d8`; TOC `r2 = 0x008ad4d8`, below `0x260000`
where the TOC needs no correction, [memory.md](memory.md)) is the `Single Player` screen's per-frame update. It is
the only function that loads the string `NumPlayersPadlock` (`0x00797ee0`): its TOC slot `0x008b1790` is read by
`lwz r4, 0x42b8(r2)` at `0x00249c44`, and `NumberOfPlayers` (`0x00797dd0`, slot `0x008b1748`) is the screen's own
widget table. Reading it:

1. Each padlock widget is looked up by name (`FUN_0016a920(screen, name, 0)` for the six `*Padlock` slots
   `0x42a4`..`0x42bc` off `r2`) and its flag word at `+0x34` has bit `4` (visible) cleared. So **every padlock
   starts hidden every frame**.
2. `if (**(char **)(r2 + 0x41b0) == 0) goto end;` - the slot at `0x008b1688` holds `0x00b979fc`, the byte
   `g_FuryContentFlags`. Zero means nothing below runs and no padlock is ever shown.
3. Otherwise the routine compares each list's current string against a short table of strings in the binary
   (the `Mode` list against `Tournament` - which the screen shows as ELIMINATOR, see
   [zone-effectsettings-loader.md](zone-effectsettings-loader.md) - and two more mode strings, the `Class` list
   against its entries, the `Weapons` list against `FE_ON`, the `Difficulty` list against `MSC_NONE`) and sets
   bit `4` on the matching padlock when the entry is one the trial locks.
4. **The `NumberOfPlayers` row**: `FUN_00676958(list.value, 0, 10)` parses the list's current string as a
   base-10 integer, and when it is `2` the `NumPlayersPadlock` gets bit `4` set and the screen counts as locked
   (`bVar3`). A locked screen shows the big `Padlock` over the icon and clears a bit of the confirm control
   (`FUN_001da6d0`), which is what stops START.

## Why the flag is the trial, not progress

`FUN_0016cb40` (`0x0016cb40`) reads the same byte (`PTR_g_FuryContentFlags_008aba4c`, slot `0x008aba4c` also holds
`0x00b979fc`) and, when it is non-zero and the current screen is not one of `ProductInfoScreen` and three siblings,
shows `FE_Screen->theTriangleButton`, `ControlTextTriangle`, `PurchaseGameRedirect` and `TrialText`. Those are the
trial's upsell widgets, and `UpgradeToFullVersionCheckout_Screen.cpp`, `demo_definition.xml` and the main menu's
`<Item name="TrialItems">` say the same ([hd-frontend.md](../../../formats/hd-frontend.md), the boot-chain
section). Nothing in the routine reads a profile, a medal or a progression counter: the lock is a function of the
flag and of the list's own current string.

## What is not read

- **Who sets the byte.** Its only xref marked `WRITE` is inside a library routine (`FUN_00631840`, `0x00631840`,
  an unrelated function that happens to store through a nearby address); the rest are `PARAM` loads of the address.
  The reading "non-zero is the trial" rests on the two routines above and on the upsell widgets, not on a located
  writer. A retail disc and the PSN full game are both expected to hold zero; the trial, which this project has
  no image of, would hold non-zero.
- **The strings the other padlocks compare against** are named only as far as the three above; which of the
  five modes and four classes the trial locks was not enumerated.
- **The `intvalue`s of the lists are not unlock ids.** `racebox_definition.xml` says it itself in the comment
  above the lists: `intvalue` "is indicative of the index in the array of images to display adjacent to the
  list". They number the preview images across every row (`Mode` 0-4 and 28-30, `Class` 5-8, `Weapons` 9-10,
  `Difficulty` 11-13, `NumberOfPlayers` 14-15, `SplitScreen` 18-19) and pick what the `BigIcon` shows. An earlier
  reading of 14 and 15 here as unlock ids was wrong.

## This build

Two players are offered unlocked and no padlock is drawn on the row. That is the full game's behaviour read
above (confidence 72), not a project choice; the trial's lock is not modelled because no trial build is in the
toolchain. The `Choice::unavailable` mechanism (`oag_ui::menu`) is where a lock would go if one is ever measured.
