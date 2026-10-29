# ADR-0080: A WebAssembly mod may own a screen in a third, frontend world, and save one small blob

## Status

**Proposed** (2026-10-29). A design for the maintainer to accept, change or reject;
nothing is built.

**Supersedes [ADR-0066](0066-the-menu-is-the-disc-plus-mods.md) decision 4 for
wasm-defined screens only**, including its closing consequence that "a mod changes
only the main page. It cannot add a page of its own, a setting, a race mode or a
choice row". For every other mod row, and every non-wasm mod, decision 4 and that
consequence stand unchanged. **Extends
[ADR-0079](0079-extensions-are-sandboxed-webassembly-guests-behind-two-worlds.md)
decision 1** (two worlds) with a third, and **narrows its decision 6** ("no menu
vocabulary") by exactly one allowance. ADR-0066 and ADR-0079 are not edited. Builds
on [ADR-0078](0078-a-mod-is-turned-off-by-id-live-and-the-built-in-never.md) (the
MODS page), [ADR-0055](0055-replays-are-inputs-and-a-ghost-is-poses.md) (replays) and
[ADR-0070](0070-the-network-transport-is-webtransport.md) (the lobby). The interface,
limits and worked behaviour are
[`docs/architecture/wasm-extensions/README.md`](../wasm-extensions/README.md) section
2.3; the engine meeting points are [`hooks.md`](../wasm-extensions/hooks.md) and
[`determinism-and-state.md`](../wasm-extensions/determinism-and-state.md).

## Context

ADR-0079's two worlds attach a mod to a race: a sim module that acts in the tick and a
HUD widget that draws in it. The maintainer's examples also want a mod to carry an
**editor**: a craft tune the player adjusts, a livery chooser, a custom-campaign grid.
That is a page, with pad and pointer input, that exists while no race does. ADR-0066
decision 4 forbids it: a mod row is a `submenu` or an `action` of the closed
vocabulary, "a mod cannot invent behaviour". ADR-0079 decision 6 repeats it ("no menu
vocabulary") and its non-capability table says a mod's extension kinds "attach to a
race, not to the menus".

Three facts shape any answer. The front end's screens are all `Input` and `Pointer`
in, `Outcome` and `Draw` out, and every new screen ships with mouse and touch in the
same change, so a guest cannot be left to re-implement input. A tune or a layout has
to survive between sessions, under the modern-saves rule of no slots. And a byte that
reaches a race is simulation input, with everything ADR-0079 says about determinism,
the replay header and the lobby.

## Decision

1. **A third world, `frontend`.** A guest exports `open(context) -> view` and
   `event(event) -> reply`, imports a namespace of `string`, `texture` and `log`, and
   runs on the UI side while no race exists. It has no determinism requirement, no
   state in any hash and no `World`. It **never marks a session MODDED**: only a sim
   module's `simulation = true` does.
2. **One new `Action`, `open-mod-screen`.** The vocabulary stays closed
   (`crates/ui/src/menu/vocabulary.rs:13` says a new action is added there and
   handled in the composition root); a mod row names the screen it opens
   (`screen = "<id>"`), declared by `[[extension.screen]]` in `oag-mod.toml`. A
   screen the mod does not declare is a load error naming the mod and key. A mod
   adds no row to a disc's own pages and no behaviour to them (ADR-0066 decision 4
   holds there).
3. **Host widgets plus a capped canvas, not raw draws.** The guest returns a retained
   **view**: widgets (`list`, `slider`, `toggle`, `choice`, `text-entry`, `button`)
   and a display-only canvas of the HUD world's `fill`, `sprite` and `text` draws.
   The host owns focus, pad, pointer and touch, scrolling, the title's skin and the
   navigation sounds, and sends the guest events already interpreted. This is the
   decisive reason: the pointer rule applies to a mod screen as to any other, and a
   guest that drew everything would have to implement it, mod by mod.
4. **Text is string ids** in the mod's own `strings/<language>.toml`, with
   `english.toml`'s `{ text, human }` convention; literal text with no id is refused.
5. **The same limits, one failure answer.** The fuel rewrite and memory ceiling of
   ADR-0079 decision 7 apply; fuel is recommended over a deadline (README open
   question 2). Caps are proposed (64 widgets, 256 canvas draws, 4 KiB blob). A trap,
   a refused value or an oversize blob closes the screen, discards that reply and
   names the mod. The module compiles when its row is first opened, off the render
   thread (asynchronously on the web), behind a loading state; nothing compiles at
   start-up.
6. **The only output is one blob, at most 4 KiB per mod and title** (the number the
   README already proposed for a save), a table in the records `Store` beside
   `campaign` and `loyalty` (`crates/game/src/records.rs:539`), written through
   `records::save_in_background` (`:1090`; inline on the web, `:1098-1102`). The cap
   is on raw bytes, before the TOML's base64. The blob is readable by that mod's
   screen and that mod's sim module only, and is listed and clearable on the MODS
   page.
7. **A blob enters a race at `race-start` only,** as `setup.tune`, to the same mod's
   sim module, which copies what it needs into its state blob; from there it is
   hashed with the rest of that blob. A blob no sim module reads never enters a hash,
   a replay header or the lobby fingerprint. The bytes are read once at race load.
8. **Replays and the lobby carry it (phase 3).** The replay header's `mods` entry
   carries the tune bytes themselves, so playback reproduces the run on a machine with
   another tune. A session-shared tune folds into `content_fingerprint` with the
   module hash. A per-seat tune is not folded in (it would refuse every join where two
   players differ) and is race-start input with the seat: an open wire question, not
   decided here.
9. **With no module loaded, nothing runs.** No wasmtime `Engine`, `Store` or instance
   is built until a race or a screen loads a module; the per-tick hook iterates an
   empty list. This applies to all three worlds and is tested (README section 4.5).

## Alternatives considered

- **Keep ADR-0066 decision 4 and ship editors as engine screens.** Every editor a mod
  author wants would be an engine change and a release; the mod system's point is
  that it is not. Rejected for screens the mod defines; engine screens remain for
  anything measured from a disc.
- **Raw `Draw` list only.** The smallest ABI, and the guest would own input. Pointer
  and touch parity, the drag threshold and the click sounds would be re-implemented
  and mis-implemented per mod. Kept only as the display-only canvas.
- **Let the screen write any file or a save slot.** Breaks modern saves and the "no
  files" non-capability; the MODS page could not list or clear what a mod left.
- **Write the tune straight into the sim state at `race-start`.** Bypasses the
  module's state blob and its hash coverage; a second path to audit. The guest copies
  from `setup.tune` instead.
- **Hash the tune but not store it in the replay.** Playback on another machine
  cannot reproduce the run; a replay is inputs that re-simulate (ADR-0055).
- **A fresh pooled instance per call, as in the sim world.** The rollback reason does
  not apply here: nothing in a menu rolls back, so the instance lives from open to
  close and may hold the screen's working state.
- **A `frame` export per frame.** Costs a guest call per frame while a menu is open;
  held as README open question 14.

## Consequences

**Good.** A mod can ship an editor with pad, mouse and touch support it did not have
to write, in the title's own skin, with nothing running until the player opens it. The
vocabulary grows by one entry. A screen-only mod needs no `simulation = true` and
marks no session. The save is one profile file the player already has, bounded and
listed.

**Bad.**

- **ADR-0066's "a mod cannot invent behaviour" is now false for wasm screens.** The
  invented behaviour is bounded (a page that cannot start a race, change a setting or
  read a file) but it is real, and a mod screen can look like a disc page. The MODS
  page and the row's source mod must make the origin visible.
- **The widget set is a ceiling,** like the command set in ADR-0079: a control the
  host lacks waits for a host change. A canvas is display-only on purpose.
- **A mod's work now runs while a menu is open,** with a compile on first open and a
  loading state to design on every target; the web build compiles asynchronously.
- **The tune does not close the multiplayer question.** A per-seat tune needs a wire
  change; until then a tune is single-player or session-shared.
- **The records file grows a table a mod controls,** capped at 4 KiB a mod and title,
  base64 in TOML. Uninstall policy and orphaned blobs are open (README open question
  16).
- **Disc data a screen may read is undecided** (README open question 15); the first
  editor may need a read-only table copy.

**Revisit if:** the widget set grows past a handful a month (then a general retained
tree deserves a look); or if per-seat tunes are wanted before phase 3 (then the wire
change is its own ADR, or this one is amended by a successor).
