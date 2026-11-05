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
- Whether the original's menus answer the d-pad at all. Static reading says little beyond the
  skin list below; nobody has pressed them on Vita3K.

### PS TV mode: no separate control path found (2026-11-05, confidence 55)

Static only, on the EU v1.04 package; Ghidra was down, so no input code was decompiled.

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

So step 4 below is likely the main path, not the exception: expect a chosen focus state.

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
