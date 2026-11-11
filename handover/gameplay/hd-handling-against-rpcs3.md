---
status: ready-to-wire
titles: [hd]
updated: 2026-11-10
summary: "HD late speed closed (body damping 0.01); open: AI field on Talon's (808 contact ticks), Omega craft laws, 2048 integrator read"
size: M (40)
difficulty: medium (40)
---

# HD handling against RPCS3: the AI field and Omega's craft laws

2026-10-08, `hd-handling`; 2026-10-09, `hd-pitch-airbrake`, `hd-airbrake`. Results:
`docs/physics/hd-handling-ground-truth.md`. Tool: `scripts/rpcs3-trace.py`
(`docs/reverse-engineering/rpcs3-capture.md`, "A per-frame craft trace"). HD functions:
`docs/ghidra/functions/ps3-hdfury-eu/physics.md`, "The handling update", and
`docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md` and `craft-inertia.md`.

Thrust, the airbrake ramp and sideshift match Pulse's laws on HD's own tables; steering does with
HD's ramp clamp and inertia (below). ~~Pitch settles 30% shallower~~: closed 2026-10-09. HD hovers on four probes, `0.15` each,
all cast every frame and pushed along the hit normal (`Craft_HoverFourPoint`, `0x000ede88`); now
`oag_hd::race::HOVER_RIG`, and ours settles `+6.25` / `-2.99` deg against HD's `+6.3` / `-2.8`.

~~One airbrake turns HD 9-14% less~~: closed 2026-10-09 (`hd-airbrake`). HD builds the craft's
box inertia with mass `1.0` (`I_yy` 24, `Ship_Construct` `0x000debc8`) where Pulse passes `0.9`,
and HD's steering ramp stops at its target where Pulse's cycles `83.3/91.7/100`; the old steering
match was the two cancelling. Both are `oag_hd::race::CRAFT_LAWS`; one airbrake `-11.54` against
HD `-11.3` at 0.5 s, steering `-46.28` against `-46.2` at 0.8 s, yaw rate within 1.6% every frame.
`oag-game --trace-out` now writes `omega_*`/`avel_*`.

## Open

- **HD's AI field on Talon's Junction** (`hd-ai-talon`, 2026-10-09): the lone Ace's 6 contact
  ticks were at line samples 3186-3190, a corner the correct plan clears by under a unit, and the
  Ace's 7 % plan slack put it 0.3-0.6 units wider. Fixed: the plan marks such samples tight and a
  driver spends no level slack within 200 units of one; every HD lone row is 0 contacts. **Still
  open: the field**, 808 contact ticks over five seeds against 642 before the craft-law fix (788
  after it), at samples 2700-2749 and 3150-3199, where traffic hands character back. Zeroing the
  traffic term there too gave 505 but put `craft_sticking_ground_truth` at 2,002 against its
  2,000 limit. Numbers and both rejected variants: `docs/gameplay/ai.md`, "Tight corners".
- **The AI's steering loop leaves lock unused while running wide** (measured on the same corner,
  2026-10-09): over Talon's samples 3080-3190 the steering ramp sits at -60 to -82 of 100 and the
  yaw rate at 1.30-1.35 rad/s while the craft is 12-14.6 units outside the line; the rate loop's
  pure-pursuit aim does not ask harder as the craft drifts. Not pursued; a smarter-driving lever
  that costs no speed.
- **`oag-trace`'s own spawns** (`crates/trace/src/main.rs`, the two `box_inertia()` sites) still
  give every craft Pulse's tensor and unclamped ramp. Harmless while it replays Pulse captures; an
  HD replay there would need `CraftLaws` passed in.
- **Omega's craft laws**: not checkable in this pass, not located statically: no `12.0f` immediate near the craft code, no steering
  ramp among the `vminss` users in `0x01310000`-`0x01330000`. Omega keeps Pulse's (`craft_laws:
  None`). 2048 not checked.
  **2026-11-11 (craft-laws):** the inertia half is closed on both: Omega's ship constructors
  (`FUN_01309e90`, `FUN_0130e540`) store the folded mass-`1.0` tensor and 2048 calls the box
  builder with `1.0` (`docs/ghidra/functions/ps4-omega-eu/craft-laws.md`); both titles carry
  `CRAFT_LAWS` now. The steering ramp is still unread on both.
- ~~Speed past 2 s is about 1% low on HD~~: closed 2026-11-10 (`hd-late-speed`). HD's
  `Body_Integrate` takes `0.01` per second off the linear and angular velocity after the forces
  (`0x000f6fd8`, `0x000f70f8`; `Body_Init` `0x000f5878` writes the pair), Pulse's law, read on
  Pulse long ago and never flown. Measured: one step of ours from 203 HD frames lands
  `0.0100 x dt x speed` fast without it and `0.00005` units/s off with it; open loop, ours is
  within `0.04` of HD on every frame from 2.0 to 3.5 s (was `+0.61`..`+0.86`). Flown on HD,
  Omega (read off its own executable) and 2048 (inherited) as `oag_physics::laws::BodyDamping`;
  Pulse and Pure have the law and do not fly it yet (`handover/gameplay/pulse-body-damping.md`).
  Evidence: `docs/ghidra/functions/ps3-hdfury-eu/body-damping.md`.
- **2048's integrator is not read for the body damping** (`hd-late-speed`, 2026-11-10): it
  flies HD's `0.01` by lineage. Omega's was found by the packed `0.01, 0.01` store
  (`mov rcx, 0x3c23d70a3c23d70a`) into the body and the reader beside the speed store; the same
  census on `/2048/eboot-vita-2048-eu-v104.elf` is the check.
- **An idle craft on HD's Talon's Junction grid yaws** (seen 2026-11-10, not pursued): ours'
  forward `z` drifts from `+0.003` at tick 280 to `-0.023` by tick 700 with nothing held, where
  HD's craft sat at `-0.003`..`-0.006` through a 10 s run. Enough to put ours into the right
  wall 3.1 s into a held-throttle run that HD flies clean past 3.5 s.
- **Omega**: `Craft_UpdateSurfaceProbes` (`0x0131b510`) marches HD's four hull segments; its
  spring share and offsets are unread, so Omega keeps `hover_rig: None`. 2048 not checked.
- ~~HD's grounded craft carries a vertical residual our hover does not explain~~: closed
  2026-10-20 (`hd-hover-residual`). It was the pose walk seeding every row with the grid's hover
  cap; released, the grounded median is `dv_up -0.010`. The ride height matches too, ramp
  included. Found on the way and ported: HD sums its hover escapes (`HoverRig::sum_escapes`).
  Evidence: `docs/ghidra/functions/ps3-hdfury-eu/hover-four-point.md`, "The grounded vertical
  residual was the harness" and "The escapes sum"; `docs/physics/hd-ride-height.md`.
- ~~HD's air laws rest on one Autopilot run~~: closed 2026-10-20 (`hd-air-player-input`).
  Re-measured with player input alone (placed Time Trial runs, throttle held, the Autopilot bit
  clear on all 3,875 rows): class gravity on flight gravity (PHANTOM and Venom), the in-air bias
  as each team's own `+0x9c` (Feisar and Harimau `-4`, Assegai `+4`), the escape sum (131 rows),
  and the pitch gate seen closing at `0.9` for the first time. All four stand, confidence 90-92.
  Chenghou's "2 units low" takeoff was the sweep placing ours under the hover cap's release
  ramp. Evidence: `docs/physics/airborne-ground-truth.md`, "Wipeout HD under player input".
- ~~**A forward residual in the pose walk**~~ closed 2026-10-21 (`hd-throttle-and-flags`): the
  captures ran with Pilot Assist on, so `craft+0x30c` reads `88` against a held `100`, and the
  walk stepped with `100`. Stepping with the row's throttle, the median forward residual per
  frame is `+0.02..+0.03` airborne and `+0.02..+0.04` grounded on PHANTOM and Venom (was
  `+0.13..+0.14` and `+0.15..+0.45`). It was the walk, not a law, and not the late-speed item.
- **Not measured**: barrel roll, weapon slowdown, sideshift's gesture timing (as before).
- HD's hover pieces not ported: the trailing 10-unit ray (`0x000ee510`), the escape's class
  gates (material type `2` skips a probe, a full magnet lock skips the escape), `craft+0x358`.
  None is needed to close the grounded median; a `+0.015` position bias stays on escape rows.
- **Omega and 2048 escapes**: they fly Pulse's two-point rig until their springs are read. That
  rig sums its escapes too since 2026-10-21 (Pulse's own loop read and measured,
  `docs/physics/airborne-ground-truth.md`), so both now sum by inheritance; their own loops are
  unread.

## Next Steps

1. HD's AI field on Talon's Junction: traffic near a tight sample. A traffic share that keeps the
   sticking ratchet (1,819 with the level term only, 2,002 with traffic zeroed too) and brings
   the field to 642 or under; `hd_ai_craft_laws_ground_truth::field_on_talons_junction`, run with
   `OAG_SWEEP=1` (print-only, so gated out of `just test-data` since 2026-10-12). 1-2
   hours.
2. Omega: reach the craft update through `Craft_UpdateSurfaceProbes`'s vtable slot, then read its
   steering ramp and its `Body_SetBoxInertia` call's mass. 1-2 hours.
3. Read Omega's spring (the reader of `+0x330..+0x450`) to wire its rig, and whether its escapes
   sum as HD's do (`HoverRig::sum_escapes`). 1 hour.
4. 2048: read its integrator for the body damping pair (above). 30 minutes.
5. The idle yaw drift on HD's grid (above): find which hover term turns a resting craft. 1 hour.
