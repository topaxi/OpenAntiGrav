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
- **Idle inhibition**: a demo does not hold the screen awake; a windowed wallpaper does, a desktop background (below) does not, so the screen still blanks and locks over it.

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
oag-game --wallpaper            # the same, input ignored; the desktop background on a layer-shell compositor
oag-game --wallpaper --windows 3   # elsewhere: three windows, a camera each
oag-game --demo --dry-run       # no window: two demos back to back, one line each
oag-game --demo --screenshot demo.png --ticks 500
```

`--demo` is the original's behaviour (input ends it); once it has, the menu's own idle count takes
over and the next demo starts after the idle time. `--wallpaper` is for a desktop background: no
input reaches it, no player-name prompt opens over it, and it draws at most `--wallpaper-fps`
frames a second (default 30, over the VSYNC and FRAME LIMIT settings).

**A demo's windows are cameras, not seats** (`Race::watch_in_windows`, **chosen, not measured**:
the original has one screen). A demo shown in several windows deals one view to each window and
seats nobody: window 1 keeps the demo's own camera (Pulse's spectator director), each further window
follows the next craft of the grid with the chase camera of a split screen's extra view, cycling
once every craft fielded (at most eight) has a window. The HUD is hidden in every window, and a
window waiting for its camera (the menus, the loading screen) is black, with no `PLAYER n` label.
View side only: the state hash is the same as one window's
(`tests::windowed::a_demo_watched_in_three_windows_follows_three_craft_and_moves_no_hash`). The
windows come from `--windows N` (a window each, on any desktop), or on a layer-shell compositor
from the outputs, below.

## A desktop background

On a Wayland compositor with `wlr-layer-shell` (`zwlr_layer_shell_v1`: niri, sway, Hyprland, river,
labwc, Wayfire), `--wallpaper` opens no window. Each output gets a `background`-layer surface,
anchored to all four edges at size 0x0 (the whole output), exclusive zone -1 (drawn under panels,
moving nothing), keyboard interactivity none and an empty input region, namespace `oag-wallpaper`
(`crate::wallpaper`). Each is drawn as a window of the one session, a camera each, as above. A
compositor with no layer shell (GNOME, KDE without it, X11) gets the windowed wallpaper and a
warning; `WINIT_UNIX_BACKEND=x11` asks for it.

- **The loop.** winit has no layer shell, so the run has no winit window and no winit loop:
  `wallpaper::run` owns a calloop loop over the Wayland socket (`smithay-client-toolkit`, the toolkit
  winit's own Wayland backend is built on) and calls `Session::frame` as a window's redraw would,
  the way `--drive` runs a session with no window. Each surface is a wgpu surface over the raw
  `wl_surface` (`gpu::RawTarget`; the one `unsafe` is the surface's creation).
- **Pacing.** An output is drawn only when the compositor answered its last frame callback
  (`gpu::Pace`): a background a compositor stops asking frames of (a powered-off output, a
  fullscreen window over it on some compositors, a locked screen) is not drawn, and is tried again
  every 2 s. While no output wants a frame, the simulation does not tick. The present mode is a
  further window's non-blocking one (Mailbox, else Immediate): a blocking FIFO on a surface the
  compositor stopped showing would hold every output's frame.
- **Outputs come and go.** A new output gets a background and a camera; an unplugged one is
  dropped and the demo is dealt again over what is left. When window 1's output goes, the next
  output's surface becomes window 1 and keeps the demo's own camera.
- **Scale.** Integer output scale: the buffer is drawn at the size times the scale
  (`wl_surface.set_buffer_scale`). A fractional scale (niri's 1.25) is drawn at the next integer
  scale up and scaled down by the compositor; `wp_fractional_scale_v1` is not read.
- **Leaving.** SIGINT or SIGTERM ends it through the same way out as closing a window
  (`Session::wind_down`), as does the compositor going away. No idle inhibitor is held.
- **A disc must be named** (`oag-game <image> --wallpaper`) when the search path holds several:
  the disc picker needs input, so it opens in a window.

**Checked in a headless sway** (2026-11-13, sway 1.12, wlroots headless backend, GLES2 renderer;
our device on Vulkan, RADV): three outputs of 1280x720, 1024x768 and 800x600 at scale 2, a
floating `foot` above output 1. Each output showed the demo behind the window, output 1 through the
spectator director and outputs 2 and 3 chasing two other craft; no HUD. Unplugging output 2 and
then output 1, and creating a 1920x1080 output, kept every remaining output drawing (window 1
moved to the next output). With every output powered off (`swaymsg output X power off`) the process
used 0.2 % of a core against 21 % drawing two outputs at 30 fps, and went back to 30 fps on
power on. SIGTERM left through the normal exit. The recipe:

```sh
R=$PWD/rt; mkdir -p -m 700 "$R"
printf 'xwayland disable\noutput HEADLESS-1 resolution 1280x720\n' > sway.conf
env -i HOME=$HOME PATH=$PATH XDG_RUNTIME_DIR=$R WLR_BACKENDS=headless WLR_RENDERER=gles2 \
    WLR_LIBINPUT_NO_DEVICES=1 sway -c sway.conf &
export XDG_RUNTIME_DIR=$R WAYLAND_DISPLAY=wayland-1 SWAYSOCK=$(echo $R/sway-ipc.*.sock)
swaymsg create_output; swaymsg create_output
swaymsg output HEADLESS-2 resolution 1024x768; swaymsg output HEADLESS-3 resolution 800x600 scale 2
oag-game data/images/pulse-psp-eu.chd --no-audio --wallpaper &
foot &                       # then: swaymsg '[app_id=foot] floating enable, resize set 560 240'
grim -o HEADLESS-1 out1.png; grim -o HEADLESS-2 out2.png; grim -o HEADLESS-3 out3.png
swaymsg output HEADLESS-2 unplug; swaymsg create_output; swaymsg output HEADLESS-1 unplug
```

**niri** (26.04 on the maintainer's machine). Not checked live (the maintainer's screen was
locked); none of the following was observed, it is what may differ from sway:

- niri draws `background`-layer surfaces in its backdrop and per workspace; a layer rule can match
  the namespace `oag-wallpaper` (`layer-rule { match namespace="^oag-wallpaper$" ... }`), e.g. to
  place it in the overview backdrop (`place-within-backdrop true`).
- Whether niri withholds frame callbacks from a background it does not show (a locked session, an
  output off, a fullscreen window over it), which the pacing above relies on to stop drawing. If it
  keeps sending them, the wallpaper draws at `--wallpaper-fps` regardless.
- Fractional scale: niri prefers `wp_fractional_scale_v1`; this draws at the integer scale above it.
- Keyboard interactivity none and an empty input region: the background takes no clicks, and
  niri's own background colour shows nowhere it covers.

```sh
oag-game data/images/pulse-psp-eu.chd --no-audio --wallpaper    # on the niri session; Ctrl-C or kill to end
```

## Tests

`demo_ground_truth` (chains on Pulse, Pure and HD with every craft flown, the field `Rules::craft`, the screen over the race read and drawn, Pulse's spectator camera cutting, the headless
`--demo --dry-run` run, 2048/Omega's authored config, `--wallpaper --windows 3 --screenshot` writing three different pictures), `tests::windowed` in `oag-raceplay` (a demo in three windows: three craft, no person, the same state hash), `drive_ground_truth` (idle launch, press
restart, each input kind ending a demo, `--demo` chaining, `--wallpaper` ignoring input), and unit
tests in `oag_game::demo`, `oag_raceplay::demo`, `oag_tables::demo_config`.
