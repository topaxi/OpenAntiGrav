# Omega's music is a Wwise `Music_Track` state per artist; nothing plays it yet

2026-10-05. Omega is the one title that plays no music. The evidence for
everything below is in `docs/formats/wwise.md`, "Music: a `Music_Track` state
per artist". Do not re-derive it.

What is read: the playlist (`data/plugins/music/Definition.xml`, 29
`PI_Music` entries), the switch group (`Music_Track`), its states (the
artist's name with spaces removed, 25 of 28 named), and the media (ATRAC9
`.wem` streams, 154 stereo and 11 8-channel among 165 loose files).

## Open

- The music switch's association tree (state to segment), segment child lists
  and the music playlist containers are unread, so state to `.wem` is inferred
  rather than walked.
- Three state hashes are unnamed (`404193461`, `3252658421`, `4067886831`);
  Code Manta, Burufunk and Noisia are the unmatched artists.
- 11 music streams are 8-channel; `oag_formats::wwise::wem` decodes mono and
  stereo only.
- `PlaylistId` 1 versus 2 is unread (two in-game playlists?).
- What sets the state at race start (an event per state, or code) is unread.

## Next Steps

1. Read the type 12 music switch's tree and the type 10 segment and type 13
   playlist child lists in `oag_formats::wwise::hirc`, and walk
   `Music_Track` = `fnv1(artist without spaces)` to its segments' track
   sources. A wrong layout shows as children that resolve to no object.
2. Wire it the way 2048's and HD's playlists play, from `PI_Music`: the
   location order, artist and title from the XML, audio from the walked
   `.wem`. Skip the 8-channel streams and say so, or add 8-channel ATRAC9.
3. Prove it with a headless race WAV, never the speakers.
