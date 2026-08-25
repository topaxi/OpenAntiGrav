# Wipeout HD's soundtrack plays; finding it fixed a disc-wide PSARC bug

2026-08-17. 36 plain MPEG-1 Layer III files under `/data/music/`, read through the same `oag_game::catalogue::music` schema all three titles share. **The bug, not specific to music**: `oag_formats::psarc` applied the `0x78` zlib-marker test to every block including full-size stored ones, which are raw by definition - one MP3 block that happens to start `78` was handed to `miniz_oxide` and the whole entry read as unreadable. Fixed and pinned by a fixture; any HD asset with such a block was affected. `symphonia` (MP3 only, `default-features = false`) is now the dependency for this path rather than ffmpeg, per [ADR-0024](../docs/architecture/adr/0024-in-process-codecs-and-ffmpeg-as-a-last-resort.md). `crates/game/tests/hd_music_ground_truth.rs` (9 tests), [hd-status.md](../docs/formats/hd-status.md#music-plain-mp3-declared-the-way-the-psp-titles-declare-theirs). **Still open**: HD's front end is `None` (declared boot chain, not measured), so the front-end track has never been *heard* even though it is decoded and pinned by a dedicated test; `FEship.mp3`, a second front-end track, has an unread trigger and is unwired; and base-stereo versus `_fury` front-end cuts are confirmed different music, not two encodes, but which plays when is unwired. The `CustomMusicManager`/`/dev_hdd0/` strings in the ELF are the PS3's own custom-soundtrack feature, not disc content - not a thread.

## Open

- HD's front-end music track has never been heard in a running front end - the boot chain is declared, not measured - though it is decoded and pinned by a test.
- `FEship.mp3`'s trigger is unread and the track is unwired.
- Base-stereo versus `_fury` front-end music cuts are confirmed different encodes, but which plays when is unwired.

## Next Steps

- No next step named in the original record - read the prose above and decide one.
