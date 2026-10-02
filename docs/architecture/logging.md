# Logging

What a launch prints, which level a message belongs at, and how to get the rest
back. The sink is `env_logger` over the `log` facade, installed by the binaries
alone (`oag-game`, `oag-view`, `oag-trace`); a library only calls the macros. The
simulation crates log nothing - see the diagnostics note in the workspace
`Cargo.toml`.

## The default

`warn` globally, our own crates (`oag*`) at `info`, `calloop` at `error`. A
launch to a race prints the disc it found, the renderer it chose, the race it
started and anything degraded or missing, and nothing else. The format is the
level and the message, with no timestamp and no module path.

`warn` stays the floor for everything that is not ours, because at `info` the
graphics stack narrates every adapter, shader module and pipeline it builds.
Those lines are not this project's to classify: a Vulkan layer failing to load
on the player's machine, or a validation `PERFORMANCE` warning in a debug build,
arrives through `wgpu` at whatever level the stack chose.

## Which level

One question sorts a message: **what does a player lose, or want to know?**

| Level | What it is | Examples |
| --- | --- | --- |
| `error` | Something the player loses. | An asset the race needs failed to load; the device was lost; a save could not be written. |
| `warn` | A degraded but working path. | A fallback is in use, an optional asset is absent, a texture was trimmed to a smaller mip, and every loader line that says something **draws or plays nothing**. |
| `info` | A lifecycle line a user would want to read. | The disc image, the audio device, the renderer, a race starting (`racing on <title>: <circuit>`), the grid, a ghost saved, leaving the race, a pilot saved or deleted, a conversion that explains a wait ("decoding ... once; cached after this"). |
| `debug` | What a load did. | Counts, sizes, timings, loader reports, screen transitions, which language was picked. |
| `trace` | One line per item. | One line per sound cue, per ship, per frame or tick. |

Two things that are not obvious from the table:

- **An absence stays at `warn`.** The project rule is to draw nothing and say so
  ("Never invent what the assets already author" in `CLAUDE.md`); hiding that
  statement at `debug` would hide the absence it exists to report. A message
  that is only a count of what *did* load is `debug`, even when it is long.
- **A flag or environment variable the user passed is acknowledged at `info`.**
  `--prefetch` progress, `--autopilot`, `--zone-stage`, the shadow dump from
  `OAG_DUMP_SUN_OCCLUSION`: the user asked, so the answer is theirs to see at
  the default filter. `--trace` raises screen transitions to `info` for the
  same reason. A line that **reports the result of a `--screenshot` or
  `--trace-out` capture** is the same kind of thing (`frame at tick N: ... draw(s)
  submitted`), and `wreck_ground_truth` parses it: leave such a line where it is
  and grep `crates/*/tests` and `scripts/` for its text before moving any.

## Loader reports

The loaders (`race::load`, `boot`, the sound banks, the preview cards) return
their account of themselves as a `Vec<String>`, because a test or a `--dry-run`
reads the same strings. The strings carry no level, so they all go to the log
through `oag_game::loader_log`:

- `loader_log::lines` logs a report at `debug` and each line that says an asset
  is absent or was not used at `warn`.
- `loader_log::lines_at(Level::Trace, ...)` is the same for a report with one
  line per item.
- One report puts at most eight absences at `warn`. The rest go to `debug`
  and one more `warn` line says how many there were: 2048 lacks thirty-odd
  particle effects, and thirty near-identical lines bury the three that differ.
  The cap is `WARN_CAP`; nothing is dropped, only moved.
- What counts as "says an asset is absent" is one list, `ABSENCE` in
  `crates/game/src/loader_log.rs`, with a test over lines taken from real
  reports. A loader that learns a new way to say "this draws nothing" adds its
  phrase there; a phrase missing from the list hides an absence at `debug`,
  which is why the list errs towards `warn`.

Call `loader_log` rather than `info!("{line}")` in a loop. A loop at `info` is
how a launch got to print four hundred lines.

## What stays on stdout

A `println!` is a command's output, not a note about it: `oag-unpack` tables,
`oag-wad` listings, `oag-trace` CSV and summaries, `oag-view` reports, the
`--screenshot` and `--trace-out` result lines, and the key hints the game prints
for a player at the terminal. They are what a user redirects, so `RUST_LOG` does
not touch them. Runtime chatter belongs on the logger.

## Getting the rest back

`RUST_LOG` replaces the whole default expression, so name `warn` again unless
you want the third-party crates at their own level.

```sh
RUST_LOG=warn,oag=debug oag-game ...                 # what every load did
RUST_LOG=warn,oag=trace oag-game ...                 # every line there is
RUST_LOG=warn,oag_game::audio=trace oag-game ...     # one module
RUST_LOG=warn,wgpu_core=info oag-game ...            # the graphics stack as well
RUST_LOG=warn,calloop=warn oag-game ...              # put back the key-repeat noise
RUST_LOG=error oag-game ...                          # only what the player loses
```

The defaults live in `init_logging` in `crates/game/src/main.rs` (which carries
the reasoning for `calloop`), `crates/view/src/logging.rs` and
`crates/trace/src/logging.rs`.
