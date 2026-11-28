# Gate wall time: what the lock is held for

A member's gate and the lead's batch gate run one at a time under
`$HOME/.cache/oag/gate.lock` (`.claude/skills/oag-drive/SKILL.md`, "Serialise
gates with `flock`"). With four or five members gating, a gate spends most of
its life waiting for the lock, so the useful number is how long each holder
keeps it. This page records what a hold is made of, what moved work out of it,
and how to read the log that now records every hold.

## The log

`scripts/gate-run.sh RECIPE` appends one tab-separated row per run to
`$HOME/.cache/oag/gate-times.tsv` (no header):

```
when  worktree  recipe  waited_s  held_s  prebuild  exit_code  load_1min
```

`prebuild` is `none` (no prebuild: the lock was free, or the recipe has none),
`done` (the prebuild finished while the gate waited) or `stopped@Ns` (the lock
came first and the prebuild was stopped after N seconds). `load_1min` is the
load average when the lock was taken. Summarise it:

```sh
mlr --tsv --implicit-csv-header label when,worktree,recipe,waited,held,prebuild,rc,load \
  then stats1 -a count,mean,p50,max -f waited,held -g recipe ~/.cache/oag/gate-times.tsv
```

`lead-test-data` takes the lock per slice itself and does not write the log; its
per-slice wait and run time are in its own output.

## What a hold is made of

Measured on this workstation (24 threads) with a probe edit to `oag-render`
(`pub fn` appended to `lib.rs`; `affected.py` selects `oag-render` and four
packages above it), two commits after the last build, before `gate-run.sh` existed:

| step | lock wait | hold | made of |
| --- | ---: | ---: | --- |
| `just gate-affected` | 916 s | **152 s** | compile in three profiles, `check-web`, `check-guests`, the quick tests (54 s and 44 s of test run) |
| `just test-data-affected` | 1,216 s | **598 s** | a 43 s debug build, then 2,068 tests in 552 s |

Compiling is most of `gate-affected` and little of `test-data-affected`.
`gate-affected` builds four things that share no artefacts: the clippy cone
(`lint`, `check-ballisticng`, `check-extensions`), the quick test cone
(`test-affected`, `check-schemas`, the two feature sets), the browser module
(`check-web`, its own target directory and a rebuilt std) and the mod guests
(`check-guests`, their own target directories). Each cold, one after the other,
at load 50 to 70 with a warm `kache`: workspace clippy 81 s, `oag-net` clippy
9 s, ballisticng clippy 45 s and build 42 s, extensions clippy 26 s and build
31 s, `check-web` 66 s, `check-guests` 38 s, the workspace quick build 45 s.

A new commit made it worse: `oag-net`'s build script embeds the commit hash
(`crates/net/build.rs`), so every commit rebuilt `oag-net` and everything that
links it, once per profile. In the `test` profile alone that is `oag-net` and
`oag-game` (59 s wall, 282 CPU-s at load 55, nothing else changed).

## What changed

- **`scripts/gate-run.sh`** runs a gate recipe under the lock (`flock -o`, so
  the file descriptor does not leak into children) exactly as before, and
  while the lock is busy it runs `just gate-prep` (`affected` scope for
  `gate-affected` and `test-data-affected`, `full` for `check`) at idle
  priority (`chrt -i 0 ionice -c 3 nice -n 19`): the same clippy, nextest
  `--no-run`, `check-web` and `check-guests` commands as the recipes, so the
  gate finds every artefact built. Nothing but the mod guests' own tests runs.
  It sits below the lead's `nice -n 10`, so a prebuild only takes cores the
  holder leaves idle (checked with `ps` on a real prebuild: every `rustc`,
  `clippy-driver` and `cargo` is in the idle class and in the prebuild's process
  group; only `kache`'s resident daemon, which restores cached units, keeps its
  normal priority). When the lock is taken the prebuild is stopped (a
  half-built cargo is safe to stop) and the gate finishes the rest at its own
  priority (stopping the process group left nothing behind and the next `cargo`
did not wait for a build-directory lock); a free lock is taken at once with
nothing prebuilt. An interrupted
  `gate-run.sh` stops its prebuild (the whole process group, until it is empty),
  signals a recipe that holds the lock through its own process and exits only
  once the recipe is dead, so the lock is never free while the recipe runs.
- **`OAG_GIT_HASH` and `OAG_BUILD_DATE` are fixed** for a `gate-run.sh` run, as
  `scripts/test-data.py` fixes them, so a new commit no longer rebuilds
  `oag-net`'s cone. No test reads the hash or the submodule pins beyond
  `oag-net`'s own self-consistency test; `just test-data` already ran every test
  with the fixed value. What this costs: a member's own `cargo build` or
  `just play` (hash unset) and a gate (hash fixed) rebuild `oag-net`'s cone when
  one follows the other in the same worktree, as the dev build and the gate
  did after every commit before. That rebuild now happens in the prebuild
  while the gate waits.
- **`just gate-prep`** is the prebuild, with the feature lists of
  `check-ballisticng` and `check-extensions` shared as justfile variables. A
  prebuild that falls behind a recipe costs the lock a little compile and never
  a wrong answer.
- **`just check-gate-run`** (offline, twenty seconds) holds the behaviour above
  against a stand-in `just` and a private lock.

## Measured

Same probe, same worktree, two commits later (`oag-render` again), through
`scripts/gate-run.sh`; the prebuild finished before the lock came:

| | before (`flock -o ... just <recipe>`) | after (`scripts/gate-run.sh <recipe>`) |
| --- | ---: | ---: |
| `gate-affected` hold | 152 s (load 35) | **89 s** (load 57), prebuild done |
| `test-data-affected` hold | 598 s (load 4), of which a 43 s build | **559 s** (load 8), prebuild done; 2,068 tests, suite 557 s against 552 s |
| the pair | 750 s | **648 s** |
| lock wait, `gate-affected` | 916 s | 1,354 s |
| lock wait, `test-data-affected` | 1,216 s | 2,156 s |

One run each, at different loads, so read the first two rows to a few tens of
seconds; the waits are other members' holds and say nothing about the change
(the queue held two to five gates throughout). Where the time went is in the
logs of the same runs: the ten cargo builds that finished inside the
`gate-affected` hold before sum to 358 s (they overlap, as the recipe's
dependencies run in parallel) and compiled 33 units; the nine that finished
inside the hold after sum to 42 s and compiled 5 units, 35 s of it one quick
build of the `-p` selection that the first prebuild did not cover (it now does,
`just prep-cargo`). `test-data-affected` fell by its build only: 43 s and 34
units before, 0.3 s and none after, with the same 2,068 tests and a suite time
within the noise. A hold with nothing selected, everything prebuilt, was 51 s
(this branch's own gate), which is the quick unit tests' 49 s.

The hash pin, on a branch where only HEAD moved (an empty commit), builds in the
same worktree at load 41 to 87, wall seconds and `oag-*` units recompiled
(`kache` warm, so a unit whose inputs were seen before restores in about a second):

| profile | hash unset | hash fixed |
| --- | ---: | ---: |
| `lint` (clippy, `--workspace --all-targets`) | 23.3 s, 2 units | 0.8 s, 0 |
| `test` (quick, `--no-run`) | 36.4 s, 2 units | 0.9 s, 0 |
| `test-data` (debug opt-level 2, `--no-run`) | 45.3 s, 2 units | 1.2 s, 0 |

The first build after the setting changes recompiles the two units once (1.4 s
here, restored from `kache`); the unset side never has anything to restore,
because every commit is a new hash.

## What the test-data hold is

A member's `test-data-affected` is bound by the `gpu-vram` group, not by CPU.
In the 2,068-test run above, 321 tests are in the group and their summed time
is 1,788 s: at the cap of 4 that is 447 s of the 552 s of test wall clock, and
the load average sat at 4 to 14 on a 24 thread machine while it ran. Replaying
that run's durations through a 24 slot scheduler with the cap gives 514 s with
the queue as it was, 506 s with every group test first and 719 s with them last,
so queue order buys at most 8 s: the floor is the group's summed seconds. Per
group module the most seconds are `campaign_cursor_ground_truth` (210 s in 33
tests), `drive_ground_truth` (193 s, 24), `pause_menu_ground_truth` (186 s, 26)
and `net_rejoin_ground_truth` (113 s, 3); a typical test is one launch of
`oag-game` at 6 to 8 s. The lead's four slices are close to it too: 181 to
210 s of wall clock against 118 to 133 s of group floor and 100 to 120 s of
CPU floor per slice (the four slice logs of the lead's `just lead-test-data` on
2026-11-23: wall clock from each `Summary` line, group seconds from the test
names against `.config/nextest.toml`'s `gpu-vram` modules).

Less group time, not a different order, is what shortens these holds: fewer
launches per test, a launch that starts faster (`docs/architecture/load-time.md`),
or a larger cap (the cap exists for an 8 GiB card).

### What a launch costs (second round)

A launch of the game is 0.9 s alone for a menu test and 2.7 to 4.4 s for one
that races, and the group's summed seconds are those launches' CPU under
contention: the gate's mean of 6.4 s per `campaign_cursor_ground_truth` test is
2x its mean at the cap alone. So the launch's CPU is the lever, not only its
critical path. The profile, the table of phases and the two changes that came
out of it (a pool of opened CHD handles, a faster PSARC path digest: user CPU
of a campaign-cursor run 243 s -> 171 s) are in
`docs/architecture/load-time.md`, "A scripted launch".

### Third round

Three more changes to the launch (`docs/architecture/load-time.md`, "Third round"),
by user instructions of the whole module at the gate's profile:

- The parsed PSARC manifests are shared by every open of the same bytes: a menu
  launch 4.85 G -> 3.24 G, a race launch 20.5 G -> 18.9 G.
- The drive's `config` step switches `controls.menu_back` from any screen, so
  `campaign_cursor_ground_truth` (1,753 G -> 1,141 G, wall 60 to 70 s -> 42 s) and
  four `footer_hint_ground_truth` tests (671 G -> 545 G) launch once, not once
  per choice. The last per-choice launches followed on 2026-11-28 (the two modules'
  user CPU 75.8 -> 69.4 s, summed test seconds 83.1 -> 77.3; `load-time.md`, same
  section). No test loops a launch over MENU BACK any more.
- Measured and not changed: the race build's pipeline compile (a disk pipeline
  cache adds about 1 % over mesa's own shader cache, which already takes a racing
  launch from 65.7 G to 24.0 G instructions, and naga's work runs before it; one module per stage 1.5 to 2 %), the PS2 music read.
- Measured and not changed (`load-time.md`, "Fifth round"): SPIR-V written at build time
  and passed through. naga is about 31 % of a race launch's instructions, but 29 of it is
  the mesh pipelines, whose override constants exist only at run time; what a build-time
  blob reaches is 1.6 %.
- Measured and not built (`load-time.md`, "Sixth round"): resolving the override constants
  once per distinct stage and handing `wgpu` the IR, no `unsafe`. Race walk -6.2 %
  (22.01 -> 20.63 G user instructions), bare race launch -8.6 %, one frame byte-identical;
  under the 10 % bar. naga 30.0.2 hands the resolved module back with its overrides arena
  still filled, which makes `wgpu-hal` resolve it again (+4.3 % worse) until it is cleared.

## Still open

- `lead-test-data` builds under slice 1's lock when `gate-run.sh check` was not
  run first in the same worktree (the `full` scope prebuilds the same debug
  workspace build); a lead who starts it cold pays the build there.
- `flock` does not queue in arrival order, so one gate can wait much longer than
  the rest (waits of 665 and 1,681 s for equal slices on 2026-11-23). The mean
  wait is unchanged by order; its spread is not.
- A `gate-run.sh` killed with `SIGKILL` leaves its idle-priority prebuild to
  finish on its own.
