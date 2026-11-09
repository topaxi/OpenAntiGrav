---
status: measure
titles: [pulse]
updated: 2026-11-09
summary: "Hull, pads, list B match the original; flares now sort into list B by depth (landed). Open: a pad reproduction, HD/Omega/2048 keys unread"
---

# Pulse's blended scenery draws before craft and pads

The maintainer's report from play, 2026-11-07: in Pulse, trees render behind ships, speed pads and
weapon pads, even when the tree or bush is in front. Observed on Platinum Rush White (`13_Track`).
Measured on PPSSPP; the numbers, the method and the confidence are on
`docs/rendering/draw-order.md`, "Hull, pads and trees on Platinum Rush White".

## Open

- **Hull and pads: ours already matches the original at the default.** The original draws hull, then
  speed and weapon pads (layer `0x4a000000`), then list B (`0x4d000000` plus depth, no depth write).
  Ours does the same whenever `Race::ribbons_first()` holds (PSP ribbon, `trail_order = original`, the
  maintainer's own setting). The symptom as reported is the other order: `over-floors`, or the HD tube
  path (HD, Omega, 2048). Not reproduced on White: three matched poses and one synthetic pose.
- **Flares: landed 2026-11-09 (`pulse-flares`).** Under `ribbons_first()` a craft's engine flare quad is queued among
  the list-B meshes by the original's key (`0x4d` plus the nozzle's depth, unscaled) against a mesh's `z * fov / 65`
  (the scale is `g_camera_fov_degrees`, read off `Camera_SubmitScene`: the page's `display+0x4310` was that global's
  address). Plumes, shield shells, weapon bodies, wrecks and the Rocket ribbon have no key read and stay where they
  are, so they now draw after the flares. Rule and held-by on `docs/rendering/draw-order.md`, "The flares are wired".
  **Picture is ours only:** an AI flare behind a nearer tree was never caught in the original (four runs of the pack
  passing a placed player), and the original does not take an opponent placed by hand (`psp-drive.py place --craft` on an
  AI: back on its own line 366 units away in two ticks). A matched pair is still wanted: a frame of a race with an
  opponent a few tens of units ahead beside a tree-lined section. `psp-race-field.py hunt --place 3-6` Autopilots the
  original into a mid-pack frame and stops there (Autopilot is fine for getting somewhere), then `psp-pulse-pair.py
  --field`, with a base-against-branch render of ours as the overlap detector.
- **Open on the original:** the maintainer's spot (their settings hold `camera_view = "far"`; the
  disc was Pulse PSP, they confirmed 2026-11-07, so the PS2 path is not the cause), which of
  White and Black each Track Select press landed on (`--track-down 18` and `19` gave the same start
  frame), the hull body's second layer (the `0x4a` bump moved it too), the tree's own key under a
  list-B bump (the prim vanished from the census), the weapon pads' submit (`0x0890cdc8`), PS2, Pure's,
  HD's, 2048's and Omega's queue keys.

## Next Steps

1. ~~Wire the flares into the blended list by depth~~ (landed, see Open).
2. Reproduce with a **speed or weapon pad**, not a craft. The maintainer specifically saw a pad and
   may have misremembered the ship (2026-11-07). The measured poses and the synthetic one used a
   craft. Use far camera, Pulse PSP, Platinum Rush White, and a tree between the eye and a pad. If it
   reproduces at `original`, read the frame's order with the key bump on the page. Placing the
   camera is the easy part in ours (maintainer's suggestion): park the race in photo mode
   (`docs/ui/photo-mode.md`) or use a capture pose, and put a tree between the lens and a pad. Then
   find a matching spot on PPSSPP for the original.
3. For HD, Omega and 2048, read each title's queue keys before moving a blended list: their path draws
   it before the craft today.
