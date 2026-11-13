---
status: ready-to-wire
titles: [pulse]
updated: 2026-11-13
summary: "Pulse attract demo measured live: wire NextDemo, buttons-only exit, Flash/8-craft/Autopilot set-up and the track/team pick into Title::demo"
difficulty: medium (60)
---

# Pulse's attract demo laws are recovered

Read and measured 2026-11-13 (lane `demo-re`): every law, address and capture is on
`docs/ghidra/functions/psp-pulse-usa/attract-demo.md`. This thread lists what the
wire lane's `Title::demo` (`oag_title::demo::Demo`) needs and which of its current
`Demo::PULSE` values the measurement changes.

## Open

Values for `Demo::PULSE`, each measured unless marked:

1. **`idle_seconds` = 60, measured (93).** Strictly more than 60.0 s of front-end
   time from the later of the main menu becoming current and the last press of
   any button on it (the stamp is taken on the menu's first frame, not at the
   press that led there); main menu only. Live 60.028 s and 60.009 s. The title screen
   (PRESS START) never launches it (75 s idle, live).
2. **`duration_seconds` = 120, measured (92).** 120.0 s of race `dt` from the
   race's first update (the frame that loads the world does not count).
3. **`after_end` = `NextDemo`, measured (92), not `MainMenu`.** `Reset Demo`
   destroys the race and redirects to `Demo InGame`, which picks a new circuit
   and team and shows the loading screen. Seen twice in a row live. The current
   `Sourced::chosen(AfterEnd::MainMenu)` is wrong for Pulse.
4. **`exit`: any button, measured (92); the analog stick does not exit** (two
   seconds of full stick left the demo running). Lands on the main menu with
   the cursor on its first row; the press is consumed and does not activate it.
   Pointer and touch are ours to add (no pointer on a PSP), but the stick should
   not end the demo.
5. **`selection`: a new variant, measured (86).**
   - circuit: uniform over every circuit with `Type` `Race`/`PrimarilyRace`
     present on the disc (all 24 on Pulse), **unlocks ignored** (live: a locked
     `22_Track` on a fresh profile), never the previous demo's circuit
     (except raw list entry 0, which can repeat one time in `n`; see the page);
   - team: uniform over present `Race` teams with `multiplayer="true"` (all 8 on
     Pulse), never `Zone`, never the previous demo's team;
   - the original's off-by-one leaves the pick uniform whenever list entry 0
     is eligible; a plain uniform pick excluding the previous circuit is the
     faithful approximation;
   - the original draws from the wall clock; ours from the seeded `Rng`.
6. **Race set-up, measured or read at 85-92:** speed class **Flash** (ordinal 1),
   **8 crafts, 3 laps**, opponents from `multiplayer="true"` teams only, the
   player's craft on the Autopilot (`autopilot_input`), AI weapon use doubled
   (`weapon-ai.md`), weapons on and the soundtrack playing (both live, 85),
   **no end on laps** (a demo with its timer held off raced 376 s without
   finishing, live, 85: only the timeout ends it), flyby skipped at once with no track description, no Now
   Playing caption, no camera shake, nothing recorded to the profile.
7. **Overlay: play `Data\Plugins\PI001\GUI\Demo_Definition.xml`'s `Demo InGame`
   state** (the `demo_logo.mip` mark, two sliding cyan rules, pulsing
   `BOOT_PRESS_ANY2`), and hide the race HUD entirely (live frames). Do not
   hand-copy its coordinates.
8. **Camera (88): read, it is the spectator director** (`attract-demo.md`, "The demo camera is the spectator director"). Wired in the `demo-follow` lane.

## Next Steps

1. Wire items 3-7 into `Demo::PULSE` and the demo race set-up, with tests that
   fail if each value is dropped (owner: the wire lane).
2. Read the demo camera (`FUN_08886320`, class `FUN_08a6eb5c`, set up in
   `FUN_08887470` at `0x08887628`): which camera when, and how long each cut
   lasts. Measure live with a breakpoint on its update.
3. Why the lap count never finishes a demo race (the finish path in mode 2) is
   unread; the behaviour (it does not) is measured.
4. Not done (outside this lane's files): `docs/reverse-engineering/ppsspp-debugger.md`
   ("Idle ~120 s at the menus") and the `menu()` comment in `scripts/psp-drive.py`
   still say about 120 s; the threshold is 60 s.
5. HD, seen in passing and not chased: HD ships its own `demo_definition.xml`
   (`Demo Launch Real`, `forward="any"`), per the wire lane's `oag_title::demo`
   module doc.
