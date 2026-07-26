# Recovered structures

One page per structure, with the evidence for each field's offset and type.

**Currently empty.** Milestone M2.

## Conventions

PascalCase type names, `snake_case` fields. Unknown fields are named for their
offset and size, so they are obviously unknown:

```c
typedef struct Ship {
    Vec3  position;        // +0x00
    Vec3  velocity;        // +0x0c
    float steering_angle;  // +0x1c
    u32   unk_0x20;        // +0x20, unknown
    u8    unk_0x24[12];    // +0x24, unknown
} Ship;
```

`unk_0x20` beats `field20` because it states the offset, and beats `padding`
because `padding` is a claim, and usually a wrong one.

Full conventions in [naming-conventions.md](../naming-conventions.md).

## Page contents

Each page should carry:

- The structure's size, and how that size was established
- A field table: offset, type, name, and the evidence for each
- Which functions read and write each field
- Confidence, per the [rubric](../../reverse-engineering/confidence-rubric.md)
- The corresponding layout on the other platform, if known

Field-level evidence matters more here than for functions. A structure is
usually recovered one field at a time from scattered access sites, and the
confidence in `+0x00` is often very different from the confidence in `+0x3c`.

## Index

_(no structures documented yet)_
