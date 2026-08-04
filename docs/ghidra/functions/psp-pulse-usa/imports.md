# Import stubs

Functions in `PSP_GAME/SYSDIR/BOOT.BIN` (Pulse PSP, UCUS-98712), image base
`0x08804000`.

**Applied**, and unusually cheap to trust: these names are *checked*, not
inferred. See [names.tsv](names.tsv) and `just apply-names`.

## Why this is a verification and not a guess

The ELF carries no symbol for an imported function. Each one is identified by a
32-bit **NID**, and a NID is the first four bytes of `SHA-1(name)`,
little-endian. That is one-way, so a NID cannot be turned back into a name, but
a *candidate* name can be hashed and compared. A wrong name cannot produce the
right NID.

So the procedure is: hash the public PSP SDK function names
([`scripts/psp-import-names.txt`](../../../../scripts/psp-import-names.txt)), and
keep the ones that match. **306 of 335 stubs** match.

The rule itself was confirmed before being relied on, against the two NIDs
[video](frontend-video.md) had already recovered independently by call-site
arity: `SHA-1("sceMpegRingbufferConstruct")` gives `0x37295ED8` and
`SHA-1("sceMpegCreate")` gives `0xD8C5F121`, both exactly as recorded there.

Confidence **99** for every matched name. The residual doubt is not about the
hash but about synonyms: a name that changed between SDK revisions would hash to
a different NID and simply not match, so the failure mode is a missing name
rather than a wrong one.

## Layout

Everything comes from the ELF's own section headers, so nothing here is a guess
about where the tables are:

| Section | Address | Contents |
| --- | --- | --- |
| `.lib.stub` | `0x08a774d0` | 38 entries of 20 bytes, one per imported module |
| `.rodata.sceNid` | `0x08a77ac4` | 335 `u32` NIDs |
| `.sceStub.text.<module>` | varies | 8 bytes per stub, parallel to that module's NIDs |

```c
struct SceLibStub {        // 20 bytes
    char     *name;
    u16       version;
    u16       flags;
    u8        entry_size;  // 5, in words
    u8        var_count;
    u16       func_count;
    u32      *nids;        // func_count entries
    void     *stubs;       // func_count * 8 bytes
};
```

The two size fields agree with the sections exactly: `0x2f8 / 20 = 38` modules,
and the module function counts sum to `0x53c / 4 = 335` NIDs. That arithmetic
holding is the check that the entry layout is read correctly.

Pointers in the file are unrelocated, so the image base is added on the way out.

## The modules

| Module | Fn | Resolved | Notes |
| --- | ---: | ---: | --- |
| `ThreadManForUser` | 63 | 63 | Threads, semaphores, event flags, VTimers |
| `sceUtility` | 28 | 20 | Savedata, message dialog, netconf, **HTML viewer** |
| `sceSasCore` | 26 | 26 | The hardware voice synth: 32 channels, ADSR, reverb |
| `sceMpeg` | 23 | 23 | Video, see [video](frontend-video.md) |
| `IoFileMgrForUser` | 19 | 19 | File I/O |
| `sceNetInet` | 18 | 18 | Sockets |
| `sceAudio`, `sceMp3` | 13, 13 | 13, 13 | PCM output and streamed music |
| `sceGe_user` | 10 | 10 | Display-list submission |
| `UtilsForUser` | 9 | 9 | Cache control, MT19937, MD5, SHA-1 |
| `sceNpService` | 9 | 0 | See below |
| `sceAtrac3plus`, `SysMemUserForUser` | 8, 8 | 8, 7 | ATRAC3+ decode, partition memory |
| `ModuleMgrForUser`, `sceNetAdhoc`, `sceHttp` | 7 each | 6, 7, 5 | |
| `sceUmdUser`, `sceNetApctl` | 6, 6 | 6, 6 | |
| `sceNetAdhocctl`, `sceNetResolver` | 5, 5 | 5, 5 | |
| `sceDisplay`, `sceNet`, `sceNpAuth` | 4 each | 4, 4, 0 | |
| `Kernel_Library`, `StdioForUser`, `LoadExecForUser`, `sceSuspendForUser`, `sceCtrl`, `sceRtc`, `sceNp` | 3 each | 3, 3, 3, 3, 3, 2, 0 | |
| `sceImpose`, `sceHprm`, `sceNetAdhocMatching`, `sceSsl` | 2 each | 2 each | |
| `sceOpenPSID`, `scePower`, `scePspNpDrm_user`, `sceWlanDrv` | 1 each | 1 each | |

Three things in that list are worth knowing before the corresponding subsystem
is reimplemented:

**Audio is `sceSasCore`, not software mixing.** 26 of its functions are
imported, including `__sceSasSetADSR`, `__sceSasSetADSRmode`, `__sceSasRevType`
and `__sceSasCoreWithMix`. The PSP's synth does the mixing, envelopes and reverb
in hardware, so an accurate audio path means modelling those semantics rather
than just playing samples.

**`sceUtilityHtmlViewer` is imported**, all four of its functions. Pulse has an
in-game browser path. Nothing in the front end has been traced to it yet.

**Only four `sceDisplay` functions**, and the confirmation that matters:
`sceDisplayWaitVblankStart` (non-callback) is genuinely absent while
`sceDisplayWaitVblankStartCB` is present, exactly as
[frame pacing](../../../psp/frame-pacing.md) reads it. `sceUtilityLoadAvModule`
is likewise absent: AV modules load through the general
`sceUtilityLoadModule`, as [video](frontend-video.md) says.

## The 29 that do not resolve

`sceNp`, `sceNpAuth` and `sceNpService` account for 16 of them, and they are the
interesting case: **their NIDs are not SHA-1 of the name.** `0x857B47D3` is the
published NID for `sceNpInit`, but `SHA-1("sceNpInit")` is `0x98BEF739`. The NP
libraries were assigned NIDs by another scheme, so naming them needs a published
table rather than a hash, and none is used here.

The rest are one or two entries each in `ModuleMgrForUser`,
`SysMemUserForUser`, `sceUtility`, `sceRtc` and `sceHttp`: names outside the
candidate list. Adding a name to
[`scripts/psp-import-names.txt`](../../../../scripts/psp-import-names.txt) and
re-running is the whole fix, and the hash decides whether it was right.

## Reproducing

```sh
just resolve-imports    # binary -> data/ghidra/psp-imports.tsv, with coverage
just apply-names        # that plus names.tsv, into the open Ghidra program
```

The generated TSV is not committed: it is derived from your own copy of the
binary, and the script rebuilds it in under a second.
