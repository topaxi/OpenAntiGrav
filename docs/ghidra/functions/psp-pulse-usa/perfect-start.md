# The perfect start's effect, and the AI's grade 3

**2026-10-04, `pulse-start-leftovers` lane.** Binary `BOOT.BIN` of
`pulse-psp-usa.chd` (Ghidra `/pulse/BOOT-psp-pulse-usa.BIN`). Static reads; the
grader itself was watched on five launches by the `pulse-launch-boost` lane
([launch-boost.md](../../../physics/launch-boost.md),
[engine.md](engine.md)). Neither finding below was watched live.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x08904fd4` | `ExhaustFlare_OnPerfectStart` | 85 |
| `0x089044d8` | `ExhaustFlare_Construct` | 75 |

## `ExhaustFlare_OnPerfectStart` (`0x08904fd4`): a pad's flare and a Turbo's sound

`Race_UpdateLaunchGrade` (`0x0882773c`) walks the grid. In the perfect window
(`windowStart <= c < windowEnd`), for a craft whose player record is the local
human (`player+0x368 == 0`) and whose first-thrust edge is this frame
(`craft+0x2d5`), it writes grade 2 and, if the player's `+0x78 -> +0x78` is
non-null, calls this function with that object in `a0`
(`0x0882783c`-`0x0882784c`). No other caller exists.

```c
void ExhaustFlare_OnPerfectStart(flare) {
    flare->boost_timer = flare->boost_seconds;       // +0xb8 = +0xb4   (lwc1 f12,0xb4 / swc1 f12,0xb8)
    racer = flare->racer;                            // +0xc0
    if (racer) {
        if (racer->+0x368 == 0 && (debug || g_game_mode != 2))
            FUN_0883e9b0(racer, weapons_bnk, "TURBO", 0x400, 0);         // dry, full volume
        else
            Sound_Play(1.0, racer->emitter, weapons_bnk, 0, "TURBO", 0);  // positional, racer+0x50
    }
}
```

It is `ExhaustFlare_OnSpeedupPad` (`0x08904f10`, [pads.md](pads.md)) line for
line with two differences:

- **The duration is a field, but always 0.8.** The pad stores the literal
  `0x3f4ccccd`; this copies `flare+0xb4`. The only `swc1 ..., 0xb4(..)` in the
  flare's code range is `0x0890452c`, in `0x089044d8`, which installs the
  object's vtable and class tag, zeroes `+0x64`, `+0x78` and `+0x88` (the
  `~ENGINE` emitter and the plume clock [exhaust.md](exhaust.md) names) and
  stores `0x3f4ccccd` to `+0xb4`. Named `ExhaustFlare_Construct` at 75: the
  field layout matches the flare's, its one caller (`0x089045e0`) was not
  read.
- **The cue is `"TURBO"` from `weapons.bnk`** (`DAT_08ac1df8`, the bank the
  autopilot and the shield also play from, [autopilot.md](autopilot.md)),
  where the pad's is `"SPEEDUPPAD"` from `hud.bnk`.

`boost_timer` reaches the flare's size, the `<Team>boost.vex` plume's reveal
and the engine-on flag ([exhaust.md](exhaust.md#what-a-boost-actually-changes)),
so **a perfect start looks like a pad crossing and sounds like a Turbo**. It
is not a `.POB` effect. Only the human gets it: an AI craft's grade 3 (below)
never reaches this call.

**Ported**: `oag_raceplay::perfect_start` arms the flare with
`exhaust::BOOST_SECONDS` (the pad's `0.8`) and raises `Cue::Turbo`
(`CraftUnlessPlayer`, as `SPEEDUPPAD`) on the tick a human craft's grade
becomes `Perfect`. `TURBO` resolves on Pulse (3 waveforms) and HD (15) through
the loader. Tests: `launch_boost_ground_truth::only_a_perfect_start_fires_the_flare_and_turbo`
(disc), `race::tests::perfect_start`.

**Our fired Turbo plays `TURBO`, wired 2026-10-06.** `Turbo_Fire` (`0x088614c4`) sets
craft bit `0x200`, `craft+0x14c = <Turbo time>`, clears the held id and, when the
craft has an exhaust flare (`craft->0xf0->0x78`), calls `FUN_0890514c`: `flare+0xb8 =
0.8f` and `Sound_Play(TURBO)`. That is `ExhaustFlare_OnSpeedupPad`'s body with the
other cue, and `Ship_UpdateSideshiftInput_q` (`0x08846b60`) calls it too (the
perfect start). Confidence 88. So the plume a fired Turbo draws is the pad's, by the
original's own path - the `weapons.rs` note calling it chosen is retired.

## The AI's grade 3, confirmed from both ends

Read before ([engine.md](engine.md), 80) from the grader alone; now from the
reader as well:

- **Writer.** `Race_UpdateLaunchGrade`'s perfect-window loop writes
  `player+0x36c = 3` for every craft with `player+0x368 != 0`, on every frame of
  the window, **with no thrust-edge test** - the `+0x2d5` check sits only on the
  human branch. The stall window after it writes grade 1 only on an edge, so an
  AI that has not thrust keeps grade 3.
- **Reader.** `Ship_UpdateStartBoost` (`0x0883fdec`) case 3 sets
  `craft+0x294 = boostMul * *(g_race_manager + class * 0x10c + player+0x914 * 4 + 0x320)`,
  the per-grid-slot `StartBoost[8]` of the class's AI stats
  ([ai-stats.md](ai-stats.md)).

**Confidence 85** for the rule (both ends read, the case numbers agree). The
disc's per-slot figures on `AIRaceStats_Venom.xml` fall from the front of the
grid to the back and none exceeds 1.0, so the original's AI launches at
`boostMul` at the front and up to a quarter less at the back, with no thrust
timing at all.

**Not ported as read**, by the maintainer's rule that the AI obeys the
player's physics: a grade without a thrust edge and a multiplier no player can
earn are both outside it. What this project's AI may do is earn grade 2 the
player's way, by timing its first thrust into the perfect window; see
[launch-boost.md](../../../physics/launch-boost.md).

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0890514c` | `ExhaustFlare_OnTurbo` | 85 |
