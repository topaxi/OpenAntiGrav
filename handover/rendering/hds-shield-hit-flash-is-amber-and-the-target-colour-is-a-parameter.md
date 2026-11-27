---
status: re-first
titles: [hd]
updated: 2026-11-27
summary: "HD shield paired with the original (raised, hit flash); open: ours adds ~75 % of its blue and the original barely flickers"
---

# HD's shield hit-flash is amber, not Pulse's cyan - and the steady-state colour is a parameter this pass did not trace to its source

2026-09-16. The user, who plays the originals, reported from play that HD/Fury's
shield "renders and animates, it just does not look exactly like in the
original (shape/color)." The renderer was drawing HD's shield through Pulse's
own recovered law and constants, title-blind - the entry-name resolution
already picked HD's own per-team `shipshield.vex`
(`crates/livery/src/entry.rs::shield_entry_names`), but the colours and
swell/scale constants painting it were `oag_render::shield`'s Pulse ones on
every title.

**Read HD's own `Shield.cpp` on `/ps3-hdfury-eu/EBOOT.elf`.** Full account in
[`docs/ghidra/functions/ps3-hdfury-eu/shield.md`](../../docs/ghidra/functions/ps3-hdfury-eu/shield.md).
The law is the same one Pulse's page already recovered - fixed ~60 Hz substep,
per-channel colour lerp, swell lerp, `sin`-shaped alpha/scale flicker, the
external-camera-vs-cockpit-sphere branch - checked field for field against
[`shield-pickup.md`](../../docs/ghidra/functions/psp-pulse-usa/shield-pickup.md).
Two colours are confirmed to differ:

- **The hit flash is amber, `(1.0, 0.2, 0.0, 1.0)`**, where Pulse's is cyan,
  `(0, 1, 1, 1)`. Read off `ShipShield_InitColours`'s own literal stores into
  a runtime colour block, and confirmed as the value `ShipShield_Hit` copies
  on an absorbed hit.
- **The activation colour is a dim red, `(0.547, 0.1, 0.1, 0.0)`**, where
  Pulse's is transparent black. Alpha is still `0.0` on both, so the shell is
  still invisible on the activation frame - only the hue the fade-up starts
  from moves.

**The steady-state colour is a call argument on HD, not a hardcoded
constant.** Pulse's `ShipShield_Activate` writes white itself; HD's takes it
from its caller. The likely caller (`FUN_000d5610`, HD's probable
`Shield_Fire`) was not traced through this binary's TOC defect in the time
this pass had - see [`README.md`](../../docs/ghidra/functions/ps3-hdfury-eu/README.md)'s
"code xrefs are sound, data xrefs are not". A per-team `ShieldColour` config
key exists and is parsed elsewhere (`FUN_000dba30`), which is a live
hypothesis for what feeds that argument, not a confirmed one.

**Ported behind a title axis.** `oag_render::shield::Palette` (`activation`,
`target`, `hit`) is a new small `Copy` type; `PULSE_PALETTE` is exactly what
`ShipShield::new()` always drew (unchanged, every existing test still passes
unmodified) and `HD_PALETTE` carries the two measured colours plus
`TARGET_COLOUR` - Pulse's white, **chosen rather than measured**, labelled so
in the doc comment, because it is the least-invented default until the real
call site is read. `Setup::shield_palette` is resolved in
`crates/raceplay/src/load.rs` from `craft_title.name == oag_hd::TITLE.name`,
the same title `shield_entry_names` already keys its own path resolution off,
and threaded into `Race::start` the same way `hd_trail` is. Two new render
tests pin the palette difference and the shared-target choice
(`crates/render/src/shield/tests.rs`).

**Not captured live.** The `--give shield` debug affordance never produced a
visible activation in repeated `--screenshot` runs across a wide tick range
(10 through 900) on `talons_junction` - `holding Shield` stayed the telemetry
line throughout, which only says the pickup slot is non-empty (kept refilled
by `--give`), not that `Shield_Fire`'s bit ever set. The wiring this pass
*did* exercise end to end is the existing unit-test suite -
`race::tests::weapons::a_fired_shield_arms_the_timer_for_its_authored_duration`,
`...the_cockpit_view_swaps_the_shield_shell_for_its_sphere`,
`...absorbing_a_shield_pays_the_shields_own_absorb` and the two new palette
tests all pass - so the law is verified against the recovered constants, but
nobody has watched HD's amber flash in a rendered frame yet. Whether that is a
timing quirk of `--give`/`--press` (the grant landing one frame later than the
edge it should coincide with) or something else is unresolved; a Weapon Pad
crossing in a real race, or a rival-contact hit while carrying one, would
sidestep it.

## Read 2026-11-03 (`hd-shield-read`): the target colour is per team

`ShipShield_Activate` takes no colour argument. The target is the
`ShieldColour` material parameter authored in each team's `shipshield.rcsmodel`
(`ShipShield_BindColourParam`, `0x00125690`); live RPCS3 read of the shield
object on a Feisar race gave target `(0.0, 0.031, 0.996, 1.0)` (blue), lerping
from activation `(0.547, 0.1, 0.1, 0)` over about 2 s. Table of all teams and
the read recipe: `docs/ghidra/functions/ps3-hdfury-eu/shield.md`, "The
steady-state colour is read".

## Watched and paired 2026-11-27 (`hd-shield-flash`)

Amber hit flash watched live (`(1.0, 0.2, 0.0)`, swell `1.1`) and the raised shell and flash
paired against ours: `docs/ghidra/functions/ps3-hdfury-eu/shield.md`, "Watched on RPCS3".
`--give shield` fires in ours (the 2026-09-16 note above no longer holds). `--force-hit` on a
craft with a running Shield now flashes its shell. The activation tint item is closed: the draw
overwrites it every frame.

## Open

- **Ours' settled shell is dimmer.** Its own light over the craft box is (0.7, 2.2, 14.3) mean
  8-bit levels against the original's (0.6, 4.8, 19.2), mostly nose, shoulders, wingtip rims
  (cyan-white glow there in the original). Cause unread.
- **The original barely follows the `sin` alpha flicker** (blue ratio 0.88 across `sin` +1 to -1,
  ours 0.46; one run per age, conf 70), although it computes it and writes it as the node tint.
  Dropping ours' flicker alone would overshoot, so nothing is wired.
- **Landed:** HD's per-team `ShieldColour` target (`Livery::shield_colour`,
  `Setup::shield_targets`), and the lattice's colour scroll (`u + 3 time`). Zone's two values are
  not a bug (normal Zone mode has no shield; Zone Battle on Fury may differ, unverified). Omega and
  2048: `omega-2048-shield-shell-and-palette.md`.
- `FUN_00125d98` is read (`ShipShield_DrawShell`); `FUN_00676fe8` is `sin`.
- Unwired in `hexagonalshield_rich`: `constantAmbientColour`, the lit variants' light terms, fog.
  Zone's `zonebattle_shield` program is unread (no mode of ours raises it).
- Our own `graphics.own_shield` reads its eye from the origin (`docs/rendering/own-shield.md`, "Open").

## Next Steps

1. Dump the shield draw from one frame's command ring (`scripts/hd-draw-dump.py`): blend factors
   and colour/alpha write mask (does it write the bloom's glow mask?), the vertex attribute node
   `+0xf0` feeds, and `constantAmbientColour`'s value. One capture should settle both open items.
2. Wire what that read shows as HD `Title` data, then re-score the two `hd-shield-*` pairs.
3. Only if Zone Battle (Fury) turns out to raise a shield: check which Zone copy it reads.
