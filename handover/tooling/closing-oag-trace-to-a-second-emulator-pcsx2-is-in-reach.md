# Closing oag-trace to a second emulator: PCSX2 is done, RPCS3 has no transport, Vita3K/ShadPS4 have no tooling and ShadPS4's install is broken

Investigated 2026-09-16, priority order PCSX2 -> RPCS3 -> Vita3K -> ShadPS4 as asked.
`oag-trace`'s reading half (`crates/trace/`, `docs/tools/oag-trace.md`) is already
platform-agnostic - a PS2 pressing loads through the same `oag_assets::Archives`
path PSP does, same entry names, same 196 colliders. What is missing per platform
decomposes into four questions, and a different one blocks each:

- **(a)** scriptable per-tick breakpoint (or equivalent) + memory read
- **(b)** deterministic input injection
- **(c)** that binary's own craft/body struct offsets - a different binary, a
  different layout, `craft+0x290`/`body+0x150` etc. never transfer
- **(d)** does our own side implement the game at all, so a trace has something
  to diverge against

## PCSX2 (PS2 Pulse) - closed: a real capture works

All four are already in reasonable shape:

- **(a)/(b)** `scripts/pcsx2-drive.py` + `scripts/pcsx2_pine.py` talk to PCSX2's
  built-in PINE socket: memory read/write, savestates, and a **verified**
  frame-advance - `docs/reverse-engineering/pcsx2-debugger.md`'s "Deterministic
  capture" section measured two independent 60-tap runs from the same savestate
  landing on identical EE RAM hashes and pixel-identical (`AE = 0`) screenshots.
  This is strictly better determinism than PPSSPP's breakpoint approach gives -
  no per-emulator-version `--script-lead` guess needed the same way, though it
  still needs its own measurement once a real capture exists.
- **(c)** `docs/ghidra/functions/ps2-pulse-eu/craft-update.md` already has most
  of the body struct: basis rows `body+0x00/0x10/0x20`, position `body+0x30`,
  velocity `body+0x100`, angular rate `body+0x150`/`body+0x160`, grounded/contact
  `craft+0x1e0`/`craft+0x2e0`. Comparable richness to what `psp_trace_fields.py`
  needs, just never assembled into that shape.
- **(d)** Pulse's own physics/gameplay already run and are PSP-primary per
  [[project_psp_over_ps2]] - PS2 is corroboration, not a second ground truth,
  and the plan below should keep saying so rather than implying otherwise.

**Landed this session**: `scripts/pcsx2_trace_fields.py` and
`scripts/pcsx2-trace.py`, both compiling and argument-validated (no PCSX2
running to test a real capture against yet). Reading `craft-update.md` in
full surfaced three gaps the earlier pass's offset table didn't anticipate,
each written into the new files rather than papered over:

- **`steer` and `brake`'s ramped, persisted state have no craft offset
  anywhere on that page** - only local pseudocode variables, unlike
  `throttle` (which the "dead ramp" finding hands over almost by accident:
  `Ship_UpdateEngine` overwrites its own ramp with `controls->thrust`
  verbatim right before use, so the raw control input *is* the value the
  force law reads) and the two airbrakes (`craft+0x2d8`/`craft+0x2dc`, read
  directly by `Ship_ApplyLateralGrip`). `pcsx2-trace.py` writes both as `0.0`
  with a loud warning on every run and in the CSV's own header comment,
  rather than guessing an offset - the ADR-0005 confidence floor applied to
  data, not only to function names.
- **`dt` has no craft offset either, and PINE cannot read it the way a
  breakpoint can** - PINE exposes memory and savestates only, no CPU
  registers, so even a located offset would need a *known* pointer to read it
  from, which is the next bullet. `pcsx2-trace.py` writes a fixed `1/50`
  (PAL, confirmed live this session), stated as an assumption about what the
  game integrates, not a measurement of it.
- **No known static memory location holds the current race's craft
  pointer.** PPSSPP's capture gets one for free off the `a0` register at a
  breakpoint; PINE has no breakpoint. `pcsx2-trace.py` therefore takes
  `--craft <address>` as a required argument rather than discovering one -
  usable because PCSX2's savestate loading is bit-exact, so a craft found
  once under a given grid save stays valid for every capture that starts
  from that save, but someone has to find it by hand first (a live memory
  scan or diff, the same technique that found the frame counter).

None of these blocks the infrastructure - boot-to-grid, verified per-tick
frame-advance via `pcsx2-drive.py frames 1` shelled out to (not imported;
its `Keyboard`/`advance_frames` were never split into an importable sibling
the way `pcsx2_pine.py` is, so re-implementing them would be a second copy of
already-measured-deterministic code), batch-reading the confirmed
craft/body/controls fields via `pcsx2_pine.read_bytes`, and writing a
`REQUIRED_COLUMNS`-complete CSV - is all there and argument-validated. What
they block is a capture being *useful for verifying steering, braking, or
timing precisely* until a live PCSX2 session locates the three gaps above,
and *reproducible across sessions* until someone finds a craft address once.

**All three gaps above closed in a live session, 2026-09-16.** Full method
and citations are in `scripts/pcsx2_trace_fields.py` and
`docs/ghidra/functions/ps2-pulse-eu/craft-update.md`'s new "`steer` and
`brake`, found live rather than read" section; the short version:

- **Craft pointer found** via a thrust-vs-coast memory diff from the same
  savestate - PCSX2's frame-advance is deterministic, so everything except
  the player's own input cancels between the two runs, cutting 2.1M EE RAM
  words to 1,743 candidates. Confirmed three independent ways (orthonormal
  cached basis matching the body's own, hover height within 0.06% of the
  PSP's established range, all-zero controls at rest). Instance data, not
  repeated here - it's specific to one savestate.
- **`steer` (`craft+0x2f0`) and `brake` (`craft+0x2ec`) confirmed**,
  confidence 90 each, by holding inputs and watching the craft block ramp
  and decay exactly the way `Ship_UpdateSteering`/`Ship_UpdateBrakes`'s own
  pseudocode says it should. Both are now real reads in `pcsx2-trace.py`,
  not the `0.0` placeholder.
- **A real bug found in the process, not just an offset**: one PCSX2-verified
  frame is one PAL video *field* (50 Hz), not one game tick (25 Hz) - five
  independent fields read bit-identical across alternating steps. Half of
  every capture taken the original way was an exact duplicate row.
  `pcsx2-trace.py` now steps `PHYSICS_STEP_FRAMES = 2` per recorded tick;
  `dt` is `2/50`, still an assumption (PINE has no register read to confirm
  it against) but now scaled to the tick the capture actually samples, and
  the earlier single-step `dt` evidence is retracted as inconclusive - see
  `pcsx2-debugger.md`.

**First real capture**: `data/traces/ps2-moa-therma-white-thrust-smoketest.csv`
(gitignored). `oag-trace show` reports 15/15 clean ticks and `dt` uniform at
exactly 25.00 Hz - the first working PS2 trace this project has produced.

**Still open, lower priority:**

1. Name the frame counter (`0x0027a7e8`) now that its field-vs-frame nature
   is understood, and a front-end state word for scripted menu walks -
   neither blocks a trace.
2. Measure `--script-lead` for this transport against a real input-script
   capture (only a `--hold`-driven smoke test has run so far).
3. `dt`'s true nature (measured vs. fixed) is still unconfirmed at the source
   - the `2/50` value is corroborated by five fields at once rather than
   asserted, but nothing has read it off a register or a located craft field.

Rough size for the remainder: one live PCSX2 session to unblock a first real
capture, no further Rust or infrastructure work needed.

## RPCS3 (PS3 HD/Fury) - the transport itself has no capture channel

This one is not a smaller version of the PCSX2 gap - it is blocked at a lower
level. `docs/reverse-engineering/rpcs3-debugger.md`'s own "what is worth doing
next" section says it outright: **"A per-tick trace harness comparable to M3's
is a further step again and is not costed here: the stub is silent while the
target runs and costs 41 ms a packet while it is stopped, so there is no
capture channel on this transport at all."**

- **(a)** No. RPCS3's GDB stub answers nothing while the target runs (so a
  polling client hangs, not times out), costs ~41 ms/packet while paused
  (caps at ~24 ops/sec - no per-tick capture at 60 Hz is possible on this
  transport), and a `Z0` breakpoint only fires under `PPU Decoder: Interpreter
  (static)` and parks its thread **without sending a stop reply** - a negative
  from `wait_for_stop()` is not a negative, per the trap recorded in
  [[handover/tooling/a-ps3-title-now-boots-drives-and-screenshots]] (deleted
  once that thread's work lands; cited here as the primary source for this
  session).
- **(b)** Human/wall-clock-paced only (`just rpcs3-race` walks a scripted tap
  sequence and drives with a held button) - not frame-exact, no verified
  frame-advance equivalent to PCSX2's PINE path exists on this transport.
- **(c)** Thin. Only the throttle field is confirmed (`craft+0x30c`, confidence
  90, reproduced across three boots). Speed is confirmed **absent** from both
  scanned craft objects (three scans) - probably computed at draw time from a
  velocity reached elsewhere in the renderer. No steering field is identified;
  a 12-sample candidate at `entry+0x284` was refuted at 24 samples. Three of
  four craft vtable slots are unidentified.
- **(d)** Partial - HD/Fury "a race drives and draws textured" per the roadmap
  (2026-08-18, `[~]`), so there would be something to diverge against once (a)
  and (c) existed.

Savestates were tried and **rejected** for this purpose: ~60s to load, and they
restore to the Campaign screen rather than in-race, so they're further from a
usable start than a 45s cold boot - the opposite of PCSX2's savestate story.

**This is "far," not "days."** The rpcs3-debugger.md thread's own next-steps
(committed input script + a wall-clock-paced lap, `Start/Stop Recording` for
video as the only continuous observable this platform currently has, retesting
breakpoints against a provably-live address like `RaceManager_GetInstance`)
are all worth doing for their own sake, but none of them produces a per-tick
trace - the doc is explicit that a trace harness is "a further step again," on
top of work not yet done, over a transport that fundamentally can't sustain
60 Hz polling. Closing this gap needs either a faster RPCS3-side transport
than the GDB stub (unexplored - RPCS3 ships other debug interfaces that were
not evaluated here) or accepting a non-per-tick observable (video + a few
hand-placed savestate comparisons) as what "verification" means on this
platform. Worth a deliberate scope decision before investing further, not a
default "do the same thing as PCSX2."

## Vita3K (Vita 2048) - no driving tooling exists at all

Vita3K appears in this project exactly twice, both as a **read-only source
reference**, never run: its GPL-licensed USSE shader decoder is cited as an
oracle for `oag_rcs::gxp` (never transcribed, per this project's license bar),
and its `sce_utils.cpp` decrypt path is what `scripts/vita-self-decrypt.py`
reimplements. Nobody has booted 2048 in it. `docs/reverse-engineering/
toolchain.md`'s emulator-tooling coverage stops at PPSSPP/PCSX2/RPCS3.

- **(a)/(b)** Not started - no debugger bridge, no input-injection path, unknown
  whether Vita3K exposes anything scriptable (no equivalent of PINE or a GDB
  stub investigated here).
- **(c)** Not started - no `docs/ghidra/functions/vita-2048-eu-v104/` page names
  a per-tick craft/body struct; what exists there (`zone-environment-fallback.md`)
  is unrelated to physics state.
- **(d)** Yes, unlike the other two remaining platforms - 2048 races on our own
  side today (`just play 2048 --race`, roadmap `[~]` 2026-08-31, Altima drives
  and both craft and circuit draw), so a trace would have something real to
  compare against once the emulator side existed.

This is a from-scratch build: pick a transport (does Vita3K expose a debugger
API at all, or would this need a build with debug hooks added), find the craft
struct in Ghidra, write the capture script. No estimate given - the first
open question (does Vita3K have *any* scriptable interface) wasn't checked in
this session and decides everything downstream.

## ShadPS4 (Omega Collection PS4) - planned regardless of current roadmap scope

The roadmap does currently scope Omega Collection gameplay out - "no
gameplay/simulation work planned yet (still gated by this milestone's own exit
criterion)... Omega stays out of that scope," `docs/ghidra/functions/
ps4-omega-eu/README.md` - but that's a milestone-sequencing fact, not a reason
to skip planning this gap now; per direct instruction, this section plans it
like the other three rather than deferring on scope grounds.

- **(d)** is the one genuinely blocked question here: there is no
  reimplementation of Omega's gameplay yet, so a trace has nothing to diverge
  against until M8's exit criterion is otherwise met. That's a real dependency,
  not a policy choice to route around - the other three items below can still
  be investigated and even closed ahead of it.
- **(a)/(b)/(c)** genuinely not started, and the environment check below is
  more discouraging than the other two "not started" platforms.

**The installed `shadps4` binary is broken, not merely unconfigured** -
correcting what I reported earlier this session. `/usr/bin/shadps4` is a
**0-byte file** (`file` reports "empty", dated 2025-09-19), so `shadps4 --help`
exiting 0 with no output was the shell executing an empty file successfully,
not the emulator responding. Desktop entry, icons and a populated
`~/.local/share/shadPS4/` config (from a prior working install, `config.toml`
dated 2025-02-18) all exist, so this machine had a real install at some point
and the binary itself has since gone missing or been left empty by a broken
package update. **Reinstalling `shadps4` is a prerequisite for step 1 below**,
separate from and before any RE or scripting work.

The existing `config.toml`'s `[Debug]` section has only `DebugDump` and
`CollectShader` - no GDB-stub or remote-debug option is exposed anywhere in
this version's config, unlike PCSX2's built-in PINE or RPCS3's GDB stub. That
doesn't mean no such transport exists (ShadPS4 upstream has added a GDB debug
stub as a build option in some versions - not confirmed against whatever
version was installed here, and unconfirmable against a 0-byte binary), only
that it isn't on by default and needs checking fresh once a working build is
in hand.

The Omega Collection PKG itself **is** fully present and verified
(`data/images/omega-ps4-eu.pkg`, 23.9 GB, `.sha256` written 2026-09-15), so the
disc-image side of this is ready.

**Plan, in order:**

1. Reinstall/rebuild `shadps4` (or fetch a fresh AppImage/binary - whatever
   this machine's prior install used) and confirm it actually boots the Omega
   PKG at all, headless or otherwise. Nothing below can be checked against a
   0-byte file.
2. Check the working build for a GDB stub or any other scriptable debug
   transport - a compile-time flag, a settings toggle not in the current
   `config.toml`, or a network port opened at boot (`ss -ltnp` while it runs).
   This is the same first question Vita3K has and decides everything
   downstream the same way.
3. If a transport exists, locate the PS4 build's own craft/body struct in
   Ghidra (GhidraOrbis is already built and importing per
   `docs/reverse-engineering/toolchain.md`'s PS4 section, and
   `docs/ghidra/functions/ps4-omega-eu/` already has some function-level RE
   to build from, though nothing struct-shaped yet).
4. Building a comparable simulation to trace against is real work of its own
   and is the actual long pole - not urgent to start until M8 opens it, but
   steps 1-3 don't need to wait on that and can run independently.

## Open

- PCSX2: closed - a real capture works, `steer`/`brake` are confirmed and
  read live, and the field-vs-frame bug is fixed. What's left is lower
  priority: naming the frame counter, an input-script capture (only `--hold`
  has been smoke-tested), and `dt`'s true nature at the source.
- RPCS3: no capture channel exists on the GDB-stub transport at 60 Hz; closing
  this needs either a different RPCS3-side debug interface (unevaluated) or a
  scope decision to accept a non-per-tick verification story on this platform.
- Vita3K: unknown whether it exposes any scriptable interface at all - the
  first question, unanswered, that everything else depends on.
- ShadPS4: the installed binary is a 0-byte file, not a working emulator -
  reinstall is a prerequisite before anything else on this platform can be
  checked, independent of M8's exit criterion blocking step (d).

## Next Steps

- PCSX2 is done for now. Remaining, lower-priority follow-ups: name the frame
  counter, run an input-script-driven capture (`--script`, not just `--hold`)
  and measure `--script-lead` off it, and pin down `dt`'s true nature at the
  source if it ever blocks a real comparison.
- For RPCS3, before writing any code: check whether RPCS3 exposes a debug
  interface other than the GDB stub (its own scripting/Cheat Engine style API,
  a PINE-equivalent, anything not bound by the 41 ms/packet-while-paused,
  silent-while-running limits measured here) - that answer decides whether a
  per-tick trace is possible on this platform at all.
- For Vita3K, the first and only question worth spending time on before
  anything else: does it expose any scriptable/debug interface, at all.
- For ShadPS4, reinstall the emulator first, then run the same "does it expose
  a debug transport" check Vita3K needs - it can happen in parallel with the
  other three and does not need to wait on M8. The disc image
  (`data/images/omega-ps4-eu.pkg`) is already present and verified.

## From the HANDOVER.md index (moved 2026-09-25)

**PCSX2 is closed: a real capture works.** A live session found the player craft pointer by diffing EE RAM between a thrust-held and a nothing-held run from the same savestate - PCSX2's deterministic frame-advance cancels everything but player input, cutting 2.1M candidate words to 1,743 - then confirmed `steer` (`craft+0x2f0`) and `brake` (`craft+0x2ec`) live, both confidence 90, by watching them ramp and decay exactly as `Ship_UpdateSteering`/`Ship_UpdateBrakes`'s pseudocode says. It also found a real bug along the way: one PCSX2-verified frame is one PAL video *field* (50 Hz), not one game tick (25 Hz), so the original one-step-per-row design silently wrote a duplicate row every other tick - fixed by stepping two verified frames per recorded tick. First capture: `data/traces/ps2-moa-therma-white-thrust-smoketest.csv`, `oag-trace show` reports 15/15 clean ticks at a uniform 25.00 Hz. Also now mutes PCSX2's audio for every automated run. Remaining, lower priority: name the frame counter, run an input-script capture (only `--hold` has been tested), and `dt` is still an assumption (`2/50`, corroborated by five fields at once but never read off a register - PINE has none). **RPCS3 is blocked at the transport level, not the RE level**: its own debugger doc says outright there is no capture channel on the GDB-stub transport at 60 Hz (silent while running, 41 ms/packet while paused, ~24 ops/sec ceiling); closing it needs either an unevaluated alternate debug interface or a deliberate scope decision to accept a non-per-tick verification story. **Vita3K has zero driving tooling** - it's only ever been used as a read-only source reference (its USSE decoder, its PKG-decrypt code), never run, and whether it exposes any scriptable interface at all is the first unanswered question everything else depends on. **ShadPS4's installed binary turned out to be a 0-byte file** (correcting an in-session claim that it was installed and working) - a prerequisite reinstall blocks even checking whether it has a debug transport, separate from the roadmap's own M8 gate on Omega gameplay work existing at all to trace against.
