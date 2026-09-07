# HD's cues end loud and were cut dead, and its `.COLLISIONS` is a flattened tree

Reported from play on 2026-08-31: a thump *"a bit like a bad bass drum"*,
**only on Wipeout HD** - not reproducible on Pulse. Every few seconds, more
often around collisions, **not on contact but right after the contact effect
finished**, and **not visible in a `--tap-audio` recording** or in any of the
five `Health` counters.

## What it was

**`Mixer::render` cleared a finished voice's slot outright, and HD's cues do
not end near zero.** Measured off `hdfury-ps3-eu-dec.iso` and
`pulse-psp-eu.chd` through a real `Mixer` - the amplitude of the last sample a
voice emits before its slot is cleared, which is the size of the step the mix
takes when the cue ends:

| | waveforms | worst step into silence | over 0.1 |
| --- | --- | --- | --- |
| Pulse, every cue | 23 | **0.019** | 0 |
| HD `.COLLISIONS` | 112 | **0.766** | **102** |
| HD `SPEEDUPPAD` | 7 | **0.804** | 7 |
| HD `ABSORB` | 6 | 0.459 | 4 |
| HD `~BLOWUP` | 3 | 0.383 | 3 |

Every Pulse cue decays to silence on its own, which is why the same code
sounded clean on Pulse for as long as it has existed. HD's do not: they stop
mid-signal, at up to four fifths of full scale, and the mixer cut them there.
That step is a broadband click, and a click through a speaker is heard as a
thump - *after* the effect, exactly as reported. It is very likely what
`__sceSasSetADSR`'s release envelope covers on the real hardware, which
[ADR-0018](../../docs/architecture/adr/0018-audio-mixer-architecture.md) defers.

**The fix took two goes, and the first one is the instructive half.** It held
the finished voice's last sample and decayed *that* over 64 frames, which took
HD's worst step from 0.804 to 0.013 - and was still audible, correctly reported
as "less loud". Holding a value and ramping it to zero is a **unipolar pulse**,
which is a low-frequency thump in its own right, and lengthening the ramp only
moves the pulse deeper into the bass. There is no length that fixes it.

What works is ramping the **waveform** instead: the closing fade is applied to
the last `RELEASE_FRAMES` of the source, so the signal keeps oscillating while
its envelope falls and nothing is added after the cue. That has no floor, so the
fade can be 256 frames (~5.8 ms) rather than 64, bounded to a quarter of the
source so a short cue is shortened rather than swallowed. Measured after: HD's
worst step into silence is **0.0031**, down from 0.804 - and no cue on either
title is over 0.003.

## The other HD finding, unfixed

**`Banks::pick` chooses uniformly among whatever a cue resolves to, and HD's
`.COLLISIONS` does not resolve to alternates.** It binds nothing itself and
plays `c_CShipShip` and `c_CShipWall`, each with Small/Medium/Large children of
its own - **112 waveforms**. Measured off `hdfury-ps3-eu-dec.iso`:

| | waveforms | lengths | loudest low-frequency member |
| --- | --- | --- | --- |
| Pulse `.COLLISIONS` | 15 | 0.204s - 0.350s (1.7x) | bass peak 0.862, 43% under 120 Hz |
| HD `.COLLISIONS` | 112 | **0.410s - 3.266s (8x)** | 1.397 s, peak 0.957, **bass peak 0.809, 49% under 120 Hz** |

Pulse's fifteen are takes of one event - `sfx_ground_truth.rs` already asserts
they span under 0.2 s, "a cue whose commands were meant to play *together*
would be a stack of different lengths". HD's 112 are a **tree**, and picking
uniformly from it plays a 1.4-second heavy impact, half its energy below
120 Hz, for a light graze. That is the bass drum.

`SPEEDUPPAD` (7 waveforms, 0.547s-2.140s) and `~SHIELD` (6, 0.501s-1.232s) are
the same shape on HD and presumably the same bug.

**And it is inaudible on Pulse for a second reason: HD has no engine sound at
all.** `sfx: ~ENGINE not loaded: "~ENGINE" names no cue in shipHD`. Pulse holds
eight `~ENGINE` voices for the whole race; HD holds none, so its race mix is
music and occasional cues and nothing else. Measured over 90 s of the same
autopilot race, dumped with `--dump-audio`:

| | peak | median sample step | low-frequency onsets over 90 s |
| --- | --- | --- | --- |
| Pulse | 0.384 | 0.0140 | ~0.05 rises |
| HD | 0.278 | **0.0010** | **~0.10 rises**, 14 of them |

HD's mix is fourteen times less dense and its bass events are twice the size.
Every cue lands in near-silence. The same wrong pick on Pulse is buried under
eight engines; on HD it is the loudest thing in the race.

The 14 onsets repeat on a **42-second period** - one autopilot lap - which is
what a track-position-triggered cue looks like and what a periodic glitch does
not.

## What landed here

Only the report, deliberately: `Banks::load` now says when a cue's waveforms
are too unalike to be alternates (`not_one_event`, more than four waveforms
spanning over 2x). It fires on HD's three and is silent on every Pulse cue.
**Choosing correctly is reverse-engineering, not a patch** - see below.

## Open

**Which child the original picks, and on what.** The tree is
`.COLLISIONS -> {c_CShipShip, c_CShipWall} -> {S, M, L} -> takes`. Two
selections are needed and neither is recovered:

- **Surface.** `c_CShipShip` against `c_CShipWall` - the names are unambiguous
  and the engine's own contact path already distinguishes craft-to-craft from
  craft-to-wall, so this one is reading data rather than inventing. What is not
  established is whether the cue is fired once per contact kind or whether
  `ShipCollisionFx_Trigger` (`0x089246b4`) picks the parent itself.
- **Severity.** S/M/L against what - impact speed, normal component, damage
  dealt? A threshold invented here is exactly the guess-dressed-as-behaviour
  `CLAUDE.md` forbids, and it is why nothing was changed.

`oag_formats::sblk::child` already parses the tree, so the data side is done;
what is missing is the rule.

**`~ENGINE` on HD is a separate gap**, and a bigger one for how a race sounds:
HD's ship bank has no cue by that name. Either it is named something else in
`shipHD` or HD drives its engine from somewhere `oag_hd`'s bank table does not
point at. Nothing here looked.

## Next Steps

1. **Confirm the thump is gone by ear**: `just play hd`. That is the only test
   this session could not run, and the fade is measured but unheard.
2. Recover the parent selection - it is the half with named evidence, and
   halving the tree removes ship-to-ship impacts from wall scrapes, which is
   where the worst of the 112 live.
3. Then the severity rule, from `ShipCollisionFx_Trigger` and whatever it reads
   off the contact.
4. Find HD's engine cue. A race with no engine is a bigger fidelity gap than
   the one above and it is what made this one audible.

## What was ruled out on the way, so nobody tests it twice

- **Mixer-lock contention.** Real, fixed ([ADR-0031](../../docs/architecture/adr/0031-wait-briefly-for-the-mixer-lock.md)),
  and not this: the counter stays at zero.
- **Device underruns.** Real at a 512-frame buffer - callbacks up to 96 ms late
  with ALSA reporting xruns - and mitigated by asking for 40 ms instead. Not
  this either: the thump survived and the counters went quiet.
- **PipeWire without real-time priority.** True of this machine (`data-loop.0`
  is SCHED_OTHER, no rtkit, no `realtime` group) and not the cause; heavier
  games on the same machine are clean.
- **cpal's `realtime` feature.** Tried and reverted. On Linux
  `audio_thread_priority` promotes only through rtkit over D-Bus
  (`rt_linux.rs:92`) and cpal takes the crate with default features off, so the
  plain feature compiles to a no-op; the one that works pulls libdbus in and
  still does nothing without rtkit.
- **`~ENGINE`'s loop seam.** 0.002 of full scale against a median step of 0.055
  in the same bed. The sample loops over its whole decoded buffer cleanly.
- **The PS-ADPCM per-block loop flags.** `~ENGINE` is flagged
  `0, 6, 2 x 1903, 3, 255`. Honouring them - loop 28..53,368 rather than
  0..53,396 - was implemented, measured at **sixty times worse** (0.176 against
  0.002), and reverted. The marks do not point at the seam the waveform has.
- **A step in the mix, as the counter measured it.** `Health::scan` walks every
  frame of every buffer and its worst step never approached a quarter of full
  scale in a race - and the fault *was* a step, of up to 0.8. The counter
  missed it because a cue ending is one step in one buffer against a race
  running at millions of samples a second, and `QUIET_STEP` only reports a
  window's worst; a single 0.8 step in a window whose other content already
  moves 0.287 does show up, but only if the window happens to be reported.
  **The measurement that found it was offline**, comparing every cue's last
  sample across two titles - not a counter in a running game.
