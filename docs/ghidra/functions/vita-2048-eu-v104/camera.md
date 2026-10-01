# 2048's chase-camera rig carries the same 0.75 craft scale (0.7 in Detonator)

2048 is HD's codebase retargeted (see [README.md](README.md)), and its camera rig
shows it: the same scale, set by the same mode test, read by the same shape of
rig. Read headless on 2026-10-01 against `eboot.elf` (v1.04), decompiler output;
[`ps3-hdfury-eu/camera.md`](../ps3-hdfury-eu/camera.md) has the instruction-level
reading of the PS3 original. **Static only**; nothing here ran. Names went into
[`names.tsv`](names.tsv) in the same change.

| Address | Name | Confidence |
| --- | --- | --- |
| `0x811bd89a` | `Ship_UpdateCameraRigs` | 78 |
| `0x8151fdd0` | `g_craft_scale` (data) | 82 |

## The write

The two craft constructors (`FUN_811b8806` and `FUN_811c82dc`, the ones that
build `"%s external close tripod"`/`"%s external far tripod"` through
`0x81493e18`/`0x81493e34`) both do, in the decompiler's words:

```c
DAT_8151fdf4 = 0x3f666666;          /* 0.9 */
DAT_8151fdd0 = 0.7;
if (DAT_8153fd24 != 0xe) {          /* the mode id; 14 is Detonator */
    DAT_8151fdd0 = 0.75;
    DAT_8151fdf4 = 0x3f800000;      /* 1.0 */
}
DAT_8151fdf8 = DAT_8151fdf4;
```

(`811b8eb6`/`811b8ec2` in the first, `811c8382`/`811c8360` in the second), which is
HD's constructor with the object's `+0x44` and `+0xa8..+0xb0` turned into globals:
`0.75`/`1.0`, or `0.7`/`0.9` in Detonator.

## The consumer

`FUN_811bd89a` reads `DAT_8151fdd0` four times (`0x811bdcda`, `0x811bdcf0`,
`0x811be0fc`, `0x811be114`) and reads the handling params block at
`*(iVar12+0x80)`: `+0x5c`, `+0x58`, `+0x50` (and, a few lines on, `+0x4c`, `+0x48`,
`+0x40`, the far block), the same layout as HD's. Each pair is

```c
auVar19 = FloatVectorSub(eye,  bodyPos);                  /* bodyPos = *(*(ship+0x5f84)+0x38) */
auVar19 = FloatVectorMultiplyAccumulate(auVar19, DAT_8151fdd0 splat, ...);
auVar19 = FloatVectorAdd(auVar19, ...);                   /* bodyPos added back */
```

for the eye and again for the look-at, which is `ship + (p - ship) * scale`.

Confidence **78** for the rig's name and the scale's use: the global's two
writers and four reads are certain, the params layout agrees with HD's, but the
NEON-flavoured decompile was read for its shape and the far block less closely
than the close one. The data row is **82**: its two writes are read directly.

Not read: the other readers of `DAT_8151fdd0` (`FUN_811a76b6`, `FUN_811d0f66`,
`FUN_811a9b74`, `FUN_812c64b4`, `FUN_812caf1c`, and three sites in no function), and
`head_tilt`, which the rig is not seen to read.
