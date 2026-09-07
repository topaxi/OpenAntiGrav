# Craft-to-craft collision is implemented; the stun is still not armed

2026-08-11/19. The full read is [contact-response.md](../docs/ghidra/functions/psp-pulse-usa/contact-response.md); this row keeps what is not there. **The finding to carry: a craft bounces off another craft with `e = 0.1` and off the track with `e = 0.4`, in the same build.** `Body_ResolveContactPair` (`0x0884ef30`, 80, ported as `oag_physics::pair`) hardcodes `-1.1` where the one-body path reads the per-body `0.4` the ship constructor writes; it also gates on `vn - 0.5 <= 0`, applies **no friction**, and corrects position by a mass-independent quarter of the overlap each way. The PS2 carries the same `-1.1`, so the PSP disagrees with *itself* across its two paths - a third instance of the `0.4`-versus-`0.1` trap that page warns about. ~~**The detection is ours and there is nothing to recover**: `0x08815ccc`, the box-against-box narrowphase a pair of craft dispatches to, is **`jr ra; nop`** (90), so no pair of craft can produce a contact through `Collision_StepNarrowphase` - which *positively excludes* it from the pending-impulse hunt.~~ **Corrected 2026-08-25: `0x08815ccc` was mislabelled.** It is the *mesh*-against-mesh dispatch (a real stub, still confidence 90, just for the wrong pair). Craft, as box colliders, dispatch to `Collision_BoxAgainstBox` instead, which is **not** a stub - a full, decompiled fifteen-axis oriented-box SAT that writes a real contact on overlap. Whether it ever fires for two craft depends on two unconfirmed gates (`world+0x5464`, each collider's own `+0x68` byte); the narrowphase is reopened as a candidate for the pending-impulse hunt, not excluded. Full account: [collision.md](../docs/ghidra/functions/psp-pulse-usa/collision.md#collision_stepnarrowphase-0x088159c0-builds-the-pair-list), [contact-response.md](../docs/ghidra/functions/psp-pulse-usa/contact-response.md#0x08815ccc-is-a-stub-but-it-is-not-what-a-pair-of-craft-dispatches-to). `pair::overlap` tests oriented box against oriented box over `<Misc width height length>`. A sphere of half the hull diagonal was far too big (craft shoved each other while visibly apart); **the box was still too big, because `<Misc ...>` bounds the model rather than tracing it** - a real craft is `5.5 x 3.5 x 13` and half its length, 6.5, lands beside the 6.45 bounding radius `oag-view` reports, so a hull that tapers to the nose collides along its whole length at its widest point. `pair::HULL_SCALE` is the knob, ours with nothing behind it, and only play sets it. **`data/` can be symlinked into a worktree** (`ln -s <checkout>/data/images data/images`, same for `extracted`); symlink the *subdirectories*, since `data/` itself holds a tracked `README.md`. It turns every `#[ignore]`d ground truth from unrunnable into one command. **Both tail-call candidates for the pending-impulse writer are ruled out** (the old "lands inside `Ship_UpdateCraft`" note was a research error): `0x499f8` is `Body_Translate`, `0x49c60` a real gap after `Body_RecordContact` - boundary created (`FUN_0884dc60`, 95), a near-twin marking its ring record `1` not `0`, unnamed at 50. Neither posts `entity->0x4c + 0x110`. **A live write breakpoint then caught two writers**, both in weapon code and neither statically xref'd, so a function-pointer dispatch nobody has located calls both: `Weapon_PostBlastImpulse_q` (`0x0886794c`, 68) and an unnamed `FUN_08868ea4` (under 50; its loop bound is read through a `$gp` access this project cannot trust). **Two of the three are read and ported**, six tests each. `Ship_ApplyCollisionImpulse`: its caller passes the ship's own world position, so the lever arm is structurally zero and only the linear half needs porting (`wall::apply_pending_impulse`, called every tick from `crate::step`). `Weapon_PostBlastImpulse_q` (`wall::post_blast_impulse`): a full instruction-level re-read corrected two claims and found three missed writes, and its `target` identity was settled **by pointer equality at a live breakpoint** - enumerate the eight craft bodies off an `Ship_ApplyCollisionImpulse` hit, then catch a real AI-on-AI blast. **What that opened, unresolved**: `body+0x50` as position is in direct tension with rigid-body.md's confidence-88 reading of `body+0x40..0x70` as inverse-inertia storage, backed by `Body_Integrate` consuming that block every sub-step. Flagged in both docs, reconciled in neither; settling it means tracing what `Body_Integrate`'s `vtfm3.t` addresses. Unchased: `body+0x30` (rigid-body.md calls it "position") reads small, `y`-dominant and unit-scale on every craft. **The caller chain is two hops further.** `FUN_08867b50` sweeps every craft as a candidate (box then sphere against a per-weapon radius) and supplies `targetIndex`; its caller `FUN_08867370` decrements `craft->0x48 -= dt` and fires the sweep when it expires - a fuse on the craft fits a proximity mine. `Weapons_DispatchFire`'s `world+0x44` handler `FUN_08863a20` reads as that Mine, spawning with its direction **negated** from the craft's forward row, and setting the new entity's `+0x40` to the **owning craft's index** rather than a weapon-type enum - so the stats table looks keyed by craft, from one spawn site. **It writes `craft->0x48` nowhere** - a clean negative. **A fuse-arming candidate, unconfirmed.** `FUN_0885bf84` computes `entity->0x48 = |vector| * 3.6 + weaponTypeRecord->0xbc` (a `=`, not a `-=`, off a *different* per-weapon table), found by searching every `swc1` to a `+0x48` off a non-stack register. Its caller allocates from a differently-shaped pool (cap 16 not 32, `+0x64` indexing not `+0x44`), yet `FUN_08867370`'s own decrement uses that same `+0x64` indexing and lands on the exact field it arms; the loop bounds still differ (`+0xa4` against `+0x164`). **A correction the same live session forced**: this row previously claimed `FUN_08867370` "loops every craft"; a breakpoint measured its bound `craftArray->0x164` at **`1`** during a full eight-craft race. That says `+0x164` is not a craft count, not that the base is wrong. (It also rules out "mines currently armed" - the hit was before anyone had fired, so that would read `0`.) **And a trap**: a controlled write-watch (positive control: 30,413 hits in 90 s) saw the field change with **zero writes logged** - so in PPSSPP a write-watch proves presence, never absence ([ppsspp-debugger.md](../docs/reverse-engineering/ppsspp-debugger.md#a-controlled-watch-can-still-miss-a-write---the-pool-slot-outlives-the-log-does-not)). **Net state**: `pending_impulse` is set by nothing in this crate, so `apply_pending_impulse` is a correct, tested, fully inert no-op.

**Corrected 2026-09-04: the "remaining steps" line below mislabelled its own
open question, and `contact-response.md` had already gone further than this
row credited it for.** `FUN_0885bf84` was never a pending-impulse candidate -
it writes `entity+0x48` (a fuse) and `entity+0x3c |= 8` (a flag), never through
the `*(entity+0x4c)` indirection `Ship_ApplyCollisionImpulse` and
`Weapon_PostBlastImpulse_q` both use to reach `T+0x110`. The real open question
is narrower: does `FUN_0885bf84` arm the fuse `FUN_08867370` decrements at
`craft->0x48`? `contact-response.md`'s own 2026-08-19 session read that deeper
than this row shows: a full disassembly of `FUN_08867370`'s decrement
(`lw s4, 0x64(s3)`) matches `FUN_0886a920`'s allocation indexing
(`subsystem + cursor*4 + 0x64`) exactly, and both land on the entity
`FUN_0885bf84` arms at `+0x48` - upgrading confidence from "pool shape doesn't
match" to "slot arithmetic and field both match, cap and cursor offset still
don't." A follow-up *controlled* live watchpoint (verified healthy via a
30,413-hit positive control over 90 s) then caught `FUN_0885bf84`'s own arm
write once, logged correctly - but the same slot's `+0x3c`/`+0x48` visibly
changed again before the window closed with **zero** further hits logged,
meaning whatever recycles a freed slot does not go through the path the
watchpoint's log covers. That is a new, general PPSSPP-debugger trap
(documented on `ppsspp-debugger.md`), not a dead end specific to this
function. Full account, including the three concrete next steps it leaves
open, is [contact-response.md's own "fuse-arming candidate" section]
(../docs/ghidra/functions/psp-pulse-usa/contact-response.md#a-fuse-arming-candidate-found-by-searching-for-the-write-directly---unconfirmed).
Separately, `+0x164` is now read precisely - see
[mine.md](../docs/ghidra/functions/psp-pulse-usa/mine.md#mine_spawnexplosion-plays-wo_mine_explo):
a swap-with-last live count over the Mine's `+0x64` pool, decremented once per
removal, not a craft count and not the same shape as `+0xa4`'s write-cursor.
And `FUN_08868ea4`'s two callers are read, not just found -
`contact-response.md`'s 2026-08-25 pass names `FUN_08868a10` and
`FUN_088690fc`, both shape-confidence 60, semantics still under 50 - the
prerequisite for porting it has landed even though the port itself has not.

**Settled 2026-09-07: the load-bearing live read landed, and `pair.rs` changed
as a result.** PPSSPP v1.20.4, a full eight-craft SINGLE RACE grid on
`pulse-psp-usa.chd`. `world+0x5464` read `1` during the race; all eight craft
colliders' own `+0x68` byte read `1`. Two craft were teleported onto the same
point and a breakpoint on `Collision_BoxAgainstBox`'s own entry fired ten
times running with both proxies resolving to the two placed craft's colliders
(owner ids matching directly), and `world+0x2450`'s contact counter rose by
one across the first of those hits - a contact genuinely written, not just a
dispatch. Full recipe and readings:
[collision.md](../docs/ghidra/functions/psp-pulse-usa/collision.md#collision_stepnarrowphase-0x088159c0-builds-the-pair-list).
`Collision_BoxAgainstBox` is now decompiled in full and compared against
`pair::overlap` in the detail `contact-response.md` asked for; three
differences were found and ported into `pair.rs` (see its own updated doc
comment on [`overlap`](../crates/physics/src/pair.rs)): only the six face axes
ever choose the contact normal (the nine edge-edge cross products are
reject-only), each hull's own "up" axis needs to beat *half* the reigning best
depth to win where right/forward only need to beat it outright, and the
contact point is the plain midpoint of the two bodies' positions rather than a
support point on the chosen axis. Friction and the one-contact-per-pair shape
were confirmed unchanged. All three differences, including the "half the best
depth" bias, are confirmed at instruction level against the disassembly (a
real `mul.s` on a real `0.5f` register, not a decompiler artifact) - the VFPU
inlining trap `docs/ghidra/workflow.md` warns about applies directly to a
function this size, and it was checked rather than assumed. Three new
regression tests in `pair/tests.rs` pin each difference against the pre-fix
algorithm, found by sweeping poses rather than hand-picked. Confidence on
`Collision_BoxAgainstBox` raised **75 -> 92** in `collision.md`'s table and
`names.tsv` - a live measurement on the exact predicted chain, not an
inference from static reading. **The correction has one traced consequence
outside this crate**: it moves a full-grid race's trajectories enough to fail
`crates/game/tests/opponent_weapons_ground_truth.rs`'s
`a_field_racing_with_real_pads_does_not_mine_itself_to_death` (a mine-dodging
guard), bisected to this change alone - see
`handover/the-corrected-craft-pair-narrowphase-exposes-a-mine-dodge-gap.md`
for the numbers. That is `crates/ai`'s territory, not this one's, and is left
for its owner rather than patched here.

## Open

- `pending_impulse` is set by nothing in this crate, so `apply_pending_impulse` is a correct but fully inert no-op.
- `body+0x50` as position is in direct tension with rigid-body.md's confidence-88 reading of `body+0x40..0x70` as inverse-inertia storage; flagged in both docs, reconciled in neither.
- The stun is not armed: craft-craft elasticity (`e = 0.1`) disagrees with the ship's own track elasticity (`e = 0.4`), no friction is applied, and no severity logic is modelled.
- `FUN_0885bf84` is still an unconfirmed fuse-arming candidate for `craft->0x48` - not for `pending_impulse`, which it never touches. Slot arithmetic and the written field match `FUN_08867370`'s reader exactly; the cap (`16` vs `32`) and cursor offset (`+0xa4` vs `+0x164`) still don't. Re-checked this pass: no static `jal` anywhere in the image targets `0x0885bf84` (`search_instructions`, 525,283 instructions scanned, zero hits), confirming the indirect-dispatch shape already on record rather than turning up a new lead - settling it still needs one of the three routes below.

## Next Steps

- Settle whether `FUN_0885bf84` arms `craft->0x48`'s fuse, by one of the three routes `contact-response.md` already narrowed it to: read what `+0xbc`'s per-type table actually enumerates, find `FUN_0886b458`'s own caller (unresolved by static `jal` search, same indirect-dispatch shape as `FUN_08867370` itself), or catch a slot at the *moment* it is armed and read its `+0x48` on every tick thereafter rather than trusting a watchpoint to report absence.
- Port `FUN_08868ea4` now that both its callers (`FUN_08868a10`, `FUN_088690fc`) are read, or wire `post_blast_impulse` to a real weapon.
