# ADR-0027: Three mix buses, and a master, on the original's own lines

## Status

Accepted. Supersedes [ADR-0018](0018-audio-mixer-architecture.md)'s **"two
buses" only** - the phrase in its Decision that reads "a 32-voice pool, two
buses, and the sample loop", and the Context paragraph "There are two volumes,
not one" that motivated it. Every other item of ADR-0018 stands: the crate's
place on the non-simulation side, `cpal` under our own mixer, the state/device
split, the null backend, the clockless mixer, cues as a per-tick output, and
ADSR/reverb deferred.

## Context

ADR-0018 read the original's options menu - `"Music Volume"` at `0x08a78658`
and `"SFX Volume"` at `0x08a78668`, and nothing else - and gave `oag_audio::Bus`
exactly two variants to match. That inferred the *mixer's* structure from the
*menu's* row count, and on 2026-08-24 the mixer itself was read.

[`audio-levels.md`](../../ghidra/functions/psp-pulse-usa/audio-levels.md):
`Audio_SetGroupVolume` (`0x089956d8`) carries **sixteen group volumes plus a
master**, all clamped to `0`..`0x400`. `Audio_Init` (`0x089906e4`) opens every
one of them at `0x400` and `Audio_OutputThread` (`0x0898ca54`) hands
`master << 5` = `PSP_AUDIO_VOLUME_MAX` to the DAC.
`Audio_UpdateGroupVolumes` (`0x0893ac70`) rewrites nine of the sixteen every
frame from one fade scalar, and multiplies **group 0 alone** by a second factor
that ramps toward `1.0` and retreats when a flag is set - the shape of a duck,
almost certainly music under speech.

So the original does separate classes of sound below its two menu rows. Two
volumes is what it *shows a player*, not what it mixes.

Two other things were established at the same time and bear on the decision.

**The mix has no headroom.** Nothing in the original's chain attenuates by
default, and a measured 30-second race of this port clips 1.63 % of its output
samples - the effects bus doing so *on its own*, before music is added, while
eight `~ENGINE` voices sit inside the 50-unit engine radius together. The
console's answer is to saturate, which `Mixer::render` now reproduces. A player
who wants the race not to distort has no single control to reach for.

**Voice lines are already separated by bank.** `oag_sound::sfx` records
that `shieldactive` "lives in `speech.bnk` rather than `weapons.bnk`, which is
what says it is a voice line and not an effect", and that `Autopilot_Update`
plays `disengaging` through the dry, full-volume path rather than the craft's
emitter. The line exists in the data; the port had no way to act on it.

## Decision

**`Bus` gets a third variant, `Speech`, and `Bus::COUNT` becomes 3.**

**A cue's bus is derived from its bank, not from a second list.**
`Cue::bus()` is `match self.bank()`, so `BankName::Speech` is the whole
definition of a voice line and a cue moves bus by moving bank. Nothing can
disagree with `bank()` because nothing else is written down.

**The mixer's existing `master_gain` gets a settings row.** It was implemented
and unused; it is the port of the recovered master stage - group `0x10`,
`master << 5` - with a control attached, not a gain invented for the port.

**Four rows on the AUDIO page**: MUSIC, SFX, ANNOUNCER, MASTER. Two are the
original's own menu; two are ours, on lines the original's *mixer* draws.

**What is deliberately not claimed:** which of the sixteen recovered group
indices each bus corresponds to. That mapping is unread, so this is a port-side
split along a line the original draws - not a transcription of its table. Under
no circumstance may a bus be given one of those indices on a guess.

## Alternatives considered

**Leave it at two buses and let the player keep MUSIC and SFX in step.**
Rejected: the mix clips on the effects bus alone, so the control that fixes it
is one that moves everything, and asking a player to move two rows by equal
amounts is asking them to do arithmetic to stop distortion.

**A limiter or compressor instead of a master row.** Rejected outright, and it
is the reason this ADR exists rather than a one-line commit: the original
saturates, a limiter is a plausible-looking stand-in for behaviour nobody
measured, and `CLAUDE.md` forbids exactly that. Saturation is ported; the
loudness is the player's to set.

**Sixteen buses, one per recovered group.** Rejected: the group *indices* are
recovered but what each one holds is not, so fifteen of them would be empty
names and the sixteenth would be a guess about which. The three that exist are
the three whose membership the data answers.

**A per-cue gain multiplied into `Play::gain` instead of a bus.** Rejected: a
bus gain applies at mix time, so a live voice follows a settings change on the
row the player is standing on, and a per-play gain would not. That is the same
property `apply_setting` already relies on for the two existing rows.

## Consequences

**The ANNOUNCER row moves almost nothing today.** Exactly two cues reach
`Bus::Speech` - the shield callout and the Autopilot's one-second warning - and
a race in which the player collects neither pickup has no voice line in it at
all. The announcer lines a player would expect to control (a weapon callout,
"contender eliminated") are **not implemented**: no such cue exists in
`Cue::ALL`, because no trigger for one has been recovered. The row is honest
about what exists rather than sized for what is hoped for, and it will get
louder as those triggers land.

**ADR-0018's Context paragraph is now half-wrong on its own terms** and stays in
the tree saying so, because ADRs are immutable. Anyone reading "There are two
volumes, not one" needs this file beside it; the Status section above is the
only pointer there is.

**`Bus::COUNT` is a size and it changed**, so `bus_gain` is a `[f32; 3]` and any
future code indexing it by a hardcoded `0`/`1` is wrong in a way the compiler
will not catch. `Bus::index` is the only place that mapping is written.

**Two more settings keys to carry**, and a settings file written by an older
build gains both at their defaults - which are `100`, so an existing player's
mix is unchanged by this ADR until they move a row.

**The split may turn out not to match the original's grouping.** If the sixteen
group indices are read and speech shares a group with effects, this port will
have a control the original's mixer cannot express. That is a knob a PC build is
entitled to, but it should then be *labelled* as ours rather than quietly
defended as fidelity - and this paragraph is what says so.
