# Function documentation template

Copy this into `docs/ghidra/functions/<binary>/<Name>.md` for every function you
rename. See [ADR-0005](../architecture/adr/0005-ghidra-conventions.md) for why
this is mandatory rather than encouraged.

Filename is the assigned name, or the address if confidence is below 50 and the
function keeps its original label.

---

```markdown
# Ship_UpdateSteering

| | |
| --- | --- |
| **Address** | `0x08831af0` |
| **Binary** | `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse, PSP, UCUS-98712) |
| **Confidence** | 88 |
| **Related** | [`Input_PollPad`](Input_PollPad.md), [`Ship_ApplyRotation`](Ship_ApplyRotation.md) |

## Purpose

One or two sentences. What it does, not how.

## Signature

```c
void Ship_UpdateSteering(Ship *ship, const PadState *pad);
```

| Parameter | Register | Type | Notes |
| --- | --- | --- | --- |
| `ship` | `a0` | `Ship *` | Ship state, layout in [structures/Ship.md](../structures/Ship.md) |
| `pad` | `a1` | `const PadState *` | Not modified |

Returns nothing.

## Behaviour

What it does, in enough detail to reimplement. Pseudocode is fine. Note units,
coordinate conventions, and anything that reads or writes global state.

## Evidence

- Reads the analog stick X axis at `+0x04` of `pad`, which `Input_PollPad`
  writes.
- Writes a float to `ship+0x1c`; `Ship_ApplyRotation` reads that offset and
  treats it as an angle in radians.
- Called once per iteration from `Game_SimulationStep` at `0x0882f110`, before
  physics integration.
- Not runtime-verified, which is why this is capped below 95.

## Cross-platform

| Platform | Address | Notes |
| --- | --- | --- |
| PSP (Pulse) | `0x08831af0` | This entry |
| PS2 (Pulse) | `0x001a4c20` | Same structure, different deadzone constant |
| PSP (Pure) | not located | |

## Open questions

- The deadzone constant `0.15` differs from the PS2 build's `0.12`. Region
  difference, platform difference, or a genuine tuning change between releases?

## History

- 2026-07-26: 65, inferred from call site position.
- 2026-08-02: 88, confirmed by comparing against `Input_PollPad`'s writes.
```

---

## Required fields

Everything above the `## Behaviour` heading, plus `## Evidence`. A page without
evidence is not documentation, it is an assertion.

## Notes on filling it in

**Purpose** describes the function's role in the system. Save the mechanics for
`## Behaviour`.

**Evidence** records what you observed, not what you concluded. Include why the
confidence is not higher: it tells the next person what work would raise it.

**Cross-platform** is worth filling in even when the other entry is "not
located". Knowing that nobody has looked is useful.

**History** exists because scores go down as well as up. A score that only ever
rises is a score nobody is checking.
