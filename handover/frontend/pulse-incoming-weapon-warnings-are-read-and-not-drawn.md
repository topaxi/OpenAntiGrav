---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-11
summary: "Pulse's ForwardWarning/RearWarning HUD icons: raise law (per-weapon along-track window, 2.0 s, 4 Hz blink) measured live; wire it"
---

# Pulse's incoming-weapon warnings are read and not drawn

2026-11-11, lane `hud-warnings-re`. The law is on
`docs/ghidra/functions/psp-pulse-usa/hud-incoming-warnings.md` (95, live through the
game's own fire path). Nothing in our HUD draws `ForwardWarningIcons` /
`RearWarningIcons` yet (`docs/ui/hud.md`, "Warnings" row). No Ghidra needed to wire.

## The law, as a wire lane needs it

State per local player's HUD: two slots, `forward` and `rear`, each
`{ timer: f32, idx: Option<u8> }`, both start `0.0` / none.

**Raise** (on the tick a craft's held weapon is fired, i.e. where
`oag_raceplay`'s `spend_pickup` (`crates/raceplay/src/weapons.rs`) consumes it):
skip if the firer is the local player, if the weapons-on switch is off, or if the
race mode is not Arcade/Single race, Eliminator, Tournament or their multiplayer
forms (Time Trial, Free Play, Zone: never). Let
`g = progress(player) - progress(firer)`, wrapped into `+/- half` a lap. That is
`-oag_ai::original::gap_along_track(player, firer, lap)` (that helper returns
`other - player`; mind the sign). Skip if `|g| > 800`. Then:

| Weapon | Window (strict) | Slot | Widget |
| --- | --- | --- | --- |
| Rocket | `0 < g < 400` | rear | `RearWarningRocketIcon` |
| Missile | `0 < g < 400` | rear | `RearWarningMissileIcon` |
| Quake | `0 < g < 800` | rear | `RearWarningQuakeIcon` |
| Cannon | `0 < g < 500` | rear | `RearWarningCannonIcon` |
| Plasma | `0 < g < 500` | rear | `RearWarningPlasmaIcon` |
| LeachBeam | `0 < g < 500` | rear | `RearWarningLeachBeamIcon` |
| Repulser | `0 < g < 500` | rear | `RearWarningRepulserIcon` |
| Shuriken | `0 < g < 500` | rear | `RearWarningShurikenIcon` |
| Mine | `-500 < g < 0` | forward | `ForwardWarningMineIcon` |
| Bomb | `-500 < g < 0` | forward | `ForwardWarningBombIcon` |
| Turbo, Shield, Autopilot | never | - | - |

Inside the window: that slot's `idx = weapon`, `timer = 2.0` (overwrites a running
one). Target, lock-on and the projectile's later flight are never read.

**Update** (every HUD frame, `dt` in seconds): for each slot, compute
`on = (trunc(timer * 8) & 1) == 1` from the timer *before* decrementing; if
`timer > 0` then `timer = max(timer - dt, 0)` and the slot's background
(`ForwardWarningBackground` / `RearWarningBackground`) is drawn iff `on`; else the
background is hidden and `idx = none`. Each icon is drawn iff it is its slot's `idx`
and that slot's `on`. Result: 4 Hz blink, first 0.125 s off, eight on phases, ends
off.

The HUD is a view: the raise belongs in the per-tick output the race already
produces (a cue-like event "craft `k` fired weapon `w`" or the slot state itself),
not in `World` gameplay state if a view-side derivation works; the timers are HUD
state. Ours is fixed 60 Hz; the original's `dt` is the frame time (0.016-0.017
live), so 120 ticks per warning.

## Where to wire

- Fire site: `spend_pickup` (`crates/raceplay/src/weapons.rs`), one arm per weapon.
- Gap: `oag_ai::original::gap_along_track` (`crates/ai/src/original/cruise.rs`).
- HUD: the widgets are already in the layout model (`crates/hud`, read from
  `Arcade_HUD.xml` and friends); draw them like `TimeDiffIcon`
  (`crates/hud/src/draw.rs`, `cx.layout.sprite(...)`), by name.
- Tests that fail if the law is dropped: a Rocket fired 399 behind raises the rear
  slot and 401 does not; a Mine 100 ahead raises the forward slot; the player's own
  fire never raises; the blink schedule is off 2 frames, then on 8 / off 7-8, and the
  slot clears to none after 2.0 s.

## Open

- What makes the original's gap function (`0x0883dbf4`) refuse a pair (its flag
  byte zero). Never seen live; a craft not yet on the course is the guess. Wiring
  can treat every racing craft as valid.
- PS2: inherited from PSP (same widget names).
- HD/Fury: same widgets, backgrounds steady while the timer runs, icons blink;
  raise and blink multiplier unread (`hud-incoming-warnings.md`, "Lineage"). Pure,
  2048, Omega: see the same section; not part of this wire.

## Next Steps

1. Wire Pulse PSP (and PS2 inherited) per the table above, with the tests listed.
2. Judge it in a race with weapons on: an opponent's Rocket from just behind blinks
   the rear icon right of the time-gap line.
