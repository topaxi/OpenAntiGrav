# Omega: photo mode

Read 2026-11-06, lane `photo-read`, `/omega/eboot-ps4-omega-eu.bin`, against 2048 and HD.
Player-facing law: [docs/ui/photo-mode.md](../../../ui/photo-mode.md).

**Verdict: checked, differs.** Omega keeps `PhotoMode_Screen.cpp`, `WV_PhotoMode_Screen.cpp`
(`C:\WOPS4\Wipeout\Code\Frontend\Screens\`) and HD's `InGame Photo` and has its own rows. `shadPS4` has
no memory readout, so **a live Omega value is not available**; nothing below was run.

| Address | Name | Conf. | What it does |
| --- | --- | ---: | --- |
| `0x01561110` | `PhotoMode_ScreenEnter` | 80 | Saves camera and stick state, rebuilds the camera list, binds `ShareButton` |

`FUN_0155e450` also names `Photo Use MB` and is the likely per-frame update. It was not read and
keeps its name. Recipe: `decompile_function 0x0155e450`, match to `0x8112ea02` by the strings.

## Against HD

`ingame_definition.xml`, `data09.psarc` (111,769 bytes; `data09` carries the only complete front
end, `docs/formats/omega-status.md`):

| Row | HD | Omega |
| --- | --- | --- |
| Camera list | XML, `internal`, `close`, `track`, default `OPT_INT` | Rebuilt in code: `close`, `track` only when `DAT_01f99bc0 == 1`, `internal`; default `OPT_CLOSE` |
| `Photo Blur` (speed blur) | present | gone |
| `Saturation` | present | gone |
| `Exposure` | default set on entry | XML `default="50%"` |
| Effects | One list, `Photo Use DOF` | Two lists, `Photo Use MB` and `Photo Use DOF`; `Shutter Speed` 70, `DOF Focus Distance` 5, `DOF Aperture` 40 |
| Take photo | `FE_CONFIRM_BUTTON`, `MSC_PHOTO_TAKE` "TAKE PHOTO", writes a JPEG | `ShareButton`, `MSC_PHOTO_TAKE` is **"SHARE"**; no `.jpg` format string, `libSceShareUtility` imported |

Omega's own shot is the console's share function, so it records no resolution or format of its own.
`CHANGEINFO_102` in the language file: "Changed photo mode UI hiding behaviour".
`Race End Photo Redirect` stays; `MSC_PHOTO_PRESS_SELECT` reads "PRESS OPTIONS BUTTON FOR PHOTO MODE".

`gui/vita/vita_ingame_definition.xml` ships `InGame2048` with the Glory Moment widgets,
`WV_PhotoMode` strings stand in the binary, and `photomode_dots.gnf`, `Photomode_controls.gnf` and
`photo_click.xml` ship. Whether that touch screen is reachable is unread; PS4 has no pause entry
pointing at `InGame VitaPhoto` in `ingame_definition.xml`.
