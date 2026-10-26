# 2048's Autopilot update, read in place (2026-10-26, `autopilot-expiry`)

Where Wipeout 2048 counts the Autopilot's timer down, warns at one second, clears the
running bit, and hands the controls back, read statically in Ghidra
(`/2048/eboot-vita-2048-eu-v104.elf`, ARM Thumb-2). Pulse's reading is
[psp-pulse-usa/autopilot.md](../psp-pulse-usa/autopilot.md), HD's
[ps3-hdfury-eu/autopilot.md](../ps3-hdfury-eu/autopilot.md#expiry-mid-corner-2026-10-26-autopilot-expiry).
**Not run live**: no Vita3K session was up and none was started (see "Not done").

The maintainer's question was whether the timer is held while the craft banks, pulls g or
crosses a turn zone, and whether the hand-back is eased. Answer for 2048, **no to both,
static, confidence 80**: the update has no condition on the craft or the track, and the one
reader of the controls' blend only tests it against zero.

**Nothing here is named.** Ghidra has not delimited the function that holds the update: it
is the middle of one long per-craft weapons-and-pickups routine (the switch continues past
`0x8120ee26`), so there is no function to rename and no `names.tsv` row. The addresses
below are instruction addresses in that undelimited body.

## The update (v1.04, `0x8120e82c..0x8120e8cc`)

`r5` is the pickup record, `s16` the frame's `dt`, and the record's flags word is `[r5+0x1f4]`
(`r12` and `lr` carry it between tests). The running bit is `0x800`, as on Pulse's `+0x1b8`.

    0x8120e854  ands r1, r0, #0x800          ; running?
    0x8120e858  beq  0x8120e8cc              ; no: next pickup test
    0x8120e85a  vldr s0, [r5,#0x148]         ; timer
    0x8120e862  vsub s0, s0, s16             ; timer -= dt
    0x8120e86a  vstr s0, [r5,#0x148]
    0x8120e872  bge  0x8120e888              ; timer >= 1.0: skip the warning
    0x8120e878  vldr s1, [r5,#0x144]         ; previous timer
    0x8120e884  bgt.w 0x8120ed34             ; previous > 1.0: the warning block
    0x8120e888  vldr s0, [r5,#0x148]
    0x8120e89a  bgt  0x8120e8c0              ; timer > 0: keep running
    0x8120e8a8  bfi  r0, r1, #0xb, #0x1      ; timer <= 0: clear bit 0x800 of [r5+0x1f4]
    0x8120e8b4  str.w r8, [r5,#0x1b8]        ; and store 0 at +0x1b8 (what it is: unread)
    0x8120e8bc  bne.w 0x8120ee12             ; held voice at [[r5+0xe8]+0x5588] -> stop it, zero it
    0x8120e8c4  vstr s0, [r5,#0x144]         ; previous = timer

The shape is Pulse's `Autopilot_Update` and HD's, with the same two fields (`+0x148` timer,
`+0x144` previous) as Pulse. **There is nothing between the running test and the decrement**:
no read of the craft, the body, the track section or a flag that something else sets, so
the timer falls by `dt` every dispatch and the bit clears the first time it reads `<= 0`.
The warning block (`0x8120ed34..0x8120ed5e`) passes the cue string at `0x814a6408`
(`"disengaging"`, read from memory) to `0x811c483c` after a guard call
(`0x81000930(r10, -1)`, the owner/seat test HD's `0x000cfeb8` also makes, not read to a
meaning). The edge is the same `now < 1.0 && previous > 1.0`: one second before the bit
clears, not tied to the shape of the track.

## The fire (inlined, `0x8120e9c8..0x8120e9ee`)

    0x8120e9da  orrs r0, lr, #0x800          ; running bit set
    0x8120e9e6  vstr s0, [r5,#0x148]         ; timer = [table + 0x10c] (the weapon file's `time`)
    0x8120e9ee  mvns r0, #0                  ; held = -1, the pickup is spent

The authored `time` sits at table offset `+0x10c` here (Pulse `+0x94`, HD `+0xb4`).

## Who else writes the timer

The `[rX,#0x148]` stores in the image are many (the offset is common). The ones in this
routine are the two above. `0x81205102` and `0x81205116` (`FUN_812050e6`) are a different
object: it accumulates `dt` into `[r4+0x148]` and resets it to `0` past a limit read from
`0x81520134`, with a flag at `[r4+0xc4]`, no relation to a pickup record. A census of every
`+0x148` store was not completed, so "only fire, update and an absorb cancel" is **not**
claimed for 2048 the way it is for Pulse (90); the cancel itself is unread.

## The hand-back

The controls' blend is `craft+0x278`, as on HD (setter `0x811d1326`, uncapped setter
`0x811d132c`, see [ps3-hdfury-eu/autopilot.md](../ps3-hdfury-eu/autopilot.md#the-other-titles-read-off-each-executable-no-ghidra-program-opened)).
Its readers in the craft update:

    0x811cfab4  vldr s0, [r5,#0x278] ; vcmpe s0, #0 ; bne.w 0x811d03d6   ; the driver's-record copy
    0x811cfea4  vldr s0, [r5,#0x278] ; vcmpe s0, #0 ; bne  0x811cfec0

Both compare against zero and branch; neither multiplies by it, so the driver's record is
used at full strength until the blend reads `0` and the pad's after, a one-tick hand-back.
The same two-site shape HD has (`0x000f19fc`, `0x000ecab4`). The other `+0x278` loads in
the image (`0x8101268e`, `0x8107e046`, ...) are other structures; not read one by one.

## Not done

- A live check. Vita3K was not running (`pgrep -a Vita3K` empty) and its capture recipe
  ([vita3k-capture.md](../../../reverse-engineering/vita3k-capture.md)) needs a race
  state that is not in `data/saves/`; building one is a lane of its own.
- 2048's absorb cancel, its pad grant under a running stint, and Omega's own update and
  cancel stay unread; both run HD's law, `Origin::InheritedFrom`.
- Omega: out of scope for this pass (maintainer's ruling), neither read nor run.
