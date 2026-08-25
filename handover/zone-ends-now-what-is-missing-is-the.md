# Zone ends now; what is missing is the explosion, not the rule

2026-08-10. The chain is closed end to end and implemented: pool empties -> `Ship_Damage` sets craft state 4 -> the explosion runs `0.5 s` (`0x088404c8`, four lines) -> `Ship_SetState` case 5 sets **bit 12 of `entity+0x860`**, which is what `Zone_UpdateRacing` ends on. [zone-mode.md](../docs/ghidra/functions/psp-pulse-usa/zone-mode.md) had that bit down as set by nothing findable; the search had been in the Zone code and on the contact path, and it is in the craft's own state machine three states later. `oag_physics::damage::CraftState` and `oag_race::RaceState::eliminate` are the port, confidence 88, no runtime leg. **The sound landed 2026-08-24** and case 4 is read at instruction level: the decompiler cannot follow `Ship_SetState`'s nine-entry jump table, so **disassemble the arm at `0x0884430c` rather than decompiling the function** - the table is at `0x08a7bc18` and all nine arms are on the page. `_BLOWUP` is `~BLOWUP` (`hud.bnk`), played dry at `0x400` with the handle at `craft+0xcac`, **before** the `craft+0x368` gate that makes everything after it the local player's alone - so an opponent's destruction plays nothing there and whatever it does play was not found. **The HUD hide and the camera mode are still absent**, now with four addresses instead of a shrug; three of the globals are unidentified and the camera call passes a literal `5`. Case 4 also zeroes the camera's `+0xe4`, which is `FUN_08878750`'s shake duration - a second sighting of the setter the autopilot row found. Also unmodelled: states 6 and 8, the respawn and the Eliminator's kill bookkeeping, which state 5 leads into after a further `1.5 s`.

## Open

- The HUD hide and camera mode on elimination are still unimplemented; three of the four addressed globals are unidentified and the camera call passes an unexplained literal `5`.
- States 6 and 8 (respawn, and the Eliminator's kill bookkeeping) are unmodelled.
- What sound plays for an opponent's destruction was not found - `_BLOWUP` plays only before the local-player gate at `craft+0x368`.

## Next Steps

- Identify the three unidentified globals behind the HUD hide and camera mode, and what the camera call's literal `5` selects.
