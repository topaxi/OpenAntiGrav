# RPCS3's 16-bit generated index buffer: which HD references it touches, and the build without it

Measured 2026-11-05 on `hdfury-ps3-eu-dec.iso`, 1920x1080, private profile, lane `rpcs3-quads-index-census`. Base build
RPCS3 `v0.0.42-19980-028d1e8f`, the packaged binary every HD reference was taken with.

**Only the Fury backdrop is touched: every Fury menu frame carries the wrap, no race frame does. Confidence 90 on the census.**

## The defect

Read at `028d1e8f4`. `vk::get_appropriate_topology` (`VKVertexBuffers.cpp`) returns "emulated" for `line_loop`, `quads` and
`polygon` on Linux. `triangle_fan` and `quad_strip` are native there, so they have no such path. An emulated draw without
guest indices takes `generate_emulating_index_buffer`: `index_count * sizeof(u16)`, `VK_INDEX_TYPE_UINT16`, filled by
`write_index_array_for_non_indexed_non_native_primitive_to_buffer` (`BufferUtils.cpp`) through a `u16*`. Index `4 * i` truncates, so
quad 16,384 reuses quad 0. The OpenGL backend (`GLVertexBuffers.cpp`) shares the writer and has the same width.

A draw with guest indices (`draw_indexed_array_command`) uses `get_index_type(rsx::method_registers.index_type())`, the game's own
width, whether or not its primitive is emulated, so it cannot wrap here. Confidence 95 on this section: the source is
unambiguous, and the wider build below changes the picture exactly as it predicts.

## Census

`scripts/rsx-large-draws.py <dump dirs>` walks the same ring dumps as `rsx-pass-events.py`. The patched build also logs one line per
new peak, per primitive and draw command (`oag-draw-peak`, `RPCS3.log`), which sees every draw of a run rather than one frame. No
`.rrc` RSX capture exists on this machine.

| Screen or state | Evidence | Largest draw | Emulated? | Wraps? |
| --- | --- | --- | --- | --- |
| Main Menu, Campaign Select, Campaign grid, Campaign cell, Racebox setup, Track Select, Ship Select, Options | one ring dump each, patched build | `QUADS` arrays 213,956 to 214,408, twice in a frame (`#1` and the mirror pass) | yes | **yes** |
| every other Fury menu frame on disc | 38 older dumps, two lanes | `QUADS` arrays 208,380 to 220,000 | yes | **yes** |
| Track Select | same | `TRIANGLES` indexed 49,239 | no | no |
| Zone, Moa Therma | peak log; two older dumps | `TRIANGLES` indexed 84,687 | no, guest indices | no |
| 22 race states, one per save in `data/saves/hd-fury/`: all twelve circuits, grids, pack, mid-race weapons | peak log over 22 s of held throttle on a fresh boot per state | `QUADS` arrays 1,536; `TRIANGLES` indexed 11,904; `TRIANGLE_STRIP` 431 | QUADS yes | no, about 40 times under the limit |
| 98 older race dumps | ring walk | `QUADS` arrays 1,648 | yes | no |

No `LINE_LOOP`, `POLYGON`, `TRIANGLE_FAN` or `QUAD_STRIP` appears in any dump or peak log. 31 older dumps walk to no primitive at all (the
ring's head was never dumped: `hd-capture/*-matched`, `talons-fifo*`), so they are no evidence; their circuits are covered above.
Not walked: the HD-style front end, and a race with a full late-race weapon exchange past what the pack state shows.

| Compare-page pairs | Verdict |
| --- | --- |
| `hd/menu-campaign-select`, `menu-cell-select`, `menu-grid-select`, `menu-ship-select`, `menu-track-select`, `gap-main`, `gap-options`, `gap-racebox` | wrapped. Re-taken on the wide build and held, not adopted into `data/reference` |
| `hd/loading-loading` | clean: one ring dump 3 s into a Vineta K load, `QUADS` arrays 4, `TRIANGLE_STRIP` 1,415. One frame, seen once |
| the 13 `hd/<circuit>` pairs, `hd-race/talons-pack` | clean: same circuit in the race census. Inferred from another frame of that circuit, with a margin of about 40 times |

## The patch

Local only, deliberately untracked: three diffs against `028d1e8f4` under `data/tools/rpcs3-wide-indices/`, applied and built by `build.sh` beside them. Rebuilding needs that directory; the changes are:

- `rpcs3-wide-emulated-indices.patch`, the fix: the writer goes through `u32*`, and both backends allocate and bind 32-bit
  indices. 9 lines added, 22 removed, most of them the SSE `iota16` that only `line_loop` used.
- `rpcs3-draw-peak-census.patch`, the log line above. No effect on a frame.
- `rpcs3-gnu-ld-ffmpeg-group.patch`, a link group GNU `ld` needs, nothing to do with the defect.

Two traps. This commit does not compile against LLVM 23 (`ConstantFoldInstruction` wants an `Instruction *`), so LLVM comes from the
22.1.8 submodule. A second CMake pass over a configured tree rewrites the vendored LLVM's flags and its compile then fails against
libstdc++ 16, so the patches go in before the first configure. With `ccache` the first full build took 11 minutes at `-j 10`.

The built binary is `data/tools/rpcs3-wide-indices/src/build/bin/rpcs3` (Qt and system libraries, LLVM static; it reports `HEAD | local_build`).
It boots the Fury disc headless with `--no-gui`, and restored all 23 HD save states taken on the stock build. Not for upstream.

## Taking a reference with it

    OAG_RPCS3_BIN=$PWD/data/tools/rpcs3-wide-indices/src/build/bin/rpcs3 OAG_COMPARE_PLATFORM=hd-fury \
        uv run --with evdev python3 scripts/rpcs3-drive.py --image data/images/hdfury-ps3-eu-dec.iso serve

`OAG_RPCS3_BIN` is the way in: `uv run` puts `/usr/bin` first on `PATH`, so a wrapper directory on `PATH` silently runs the packaged
build. **Read the first line of `RPCS3.log` before believing a capture**: two sets here were labelled patched and were stock.

Then `scripts/rpcs3-hd-menu-pair.py` as usual. Rules this needs, each paid for here:

- The Main Menu leaves for the attract demo after an idle minute or so, and a poke or a pause carries on into it. Tap a button
  every 20 s and refuse a frame whose screen is not `Main Menu`.
- The cloud does not change while `g+0x414` forces a path. Release, wait for the count to change, force again; 19 clouds take
  minutes, and one patched boot went four cycles of about nine minutes each without offering `mirage_c1` once.

## The re-capture

Frame `484` of path 6, cloud `mirage_c1` (count 53,602, `QUADS` 214,408 in the ring on both builds), music factor poked to `0`, particle colour pinned,
glow at its floor. Ours is the same state through the scratch overrides of cloud and colour on `main` at `70166e693`. Rows `230..790`, columns `600..1900`.

| Natural frame 484, `pc 0.088` | R mean | R q90 | R q99 | RMSE to ours | RMSE to `paused-02.png` |
| --- | ---: | ---: | ---: | ---: | ---: |
| stored reference `paused-02.png`, wrapped | 48.3 | 137 | 255 | | |
| stock, this state, boot A, 2 frames | 43.4, 45.3 | 127, 132 | 255 | 33.3, 34.6 | 13.8, 7.3 |
| stock, this state, boot B, 3 frames | 18.8 to 20.3 | 73 to 78 | 255 | 33.6 to 35.8 | 51.8 to 53.1 |
| **patched, 4 frames** | **13.1 to 14.2** | **45 to 49** | **111 to 118** | **13.0 to 13.4** | 31.0 to 32.7 |
| ours, as on `main` | 14.0 | 48 | 121 | | |

With everything that carries history removed (exposure 1, no bloom, no glow, no trail, clip frame held, `pc 0.3788`):

| Fresh particles | R mean | R q90 | R q99 | RMSE to ours |
| --- | ---: | ---: | ---: | ---: |
| frame 300, stock, 2 frames | 14.1, 15.1 | 52, 58 | 190, 195 | 17.9, 18.3 |
| frame 300, patched, 3 frames | 6.0 to 6.4 | 21 to 24 | 82 to 88 | 3.8 to 7.8 |
| frame 300, ours | 7.1 | 27 | 96 | |
| frame 484, patched, 2 of 3 frames | 5.3, 6.0 | 20, 22 | 67, 76 | 3.2, 5.7 |
| frame 484, ours | 6.4 | 24 | 81 | |

One patched frame 484 read `1.6`, one stock frame `21.2` against `11.8`: a held frame is not perfectly settled at the first shot.

The hull's near half, lower right of the frame (rows `800..1080`, columns `1500..1920`, frame 300): `1.3` percent of pixels past `32` on stock, `13` to `17`
percent patched. The prediction was that R falls toward ours and the near hull appears, with the premise wrong if R stayed near stock: it came out as predicted.
**Confidence 78** that the backdrop's level gap was entirely the wrap, on one patched boot: `mirage_c1` never came up on the second, so **the patched rows are seen on one boot only**. The wrap's existence stays at 95 from the source; what is lowered is the claim that nothing else is in the gap.

What remains is ours: fresh particles read `11` to `18` percent over the patched frames, the residual the wrapped comparison put at `14` to `25`.

The `R` difference between stock boots A and B at one pinned state is unexplained and not pursued. `paused-02.png` agrees with A. It means a boot-to-boot term of that size could exist on the wide build too, unseen on one boot.

### Whole-frame pair scores do not show it

The re-taken menu pairs differ from the old ones by RMSE `1.65` to `17.35`, but a pair's cloud differs per boot and the menu panels cover
most of the frame, so `ours` moves little: Campaign Select `15.97` to `16.64`, cell `20.30` to `20.24`, grid `33.93` to `33.66`, Ship Select `30.91` to `30.09`,
Track Select `33.23` to `34.36`. A level claim needs the pinned state above, not a pair.

## 2048 and Omega

Checked, differs. Neither carries `BackgroundAnimFury`, so neither has this draw.

- **Omega:** shadPS4 and `wipeout_pc` have no RSX path. shadPS4 0.20's `buffer_cache.cpp` has no generated quad index at all (source read, not run). Whether any Omega
  draw reaches a limit of shadPS4's own is **not checkable** live: no memory readout, no ring.
- **2048:** Vita3K has no RSX path either, and GXM has no `QUADS`. No count census exists: **not checkable** without a Vita3K draw log.
