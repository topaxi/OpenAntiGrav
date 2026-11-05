---
status: re-first
titles: [hd]
updated: 2026-11-05
summary: "HD's 1080p scene width moves in 20 px steps inside a boot (1300-1920 seen); the controller is not found, and 1080p pairs carry it"
---

# HD's 1080p dynamic width: the steps are measured, the controller is not found

`docs/rendering/hd-track-tone.md`, "The 1080p path renders a reduced, dynamic width", has the
16 widths. Size: half a day of RE (confidence 50), unknown if the value is computed on an SPU.

## Open

- **The controller.** What reads an overrun and writes the width. A third party quotes Digital
  Foundry (2009) as 1920 to 1280, 32-pixel steps, one step a frame, reactive, 1080p only; RPCS3
  on 02.00 shows 20-pixel steps. Their article could not be fetched from this machine, so the
  quotation is unchecked. 02.51 is unmeasured.
- **What a pair's score owes it.** Each 1080p original frame is one width of many, unrecorded.

## Next Steps

1. Live: in an attract race at 1920x1080 scale 100, dump two frames with different clips
   (`scripts/rsx-pass-events.py`), scan guest memory for the int or float holding both, watch it.
2. Static dead ends, do not repeat: `Debug.Enable Frame Buffer Throttling` is registered at
   `0x003a9464` at offset 1442 with no `lbz` reader; `mulli .., 20` appears only at `0x0056d1ec`
   and three neighbours; no `cmpwi` against 1280 or 1920.
3. Then record the width in the pair metadata (`scripts/rpcs3-hd-pair.py`), or pin it with a
   patch for references (the sender's approach), so a pair names its width.
