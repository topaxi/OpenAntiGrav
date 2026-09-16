# The archives are mounted through Sony's FIOS2, not read by this binary's own code

2026-09-16. `eboot.bin` (WipEout: Omega Collection, PS4, `CUSA05670`, EU),
`x86:LE:64:default`, image base `0x01000000`. Written while chasing the PSARC
"block data location" open question (see
[`docs/formats/psarc.md`](../../../formats/psarc.md)'s "Block data location"
section) into the executable - the question this page actually answers is
narrower than the one that sent it here: not *how* an entry's bytes are
located (this binary never does that itself), but *whether* there is a
game-owned read path to compare `oag_formats::psarc` against at all. There
is not.

**The names here are applied**, from [names.tsv](names.tsv).

## `PsarcArchive_Mount` - `0x013a26b0`

**Confidence: 85.**

`search_strings("data%02d.psarc")` returns exactly one match, at
`0x01817817`, immediately followed at `0x01817826` by `"data00.psarc"` - the
`sprintf` format string and its own rendered example sitting back to back the
way a compiler places a literal and the debug build's sanity-check copy of
it. `get_xrefs_to` on `0x01817817` names exactly one caller,
[`PsarcArchive_WaitAndMountAll`](#psarcarchive_waitandmountall---0x013a27d0),
which calls through to this function once per archive.

The decompilation is short and unambiguous - three calls, all to
NID-resolved Sony imports, no project-authored logic in between:

```c
void PsarcArchive_Mount(undefined8 filename, undefined8 *out_handle_and_len, undefined4 *out_handle)
{
    char mount_params[1032];
    FUN_013a1a40(mount_params, filename);           // builds a sceFiosOpenParams-shaped struct
    long size = sceFiosArchiveGetMountBufferSizeSync(0, mount_params, 0);
    if (size >= 0) {
        out_handle_and_len[1] = size;
        void *buf = FUN_0139e070(size);              // operator new[]
        out_handle_and_len[0] = buf;
        sceFiosArchiveMountSync(0, out_handle, mount_params, &DAT_01a17ee0, buf, size, 0);
        sceFiosIsValidHandle(*out_handle);
    }
}
```

`sceFiosArchiveGetMountBufferSizeSync`/`sceFiosArchiveMountSync` are both
PS4 SDK FIOS2 exports, resolved by name at import time (NIDAnalyzer), not
recovered here - the same "recovered library symbol keeps its real name"
rule `names.tsv`'s own header states. **`sceFiosArchiveMountSync` is the load
of a PSARC archive as Sony's own SDK defines it**: FIOS2's public
documentation names `.psarc` (built by the SDK's own `psarc` tool) as the
one archive format `sceFiosArchiveMount*` accepts. This corroborates
`docs/formats/psarc.md`'s framing from the other direction - the container
is "publicly documented rather than recovered here" because it is literally
a first-party Sony format with a first-party mounter, not a Wipeout-specific
scheme this project is the first to describe.

Not full marks because the two buffer arguments passed to
`sceFiosArchiveMountSync` (`&DAT_01a17ee0`, and the pair this function
allocates) are not traced further - what FIOS2 does with them, and where its
own block-table arithmetic lives, is inside `libSceFios2.prx`, a signed Sony
system module this project holds a copy of
(`data/extracted/ps4/omega-eu/uroot/sce_module/libSceFios2.prx`) but has not
opened in Ghidra. See "Not read", below, for why.

## `PsarcArchive_WaitAndMountAll` - `0x013a27d0`

**Confidence: 80.**

Reached by `get_xrefs_to` on `PsarcArchive_Mount`'s own format string, above.
`get_xrefs_to` on this function's own entry point finds no direct `CALL`
at all - only one `[INDIRECTION]` and two `[DATA]` references, the shape a
function reached through a function-pointer table (a thread entry point,
here) rather than a straight-line caller leaves.

The body is a polling loop, not a one-shot mount: it opens a PlayGo chunk
handle (`scePlayGoOpen`/`scePlayGoGetChunkId`), then loops
`scePlayGoGetLocus` against all five chunks, calling `PsarcArchive_Mount` for
chunk *n* only once that chunk's locus byte reads bit 0 set (`(bVar5 & 1) ==
1`, `puVar9[-0x18] == '\0'` - a "not yet mounted" guard so a chunk is
mounted exactly once), and sleeping one second
(`sceKernelUsleep(1000000)`) between passes until all five report installed
(`iVar7 < 5`). This is a PS4 SDK PlayGo streaming-install monitor: on a real
console, `scePlayGoGetLocus` distinguishes "on the Blu-ray disc" from
"copied to the internal HDD/SSD" per chunk, and a title mounts a chunk only
once the copy finishes rather than blocking boot on the whole disc. Not
full marks because the two `puVar9[-0x18]`/`DAT_01a7b1b4` locus-byte
bit tests are read from the decompilation's own shape rather than against
a documented `ScePlayGoLocus` bit layout - plausible, not confirmed against
an SDK header.

**Consequence for the block-data-location thread**: this PlayGo mechanism
is disc-streaming/HDD-copy state, not asset-packing state - it explains why
a *live* PS4 console might read a not-yet-copied chunk as unavailable, and
is one candidate mechanism for why a static, offline-extracted `.pkg` dump
could legitimately carry content that was never resolved past a
disc-streaming placeholder at packaging time. It is a plausible mechanism,
not a proven cause: nothing in this function's own logic touches archive
*content* at all, only whether a `data%02d.psarc` gets mounted, and mounting
happens once per whole archive, not per entry - so it cannot itself explain
an archive that mounts fine and still reads some entries as zero and others
real. See `docs/formats/psarc.md`'s "Block data location" section for the
per-entry measurement this page's finding does not resolve.

## Not read

- **`libSceFios2.prx`** - the actual archive-mount/block-read implementation,
  a separate, signed Sony system module this project holds
  (`sce_module/libSceFios2.prx`) but has not imported into Ghidra. This
  binary's own code, above, never touches an archive's block table at all;
  every byte-location decision after `sceFiosArchiveMountSync` returns
  happens inside that module. Not chased further this session - it is a
  large, closed-source, general-purpose Sony middleware component rather
  than Wipeout-specific game code, and disassembling it would explain the
  *container's* layout (already documented at 88-92 confidence from the
  archives' own bytes) without being able to explain why specific bytes
  *in a static, offline-dumped file* are zero, which is a question about
  this one dump's provenance, not about FIOS2's mount-time behaviour.
- The exact bit layout of `ScePlayGoLocus` (`PsarcArchive_WaitAndMountAll`'s
  own locus-byte tests) - inferred from the decompilation's control flow, not
  checked against an SDK header this project does not hold.

## See also

- [`docs/formats/psarc.md`](../../../formats/psarc.md) - "The PS4 Omega
  Collection family", the container-level findings this page corroborates
  from the executable side.
