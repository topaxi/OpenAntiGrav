# HD's Bomb blast pair has no scene block, and the LeachBeam's strips are unread

2026-10-06, hd-weapons. The rest of the weapon-drawable scene work landed
(`docs/rendering/hd-unlit-programs.md`, "Weapon scene blocks").

## Open

- The Bomb's hemisphere and shockwave, the Repulser field and the mag floor
  hold `Scene::off`. Frames need a detonation, which needs a rival on the
  bomb; keep one pinned to the grid ticks before writing them.
- **LeachBeam body.** HD's beam is two `0x63d0`-byte strip objects at
  `LeachBeam+0x50`/`+0x6420` (`0x00115770`, constructors `0x001153c0`,
  `0x00115cc0`; 300 samples of `0x50` bytes, colour `0xffffff00`), built by
  `0x00153218`, from `leachbeam_triangle` + `hd_leachbeam.rcsmaterial` +
  `hd_leechbeam_glow.gtf`. Their draw is unread. HD no longer asks for Pulse's
  `pulse_leechbeam1_ADD.mip` (the WARN is gone; the report says the strips are
  unbuilt).
- **Ghost static.** `search_strings` in `/hdfury/EBOOT-ps3-hdfury-eu.elf` finds
  no `staticglow` and no `static.gtf`; `GhostShip.cpp` exists. So nothing in
  the executable binds the texture and the WARN is by design, but it still
  names a Pulse asset on HD: gate it with a `Title` field.

## Next Steps

1. Read what `0x00153218`'s strips are drawn with, then build them beside the
   other three ribbons (`docs/rendering/trail-ribbon.md`).
2. Add the ghost-static axis to `Title` so HD stops asking.
