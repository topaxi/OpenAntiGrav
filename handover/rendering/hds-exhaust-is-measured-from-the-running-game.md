# HD's exhaust is measured from the running game now: the trail is a 54-sample three-fin tube, the flame breathes with the throttle, and the plume scales rather than blinks

2026-08-24, superseding this row's 2026-08-23 predecessor - that day's survey, flare-model and plume findings are on [trail-ribbon.md](../../docs/rendering/trail-ribbon.md) and [engine-flare.md](../../docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md), and the new session's durable record is [engine-trail.md](../../docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md) (geometry, colour/alpha/u laws, the tuning file, seven names.tsv rows) with `scripts/rpcs3-trail-dump.py` as the reproducer and `oag_fx::exhaust::hd` + `crates/game/tests/hd_engine_flare_ground_truth.rs` as the implementation and its pin. **What is not on the page.** (1) *The route*: RPCS3's GDB stub reads RSX local memory at its guest `0xC...` addresses, so SPU-built vertex buffers are dumpable - it generalises to any HD effect built off-PPU. (2) *Three traps that each cost a round*: a float column that reads as plausible floats can be addresses (`-89.05` at `+0x1214` is the vertex buffer; diffing two pauses exposed it); `stfs ...,0x12c` offset-collides with a HUD sprite's rotation angle, so filter timer-writer candidates by an upstream `lwz rX,0x5f70(craft)`; and `Race::nozzle_of` is **world**-space, so a model-space scale about it flings the flame off-screen - `nozzle_local_of` is the fix. (3) *Three faint-trail bugs*: the naive mirror fins left one fully backfacing from chase cam, where the dumped normals tilt BOTH off-vertical fins toward up (fins 0/60/120 from up, normals `cross(back, fin)`); the flare's speed field is world speed x ~5.4 (`hd::SPEED_FIELD_GAIN` 1.5 over our km/h; two earlier 3.6 readings superseded on the page); the white-to-red vertex ramp was read as *universal* - **refuted 2026-10-10**: it is the Fury skin's ramp; a classic craft's is cyan to light violet (engine-trail.md, "The vertex colour ramp is per skin"), and ours now follows the flag; and the tube must skip the linear-target gamma decode, its own program applying no transfer function. *The method*: bisect the fragment chain with a temporary solid-magenta shader on a captured frame - CPU-side probes said "visible" while the screen said "nothing". Verified on-screen at tick 1050; dead-astern over a bright floor the visible strip is the measured attack ramp and stays subtle - **compared 2026-08-24 at matched views** (numbers on engine-trail.md): over dark-to-mid background the tube matches the original and the halo skirt is not narrower, so neither the trail's constants nor `post::hd_bloom` was touched; over bright sections ours vanishes because our background saturates where the original's does not - that gap is the scene-calibration umbrella item, not the trail. (4) *Still open*: the spikes' per-shape flicker, the Fury afterburner's second blend, the flame's `Speed*time` clock, and what arms the boost timer (observed live: multiples of 1/30 up to 0.70, all below the snap). (5) *The sprite flare draws now* - `Engine_Flare_Rich.gtf` at the tuning file's radius 3 with the `Slow Alpha Noise` walk; its spin, chromatic fringe, distance fade and occlusion stay read-not-drawn (`exhaust::hd::Sprite` lists them), and `wo_engine_flare.pob`/`wo_engine_jetflare.pob` are dead assets - nothing in the executable references them. (7) *A 2026-08-24 follow-up closed four more opens, all on engine-trail.md*: the ring at a race start is **born full, bunched within 0.2 units, every vertex alpha 0** (no fill gate exists; `hd::Tube` now seeds from the first push rather than borrowing the PSP's 0.9 s gate, and a respawn re-bunches as a stated approximation); `+0x11e4` is the trail's sort-key-slot **pointer** (the float-misread trap a third time); the splatted draw-state constant is a **global seconds clock**, refuting "it is the live TrailSpeed patch" and becoming the candidate for the flame's `Speed*time`; the trail material's third sampler is the standard **`lightmap`** slot, null-pathed and inert; and the fin `v` orientation is dump-verified correct. Two `_q` names landed: `Game_PresentLoop` (0x00018848) and `Render_FrameContextPtr` (0x00936fd4). (8) *The scroll chain's last open link is closed, and no code changed.* The question was whether the original applies the scroll phase twice - once in the SPU's vertex `u`, once as the patched `TrailSpeed` - which would make our single application half-rate. **Both halves are measured and the answer is no**: the SPU's `u` carries **no phase term** (the earlier fit's `+ phase` was wrong), and `TrailSpeed` is not patched per frame but bound by **value pointer** to `&block[0x1210]` once at construction - confirmed as the live phase at 94, so engine-flare.md's claim stands. One application total is correct because the vertex has none. **Three method notes.** (a) *The decisive evidence was offline*: the trail's own **vertex** program had never been disassembled. `ps3-microcode.py vp-file` shows `TrailSpeed` at `c[210]` and `ADD o[TC3].z, u, c[210]`, and the FP keeps the *raw* `u` for the colour lookup - **one add per path, not two on one**. (b) *Ghidra cannot find a TOC string reference*; `scripts/ps3-toc.py`'s `scan_toc_loads` can, and found the single `lwz` reading `'TrailSpeed'` in a minute - which turned a planned packet scan into a pointer-identity check. (c) *That packet scan would have found nothing*: nothing copies the phase into the command stream. **The trap that cost a run**: the instance objects sit at `0x40cb....` while the array pointing at them sits at `0x3058....`, and a heap-range guard ending at `0x40000000` rejected all four, returning an empty hit list that read exactly like "the binding is not there". `--params` now reports every field it looked at, misses included. Five `names.tsv` rows landed.

## Open (trimmed 2026-09-04 - three of the four original items are settled, not just narrowed)

`engine-trail.md` kept moving after this file's 2026-08-24 snapshot and was
never folded back; its later sessions already close three of the four items
below. Do not requote this file - `engine-trail.md`'s "Open, in rough order
of visible cost" section (and, for the boost timer, its own dedicated
section) is the current, authoritative state.

- **The flame's `Speed*time` clock is drawn, not just read.** `time` is
  engine parameter slot 0, a global seconds clock every draw-state builder
  writes; the flame's noise tap scrolls at `Speed * time` = `2.0 * seconds`
  and this renderer implements it -
  [`crates/mesh/src/mesh/flame.rs:41`](../../crates/mesh/src/mesh/flame.rs).
  Closed.
- **The Fury afterburner's second blend is measured and deliberately not
  reproduced**, not unexplained: `Afterburner Chase Rate` 0.03, `Afterburner
  Scale` 0.5, but it rides a Fury boost-banking mechanic this simulation
  does not model yet - implementing the blend without the mechanic behind
  it would be inventing behaviour, so
  [`crates/fx/src/exhaust/hd.rs:571`](../../crates/fx/src/exhaust/hd.rs)
  says so at the type it would apply to. Stays undrawn until boost-banking
  itself is built; not a gap in this thread.
- **The spikes' per-shape flicker is measured and still genuinely
  undrawn**, and it is a feature gap, not a wiring step: each of the five
  spike shapes flickers at `RandRange(0.65, 0.85)` per frame
  (`EngineFlare_PlaceShapes`), but the renderer draws the flare as two
  groups (`EF_Main`, `EF_Boost`), not the disc's ten shapes - there is no
  per-spike geometry to flicker yet. See `engine-trail.md`'s
  `EngineFlare_PlaceShapes` bullet.
- **What arms the boost timer - reframed 2026-09-05, not yet closed.**
  `engine-trail.md`'s "What arms the boost timer stays open, minus two
  wrong answers" section (2026-09-05 entry) has the full account. The
  premise in this bullet's earlier wording was wrong: `craft+0x108` is not
  armed by an external writer at all. It is a self-contained rise/fall ramp
  entirely inside the one confirmed function, `0x00090d30` - it rises at a
  rate (`2.0` or `4.0` per second) selected by a byte at `craft+0xfc`, falls
  at a flat `2 * dt` otherwise, and the whole block only runs when the
  craft's race mode reads `10` (plausibly Zone Battle, where boost is
  banked) - confirmed from raw disassembly, not the decompiler's pseudocode
  for this function, which is corrupted (`halt_baddata()`, eleven
  unreachable-block warnings). A clean negative narrows where to look next:
  batch-decompiling all ~20 sibling per-tick calls from the craft's own
  master `Update` (`FUN_0009e3d0`, `0x00090d30`'s only caller) found none of
  them touch `+0x108` either. So an external "arm" event, if one exists,
  is not in the per-tick chain - it would be pad-contact, barrel-roll input
  or race-start code reached some other way. A candidate for what instead
  sets the `+0xfc` rate-selector byte (confidence 35, not a name, a
  hypothesis) - a class-ID compare (`sub-object+0x58 == 0xb`) gating a
  transform load from what may be a pad/track object - is on the page but
  not verified live. **Re-run 2026-09-15 on the reimported program
  (both `0x00090d30` and `0x0009e3d0` had been holed by an undecoded `lvlx`
  when the negative was first drawn) and re-confirmed at 85**: all 50
  callees of `0x0009e3d0` decompile clean and only `0x00090d30` touches
  `+0x108`; a whole-image sweep of the 1,006 stores with a `0x108(`
  displacement leaves 236 off the stack, the 20 in craft-sized code all
  classified to other objects (the flare's own `+0x108`, a RaceManager
  slot record, the pad object, weapons, HUD), the 14 `stfs` in the rest
  opened one by one, and the dozen `+0x108` sites inside the craft-method
  address range all integer reference counts on a shared resource - none
  takes a craft. What the clean decompile adds:
  the rising branch is entered only past the `+0x58 == 0xb` check and the
  `+0x5f70` transform read, so the pad-contact reading is now what the
  decompiler shows too, still never observed live; and `+0xfc` has a
  third clearing writer (`0x00091a68`, in the formerly hidden stretch) and
  still no setter. Full account: `engine-trail.md`, the 2026-09-15 entry
  under "What arms the boost timer".

## Next Steps

- Confirm live whether `craft+0xfc`'s rate-selector byte is set by driving
  over a speed pad (or by a barrel roll / the start boost), and whether the
  `sub-object+0x58 == 0xb` class-ID read this session found is really a pad
  contact. `engine-trail.md`'s own record is eight sessions of breakpoint
  tracing on adjacent ground (the flare's render gate) before it converged -
  this needs a live RPCS3 trace bracketing a boost pickup/barrel
  roll/start sequence with a breakpoint on `craft+0xfc`'s write sites -
  static reading (`search_instructions` for `stb`/`stbu` on `0xfc(`,
  whole-binary) found exactly two writers of that exact addressing form -
  three after the 2026-09-15 reimport, `0x00091a68` having sat in a hole -
  **all inside `0x00090d30` itself, and all clearing it to `0`**
  (`stbu r0,0xfc(r28)` at `0x0009113c`/`0x00091a68`/`0x00091b30`, `r0`
  loaded from `li r0,0x0`). Nothing anywhere sets it to `1` this way, so whatever arms
  the rise rate either writes a whole word/struct that happens to overlap
  this byte, or reaches it through indexed addressing (`stbx`) this sweep
  would not catch - read for that live rather than assuming a breakpoint on
  the literal `+0xfc(r31)` address will ever fire. A blind `stw`/`stfs
  0x108(` sweep across the whole binary is
  **not** useful alone - confirmed again this session, 48+ unrelated hits,
  mostly stack-frame saves at the same offset; the 2026-09-15 re-run did
  it properly (1,006 stores, 770 on `r1`, the rest classified by whether
  the function touches a craft-sized offset) and it need not be repeated -
  the next evidence is live, not static.
- Separately, and explicitly not the next step here: implementing the five
  spike shapes (their own geometry, not just the flicker constant) is a
  renderer feature of its own size, tracked by this bullet rather than
  scoped into a quick fix.
