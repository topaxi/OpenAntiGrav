# Resource loading: how a WAD entry becomes a live object

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`, language `Allegrex:LE:32:default`.

**The names below are applied**, from [names.tsv](names.tsv) via
`just apply-names`. Per
[ADR-0005](../../../architecture/adr/0005-ghidra-conventions.md), applying a
rename needs a page carrying its evidence; this page is that evidence.
Nothing here is under 70, so nothing carries a `_q`.

This is the connecting page. The two ends were already read - the WAD
device on [wad-subsystem.md](wad-subsystem.md) (hash, directory, LZSS) and
the `.vex` node tree on [vex.md](../../../formats/vex.md) (header, classes,
payloads) - and what was missing is the middle: the VFS that turns a path
into a device, the **resource object and cache** that own the bytes, and
the **scene-graph node** the bytes are turned into, with its three linked
lists. Read top to bottom it is one call chain:

```
Vex_LoadModel(model, "wad:Data\Ships\Ship.vex", ...)
  -> Resource_LoadFile(path, use_cache, from_top)
       -> Resource_FindLoaded(path)              cache hit: refcount++, return
       -> Vfs_Open(path, 0x2001)                 device list, by prefix
            -> Wad_Open (device vtable +0x1c)    hash, directory scan
       -> Mem_AllocAligned(size, 64) / ...FromTop
       -> Vfs_Read(file, buf, size)              128 KiB chunks on a streaming device
            -> Wad_Read (device vtable +0x2c)    LZSS/zlib into the buffer
       -> new Resource{data, size, hash, name, device}; Resource_Link
  -> Vex_RelocateNodeTree(payload)               once per resource, flag bit 0
  -> Vex_FindClassDescriptor(root class_id)
       -> descriptor vtable +0x74: construct     Node_ConstructBase + per-class ctor
       -> node vtable +0x7c: init(resource)      per-class payload read
  -> Vex_CollectNodesByClass(Mesh), (Texture), ...
  -> Texture_BindEmbeddedData per texture, Mesh_BuildModelDrawData
```

## Summary

| Address | Name | Kind | Conf |
| --- | --- | --- | ---: |
| `0x08943e18` | `Resource_FindLoaded` | function | 88 |
| `0x08943ce0` | `Resource_Link` | function | 85 |
| `0x0893e578` | `Vfs_Read` | function | 90 |
| `0x0893e3e0` | `Vfs_LinkFile` | function | 85 |
| `0x0893de1c` | `Vfs_ResolveDevice` | function | 85 |
| `0x08943f08` | `Object_ConstructBase` | function | 85 |
| `0x08944fc0` | `Node_ConstructBase` | function | 85 |
| `0x08944bd4` | `Node_AttachChild` | function | 88 |
| `0x08944254` | `Node_Destroy` | function | 85 |
| `0x0894402c` | `Node_UpdateTree` | function | 85 |
| `0x08944328` | `Node_ReapPending` | function | 88 |
| `0x08a71364` | `Vex_CollectNodesByClass` | function | 88 |
| `0x088f3540` | `ParticleManager_LoadResource` | function | 85 |
| `0x08ac1e88` | `g_resource_list` | data | 90 |
| `0x08ac1e94` | `g_resource_sema` | data | 90 |
| `0x08ac1e38` | `g_vfs_device_list` | data | 88 |
| `0x08ac1e3c` | `g_vfs_file_list` | data | 85 |
| `0x08b66444` | `g_vfs_file_sema` | data | 88 |
| `0x08ac1edc` | `g_node_count` | data | 80 |
| `0x08b30f10` | `g_pending_destroy_nodes` | data | 88 |
| `0x08ac1eec` | `g_pending_destroy_count` | data | 88 |

Already named elsewhere and used here without a new row:
`Resource_LoadFile` `0x08943330` and `Vex_LoadModel` `0x08912b80`
([vex.md](../../../formats/vex.md)), `Vfs_Open` `0x0893de94` and
`Vfs_FindDevice` `0x0893e0dc` ([memory.md](memory.md)), `Vfs_SplitDevicePath`,
`Wad_Open`, `Wad_Read` ([wad-subsystem.md](wad-subsystem.md)).

## The VFS: a path is a device prefix and a name

`g_vfs_device_list` (`0x08ac1e38`) is a singly linked list of device
objects - name string at `+0x00`, next at `+0x24`, vtable at `+0x30` -
registered by `Wad_MountArchive` and its siblings (`wad:`, `fe:`, `fedata:`,
`bedata:`, plus the memory-stick and `host0:` devices that the same
`Vfs_Open` reaches). The device vtable slots this page touches:

| Slot | Operation | WAD implementation |
| --- | --- | --- |
| `+0x1c` | open(name, flags, mode) -> fd | `Wad_Open` |
| `+0x24` | close(fd) | `Wad_Close` |
| `+0x2c` | read(fd, buf, len) -> n | `Wad_Read` |
| `+0x3c` | seek(fd, offset, whence) -> pos | whence `0` set, `1` **end** (returns the size), `2` **tell** |

Each vtable entry is paired with a `short` this-adjustment at the previous
slot (`+0x18`, `+0x20`, `+0x28`, `+0x38`), the MIPS ABI's way of calling
through a multiply-inherited interface; every call site adds it to the
device pointer first.

`Vfs_Open(path, flags, mode)` (`0x0893de94`): `Vfs_SplitDevicePath` finds
the device by prefix and hands back the bare name; the device's `open` runs;
unless flag `0x200` is set the size is read (seek end, seek set) and an
empty file is closed and treated as absent. With an explicit prefix a miss
is final; a prefix-less path moves on to the next device in the list. The
0x110-byte file object is set up by `Vfs_LinkFile` (`0x0893e3e0`) - device
at `+0x00`, fd at `+0x04`, the path copied at `+0x0c`, vtable `0x08ad3094`
at `+0x10c` - and appended to `g_vfs_file_list` (`0x08ac1e3c`) under the
`"FS File Sema"` (`g_vfs_file_sema`, `0x08b66444`, created lazily on the
first open).

`Vfs_Read(file, buf, len)` (`0x0893e578`): on a device whose kind word at
`+0x2c` is `1` - the streaming UMD path - reads in **128 KiB** (`0x20000`)
pieces with `sceKernelDelayThread(100)` between them, so a 7 MB archive load
yields to the loading-screen thread ([main-loop.md](main-loop.md)) between
pieces; any other device gets one read call. `Vfs_ResolveDevice(path)`
(`0x0893de1c`) opens with mode `1`, keeps only the device pointer, and
closes - it exists so the resource cache can tell "same name on a different
device" apart.

## The resource object: `Resource_LoadFile` (`0x08943330`)

`Resource_LoadFile(path, use_cache, from_top)` - confidence 88 on
[vex.md](../../../formats/vex.md), and this is the reading behind it:

1. If `use_cache`, `Resource_FindLoaded(path)` (`0x08943e18`) walks
   `g_resource_list` (`0x08ac1e88`) comparing the `Wad_HashName` of the
   path against each entry's hash, and - when the path resolves to a device
   and the entry recorded one - the device *names* by `strcasecmp`. A hit
   resets the payload cursor to the start of the data, bumps the refcount,
   sets flag bit 0 and clears bit 1, and returns the existing object. **The
   same bytes are never loaded twice while anything holds them.**
2. Otherwise, under `g_resource_sema` (`0x08ac1e94`): `Vfs_Open(path,
   0x2001)`, size via the seek triple, and a buffer of
   `round32(size) + 16` bytes at 64-byte alignment from `Mem_AllocAligned`
   or - when `from_top` - `Mem_AllocAlignedFromTop`
   ([memory.md](memory.md)), tagged with a copy of the path as its `file`
   argument. `Vfs_Read` fills it and the file is closed.
3. A 40-byte `Resource` is allocated and linked at the tail of
   `g_resource_list` by `Resource_Link` (`0x08943ce0`):

| Offset | Field | Set to |
| --- | --- | --- |
| `+0x00` | `data` | the buffer |
| `+0x04` | `cursor` | the buffer; `Vex_LoadModel` advances it past the 16-byte file header |
| `+0x08` | `size` | file size |
| `+0x0c` | `hash` | `Wad_HashName(name)` |
| `+0x10` | `name` | heap copy of the path |
| `+0x14` | `flags` | 0; bit 0 = "handed out before" (so the relocation pass runs once), bit 1 = released |
| `+0x18` | `next` | list link |
| `+0x1c` | `refcount` | 1 |
| `+0x20` | `device` | the device that served the open |
| `+0x24` | vtable | `0x08ad33ac` |

`use_cache` is `!(flags & 0x10)` from `Vex_LoadModel`'s own flags word, so a
model can opt out of sharing; `from_top` is always 0 from that caller and
is the resident-archive placement `Wad_MountArchive` uses.

**Relocation happens once per resource, not once per load.** `Vex_LoadModel`
checks the resource's flag bit 0 before calling `Vex_RelocateNodeTree`; a
cache hit arrives with the bit set, so a second model built from the same
`.vex` reuses already-fixed pointers. [pob.md](../../../formats/pob.md)'s
`ParticleManager_LoadResource` (`0x088f3540`) makes the same check before
its own fixup (`FUN_088f8e38`) and additionally keeps a private
name-to-resource table of 0x8c-byte rows in the particle manager (count at
`+0x468c`), so a `.POB` is looked up twice: once by the manager's
`strcasecmp` table, then by the global cache.

## The scene graph: what a node is

Every loaded thing becomes a node, and every node starts with the same two
constructors. `Object_ConstructBase` (`0x08943f08`) lays down the 0x3c-byte
object header; `Node_ConstructBase` (`0x08944fc0`) calls it, installs the
node vtable `0x08ad342c`, ORs `0x1001000` into the flags and increments
`g_node_count` (`0x08ac1edc`). [vex.md](../../../formats/vex.md) counted 32
call sites of the latter, `Vex_LoadModel` among them.

| Offset | Field | Notes |
| --- | --- | --- |
| `+0x04` | class tag | a runtime type id; `Vex_CollectNodesByClass` compares it |
| `+0x08` | parent | |
| `+0x0c` | next sibling | |
| `+0x10` | first child | |
| `+0x18` / `+0x1c` | update chain head / next | |
| `+0x20` / `+0x24` | draw chain head / next | |
| `+0x28` | name | `"Unknown"` until a class sets it |
| `+0x2c` | flags | `0x3006` at construction; `0x02` = updatable, `0x08` = destroy me, `0x20` = not on an update chain, `0x40` = not on a draw chain, `0x8000` = skip post-update, `0x100000` = registered with the manager at `DAT_08ac00c0` |
| `+0x38` | vtable | `+0x0c` post-load callback, `+0x24` pre-update(dt), `+0x2c` post-update(dt), `+0x6c` destructor, `+0x74` class construct, `+0x7c` init from resource |

**Three lists, not one.** `Node_AttachChild(parent, child)` (`0x08944bd4`)
appends the child to the parent's child chain and then, unless the child's
flags say otherwise, to two more: the **update chain** (`+0x18`/`+0x1c`) of
the nearest ancestor that is itself updatable (walks up past ancestors with
flag `0x20`), and the **draw chain** (`+0x20`/`+0x24`) of the nearest
drawable ancestor (walks up past flag `0x40`). That is how one tree serves
`Game_UpdateFrame` without a per-frame recursion over the hierarchy:
`Node_UpdateTree(dt, node)` (`0x0894402c`, what `Game_UpdateFrame` calls
with the clamped delta [frame-pacing.md](../../../psp/frame-pacing.md)
derives) skips a node flagged `0x08`, calls its pre-update virtual
(`+0x24`), and if that returns non-zero walks the node's **update chain**,
recursing into each entry that carries flag `0x02`, then calls the
post-update virtual (`+0x2c`) unless flag `0x8000`. The chain walk keeps
its cursor in the node's own `+0x14` (`next | 1` while walking), so a
callee that detaches itself mid-walk does not break the iteration.
Confidence 88 - the three appends are unambiguous, and the update walk is
read; the draw chain's consumer is inferred by symmetry.

### Destruction is deferred: a flag plus a 32-slot list

`Node_Destroy(node)` (`0x08944254`) is the teardown, and it has two modes.
With flag `0x08` set on the node it destroys the subtree leaf-first
(marking each child `0x08` as it goes), increments `DAT_08ab060c`, and
calls the node's own destructor (vtable `+0x6c`, argument `3`). **Without**
the flag it only recurses, so `Node_Destroy(root)` on an unflagged root is
a sweep that destroys every flagged node under it and nothing else.

Nobody calls it directly to retire an object. Instead every retirement
site in the binary carries the same inlined epilogue, and it is worth
recognising because Ghidra shows it hundreds of times (`Vex_LoadModel` on
a missing file, `CloudGroup_Init`, `Missile_Update` when a missile is
spent, `Trail_Init`, `InGame_Construct`, `Race_CreateModeObject`, ...):

```c
node->flags = (node->flags & ~4) | 0xa;                 // updatable off... and destroy me
if (g_pending_destroy_count < 32) {
    for (i = 0; i < g_pending_destroy_count; i++)
        if (g_pending_destroy_nodes[i] == node) goto listed;
    g_pending_destroy_nodes[g_pending_destroy_count] = node;
}
g_pending_destroy_count++;
```

`Node_ReapPending(root)` (`0x08944328`), called by `Game_UpdateFrame`
right after `Node_UpdateTree`, drains it once per frame: one entry is
destroyed directly; up to 32 are first deduplicated against each other by
ancestry (`FUN_08943fdc(a, b)`, an "is `a` under `b`" test, clears the
descendant so a subtree is not destroyed twice) and then destroyed; and
**more than 32** - the array overflowed, the count kept climbing - falls
back to `Node_Destroy(root)`, the whole-tree sweep. If destroying entries
queued more (the count moved), the sweep runs as well. Then the count is
reset to 0.

The construction-time variant is the same list used as a scope: `Game_MainLoop`
saves `g_pending_destroy_count` before each of its four root children,
and if a child comes back flagged `0x08` calls `Node_Destroy` on it at once
and restores the count, so a constructor that gave up does not wait a frame
to be reaped. That is also why `Vex_LoadModel` returns `model` on a missing
file rather than 0: the dead node stays in the tree until the next reap.
Confidence 88, from `Node_ReapPending` itself plus dozens of push sites.

## `Vex_LoadModel` (`0x08912b80`): bytes to tree

With the pieces above, the loader reads straight through
(`Vex_LoadModel(model, path, world, layer, sort, flags)`):

1. `Node_ConstructBase(model)`, model vtable `0x08ad17a4`, a transform at
   `+0xb0`. Flag `0x08` in `flags` becomes the byte at `+0x1c4`.
2. `res = Resource_LoadFile(path, !(flags & 0x10), 0)` into `+0x9c`. On 0
   the failure epilogue runs and the model is returned dead.
3. The cursor at `res+0x04` is advanced 16 bytes past the `.vex` file
   header; `Vex_RelocateNodeTree` runs if the resource's bit 0 is clear.
   The embedded texture block starts at `payload + tree_size + 16`.
4. `Vex_FindClassDescriptor(root->class_id)`, then the descriptor's
   **construct** virtual (`+0x74`, with `model` and `world`) makes the root
   node, stored at `+0xa0`, and the root's **init** virtual (`+0x7c`) is
   handed the resource. Per-class constructors (`CloudGroup_Init`,
   `FogCube`, `WeatherPos`, the sound emitters - each on its own page) are
   what run here; they read their payload through the resource cursor and
   attach children with `Node_AttachChild`.
5. `Vex_CollectNodesByClass` (`0x08a71364`, the unconditional recursive
   collector [vex.md](../../../formats/vex.md) quotes) gathers up to 2,000
   `Mesh` nodes (each gets `layer`/`sort` at `+0xcc`/`+0xd0`, and flag
   `0x02` sets its `+0x51`), up to 1,000 texture nodes (array at `+0x1a4`,
   count `+0x1a0`), and up to 64 of a third class (`+0xa8`/`+0xa4`).
6. A post-order walk (`FUN_089444d8`) calls vtable `+0x0c` on every node
   with the resource - the "loaded" callback. Then each texture node gets
   `Texture_BindEmbeddedData(tex, block_ptr, 0)`, which returns the next
   block pointer, so the embedded textures are consumed in tree order.
7. `Mesh_BuildModelDrawData(model, meshes, count)` unless `flags & 1`
   (then `FUN_08910e38`, the alternate build) or `flags & 4` (no draw data
   at all). The child with the `FUN_08a6bd6c` tag is cached at `+0xac` -
   the "world pointer cache" [lighting.md](../psp-pulse-eu/lighting.md)
   describes - and the presence of a `FUN_08a6bfc0`-tagged child sets
   `+0x1a8`, which switches on the environment-map light basis: two GE
   command lists at `+0x48` and `+0x70` are written from
   `g_envmap_light_basis` and written back from the data cache.
8. `FUN_08945588(model)`: if flag `0x1000000` (set by `Node_ConstructBase`)
   is still up, clear the top byte and call `FUN_08944f6c(model, 0x1000)` -
   the "construction complete" transition.

So the answer to "how does a WAD entry become a live object" is: **one
resource object per distinct file, refcounted and relocated once; one node
per `.vex` node, constructed through the class-descriptor table's virtuals;
and every node threaded onto up to three chains at attach time, which is
what makes it live.** Nothing is ever looked up by name after load - the
name survives only as the resource's cache key and the node's `+0x28`
debug string.

## Cross-platform

| Platform | Notes |
| --- | --- |
| PS2 (Pulse) | `Resource_LoadFile`'s counterpart is `0x001fd6b8` per [ps2 xml-reader.md](../ps2-pulse-eu/xml-reader.md); the rest not located on this pass. |
| PSP (Pulse, EU) | `Resource_LoadFile` `0x08942e0c`, `Vex_LoadModel` `0x0891267c` ([corroboration.md](../psp-pulse-eu/corroboration.md)); the helpers named here were not carried across. |

## Open questions

- What the third collected class (`FUN_08a6ba4c`'s tag, up to 64 per model,
  `+0xa4`/`+0xa8`) is. The per-element call `FUN_088786d0(*(node+0x58))`
  sits in the camera code's address range, so lights or cameras are the
  candidates; not read.
- Which class tags `FUN_08a6bd6c` and `FUN_08a6bfc0` return. The first is
  the world/root cache the EU lighting pass found; the second gates the
  env-map basis.
- `FUN_08944a38`'s manager at `DAT_08ac00c0` (flag `0x100000`): a
  registry every dead node is removed from; not read.
- `DAT_08ab060c` / `DAT_08ab0610`, zeroed by `Node_ReapPending` and
  incremented by `Node_Destroy`: per-frame destroy counters, reader not
  found.

## History

- 2026-09-16: first reading, static. The device vtable's whence codes are
  corroborated by two independent call sites (`Vfs_Open` and
  `Wad_MountArchive`) using them the same way; the update chain is
  corroborated by `Node_UpdateTree` walking exactly the list
  `Node_AttachChild` appends to. No runtime leg.
