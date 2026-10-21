# A barrel roll's payout and a fired Turbo arm the same flare and play the same cue

**2026-10-21, `pulse-roll-turbo-boost` lane.** Binary `BOOT.BIN` of `pulse-psp-usa.chd`
(Ghidra `/pulse/BOOT-psp-pulse-usa.BIN`). Static read, then **watched live** on PPSSPP 1.20.4
from a Time Trial grid state.

| Address | Name | Confidence |
| --- | --- | ---: |
| `0x0890514c` | `ExhaustFlare_OnTurbo` | 90 |

## The function

`0x0890514c(flare)` has exactly two callers (`get_xrefs_to`): `Turbo_Fire` (`0x088614c4`, call at
`0x08861520`) and `Ship_UpdateSideshiftInput_q` (`0x08846a54`, call at `0x08846b60`).

```c
void ExhaustFlare_OnTurbo(flare) {
    flare->boost_timer = 0.8f;                         // +0xb8 = 0x3f4ccccd, a literal
    racer = flare->racer;                              // +0xc0
    if (racer) {
        if (racer->controller_class == 0 && ...)       // +0x368, the human
            FUN_0883e9b0(racer, bank, "TURBO", 0x400, 0);   // dry, 0x08905 1d4
        else
            Sound_Play(1.0, racer->emitter, bank, 0, "TURBO", 0);  // positional, 0x089051fc
    }
}
```

The name pointer is `*(0x08a84c68) = 0x08a84c60`, the bytes `TURBO\0`. It differs from
`ExhaustFlare_OnPerfectStart` (`0x08904fd4`, [perfect-start.md](perfect-start.md)) only in that
it stores the literal `0.8` where that one copies `flare+0xb4`.

## The roll's call site

In `Ship_UpdateSideshiftInput_q`, with the contact bit (`craft+0x1c0 & 1`) set, the running-turbo
bit `entity+0x860 & 0x200` clear and a roll armed (`+0x860 & 0x180`), `entity+0x87c` (the roll
phase) beyond `+-0.5` pays out at `0x08846b4c`: `entity+0x898 = g_roll_turbotime`, and when the
craft has a flare (`entity+0x78`) `jal 0x0890514c`. It then counts the roll (`settings+0x238`)
and clears the two arm bits. Nothing here is conditional on the human: an AI craft's roll calls
the same function, and its cue is the positional one.

## Live, 2026-10-21 (`scripts/psp-roll-turbo-flare.py`)

Time Trial, Talons, Venom; breakpoint at the sound call `0x089051d4` (`a0` = craft, `a1` = bank
handle, `a2` = name, `ra` saved at `0(sp)`).

| Case | Caller (`ra`) | Flare `+0xb8` | Flare `+0xb4` | Bank handle | Cue | `craft+0x898` |
| --- | --- | ---: | ---: | ---: | --- | ---: |
| Roll payout (taps `R L R`, contact forced on tick 120) | `0x08846b68` | 0.8000 | 0.8000 | 7 | `TURBO` | 0.5000 |
| Fired Turbo (fire word `0x400`, `Weapons_DispatchFire`) | `0x08861528` | 0.8000 | 0.8000 | 7 | `TURBO` | 0.0000 |

`+0x368` read 0 (the human) in both. The contact bit and the d-pad taps were scripted, as
`psp-roll-gate.py` does; this is a trigger check, not a physics measurement. Bit `0x400` of the
fire word is the Turbo's request (`Weapon_RequestFire` case 4); `0x200` is the running state and
fires nothing.

Free-running frames a fraction of a second after each event (frame 1 of 5) show the boost plume
and the HUD's lap and time read-outs drawn as smeared copies, the pattern
[hud-boost-smear.md](hud-boost-smear.md) reads off the flare's timer, for the roll and for the
Turbo alike.

## Ours

`oag_raceplay::roll_payout` raises `Cue::Turbo` and arms `Exhaust::boost` on every tick a craft's
`roll_payout_timer` rises (physics arms it on the landing after a completed roll, after the
per-tick decay, so a rise is the arming tick, and a second landing inside a payout re-fires as the
original calls the function again). The Turbo's branch in `weapons.rs` already did both
(2026-10-06), and now has a test that the flare and the HUD's `boost_phase` are armed.

## Cross-title

- **Pure, HD, Omega, 2048:** *checked, applies, not measured.* Ours raises the same cue and flare
  wherever a handling file authors `roll_turbotime` and a craft completes a roll; no other title's
  executable was read for this call. HD's `TURBO` cue resolves (15 waveforms, per
  perfect-start.md).
