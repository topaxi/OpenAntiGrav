# `--until` cannot reach a late movie frame on a machine with an audio device

The movie is clocked by its own sound (ADR-0019) and a headless capture runs far faster than real time, so the playhead only ever advances by the wall-clock seconds the process is alive - a few - while `--until` spends its 3600-tick ceiling and fails with "never reached X". **Every `--screenshot` of a movie leg is therefore an early frame**, which cost this session two wrong readings: a near-white reel frame and a black intro frame were both taken as evidence that a movie was not rendering. `crates/game/src/audio.rs`'s `movie_playhead` already half-documents the cause ("the mixer has to be **moving** as well as sounding"). It would work on CI, which has no device. A `--no-audio` flag, or letting `--dump-audio` drive the mixer deterministically when a device is attached, would make movie legs screenshot-able; until then use a windowed run to check anything that plays.

## Open

- Every `--screenshot` of a movie leg captures only an early frame, not the intended target - already cost two wrong readings.
- No `--no-audio` flag or deterministic audio-driven mixer exists yet to make movie legs screenshot-able.

## Next Steps

- Add a `--no-audio` flag, or let `--dump-audio` drive the mixer deterministically when a device is attached, so movie legs become screenshot-able.
- Until that lands, use a windowed run to check anything that plays.
