# Which sounds Pulse's front end plays, and when

Pulse PSP (USA) `BOOT.BIN` (`/pulse/BOOT-psp-pulse-usa.BIN`, image base `0x08804000`), 2026-10-08, lane
`pulse-menu-sfx`. Headless-equivalent reading through the Ghidra bridge (decompile and call-site scan, no live run).
One name recovered: `SoundManager_LoadBootBanks`.

## The path every menu sound takes

`Sound_PlayNamedInSlot` (`0x0893a768`, already named) reads a **bank slot** out of the sound manager
(`*(mgr + slot*4 + 0x124)`), stores two extra bytes into a 32-entry table (`FUN_0898dac0`, below) and calls
`Scream_PlaySoundByName(bank, 0, name, 0x400, ...)`. `0x400` is unity pitch and there is no emitter: every menu
sound is dry. A scan of its 79 call sites (decompiling each caller; list in this lane's scratch,
`play_sites.txt`) gives every cue the front end names and the slot it plays it from.

`SoundManager_LoadBootBanks` (`0x0893aba4`, called from the sound manager's constructor `FUN_08939fe0`) fills the
slots, in this order:

| Global | Bank | Hash entry (Pulse USA `Data.wad`) | Label |
| --- | --- | --- | --- |
| `DAT_08ac1de4` | `Data\Sound\frontend.bnk` | `#856 75a91641` | `FRNTEND` |
| `DAT_08ac1de8` | `Data\Sound\FRONT_END_VO.bnk` | `#857 a3aada5f` | none |
| `DAT_08ac1e00` | `Data\Sound\speech_results.bnk` | `#864 7c778043` | none |
| `DAT_08ac1dec` | `Data\Sound\hud.bnk` | `#859` | `HUD` |
| `DAT_08ac1df0` | `Data\Sound\generaltrack.bnk` | `#858` | `gentrak` |

Confidence **80**: the decompile is five `FUN_0893a654(mgr, path)` calls storing their result in five globals, and
every call site below passes one of them. The other slot globals (`1df4`, `1df8`, `1dfc`, `1e04`) are loaded elsewhere
and are race banks (`weapons`, `ship`, `speech`, ...).

## `frontend.bnk`: six cues, five of them played

`oag-wad sounds 'data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/Data.wad' --bank FRNTEND`:

| Cue | Waveforms (Pulse) | Waveforms (Pure) | Played by |
| --- | ---: | ---: | --- |
| `UPDOWN` | 3 | 1 | the cursor moving to another cell or row |
| `LEFTRIGHT` | 1 | 1 | a value stepping, and the grid controller's left/right |
| `ACCEPT` | 1 | 1 | a confirm that went through |
| `DECLINE` | 3 | 1 | back, a refused confirm, a move onto an empty cell |
| `TELETYPE` | 1 | 1 | a `Menu` row revealing with its cursor on; **no screen reaches it** (see `TELETYPE`) |
| `PAUSE` | 1 | 1 | **nothing recovered** |

Confidence **88** for the four navigation cues (several independent call sites each, strings in the pointer pool at
`0x08a7f068`, `0x08a7f074`, `0x08a81898`, `0x08a818a4`, ...). The mapping of event to cue, from the callers:

| Event | Cue | Where |
| --- | --- | --- |
| Cursor moves to a live cell | `UPDOWN` | `FUN_088a3b1c` (grid move, the `else` of the empty-cell test), `FUN_088a4e60`, `FUN_088a9db4`/`FUN_088a9e7c` (previous/next), `FUN_088b528c`, `FUN_088cae88`, `GridSelection_Update`, `TrackSelection_Update` |
| **Cursor move onto an empty cell** | `DECLINE`, *instead of* `UPDOWN` | `FUN_088a3b1c`, `FUN_088a3e8c` |
| Value stepped left or right | `LEFTRIGHT` | `FUN_088aaae4` (the grid controller's two buttons), `FUN_088cae88`, `FUN_088e4fd0` |
| Confirm resolved (the redirect evaluated non-zero) | `ACCEPT` | `ConfirmButton_Update` (`0x088c8e88`), `FUN_088ed05c` |
| Confirm refused (the button's own check `FUN_088b4fc8() == 0`, or the confirm's target not found) | `DECLINE` | `ConfirmButton_Update` |
| Secondary button (back) | `DECLINE` | `ConfirmButton_Update`, `GridSelection_Update`, `FUN_088cae88`, `FUN_088e4fd0`, `FUN_088ed05c` |
| Page transition by itself | **no sound** | see below |

`ConfirmButton_Update` skips `ACCEPT` and the refusal's `DECLINE` when the widget's byte `+0x269` is set (the XML
`Silent` property sits in the same string pool, `0x08a81860`); the back button's `DECLINE` does not read it. Which
widgets set it was not swept (open).

**A page transition makes no sound of its own.** `FrontEnd_TransitionTo` (`0x088e24fc`) calls `FUN_08965630`, which
the earlier `state-machine.md` called a front-end sound. It is three stores: `FUN_08965628(p, 0)` writes `*p = 0`
and `*(p + 4) = 0` on the singleton `FUN_0896565c` returns (a 0x140-byte object allocated on first use). Neither
reaches `Sound_*` or `Scream_*`. The cue rides the button that caused the transition. Confidence 85.

## `TELETYPE`

Corrected 2026-10-11 (lane `pulse-teletype`). The earlier reading here, "once per revealed character", was wrong, and
the cue is **never reached by any screen the shipped front end shows**. Nothing is wired, and that is the finding.

**The caller is `Menu_PlayTeletypeTick` (`FUN_088b6514`, `0x088b6514`), the second virtual slot of the `Menu` and
`MenuBitmap` widget classes** (vtables `0x08acd75c` and `0x08acd844`, slot at `+0x08` of the pair table; the class names
are the registered strings `Menu` at `0x08a8029c` and `MenuBitmap` at `0x08a8043c`). It indexes the menu's *entries*
(`<Entry IDString/String/Value>` children, `FUN_088b5a60`), not characters: while `+0xd0` (reveal complete) is 0, `+0xcc`
(revealed rows) is below the row count and `+0xf0` (cursor enabled) is set, it calls
`Sound_PlayNamedInSlot(slot, bank, "TELETYPE", 0x400, pitch, 0, 0)` at `0x088b669c` (pitch in `t0`) and parks the `_`
cursor widget (`+0xe0`, the string at `0x08a80138`) at the end of row `+0xcc`. The pitch is the formula below from the
row's measured text width `FUN_08919960(FUN_088cd21c(row text))`: `(width / 100 - 1) * 0.2 * 90`, `+360` when negative,
truncated to an integer. The reveal state machine is `FUN_088b6088`: `+0xcc` advances once per update through
`FUN_088883d4`, which is `jal FUN_088885ec; li v0, 1` and so **always reports the row done**, so the reveal-wait branch
(`FUN_08888418`, the `+0x100` width test) is dead code on this build. A press of input 0 completes the reveal
(`FUN_088b521c`); input 1 steps the selection and completes it (`FUN_088b528c`, which is also where `UPDOWN` plays).

**What `MenuDisplayType` does** (`FUN_088b5a60`, the `Values` element's attribute): `NORMAL` -> mode 3 and **clears
`+0xf0`**; `REFRESH_NORMAL` -> mode 2 (completes on the first update); `SCROLLING` -> mode 0; `TELETYPE` -> mode 1. The
constructor `FUN_088b49a4` defaults to mode 3 with `+0xf0 = 1`.

**Runtime check (PPSSPP v1.20.4, software, cold boot, USA).** A log-only breakpoint on the `jal` at `0x088b669c` and a
second on `0x088b6514` itself, armed through the whole first boot and then the walk Boot, `Main Menu`, `Single Player`,
`Racebox` (Time Trial, track select) into the race: **0 hits at the `jal`** against 212 hits at the entry. At the
entry every Menu object read mode `+0xdc = 3` and `+0xf0 = 0`; the main menu's object revealed its seven rows one per
frame (`+0xcc` 1..7 over seven consecutive updates) with `+0xf0 = 0`, so it is silent and the rows appear within seven
frames (about 0.12 s at 60 Hz), not as a visible type-in. Confidence that no screen on that walk plays `TELETYPE`: 85.

**Authored XML.** Across every entry of `Data.wad`, `FE.wad` and `FEData.wad` (USA), `MenuDisplayType` is authored 21
times: 19 `REFRESH_NORMAL` and 2 `NORMAL`; there is **no `TELETYPE` and no `SCROLLING`**. An unauthored Menu keeps the
constructor's `+0xf0 = 1` and mode 3, which is the one path that could tick; the walk above met none, and the
other screens were not walked (open: a Menu with no `MenuDisplayType` on an unvisited page).

**Other titles (checked 2026-10-11).** Pulse PS2: same `frontend.bnk` cue set (psp-audio.md), but its executable was
not read: not checkable here. HD/Fury: checked, differs - its bank has `TextBox01`..`TextBox04` in place of any
`TELETYPE`, trigger unread (`ps3-hdfury-eu/menu-sounds.md`). Omega: checked, applies, not wired - HD's front end, no SFX
path. Pure: `TELETYPE` is in the bank, its caller not read.

**The other `TELETYPE` is a different widget, and it does not type in.** `DisplayType="TELETYPE"` on a `<Text>` widget
(`<a ... DisplayType="TELETYPE">`, the attribute is tokenised to one letter in some `.cod` files, `o="TELETYPE"` in
`01092`) is authored on button prompts (`FE_CONFIRM_BUTTON`, `FE_CONFIRM`, `FE_BACK`), screen titles (`PRO_STATS`,
`FE_ONLINE`) and `BOOT_PRESS_ANY2`, in 13 of the `.cod` files (58 widgets). Read 2026-10-11 (lane `pulse-typein`,
[text-display-type.md](text-display-type.md)): the `Text` class keeps it as one flag bit that nothing acts on, its
draw prints the whole string, and the one update path that reads the bit calls an empty function. The widget draws
whole on its first frame, confirmed live on PPSSPP. The earlier wording here, that the original types these in
silently, was an inference and is withdrawn.

## `PAUSE`

No code reference. A byte search of `BOOT.BIN` for `PAUSE\0` has no hit, so the executable never spells it; the
pause menu (`InGame Pause`, `0x08a795ec`) is driven by the same `ConfirmButton_Update` cues as any other. Whether a
formatted name could reach it was not ruled out (confidence 60 that nothing plays it).

## Text entry, dialogs and the boot (2026-10-16, lane `pulse-frontend-sfx`)

**Method.** PPSSPP v1.20.4 (software renderer, USA), a breakpoint on `Sound_PlayNamedInSlot` (`0x0893a768`), and at each
hit the front end's state name, the cue name (`a2`) and `ra`, with scripted presses in between
(`scripts/psp-menu-cues.py`). Every cue below is a hit, not a reading. Only the most recent breakpoint fires on this
build, so the probe arms exactly one.

### `TagInput`, the text-entry widget: `TagInput_Update` (`0x088cae88`), confidence 88

The widget class is registered by `FUN_088cbcbc` (it passes the string `TagInput`, `0x08a81b88`, and installs vtable
`0x08acf2d4`); the vtable's fourth slot is `0x088cae88`, which holds the cell cursor (`+0xe8`), the cell count
(`+0x110`) and the per-cell glyphs. The button numbers are the abstract layer's (`input.md`: 0 up, 1 down, 2 left,
3 right, 4 circle, 5 cross). The decompile and the live walk agree on every row:

| Press | Condition | Cue | Live (`ra`) |
| --- | --- | --- | --- |
| Up or Down | cursor on a cell (not the confirm slot) | `UPDOWN`, glyph steps | `0x088cb0b4`, `0x088cb144` |
| Up or Down | cursor on the confirm slot | silent | none in 1.5 s |
| Left | cursor above cell 0 | `LEFTRIGHT`, cursor steps | `0x088cb1a4` |
| Left | cursor on cell 0 | `DECLINE` | `0x088cb1dc` |
| Right or Cross | the cursor can advance (cell to cell, last cell to the confirm slot) | `LEFTRIGHT` | `0x088cb2d0` |
| Right or Cross | cursor already on the confirm slot | `DECLINE` | `0x088cb2fc` |
| Circle | cursor above cell 0 | `LEFTRIGHT`, cursor steps back | `0x088cb348` |
| Circle | cursor on cell 0 | silent (no back) | none |
| Square | any | silent | none |

Holding **Up or Down** repeats, each repeat with `UPDOWN` (the decompile reads `Input_IsHeld` for those two only: first
repeat after 10 frames, then every 2); Left, Right, Cross and Circle are presses only. **Cross on the confirm slot plays
two cues on the same press, within a frame or two of each other** (the debugger's clock cannot say which): the widget's own advance has nowhere to go (`DECLINE`) and the sibling
`ConfirmButton_Update` then accepts (`ACCEPT`), measured on `NameSetup2FromBoot` and `TagSetup2FromBoot` (a 10 cell
name and a 3 cell tag). A blank name did not stop the cursor reaching the slot. The decompile also reads a flag
(`+0x129`) and a count (`FUN_088ca80c`) that cap the advance one short of the slot; which field sets them (the
`AllowBlank` attribute is the candidate) was not measured, so a field that refuses a blank advance is open.

**What the port does with it** (`oag_ui_screens::tag_entry::TagEntry`, pad and pointer): the cues are the original's,
on the port's own bindings, which are not the original's. The port scrolls on Up/Down and steps on Left/Right as the
original does; but its Cross on a cell does nothing (the original advances), its Circle cancels the prompt (the
original steps back and has no cancel on the widget), and its Square deletes (the original ignores it). Those are
**chosen, not measured** and unchanged by this lane; their cues are `LEFTRIGHT` for a delete or a typed character and
`DECLINE` for a cancel. The pointer plays the pad's cues for the same move: hovering onto a cell steps the cursor
(`LEFTRIGHT`), the wheel scrolls the glyph (`UPDOWN`), a click on the confirm slot is Cross on it (`DECLINE` then
`ACCEPT`), the secondary button is a cancel (`DECLINE`). Walked live on the PPSSPP-independent port run (xdotool, Xvfb,
Pulse USA, `--no-audio`): hover onto cell 3 logged `MenuStepRight`, wheel up and down `MenuDown` and `MenuUp`, the
secondary button `MenuDecline`.

### `ConfirmButton_Update` plays `ACCEPT` on every screen it sits on (`0x088c9174` for the back button's `DECLINE`)

Every `ACCEPT` below has `ra = 0x088c90bc`, inside `ConfirmButton_Update` (`0x088c8e88`); the back button's
`DECLINE` has `ra = 0x088c9174`.

| Screen (state name) | Press | Cue |
| --- | --- | --- |
| `LogoFMV` (the intro movie) | Start or Cross, the skip | `ACCEPT` |
| `Show Logo` (PRESS START) | **Cross or Start** (`frontend-boot.md` says Start only; Cross works) | `ACCEPT` |
| `RemoveMemoryStickWarning`, `MemoryStickWarning` | Cross | `ACCEPT` |
| `NameSetup2FromBoot`, `TagSetup2FromBoot` | Cross on the confirm slot | `DECLINE`, `ACCEPT` (above) |
| Main menu, a row | Cross | `ACCEPT` |
| In race | **Start** (pause) | `ACCEPT` (state `InGame Pause Redirect`) |
| `InGame Pause SP Time Trial` rows | Up or Down | `UPDOWN` (`ra = 0x088b5340`, the `Menu` class step) |
| the pause menu | Circle | silent, the root has no back |
| the pause menu | Start | `ACCEPT` (resumes) |
| `CONTINUE`, `GAME OPTIONS`, `RESTART RACE`, `QUIT RACE` | Cross | `ACCEPT` |
| `InGame Settings TT` (a pause sub-page) | Circle | `DECLINE` (`ra = 0x088c9174`) |

**There is no confirm dialog on the pause menu.** `QUIT RACE` goes straight to `Kill Game Transition` and the main menu
with one `ACCEPT`; `RESTART RACE` goes to `InGame Restart` and the track description screen with one `ACCEPT`. The
pause menu is `CONTINUE`, `GAME OPTIONS`, `PHOTO MODE`, `GAME MANUAL`, `RESTART RACE`, `QUIT RACE` over the race.

**`Language Selection` was not seen.** On the USA disc a first boot with no profile goes `LogoFMV`,
`LogoFMVRedirectScreen`, `Show Logo`; the disc's `LanguageAutoRedirect` is taken, so the picker is never shown (the EU
disc's picker was not walked: this probe's address is the USA binary's, and a second image needs its own). The screen's
widgets are the `Menu` (name `Language`) and `ControlTextConfirmButton` classes whose cues are above, so the port's
picker plays `UPDOWN` for a step and `ACCEPT` for a confirm **by widget class, chosen, not measured**.

`FUN_088e4fd0` (a caller of `Sound_PlayNamedInSlot` with `LEFTRIGHT` and `DECLINE`) is the medal gallery's update
(`GalleryScreen`, `Medal %d`), not a text widget. Unnamed (confidence below 50 that its name is useful).

**What the port does with the boot and the pause** (pad and pointer, `Frontend::take_nav`, `press_cue`): the movie
skip, `Show Logo`'s press, the picker's step and confirm, and START pausing or resuming a race each play the cue above.
The port's pause has no rows and no sub-pages, so the pause menu's `UPDOWN` and `DECLINE` have no counterpart; the
port's Circle while paused leaves the race and plays nothing, as the original's root Circle does. The memory-stick and
profile-setup screens, the `Create Profile` flow and the in-race options pages are not in the port.

**Dialogs and the grid keyboard have no counterpart on the disc.** The port's yes/no `Confirm` (DELETE PILOT) and its
key grid `Keyboard` (a name `TagEntry` cannot spell, a LAN address) borrow the cues above and are **chosen, not
measured**: a step between answers or keys, `LEFTRIGHT` for a yes/no flip and a typed or deleted character, `UPDOWN`
for a grid cursor move, `ACCEPT` for yes and accept, `DECLINE` for no, back and a refused character (a full or empty
buffer, as the widget's cannot-advance step). Whether the original's "no" on a yes/no page would be a resolving button
(and so `ACCEPT`) was not settled, no such dialog being on a path walked.

**The campaign's refused launch.** `Session::campaign_nav` already turned a locked tier or cell into `DECLINE`; it now
also asks `plan_cell` (the same pure plan `launch_campaign_cell` runs) so a cell the build cannot race (a mode it does
not run, a circuit the disc lacks) plays `DECLINE` instead of `ACCEPT` followed by the notice. **Chosen, not
measured**: the original has no such refusal.

**Which titles hear this.** Everything above goes through the title's own front-end bank (`play_navs`), so Pulse hears
measured cues; **Pure and HD inherit them unmeasured** (the cue names resolve from each title's `Title` data: HD's
`accept`, `navUp` and so on), which means pausing an HD race with START, HD's rename and delete prompts and its boot
screens now play HD's cues too, with nothing measured on HD for any of them; Omega and 2048 have no front-end bank in
this build and stay silent. Scoping HD or Pure out is a one-line gate at the callers and was left to the lead.

**Pure and the PS2 pressing.** Pure is **not checkable here**: `/pure/BOOT-psp-pure-usa.BIN` is not open in the shared
Ghidra bridge and opening it would change the maintainer's session; the four cues play on its menus already and
`TagInput` is in its `Skin.xml`, so the port's mapping applies unchanged and is unmeasured on Pure. Pulse PS2 shares
one definition with the PSP in the port; its executable was not read for `TagInput_Update` (**not checked**).

## The voice banks

| Slot | Cue | Caller | Selector |
| --- | --- | --- | --- |
| `FRONT_END_VO` | `TMTX_NAME` | `TeamSelect_PlayNameVoice` (`0x088ea048`) | table slot 2 = the team's `soundregister`; wired 2026-10-11, see "The name voices" |
| `FRONT_END_VO` | `TRTX_NAME` | `TrackSelect_PlayNameVoice` (`0x088ed9a8`) | table slot 1 = the circuit's `soundregister`; wired 2026-10-11, see "The name voices" |
| `speech_results` | `FIRST_PLACE` / `SECOND_PLACE` / `THIRD_PLACE` | `EndRaceResults_Update` | wired 2026-10-11, see "The end-of-race voices" |
| `speech_results` | `gold_med` / `silver_med` / `bronze_med` | `EndRaceRewards_OnEnter` | wired 2026-10-11, see "The end-of-race voices" |

## The name voices

Lane `pulse-name-voices`, 2026-10-11. Disassembly of `BOOT.BIN` with `llvm-objdump` (the ELF is relocatable at VMA 0, so
address = VMA + `0x08804000`; no Ghidra bridge was up) plus a live PPSSPP v1.20.4 run of the USA disc. The earlier
reading of this page, that `FUN_088ea048` itself calls `FUN_0898dac0`, was wrong in one detail: the two wrappers pass
the slot and value to `Sound_PlayNamedInSlot` as its two **last arguments**, and that function does the store.

**The play call.** `Sound_PlayNamedInSlot(mgr, bank_slot, name, pitch, flags, table_slot, value)` (`0x0893a768`):
`$a0..$a3 = mgr, bank slot, name, 0x400`, `$t0 = flags`, `$t1 = table_slot`, `$t2 = value` (sign-extended byte). It
calls `FUN_0898dac0(table_slot, value)` before it starts the cue. `FUN_0898dac0` (`0x0898dac0`) is `if (0 < slot < 0x21)
byte[0x08ac3247 + slot] = value`; the store is `sb` at `base + slot - 1` from a base of `0x08ac3248`. **76 of the 79
call sites pass `$t1 = 0`** (`move $t1, $zero`), which is out of range and stores nothing; one more is a register move
this scan did not follow, and the two name voices pass 2 and 1. So ordinary menu cues never write slots 1 and 2.

**The reader.** `Scream_OpGuard`'s negative variable reads `0x08ac3247 - variable` (`sound.md`), so variable `-1` is slot 1
and `-2` slot 2. Every command pair in `TMTX_NAME` is guarded `[0xfe, 1, N]` (variable -2, mode 1 = skip unless equal,
immediate N) and every command pair in `TRTX_NAME` `[0xff, 1, N]` (variable -1). The writer and the reader agree on the
slot arithmetic, and both banks' guards name their own slot.

- **`TMTX_NAME`** (cue 11, 48 commands): for N in 1..=9 and 11..=13, a guarded pair of key-ons, the second **10 master
  ticks** after the first (two grains at 30 and 330 degrees). **N = 10 has no command**: a team whose register is 10
  says nothing, and no value is made up for it.
- **`TRTX_NAME`** (cue 1, 66 commands): for N in 1..=32 a guarded `0x08` child record naming the circuit's own cue, in
  the bank's own order: 1 `TALONS_WHIT`, 2 `TALONS_BLACK`, 3 `MOA_WHITE`, 4 `MOA_BLACK`, 5 `METROPIA_WHITE` ..
  32 `ORCUS_BLACK` (cues 15..=46). N = 0 passes a no-op `0x14` and plays nothing.

**The value is authored data: the `soundregister` of the node.** `Data\Plugins\PI001\Definition.xml` carries
`soundregister` (tokenised `k`) on every `PI_Track` (1..=24 in file order on the PSP discs, `16_Track` 1, `32_Track` 2,
`03_Track` 3, ... ) and every `PI_Team` (`Feisar` 1, `AG_Systems` 3, `Assegai` 4, `Qirex` 5, `Piranha` 7, `Triakis` 8,
`EGX` 9, `Goteki` 13). `oag_raceplay::catalogue` documented the attribute as "what it selects is not established"; this is
what it selects. A DLC pack's entries carry theirs; the four DLC teams are presumably the registers the eight base teams leave unused (2, 6, 11, 12), **inferred, not measured** (no pack was mounted on the capture).

**Live capture (PPSSPP, USA disc, a fresh profile).** A breakpoint on `Sound_PlayNamedInSlot` with condition
`t1 != 0`, `t2` read at each hit, a screenshot taken at the hit, and the selection screen's item array dumped
(`this + 0xe8` array, `+0xec` count on Team Selection; `this + 0xec` array, `+0xe4` count on Track Select, items
`+0xac` or `+0x160` the register, `+0x74` the folder id):

| Screen and action | Hit | `t1` | `t2` | What the screen showed |
| --- | --- | --- | --- | --- |
| Track Select, Confirm on 1/3 | `TrackSelect_PlayNameVoice` (`ra 0x088ed9e0`) | 1 | **1** | `16_Track`, Talon's Junction White |
| Track Select, Confirm on 2/3 | the same | 1 | **3** | `03_Track`, Moa Therma White (screenshot) |
| Track Select item 2 | (array dump) | | **5** | `18_Track`, loading `02_Track` reversed: `METROPIA_WHITE` |
| Team Selection, Confirm on 1/8 | `TeamSelect_PlayNameVoice` (`ra 0x088ea080`) | 2 | **4** | Assegai (screenshot) |
| Team Selection, Confirm on 6/8 | the same | 2 | **13** | Goteki (the list's sixth item) |
| Team Selection array | (dump) | | 4, 5, 1, 3, 9, 13, 8, 7 | Assegai, Qirex, Feisar, AG Systems, EGX, Goteki, Triakis, Piranha: **each equal to that team's `soundregister` in `Definition.xml`** |

**Triggers.** The voice sounds on **Confirm** and never on scroll: 4 downs on Track Select and 14 downs on Team Selection
(every item visited) produced no hit under the same breakpoint. Pad and pointer reach it the same way; the wrapper's
`$a1` flag gates it and a screen in its music-select or help state does not speak. The team voice plays on the Confirm
that leaves Team Selection for `Launch Game Transition`. The circuit voice plays on the Confirm that leaves Track Select
for Team Selection.

| Address | Name | Confidence | What it does |
| --- | --- | --- | --- |
| `0x0898dac0` | `Scream_SetGlobalVariable` | 88 | `(slot, value)`: stores the byte at `0x08ac3247 + slot` for `1 <= slot <= 32`, ignores the rest. `Scream_OpGuard` reads the same table and `Scream_OpSetRegister` (`0x1e`) writes it from a cue's own commands |
| `0x088ea048` | `TeamSelect_PlayNameVoice` | 92 | `(screen, register)`: `Sound_PlayNamedInSlot(bank slot, "TMTX_NAME", 0x400, 0, table slot 2, register)`. Live: hit at Confirm with `t1 = 2`, `ra = 0x088ea080` |
| `0x088ed9a8` | `TrackSelect_PlayNameVoice` | 92 | the same with `"TRTX_NAME"` and table slot 1. Live: `t1 = 1`, `ra = 0x088ed9e0` |

The two callers (`0x088ea02c` in `0x088e9fa0`, `0x088ed714` in `0x088ed6cc`) are not renamed: they read the selected
item's register (`item + 0xac`, `item + 0x160`) behind three string compares of the screen's current page
(`Pre Race Music Select`, `Team Help`, `Track Help`), which is the gate above, not read further.

**Wired** (`oag_sound::sfx::NameVoices`, `MenuSfx::play_team_name` / `play_circuit_name`, `oag_game`'s
`Session::speak_name` on the picker's `Event::Confirmed`): each register's voice is the cue's walk with the global set
(`oag_formats::sblk::timeline::WalkModel::globals`, which also walks the `0x08` child), not one picked waveform, so a
team is its two grains ten ticks apart and a circuit its own child cue. The title names the bank
(`oag_title::FrontEndSounds::voices`, `Some` on Pulse only). Played dry on the speech bus, **chosen, not measured**
(the original plays it through the one front-end slot, as the end-of-race lines). The grains' 30 and 330 degree angles
play centred: the front end is dry. The ground truth is `crates/raceplay/tests/name_voice_ground_truth.rs`: every circuit
value's walk equals its child cue's own, the first three are the circuits the capture saw, a team is a pair ten ticks
apart, eleven teams give eleven different names, the registers read from the discs' definitions are the measured ones,
and a headless WAV is silent until a confirm and sounds on it.

**Other titles.** Pulse PSP EU and Pulse PS2 EU: **ported**, the same bank entry, the same guards, and the same
registers in each disc's `Definition.xml` (the PS2 disc lists 32 circuits, each register once); the PS2 trigger was not
observed running, only its data. Pure: checked, not wired: the executable was not read for these calls
(`FrontEndSounds::voices` is `None`). Wipeout HD/Fury: **checked, differs**: its page lists `TMTX_NAME` playing when the
team *changes* (`0x002502d8`, confidence 80), the bank is not `FRONT_END_VO.bnk` (no such entry in its archives) and a
census of `frontend.bnk` and `speech.bnk` finds no cue of that name, so its voice bank and selector are not located.
Omega: not checkable, its front end is HD's and there is no PS4 capture path.

## The end-of-race voices

Decompiled headless 2026-10-11 (`/pulse/BOOT-psp-pulse-usa.BIN`, `EndRaceResults_Update` `0x088da530`,
`EndRaceRewards_OnEnter` `0x088dbbd4`). Both play with `Sound_PlayNamedInSlot(DAT_08ac1e10, DAT_08ac1e00, name,
0x400, 0, 0, 0)`: the `speech_results` slot, dry. `EndRaceResults_Update` keeps a once-flag at widget `+0xe8`.

**The place line** (`EndRaceResults_Update`; `+0xe4` points at `g_endrace_result`, word 0 the finishing place, word 1
the Tournament standings rank, `tournament.md`):

| `g_game_mode` | When | Place read |
| --- | --- | --- |
| 3 (`Arcade`, Single Race) and 8 (`Elimination`) | every update tick until the flag is set, so the first | word 0 |
| 4 (`Tournament`) | the first three-second page flip. `EndRaceResults_OnEnter` leaves the page flag `+0xe0` at 1, so the first flip takes the else branch, `ER_TOUR_STAN` | **word 1**, the standings rank, not the leg's place |
| 0x10 (multiplayer Tournament) | the same flip | the same word; not reachable here, this build has no split-screen |
| everything else (Time Trial, Speed Lap, Zone, `Head2Head` = 9) | never | - |

1 plays `FIRST_PLACE`, 2 `SECOND_PLACE`, 3 `THIRD_PLACE`; 0 and 4 or worse play nothing and **leave the flag clear**
(it is set only on a play). The earlier reading of this page and `endrace-screens.md` ("once per results screen, off
the finishing position") held for modes 3 and 8 only. Confidence 85 (a direct read of the branch; the once-flag's
reset on re-entry is not read: `OnEnter` never writes `+0xe8`. `VIEW RESULTS AGAIN` speaking the place again is
**chosen, not measured**).

**The medal line** (`EndRaceRewards_OnEnter`): only under `DAT_08b30ffc != 0` (a campaign cell in play), on entry,
off the medal ordinal at `result + 0x1160`: 0 `gold_med`, 1 `silver_med`, 2 `bronze_med`, otherwise nothing. The same
branch unpauses the matching trophy. Confidence 85.

**Wired** (`oag_game::endrace_voice`, the session's `tick_endrace`): `speech_results.bnk` is a second front-end bank,
`oag_title::FrontEndSounds::results` (`Some` on Pulse, `None` elsewhere), `oag_sound::sfx::BankName::SpeechResults`
and `Cue::RESULTS`, loaded by `MenuSfx` beside the navigation cues and only when the title names it. The bank rides
the speech bus, **chosen, not measured**. `FRONT_END_VO.bnk` (team and circuit names) reuses the same shape (see "The name voices"): one
more optional path in `FrontEndSounds`, one more `BankName`.

**Cross-title** (census by `endrace_voice_ground_truth.rs`):

| Title | Result |
| --- | --- |
| Pulse PSP (USA, EU) | **ported**: all six names resolve, `FIRST_PLACE`.. 2 waveforms each, `gold_med`.. 4 each |
| Pulse PS2 (EU) | **ported**: same bank under the same name in `WADS2.WAD`, same counts |
| Pure PSP | **checked, differs**: no `speech_results.bnk` entry; `gold_med`/`silver_med`/`bronze_med` sit in `speech.bnk`, there is no place line there. Its call sites were not read, so nothing is wired |
| HD / Fury | **ported** (2026-10-11): `speech_results.bnk` ships with all six names (2 waveforms each, a stereo pair), and HD's own call sites differ: the place on entry, never standings, and not in a campaign cell, where the medal plays instead; see [endrace-voices.md](../ps3-hdfury-eu/endrace-voices.md) |
| 2048, Omega | **not checkable**: both load a `speech_results` bank (`game-boot.md`) but as a nameless v5 bank / Wwise events, with no SFX path to resolve a cue by name |

## Wired by this lane

`oag_gameplay::cue::Cue::MenuUpDown`, `MenuLeftRight`, `MenuAccept`, `MenuDecline` (bank `BankName::Frontend`, named by
`oag_title::SoundBanks::frontend`, `Some` on Pulse and Pure only) and `oag_ui::menu::nav::Nav`, which the main menu,
the selection screens, the campaign screens and the EndRace menu raise. Chosen, not measured, and scored nowhere:
pointer hover and click use the pad's cues (hover sounds once per row change); `LEFTRIGHT` on confirming a toggle (the original has no confirm-steps-
value); `DECLINE` on closing the root menu. Pure's triggers are Pulse's read applied (Pure's executable carries the
same six names, minus `PAUSE`, but its call sites were not read).

## Played in a real walk (2026-10-11, lane `voices-ingame`)

The ground truths above drive `Voice`, `MenuSfx` and the name lookup directly, so until this walk nothing had shown the
**session** starting them. Pulse PSP EU, `oag-game` windowed in a private Xvfb on software Vulkan, keys sent with
`xdotool`, `--autopilot` flying the player, and `--dump-audio FILE` writing the session's own mix when the window
closed (a windowed dump is new: it needs no `--screenshot` any more, and clap forbids `--no-audio` beside it, so
`Audio::open` forces the null backend and the run is silent on the host by construction). Each cue's debug line
carries `dump frame N`, the WAV frame it starts at (48 kHz).

| Step on screen | Log line | Frame |
| --- | --- | --- |
| Racebox, Single Race, Track Select on TALON'S JUNCTION WHITE, Confirm | `menu sound: MenuAccept` and `name voice: Track 16_Track (soundregister 1, started: true)` | same frame |
| Ship Select on ASSEGAI, Confirm | `name voice: Ship Assegai (soundregister 4, started: true)` | same frame as its `MenuAccept` |
| Race over, `Race End Photo`, Confirm, `RESULTS 3RD PLACE` | `menu sound: MenuAccept`, then `end-of-race voice: ResultsThird` | `ACCEPT` + 800 (one tick later) |
| `EndRace Menu`, `VIEW RESULTS AGAIN` | `end-of-race voice: ResultsThird` again (chosen, not measured) | one tick after the confirm |
| Campaign grid 1, a Single Race cell, Ship Select on ASSEGAI, Confirm | `name voice: Ship Assegai` only (no Track Select is shown in a cell, as recorded) | with `ACCEPT` |
| That race to the flag, `RESULTS 3RD PLACE`, then `REWARDS BRONZE MEDAL AWARDED` | `ResultsThird`, then `end-of-race voice: MedalBronze` | one tick after each confirm |

**What the WAV shows.** The brief's "silence before each cue" did not hold and is not the test: menu music, the race
ambience under `Results` and `Rewards`, and (on the confirms) `ACCEPT`, which saturates the output, are all sounding.
The proof is a normalised cross-correlation of each cue's isolated render (the same `MenuSfx` calls played into a null
`Audio` and written with `--dump-audio`'s writer, onset trimmed) against the walk's mix around the logged frame, after a
first-difference high-pass to take the music's bass out:

| Cue | Peak correlation | Lag from the logged frame | z over the lags -0.5 s..+1 s |
| --- | ---: | ---: | ---: |
| `ResultsThird`, Single Race | 0.55 | +88 frames (2 ms) | 52 |
| `ResultsThird`, `VIEW RESULTS AGAIN` | 0.56 | +88 | 52 |
| `ResultsThird`, campaign cell | 0.60 | +88 | 53 |
| `MedalBronze` | 0.61 | +88 | 43 |
| `TMTX_NAME` Assegai, first grain (the team name is two grains ten ticks apart) | 0.67 | +104 | - |
| `TRTX_NAME` Talon's Junction White | 0.31 | - | 25 |

The isolated renders start 87 to 105 frames after their play call, so +88 is the cue starting on its logged frame.
Controls, the same templates against three `ACCEPT`-only moments (main menu, `EndRace Menu`, campaign grid): peak
correlation 0.02 to 0.04, with no consistent lag. The whole-template peak of both name voices sat at about -36 ms,
which no cue can do. For the team, correlating the first grain alone puts it at +104 (against `ACCEPT`'s own +128), so
that one is a grain-pairing artefact of the template; the circuit's -36 ms was not chased and stays unexplained
(the cue is present, z 25, and starts with its `ACCEPT`).

**What the session wiring is tested by.** There is no route that builds a `Session` without a window (`Gpu::new` takes
the event loop), so the calls are one-line forwards into GPU-free code that tests drive: `Speaker` (the screen-on-top
reading, with `Flow`, tested through Photo, Results, Rewards, Menu in `race_stage/speaker_tests.rs`), and
`name_register`/`play_name` (`session/menu_sound/tests.rs`, disc-backed, `#[ignore]`d: the registers 1 and 4 and the
waveform each kind starts). Eight mutations were run by hand against that:

| Mutation | Test | `just lint` |
| --- | --- | --- |
| `Speaker::tick` maps `Results` to no screen | fails | - |
| `name_register` swaps the circuit and team lists | fails | - |
| `play_name` swaps the circuit and team voice | fails | - |
| `EndRaceRuntime::new` handed `None` for the voice (was `set_voice`) | passes | fails (unused variable) |
| `voice_tick` call dropped in `tick_endrace` | passes | fails (dead method and field) |
| `play_voice` call dropped in `tick_endrace` | passes | fails (value never read) |
| both `speak_name` calls dropped in the picker | passes | fails (unused variable) |
| `Session::speak_name` not playing what it resolved | passes | fails (`play_name` unused) |

So the **Session forwards are guarded by the compiler's dead-code and unused warnings under `-D warnings`, not by a
test**: a deletion that also removes the binding would pass both. The walk above is what shows them wired.

Open: not heard by a human; a campaign cell speaks only the team (no Track Select there), consistent with the
recorded triggers; the medal was only walked for bronze (the autopilot finished third on both races).
