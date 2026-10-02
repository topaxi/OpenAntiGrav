---
categories: [gameplay, rendering]
---

# Zone ends now, and the wreck explodes; four reads are still open

**The explosion landed 2026-10-01** (wreck swap, `WO_SHIP_EXPLOSION`, the circuit-camera cut - see the rendering thread on a wrecked craft). What remains is the four open items below.

2026-08-10. The chain is closed end to end and implemented: pool empties -> `Ship_Damage` sets craft state 4 -> the explosion runs `0.5 s` (`0x088404c8`, four lines) -> `Ship_SetState` case 5 sets **bit 12 of `entity+0x860`**, which is what `Zone_UpdateRacing` ends on. [zone-mode.md](../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md) had that bit down as set by nothing findable; the search had been in the Zone code and on the contact path, and it is in the craft's own state machine three states later. `oag_physics::damage::CraftState` and `oag_race::RaceState::eliminate` are the port, confidence 88, no runtime leg. **The sound landed 2026-08-24** and case 4 is read at instruction level: the decompiler cannot follow `Ship_SetState`'s nine-entry jump table, so **disassemble the arm at `0x0884430c` rather than decompiling the function** - the table is at `0x08a7bc18` and all nine arms are on the page. `_BLOWUP` is `~BLOWUP` (`hud.bnk`), played dry at `0x400` with the handle at `craft+0xcac`, **before** the `craft+0x368` gate that makes everything after it the local player's alone - so an opponent's destruction plays nothing there and whatever it does play was not found. **The HUD hide and the camera mode are still absent**, now with four addresses instead of a shrug; three of the globals are unidentified and the camera call passes a literal `5`. Case 4 also zeroes the camera's `+0xe4`, which is `FUN_08878750`'s shake duration - a second sighting of the setter the autopilot row found. Also unmodelled: states 6 and 8, the respawn and the Eliminator's kill bookkeeping, which state 5 leads into after a further `1.5 s`.

**2026-08-27: two of the three globals moved.** `0x08ab0838` is identified -
it is `DAT_08ab0838`, the shield/energy-bar widget `shield.md` already
documents as `Hud_SetEnergyBar`'s object argument - and case 4's call on it
releases exactly three texture handles through the same tracked-free helper
`Hud_SetEnergyBar` itself uses, confidence 85. `0x08ab10e8` gets a structural
hypothesis (a second camera-shaped object, flag-cleared in the same breath the
active camera's identical flag is set) at confidence 55, short of a live
watchpoint. `0x08ab2120` found no further lead - still fully unidentified.
The "camera call passes a literal `5`" question was answered in the
2026-08-24 pass already on the page (mode 5 is the death camera, confidence
80, what it renders is unread) - it was carried in this thread's `Next Steps`
after the doc had already moved past it. Full evidence:
[zone-mode.md](../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md#not-determined).

## Open

- `0x08ab2120`'s identity (`(*addr)+0x58 = 0`, a byte) has no lead from static reading - single use, no other reference to the corrected address anywhere in the program.
- `0x08ab10e8`'s identity is a structural guess (paired with the active-camera object) at confidence 55, not confirmed - needs a live read.
- States 6 and 8 (respawn, and the Eliminator's kill bookkeeping) are unmodelled.
- ~~What sound plays for an opponent's destruction~~ **Found 2026-10-02 (`pulse-zone-rest`, live)**: the announcer line `cont_elim` at state 6's expiry (`FUN_08840500`), 2.8 s after the destruction begins, in modes other than 2, 8 and 18; the explosion sounds `EXPLSMALL`/`EXPLBIG` do not start for an opponent. **Not wired** (it is the state-6 bookkeeping of the Eliminator lane and needs a new cue and the speech bank): see [zone-rest.md](../../docs/ghidra/functions/psp-pulse-usa/zone-rest.md#what-sounds-an-opponents-destruction-makes-answer-to-the-_blowup-open-question).

## Next Steps

- Set a PPSSPP watchpoint on `0x08ab10e8` (and, if time allows, `0x08ab2120`) to settle their identities the way `ship-parts.md`'s airbrake read did.
