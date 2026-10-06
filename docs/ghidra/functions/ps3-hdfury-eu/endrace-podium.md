# `EndRace Podium`: what its code fills in, and the entry nobody has found

2026-10-06, lane `hd-endrace-podium`. Static reading only (Ghidra bridge,
`/hdfury/EBOOT-ps3-hdfury-eu.elf`); no live RPCS3 run backs any row, so
nothing scores above 84. **No names are recovered into `names.tsv`**: the
functions below are unnamed here (their decompile is readable but the
identity of several fields is not), and a guess dressed as a name is worse
than `FUN_`.

## What the screen is

`EndRacePodium_Screen.cpp` (`0x00793ec0`) registers the type `EndRace Podium`
(`0x00793ee0`) through `FUN_0021f110` (a static init/destroy pair, `param_2 ==
0xffff`). Its widgets are the authored `pod_head.N`/`pod_img.N`/`pod_text.N`/
`pod_dots.N` (`0x00793f40`..`0x00793f80`, `%d` formats) and the sounds
`~podiumticker`/`podiumtickend`. It lives in `DATA05`/`DATA06` only and
authors **no** `NavigationController` and no `Redirect` except the network
exception handler (`goto="InGame Exception Pause"`).

## The slot setter, `FUN_00220108` (confidence 78)

Called seven times by one function, `FUN_00220820`, as
`setter(screen, slot, name_string, place, ship, is_local_player)`:

- `place == 0` hides `pod_head.<slot>`; otherwise the slot is built.
- `pod_text.<slot>` gets the name string and sits at
  `x = 0x46 + slot * 0x168` (70 + 360 slot), `y = 0x214 - 0x50 * (4 - place)`
  (532 - 80 (4 - place)).
- The place picks one of three string pointers (`+0x2f6c`/`0x2f70`/`0x2f74`
  off the TOC) for the heading - first/second/third. Which idstrings they
  are is **unread**; `IG_HUD_1ST`/`2ND`/`3RD` all exist in HD's English table,
  and the authored `pod_head` idstrings are all `IG_HUD_1ST`, so the code is
  what overwrites them.
- The local player's name is coloured with the screen's `HD_Blue` slot
  (`+0x228`), anyone else `0xff646464`.
- A ship texture is looked up per record and applied to `pod_img.<slot>`.

## The caller, `FUN_00220820` (confidence 76)

It gets the race manager (`RaceManager_GetInstance`, records at `+0x1980`,
stride `300`) and fills three slots:

- first place goes to **slot 2**, second to slot 1, third to slot 3, so the
  winner stands in the middle: slot 2's `x` is `790`, the authored
  `pod_head.2` `x="795"` that pins the 360 step.
- Two branches. When `g_GameState+0xe0 == 0x11` (and a byte at `DAT_008b0458`
  is zero) the places come from a rank pass over the records (ties share a
  place: the loop at `iVar16 + n*300 + 0xac8`). In every other case the
  places are literally `1`, `2`, `3` over the first three records. Neither
  branch asks whether the race is online. What mode `0x11` is, is **unread**.
- Third place is skipped (`setter(..., 3, ..., 0, 0, 0)`, which hides it) when
  the field has two or fewer craft.
- Then eight `FUN_0021f9e8` calls fill the `b_b.N` badge panels from a list
  (`FUN_0004d7f8`), a per-player achievement list: badge, player name.

## Not found: who enters the screen

No `goto=` in any of the seven archives names it (the same shape as
`EndRace Rewards`), and the one data reference to its name string
(`0x0077b770`, from `0x008a6394`) sits in a run of string pointers beside
`EndRace Results` and `Race End Alone`, not a code site. The type's only code
xrefs are the registrar above. What would enter it is therefore code naming
the screen by a path this pass could not follow. **Open**, at the entry:
find who pushes the screen by name/type, and under which condition.

## What this project does with it

`oag_ui_screens::endrace::hd::hd_podium_draw_list` draws the three places at
the measured positions off the copy that authors the screen, reached only
by `--menu-page endrace-podium`. The live flow stays Results -> Menu
because the original's entry is unread. See
[hd-endrace-screens.md](../../../formats/hd-endrace-screens.md#endrace-podium-read-and-drawn-entry-untraced).
