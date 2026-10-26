---
status: measure
titles: [pulse, pure]
updated: 2026-10-26
summary: "No PPSSPP frame of the rotated 06_Track pods; PS2 PSP-format batches disagree with faces; Pure v3 files unread"
---

# `.vex` normals carry the node rotation: what is left

Landed 2026-10-26 (`pulse-vex-normals`): `oag_mesh::mesh` carries a vertex normal through its
node's rotation, as the original's GE world matrix does. Evidence and the census are in
`docs/formats/vex.md`, "A vertex normal is in its node's space".

## Open

- **No emulator frame of the rotated pods.** The `06_Track` pod structures (nodes 725 and 775,
  540 shine vertices each) differ by 66,000 and 72,000 pixels before and after, and the new
  frame follows the faces, but nothing compared it with PPSSPP's. A matched frame at
  `--camera-pose=-306,97,87,-0.688,-0.229,-0.688,0,1,0` would close it.
- **PS2's PSP-format batches (697,765 vertices, types `0x121` to `0x13f`) agree with their faces on
  about 52% of pairs under an identity node**, against 99.9% for its VIF batches and for every PSP
  batch, so the face test cannot judge their placement. Either their normals are not unit face
  normals or their winding differs from the PSP's; nothing reads them differently from the PSP's
  today.
- **Pure's 15 version-3 `.vex` files break the batch header** (`pure-status.md`) and are not in the
  census; their rotated nodes are unmeasured.
- **17 mirrored nodes** (negative determinant, `10_Track` zone variants, `04_Track`) take the GE's
  plain product, which reflects the normal and agrees with the faces once the face is read with
  the determinant's sign. Whether the original's culling treats their reversed winding the same
  way is not read.

## Next Steps

1. Capture the pod frame on PPSSPP at the camera above and difference it against ours.
2. Read why the PS2's PSP-format batches disagree with their faces (a strip parity or a normal
   layout in `oag_vex::vex`'s PSP path on a PS2 file).
