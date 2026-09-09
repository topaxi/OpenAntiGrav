# ADR-0010: Decode movie frames on a worker thread, one per movie

## Status

Accepted.

Answers the open measurement [ADR-0008](0008-av1-movie-cache.md) left behind. Its
"Consequences" section said, of the in-process decoder it had just chosen:

> Playback decodes on the render thread; at 480x272 there is roughly thirty times
> the headroom needed, but the per-frame worst case inside a frame budget has not
> been measured, only the whole-clip throughput.

It has now been measured, and the worst case is what matters. ADR-0008's choice
of codec, container and flags stands unchanged; only where the decode runs
changes.

## Context

`movie::FrameStore::read_frame` decodes one frame synchronously, and both places
that draw a movie called it from the thread that draws:

- `FrontendStage::sync_video` - the intro. **The older of the two, and it had
  always done this.**
- `MenuStage::render` - the looping menu backdrop, added later. It only made the
  cost measurable, because a menu is where a player sits with a frame counter
  open.

ADR-0008's "thirty times the headroom" is a throughput figure and it is correct:
a 480x272 stream decodes at roughly 900 frames a second against playback at 30.
It is also the wrong figure. What a frame loop has is a *budget*, and at 240
frames a second that budget is 4.17 ms.

Measured on the real disc, release build:

| | `read_frame`, one frame |
| --- | --- |
| PSP `Backdrop.PMF`, 480x272, quiet half (frames 0-32) | under 3 ms |
| PSP `Backdrop.PMF`, busy half (frames 33+) | 12 ms to 30 ms |
| Any second call | 0.03 ms |
| `Renderer::upload_frame`, any frame | 0.06 ms |

Every *other* call is nearly free because `av1::FrameSource` is built with
`set_max_frame_delay(1)`, so a picture is usually already queued: the real shape
is a spike on every second frame change. At 30 frame changes a second that came to
roughly **a quarter of every second spent decoding on the thread that draws**.
The upload was never the problem.

What that did to the loop, measured over a 45-second run through both stages on
both discs, with the pacing meter reading the same `elapsed` the timestep does.

**Read the "front end" rows carefully**: every run passed
`--movie 'Data\Movies\Backdrop.PMF'`, which substitutes the nine-second backdrop
for the forty-second intro so that one run reaches both stages. So those rows
measure `FrontendStage::sync_video` - the intro's code path, the one that has
always decoded on the render thread - playing the *backdrop's* stream rather than
`Intro.PMF`'s. The two streams are the same 480x272 cache format and the same
decoder, so the code path is exercised faithfully; the per-frame cost of
`Intro.PMF` specifically is not what is tabulated here.

| Disc, stage, limit | fps (min) | worst frame |
| --- | ---: | ---: |
| PSP front end, 240 | 213.5 | 43.35 ms |
| PSP menu, 240 | 197.7 | 51.85 ms |
| PSP menu, unlimited | 438.9 | 46.51 ms |
| PS2 menu (`BG512.IPF`, 512x512), 240 | 169.6 | 82.25 ms |

The unlimited row is the one worth dwelling on, because it is why this went
unnoticed for so long: the *median* was 942 fps and looked perfectly healthy,
while individual frames were 46 ms long. An fps counter cannot show that and a
pacing graph can. The frame limiter did not make anything slower either - it
removed the headroom that had been hiding the stalls.

Two dead ends, both tried rather than reasoned about:

- **`settings.set_n_threads(4)` on the decoder changes nothing.**
  `max_frame_delay(1)` leaves no frame-level parallelism to find and these streams
  are single-tile. The dial is not there.
- **Decoding the whole movie into RAM at load** costs 52 MB and a ~2.7 s stall for
  the backdrop alone, at the exact moment the menus open on a keypress out of a
  race. A 2.7 s freeze reads as a hang, and the intro would be far worse.

## Decision

**One decode worker thread per movie, decoding forward into a small ring, with the
drawing thread asking for the newest frame at or before the playhead.**

`movie::Feed` owns a `FrameStore` outright and moves it onto a
`std::thread`. The drawing thread has no way to reach the decoder.

- **The two sides talk only through a monotone position.** `Player::position()`
  counts frames since playback began and never wraps, where `Player::frame()`
  does. The worker maps position to frame index with the same arithmetic
  (`position % len` for a movie that loops), so the two agree at every wrap by
  construction rather than by bookkeeping.
- **Four frames of lookahead**, which at 30 Hz is 133 ms of slack against a 30 ms
  worst case, and 784 KB at 480x272 or 1.5 MB at the PS2's 512x512. It is a bound
  as much as a target: the worker fills the ring and parks, so a decoder that runs
  at 900 frames a second cannot race thirty seconds ahead of a movie played at 30.
- **A frame that has not arrived is not an error.** The drawing thread keeps the
  picture it last uploaded. Before *any* frame has arrived, the video draw is left
  out of the list entirely, because zeroed I420 planes are not black - `Y=0, U=0,
  V=0` is green - and a green rectangle over the menu is worse than one frame of
  the black the menus already fall back to.
- **A playhead that jumped forward does not replay what it skipped.** The ring
  hands back the *newest* frame at or before the position asked for and drops the
  rest, so playback never runs backwards and never drifts further behind than it
  must.
- **The wrap is cheap and is not worked around.** `av1::FrameSource::frame(0)`
  after frame 269 does flush the decoder, but frame 0 is the very next frame
  wanted, so the cost is one flush plus one frame - not a walk through the movie.
  That is the difference between wrapping a loop and seeking backwards into the
  middle of one, and it happens on the worker either way.
- **The backdrop's feed lives on the `Session`, not on the menu stage**, so
  menu -> race -> escape -> menu does not rebuild a decoder and re-read the cache
  file. Reopening calls `Feed::restart()`, which is one flush.
- **`restart()` carries an epoch.** A restart can land while the worker is 30 ms
  into a decode, and that frame belongs to playback that no longer exists. The
  worker re-reads the epoch after decoding and discards the frame if it moved.
  Without this the *second* menu open is wrong and the first is fine, which is the
  worst shape a bug can have.
- **The headless capture path keeps the synchronous `FrameStore`.**
  `--screenshot` wants one exact frame and has no loop to stall; a worker there
  would be machinery for nothing.

### Why this does not touch the determinism rules

[`determinism.md`](../determinism.md) requires the simulation to be
single-threaded, and this is entirely presentation-side. The feed is **write-only
toward the GPU**: a `Player` hands it a position and it hands back pixels.
Nothing it produces is read back into sequencing.

The case worth naming, because it is the one that could have gone wrong: the
intro's state machine compares `Player::frames_produced()` against 144, 231 and
260, and those decide when the sequence pauses and when it fires its exit. That
number comes from the player's fixed-timestep `update` and is identical whether a
picture ever arrives or not - it is already the case that `--no-video` and a
missing `ffmpeg` play the whole sequence with a black screen. So no simulation
state, and no state the simulation reads, can depend on when a decode finished.

## Alternatives considered

**One shared decode thread serving both movies.** At most two feeds ever exist,
and never both being drawn at once - the intro's stage and the menus' are
different stages. A shared thread would need a scheduler between them and a
teardown protocol that knows about both, to save one parked thread. Rejected as
strictly more machinery for less isolation.

**A bounded channel instead of a ring under a mutex.** Simpler, and the obvious
first answer. Rejected because a channel cannot be *inspected*: the consumer polls
several times per movie frame, so it would either drain everything available -
running the picture a fixed few frames ahead of the playhead the draw list names -
or block. Peeking at positions without consuming them is the whole reason the ring
is a ring.

**Pre-decoding the whole movie into RAM at load.** 52 MB and ~2.7 s for the
backdrop, worse for the intro, paid at menu-open time. Rejected: it converts a
recurring 30 ms stall into a one-off 2.7 s one, which is not obviously better and
is much more visible.

**More decoder threads.** Tried; changes nothing. See Context.

**Recycling frame buffers through a free list** rather than allocating a `Vec` per
frame. Rejected as unmeasurable: 196 KB thirty times a second is 6 MB/s of
allocator traffic on a background thread, against roughly 60 microseconds a second
of actual `malloc`/`free`.

## Consequences

**Good.** Measured on the same runs as the Context table, same instrumentation,
same settings:

| Disc, stage, limit | fps (min) | worst frame |
| --- | ---: | ---: |
| PSP front end, 240 | 240.0 (was 213.5) | 4.44 ms (was 43.35) |
| PSP menu, 240 | 240.0 (was 197.7) | 4.71 ms (was 51.85) |
| PSP menu, unlimited | 740.7 (was 438.9) | 7.45 ms (was 46.51) |
| PS2 menu, 240 | 238.9 (was 169.6) | 7.04 ms (was 82.25) |

The same `--movie` substitution as the Context table, so "front end" means
`sync_video` playing the backdrop's stream. The one number nobody has measured
either side of this change is a 240-limited loop over `Intro.PMF`'s own 1200
frames; the cache for it is 33 MB and warm, so it is one 50-second run per
revision if anyone wants it.

A 240 limit is now held exactly rather than approached, and the worst frame is the
limiter's own jitter rather than a decode. The PS2's 512x512 cut, which was the
worst case at 82 ms, is no longer distinguishable from the PSP's.

**Bad.** There are now up to two threads in a process whose simulation is
deliberately single-threaded, and the reason they are safe is an argument in prose
rather than something the compiler checks. Anyone adding a reader of `Feed` has to
re-make that argument. The obvious way to get it wrong is to make some piece of
sequencing wait for a picture; nothing does today.

**A frame can be one frame late**, where before it was always exactly the frame
the playhead named. At 30 Hz against a loop running at 240 or more this is
invisible, and it is why `menu::Backdrop.frame` now reports the frame in the
planes rather than the one the player wants - the draw list stays honest about
what is on screen. `oag-render` ignores that field, so nothing else notices.

**The first frame or two after a stage opens have no picture.** Deliberate, and
the only alternative was blocking on frame 0 - which is a stall, if a small one.
The intro fades up from black anyway.

**A silent failure mode was created and closed.** `VideoFormat::of` reads
`Movie::frames`, which is `None` once a feed has taken it, and the honest reading
of `None` is "no picture" - so a renderer would be built with no video pipeline
and the menus would draw on black, with no error anywhere. Worse,
`--menu-page` would still look right, because it goes through the headless capture
path with a different `Movie` that kept its frames. `VideoFormat::of_feed` exists
for this and the live path is checked by a screenshot of a real window rather than
by `--menu-page`.

**Memory is up by the lookahead**: 784 KB per PSP movie, 1.5 MB for the PS2's
backdrop. Both are noise, and both are bounded rather than growing.

**Ring memory is per feed, not per process.** Two feeds alive at boot means both
rings, so about 2.3 MB on the PS2 while the intro is playing.

## What else on the presentation side would benefit from this

Recorded as candidates, in rough order of how much they would repay. **None of
this is implemented**, and none of it should be bundled with a movie change:

- **Texture decode** (`oag_texture::texture`, `ps2_texture`, `.mip` palette
  expansion). Happens during a load rather than in a frame loop today, so it costs
  stall time and not frame time - which makes it lower priority than it looks, but
  it is embarrassingly parallel across textures and would shorten every load.
- **WAD reads and CHD decompression** (`oag-assets`, `oag-disc`). The dominant
  cost of opening a race, and I/O-bound rather than CPU-bound, so it is the
  natural next one: a race load currently freezes the window for a few hundred
  milliseconds and the loop already marks that frame as stalled rather than
  measuring it.
- **The whole race load** (`race::Loaded`), off-thread with a progress screen
  drawn by the loop that is still running. This is the change a player would
  actually notice, and it is the largest: it needs every step of the load to be
  interruptible and none of them to touch the GPU.
- **Buffer and texture uploads** on a wgpu transfer queue. Different in kind from
  the above - the work is already on the GPU's side of the fence - and only worth
  it once there is enough per-frame upload to matter. There is not yet.
- **The transcode itself** (`movie::run_ffmpeg`). A 13-second freeze at boot for
  the backdrop and 81 seconds for the whole intro, once per install. Genuinely
  annoying, and awkward rather than hard: it wants a "still converting" state in
  the front end, which is a design question about what the game shows meanwhile,
  not a threading one.

Deliberately *not* on this list: anything in `oag-physics`, `oag-gameplay` or
`race::Race::tick`. Those are the simulation, and
[`determinism.md`](../determinism.md) governs them.
