---
status: blocked-hardware
titles: [all]
updated: 2026-11-18
summary: "Second-race RSS fixed on the desktop (malloc_trim); the Deck, the lag, the 450 MB in-race live heap and a campaign Time Trial are unchecked"
categories: [rendering]
size: S (60)
difficulty: medium (50)
---

# A second race holds as much memory as the first (desktop); the Deck and the lag are open

`crate::heap::release_freed` returns the heap a race freed to the system after the race build and after
the race drop; the measurement, the cause (glibc arena retention, not a Rust-level survivor) and the
walk that reproduces it are in [`docs/architecture/memory.md`](../../docs/architecture/memory.md).

## Open

- **The Deck.** Not run there. Release build of the branch, Dev overlay's memory line over Race Box +
  `slipstream` then a Time Trial, then a campaign Time Trial (Ubermall, Phantom, Feisar Fury hull), and a
  fresh start for the first-race baseline (the maintainer remembers 0.5 GB earlier; the desktop first
  race has been about 1.0 GB for a week, `c476dee64` read the same).
- **The lag in the second race.** Headless sim throughput is flat across races before and after the fix, so
  the growth does not explain it. Needs a presented frame: frame time and input-to-`InputSnapshot` latency,
  race 1 against race 2, on the Deck (`docs/tools/render-bench.md`, `--repeat`).
- **About 450 MB of the in-race live heap goes at the pause menu** (`uordblks` 575 MB in race, 165 MB
  paused). Not attributed: `systemd-run --user --scope -p MemoryMax=16G -p MemorySwapMax=0 heaptrack`
  on a `--drive` walk to tick 3000, then to the pause.
- **The campaign Time Trial path** (the maintainer's second race) was not driven; the walk used Race Box.
- **A finished race is dropped on the frame thread** (`Session::open_menus` replaces the stage in place),
  not through `race_build::drop_off_thread`.
- Windows and macOS read memory differently; `release_freed` does nothing there and nothing was measured.

## Next Steps

1. Build the branch release for the Deck, run the Race Box then campaign sequence, read the overlay.
2. If the Deck still grows, run the `mallinfo2` shim from `memory.md` there (`LD_PRELOAD`, SIGUSR1).
3. heaptrack the 450 MB, then decide whether it is a cache that could be built lazily.
