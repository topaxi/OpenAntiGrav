# Memory

Where the process's resident memory goes, and what keeps it from coming back.
The `Dev` perf overlay's memory line is the process's resident set size
(`crates/present/src/perf/memory.rs`, `memory_stats`); every number here is that
figure, read from `/proc/<pid>/status` (`VmRSS`), unless it says otherwise.
Engineering, not reverse engineering: nothing here describes the original.

## A second race in one session (glibc retention)

**Report from play.** Steam Deck, HD/Fury: the overlay read about 1.0 GB in the
first race of a session and 1.8 GB in the second (a Race Box race with the
`slipstream` mod first, then a campaign Time Trial with no mod).

**It is not a leak.** Reproduced on the desktop (Wipeout HD Fury, Race Box, the
`--drive` front-end walk, headless, `target/debug/oag-game`, Ryzen 9 7900,
load 3 to 6):

| Race in one process (RSS in the race, at tick 3000) | 1 | 2 | 3 | 4 |
| --- | --- | --- | --- | --- |
| iGPU (RADV RAPHAEL_MENDOCINO), before | 1250 | 1820 | 1823 to 1843 | 1882 to 1910 |
| iGPU, after | 1086 to 1089 | 1108 to 1127 | 1104 to 1106 | 1109 to 1126 |
| RX 7800 XT, before | 1202 to 1353 | 1242 to 1618 | 1637 to 1811 | 1708 to 1778 |
| RX 7800 XT, after | 1085 to 1100 | 1066 to 1111 | 1084 to 1087 | 1099 to 1113 |
| iGPU, Race Box Single Race + `slipstream` first, then Time Trial twice, before | 1432 | 1993 | 2296 | |
| same, after | 1226 | 1226 | 1235 | |

Every number is from `target/debug` (dependencies at opt-level 2); a release
build was not measured. `RssAnon` carries the whole growth and `RssFile` and
`RssShmem` stay at 70 to 80 MB and 0: the growth is anonymous heap, and the
overlay line would not show a GPU-side leak on this driver.

**Per-race or one-time?** Neither cleanly: it is allocator retention that jumps
at race 2 (+570 MB on the iGPU, +560 with `slipstream` first) and tapers off
(+0 to +60, then +300 in the `slipstream` flow). After the fix all races read
within 40 MB of each other, including the flow that loads the WebAssembly host
in race 1 only, so that host leaves nothing behind.

**What survives a race.** Nothing at the Rust level. `mallinfo2` (through a
`LD_PRELOAD` shim answering a signal) over four unfixed races shows the bytes in
use flat and the arena growing:

| | arena | in use (`uordblks`) | free in the arena (`fordblks`) |
| --- | --- | --- | --- |
| after race 1 | 856 MB | 164 MB | 692 MB |
| after race 2 | 1346 MB | 142 MB | 1204 MB |
| after race 3 | 1810 MB | 180 MB | 1630 MB |

A race load decodes and parses a few hundred megabytes of short-lived data on
several threads (`race-load`, `race-build`, `race-music`). glibc returns only the
top of a heap to the kernel by itself, so what is freed in the middle of an
arena stays resident, and the next load's allocations land past it. A
`malloc_trim(0)` after four races returned 1.2 GB (2271 to 1031 MiB). The
parked race is held (`Session::suspended_race`) and dropped on `LAUNCH RACE`
(`race_build::drop_off_thread`), so the Rust side was already releasing it.

**The fix.** `crate::heap::release_freed` calls `malloc_trim(0)` on glibc Linux
(a no-op elsewhere, one `#[allow(unsafe_code)]` with its SAFETY comment). It runs
where a lot was just freed, never on the frame thread:

- the end of `race_build::build` (the load's decode buffers are free once the
  scene is built), on the `race-build` thread;
- the `race-drop` thread, after it drops a parked or quit race.

A third trim at the race hand-off (a `heap-trim` thread in
`Session::finish_race_loading`, for what the swap of the loading stage frees)
was measured and dropped: with it off, the iGPU read 1090/1096/1108/1098,
1108/1101/1111/1095 and 1081/1110/1102/1119 over three four-race runs, the same
as with it on.

A trim takes 6 to 20 ms (`RUST_LOG=oag_game=debug`, "returned freed heap").
`a_race_let_go_is_dropped_and_the_heap_returned_by_the_drop_thread` pins the
drop thread's call (a canary dead and the release count up by one once the
thread ends) and `race_build_polls_never_wait_on_the_build` (disc-backed) the
build's.

**Measuring it.** An in-race number is easy to take at the wrong moment: the
drive's `ticks N` is unthrottled, a 3000-tick race lasts a quarter of a second,
and a pause (Escape) frees about 450 MB of the race's live state by itself, so
read the resident set while `stage=race` (poll every 20 ms), not from a
sample taken after the next state line. The pause-menu value drifts between runs
for the same reason.

**Is the first race itself bigger than a week ago?** No change beyond noise. A
build of `c476dee64` (2026-11-11, same walk, iGPU, two runs) reads 1194 and 1021
MiB in race 1 and 1221/1149 and 1667/1534 in races 2 and 3; this branch before
the fix read 1046 to 1255 in race 1. The old commit grows the same way.

**Does the lag track the growth?** Not in what the headless walk can measure.
The unthrottled sim in the walk runs 3000 ticks in 0.23 to 0.26 s in race 1, 2, 3
and 4 both before and after the fix (about 12000 ticks/s, load 3 to 5; a
Single Race with `slipstream` takes 0.7 to 1.0 s for its first 3000 ticks, wasm
module and opponents, and then 0.24 s in the Time Trials that follow). The
frame time and the input latency of a presented frame need a window and were
not measured on the Deck; an input is sampled once per tick into the
`InputSnapshot`, which does not depend on the heap.


**Reproducing it.** One process, several Race Box races, resident set read
while the race is the stage (the walk's `ticks N` is unthrottled, so a 3000-tick
race lasts a quarter of a second and a pause frees about 450 MB of the race's
live state by itself: poll `VmRSS` every 20 ms and keep the last value of
`"stage":"race"`, not a sample taken after the next state line):

```sh
# a profile of its own; MESA_VK_DEVICE_SELECT=1002:164e! is the Raphael iGPU, 1002:747e! the RX 7800 XT
export XDG_CONFIG_HOME=$P/c XDG_DATA_HOME=$P/d XDG_STATE_HOME=$P/s
env -u DISPLAY -u WAYLAND_DISPLAY MESA_VK_DEVICE_SELECT=1002:164e! \
  oag-game data/images/hdfury-ps3-eu.iso --no-audio --no-intro --state-log state.jsonl --drive '
    boot; select racebox; press cross; until page=race
    select launch_race; press cross; until screen=picker within 600; press cross
    until screen=picker within 600; press cross; until stage=race within 3000; ticks 3000; escape; ticks 60
    select quit_race; press cross; until page=main within 600; select racebox; press cross; until page=race within 600
    select launch_race; ...   # the same three lines again for each further race'
```

`slipstream` first: `OAG_MODS=assets/mods/official`, `[mods] enabled =
["oag.slipstream"]` in the profile's `settings.toml`, `set race.mode=single_race`
for race 1 and `set race.mode=time_trial` after. The live-versus-free split is
`mallinfo2` read from inside the process: `LD_PRELOAD` a shared object whose
constructor blocks `SIGUSR1`/`SIGUSR2` and starts a thread in `sigwait`; on
`SIGUSR1` it appends `arena`, `hblkhd`, `uordblks` and `fordblks` of
`mallinfo2()` to a file, on `SIGUSR2` it calls `malloc_trim(0)` first (a handler
doing the same can deadlock inside a `malloc`). Send `SIGUSR1` from the poller.

## A finished race held under its replay (ADR-0099 step 5)

**The "before" numbers.** [ADR-0099](adr/0099-a-run-context-and-an-activity-stack-own-navigation.md)
step 4 keeps a finished race resident under its replay viewer. These are the
figures on `main` at `03002a408`, where WATCH REPLAY still drops the finished race
(live heap at (c) matches (b): 977 against 959 MiB on HD; the phone's native heap
1007 against 989). All figures are MiB.

Points, all from one process walking one Time Trial flown by `--autopilot`:

- **pre**: the RACE page, before the first race.
- **a**: in the race, sim ticks 3000 to 3600.
- **b**: the finished race's end menu, the 700 sim ticks the walk holds it.
- **c**: the replay viewer after WATCH REPLAY from that menu, ticks 3000 to 3600.
- **d**: the main menu after leaving the viewer. **d trim**: the same after `malloc_trim(0)`.

Three runs per cell, each value the median over the window's samples, then over the
runs. Live heap agrees within 10 MiB between runs; (a) and (b) agree within 30 unless
a range or a note says otherwise; resident figures at pre, (c) and (d) move by up to
70. Titles: Pulse PSP
(`pulse-psp-eu.chd`) and HD/Fury (`hdfury-ps3-eu-dec.iso`, the decrypted image;
the table in the section above used `.iso`).

**Setup differences that matter.** The walk is headless (`--drive`) at 1440x816,
draws only at its screenshots and has no swapchain, so a windowed race holds more.
`--no-soundtrack` leaves out music decoding. Desktop is `target/debug` on glibc
(Ryzen 9 7900, Linux 7.3.6, glibc 2.44; load 0 to 20 across runs, memory did not
move with it). The phone is a Galaxy S21 (SM-G991B, Mali-G78, Android 14,
7.0 GiB) running the `dist` APK (Scudo), battery 28 to 34 C.

### Desktop

RSS is `VmRSS`. Live heap is `mallinfo2` `uordblks + hblkhd`, one reading per
window. VRAM and GTT are the process's own `/proc/<pid>/fdinfo` figures; **none of
them is in `VmRSS`**.

| Pulse PSP | pre | a | b | c | d | d trim |
| --- | --- | --- | --- | --- | --- | --- |
| RX 7800 XT: RSS | 178 | 278 | 279 | 332 | 338 | 226 |
| RX 7800 XT: live heap | 81 | 179 | 182 | 195 | 97 | 97 |
| RX 7800 XT: VRAM | 78 | 346 | 348 | 383 | 188 | 188 |
| iGPU (Raphael): RSS | 181 | 281 | 282 | 331 | 339 | 227 |
| iGPU: live heap | 81 | 182 | 185 | 198 | 100 | 100 |
| iGPU: VRAM + GTT | 64 + 15 | 201 + 155 | 201 + 156 | 233 + 161 | 33 + 162 | 33 + 162 |

| HD/Fury | pre | a | b | c | d | d trim |
| --- | --- | --- | --- | --- | --- | --- |
| RX 7800 XT: RSS | 350 | 1121 | 1122 | 1166 | 1005 | 359 |
| RX 7800 XT: live heap | 200 | 956 | 959 | 977 | 212 | 212 |
| RX 7800 XT: VRAM | 230 | 972 | 974 | 982 (to 1060) | 319 (269 to 383) | same |
| iGPU: RSS | 356 | 1123 | 1124 | 1170 | 1014 | 363 |
| iGPU: live heap | 200 | 959 | 963 | 980 | 215 | 215 |
| iGPU: VRAM + GTT | 184 + 47 | 258 + 736 | 258 + 739 | 234 + 771 | 154 + 244 | same |

`VmHWM` for HD is 1268 up to (b) and **1850 by (c)** (the replay's load). Today
that load starts with the finished race already dropped (VRAM falls from 974 to
406 within 0.3 s of the loading stage).

**A full field costs a little more than a Time Trial.** Same walk to (a) with
`race.mode=single_race` (eight craft; two runs, RX 7800 XT): HD live heap 1004
against 956 and VRAM 1046 against 972; Pulse 183 against 179 and about 350 against
346. A mid-Tournament leg, the case step 4 serves, is the full field.

### Galaxy S21

`resident + swap` is `VmRSS + VmSwap`, which includes the Mali buffers mapped into
the process. HD runs into zram, so `VmRSS` alone spreads by 700 MiB between runs
while the sum holds within 15 up to (b). `GPU` is `dumpsys gpu --gpumem`.
**`dumpsys meminfo`'s TOTAL PSS and RSS leave the GPU memory out** (its Graphics
row reads 2 MiB here).

| Pulse PSP | pre | a | b | c | d |
| --- | --- | --- | --- | --- | --- |
| resident + swap | 321 | 728 | 728 | 778 | 501 |
| native heap alloc | 61 | 180 | 183 | 197 | 89 |
| GPU | 39 | 339 | 339 | 339 | 104 |
| TOTAL PSS | 168 | 264 | 265 | 313 | 271 |
| TOTAL RSS (meminfo) | 272 | 368 | 369 | 417 | 375 |

| HD/Fury | pre | a | b | c | d |
| --- | --- | --- | --- | --- | --- |
| resident + swap | 690 | 2998 | 2993 | 3249 | 966 |
| native heap alloc | 188 | 984 | 989 | 1007 | 210 |
| GPU | 209 | 1869 | 1868 | 2124 | 335 |
| TOTAL PSS | 377 | 1035 | 1035 | 1057 | 556 |
| TOTAL RSS (meminfo) | 462 | 910 | 844 (356 to 1093) | 1009 | 529 |

HD fits today: all three walks finished. `VmHWM` reaches 3250 to 3363, and 22, 15
and 0 cached processes of other apps died during the three HD walks (none during
Pulse's), which is consistent with memory pressure; the `lmkd` lines were not
captured. HD's GPU figure is about twice the desktop's because Mali has no BC and
the disc's blocks are uploaded as RGBA8
([texture compression](../rendering/texture-compression.md)).

### What holding the race would cost

**An estimate, not a measurement**: (b) minus the floor (d), the race itself being
what (b) has and (d) does not. It assumes the two race scenes share nothing (the
texture upload cache is per `Scene::new`).

| | heap | GPU memory | together |
| --- | --- | --- | --- |
| Pulse, RX 7800 XT | 85 | 160 (270 against pre) | about 250 to 350 |
| Pulse, iGPU | 84 | 163 (278 against pre) | about 250 to 360 |
| Pulse, S21 | 94 | 235 | about 330 |
| HD, RX 7800 XT | 748 | 655, 591 to 710 (745 against pre) | about 1.3 to 1.5 GiB, half of it VRAM |
| HD, iGPU | 747 | 596 (766 against pre) | about 1.3 to 1.5 GiB, all system RAM |
| HD, S21 | 779 | 1533 | about 2.3 GiB |

- **Heap** is live heap on desktop and native heap alloc on the phone. Do not use
  `VmRSS` (b) minus (d): it reads 113 on HD and -59 on Pulse, because the race
  dropped on leaving the viewer is not returned to the kernel (the Open bullet
  below). `VmRSS` (b) minus **d trim** agrees with live heap on HD (762 against
  748) and not on Pulse (53 against 85, fragmentation).
- **GPU memory** has two floors because (d) retains more than (pre): the low
  figure is against (d), the high one against (pre).
- The phone's `resident + swap` (b) minus (d) is 228 (Pulse) and 2020 (HD), in
  line with the two columns added.

**The risk is the replay's load, not the steady state.** HD's load peaks 580 above
a first race load even with the finished race dropped. With the race held, an
upper estimate is today's peak plus the held race: about 2600 MiB of `VmRSS` on
desktop (1850 + 750), and about 5300 to 5700 MiB on the S21 (`VmHWM` 3250 to 3363
plus 2020 to 2312), on a phone with 7200 MiB in total that had 3000 to 3600
available with the game not running. Pulse is not a concern on any of these. **Steam Deck not
measured**; the iGPU row is the nearest proxy (shared system RAM). 2048, a desktop
release build and the VRAM ceiling were not measured.

### Reproducing it

The walk script comes from this generator (`mkwalk.sh pulse > walk.drive`):

```sh
#!/usr/bin/env bash
# mkwalk.sh pulse|hd  > walk.drive
hold() { for _ in $(seq "$1"); do printf 'key; ticks 1000\n'; done; }
case $1 in
  pulse) finish="ticks 2800; until endrace=photo within 1500
ticks 100; press cross; ticks 100; press cross; ticks 100"; end=8000; watch="press down down down" ;;
  hd) finish="ticks 2800; until endrace=other within 600
ticks 100; press cross; ticks 100"; end=7700; watch="press down down" ;;
esac
cat <<WALK
boot; select racebox; press cross; until page=race
set race.mode=time_trial
$(hold 250)
select launch_race; press cross; until screen=picker; press cross
until screen.id=ship-select; press cross; until stage=race within 4000
ticks 2800; until sim_tick=3000 within 600; screenshot SHOTS/a.png; until sim_tick=3600 within 700
$finish
screenshot SHOTS/b.png; until sim_tick=$end within 1500
$watch; press cross; until replay=playing within 4000
ticks 2800; until sim_tick=3000 within 600; screenshot SHOTS/c.png; until sim_tick=3600 within 700
escape; until page=replays within 400; ticks 30; press circle; until page=main within 100
ticks 60; screenshot SHOTS/d.png
$(hold 700)
quit
WALK
```

SHA-256 of the output, before `SHOTS` is replaced by a directory:
`pulse` `c0b69394eb8ac1032f98cf657bafdecc227cc43f24442a4227d064d321be3e05`,
`hd` `69ddcd222099875b836e63ecd4acea02052c0254ab26a80b47676d72e2e2c030`.

- **Why it is shaped so.** `ticks` runs unthrottled and `until` runs one tick per
  16.7 ms, so each measured point is held by an `until` on `sim_tick`. A menu has
  no tick to wait on, and a long `ticks` there starts the attract demo, hence
  `key; ticks 1000`.
- **Desktop.** A profile of its own (`XDG_CONFIG_HOME`, `XDG_DATA_HOME`,
  `XDG_STATE_HOME`), `OAG_MODS` and `OAG_TRACKS` on empty directories, no
  `DISPLAY` or `WAYLAND_DISPLAY`, `MESA_VK_DEVICE_SELECT=1002:747e!` (7800 XT) or
  `1002:164e!` (iGPU):

  ```sh
  oag-game <image> --no-audio --no-soundtrack --no-intro --seed 7 --autopilot \
    --drive "$(cat walk.drive)" --state-log state.jsonl
  ```

- **Phone.** Move `files/config` and `files/data/oag/replays` aside first and put
  them back after (the walk writes settings, records and a replay). Push the walk,
  write the [launch file](../tools/android.md), one entry a line, with `F` the
  app's files directory, then `am force-stop` and `am start`: `<image path>`,
  `--no-audio`, `--no-soundtrack`, `--no-intro`, `--autopilot`, `--drive-file`,
  `F/walk.drive`, `--state-log`, `F/walk-state.jsonl`, `OAG_MODS=F/walk-empty`,
  `OAG_TRACKS=F/walk-empty`. The `dist` build has no `--seed`, so the phone flies
  a different race (finish at sim tick 6833 against 6786 on Pulse). The headless
  walk drew no ANR.
- **Sampling.** Read the state log's last line together with the memory, every
  20 ms on desktop, back to back over adb on the phone (`/proc/<pid>/status`,
  `dumpsys meminfo <pid>`, `dumpsys gpu --gpumem`; keep a phone sample only if the
  window is the same before and after the call). Window rules: **pre** `stage=menu
  page=race` before the first (a); **a** `stage=race`, no `replay`, no `endrace`,
  `sim_tick` 3000 to 3600; **b** `stage=race`, `endrace=other`, `sim_tick` within
  700 below the walk's `end`; **c** `replay=playing`, `sim_tick` 3000 to 3600;
  **d** `stage=menu page=main` after (c).
- **Heap.** The `mallinfo2` shim described under "Reproducing it" above:
  `SIGUSR1` one second into each window, `SIGUSR2` (trim) 2.5 s into (d), with
  **d trim** read from 3.5 s on.

**After step 4 the walk's tail changes, and this tail has not been run.** EXIT pops
back to the end menu, so replace the `escape; until page=replays ...` line with:

```text
escape; until endrace=other within 400
ticks 100; screenshot SHOTS/e.png; until sim_tick=<end + 700> within 1500
press up up up; press cross; until page=main within 400
```

(`up up` on HD). Add a window **e**: `endrace=other`, no `replay`, after (c); it
should read like (b). Where focus lands after the pop is unknown, so read `e.png`
and adjust the `up` presses until the cursor is on RETURN TO MENUS.

## Open

- A race holds about 1.1 GB in play on this build (debug, opt-level 2 for
  dependencies) and the load peaks at 1.3 to 1.4 GB: the first race is not
  smaller for the fix. About 450 MB of the in-race live heap is gone as soon as
  the race is parked behind the pause menu; what it is has not been attributed.
- Windows and macOS were not measured; their allocators return memory
  differently and `release_freed` does nothing there.
- Not yet checked: the Steam Deck itself, the campaign Time Trial path (the
  walk used Race Box) and the second race's lag on a presented frame.
- A race that ends by finishing is dropped in place on the frame thread
  (`Session::open_menus`), not through `drop_off_thread`; the next build's trim
  covers its memory.
- After numbers owed by ADR-0099 step 4: re-run the walks under "A finished race
  held under its replay" with the race held, desktop and S21, HD first.
- Step 5's fallback (drop the held GPU scene) leaves HD's roughly 750 MiB of
  heap; `graphics.texture_compression=astc` on the phone is the other lever for
  the GPU half. Neither is measured (ASTC needs two races on a circuit, since the
  first fills the cache).
- Leaving the replay viewer for the menus drops its race with no trim: HD sits at
  about 1005 MiB resident there against 359 after `malloc_trim(0)`.
