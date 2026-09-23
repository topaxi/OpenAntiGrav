# `/pure/BOOT-psp-pure-eu.BIN` - a `Movie` widget's `localised` suffix is a per-pressing literal, not a runtime language read

Where `Movie_ParseAttributes` builds the entry name a `localised="true"` `<Movie>`
widget resolves to, and why the answer is the same shape as
[`title-screen.md`](title-screen.md)'s `TitleFrame` wordmark finding: a single
literal string baked into each pressing's own executable, not a value chosen at
boot from a language or region setting. See
[`docs/formats/pure-status.md`](../../../formats/pure-status.md)'s "The
dev/pub reel and second boot movie's own region suffix" section for the bug
this closes and where
`oag_pure::names::INTRO_MOVIE_CUTS`/`FMV_INTRO_MOVIE_CUTS` were recovered.

| | |
| --- | --- |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pure EU, `UCES-00001`), image base `0x08804000`, and Pure USA (`UCUS-98612`), same base |
| **Decompiler constant offset** | Same as `title-screen.md`: every `lui`/`addiu`-built `.rodata` address this decompiler prints is the real address minus `0x08804000`. |

## The suffix is a literal, one per pressing, appended unconditionally when `localised="true"`

`Movie_ParseAttributes` (`0x088ca968` EU / `0x088cb0cc` USA, bodies identical
past relocated immediates) is the generic `<Movie>` node attribute parser: it
confirms the node carries a `Values` child (the same "`Values` carries its
parent's attributes" convention `oag_ui::screen`'s own module doc already
names), walks the node's attributes comparing each name against `src`,
`sound`, `width`, `height`, `autoredirect`, `autostart` and `localised`
(string literals at `0x08a50118`-`0x08a5015c` EU, `0x08a54ec4`-`0x08a54f08`
USA - read directly off memory, not inferred), then assembles the movie's
resolved name:

```c
// iVar2 = &movie->src_out (local_34 + 0x1dc)
if (src[0] != '\0') {
    strcpy_bounded(iVar2, src, 0x80);
    if (localised[0] == '\0' || strcasecmp(localised, "true") != 0) {
        strcat(iVar2, ".PMF");        // 0x24c17c EU / 0x250f28 USA
    } else {
        strcat(iVar2, "_EU.PMF");     // 0x24c170 EU - literal, not a variable
    }
}
```

The USA binary's decompile is the same function shape with one string
different: `strcat(iVar2, "_US.PMF")` at the analogous offset. **Read directly
off memory on both binaries** (`inspect_memory_content`, not just the
decompiler's constant folding):

| Address | Bytes around the suffix | Binary |
| --- | --- | --- |
| `0x08a50168`-`0x08a5017f` | `true\0\0\0\0` `_EU.PMF\0` `\0\0\0\0` `.PMF` | Pure EU |
| `0x08a54ec4`-`0x08a54ecc` region, suffix at `0x08a54f1c` | `true\0\0\0\0` `_US.PMF\0` `\0\0\0\0` `.PMF` | Pure USA |

**Neither binary's string table contains any suffix but its own.** A regex
search for `_US\.PMF|_JAP\.PMF|_KO\.PMF` against the EU binary and for
`_EU\.PMF|_JAP\.PMF|_KO\.PMF` against the USA binary returns nothing in
either direction - there is no language table, no region enum and no second
suffix sitting unused anywhere in either executable's `.rodata`. If a JAP or
KO pressing exists, this project has neither disc to read what suffix it
bakes; nothing here says it must be `_JAP`/`_KO`, only that this project's two
pressings each bake exactly the one suffix that matches their own serial.

This is the same shape of finding `title-screen.md` already made for
`TitleFrame`'s wordmark texture, not a coincidence worth re-deriving: a
`localised="true"` widget's regional cut is chosen by **which executable is
running**, not by a runtime `sceUtilityGetSystemParamInt` language read or any
value this project has found stored in save data. `oag_pure::frontend::title_frame_src`
already keys its own per-pressing literal off `oag_assets::Layout::serial`;
`Movie::entry_name` (`crates/ui/src/screen.rs`) is the analogous fix, needing
the same serial plumbed through rather than a hardcoded `_US`.

Confidence **85**: the decompile is unambiguous on both binaries, the two
bodies are structurally identical past the relocated literal (the same
cross-binary corroboration `title-screen.md` used), and the resolved names
already hash to real, independently-verified `Data.wad` entries
(`oag_pure::names::INTRO_MOVIE_CUTS`/`FMV_INTRO_MOVIE_CUTS`, recovered before
this pass by hashing the suffix pattern and confirming the entry exists).
Short of `title-screen.md`'s 90 because this function was read statically
only - no breakpoint confirms it fires during a real boot the way
`TitleScreen_AssignWordmarkTexture`'s was confirmed live.

Named `Movie_ParseAttributes` in both binaries' `names.tsv`.

## Applied names

| Address | Name | Confidence | Binary |
| --- | --- | --- | --- |
| `0x088ca968` | `Movie_ParseAttributes` | 85 | Pure EU |
| `0x088cb0cc` | `Movie_ParseAttributes` | 85 | Pure USA |

## Next steps

- No JAP or KO Pure disc is in this project's corpus, so which suffix such a
  pressing bakes (if one exists at all) is unread. Leave `title_frame_src`'s
  and `Movie::entry_name`'s own fallback at EU rather than guessing a third
  literal.
- A live breakpoint on `Movie_ParseAttributes` during a real cold boot would
  close the gap to `title-screen.md`'s confidence-90 band; not done this pass.
