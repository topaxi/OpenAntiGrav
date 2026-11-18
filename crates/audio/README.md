# oag-audio

Mixing and playback: a tick-driven voice pool and mixer, and the output device
it feeds, or none. Cues are a per-tick output, never part of the simulation's
state.

No gameplay crate may depend on it (dependency rule 1). The policy of what plays
sits above it, in `oag-sound`. See
[ADR-0018](../../docs/architecture/adr/0018-audio-mixer-architecture.md).
