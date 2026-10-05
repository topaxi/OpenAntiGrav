# Omega's material state word and its draw-order key (2026-10-05, `transparent-floors`)

Static read of `/omega/eboot-ps4-omega-eu.bin`, corroborated by a disc census. The
state word is Wipeout HD's own (see
[`material-state.md`](../ps3-hdfury-eu/material-state.md) for HD's reader).

## `0x015fa610`, the draw-order key

`Scene_BuildMaterialSortKeys` reads the `u16` at material `+0x22` and writes a one-byte key per draw:

```text
state = *(u16*)(material + 0x22)
if (state & 2) == 0:  key = 8, and if (state & 1): key = 0x18 (no subtraction path for opaque)
else:                 key = 0x10
blended or alpha-tested: key -= (state >> 9) & 7
```

So bit 1 (an alpha test) and bit 0 (a blend) pick the later buckets exactly as HD's bits do, and
bits 9 to 11 are a sort priority that pulls a draw earlier within its bucket (`waterfall_new`
carries `0xe29`). Confidence 75: the instruction reading is direct, the meaning of the three
buckets is inferred from HD's state word, which the census below agrees with.

## The census that says the word is HD's

Across 32,880 material headers of the nine archives, the `u16` at `+0x22` takes 22 values, the
same set HD's `+0x10` holds (`0x7c` opaque 30,054, `0x39` 1,320, `0x3d`, `0x3e`, `0x29`, `0x79`,
`0x7d`, `0x69`, `0xbd`, `0x6c`, `0x2d`) plus the priority bits above. Same-named materials agree
with HD's mode: `fence_alpha`, `nr_crowd_bustle`, `tree_wind_new` read mode 2; `glass_texture`,
`etched_glass_tech`, `clouds`, `tunnel_fx_glass`, `dc_lightcone` read mode 1. The Vita 2048
header holds it at `+0x12` (22,657 headers, 17 values, same set).

## What is not there: the factor pair

HD authors a source and destination factor beside the state word. Omega's header holds none:
`+0x24` to `+0x2f` are zero on every one of the 32,880 materials, and no other small-valued
word varies. HD's factors are a function of the material name in 181 of 184 names, and the Omega
state value does not predict them (`0x39` holds `ONE/ONE_MINUS_SRC_ALPHA`, `SRC_ALPHA/ONE_MINUS_SRC_ALPHA`
and `SRC_ALPHA/ONE` names alike). The blend packers (`BlendState_SetColor` `0x012091a0` and kin)
have 46 callers; the only variable-argument ones are `FUN_017a39e0` and `FUN_017a1b30` (effects),
and no caller on the material pass programs a per-material pair. **Where Omega gets a
material's equation is not found**; the renderer uses alpha-over, chosen, not measured.

| address | name | conf |
| --- | --- | --- |
| `0x015fa610` | `Scene_BuildMaterialSortKeys` | 75 |
