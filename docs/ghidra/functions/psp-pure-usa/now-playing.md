# Pure's race-screen song text: three `Text` widgets, a fade, a 5 second hold

**Binary:** `psp-pure-usa` `BOOT.BIN`, image base `0x08804000`, Ghidra `/pure/BOOT-psp-pure-usa.BIN`. Read 2026-11-25
(`pure-classic-vr-set`), decompiles and disassembly, checked against frames of the original in PPSSPP 1.20.4
(`docs/formats/now-playing.md`, "Pure's own music text"). None of these functions is renamed: they are named here
by address only, so there is no `names.tsv` row.

| Address | Role | Confidence |
| --- | --- | ---: |
| `0x08812164` | The race manager's constructor: looks up the widgets `InGameArtistText` (string at `0x08a44d38`), `InGameTrackText` and `InGameLabelText` and keeps them at `race manager + 0x2164`, `+0x2168`, `+0x216c` | 85 |
| `0x0880f598` | Starts the display: when the music state at `_DAT_00288eec + 0x40` is 1 or 2, sets the artist widget from `_DAT_00288eec + 0xb8`, the track widget from `+0x1b8` and the **label widget from a constant** (`0x240bd0`, so it never shows a label), then state `+0x215c = 0`, alpha `+0x2170 = 0`, step `+0x2174 = 5` | 85 |
| `0x08811da8` | One update of the display (per frame): state 0 adds the step to the alpha and, past `0xff`, clamps to 255, sets state 1 and the timer `+0x2178` to 0; state 1 waits until `5.0 < timer` (float seconds) and sets state 2; state 2 subtracts the step until the alpha is below 0, then state 3; state 3 blanks the three widgets. `0x08811f08` writes the alpha into the top byte of each widget's colour | 85 |
| `0x0880f13c` | The race manager's per-frame update: adds the frame's `dt` to the timer at `+0x2178` (`0x0880f214` to `0x0880f234`) | 80 |
| `0x0882fbec` | The music manager's "start the next track": picks the track (sequential `(i + 1) % n`, previous with flag `0x4`, random with flag `0x1`), copies the node's artist (`node + 0xac`), name (`node + 0x7c`) and label (`node + 0x1ac`) strings, and **calls `0x0880f598` for every track it starts** | 80 |

## What it says about the picture

- The display is plain widgets (the layout is `InGame_Definition.xml`, screen `InGame`: `x 80`, `y 240/250/260`, `HUDSmall`), no bar, no scroll.
- **Fade:** 5 of 255 per update, 52 updates to pass 255 (alpha `> 0xff`), the same 52 down (alpha `< 0`). The frames of the original (30 updates a second) read about 1.8 s each way.
- **Hold:** 5.0 s of accumulated frame time after the alpha reaches 255, so the text leaves at about 1.7 + 5.0 = 6.7 s after it starts: the frames read full at 6.75 s and gone by about 8.5 s on Karbonis and Mandrashee (two songs).
- **Song change:** every track the music manager starts calls the show function again, so the text returns with the same fade and hold for each song (read, not watched: the frames stop at the first song).
- The label widget is set to a constant, not to the node's label string: the original never shows the label (the frames show two lines).

- **Layout ancestry** (checked 2026-11-25 in the extracted `InGame_Definition.xml`): the widgets' ancestors are `Screen name="InGame Screens"` and `Screen name="InGame"`, both with no `Values`, so the authored `x 80`, `y 240/250` are the screen's own (480x272) coordinates with no parent offset; the pair frames put the text at those rows to the pixel.
- **No artist:** `0x0882fbec` copies the node's artist string into its widget whatever it holds, so a track with no artist leaves the artist line empty and the title stays on the track line (`y 250`). Ours does the same (confidence 80, read, not watched). A disc track nothing names reads its number through the string table on the track line, as Pulse's marquee does (**chosen, not measured**).

Not read: what `_DAT_00288eec + 0x40` is called (the show function runs only while it is 1 or 2, and `0x0882fbec` sets it 3 when no track exists), and the other caller condition `DAT_002884b3 != 0 || _DAT_00053534 != 2`.
