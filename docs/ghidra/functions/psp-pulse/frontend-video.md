# Video playback and the Movie widget

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`.

Covers the boot intro sequence, which is the first vertical slice worth
building. **The names here are applied**, from [names.tsv](names.tsv).

## Pulse drives `sceMpeg` directly

There is **no `scePsmf` or `scePsmfPlayer`** in this binary; no such stub
section exists. Pulse uses the lower-level `sceMpeg` API and does its own
demuxing, ring-buffer feeding and audio output.

That matters for reimplementation: there is no black-box "play this PMF" call to
mirror. The playback loop is the game's own, and it is fully visible.

The `.lib.stub` table is at `0x08a774d0` (38 entries of 20 bytes). No symbols
exist, so every thunk was resolved from NIDs and cross-checked against call-site
arity. Those NIDs are now checked against `SHA-1(name)` directly, which
[import stubs](imports.md) does for the whole table.

`sceMpeg` thunk base `0x08a76eec`, with the two easily-transposed entries:

| Address | Function | NID |
| --- | --- | --- |
| `0x08a76f0c` | `sceMpegRingbufferConstruct` | `0x37295ED8` |
| `0x08a76f84` | `sceMpegCreate` | `0xD8C5F121` |

Confidence **96**, cross-validated at eight call sites. The investigation
initially had these two swapped and corrected itself on arity, which is worth
recording: `RingbufferConstruct` takes six arguments ending in
`(callback, param)`, `Create` takes seven with the ringbuffer as argument three.

AV modules are loaded at `0x08956e4c` via `sceUtilityLoadModule`:
`AVCODEC` (0x300), `SASCORE` (0x301), `ATRAC3PLUS` (0x302), `MPEGBASE` (0x303),
plus `USB_MIC` and `NP_DRM`. Note `sceUtilityLoadAvModule` is **not** imported.

## The player

A raw `sceMpeg` wrapper, **embedded at `+0x94` inside the Movie widget** rather
than allocated separately.

| Address | Name | Conf |
| --- | --- | ---: |
| `0x08914858` | `MoviePlayer_Create` | 90 |
| `0x089138bc` | `MoviePlayer_Open` | 88 |
| `0x08913fa0` | `MoviePlayer_ParseHeader` | 88 |
| `0x08913a98` | `MoviePlayer_Update` | 88 |
| `0x0891448c` | `MoviePlayer_DecodeVideoStep` | 88 |
| `0x08914364` | `MoviePlayer_DecodeAudioStep` | 85 |
| `0x089145bc` | `MoviePlayer_Shutdown` | 88 |
| `0x08913d34` | `MoviePlayer_IsFinished` | 85 |
| `0x08913e88` | `MoviePlayer_GetCurrentTexture` | 82 |
| `0x08914c78` | `MoviePlayer_ReadThreadMain` | 85 |
| `0x08914fd8` | `MoviePlayer_SoundThreadMain` | 82 |

`Create` runs `sceMpegInit`, sizes and allocates the ring buffer and MPEG
context, constructs the ring buffer with callback `0x08913730`, creates the MPEG
instance at frame width `0x200`, registers stream 0 (AVC) and optionally stream
1 (ATRAC), then spawns two threads: a **reader** at priority `0x41` with a 32 KiB
stack and a **sound** thread at priority `0x3c` with 16 KiB.

`ParseHeader` reads `0x800` bytes and uses `sceMpegQueryStreamOffset` and
`sceMpegQueryStreamSize` to find the payload.

Audio for movies goes through **`sceMpegAtracDecode`**, not the `sceAtrac*` API.
Only one `sceAtracSetDataAndGetID` cross-reference exists and it belongs to the
separate music path. `sceMp3` is likewise the streamed-music path, not this one.

Player layout, confidence **85**: `+0x03` repeat, `+0x0c` file, `+0x10` has
sound, `+0x28` stream offset, `+0x2c` ring buffer, `+0xac` `SceMpeg`, `+0xb0`
and `+0xb4` stream ids, `+0xbc` and `+0xfc` video and audio access units,
`+0x1c4` and `+0x1cc` produced and consumed counters, `+0x1c8` frame buffers,
`+0x1d0` paused.

## The Movie widget is XML-driven

Registered as `"Movie"` by `0x088ba8c8` with vtable `0x08acdacc`. The attribute
parser is `Movie_ParseAttributes` at `0x088ba284`, confidence **95**:

| XML attribute | Field | Meaning |
| --- | --- | --- |
| `src` | `+0x284` | Base name, **no extension** |
| `sound` | `+0x304` | |
| `width` | `+0x27c` | |
| `height` | `+0x280` | |
| `autoredirect` | `+0x306` | Transition when finished |
| `autostart` | `+0x305` | Play on entering the screen |
| `repeat` | `+0x307` | Loop |
| `localised` | - | Selects the filename suffix |
| `stoppowersave` | `+0x308` | |

Booleans are true when the value is `"true"` **or** `"1"`.

The filename is assembled at runtime:

```c
copy(src, buf);
if (localised is absent or "true") append(".PMF");
else                               append("_US.PMF");
```

So **the intro movie's filename is not in the executable**. It lives in the
front-end XML, which is why it never turned up in a string search.

Rendering (`0x088ba660`) tiles the decoded 512-pitch frame into 32x64 sprite
quads and submits them through the GU.

### Correction: `profile_movie.pmf` is not an intro video

`Data\FE\Profile\profile_movie.pmf` and its siblings are registered by
`0x08947440` as PSP **save-data icons**: `profile_FULL.PNG` becomes `PIC1.PNG`,
`bkgrd.PNG` becomes `ICON0.PNG`, `SND0.AT3` becomes `SND0.AT3`, and
`profile_movie.pmf` becomes `ICON1.PMF`. Confidence **92**.

Worth stating because the name invites the opposite conclusion.

## The intro sequence, and skipping it

The player never reads the pad. The skip lives in the intro **state's** update,
`0x088d7e1c`:

```c
if (Input_IsPressed(g_input, 0xe, 0)) {   // 0xe = START, a bit index
    StateMachine_Fire(state->devPubRedirect);
    state->devPubRedirect = 0;
    Input_ConsumePress(g_input, 0xe);
}
```

`0xe` is the abstract **bit index** for START, not a mask. See
[input](input.md).

The state's `OnEnter` (`0x088d7d80`) caches two transition targets by name:
`"DevPubRedirect"` and `"Intro Screen->IntroMovie1"`.

Playback is then paced against the movie's own frame counter:

| Frame | Action |
| ---: | --- |
| 144 (`0x90`) | Pause, stamp the time |
| 231 (`0xe7`) | Pause, stamp the time |
| 260 (`0x104`) | Set the finish flag, stamp the time |

A pause is released once **2 seconds** have elapsed, and the finish flag fires
`DevPubRedirect`. Reading this as the developer and publisher logo cards being
held for two seconds each is an inference; the numbers are certain, the
interpretation is **82**.

Skipping does **not** stop the player directly. Firing `DevPubRedirect`
transitions the state, and the widget's exit path
(`0x088ba188` → `0x088ba210` → `MoviePlayer_Shutdown`) tears the player down as
the state unwinds. Reproducing that ordering matters: the teardown is a
consequence of the transition, not a peer of it.

`"Intro Screen->ProfileMovie1"` and `ProfileMovie2` are handled by a different
state (`0x088e3938`), which starts playback immediately on entry.

## Not determined

- **The actual intro movie filename**, which is built from XML at runtime.
  Extracting and reading the front-end XML would settle it.
- Whether the profile movies can be skipped; only the `IntroMovie1` state's
  update was decompiled.
- Where `libmp3.prx` is loaded. The string at `0x08a8b200` is referenced only
  from a data table at `0x08ac2318`, which was not decoded.
- ~~The `sceKernelLoadModule` thunk~~: `0x08a7738c`, one of the six
  `ModuleMgrForUser` stubs resolved by NID in [import stubs](imports.md).
- The exact GU texture state in the render path.
- Whether the parser requires a `Values` child element or accepts attributes
  directly on `<Movie>`.
