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
- How the original marks focus when driven by pad buttons. It is a Vita game with both touch and
  buttons, so the state may be authored in the screen XML (a focus or highlight widget, tween or
  texture), and it may differ per screen.

## Next Steps

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
