# Now-playing display: Pulse wired, the other four titles are found and not wired

2026-10-26, lane `hud-now-playing`. Pulse (PSP) shows the song as the original does:
`oag_hud::now_playing` (geometry off `InGame_Definition.xml`, the executable's fade and scroll
law, the clipped marquee appended to the HUD frame), names from `PI_Music` through
`oag_music::song_names`, the title's row is `oag_pulse::NOW_PLAYING`. Evidence:
`docs/ghidra/functions/psp-pulse-usa/now-playing.md`, `docs/formats/now-playing.md`.

## Open

- **HD/Fury**: the layout is authored (`ingame_definition.xml`, `item InGameMusicDisplay`, a
  textured bar from `HUD_Components.gtf`, 1920x1080 grid). The timing is unread: Ghidra's xrefs
  to the name strings land in a shared string pool; the real consumer is reached through the
  data tables at `0x008a7974`/`0x008a6920` in `/hdfury/EBOOT-ps3-hdfury-eu.elf`. The marquee's
  bar is a `Draw::Fill` today; HD's is a sprite, so `Geometry` needs an image source.
- **Omega**: same item as HD (checked, applies); songs are Wwise states, titles in
  `oag_music::omega::Song`. Executable not read.
- **2048**: no music item in `NEWGUI/InGame_Definition.xml`; the executable has the names.
- **Pure**: no display authored (no `InGameMusic` string in either pressing's `BOOT.BIN`);
  borrowing Pulse's would be chosen, not measured. Lead's call.
- Pulse: no `Dynamic HUD` setting to hide it; split screen draws none; the race music starts
  at load where the original starts it at the green light (the display waits for the light, the
  sound does not); the 60 Hz step is not confirmed by a PPSSPP capture; the original may leave a
  faint empty bar after the fade (the executable never clears the bar alpha after state 2, read
  from the decompile, not seen); PS2 boots show nothing (the PS2 archive has no names).

## Next Steps

- Read HD's consumer of `InGameMusicDisplay` and put a `NowPlaying` row on `oag_hd::MUSIC`; give
  `Geometry` a textured bar; check Omega against it.
- Move Pulse's race music start to the green light, then drop the display's wait.
