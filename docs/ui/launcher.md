# The title chooser

The screen a run shows when nothing names a source and the search path holds
more than one image, or when `--launcher` asks for it. This project's own screen:
no title ships one. When it appears, and what a row that will not open says, is
in [oag-game](../tools/oag-game.md). This page is how it looks and where its
pictures come from.

## Cards (the default)

One card per title, drawn from the title's own system art: the picture the
console shows for the game, read at run time from the player's own files.
Nothing is copied out of an image, cached to disk or shipped.

- **One card per title**, however many images of it there are. Titles appear in
  the order their first image does. Every image of the title, and every picture
  an image carries, is one of the card's *variants*.
- **The cog** at a card's top right, shown only when there is more than one
  variant, steps through them. The caption under the card names the pressing
  shown (platform and serial, or the file name where the source carries no
  serial) with its place, `2/3`. Two pressings may carry the identical picture,
  so the caption is what says the switch happened.
- **The choice is remembered** in `settings.toml`, `[launcher.pictures]`, keyed
  by title. Before the pictures are read, confirming still boots the remembered
  pressing. With nothing remembered, a card shows its first variant with a
  picture.
- **A source with no picture**, or one that will not decode, is a plain card
  with the title's name. No stand-in art is drawn.
- **The same pressing twice** (an encrypted PS3 image beside its decrypted copy:
  one serial) is one variant. A locked copy of a title that already plays gets no
  card and no note.
- **An image that will not open as a title** has a card of its own: one that
  needs a disc key opens the key prompt, any other cannot be selected, and a line
  under the cards says why. At most two such lines are drawn; the list shows
  them all.
- Every card fits on the screen at once, so nothing scrolls and a hover only
  moves the focus.

| input | move | boot | next variant | quit |
| --- | --- | --- | --- | --- |
| pad | d-pad | cross, start | square | the footer hint |
| keyboard | arrows | Enter, X | C | Escape |
| mouse | hover; wheel steps | click the card | click the cog | click ESCAPE QUIT |
| touch | | tap the card | tap the cog | tap ESCAPE QUIT |

The cog's hit area (24x24 of the 480x272 grid) is larger than the glyph.

## The list

One row per disc image, with its title, platform, serial and file name. It is
the view for debugging, and `just launch` uses it. Opt in with
`--launcher-list` (one run) or:

```toml
[launcher]
style = "list"
```

The not-found screen and the disc-key prompt look the same in both.

## Where the art comes from

Read by `oag_source::art`, which goes by what the source *is*, not by which title
it turns out to be:

| source | picture | drawn over it | size | page |
| --- | --- | --- | --- | --- |
| PSP disc (Pulse, Pure) | `PSP_GAME/PIC1.PNG`, wordmark included | | 480x272 | [PSP disc layout](../psp/pulse-disc-layout.md#system-art) |
| PS3 disc (HD/Fury) | `PS3_GAME/PIC1.PNG`, no wordmark | `PS3_GAME/ICON0.PNG`, the logo | 1920x1080, 320x176 | [PS3 disc](../formats/ps3-disc.md#system-art) |
| installed PSN folder (HD) | `PIC1.PNG` | `ICON0.PNG` | 1920x1080, 320x176 | [HD PSN](../formats/hd-psn.md#system-art) |
| Vita folder or `.vpk` (2048) | `sce_sys/pic0.png`, wordmark included | | 960x544 | [Vita packages](../formats/vita-package.md#system-art) |
| PS4 `.pkg` (Omega) | header entries `pic0.png` and `pic1.png`, two variants | | 1920x1080 | [PS4 packages](../formats/ps4-package.md#system-art) |
| unpacked PS4 folder | the same, read out of the title's `.pkg` if one is still on the search path | | | |
| PS2 disc (Pulse) | none | | | |

- The HD logo is drawn as the opaque rectangle it is (`ICON0.PNG` is RGB with no
  alpha): no colour is keyed out. It sits over the left of the picture, at half
  the card's width; the position is this screen's own.
- A `.vpk` is covered by the same file match as a disc image, but no real `.vpk`
  with a `pic0.png` has been read here: the extracted folders have.
- An unpacked Omega folder keeps no system art (the extraction is the package's
  file system, and the pictures are header entries outside it). With the `.pkg`
  deleted the card shows the name.

## How it loads

The art is read, decoded and packed on a worker (`oag_game::launcher::art`,
`oag_thread::Task`) once the chooser is up; cards show names until it lands. A
picture wider than 800 pixels is averaged down to 800 before it goes into the
sprite sheet, which has no mip maps. A dropped disc starts another read.

## Scripting and tests

`oag-game --launcher --drive` walks the chooser with no window: `page` is
`launcher-cards` or `launcher-list`, and `focus` is the focused card's title slug
or what it shows (`<file name>:<picture>`, `none` for a name card). A step waits
while the art is being read.

| claim | test |
| --- | --- |
| each source shape offers its pictures, at these sizes | `crates/source/tests/system_art_ground_truth.rs` |
| the art reaches the screen, a switched picture survives a restart, the flag and the setting bring back the list | `crates/game/tests/launcher_cards_ground_truth.rs` |
| grouping, input, the name fallback, layout | `crates/game/src/launcher/cards/tests.rs` |

## Open

- Looked at headless only, at 1280x800, 1440x816, 1920x1080 and 3840x2160. Not
  seen on a phone or in a browser, where nothing here was run.
- More than six cards shrink to fit; past about a dozen they would be small. No
  set of real images here reaches that.
