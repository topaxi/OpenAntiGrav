---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-12
summary: "Pulse 0.01 body damping measured per frame, not flown: with it de Konstruct Black first jump falls short (Autopilot port, PHANTOM relaunch)"
size: M (50)
difficulty: medium (50)
---

# Pulse: the rigid body's velocity damping is measured and not flown

The law, every title's read and the per-frame measurement on Pulse:
`docs/physics/body-damping.md` (linear `0.01` live, confidence 90; angular in
`docs/physics/roll-step-response.md`, "The damping", 85). `BodyDamping::PULSE` exists;
`Laws::PULSE` carries `NONE`. Pure flies its pair since 2026-11-12.

## Done (2026-11-12, `pulse-body-damping`)

- `crates/trace/tests/body_damping_pose_walk_ground_truth.rs`: one step from each frame of the
  PSP held-throttle standing start, `+0.00961` of `speed * dt` off, `-0.00043` with
  `BodyDamping::PULSE`.
- `oag-trace` `replay::tests`' free-body self-checks step with `BodyDamping::NONE`.
- Pure's `CRAFT_LAWS` carry `0.01`/`0.01` (read).
- The eight tests the HD lane listed are diagnosed on the doc page ("What it moved"); four no
  longer move with the flip.

## Open

- **De Konstruct Black's first jump (`05_Track` forward, lip at line samples 205-208) falls short
  with the damping on**, two ways (doc page, "Why Pulse does not fly it yet"):
  - the original Autopilot's port, VENOM, on the authored magstrip line: 2 of 5 perturbed runs
    clean against 4 of 5 off (`autopilot_original_ground_truth::..._05` needs 3). Physics, not AI:
    what the craft keeps of its climb over the strip's convex top (ours `85` u/s of climb at peak,
    `62` once the lock lets go, then a unit or two under the lip at 112 u/s);
  - our PHANTOM field: a relaunched craft loops at sample 95 at 85 u/s, 19 dead stops and 23
    respawns against 2 and 4 off (`ai_dekonstruct_symptoms_ground_truth::de_konstruct_symptoms_phantom`,
    bound 6). Its drift puts it 8-9 units off the takeoff line, on the strip.
- **The reseeded crest of `talons-junction-clean-lap.csv`** lifts off one tick after the original
  with the damping (1596 against 1595); off it read half a tick late already. Residual, not a
  blocker.

## Next Steps

1. **Capture the magstrip takeoff on the original** (PPSSPP, Pulse PSP, `05_Track` forward,
   VENOM, player input: `psp-trace.py --place` a few hundred units before the ramp, throttle held,
   no steer, on the authored line). Record per frame the speed, climb (`vel_y`), the mag-lock state
   and whether it clears the lip; run ours under the same script (`oag-trace run`) with the damping
   on. If ours loses more climb over the strip's top than the original, that is the law to fix; if
   the original falls short too, the Autopilot port's rescue there is faithful and the test is
   re-based with that evidence. 2-4 hours.
2. **The drift on a takeoff run-up**: in `oag_ai::Driver::drift`, return no offset while
   `ctx.line.is_takeoff(self.index)`. With the damping on it took the PHANTOM field from 19 dead
   stops and 23 respawns to 3 and 8 (lone best lap 1,332 to 1,333 ticks). Measure it with the
   damping off too, and on the other classes and both de Konstruct layouts, before landing. 1 hour.
3. **The flip**: `Laws::PULSE.body_damping = BodyDamping::PULSE`, and in the same commit
   `launch_hover/tests.rs`'s Pulse arm to `PULSE`; `plasma_timing_ground_truth` pins tick 76
   (`ORIGINAL_DETONATION`), all seventeen `ORIGINAL_DISTANCE` rows at `0.25`, and drops the
   absolute `ORIGINAL_LAST_BEFORE_BLAST` check (doc page, "Plasma"); `hover_contact_ground_truth`
   expects `RECORDED_LIFTOFF + 1`, named as the open residual. Quote the lone-craft gate before and
   after (it stayed twelve clean, 0 respawns).
4. **The hashes**, in their own commit naming this law: `oag-physics` and `oag-ai` determinism,
   `oag-raceplay` `VANILLA_FOLD`.
