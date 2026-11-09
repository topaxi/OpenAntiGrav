---
status: measure
titles: [pure]
updated: 2026-11-09
summary: "Pure back-face cull per batch as Pulse: 4 pairs moved, vineta worse 0.0699 to 0.0724, batch state unread on Pure"
categories: [rendering]
size: S (25)
difficulty: easy (25)
---

# Pure: does a circuit cull its batches' back faces?

Pulse's does ([batch-cull.md](../../docs/rendering/batch-cull.md)); Pure's rule is `UNREAD`.

## Open

- With the rule on, the four Pure race pairs move 10, 34, 167 and 401 pixels; blue-ridge, chenghou and modesto
  are within 0.0001 rmse, vineta worsens 0.0699 to 0.0724 around a yellow `eur` sign. Not traced beyond that.
- `Mesh_InitBatch` (the cull state setter) is unread on Pure.

## Next Steps

1. Read the cull enable in Pure's batch state setter and check `pass_mask & 0x20` drives it.
2. Look for a back-to-back board in a Pure circuit, and read the vineta sign at the pair's pose.
3. If it holds, give Pure `batch_back_face_cull` as inherited or measured and flip the Pure row of
   `a_circuit_culls_per_batch_where_its_titles_rule_says_so`.
