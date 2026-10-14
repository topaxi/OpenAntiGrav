# The front-end colour template: which block overwrites `HD_Colours`

2026-10-14. `FUN_0154dd80` in the base `eboot.bin` (`/omega/eboot-ps4-omega-eu.bin`,
image base `0x01000000`) is the one function that names `WOHD_Colours` (six
`LEA`s) and `Fury_Colours` (six), and the only function Ghidra finds writing (twice) the byte at
`0x01a7d348` that the rest of the front end reads (23 readers) to ask which
style it draws. **Left unnamed**: it also lays out the boot dialog
(`HDDBootDialog`, `ProfileSaveIcon`, `GhostSaveIcon`) and picks the legal line
(`AmericanLegalLine`), so no one name is true of it. The behaviour below is what
the lane needed; the rest of it was not read.

## The rule (confidence 90)

Per branch it does this, six times, once per name in
`HD_Blue, HD_Grey, HD_LightGrey, HD_White, HD_BG, HD_transBG`:

```
base   = Screen(DAT_01a7d738, "HD_Colours").Variable(name)
tmpl   = Screen(DAT_01a7d738, "WOHD_Colours" | "Fury_Colours").Variable(name)
strcpy(base.value_buffer /* +0x100 */, tmpl ? tmpl.value /* +0xd8 */ : "")
```

(`FUN_013cd050` finds a child screen by name, `FUN_013cbc60` a child widget by
name, both read off their uses, not decompiled.) So **the template's value string is copied over the base block's
variable of the same name**, and a template variable that is missing copies the
empty string. The values are copied in place, which is why the template's
`<Variable>`s author `name=` and **no `global=`**: they are never registered as
globals themselves. `HD_SubGrey` is not in the list - all three blocks carry it
with `global=`, a registration, and its value is the same (`0xff646464`) in the
two blocks the boot can pick. **How the original's registry treats three
registrations of one name is unread**: this build keeps the last, `0xff646464`; a
registry that kept the first would hold `HD_Colours`' `0xff767476`.

`skin.xml`'s own comment on the base block says the same in prose: *"THESE
COLOURS ARE IGNORED AND OVERWRITTEN by the WOHD_Colours and Fury_Colours
templates below"*.

The branch also writes `DAT_01a7d348` (`0` for `WOHD_Colours`, `1` for
`Fury_Colours`) and toggles `|= 6` / `&= 0xf9` on the `bgAnim` widget of
`FE Screen`, and flips a byte at `DAT_020f8c98 + 0x74`. None of that is wired
here; see "Open".

## Which template: always `WOHD_Colours` (confidence 85)

The branch condition is `strcasecmp(&DAT_0182d98e, &DAT_0182d98e)`: the
disassembly is `lea rdi, [0x182d98e]; mov rsi, rdi; ...; call strcasecmp; test
eax, eax; jz <WOHD branch>`, the literal at `0x182d98e` being `"HD"`. A string
compared with itself is zero, so the `WOHD_Colours` branch is always taken and
the `Fury_Colours` branch is unreachable in this build. No `<List
global="FE Style">` row exists in any of the 45 front-end XML files of
`data09.psarc` (`grep -i "FE Style|OPT_FE_STYLE"`), which agrees: Omega offers no
choice. HD, which does, has `OPT_FE_STYLE` and a byte at `0x009947c8`
(`FrontEnd_IsFuryStyle`, [menu-blocks.md](../ps3-hdfury-eu/menu-blocks.md)).

**The patch's `eboot.bin` is what runs**, and it was read too (it is not in the
Ghidra project): its SELF segment table maps file offset `0x5430` to virtual
address `0` (text) and `0x99d990` to `0x99c000` (data); the six `WOHD_Colours`
references are at `0x5703ed ... 0x5705d9` (add the image base for Ghidra's
addresses), and the selector at `0x5700da` is the same shape:

```
5700da lea rdi, [rip + 0x32208e]    ; 0x89216f = "HD"
5700e1 mov byte ptr [rip + 0xad7d2c], 1
5700e8 mov rsi, rdi
5700f3 call 0x7fed00                ; strcasecmp
5700f8 test eax, eax
5700fa je  0x5703ba                 ; the WOHD_Colours branch
```

The patch has a string the base lacks, `WOHDFURY`; nothing was read of what uses it.

## Why the compare is against itself (not read)

Probably a compile-time constant standing where a setting once stood, the
remains of an option Omega dropped. That is a guess and nothing here depends on it.

## Open

- The `bgAnim` flag change on `FE Screen` in the `WOHD_Colours` branch (`|= 6`):
  what bits `0x2` and `0x4` of `+0x60` mean on that widget is unread, and Omega's
  menus draw the `BackgroundAnim` scene regardless
  ([omega-status.md](../../../formats/omega-status.md)). If the bits hide it the
  original shows a plain white page, which would be a visible difference.
- The `DAT_020f8c98 + 0x74` byte and `FUN_01730e80`, called on a flip.

## Cross-checks

- **HD/Fury** (`EBOOT.elf`): `strings` finds `HD_Colours` and neither
  `WOHD_Colours` nor `Fury_Colours`; HD's `skin.xml` carries one `HD_Colours` block
  per archive, baked (Fury's values in `DATA00`, HD's in `DATA06`). Checked,
  differs: HD never runs this copy, and its own switch is
  [`FrontEnd_IsFuryStyle`](../ps3-hdfury-eu/menu-blocks.md).
- **2048** (`eboot.elf` v1.04): `HD_Colours` is the only such string, and
  `NEWGUI/Skin.xml` carries that one block inherited verbatim
  ([2048-frontend.md](../../../formats/2048-frontend.md)). Checked, differs.
