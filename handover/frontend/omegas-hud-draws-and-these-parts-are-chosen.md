---
categories: [frontend, rendering]
---

# Omega's HUD draws off HD's five roots and its own `.gnf`; the 2048 skin set, Zone classes and the runtime tables are open

2026-10-06, `omega-hud`. Evidence and every number: [omega-status.md](../../docs/formats/omega-status.md)
(the in-race HUD section) and
[race-hud-selection.md](../../docs/ghidra/functions/ps4-omega-eu/race-hud-selection.md).

**Shipped.** `oag_omega::hud` names `Data\XML\{Arcade,Elimination,TimeTrial,SpeedLap,Zone}_HUD.xml`
(measured off the single-player race managers) and `texture_extension: .gnf`; the HUD font takes
`Space::font_texel_scale` so Omega's 2x faces lay out at HD's size. The two HUD-atlas WARN lines are
gone. Disc-backed: `crates/game/tests/omega_hud_ground_truth.rs`. An earlier "Omega ships no per-mode
composition" claim was a truncated census (NUL-separated PS4 manifest read by `scripts/psarc.py`).

## Open

- **Which race selects the `2048_hud\` set.** It composes on Omega and its atlases resolve, but its
  constructors are reached by virtual dispatch (`FUN_015ae4d0`, `FUN_015ac880`, `FUN_015bc700`) and
  `Title::hud` is one set per title. Likely the `environments2048` circuits, as with particles; not read.
- **`wo3_hud` and `2097_hud`** are the "HUD Style" values the arcade manager branches on; not wired
  (same state as HD, `hud-legibility-and-hud-skins.md`).
- **Zone's speed-class text** draws nothing: `zone_speed_classes` is `None` until Omega's class names
  are read.
- **`always_on`, sights, `RUNTIME`, `shield_percent`, `message_slots` are HD's readings**, chosen, not
  measured; every name exists in Omega's layouts, the values are unchecked. No PS4 emulator to compare.
- **The HUD font's half-size layout is an inference (confidence 60)**, shared with the front end.
- `hdHUD.gnf` is HD's art in part redrawn (mean difference 17 of 255); what changed is not looked at.

## Next Steps

1. Read the 2048-lineage manager's selector (xrefs to the vtables near `0x018c2380`) and make
   `Title::hud` choose by circuit lineage if it is measured.
2. Read Omega's Zone class strings and fill `zone_speed_classes`.
