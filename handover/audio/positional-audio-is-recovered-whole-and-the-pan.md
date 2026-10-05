# Positional audio is recovered whole, and the pan is a table the disc computes from `cos` and `sin`

2026-08-24, runtime-verified 2026-09-01. The law, the emitter layout, the
cross-title scan and every confidence are on
[positional-audio.md](../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md),
which is the durable record; this row is what is not there. **The cheapest
upgrade this project had open is done, and so is its own follow-up**: a live
Pulse USA capture (`scripts/psp-watch-soundemitter.py`, swapping one execution
breakpoint between `SoundEmitter_ComputeVolumeAndAngle`'s entry and return
address, since PPSSPP v1.20.4 only ever fires the most-recently-armed one)
reproduced the recovered volume/angle law exactly on **2,910/2,910 live
hits**, across two sessions. Confidences moved **85-88 -> 90-94** across the
emitter chain; see the doc's own
[runtime-verification section](../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md#runtime-verification-2026-09-01)
for the per-claim breakdown. **Two things fell out for free**: the doppler
scale `inst[0x0c]`, previously read as a field and never as a value, read
exactly `0.0005` on every hit (against the engine note only - not yet checked
against other cues); and the cone-enabled flag read `False` on all 1,610
samples of the first session, real negative evidence rather than "not looked
at". **The `d > radius` zero-volume gate branch never fired live even once,
deliberately targeting an emitter that sat queued and out of range for over a
minute - and that turned out not to be a sampling gap.** Decompiling
`SoundManager_Update`'s full per-emitter dispatch found the actual gate one
level up: `SoundEmitter_ServiceRequests` (newly named, `0x089394dc`) refuses
to call into `SoundInstance_UpdateSpatial` at all once an emitter's
out-of-range latch is set, and `SoundEmitter_Update` sets that latch in the
same tick distance first reaches radius, before the dispatcher runs - so
`SoundEmitter_ComputeVolumeAndAngle`'s own `d > radius` line is provably
unreachable, not merely unobserved. See
["The zero-volume gate is dead code"](../../docs/ghidra/functions/psp-pulse-usa/positional-audio.md#the-zero-volume-gate-is-dead-code-2026-09-01).
**Four things landed as side effects and are easy to lose**: [camera.md](../../docs/ghidra/functions/psp-pulse-usa/camera.md)'s
transposition finding moved **80 -> 88** (a second subsystem assumes both
halves); [pads.md](../../docs/ghidra/functions/psp-pulse-usa/pads.md)'s
`racer+0x368` moved **45 -> 60 -> 85 and is now named `controller_class`**
(2026-09-01: found the write site, `Craft_Construct_q`'s own second argument,
then cross-checked live against all eight racers in a single race - `0` on
the human-controlled craft, `2` on all seven AI, no other value seen), with
`oag_sound::sfx::Placement::CraftUnlessPlayer` now confirmed rather than
merely uncontradicted; exhaust.md's "world position at `+0x50`" was **wrong**
and is corrected to a scene-node pointer; and reaching craft fields from a
`Ship_UpdateCraft` breakpoint needs a dereference through `+0x1c4` first (the
"craft" this page and camera.md mean is that entity, not `Ship_UpdateCraft`'s
own `a0`) - already documented in `scripts/psp_trace_fields.py`, mis-applied
once this session before the fix. **The PS2 has no such table in either byte order**, and a
follow-up scan over 8 scales x 3 steps x 2 orders found no variant, so how
`SCES_547.48` pans is unknown - the one clearly-shaped open question here.
**The three `.vex` classes are placed, 2026-09-04**, and with them one of the
ten unowned construction sites: `VexSound_Init` (`0x08925a84`) is what a circuit
constructs its own emitters through, `soundcone` `0x3e9` is what sets the cone
flag, and `speaker` `0x3cc` is registered and authored by nothing on the disc.
See
[track-sound-emitters.md](../../docs/ghidra/functions/psp-pulse-usa/track-sound-emitters.md);
the wiring half is its own thread,
[a circuit's sound emitters parse, and nothing plays them](a-circuits-sound-emitters-parse-and-nothing.md).
**Still unplaced from this thread's own list**: the doppler term's unit, and
nine of fourteen emitter construction sites.

## Open

- How `SCES_547.48` (PS2) pans is unknown - no byte-order/scale/step variant matched across 8 scales x 3 steps x 2 orders
- Nine of fourteen emitter construction sites are still unowned, and the doppler term's unit is still unrecovered
- The doppler `1536` unit is still unrecovered - needs an audible reference, not a register read
- Which authored cone angle reaches the emitter's `+0x40` half-angle: `soundcone`'s own init has not been found, so the cone is read at the data end and not at the code end

## Next Steps

- None scoped on this thread right now. The `.vex` classes were its one step with fresh evidence available and they are done; what is left needs a different kind of evidence than a breakpoint - the doppler unit needs to be heard, and the PS2 pan and the nine construction sites need a fresh sweep angle rather than a rerun of this one
