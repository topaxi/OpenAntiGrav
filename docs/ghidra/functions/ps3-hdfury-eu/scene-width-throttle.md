# HD's 1080p scene width: 32 prebuilt targets and a one-step-a-frame controller

2026-11-10, lane `hd-scene-width`. HD's 1080p path draws the scene narrower than
1920 and stretches it in the final stage
([hd-track-tone.md](../../../rendering/hd-track-tone.md), "The 1080p path renders a
reduced, dynamic width"). This page has what moves the width: where the level lives,
the code that steps it, its inputs, and how to read or pin it on RPCS3.

Read on disc 02.00 (`BCES00664`), live on RPCS3 0.0.43-20574 at 1920x1080, scale 100,
in the attract race (Ubermall). 02.51 was read statically only.

## The law

`level` is a `u32` at `g_RenderGlobals + 0x88` (`0x00c81b08`), 0 to 31. The scene's
width is `1920 - 20 * level`: 1920 at level 0, 1300 at level 31, always 1080 lines.
So the floor is **1300**, not the 1280 an earlier reading guessed, and there are 32
widths, not 32 steps below 1920.

Once a frame, at the end of `Render_FinishFrame` (`0x003e3320..0x003e33cc`):

```text
slot  = 1 - slot                              ; g + 0x4da0, two report pairs alternate
r     = reports[slot]                         ; g + 0x4d98 + 4 * slot
if r != 0:
    us      = (r[1].timer - r[0].timer) / 1000    ; RSX report timestamps, ns to microseconds
    on_time = Gcm_FlipOnTime()
    if us > 16111 or not on_time:             ; cmplwi r29, 0x3eef
        if level < 31: level += 1             ; narrower by 20 px
    elif us <= 15555 and level != 0:          ; cmplwi r29, 0x3cc3
        level -= 1                            ; wider by 20 px
    if colour_targets[level].height < 1080:   ; g + 8 + 4 * level, +0x20
        level = 0
    if not enabled:                           ; g + 0x324
        level = 0
```

| Part | Value | Confidence |
| --- | --- | --- |
| The variable | `g_SceneWidthLevel`, `0x00c81b08`, width `1920 - 20 * level` | 92 |
| Bounds | level 0 to 31, 1920 to 1300 px | 92 |
| Step and rate | one level (20 px) a frame, either way, or hold | 90 |
| Narrow when | the timed span is 16112 us or more, **or** the last flip was not on time | 88 |
| Widen when | the timed span is under 15556 us and the last flip was on time | 88 |
| Hold | 15556 to 16111 us with an on-time flip | 84 |
| Off | `g_SceneWidthThrottleEnabled` clear: level 0 every frame | 92 |
| 1080p only | a level whose target is under 1080 lines resets to 0 | 80 |

**What is timed.** `Rsx_SetReport(ctx, 1, offset)` writes `0x00041800, 1 << 24 | offset`
(`NV4097_GET_REPORT`). A report is 16 bytes and its first 8 are the RSX's timestamp
when the command is reached. `Render_FinishFrame` takes two reports from
`Gcm_AllocReports(2)` at its end and issues the first at once, so it lands at the head
of the next frame's commands; the second is issued in the same function on the next
frame, after the resolve and the post chain and before the flip. Their difference is
the time the RSX spent between the first and the last command of one frame. The pairs
alternate between two slots, so the controller reads a span two frames old. Read live
in the attract race: 3584 us and 2648 us in the two slots with the level at 1 and
falling.

**On time.** `Gcm_Flip` (`0x005cb7b8`) reads the time base after its flip call and
sets a byte: `(time_after_flip - last_vblank_time) - 1 < 29999` ticks, 0.376 ms at the
PS3's 79.8 MHz. `last_vblank_time` is the time base stored by `0x005ccc60`, a callback
that also counts a flip interval down; it is taken to be the vblank handler
(confidence 60, its registration was not read). So a frame is on time when its flip
returned right at a vblank, that is when the game was waiting for the display and not
the reverse. Live, 100 Hz samples: byte 1 with level 0 28,242 times and with a level
above 0 148 times; byte 0 with a level above 0 3,348 times and with level 0 21 times.

**Why RPCS3 shows 1300 so often.** Both inputs are wall-clock measurements. On a loaded
host the emulated flip misses its vblank and the level climbs to 31 whatever the scene
costs: the level sat at 31 (1300) while the host's load average was over 60 and roamed
0 to 31 once it fell. Nine of the thirteen stored race originals are at
1300. A PS3 at 60 Hz would move it with the RSX's real load; nobody here has measured
one.

## The targets

`g_RenderGlobals` holds six arrays of 32 pointers, one entry a level, at `+0x08`
(`g_SceneColourTargets`, `0x00c81a88`), `+0x8c`, `+0x10c`, `+0x18c`, `+0x20c` and
`+0x28c`, with the level word between the first and the second. Each entry is a
texture object whose `+0x1c` is its width and `+0x20` its height; the objects are heap
allocations `0x1e0` apart (`0x30025b00` for 1920 through `0x300360e0` for 1300 on this
boot). `Rsx_SetRenderTargets` (`0x005a40f8`) copies the colour target's `+0x1c` and
`+0x20` into a surface block and `Rsx_EmitSurfaceBlock` (`0x005c1b80`) writes it to the
command stream as one `0x00200200` packet: clip horizontal, clip vertical, format,
pitches and offsets. That packet is where a frame dump shows the width.

Only the first array's role was read (the scene's colour target: its `+0x20` is the
1080-line test, and repointing all six moves the scene). The other five are the depth
target and the resolve sources by position (`+0x8c` or `+0x28c` feeds
`Post_BindResolveParams`, chosen by `g + 0x414`); which is which was not separated.

## Who switches it

`Render_SetSceneWidthThrottle` (`0x003de3c8`) stores its argument at `g + 0x324`. Its
callers, through the thunk `0x00676a28`:

| Call site | Argument | Where |
| --- | --- | --- |
| `0x00031740`, `0x000320b0` | 1 | two race set-ups (`FUN_000316d8`, `FUN_00032048`) |
| `0x00031f1c`, `0x0003288c`, `0x000329fc` | 0 | three tear-downs (each stores a vtable first) |
| `0x0023f64c` | 0 | `PhotoMode_Update`, entering Photo Mode |
| `0x0023f824` | 1 | `PhotoMode_Update`, leaving it |

A `stb r0, 0x324(r3)` at `0x0038a99c` sits in an initialiser whose object was not
identified. So the menus and Photo Mode draw 1920 and a race throttles. Live: `Main Menu` read enable 0 and level 0 on 416 of 416 samples with only
1920 in the ring; the byte went to 1 at `Demo Launch`, before the race was on screen,
and the level reached 31 during the load while the loading screen itself was drawn on
1920-wide surfaces (six 1920x1080 surface blocks a frame in the ring, single stray
ones at the level's width).

`Debug.Enable Frame Buffer Throttling` (registered at `0x003a9464`) was not connected
to this byte: no reader of the option was found, and the byte's seven writers are the
calls above. Whether the option reaches one of the race set-ups is open.

## The names

| Address | Kind | Name | Confidence |
| --- | --- | --- | --- |
| `0x003e3268` | function | `Render_FinishFrame` | 75 |
| `0x003de3c8` | function | `Render_SetSceneWidthThrottle` | 88 |
| `0x005c1b80` | function | `Rsx_EmitSurfaceBlock` | 88 |
| `0x005c766c` | function | `Rsx_SetReport` | 86 |
| `0x005cc710` | function | `Gcm_AllocReports` | 74 |
| `0x005cb7b8` | function | `Gcm_Flip` | 72 |
| `0x005cb500` | function | `Gcm_FlipOnTime` | 74 |
| `0x00c81b08` | data | `g_SceneWidthLevel` | 92 |
| `0x00c81da4` | data | `g_SceneWidthThrottleEnabled` | 90 |
| `0x00c81a88` | data | `g_SceneColourTargets` | 85 |

- `Render_FinishFrame`: 75. Called once a frame from `0x003bf31c` and `0x006762a4`.
  It binds the resolve (`Post_BindResolveParams`), issues the closing report, swaps
  the report slots, steps the level and takes the next pair. The whole body was not
  read (the 2D and fade branches in the middle), so the name covers what was.
- `Render_SetSceneWidthThrottle`: 88. One `stb` to `g + 0x324`, whose only reader is
  the controller's `lbz` at `0x003e33c0`; clearing the byte live pinned the level.
- `Rsx_EmitSurfaceBlock`: 88. Thirty words: `0x00200200` and eight values, then
  `0x0004022c`, `0x00100280`, `0x000402b8`, `0x00041d88`, `0x00041d7c`, `0x00080194`,
  `0x0004018c`, `0x000801b4`. Its one caller is `Rsx_SetRenderTargets`; `0x005bd6dc`
  is the same body without the space check. The ring holds these packets.
- `Rsx_SetReport`: 86. Writes `0x00041800` and `type << 24 | offset`. A second user,
  the engine flare's occlusion query, is on [engine-trail.md](engine-trail.md).
- `Gcm_AllocReports`: 74. Takes `n` 16-byte slots from a per-frame counter under a
  lightweight mutex and returns the first; the counter's reset was not read.
- `Gcm_Flip`: 72. Queues the flip, stamps the time base before and after, sets the
  on-time byte and writes the control register's put pointer. Its two flip variants
  (`0x005a53e0`, `0x005a51a8`) were not read.
- `Gcm_FlipOnTime`: 74. Three instructions: the byte at `+0x10` of the flip record
  (`0x00f74180`).
- The three data names are fields of `g_RenderGlobals` (`0x00c81a80`,
  [renderer.md](renderer.md)).

Not renamed: `0x005ccc60`, the callback that stores the time base the on-time test
subtracts. Hypothesis: the vblank handler (confidence 45).

## Evidence

**Static.** The instructions, off the image:

```text
003e3320  lwz    r0, 0x4da0(r31)        ; r31 = g_RenderGlobals
003e3324  subfic r0, r0, 1
003e332c  stw    r0, 0x4da0(r31)
003e3338  lwz    r11, 0x4d98(r9)        ; reports[slot]
003e3350  ld     r10, 0(r11)
003e3358  ld     r0, 0x10(r11)
003e3360  subf   r0, r10, r0
003e3368..70                            ; / 1000 by 0x20c49ba5e353f7cf
003e3374  bl     0x5cb500               ; Gcm_FlipOnTime
003e337c  cmplwi cr7, r29, 0x3eef
003e3380  ble    cr7, 0x3e3578
003e3384  lwz    r9, 0x88(r31)
003e3388  cmplwi cr7, r9, 0x1e
003e338c  bgt    cr7, 0x3e3398
003e3390  addi   r0, r9, 1
003e3394  stw    r0, 0x88(r31)
003e33a8  lwz    r11, 8(r9)             ; colour_targets[level]
003e33ac  lwz    r0, 0x20(r11)
003e33b0  cmplwi cr7, r0, 0x437
003e33bc  stw    r0, 0x88(r31)          ; r0 = 0
003e33c0  lbz    r0, 0x324(r31)
003e33cc  stw    r0, 0x88(r31)          ; when the byte is 0
003e3578  clrlwi r0, r3, 0x18           ; on_time
003e3580  beq    cr7, 0x3e3384
003e3584  cmplwi cr7, r29, 0x3cc3
003e3588  bgt    cr7, 0x3e3398
003e3594  beq    cr7, 0x3e3398          ; level == 0
003e3598  addi   r0, r9, -1
003e359c  stw    r0, 0x88(r31)
```

These four `stw .., 0x88(r31)` are the only stores to the level found: a scan of the
60 instructions after each of the 54 loads of `g_RenderGlobals`' TOC slot
(`lwz rD, -0x578c(r2)`) shows loads of `+0x88` everywhere else.

**Live, the variable holds the width.**

1. Polling the level beside the ring's surface packets for 20 s: `1920 - 20 * level`
   was among the ring's 1080-line clips on 2,683 of 2,871 samples where the level did
   not change across the read.
2. Two paused frames read with `scripts/rsx-pass-events.py`, the level steady for
   twelve 8 ms samples before each pause:

   | Level in the pause | `1920 - 20 * level` | Frame's clips |
   | --- | --- | --- |
   | 31 | 1300 | 244 at `1300x1080`, 4 at `1920x1080` (the final stage) |
   | 0 | 1920 | 268 at `1920x1080` |

   While the level moves the buffered frame is one step behind it (level 8 with 234
   draws at 1780, level 7 with 35 at 1760): the step is taken after the frame's
   commands are written.
3. Clearing `g + 0x324` in the race: the level read 0 on 1,047 of 1,047 samples and
   the ring held only 1920; set again, the level roamed 0 to 31 within 6 s.
4. With the byte clear, entry 0 of the six arrays pointed at entry `k`: the ring took
   `1920 - 20 * k` for each of 24 levels tried (0 to 14, then every second, and 31).

**02.51.** The same block is at `0x0006acd4` in the 02.51 executable: the divide by
1000, `cmplwi .., 0x3eef`, `cmplwi .., 0x3cc3`, the `+1` up to `0x1e`, the `-1` down to
0 and the `0x437` height test, with the level at `+0xa8` of that build's globals and
the arrays at `+0x28`. Not run. Confidence 85 that 02.51's law is the same.

**Omega: checked, differs** (confidence 60). Neither `eboot.bin` (base, patch 1.07)
has the two thresholds within `0x2000` bytes of each other as 32-bit immediates
(`0x3ef0`/`0x3eef` with `0x3cc4`/`0x3cc3`), and neither has the option's string or any
`Debug.Enable` string. Not watched live: shadPS4 gives no memory readout. Absence of two
constants is weak evidence (a float compare in milliseconds would not be found this
way), hence 60. 2048 was not checked.

## Reading the width off a frame

`scripts/hd_scene_width.py` reads a 1920-wide frame. A linear stretch from `w` to 1920
makes the horizontal second difference periodic at `(1920 - w) / 1920` cycles a pixel,
bin `20 * level` of a 1920-point spectrum of the column profile (mean absolute second
difference of luma down each column). Each level's bin is scored against the median of
the 16 bins around it; the best level wins if it scores 5.5 or more, otherwise the
frame was not stretched.

Checked on frames whose width was known:

| Frames | Truth | Result |
| --- | --- | --- |
| 24 with the width pinned (evidence 4), each paused and grabbed | 1920 down to 1300 | 24 of 24; the stretched ones scored 6.7 (1840) to 35, the 1920 one 2.9 |
| 2 free-running, level steady | 1300, 1920 | 1300 at 41.2; no peak (3.8) |
| 4 free-running, level moving | the level two frames before the pause | 1880, 1340, 1880, 1340 |

Limits. It was checked on race frames only. A 2D frame with a regular pattern scores
like a stretch: the loading screen's dot grid reads 1840 at 9.1, and that frame is
drawn at 1920. A menu or loading pair therefore records 1920 from how it is drawn and
keeps the score beside it, unused. The level read in a pause is not the presented
frame's when the controller is moving (two steps off in the four frames above), so a
pair's width is the frame's, with the level beside it.

## Reproducing

```sh
scripts/emu-env.sh <lane> 92 23492 ; source data/scratch/<lane>/env.sh
OAG_COMPARE_PLATFORM=hd-fury OAG_RPCS3_GEOMETRY=2200x1300x24 \
  uv run --with evdev python3 -u scripts/rpcs3-drive.py --image <iso> --log-dir $W/serve serve --timeout 500
# idle on Main Menu about 40 s: the attract race starts
python3 scripts/hd_scene_width.py <frame.png>
```

A first boot on an empty cache compiles the executable for about three minutes with
the screen black; the default 240 s boot limit kills it, hence `--timeout 500`. Guest
memory is `/proc/<pid>/mem` at `0x300000000 + address` through the `serve` socket
(`RemoteMem`): the level is four bytes at `0x00c81b08`, the frame's surface packets
are the words `0x00200200` in `0x40070000..0x400b0000`.

Pinning 1920 for a reference frame: clear the byte at `0x00c81da4` after the race has
loaded (it is set again by the next race set-up and on leaving Photo Mode). As a
`patch.yml` entry the same thing is one word, `0x003e33c0` from `lbz r0, 0x324(r31)`
(`0x881f0324`) to `li r0, 0` (`0x38000000`); the byte was tested, the patch was not.

## Open

- A PS3's own behaviour: how often a real console leaves 1920, and on which circuits.
- The five arrays beside the colour targets, by role.
- Whether `Debug.Enable Frame Buffer Throttling` reaches a race set-up's argument.
- `0x005ccc60`'s registration.
