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
