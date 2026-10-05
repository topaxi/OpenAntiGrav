# Pulse authors its own text entry, and it is not a keyboard

Found 2026-09-06, while checking whether the disc had a keyboard to play
before this project drew one of its own for the pilot editor's RENAME. It
does not - but it has something better, and nobody had looked.

## Three results, and the middle one was a surprise

1. **No keyboard layout and no key glyphs anywhere on Pulse.** A
   case-insensitive sweep of all four WADs for `keyboard`, `keypad`,
   `alphabet`, `textentry`, `entername`, `nameentry`, `charset` and
   `qwerty` returns nothing but binary noise inside compressed payloads.
2. **`sceUtilityOsk` is refuted, not confirmed** - the standing assumption
   was that a PSP game hands text entry to the firmware dialog. All four
   OSK NIDs are **absent** from both shipped Pulse executables, while
   `sceUtilityMsgDialog*`, `sceUtilitySavedataInitStart` and
   `sceUtilityNetconfInitStart` are all present, as are the must-hit
   controls the method was validated on first. **Confidence 92.**
   Wipeout **Pure is the opposite**: it links the firmware OSK *and* has
   `TagInput`, with a different 43-character uppercase-only alphabet.
3. **Pulse authors the whole text-entry screen in disc data.** Not a key
   grid: a `<TagInput>` widget, a fixed row of `length` character cells
   the player scrolls one glyph at a time, with a `FE_CONFIRM` item
   bracketed by two gradient bars immediately right of the last cell.
   **Confidence 90.**

## Where it is

- **`Data.wad` entry #1083**, hash `b94fe6f9`, 40,083 bytes - holds
  `NameChange` and `TagChange`. **Its name is unresolved**: 16 prefix
  variants were tried and none hashes to it, so it is reachable by index
  only and there is no `names.tsv` row to write.
- 15 `<TagInput>` instances across `Data.wad`: `Name` at `length="10"`,
  `Tag` at `3`, and the online `GameNameTag`/`GamePasswordTag`/
  `UsernameTag`/`PasswordTag` at `14`, carrying `Encrypt="true"` and
  `AllowBlank`.
- Geometry is authored per screen and **must be parsed, not transcribed**:
  cells `28x25` at pitch `30` from `OffsetX="46" OffsetY="88"`, fill
  `0x2fffffff`, text at `y="85"` scale `2.0` in `FEGlobals->TextColor`.
  Those numbers are the parser's test target, not a `const` - see
  CLAUDE.md's "Never invent what the assets already author", which this
  file exists to serve rather than to work around.
- **The alphabet is in the executable**, not the WAD:
  `PSP_GAME/SYSDIR/BOOT.BIN` file offset **2,808,976**, vaddr
  **`0x08AB1C10`** at the standard load base, 70 bytes:
  `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz 0123456789!-+@:?*`.
  Five `lui 0x002b` / `lo16 0xdc10` pairs reference it, all inside one
  `0x0c6xxx`-`0x0c7xxx` code region that also holds the `TagInput`
  element-name reference. **Confidence 85.** The glyph-map rival reading
  is ruled out by Pure's copy having no lowercase at all while its front
  end demonstrably draws lowercase.
- String ids `PRO_ENTER_NAME` and `PRO_ENTER_TAG`, in nine languages.

Full evidence, with every command re-derivable, is in the investigation
scratch file - not committed, because it is 556 MB of extracted disc
content away from being reproducible and this summary is the durable part.

## Why the pilot editor did not use any of it

`crate::prompt::Keyboard` (landed 2026-09-06) is a **grid**, and that is a
deliberate divergence with the evidence above behind it rather than an
unchecked one. Two reasons, both structural:

- **The authored alphabet cannot spell a pilot name.** 70 characters
  including uppercase, a space and `!-+@:?*`; `pilots::check_name` allows
  lowercase, digits, `-` and `_` and nothing else, because the name
  becomes a filename. Using the widget would mean re-authoring a
  38-character subset - invention either way, and invention that looks
  recovered, which is the worse of the two.
- **The geometry is authored for a PSP profile screen.** Adopting
  `OffsetX=46` for a page no PSP has would be less faithful than deriving
  from the live `menu::Skin`, which is what lets an HD skin get an
  HD-sized panel.

A pilot name is this project's own data on this project's own screen. A
profile name is the disc's, on the disc's.

## Landed 2026-09-25: parsed, drawn, and wired as RENAME's other shape

`docs/formats/fexml.md` has a `TagInput` row and section now, with the
schema table and the anonymous-screen gap below written up there.
`oag_ui::screen::TagInput` (+ `Screens::tag_input_from_node`) parses the
element itself, checked in `crates/ui-screens/tests/tag_input_ground_truth.rs`
against `Create Profile Setup`'s own `Tag` - the one instance under a
*named* `Screen`. The other three in this entry - `Name` and the earlier
`Tag` (the ones this project actually draws) - sit under an **anonymous**
`Screen`, which `Screens::collect` walks through without registering (that
rule is shared code and was not changed for this); `oag_ui_screens::tag_entry::geometry`
reads them by walking `fexml::parse`'s tree directly instead, checked
against the real disc in `crates/ui-screens/tests/tag_entry_ground_truth.rs`.
`oag_pulse::tag_input::ALPHABET` is the 70-byte alphabet as a recovered
constant (the same shape `oag_vex::CLASS_*` already is - the runtime never
maps `BOOT.BIN`, so there is no live path to read it from, unlike the WAD
entries above), checked against both pressings' real bytes in
`crates/pulse/tests/tag_input_alphabet_ground_truth.rs`; EU sits at file
offset 2,806,800, USA at 2,808,976, byte-identical.

`oag_ui_screens::tag_entry::TagEntry` is the interactive model, drawn over its own
scrim and panel (the real screen has no live menu behind it to hide; this
one always does). `session::pilot_editor::Session::tag_entry_for_rename`
gates RENAME PILOT onto it: entry parses, `Name`'s geometry is found,
the current name fits the authored `length` (10), and every glyph the name
holds survives `oag_pulse::tag_input::ALPHABET` filtered through
`oag_ui_screens::prompt::accepts` - failing any of those falls back to
`oag_ui_screens::prompt::Keyboard` unchanged, logged at `info!` naming which
condition failed. Screenshotted via `--menu-page pilots --menu-prompt
tag-entry`/`tag-entry-typed` (extended for this, needs `--race`'s own
`source` to read the disc live) against `pulse-psp-eu.chd`, both frames
looked at: a still one and one after four scripted glyph edits, both
legible.

**The authored `scale`/position numbers are not all played verbatim,
and that is recorded rather than silently diverged from**: the disc's
`TagInput.scale` (e.g. `2.0`) is calibrated for Pulse's own renderer, and
feeding it straight into this renderer's `Draw::Text.scale` drew glyphs
several times too large - found by looking at a capture. Glyph and label
*text size* is sized off `Skin::row_scale()` instead, the same convention
`crate::prompt::Keyboard` already uses; every *position* (cell rects, the
confirm label, the bars) is still the disc's own, unchanged.

## Open

- **Input mapping is still unsettled, at 65** - the PPSSPP capture in the
  superseded Next Step 1 below was not taken (time-boxed and skipped this
  round). Implemented as the only reading the cell strip and a `sll`/`sra`
  byte sign-extend support: up/down cycles the glyph under the cursor,
  left/right moves along the row and onto the confirm slot. Labelled
  "chosen, not measured" in `TagEntry::update`'s own doc, no confidence
  score.
- **The filtered alphabet cannot spell every name `Keyboard` can.**
  `oag_pulse::tag_input::ALPHABET` has no `_` at all, which
  `crate::pilots::check_name` allows - so a pilot named with one, or longer
  than 10 characters, always falls back to the grid. Not a bug; recorded
  because condition 4 of this thread's own review asked for it named
  explicitly.
- **Not checked at all**: PS2 Pulse, both EU discs, the HD/Fury PSARC
  interiors (a raw grep cannot see inside them), and 2048 (its PKGs are
  encrypted).
- Whether Pure *uses* its `TagInput` or its firmware OSK, having both -
  still open; Pure's own alphabet is uppercase-only, so even if wired it
  could never be pilot RENAME's fallback.
- Entry `b94fe6f9`'s own **name** is still unrecovered; reached by hash
  throughout, which is what the code already does.
- The alphabet's Ghidra consumer is still unwritten up: confidence 85 is
  the *address*, not a traced function, so no `names.tsv` row - see
  Next Step 4 below, still open.

## Next Steps

1. Recover entry `b94fe6f9`'s **name** with `just mine-names`.
2. Write the alphabet's consumer up as a Ghidra evidence page before any
   `names.tsv` row - the address is solid at 85 but the *function* was
   never traced, and this project's rule is that the page and the row land
   together.
3. Settle the input mapping with a PPSSPP capture of
   `Profile > Your Details > Enter your name`, and correct
   `TagEntry::update`'s doc and confidence once it lands.

## From the HANDOVER.md index (moved 2026-09-25)

a **positive** result found while checking whether the disc had a keyboard to play before this project drew its own. It has no keyboard and no key glyphs, and the standing `sceUtilityOsk` assumption is **refuted at 92** - all four OSK NIDs are absent from both Pulse executables, while Pure links them. What Pulse has instead is a `<TagInput>`: a row of `length` character cells scrolled one glyph at a time, 15 instances in `Data.wad` (entry **#1083**, hash `b94fe6f9`, name unresolved), geometry authored per screen, and a **70-character alphabet in `BOOT.BIN`** at file offset 2,808,976 / vaddr `0x08AB1C10`, at 85. Nothing renders it yet; `../../docs/formats/fexml.md` has no `TagInput` row. Open at 65: the input mapping, one PPSSPP capture away
