# The finish hand-over, and the spectator camera that follows it

How a craft that has crossed the line for the last time ends up flown by the AI while the
race keeps running behind the end-race panels, and what the camera does there. The behaviour
and its measurements are [`after-the-finish.md`](../../../gameplay/after-the-finish.md); this
page is the code. Image base `0x08804000`; `psp-pulse-usa`, `BOOT.BIN`.

Read from the decompiler (Ghidra headless, 2026-10-01) and **checked against live PPSSPP
runs** (breakpoint on `Ship_SetState`, write watchpoints on `craft+0x1d4` and on the control
record, a per-frame log of every craft; `scripts/psp-postrace.py`).

## The finish, craft by craft

| Address | Name | Confidence | What it is |
| --- | --- | --- | --- |
| `0x08842a18` | `Craft_UpdateLapProgress` | (already named) | at crossing `laps + 1` sets `entity+0x912` (finished) |
| `0x088418e0` | `FUN_088418e0` | not renamed | the per-entity update. For an entity with `+0x368 == 0` (the local player) or `2` (an AI craft): `if (entity+0x912 != 0 && Ship_State(entity) == 1) Ship_SetState(entity, 2)`; and `if (!(entity+0x860 & 0x800000)) Craft_SetAutopilotBlend(0.0, craft)` every frame |
| `0x08848664` | `Craft_SetAutopilotBlend` | 90 | `*(craft + 0x1d4) = value`. The autopilot weight |
| `0x0883b3b8` | `PlayerStatus_Update` | (already named) | the local player's per-frame status. Sets `craft+0x1d4` **`1.0`** when `entity+0x912 != 0 \|\| Ship_State(entity) == 2`; while the Autopilot pickup runs (`weapon record +0x1b8 & 0x800`) it sets the weight to `min(1, record+0x148)` and `craft+0x1c0 \|= 4`; otherwise `0.0` |
| `0x08849618` | `Ship_UpdateCraft` | (already named) | `craft+0x78 = craft+0x3c`; if `craft+0x1d4 != 0`, copies the pad's `0x28` bytes to the blend buffer `craft+0x44`, **then the record at `craft+0x40` over it**, and points `craft+0x78` at the buffer. A weight of any non-zero value therefore hands the craft to the AI's record whole, not a mix |
| `0x0883c870` | `PlayerInput_Update` | (already named) | writes the pad into the `player_input` record (`+0x44` steer, `+0x48` thrust, `+0x4c`/`+0x50` airbrakes, `+0x54` pitch) |

**Live** (Talon's Junction; the pad released and the pickup off 40 frames before the line):

- `craft+0x1d4` reads `0.0` on every one of the 40 frames the pad flew the craft and `1.0` on frame `F`, the frame
  `entity+0x912` and `craft+0x2a4` (the craft state) both read their finished values.
- `craft+0x78` (the control record the craft reads) is the pad's record (`craft+0x3c`) up to `F` and the blend buffer
  (`craft+0x44`) from `F+1`. `craft+0x3c` and `craft+0x40` never change.
- `Ship_SetState(entity, 2)` was caught **eight times in one Single Race, all from `0x08841ab8` (inside
  `FUN_088418e0`)**, the player's first and the seven opponents' at their own crossings. No other caller of
  state 2 fired.
- The control record is the AI's from `F+1`: `[steer, thrust, 0, 0, 0]` reads `[23.5, 100.0, 0, 0, 0]`, then `[-11.1, ...]`,
  `[18.2, ...]` (a bang-bang follower's steer), and the thrust slot goes `100 -> 56.7` and stays.

### `Race_FinishAllCrafts` is not this path

[`grid.md`](grid.md) and [`shield.md`](shield.md) say `Race_FinishAllCrafts` (`0x08824e10`) "is the flag":
it switches every craft to `autopilot_input` / `AI_input_%d` and sets state 2. That is **true of the
function and wrong for Single Race and Time Trial**. Its only caller found is `FUN_0882461c`, whose only
caller is `FUN_088dfb90`, inside the `GriefReport_*` screen code (a multiplayer path). **Live, it never ran**: the
mode flag it sets (`manager+0x1a78`) read `0` through every capture; the player's `entity+0x368` stayed
`0` (the function sets `2`); `flags860` stayed `0x600` (it sets `0x800000`); and no driver pointer changed. The
real finish is the per-craft rule above. The function and its names stand (confidence unchanged); what is
corrected is that nothing in these modes reaches it.

## The mode state and the HUD

`ArcadeRace_UpdateRacing` (`0x0882c5c4`, [shield.md](shield.md)) tests `player.flags & 0x1000 \|\| Ship_State(player) == 2`, then
`Hud_Hide`, `Race_BuildEndRaceResult`, `RaceMode_SetState(3)`. **Live**: `manager+0x7c8` goes `2 -> 3` and
`g_hud+0x168` goes `0 -> 1` on frame `F+1`, a frame after the craft's state reads 2.

## The spectator camera

The global camera object (`0x08b32c64`, constructed by `FUN_0887f9bc`) has a **spectator director** the
race-end flow drives. Static reading, with the timing checked live.

The functions below are named on [`camera.md`](camera.md#the-destroy-camera-mode-5-read-and-measured-2026-10-01)
(read independently by the destroy-camera lane the same day, from the same code): `Camera_UpdateSpectator`
`0x0887fd3c`, `Camera_UpdateSpectatorView` `0x08880c04`, `Camera_SetSubject` `0x0888058c`, `Camera_PickStation`
`0x0887fedc`, `Camera_FramingFov` `0x08880984`, `Camera_RepickNearSubject` `0x08880168`. This page adds two:

| Address | Name | Confidence | What it does |
| --- | --- | --- | --- |
| `0x08880a58` | `Camera_PickSubject` | 85 | starts from the player (`manager+0x2c0`); if the drawn craft (`+0x1e4`) carries `flags(+0x860) & 0x1000` it is reset to the player; then while the subject equals the drawn craft and more than one craft is live takes a random live craft (`rand() % count`). With `cam+0x274` set the subject is the global `DAT_08ab0df8` instead |
| `0x08880b38` | `Camera_PickRandomMode` | 85 | `Psys_RandIntRange(0, 100)`: `< 26` mode `3`, `< 51` mode `2`, `< 76` mode `6`, else mode `7`; called when `Camera_UpdateSpectator` takes a new node. **Gated on `cam+0x26c`** (a byte, `1` from the constructor): with it zero the function changes nothing, which is what `Ship_SetState` case 4 does at a player's wreck |

`Camera_SetMode` (`0x08880724`) stores the view width `+0x268`: mode `5` -> `35.0`, `6` -> `17.0`, `7` -> `50.0`; the
constructor leaves `60.0`. `Camera_PickStation` and `Camera_RepickNearSubject` pick by the node's **aim point**.

`Camera_UpdateSpectator(dt)`, every frame: `+0x3c += dt`; past `10.0` s it resets the timer, clears the subject (unless
`+0x274` is set) and `+0x1e8 = -1`. It re-picks a cleared subject, then: with no node, takes the nearest; with a node,
runs the 60-unit test, and **if a new node was taken** it rolls a random mode and records the subject as the previous
one. Then it renders.

**Live**: the subject became the player on frame **`F+61`** in all four captures and was re-picked at **`F+661`** and
**`F+1261`** (600 frames, `10.0` s) in both Single Race captures, each time a different craft from the last (a Time Trial has one
craft, so its subject stays the player). The mode changed at irregular frames among `3`, `2`, `6`, `7`, which is the node test firing.

### The node cameras (modes 5, 6, 7)

One body. The camera sits **at the node** (`node+0x90`), looks at a **smoothed** subject point and zooms:

```text
smoothed += (subject_position - smoothed) * rate          // +0x230 toward +0x240, rate = +0x250 (= +0x264)
fov      += (target_fov - fov) * 0.06                     // +0x220 toward +0x224 (FUN_08880984), rate +0x228 (= 0.06)
look      = node_position - smoothed
look.y   *= max(1 - fov * 0.008, 0.4)                     // squash, so a wide zoom keeps the horizon
view      = look_at(look normalised, up = (0, 1, 0))      // FUN_0897019c
```

`rate` is `0.4 / 0.5 / 0.6 / 0.6` for speed class `0..3` (the constructor sets `0.3` by default), the
fov rate `0.06` (`0x3d75c28f`). Mode `5` is the death camera (`Ship_SetState` state 4 calls `Camera_SetMode(cam, 5)`), so
**the destroy camera and this one are the same code with a different view width**: the port shares it
(`oag_render::camera::destroy`: the station pick, the framing field, the pose).

**The node list was read live** (`16_Track`, forward track, PPSSPP): ten nodes at `cam+0x40`, each with a unit vector at `node+0x80` (the camera's back axis), the **eye at `node+0x90`** and an **aim point at `node+0xa0`**; the eye is the track `.vex` node's world translation and the aim is the three floats at `+0x10` of its `Camera` payload, all ten matching to 0.1 unit and in file order. The director picks nodes by the aim point and sits at the eye. Ported as `oag_game::race::finish_camera`.

Modes `2` and `3` are read and measured: a rigid rear view and a rigid front view, see
[camera.md](camera.md#the-spectator-views-craft-relative-modes-and-the-directors-hand-off-rules-2026-10-02).
**Not read**: cases `0`, `1`, `4` and `8` of the view, what drives `+0x274`, and the trigger that makes `Race End Photo` call `FUN_08880788` on frame `F+61`.

## Names

`Craft_SetAutopilotBlend`, `Camera_PickSubject` and `Camera_PickRandomMode` are in
[`names.tsv`](names.tsv). Confidence: the setter is a one-store function whose effect was watched live (a write
watchpoint on `craft+0x1d4` caught it from the two call sites above); the two camera names rest on the decompile and
on the timing the captures reproduce (the `10.0` s re-pick to the frame, the `F+61` start, the mode set), not on a
breakpoint inside them.
