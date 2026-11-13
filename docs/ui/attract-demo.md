# The attract demo

An untouched main menu starts a race with nobody in it; any input ends it. `oag-game --demo`
starts in one and keeps going, `--wallpaper` does the same and ignores input.

## What a demo is

A Single Race on a circuit and team the title's rule picks, with **no slot a person flies**:
`oag_raceplay::Race::fly_as_demo` hands every controller to the AI and puts the camera slot (slot
0, which `World::primary_slot` falls back to) on the autopilot, so the opponents are flown by the
original's driver as in any race and the camera craft by the original's player driver. The AI flies
under the player's physics, as everywhere. It is launched through the ordinary race launch on a
copy of the options (`Session::launch_race`), so the player's settings and `race_options` are never
touched, and the race stage saves no result, ghost or recording and is never parked behind the
menus (`session::demo`, `oag_game::demo`).

- **Idle**: an untouched *main* page (depth one, no prompt, picker, campaign, join, LAN or online
  screen over it, no race parked, no net session) counts ticks; any button press, any key (bound
  or not), mouse move, click, wheel or touch restarts the count. A stick off centre does not
  (the original's demo kept running under 2 s of full stick, measured).
- **Exit**: the same inputs, in a demo. The press is consumed (the menu is not also fed it, and
  START does not pause a demo) and the menus open on the main page, not the pause screen.
- **Length**: the title's seconds of race time. The race is set to more laps than a demo can fly (`oag_game::demo::UNENDING_LAPS`) because the original's demo, set to three, was seen racing 376 s with its timer held off and never finishing (measured), and ours finishes a race that has run its laps; the three is `Rules::laps`, used when a title's demo ends on laps. A wreck that ends the race ends the demo too.
- **After a demo ran out**: the next demo, a new circuit and team, through the loading screen.
- **HUD**: hidden in every demo (measured on Pulse PSP), the Now Playing caption with it. The flyby and its track description are skipped.
- **Screen over the race**: `Demo InGame` of the title's own `Demo_Definition.xml`, parsed and drawn by the front end's own screen drawing (`oag_raceplay::demo_overlay`, `oag_game::demo_overlay`, `Frontend::draw_screen_over`): the `demo_logo` mark, two cyan rules and the pulsing `BOOT_PRESS_ANY2` ("Press any button"), on the demo's clock. Hidden in a wallpaper. **Pulse PSP: read and drawn. Pulse PS2: the same file reads (`demo_logo_EU.mip`, a 640x448 grid) and draws on the source's own grid (`hud.space`). Pure: inherited; the file is the same and reads, and the logo it names, `Data\FE\Images\demo_logo.mip`, is a 0-byte entry in both `Data.wad` and `FE.wad`, so Pure draws the rules and the text and no logo (nothing stands in). HD, 2048 and Omega: checked, applies, not wired**: their front end is HD's `demo_definition.xml` (a `.gtf` logo, 1920x1080 grid). The image is read through `Archives::read_texture` like the track panel's; the boot's own `read_front_end_texture` (hash specs, `FE.wad` first) is a game-crate function and was not needed for any entry here.
- **Set-up** (`oag_title::demo::Rules`, `oag_game::demo::apply_rules`, `Race::fly_as_demo`): speed class Flash, 8 craft (what a Single Race grid is; the test pins it on Pulse), weapons on, the opponents' weapon-use odds doubled (`oag_ai::weapon_ai::Law::use_scale`, `docs/gameplay/weapon-lineage.md`), no camera shake, nothing recorded to the profile; measured on Pulse PSP, inherited by the other titles.
- **Camera**: the spectator director (`oag_raceplay::finish_camera`), from the race's first tick on the camera craft, reseeded per race. It is the same object the original's post-finish camera is (`docs/ghidra/functions/psp-pulse-usa/attract-demo.md`, "The demo camera is the spectator director"). It needs the circuit's authored stations, which are read off Pulse's PSP disc only: every other title keeps its chase camera until its stations are read.
- **Idle inhibition**: a demo does not hold the screen awake; a wallpaper does.

## Per title (`oag_title::demo::Demo`)

| | Pulse PSP | Pure, Pulse PS2 | HD/Fury | 2048, Omega |
| --- | --- | --- | --- | --- |
| menu idle | 60 s, measured | inherited from Pulse | inherited from Pulse | **30 s, authored** (`DemoConfig.xml`) |
| demo length | 120 s, measured | inherited | inherited | **120 s, authored** |
| exit | any button, measured | inherited | any press, authored (`demo_definition.xml`, `Demo InGame` redirect `forward="any"`) | inherited from HD |
| circuit and team | every race circuit with unlocks ignored, a `multiplayer` team, neither repeating the last demo's, measured | inherited | inherited | circuit **authored**: `tower` (`AttractCycle`); team by the Pulse law |

Pulse's rows are measured live on PPSSPP (`demo-re`; the evidence page is
`docs/ghidra/functions/psp-pulse-usa/attract-demo.md` once merged). HD authors the demo's screens
and no timer (`/data/plugins/frontend/gui/demo_definition.xml`): `Demo Launch Real` sets the
`Mode` global to `Demo` and launches, `Demo InGame` shows the HD logo, pulsing `BOOT_PRESS_ANY`
text and two lines, and leaves on any press through `Demo Kill Game Transition` (0.4 s fade, 0.5 s
hold, audio stopped) and `Demo Kill Game` (0.2 s) to `Main Menu`; `Reset Demo` fades 0.4 s and
holds 0.7 s. The main menu's `Demo Launch` screen (0.5 s fade, 0.7 s hold) is the entry. HD
inherits Pulse's idle and length.

### `DemoConfig.xml` (2048, Omega)

`Data/xml/DemoConfig.xml`, 630 bytes, **byte-identical on 2048 (PCSF00007 `base`) and Omega
(`data00.psarc`)**, md5 `47fc86e942777b735d3dabf70d50ac47`:
`AttractStartTimeout value="30.0"`, `AttractEndTimeout value="120.0"`, one `AttractTrack
value="tower"` in `AttractCycle`, plus a `DemoMusic` (`Data\Music\07`, "School Of Funk"), a
`DemoPlayTrackList` (`tower`) and an empty `DemoShip`. Read at boot (`boot::demo_config`,
`oag_tables::demo_config`) and laid over the title's own seconds; the circuit cycle replaces the
seeded circuit pick when a name matches an offered circuit's id or environment directory. The
demo music and ship are not read: nothing here plays them. Pulse and HD ship no such file
(HD's seven `DATA` archives were searched). Omega and 2048 also carry `*_demo` plugin definitions
(`Definition_Demo.xml` for tracks, teams, music, `ingame_definition_demo.xml`, a Demo upsell set
on 2048's patch): the trial build's content, not read here.

**2048 against Omega: ported** (same file, same reader, ground truth
`demo_ground_truth::the_authored_demo_config_of_2048_and_omega_sets_their_timing_and_circuit`).
**HD against Omega: checked, applies for the screens, differs for the timing**: Omega's base
`demo_definition.xml` is 4,092 bytes like HD's and its patch's 4,164, so the screen set is HD's;
HD ships no `DemoConfig.xml`, and Omega's timing comes from that file.

**2048's main screen is its touch front end** (`Stage::Frontend`, not the menu stage), so the
idle count never runs there and an untouched 2048 front end starts no demo; `--demo` works on 2048
(`oag-game data/extracted/vita/PCSF00007 --demo --dry-run` flies `tower` twice). Pulse, Pure's
sibling Pulse, HD and Omega were driven through the idle path (`drive_ground_truth`, Pulse, HD and
Omega; Pure and Pulse PS2 share Pulse's code path, not driven).

## `--demo` and `--wallpaper`

```sh
oag-game --demo                 # skip the front end, demo after demo; any input goes to the menu
oag-game --wallpaper            # the same, input ignored, Escape quits, the screen stays awake
oag-game --demo --dry-run       # no window: two demos back to back, one line each
oag-game --demo --screenshot demo.png --ticks 500
```

`--demo` is the original's behaviour (input ends it); once it has, the menu's own idle count takes
over and the next demo starts after the idle time. `--wallpaper` is for a desktop background:
**one window on one output, one camera**. A window and camera per output is not built (see the
handover thread `attract-demo-leftovers`): the multi-window layer deals *seats* (people) across
windows and a wallpaper has none. Checked with `--demo --window-seats 1,1 --screenshot`: it writes
two windows, the second a seated view of another grid slot, because the seats are people the demo
later hands to the AI.

## Tests

`demo_ground_truth` (chains on Pulse, Pure and HD with every craft flown, the field `Rules::craft`, the screen over the race read and drawn, Pulse's spectator camera cutting, the headless
`--demo --dry-run` run, 2048/Omega's authored config), `drive_ground_truth` (idle launch, press
restart, each input kind ending a demo, `--demo` chaining, `--wallpaper` ignoring input), and unit
tests in `oag_game::demo`, `oag_raceplay::demo`, `oag_tables::demo_config`.
