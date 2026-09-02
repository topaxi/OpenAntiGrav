# Every SFX trigger is a Pulse reading, applied to Pure and HD on a bet

Split out 2026-09-02 from `hds-bnk-sound-banks-read-and-hd-makes.md`, whose own scope (the `.bnk` container format and both waveform codecs) is now fully resolved and closed. This caveat was carried in that thread's opening paragraph but was never actually part of its `## Next Steps`, and it is not really about bank *decoding* at all - it is about the nine [`oag_game::audio::sfx::Cue`](../crates/game/src/audio/sfx.rs) variants, each of whose trigger (`SpeedupPad`, `Collision`, `Absorb`, `Engine`, `Shield`, `ShieldActive`, `Disengaging`, `Blowup`, `LockOn`) was recovered from **Wipeout Pulse's PSP executable only**. `crates/game/src/audio/sfx.rs`'s own module doc already states this plainly: *"no Pure or HD dispatch has ever been looked at. Firing a Pulse-recovered edge on those titles is a bet ... Reasonable, and not a reading; confidence 50."* That confidence-50 line is the reason this is worth its own thread: it is below this project's naming and trust threshold on two of the three titles this engine plays, for every single wired sound effect.

## Open

- Whether Pure's executable calls the same `Sound_Play`/`Sound_PlayLooping` sites at the same simulation edges Pulse does, for each of the nine cues - Pure has never been opened in Ghidra for this at all.
- Whether Wipeout HD's PS3 executable does the same. HD's `ShipCollisionFx_Trigger`, `Exhaust_UpdateEngineSound`, `Shield_Activate` and so on are presumably different functions at different addresses in a different binary (PPC64, per-function TOC defect - see `docs/ghidra/functions/ps3-hdfury-eu/memory.md`), not yet located.
- Whether the *gating* logic matches, not just the call site's existence - e.g. Pulse's `Collision` cue rides the same 0.8-second spark cooldown as the spark effect; if Pure or HD's cooldown differs, playing the sound on Pulse's cadence would be audibly wrong even though the cue itself is right.

## Next Steps

- Pick one cue - `Collision` is the best first target, since `docs/ghidra/functions/psp-pulse-usa/contact-response.md` already gives a precise trigger shape ("once per surviving kind-0/1 call") to look for an analogue of - and open Wipeout Pure's PSP executable in Ghidra to find its `ShipCollisionFx_Trigger`-equivalent, or whatever calls `Sound_Play(..., "COLLISIONS", ...)` there.
- If Pure corroborates, that alone raises the finding past confidence 50 (a second binary agreeing, per the confidence rubric) without needing HD at all; HD's PPC64/TOC binary is the harder half and can follow once the PSP-side pattern (Pure vs Pulse) is established.
- `oag-wad sounds <archive>` lists the cues each bank actually carries per title, useful for confirming a candidate cue name exists on Pure/HD before spending time hunting its trigger.
