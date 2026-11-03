# A mod's sounds (`[sounds]`)

**Status: built (2026-11-03).** Our own, **chosen, not measured**: nothing here is read off
an original. A mod ships its own sounds and a module plays them with `play-mod-sound`. A mod
never ships a title's audio; the sounds are the mod author's own.

```toml
[sounds]
portal-open = "sounds/portal-open.wav"
transit = "sounds/transit.wav"
```

Schema: `schemas/v1/mod.tosd` (`[elements.sounds]`). The loader checks each name (letters,
digits, `-`, `_`) and that each path is a `.wav` in the mod, at most 64 sounds
(`oag_mod::manifest`); the file itself is read and decoded when a race loads.

## The file

RIFF/WAVE, **16-bit integer PCM, mono or stereo**, any sample rate (`oag_audio::wav::decode`,
beside the writer). The mixer steps through a sound at `file rate / mixer rate`, as it does
for the banks' sounds, so there is no resampler of ours and nothing to convert. At most 4 MiB
of PCM. Mono is the one to use for a sound that is placed (a stereo file is played through
the same pan as a mono one). Anything else (float, 24-bit, ADPCM, more than two channels, a
truncated chunk) is refused by name. OGG/Vorbis is not read: the engine decodes no Vorbis.

## Playing it

SDK `reply.play_mod_sound(&craft, index, mode)`, wire `play-mod-sound` (command 33): `index`
is the sound's place in `[sounds]` in **name order** (the same rule as `[effects]`), `mode`:

| Mode | Does |
| --- | --- |
| `Once` | Plays once from the craft. Each call plays again. |
| `Loop` | Starts a loop that follows the craft, re-placed every tick. A loop already running for that mod, sound and slot is left as it is (a module may re-send it every tick). |
| `Stop` | Ends that loop. None running is not a fault. |

A loop also ends when its module faults (the host queues the stops, as it puts back
everything else the module held), when its craft's slot is empty, when the race finishes, and
on leaving the race. A mode other than the three is a malformed record.

## Where it is heard

The same path as a title's cue on a craft: `Placement::CraftUnlessPlayer` (the craft's
emitter, **the player's own craft heard dry, the others positioned**), the effects bus
(`Bus::Sfx`), the master trim, the limiter, and the HRTF or rear cue the player chose. A
one-shot from a craft out of range is not started, as a title cue is not; a loop out of range
runs silent and is heard when the craft comes in. The player's own craft is a hypothesis the
placement type documents (`docs/ghidra/functions/psp-pulse-usa/pads.md`, confidence 45); a mod
that wants a rival's sound in the player's ears sends it from the rival's slot.

A craft out of the world in a `transit` keeps its place for this: the frame carries
`mod_craft`, the craft's poses with a craft in transit held where it left, so a portal
whoosh and a transit hum come from the craft that vanished (`craft`, the engines' and the
title cues' poses, drops it). Found by
`oag-game` `sfx_ground_truth::slipstream_is_heard_when_a_craft_goes_through_the_portal`, which
runs the real mod in a real Pulse race through `Audio` and counts the loop up and down.

An output: queued on `RaceSim::mod_sounds` beside the cues, drained by the composition root
(`Race::drain_mod_sounds`), never hashed
(`tests::extension_sounds::a_race_that_asks_for_sounds_hashes_like_one_that_asks_for_none`).
A sound that fails to load plays nothing, and the load report names it
(`mod <id>: sound <name> left out: <why>; plays nothing`).

## Tests

- Format: `oag_audio::wav::tests::a_pcm16_wav_decodes_at_its_own_rate_and_a_bad_one_is_named`.
- Manifest: `oag_mod` `a_manifest_carries_sounds_that_are_wavs_in_the_mod`.
- Loader: `oag_raceplay` `tests::extension_sounds::loading::a_mods_sound_files_are_loaded_and_a_bad_one_plays_nothing_and_is_named`.
- Queue and fault: `tests::extension_sounds::a_loop_is_queued_once_a_one_shot_every_time_and_a_stop_ends_it`,
  `a_faulted_module_stops_the_loops_it_started`.
- Mixer: `oag_sound::sfx::mod_sound::tests` (position of a one-shot, a loop that follows its
  craft and stops, a loop that ends with its craft or the race).

## Other titles

Nothing here depends on a title: the path is the engine's own cue path, so every title that
runs a simulation mod plays mod sounds identically (**ported**, one mechanism; 2048 and Omega
are checked by the same code, not by a second one).
