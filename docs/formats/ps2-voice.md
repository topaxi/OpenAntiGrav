# PS2 pre-race voice archive (`PRERACE.WAD`)

`54748/PRERACE.WAD`, 89,325,308 bytes, the third-largest file on the PS2 disc
after `PS2MUSIC.WAD` and `WADS2.WAD`. It holds 32 clips of uncompressed PCM
speech - the pre-race dialogue.

Despite the extension it is **not** the WAD container, for exactly the reason
[`PS2MUSIC.WAD`](ps2-audio.md) is not: the first word is an entry count, not a
version. `oag-wad` reads it as one and fails - *"unknown WAD version 32"* here,
*"unknown WAD version 16"* for `PS2MUSIC.WAD`. The file is fine; the reader is
looking at the wrong field.

That error now names the alternative and points at `crate::ps2_music` instead of
leaving the reader to hunt for a WAD variant that does not exist. **It is still
an error**: `oag-wad` cannot list or extract a count-first archive, and teaching
`Archive` to open one is a separate piece of work. Use the format module
directly, as the ground-truth test does.

## Layout

Byte-for-byte the [PS2 music](ps2-audio.md) container, and
[`crate::ps2_music`](../../crates/formats/src/ps2_music.rs) parses it with **no
changes at all**:

```text
+0x00  u32  entry_count            32
+0x04  entry[entry_count], 12 bytes each

entry:
  +0x00  u32  name_hash
  +0x04  u32  size
  +0x08  u32  offset               from the start of the file
```

Two arithmetic checks pin it, both asserted by the ground-truth test rather than
eyeballed:

- Entry 0 begins at **388** = `4 + 32 * 12`, so the directory is exactly full -
  no slack, no reserved fields.
- The offset chain **closes to the byte** on 89,325,308, the file's exact
  length. All 32 entries, no gaps and no padding.

Every entry size is divisible by 4, the same 16-bit-stereo frame the music
archive uses.

## The payload is dual-mono

This is what separates it from the music archive. The two channels are
**byte-identical** - a mono recording written into a stereo stream:

```text
02aea540  14 e5 14 e5  95 d7 95 d7  fd e1 fd e1  76 f7 76 f7
02aea550  57 0c 57 0c  4f 17 4f 17  04 16 04 16  50 10 50 10
```

Measured over a 2 MiB probe a third of the way into each clip, the fraction of
frames whose left and right samples are equal is **1.0000 for all 32 clips** -
not "highly correlated", identical. That is why the music test's
`correlation > 0.3` check would be meaningless here: it is 1.0 by construction,
and asserting dual-mono instead is what keeps the two archives from being
conflated.

It also explains the disc sniff's low entropy reading of 0.084: the first 64 KiB
of the file is near-silence, the lead-in to a voice-over.

## Sizes

Clip sizes run 2,271,168 to 3,376,824 bytes (567,792 to 844,206 frames),
clustered tightly - consistent with 32 takes of one line each.

## Confidence

**92** for the container: the directory closes to the byte on the file length,
which is the same evidence that carries `PS2MUSIC.WAD` at 94, one step down
because the frame size here is inherited from that archive rather than
independently established (the divisible-by-8 census that pins a 4-byte frame
for the music has not been repeated here).

**90** for dual-mono 16-bit PCM: 32 of 32 clips at exactly 1.0000, which is a
measurement rather than an inference, but the sample rate is not yet settled -
see below.

Ground truth:
[`crates/formats/tests/audio_ground_truth.rs`](../../crates/formats/tests/audio_ground_truth.rs),
`the_ps2_prerace_archive_chains_exactly_and_holds_dual_mono`.

## Not determined

- **The sample rate.** Nothing in the container states it, and unlike the music
  archive it has not been cross-validated. The lead: the PSP disc carries
  **32 mono ATRAC3plus clips** of 13.47-19.41 s (`Data.wad` entries 867-898),
  and 32 against 32 is suggestive. Bracketing this archive's byte counts, 44.1
  kHz gives 12.88-19.14 s and 48 kHz gives 11.83-17.59 s, so 44.1 kHz is the
  better fit - but the duration match is a mean gap of ~0.305 s, nowhere near
  the 0.011 s bijection [`ps2-audio.md`](ps2-audio.md) achieved for the music.
  **That is a lead, not a finding**; it needs the same side-signal
  cross-correlation the music got before either the rate or the 32-to-32 pairing
  can be claimed.
- **The clip names.** Hash-keyed like every other archive here, and unmined. The
  executable's `Data\Sound\PreRaceDialogue\%d_Track.at3` is the PSP-side
  equivalent path and may be the way in.
- **Whether the game streams these directly**, and through what - the PS2 disc
  ships `IOP/SCREAM.IRX`, the same open question `ps2-audio.md` records.
