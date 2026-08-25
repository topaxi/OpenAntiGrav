# The Autopilot pickup is built off its own two handlers; three of its parts are still ours

2026-08-24. The mechanic, the confidences and what was not found are on [autopilot.md](../docs/ghidra/functions/psp-pulse-usa/autopilot.md); this row is what is not there. **Nothing is runtime-verified** - one PPSSPP breakpoint on `Autopilot_Fire` (`0x088613bc`) with the pickup collected settles the duration and the teardown in a single run. **Two cues sit decoded and unwired**, both real: `~AUTOPILOT` (`hud.bnk`, held loop, 2.43 s) is *stopped* by the handler at `entity+0x58` and nothing located opens it, and `autopilot_eng` (`speech.bnk`, 1.28 s) has no call site at all. The search that closes the first is spelled out on the page, including the false positive it has. **Where the original swaps the control source was not found**, so the takeover is this project's reuse of `oag_ai::Driver`; `Ai_Construct` naming the local player's input `"autopilot input"` is as close as the reading gets. **A sixth `racer+0x368` use** - a craft-explosion spawner shakes the camera and plays dry only when it is zero - takes the "is the local player" hypothesis to **65**, still unnamed because no write site exists anywhere; `Placement::CraftUnlessPlayer` and `OPPONENT_ENGINE_SCALE` are the lines that move if it is wrong. **The same branch is a lead for the untraced camera shake**: `FUN_08878750`, unnamed, on the page.

## Open

- Nothing about the Autopilot pickup is runtime-verified yet
- `~AUTOPILOT` is stopped by the handler at `entity+0x58` but nothing located opens it, and `autopilot_eng` has no call site at all
- Where the original swaps the control source was never found - the takeover here is this project's own reuse of `oag_ai::Driver`
- The "is the local player" hypothesis sits at confidence 65 off the `racer+0x368` use, still unnamed because no write site exists

## Next Steps

- Set a PPSSPP breakpoint on `Autopilot_Fire` (`0x088613bc`) and collect the pickup to settle the duration and teardown in one run
- Follow the search spelled out on `autopilot.md` to find what opens `~AUTOPILOT` (it has a known false positive to watch for)
- Trace `FUN_08878750` (the untraced camera shake) as a lead off the same `racer+0x368` branch
