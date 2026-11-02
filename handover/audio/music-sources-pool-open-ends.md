---
status: re-first
titles: [pulse, pure, hd, 2048, omega]
updated: 2026-11-02
summary: "Music pool phase 1 landed (MUSIC page, folders, discs); open: add folders from the menu, other formats, menu music from the pool, playlist editing"
---

# The music pool: what phase 1 left

Phase 1 is in `docs/architecture/music-threads.md` ("The music pool"). The code is
`oag_sound::pool` and `oag_sound::folders`, the page is `oag_game::music_pool`.

## Open

1. **Adding or removing a folder from the menu** needs a text input widget (none exists). A
   folder is named in `settings.toml` under `[audio] music_folders` today.
2. **Other formats in a folder.** Only `.mp3` is read. `oag_music` decodes ATRAC3+, ATRAC9 and
   WEM from the discs; WAV, FLAC and Ogg have no reader here. WAV would be trivial; the rest
   wants a `symphonia` feature check.
3. **Menu music from the pool.** The front end keeps each disc's authored loop. A player who
   switches the booted disc off hears the authored menu loop and pool songs in races.
4. **Playlist order and shuffle.** The pool order is fixed (booted soundtrack, discs, folders);
   a folder's files play in path order. No shuffle, no skip, no editing.
5. **Now-playing names.** The HUD names a song only for the booted soundtrack. A pool song shows
   no name; folder files carry ID3 tags that could name them.
6. **The saved race place.** `music.toml` stores an index and a track count of the booted
   soundtrack; a place inside the pool's extra songs is not restored (the saved place fails its
   check and the playlist starts at its normal place).
7. **A pool song is not kept in `race_cache`** (`from` is `None`), so a resume from the menus
   decodes it again on a worker rather than starting at once.

## Next Steps

- A text-input widget lets item 1 land; then a folder row could carry a REMOVE action.
- Item 2: WAV first.
