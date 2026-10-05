# Screen filters: the PSP's LCDs and the PS2's television, as loadable WGSL

> A **screen filter** is one fullscreen WGSL pass over the finished frame -
> scene, HUD, menus and all - that simulates the display a title was seen
> on. Five ship in the binary; a player's own go in `<config dir>/oag/shaders/`
> and are picked up live. The setting is per title, on the GRAPHICS page.
> The decision is [ADR-0053](../architecture/adr/0053-screen-filters-are-loadable-wgsl-after-the-composite.md);
> the pass is `oag_post::screen`, the catalogue `oag_game::screen`,
> and the frame's plumbing `oag_game::upscale::screen`.

**This page is not reverse engineering and carries no confidence scores.**
Nothing on a disc authors a display; the "never invent what the assets
already author" rule has nothing here to apply to. What it *does* carry is
provenance: where each preset's shape and starting numbers came from, and how
far that is from a measurement - which for every one of them is "not one".

## Using one

1. OPTIONS, GRAPHICS, **SCREEN FILTER**. The list is `off`, the five built-ins
   by name, then any file of yours. It applies on the next frame, to the menu
   you are standing on - that is the only way to choose one.
2. **SCREEN FILTER STRENGTH** below it mixes the filter's output with the
   untouched frame, `100` being the preset as written. It is applied by the
   pass itself after the preset returns, so it means the same for every one.
3. Both persist in `[render_profiles.<title> (<platform>)]` in
   `settings.toml`, **per (title, original platform)**: a PSP disc can hold
   `psp-3000` while the PS2 pressing holds `crt-interlaced` and Wipeout HD
   holds `off` - and Pulse's own PSP and PS2 rows are two different rows now,
   not one shared between them.

From the command line, `--screen-filter <id>` overrides the profile for one
run, and a `--presented --screenshot` capture is the way to see one in a
still: an ordinary `--screenshot` is the scene as drawn, for the reason the
grade is left out of it (see `crates/game/src/upscale.rs`'s module doc).

```sh
just play --race --screenshot /tmp/crt.png --ticks 300 --hold cross \
    --presented --size 1920x1088 --screen-filter crt-shadow-mask
just play --screenshot /tmp/menu.png --menu-page graphics \
    --presented --screen-filter crt-shadow-mask
```

The second is the check that the filter covers the menus - which is the
reason it runs where it does - and it does: the GRAPHICS page comes back
curved, masked and scanlined, the same as the race. (`--menu-page` places no
cursor, so the SCREEN FILTER rows themselves, eleventh and twelfth on the
page, are below its fold; `crates/game/tests/menu_screen_filter.rs` scrolls
to them and checks what is drawn.)

Both captures read the same catalogue the window does, this machine's own
`shaders/` directory included, exactly as `--presented` already reads the
file's brightness and gamma: two machines whose directories shadow the same
id differently get different pictures from the same command. With
`--screen-filter off` a `--presented` menu capture is byte-identical to the
plain one - checked 2026-09-16, so the path itself resamples nothing.

A still of a **temporal** preset is one frame of it with a black history:
`crt-interlaced` shows one field lit and the other dark, and a persistence
preset shows its first frame's fade-in. That is what the frame is, not a
defect of the capture.

## The built-ins

| Id | What it draws | Grid it keys to |
| --- | --- | --- |
| `psp-3000` | The 3000's sharper panel: a shallow horizontal luminance ripple, a second ripple that appears only where the picture moved since last frame, an RGB stripe, low persistence. **Resolution-independent** - the structure is in output pixels, so a 1080p window is a 1080p-dense 3000 panel rather than a 272-row one stretched over it. | output pixels (`cell` tunable) |
| `psp-lcd` | The 1000/2000 panel as a grid: the frame resampled onto 480x272 cells with a dark border and an RGB stripe in each, lifted blacks, and the strong persistence those panels were known for. `native_grid = 0` keeps the window's pixels and draws the grid over them. | `native_size` |
| `crt-interlaced` | The PS2 port on a television over composite: one field lit per frame alternating with the frame counter, the other field last frame's output times a decay, chroma blurred wider than luma, a slot mask, a barrel and dark corners. | `native_size` |
| `crt-shadow-mask` | A curved consumer CRT: gaussian scanlines on the title's own line count, a three-tap gaussian beam across columns, a shadow mask (or an aperture grille, or none), a barrel and dark corners. | `native_size` |
| `crt-aperture` | A flat-faced aperture-grille monitor: vertical phosphor stripes, a beam whose spot widens with brightness, a little sideways bloom, two damper wires. | `native_size` |

Every tunable, with its default and range, is in the file's own header -
`assets/shaders/screen/<id>.wgsl` - and the comment under the header says
what the preset is doing and why.

**The CRT presets want three or more window rows per scanline**: 816 rows for
the PSP's 272 lines, 1350 for the PS2's 448. Below that the line structure
beats against the pixel grid, which is the same limit every CRT shader has
and not one this project can lift.

### Provenance, and what is not claimed

**The PSP presets are a plausible model, not a measurement.** They were
shaped from a design discussion - a ChatGPT thread the maintainer shared on
2026-09-16, not tracked here - whose own conclusions were:

- The PSP-3000's "interlacing" is **not fields**. Contemporary reports (Sony's
  own statement, Exophase and NeoGAF threads of October 2008 and later)
  describe thin horizontal lines or combing appearing *during motion* around
  high-contrast edges; later close-up photography attributes the structure to
  the 3000 panel's horizontal subpixel arrangement, with the older panels'
  slower response and lower contrast hiding the same structure. So `psp-3000`
  draws a shallow, never-black ripple, deepened where the frame differs from
  the last one, and does not alternate fields - the thread corrected itself
  on exactly this point midway, and a field-alternating PSP would be a CRT.
- The structure should be derived from the **output** resolution rather than
  a hard-coded 272 rows, so that a render scale above 100 % is shown through
  a correspondingly denser panel. `psp-3000`'s `cell` is that choice, at 1.
- The starting numbers - `0.035` static ripple, `0.075` motion ripple, `0.12`
  persistence - are the thread's "likely starting point" row, from a table it
  offered running from `0.015/0.035` (very subtle) to `0.08/0.18` ("I
  definitely remember this"). They are tunables with that provenance and
  nothing more.
- The 3000 was reported as more saturated than the 1000/2000, and the 2000 as
  bluish next to it (a PPSSPP developer's remark in an issue on accurate PSP
  colours). Both presets carry a colour tunable for it, **at neutral**: a
  colour transform is the easiest thing here to get plausibly wrong, and
  nothing was measured against a panel.
- `psp-lcd`'s `persistence = 0.4` is PPSSPP's own default for its "LCD
  Persistence" shader (range 0.3 to 0.8), which its author describes as the
  mix a true LCD experience wants; `black_lift = 0.04` is a guess at the
  1000/2000's grey blacks against the 3000's.

**The CRT presets are textbook.** Gaussian beam profiles summed in linear
light, phosphor masks, barrel warp, YIQ chroma bandwidth, field alternation
with phosphor decay: every CRT shader of the last decade is built from these,
and so are these three, written fresh. None is a transliteration of any
published shader - `crt-shadow-mask` is *in the style of* the well-known
public-domain arcade-monitor shader, in the sense that it uses the same
technique with the same parameter shapes, and shares no text with it. See
"Why these and not a port" below.

## Writing one

A preset is one `.wgsl` file. Its **id is the file stem**, which is what the
settings file and the menu store. Put it in `<config dir>/oag/shaders/` -
`~/.config/oag/shaders/` on Linux - and it appears in the SCREEN FILTER row
within a second, with no restart. Save an edit and the running game rebuilds
the pass; a file that will not compile leaves the last good version on
screen and puts naga's report, with the line marked, in the log. **A file with
a built-in's stem replaces the built-in**, which is how a built-in is tuned:
copy it out of `assets/shaders/screen/`, change the defaults, done.

### The file

```wgsl
//! name = "My filter"
//! description = "One line, optional."
//!
//! [[param]]
//! name = "amount"
//! default = 0.5
//! min = 0.0
//! max = 1.0
//! step = 0.05

fn screen_filter(uv: vec2<f32>, pixel: vec2<f32>) -> vec3<f32> {
    let now = frame_at(uv);
    let before = previous_at(uv);
    return mix(now, before, param_amount());
}
```

The leading `//!` lines are a TOML document: `name` (what the menu shows;
the id if absent), `description`, and up to sixteen `[[param]]` tables. Each
becomes `fn param_<name>() -> f32` in the compiled module, in declaration
order; `min`, `max` and `step` are validated (the default must lie inside
them) and reserved for a tuning row that does not exist yet. The first line
that is not `//!` ends the header.

The body defines `screen_filter`, returning the colour for the output pixel at
`uv` (`0..1` across the frame) and `pixel` (its centre in output pixels,
`0.5, 1.5, ...`). It runs against this prelude, which is
`crates/post/src/screen.wgsl` and is the whole contract:

| Name | Binding | What |
| --- | --- | --- |
| `frame` | `@group(0) @binding(0)` | The finished frame at presentation size - scene, HUD, menus and all |
| `frame_sampler` | `@binding(1)` | Bilinear, clamp to edge |
| `screen` | `@binding(2)` | The uniform, below |
| `previous` | `@binding(3)` | What this filter itself produced last frame; black on the first frame and after a resize |
| `nearest_sampler` | `@binding(4)` | Nearest, clamp to edge |

```wgsl
struct Screen {
    output_size: vec2<f32>,  // the frame's size in pixels
    texel: vec2<f32>,        // 1 / output_size
    native_size: vec2<f32>,  // the title's authored grid: 480x272, 640x448, 1920x1080
    frame: f32,              // frames this filter has drawn
    time: f32,               // seconds since the filter was built
    strength: f32,           // the strength row, applied for you after you return
    _pad0: f32, _pad1: f32, _pad2: f32,
    params: array<vec4<f32>, 4>,  // the tunables; read them through param_<name>()
}
```

Helpers: `frame_at(uv)` (bilinear), `frame_pixel(uv)` (nearest),
`previous_at(uv)`, `luma(rgb)` (Rec. 601), `param(i)`.

**Two rules the compiler enforces**, and one it does not:

- `filter` is a WGSL reserved word, hence `screen_filter`.
- Every `textureSample` has to be in uniform control flow. An early
  `return` that some pixels take before a later sample is refused by naga -
  compute a factor and multiply at the end instead, the way the CRT presets'
  `on_glass` does.
- Nothing bounds what a preset costs. A hundred taps per pixel is a hundred
  taps per pixel, and it shows on the performance overlay like anything
  else; the overlay itself is drawn after the filter and is never inside it.

The pass reads and writes the same gamma-encoded space the presentation
target is in ([ADR-0020](../architecture/adr/0020-gamma-authoritative-colour-space.md));
a preset that sums light - the CRT ones - decodes with a 2.2 power, sums, and
re-encodes, and that is the preset's own business.

### Where it runs

Presentation target, **filter**, grade, surface - inside
`Framebuffer::composite`, after the UI has composited and before brightness
and gamma. The filter is part of the picture; the grade is the player's
monitor. The performance overlay draws onto the surface afterwards and stays
outside. Off costs nothing: the pass and its two history textures exist only
while a preset is selected.

## Why these and not a port

The obvious thing to ship is a port of a known CRT shader, and the reason
none is shipped is the reason `oag_post::fxaa` gives for NVIDIA's
FXAA: the collections these come from - libretro's `slang-shaders` and
`common-shaders`, the Mega Bezel work, most of what a PPSSPP shader pack
holds - are GPL, and this MIT/Apache-2.0 tree cannot vendor or transliterate
them. The public-domain arcade-monitor shader is the exception in licence and
was still not copied, so that no reader has to check which lines are whose.

The user directory is the answer for all of them: a GPL shader ported to this
contract by whoever wants it, in their own config directory, is theirs to
carry. The contract is one function against five bindings and a struct, which
is small enough that a port is an afternoon and a converter from PPSSPP's
`.fsh` (`sampler0`, `sampler2`, `u_setting`, `v_texcoord0`) is a script.

## Verification

- `oag_post::screen::tests` - the header, the accessors, naga's
  refusals, the uniform's size. No device.
- `oag_game::screen::tests` - every built-in parses and validates; a user
  file joins, shadows a built-in, is re-read on a rewrite, keeps its last
  good revision when broken, and falls back when deleted.
- `oag_game::upscale::screen_tests` - on a device: a passthrough composites
  what it was given, a preset sees its own previous output on the second
  frame, the strength row mixes, a broken preset draws unfiltered, every
  built-in builds. Skipped where there is no adapter.
- The `--presented` captures above, looked at rather than counted - the same
  rule [README.md](README.md#measuring-a-renderer-change) gives for any
  resampler.
