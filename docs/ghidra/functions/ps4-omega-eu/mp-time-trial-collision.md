# Omega: the `MPTimeTrial` race manager clears the same collision byte, and it gates more than craft against craft (2026-10-14, `mp-collision-re`)

**Binary:** `eboot.bin` (Omega Collection, PS4, EU), `x86:LE:64`, image base `0x01000000`. The
executable imports and decompiles here, so the question is checkable. Static only, nothing above
78.

## Answer

`MPTimeTrialRaceManager_Construct` (`0x0127ecd0`, already named at 84 in
[weapons.md](weapons.md)) stores `0` to `collision + 0x8890` (`MOV byte ptr [RAX + 0x8890], 0` at
`0x0127ed0d`, `RAX = *DAT_019ff468`). The collision world's constructor `0x01393610` is the only
other writer and stores `1` (`MOV byte ptr [R13 + 0x8890], 1` at `0x01393b7d`; it also writes
`DAT_019ff468` at `0x01393b30`). The pair dispatch reads the byte before every **dynamic against
dynamic** test: box against box, box against sphere and a third pair kind. Box against track mesh
does not read it. So craft pass through each other in Omega's online Time Trial, as on Pulse, HD
and Pure.

**It differs from HD in width.** HD gates the box-box case alone (spheres only ever meet the
mesh there). Omega's byte also gates the sphere pairs, so a Time Trial would switch off a craft
against a sphere body as well. Time Trial runs with weapons off (Pulse: mode 17 is in the
default-off set), so nothing in this project's port can tell the two apart.

| Claim | Evidence | Conf |
| --- | --- | ---: |
| `0x01393610` is the collision world constructor | the file tag `...\General\Collision\Collision.cpp` has this as its one writer ([collision.md](collision.md)); it is the sole function storing `1` to `+0x8890`; it clears the contact array `+0x880`.. (`0x200 * 0x40` bytes, the count at `+0x8880`) that the pair tests below append to (`if (count > 0x1ff) return`); the same constructor shape as HD's `Collision_Construct` (`0x00034aa0`: one flag byte set to `1`, a contact count, a stored global) | 76 |
| `0x0131ca50` is the per-frame pair pass | it reads the pair count `+0x878` and a `u16` pair list, then switches on each body's shape type (`dword [body + 8]`, `CMP RAX,5 ; JA`): HD's `Collision_ProcessPairs` (`0x00038428`) does the same on `+0x0c`, with the same six-way type range | 74 |
| Six sites read the byte, all in that function | `CMP byte ptr [R13 + 0x8890], 0` at `0x0131e772`, `0x0131ec9b`, `0x0131ecdb`, `0x0131f285`, `0x0131f2ce`, `0x0131f560`; `search_instructions` finds no other reader. Four were read: `0x0131e772` (inline test, both bodies' `+0x40` enable byte checked next, a box-box test over matrices at `+0x10`), `0x0131ec9b` and `0x0131f285`/`0x0131f560` (calls `0x012539b0` and `0x012573e0`). `0x0131ecdb` and `0x0131f2ce` were **not read** | 74 |
| Shape type ids in this enum | from the three blocks read: `0` and `0` is the inline box-box case, `0` and `2` goes to `0x012539b0` (a box against a sphere: it takes half-extents from `+0x60`/`+0x68` and a radius from `+0x58`, checks `+0x40` on both, appends a `0x40`-byte contact), `5` against `0` goes to `0x01253f00` with **no** byte test. Read as `0` = box, `2` = sphere, `5` = mesh; the ids are renumbered against HD's (HD: box `3`, mesh `1`, sphere `5`) | 70 |
| Craft is the box collider | not read in Omega. HD's `Ship_Construct` builds a type-3 box and Omega's lineage keeps the class tree; the box-box inline case is where a craft pair lands | 65 |

Nothing in the argument depends on the exact numbering: the byte is cleared by the Time Trial
manager, is set by the constructor, and guards the dynamic pairs.

## Names

| Address | Name | Conf |
| --- | --- | ---: |
| `0x01393610` | `Collision_Construct` | 76 |
| `0x0131ca50` | `Collision_ProcessPairs` | 74 |
| `0x012539b0` | `Collision_TestObbSphere` | 72 |

[collision.md](collision.md) declined to name `0x01393610` on the tag alone (2026-09-15). The
sole `+0x8890` writer, the contact array and the pair pass above are the code evidence it asked
for.

## Port consequence

None: weapons are off in a Time Trial, so box-sphere pairs have no second body. The open
reading (type `2`, `0x012573e0`, the two unread gates) is worth one pass if a mode that has
sphere bodies and a collision switch ever appears.
