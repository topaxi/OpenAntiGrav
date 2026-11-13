# The attract demo (game mode 2, `Demo`)

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`, language `Allegrex:LE:32:default`. Read 2026-11-13 (lane
`demo-re`) from headless Ghidra decompiles and disassembly, the disc's own
front-end XML, and four live PPSSPP runs on `pulse-psp-usa.chd` with a fresh
memory stick (first-boot profile, nothing unlocked).

How the original's attract mode works end to end: what starts it, what race
it builds, how long it runs, what it shows, and what ends it. Everything a
reimplementation needs is in [What a reimplementation needs](#what-a-reimplementation-needs).

## Names

| Address | Name | Kind | Conf |
| --- | --- | --- | ---: |
| `0x08818fd8` | `Demo_UpdateAttractMode` | function | 92 |
| `0x088331ac` | `Demo_Construct` | function | 88 |
| `0x08818f74` | `DemoSession_Construct` | function | 88 |
| `0x08818f00` | `DemoSession_IsTrackEligible` | function | 86 |
| `0x08ac7c20` | `g_demo_session_vtable` | data | 85 |
| `0x0888e488` | `Definition_IsPresent` | function | 75 |
| `0x0880569c` | `Game_SetPlayerTeamByName` | function | 85 |
| `0x088895b8` | `Track_ListDefinitions` | function | 80 |
| `0x08821ad8` | `Race_DestroyWorld` | function | 80 |
| `0x08b310b4` | `g_track_definition` | data | 88 |

Already named and used here: `MainMenu_Update` `0x088e0784`,
`Race_CreateModeObject` `0x0882112c`, `g_game_mode` `0x08b31048`
([state-machine.md](state-machine.md)); `Team_ListRaceTeams` `0x088893e8`,
`RaceSession_IsTeamEligible` `0x08821dd4`, `g_player_team_definition`
`0x08b3104c` ([grid.md](grid.md)); `g_fe_anim_clock` `0x08b317b0`
([anim-transform.md](anim-transform.md)); `InGame_Update` `0x08813328`
([anim-transform.md](anim-transform.md)); `Definition_IsUnlocked`
`0x0888e29c` ([race-box-screens.md](race-box-screens.md));
`RaceMode_UpdateIntro` `0x08829e6c` ([race-intro.md](race-intro.md)).

`Demo_UpdateAttractMode` and `Demo_Construct` were named at 80 on
[main-loop.md](main-loop.md) and [state-machine.md](state-machine.md); this
page carries their evidence now and raises both on the live runs below.

## The state graph is data: `Data\Plugins\PI001\GUI\Demo_Definition.xml`

The demo's four front-end states are authored in `Demo_Definition.xml`
(`Data.wad`, listed by the front end's `Definition.xml` beside
`InGame_Definition.xml`). Expanded from the file's element dictionary
([fexml.md](../../../formats/fexml.md)):

| State | Backend task | Redirect | Other content |
| --- | --- | --- | --- |
| `Demo Launch` | `Launch` | default `goto` `Demo InGame` | sets the front-end global `Mode` to `"Demo"` on enable |
| `Demo InGame` | `Run` | forward key `any` -> `Demo Kill Game`; backward none | two cyan (`0xff3abcf2`) 1-pixel rules that slide in (`MainAnim16` keys), `Data\FE\Images\demo_logo.mip` at (4, 238) 128x32 tinted `0xB4FFFFFF` with `enabletransition="0.4"`, and the string `BOOT_PRESS_ANY2` ("Press any button") at (470, 245), right-aligned, `font="scroll"`, `pulse="true"`, `DisplayType="TELETYPE"`, `delay="1.0"` |
| `Reset Demo` | none | default `goto` `Demo InGame` | |
| `Demo Kill Game` | `Kill` | default `goto` `Main Menu` | |

The only two code literals are the entries into this graph:
`MainMenu_Update` sends `Demo Launch` and `Demo_UpdateAttractMode` sends
`Reset Demo` ([state-machine.md](state-machine.md), "The literal states").
No other front-end XML on the disc names any of the four states (a byte
search of every `Data.wad`, `FE.wad` and `FEData.wad` entry for `Demo Launch`
and `Reset Demo` finds only this file).

Confidence **94** for the table (read off the shipped file; the live runs
saw `Demo InGame`, `Demo Kill Game` and `Main Menu` in that order).

## Entry: 60 seconds idle on the main menu, nowhere else

`MainMenu_Update` (`0x088e0784`) keeps a stamp at `screen+0xd8` and compares
it with the front end's animation clock, `*(g_fe_anim_clock)+0x40` (seconds):

```c
if (Input_IsPressed(g_input, 0x14 /* any button */, 0)
    || g_state_machine->current (+0x1d0) != screen
    || g_display->+0x5e34 != -1)            /* FUN_0891e844 */
    screen->stamp (+0xd8) = clock;
if (clock - screen->stamp > 60.0) {         /* 0x088e09f0.. */
    FUN_0888c408(g_fe_anim_clock, 0, 0);
    StateMachine_TransitionTo(screen->machine (+0xcc), "Demo Launch", ...);
    Input_ConsumePress(g_input, 0x14);
    FUN_0894f284(g_input, 0x14);
}
```

So the timer is **60 s with no press while `Main Menu` is the current
state**; leaving the screen or any press restarts it, and it only runs while
`g_display+0x5e34` reads `-1` (it did, `0xffffffff`, on the idle menu). The
clock is the front end's own seconds clock; between two reads 93.5 emulated
seconds apart it advanced 93.43 s (ratio 0.999), so it is real time.

**Live, twice** (a breakpoint on the `Demo Launch` call at `0x088e0a0c`, reading
the stamp and the clock at the stop):

| Run | Last press | Stamp `+0xd8` | Clock at launch | Idle |
| --- | --- | --- | --- | ---: |
| fresh profile, after the first-boot dialogs (state `Main menu`) | circle on `CreateFromBoot` | 112.245 | 172.273 | **60.028 s** |
| after leaving a demo with Cross (state `Main Menu`) | the Cross that left the demo | 469.301 | 529.310 | **60.009 s** |

Both capitalisations of the main menu state arm it.

**Nowhere else.** The title screen (`Show Logo`, the PRESS START screen) was
left idle for 75.1 s (emulated and wall alike) and stayed `Show Logo`. Statically
`Demo Launch` has one code caller and no XML `goto`.

Confidence **93** for the 60 s rule and its resets (decompile plus two live
launches to within one frame of 60 s), **88** for "the main menu only" (one
negative live run on the title screen, the single literal, and the XML census).

**The "about 120 seconds" in [menus-original.md](../../../ui/menus-original.md)
was wrong**: the entry threshold is 60 s of game time. The 120 s figure is the
demo's own length (below). That page is corrected.

## The demo session: `DemoSession_Construct` (`0x08818f74`)

`InGame_Update`'s mode switch (`case 2`, `0x0881365c`) allocates a `0xa0`-byte
node and builds it with `DemoSession_Construct`: the race-session base
(`RaceSession_Construct`), vtable `g_demo_session_vtable` (`0x08ac7c20`), and
four fields:

| Offset | Field | Initial |
| --- | --- | --- |
| `+0x90` | previous demo track name | `""` (`0x08a79910`) |
| `+0x94` | previous demo team name | `""` |
| `+0x98` | seconds this demo race has run | `0.0` |
| `+0x9c` | demo races started (only printed: `"Track count %d : %s"`) | `0` |

`Demo_UpdateAttractMode` is the session's update, slot `+0x20`/`+0x24` of
`g_demo_session_vtable` (the word at `0x08ac7c44` is `0x08818fd8`; the next two
slots hold `0x088210e0` and `0x08818f6c`, not read). Live, the
session at `0x08fc8480` held the previous names and a count of `0`, `1` across
two consecutive demos, so the object survives `Reset Demo`. Confidence **88**.

## Selection: `Demo_UpdateAttractMode` (`0x08818fd8`)

`Demo_UpdateAttractMode(dt in f12, session in a0)` picks a new track and team
whenever the world does not exist (`DAT_08b32c88 == 0`; it is written by the
`0xc80`-byte race object's constructor `FUN_08886af4`), which is the first
frame after `Demo Launch` and the first frame after every `Reset Demo`. Then it
calls the session base update `FUN_08820e90`, which builds the race
(`Race_CreateModeObject` -> `Demo_Construct`) on the same frame.

### The track

`Track_ListDefinitions` (`0x088895b8`) lists every track definition. A track is
eligible (`DemoSession_IsTrackEligible`, `0x08818f00`) when all of:

1. its name (`+0x74`) **differs from the previous demo track** (`FUN_08973424`,
   the `strcmp` of [grid.md](grid.md), with `a1` passed through untouched from
   the caller: the function takes **two** arguments, the decompiler shows one);
2. the definition class's `+0xd4` virtual returns non-zero: that is
   `Definition_IsPresent` (`0x0888e488`, `jr ra; lbu v0,0x9a(a0)`), the byte the
   base definition constructor `FUN_0888e1dc` sets to `1` and only the
   memory-stick music and DLC loaders write again. **It is not the unlock
   check**: `Definition_IsUnlocked` is never called;
3. `def+0x164 & 1`: the `Type` attribute of the `<Values>` element, through the
   six-entry table at `0x08ab1b10`/`0x08ab1b28` (`Race` 1, `Classic` 2, `Zone` 4,
   `PrimarilyRace` 8, `PrimarilyClassic` 0x10, `PrimarilyZone` 0x20; stored as
   `v | v >> 3`), so bit 0 is `Race` or `PrimarilyRace`. `TrackDefinition_ReadXml`
   (`0x088c3f7c`) defaults it to `Race`. All 24 circuits in Pulse's
   `Data\Plugins\PI001\Definition.xml` are `Race`.

The pick, from the disassembly (`0x088190dc`-`0x0881913c`):

```c
n = count of eligible tracks;
if (n == 0) {                 /* fallback: last track eligible against "" */
    pick = last i with IsTrackEligible(list[i], "");
} else {
    k = (u32)sceKernelGetSystemTimeWide() % n + 1;
    pick = 0;                 /* raw list index 0 if k never reaches 0 */
    for (i = 0; i < count; i++)
        if (IsTrackEligible(list[i], prev)) { if (k == 0) pick = i; k--; }
}
```

The counter starts at `remainder + 1` and is tested **before** it is
decremented, so the chosen track is eligible entry number `remainder + 1`
(0-based), and when `remainder == n - 1` no entry is chosen and the pick is raw
list entry 0, whatever it is (the list order of `Track_ListDefinitions` was not read). The
off-by-one is in the shipped code: entry 0 of the eligible list is never picked
on its own merit, and list entry 0 is picked about one time in `n`.

The pick is stored three ways: `session+0x90` (name), `g_track_definition`
(`0x08b310b4` = `g_game+0x124`) and its name copy at `g_game+0x128`.

**The random source is the wall clock**, `sceKernelGetSystemTimeWide()`'s low 32
bits modulo the count, not `rand()` or the particle generator
([prng.md](prng.md)).

Live picks on a fresh profile: `18_Track`, then `22_Track`, then `13_Track`.
`22_Track` carries an `<Unlock>` (`Grid9`) a fresh profile has not met, so **the
demo draws from every circuit, locked ones included** (confidence 92). No pick
repeated its predecessor (three picks; the rule itself is static).

Confidence **86** for the eligibility and the off-by-one (unambiguous
disassembly, live picks consistent; the off-by-one is not observable in three
picks).

### The team

`Team_ListRaceTeams` lists the `Type="Race"` teams; then up to 10,000 tries of
`team = list[(u32)sceKernelGetSystemTimeWide() % count]` until all of:

1. name differs from the previous demo team (`session+0x94`);
2. name differs from `"Zone"` (`0x08a79914`);
3. `Definition_IsPresent(team)`;
4. `team+0xa9` (`Multiplayer`, [grid.md](grid.md)) is set.

When the tries run out the last draw is taken unchecked. The result goes to
`Game_SetPlayerTeamByName` (`0x0880569c`), which looks the name up with
`strcasecmp`, stores the definition in `g_player_team_definition` (`g_game+0xbc`)
and copies the name to `g_game+0xc0`; `Race_ReadSetupOptions` uses the same
function for the menu's `Team` global. All eight teams in Pulse's
`Definition.xml` are `multiplayer="true"`, so the pool is the disc's eight minus
the last one. Live: `Triakis`, then `AG_Systems`. Confidence **86**.

### The rest of the set-up

`Demo_UpdateAttractMode` then sets the speed class `g_game+0xb0`
(`DAT_08b31040`) to **1, Flash** (`0x08819260`-`0x08819268`; 0 is Venom,
[race-progress.md](race-progress.md); read `1` live on both demos), stamps
`DAT_08ab22c0` with the wall clock (no `lw` of that address exists; its reader,
if any, was not found), zeroes the run timer and increments the count.

`Demo_Construct` (`0x088331ac`, the `Race_CreateModeObject` case for mode 2):

- `Arcade_HUD.xml`, then **clears the HUD's widget mask bits 0-8**
  (`g_hud+0x40 &= ~0x1ff`) and its shown bits (`g_hud+0x2c &= ~6`). Arcade sets
  `1, 2, 4, 8, 0x10, 0x40, 0x80, 0x2000` instead (`ArcadeRace_Construct`);
- **grid 8** (`FUN_0880502c(g_game, 8)`; read `8` live once the race was built),
  **3 laps** (`FUN_08805058(g_game, 3)`);
- `RaceSession_DrawAiRoster` (`0x08821bd4`, the only `srand` caller, so each
  demo race reseeds the gameplay `rand()` from the wall clock, [prng.md](prng.md)),
  then seven `Race_CreateAiRacer`, then the player
  craft in the last slot with `g_player_team_definition`;
- **the player craft gets `autopilot_input`** (`FUN_0883d898(player, "autopilot_input")`),
  so every craft is driven: the player by the Autopilot, the seven others by the AI.

Confidence **88** for the constructor's values (static; grid and class live).

## Duration and the timeout: 120 seconds, then a new race

After the base update, `session+0x98 += dt`; when it exceeds `120.0`
(`lui a0,0x42f0`, `c.le.s` at `0x088192dc`) the function prints `Demo timeout`,
zeroes the timer, prints `DestroyWorld...`, calls `Race_DestroyWorld(session, 0)`
(`0x08821ad8`: queues every child node of the session for deletion, then
calls `FUN_089014d4` and `FUN_0893a988`; a non-zero second argument would first
call `FUN_08891088(g_state_machine)`, which the demo skips), prints `Switch to reset demo...`,
transitions to `Reset Demo` and sets `session+0x81 = 1` so the next build shows
the loading screen (`Loading_Show(4)` in `FUN_08820e90`). `Reset Demo` redirects
to `Demo InGame`, and the next update finds no world and picks again.

Live: from the selection breakpoint to the `Reset Demo` call, **122.24 s and
122.25 s** emulated (two demos), the front-end clock advancing 119.95 s and
119.95 s across the same spans; the state read `Demo InGame` at both ends. The
2 s difference is the frame that loads the world, which counts in emulated time
but not in `dt`. Confidence **92**.

Whether a 3-lap Flash race can finish inside 120 s on a short circuit, and
what the race-end path does in mode 2, was not read or seen; both live demos
were cut by the timer.

## Exit: any button, back to the main menu

`Demo InGame`'s forward key is `any`, the abstract index `0x14`
([input.md](input.md)). Live: two seconds of full right analog stick left the
demo running; one Cross press read `Demo Kill Game` at once and `Main Menu`
0.6 s later, with the cursor on RACE CAMPAIGN (the press did not carry into the
menu). `g_game_mode` still read `2` on the menu; nothing resets it until the
next race set-up. 60 s later the demo launched again (the second row of the
entry table). Confidence **92**.

## What else mode 2 changes

A census of every reference to `g_game_mode` with a compare against 2 within 14
instructions (235 references, 20 direct `xori ..,0x2` tests) gives the
behaviour list. Jump tables over the mode were not opened case by case, so
this is the complete set of *direct* tests, not a proof that no switch has a
case 2.

| Site | In Demo | Conf |
| --- | --- | ---: |
| `RaceMode_UpdateIntro` (`0x08829e6c`, five tests, [race-intro.md](race-intro.md)) | no `InGameTrackDescriptionScreen`, the flyby is skipped as soon as it has started, no transition to `InGame` (the front end stays in `Demo InGame`), and the HUD-shown bits are not set | 85 |
| `Demo_Construct` | race HUD hidden (above). Live frames show no race HUD at all, only the `Demo_Definition.xml` overlay | 92 |
| `FUN_08887470` (`0x08887628`), called by `Race_CreateModeObject` | builds the camera with `FUN_08886320` (class `FUN_08a6eb5c`) instead of the player's internal tripod plus `FUN_088865d0`; AI Race (mode 12) shares it. Live: ten frames 3 s apart all trackside views of the pack with cuts between them, earlier frames chase-like behind a craft. The camera's own cutting law is not read | 75 |
| `WeaponAi_DecideFireOrAbsorb` (`0x08851b18`, [weapon-ai.md](weapon-ai.md)) | AI weapon use x2 | 85 |
| `RaceSession_IsTeamEligible` (`0x08821e84`, [grid.md](grid.md)) | opponents drawn only from `Multiplayer` teams | 85 |
| `MusicPlayer_StartNextSong` (`0x08938e70`, test at `0x089390f4`, [now-playing.md](now-playing.md)) | no Now Playing caption | 85 |
| `Camera_ArmShake` (`0x0887877c`) and `Ship_Damage` (`0x08843c28`) | no camera shake, no hit flash on the player | 82 |
| `FUN_0883e9b0` (`0x0883e9fc`) and the exhaust-flare sounds (`0x08904f74`, `0x08905030`, `0x089050ec`, `0x089051b0`) | the player's "dry" cockpit sounds (`energycritical`, `TURBO`, speed pad) are not played dry; the flares play them positional instead ([perfect-start.md](perfect-start.md)) | 82 |
| `FUN_08809b38` (`0x08809b74`) | returns 0 in Demo: the predicate gating profile counters (`DAT_08b31774+0x11c`, `+0x220`, [endrace-screens.md](endrace-screens.md)) and about 70 other call sites, so a demo **records nothing to the profile**. Its other callers were not read; not renamed | 70 |
| `Zone` rest, `cont_elim` voice ([zone-rest.md](zone-rest.md)) | excluded | 85 |

Weapons: mode 2 is not in `Race_ReadSetupOptions`' "weapons off" list
([state-machine.md](state-machine.md)), and the doubled AI use above only means
anything with weapons on. The front-end `Weapons` global was not read live
during a demo. Music: the PPSSPP instance is muted, so whether the demo plays a
soundtrack was not observed; `FUN_0888c408(g_fe_anim_clock, 0, 0)` at launch and
`(…, 1, 0)` on the race build are the front-end music calls `MainMenu_Update`
and every screen make, not read further.

## What a reimplementation needs

| Value | Law | Conf |
| --- | --- | ---: |
| entry | 60.0 s of front-end time with no press, main menu only, timer restarts on any press or on leaving the menu | 93 |
| states | play `Demo_Definition.xml`: `Demo Launch` -> `Demo InGame` (overlay) -> `Demo Kill Game` -> `Main Menu`; `Reset Demo` -> `Demo InGame` | 94 |
| track | random over every `Type` `Race`/`PrimarilyRace` circuit present on the disc, **ignoring unlocks**, excluding the previous demo's track, with the off-by-one above (or a plain uniform pick, if the off-by-one is not wanted) | 86 |
| team | random over present `Race` teams with `Multiplayer` set, not `Zone`, excluding the previous demo's team | 86 |
| class | Flash (ordinal 1), always | 92 |
| field | 8 crafts, 3 laps; opponents drawn from `Multiplayer` teams | 88 |
| player | driven by the Autopilot (`autopilot_input`) | 88 |
| camera | trackside cameras with cuts (mode 2's own camera object) | 75 |
| HUD | race HUD hidden; only the XML overlay | 92 |
| no | flyby skip at once, no track description, no Now Playing, no camera shake, no profile stats, AI weapon use x2 | 82-85 |
| timeout | 120.0 s of race `dt` (from the race's first update; the loading frame does not count), then destroy and rebuild on a new track and team, with the loading screen | 92 |
| exit | any button (not the analog stick) -> main menu, the press consumed | 92 |
| RNG | the wall clock modulo the count; ours uses its seeded `Rng` | 90 |

## How to repeat the live runs

PPSSPP SDL on a private `HOME` (`scripts/ppsspp-start.sh <lane> <display> <port>
<image>`), a fresh memory stick. Answer the first-boot dialogs as
`scripts/psp-drive.py`'s `FIRST_BOOT` does, then on the main menu stop the CPU,
arm one breakpoint and resume (PPSSPP v1.20 fires only the most recently added
one; [ppsspp-debugger.md](../../../reverse-engineering/ppsspp-debugger.md)):

- `0x088e0a0c` (the `Demo Launch` call's delay slot; the stop reports pc
  `0x088e0a08`): read `*(*(0x08b317b0)+0x40)` and `*(*(*0x08b31784+0x1d0)+0xd8)`;
- `0x08819278` (after the picks): `g_track_definition->+0x74`,
  `g_player_team_definition->+0x74`, `0x08b31040`, the session from `s0`
  (`cpu.getReg`);
- `0x08819324` (the `Reset Demo` call): emulated ticks against the selection
  stop.
