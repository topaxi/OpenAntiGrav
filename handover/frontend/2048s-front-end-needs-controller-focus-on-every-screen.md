---
status: ready-to-wire
titles: [2048]
updated: 2026-11-05
summary: "2048's front end: every screen reachable by controller, with a visible focus state in the original's own style"
size: M (40)
difficulty: medium (40)
---

# 2048's front end needs controller focus on every screen

The maintainer asked for this: the 2048 front end must work from a controller. Where navigation
already works, the focused item must still be visible (colour, opacity or a focus ring), and that
state must match the original's design language.

`docs/formats/2048-frontend.md` records pad input on some screens (`GameModeChoice`, `newFEshell`,
`team`), so part of this may exist. That page is a lead, not a result: walk the screens.

## Open

- Which screens cannot be used from a controller, and which show no focus.
- Whether the original's touch screens answer the d-pad: the code below exists, but nobody has
  pressed them on Vita3K.

### PS TV mode: no separate control path found (2026-11-05, static, confidence 55)

Strings and XML on the EU v1.04 package.

- Sony's launch list left 2048 out and a review lists it as not PS TV compatible; a whitelist
  hack was the only route reported.
- The executable has no `TV` string beyond noise. Of four `SceCtrl` NIDs checked it imports
  `sceCtrlSetSamplingMode` and `sceCtrlReadBufferPositive`, not `sceCtrlGetControllerPortInfo` or
  `sceCtrlGetWirelessControllerInfo`.
- `data/FE/Images/TV.gxt` (256x256, 7 mips) is named by no XML and no executable string. Its
  picture has not been looked at.
- The screen XML authors no pad focus: 194 `<TouchButton>`, 21 touch lists, one
  `<TouchHomeButton>`. `FocusMenu`, `FocusMenuEntry`, `Selector`, `SelectorGlow`, `navUp` and
  `highlightcolor` exist in the executable, HD's inheritance, and `navUp`, `FocusMenu` and
  `SelectorGlow` appear in no XML.
- The one sign of a non-touch path: `Team_Definition.xml`'s `teamskin_touch` has `focus="true"`
  and `ArrowTouchOnly="true"`, so that list has arrows only under touch and a state without them.
- The 2048 palette is `Blue2048`, `Orange2048`, `White2048`, `Grey2048` (`FEGlobals`).

### The executable keeps a real pad path (2026-11-05, Ghidra decompile, confidence 60)

This corrects the reading above: the path is in the code, shared on every Vita, not TV-specific.

- `FUN_8122d1e0(pad, action)` is the button query (`FUN_8122cfbe` names the actions: `start`,
  `select`, `circle`, `cross`, `forward`, `backward`, ...; 0 to 3 read as the d-pad, not proven).
  66 functions call it, among them option pages, the music list, track select, online lobbies,
  photo mode and the end-of-race screens.
- Focus is a per-item flag, `(item + 0x30) & 0x200`, and `FUN_81067b92(screen, -1 or 1)` steps it
  (`FUN_810818ec`, `FUN_810e5458`). `FUN_810e5458` names the item's state `HasFocus` or
  `NotHasFocus` at `item + 0x90`.
- Navigation with sounds: `FUN_81118cfc`, `FUN_8110aa5c`, `FUN_810ddb4c` play `navUp`, `navDown`,
  `navLeft`, `navRight`. `FUN_81097ed6` moves a `Selector` and `SelectorGlow` over `Outline_%d_%d`
  cells on a grid.
- Not settled: `HasFocus`, `Selector` and `Outline_` appear in no 2048 XML, so these may be HD
  code the touch screens never reach. Whether `TouchButton`, `TouchCampaign*`, `TouchTeamGrid` or
  `TouchTrackGrid` take the `0x200` flag is unread. `Touchlist` draws its arrows only when
  `+0x12d` (`ArrowTouchOnly`) is clear (`FUN_810fce98`, parsed in `FUN_810fc48c`), which is
  arrow geometry, not yet a touch check.

If the touch screens do take the flag, the original's focus state is the thing to play. If not,
step 4 below is the main path and `Selector`'s glow is the closest authored model to borrow from.

## Next Steps

0. Press the d-pad, cross and circle on Vita3K on `GameModeChoice`, the map, `Home` and `team`.
   That settles whether the original has any button path, and shows `teamskin_touch` by button.

1. Census: for each 2048 front-end screen ours draws, drive it with `oag-game` drive verbs or
   keys, and record whether each control is reachable, activates, backs out, and shows focus.
2. Read the original's focus state from the disc's XML and textures first, then confirm it on
   Vita3K (`docs/reverse-engineering/vita3k-capture.md`, `scripts/vita3k-2048-menu-pair.py`).
   Check the finding against Omega's HD-derived front end and record the answer per CLAUDE.md.
3. Wire the missing navigation and play the authored focus state. Touch and pointer keep working.
4. Only where the original authors nothing, add a state built from the same screen's own art and
   palette, marked **chosen, not measured**, and list each one here for the maintainer.
5. Test per screen: a focus move changes the drawn state and activation reaches the focused
   control. Add a focused-state pair to the compare page where the original can be captured.
