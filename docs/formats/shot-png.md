# A screenshot's own description (`iTXt` `oag` and `oaGs`)

Maintainer's request, 2026-11-16: every PNG the engine writes of a race carries what is
needed to make the shot again. A bug report about a render is a photo-mode shot, and the file says
which title, race, camera and settings it was and holds the pilot's input stream. **Chosen, not
measured**: nothing here is an original's format. Code: `oag_game::shot_meta` (text and chunks),
`oag_replay::shot` (the `oaGs` payload), `oag_texture::png` (chunk writer and walker),
`oag_raceplay::replay::resimulate` (the re-drive).

Written by photo mode (`~/.local/share/oag/photos/`), `oag-game --race --screenshot`, and a drive
script's `screenshot`. The chunks sit between `IDAT` and `IEND`; a decoder that does not know them
skips them (both are ancillary), and the picture is the ordinary RGBA one.

## The `iTXt` chunk, keyword `oag`

UTF-8, uncompressed, TOML text (`oag-game --png-info FILE` prints it). Sections:

| Section | Holds |
| --- | --- |
| `format` | `1`; the layout of this text |
| `[shot]` | `tick`, `state_hash`, `width`, `height`, `source` (`photo mode`, `screenshot`, `drive script`), `stream`, and `no_stream_because` when there is none |
| `[title]` | `name`, `serial` (the disc's, which names the pressing), `platform` |
| `[race]` | `track` (the archive entry, never a path), `mode`, `class`, `team`, `seed` |
| `[camera]` | `lens`, `eye`, `forward`, `up`, `fov_deg`: the free lens included; absent for a shot with no camera of its own |
| `[camera.mount]` | written by a **track blur** shot only: how the camera follows its craft, so that each frame of the blur is drawn with it on the craft of its own tick (`oag_raceplay::photo::mount`). `follow` (`craft`: `eye` and `rotation` in the craft's frame, as the `internal` and `close` lenses are; `offset`: the eye relative to the craft in world axes at a world `rotation`, the free lens; `node`: `eye` is the circuit camera node the shot stood on, `rotation` the roll on top of the aim at the craft, the `track` lens), `subject` (the grid slot framed), `eye`, `rotation` (quaternion `x, y, z, w`), `fov_deg`. A ship blur holds the camera, which `[camera]` already gives. A track blur that predates this key is drawn again as one frame, and says so |
| `[picture]` | photo mode's picture controls, written on every photo mode shot, opening values included (a photo without it predates the keys): `exposure`, `saturation`, `speed_blur`, `depth_of_field` with `aperture` and `focus`, and with a motion blur `motion_blur` (`track` or `ship`), `shutter`, and `frames` (the number averaged); the panel's `0..=100` values |
| `[craft]` | position and rotation quaternion of the framed craft |
| `[render]` | the title's render profile as the settings file spells it (scale, MSAA, reconstruction, motion blur, ...) |
| `[gpu]` | adapter, backend, driver, driver info |
| `[[mods]]` | the enabled mods' ids and versions (photo mode) |
| `[build]` | the short commit (baked at build time, `oag_net::GIT_HASH`: check the repository out at it to reproduce), the crate version, and the submodule commits it pins. A tree with uncommitted changes is not marked, and a build made with the hash passed in (`just test-data` bakes `test-data`; a source tarball says `unknown`) names no commit |
| `[reproduce]` | `exact` (`oag-game --from-png`) and `flags`: the `oag-game` command line that draws this race and camera without the stream, in the shape of a compare-page pair's `ours.flags` |

**Nothing private is written**: no player name or tag, no filesystem path (the disc image is a
positional argument and is dropped from a screenshot's recorded command line, as is every other
argument that holds a path). The flags of a photo are synthesised (`--pose` puts the craft at its
position on the track's own heading, so a craft that was not flying along the circuit is only
approximated); the stream is what is exact.

## The `oaGs` chunk

`o` ancillary, `a` private, `G` reserved bit set, `s` safe to copy: a tool that edits the picture
may keep it. Data:

```text
u8 version (1), u64 tick, u64 state hash, then a replay file, raw-deflated
```

All integers little-endian. The replay is [the replay file](replay-file.md) (ADR-0055), cut at the
shot's tick, with no ghost and no pose track: its `HEAD` is the launch request, seed and options, `INPT`
the human slots' inputs to that tick (the AI reproduces itself from the seed), `EVNT` the
between-tick changes, `HASH` the stored hash a second. The hash at the shot's tick is its own field,
because a shot is rarely on a stored one. About 0.6 KB for 400 ticks (the throttle held), 5 to 6 KB
for 1,237 ticks of a pad flown by the AI driver (`bytes/tick` 6.8 in `replay_ground_truth`).

A shot made while photo mode was **rewound** carries the stream to the tick the race on screen
stood at (the rewind holds the run), and the hash of that state; a shot of a race that was not recorded
writes the text alone and says why in `no_stream_because`. A modded race records: the text's
`[[mods]]` carries each module's id, version and SHA-256 and the stream's header names the same
(`oag_replay::header::ModUse`).

## Reading it back

`oag-game --png-info FILE`: the text and a line on the stream.
`oag-game --from-png FILE [--screenshot OUT] [DISC]`: rebuilds the recorded race
(`Options::apply_replay`), drives the recorded inputs to the tick in place of a held button, prints
`from-png: state hash at tick N: MATCH` or `MISMATCH` with both hashes, and draws the frame from the
camera the text names at the file's size. **The disc must be the title's, with the same DLC and
mods**: DLC packs change the race and are not in the text (`resimulate` takes their directories, the
CLI mounts the settings'), and a simulation mod is named in the header with its SHA-256, so the
rebuild runs exactly the listed modules and **refuses by name**, before a tick, a module this
machine lacks or has another build of. The re-render draws the picture controls in `[picture]` (it prints
`from-png: picture controls applied: ...`, from `--picture`, which also sits in `reproduce.flags`), or
for a photo-mode shot without the section prints `no picture controls recorded: the shot predates
them, so it is drawn without any`. Like the mode, it draws none of the game's own motion blur (`--motion-blur` still overrides). **A motion blur shot is drawn from its frames**:
the plan the recorded kind, shutter and frame count make (`oag_raceplay::photo::shot::Plan::of`), as the mode took it, with the
race driven to the tick before the first frame and each frame drawn as the race passes its tick, at the alpha and lens sample the plan gives. A track blur has the camera ride the craft
(`[camera.mount]`), a ship blur holds the camera of the shot's tick. Each frame is read back before the grade, without the HUD, the
frames are averaged in 16 bits a channel and the HUD is drawn once over the average, as the session does (`oag-game`'s `photo::average`). The output says
`from-png: the shot is a track motion blur (...); drawing its frames` and `drew N frames from tick A to tick B`; a title whose photo mode
does not accumulate, a split screen or a track blur with no mount draws the one frame and says so. **A speed blur's ring has the history the paused mode
had**: the ring is drawn for 40 ticks of its own at the shot's moment before the frame is kept (its history weighs at most `0.25` a frame, so it has settled). Measured on HD (Anulpha Pass, tick 702, 1440x816, the redraw against the saved shot with its HUD on, outside the HUD's footprint: the pixels the HUD row changes, grown by 3):
a track blur 0.08 mean and 0.7 at the 99th percentile of the difference from its frames (11.5 and 75 from the one frame it used to be), a ship blur 0.45 and 2.0
(0.73 and 6.0); the craft a track blur's camera rides keeps 0.90 of its detail against the saved shot's 0.87 and the scenery 0.44 against 0.39 (a plain shot's redraw
differs from it by the HUD's own state alone). A speed blur at 100, mean difference along the two sides where the streak
lands (0.25 to 0.70 of the height): 1.10 and 1.38 against 2.11 and 4.10 with no history, for a shot taken after another in the visit, 1.05 and 1.81 for the first speed blur shot of a visit that
had a plain shot before it. Measured on HD only: the ring is HD's on every title, so the settled history applies to a speed blur redraw on any title and was not measured on another.
**The first shot of a visit to the mode carries a history the settled ring does not reproduce** (its streak is stronger; the redraw is 4.2 and 7.4 from it, 3.4 and 9.2 before; not a speed blur effect, the first shot being a plain one moves it to the shot after): two shots of one scene in one visit
differ by 3.5 on average, and the cause (what the preview frames left in the ring's history before the first shot) was not found.
The re-render draws
the HUD, which the text does not record: a photo taken with its HUD row off (the default) differs
there. A photo's re-render turns the visibility sets off, as photo mode does, and draws with the render scale and anti-aliasing of the photo's `[render]` section (at least 100 % and no upscaler) whatever the run's render flags (`docs/ui/photo-mode.md`, "A shot is drawn at the output's size"). **Default mods**: the
mods that ship enabled in a dev checkout (`oag.energy-turbo` among them) run a simulation module, so
the race is MODDED; it records its input and the shot carries the stream and the module hashes.
A shot of an unmodded race, remade in a checkout that has those mods, runs none of them. A shot from Race End
Photo is past the end of the recorded input (the recording stops at the finish) and has no stream
either; its text says so.

## Opening a shot in the viewer

`oag-game --replay-png FILE.png`, or the file dropped on the window over the main menu or an
attract demo, opens the stream in the replay viewer, paused on the shot's tick with the state hash
the shot was taken at checked: [replays](../ui/replays.md#a-screenshot-opens-the-viewer). The
viewer rebuilds the race by the session's own loader from the header, so a header must name
everything the race was flown with: **a `--race --screenshot` shot's header now carries
`[options] ai` (the original driver's request, when it flew) and `[setup] track` (the circuit the
load settled on when none was named)**. Shots made before that change open in the viewer only
when the race had no opponents or flew this project's driver on the default circuit; the
headless `--from-png` re-applies the command line's defaults and is unaffected.

## Evidence


- `crates/raceplay/tests/replay_ground_truth.rs`, `a_shot_survives_its_png_and_resimulates`: a
  shot at tick 1,237 (off the hash grid) of a pad-flown race on **Pulse and HD (and Pure, 2048)**
  goes into a PNG, comes back byte for byte, and re-simulates on a race rebuilt from the record alone
  to the same hash; a changed stored hash or a changed input before the shot is a mismatch.
- `crates/game/tests/photo_mode_ground_truth.rs`,
  `a_photo_carries_its_race_and_resimulates_to_the_hash_it_was_taken_at`: two real photo-mode shots
  on Pulse (the throttle held, no autopilot): at the pause (tick 402) and after a rewind (tick 90).
  Both read back and re-simulate to their hashes.
- CLI: `--race --screenshot` on Pulse (700 ticks, throttle held) and HD (900 ticks), then
  `--from-png`: `MATCH` on both, and the re-rendered pixels equal the original's (only the text
  chunk differs, for the flags). Both were made with `OAG_MODS` pointed at an empty directory.
- **A modded race** ([ADR-0091](../architecture/adr/0091-a-modded-race-records-its-inputs-and-its-header-names-the-modules.md)):
  `crates/raceplay/tests/energy_turbo_ground_truth.rs`,
  `a_modded_race_records_its_module_and_its_shot_resimulates_only_with_it`: a Pulse Single Race
  with `oag.energy-turbo` (the pickup given by a recorded event, fire held, the shield spent) shot
  at tick 600 re-simulates with the same module to the shot's hash, is refused naming
  `oag.energy-turbo` with no module, and naming it again for a build whose bytes differ by one
  custom section (same behaviour, other SHA-256); a recording that names no module runs none even
  where one is on offer. Without a disc, `raceplay/src/tests/extension_replay.rs` checks the
  header, the file round trip, a fresh modded race's `Verifier` run, every refusal reason and
  that no ghost is set. CLI on Pulse EU with the default examples on (`--race --mode single_race
  --class venom --give extension --hold cross --ticks 700 --size 640x360 --screenshot`): `--png-info`
  lists two `[[mods]]` with their SHA-256 and `# oaGs: 700 ticks ... state hash
  0x535974fd76a078bb at tick 700`; `--from-png` prints `state hash at tick 700: MATCH`, and the
  pixels equal the original's. With `OAG_MODS` set to an empty directory it exits 1 with
  `--from-png cannot make this shot again: the recording ran oag.energy-turbo
  (ext/energy_turbo.wasm) 1.0.0 and this machine does not load it`. Before the refusal was an error
  the same run drew a frame and printed `the rebuilt race differs before tick 0 (0x37cee5275358245f,
  the shot began at 0x4a450830be533ae8)` and `MISMATCH`: the modules' state is in the hash
  (`RaceSim::state_hash` calls `Extensions::hash_into`: blob, picks, fault tick, pending events).
  An unmodded shot (made with `OAG_MODS` empty) remade where the examples are found: `MATCH`.

## Checked against the other lineage

2048: **ported**, the same capture and photo code writes the chunks, and the raceplay ground truth
runs 2048 (shot at tick 1,237, hash found). Omega: **checked, applies, not run**: same code, and its
racing is incomplete (`omega-status.md`), so no Omega shot was re-simulated.
