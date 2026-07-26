# Glossary

Terms used throughout this project, split into Wipeout terminology, platform
terminology and project terminology. Where a term's meaning in Pulse is
unconfirmed, it is marked.

## Wipeout

| Term | Meaning |
| --- | --- |
| **AG** | Anti-gravity. The ships hover; they do not have wheels, and the handling model is nothing like a car's. |
| **Craft / ship** | The player and AI vehicles. Pulse has one per team, with per-team handling stats. |
| **Team** | Feisar, AG Systems, Auricom, Qirex, Piranha, and so on. Determines a ship's stats and livery. |
| **Speed class** | Vector, Flash, Rapier, Phantom, in increasing order of speed. Affects top speed and handling. |
| **Zone** | A game mode where the ship accelerates continuously through escalating speed tiers until it is destroyed. |
| **Eliminator** | A combat mode scored on kills rather than position. |
| **Airbrake** | Shoulder-triggered brakes used to tighten turns. Central to the handling model. |
| **Pitch** | Nose up/down control, used over jumps and to manage landings. |
| **Absorb** | Consuming a held weapon to recharge shield energy instead of firing it. |
| **Shield energy** | The damage pool. Reaching zero destroys the ship. |
| **Speed pad / boost pad** | Track-surface pads that apply a speed boost. |
| **Weapon pad** | Track-surface pads that grant a random pickup. |
| **Magstrip** | Magnetic track sections that hold the ship to the surface through loops and inversions. |
| **Barrel roll** | An air trick that grants a small speed boost on landing. |
| **Front end (FE)** | The menu system. The `FE.wad` and `FEData.wad` files hold its assets. |
| **Back end (BE)** | Apparently the in-race side, from `BEData.wad`. *Unconfirmed.* |
| **Grid** | Starting positions. |

## PSP

| Term | Meaning |
| --- | --- |
| **UMD** | Universal Media Disc, the PSP's optical format. ISO 9660 filesystem, 2048-byte sectors. |
| **`UMD_DATA.BIN`** | Root file holding the disc ID and a key. First field is the serial, e.g. `UCUS-98712`. |
| **`PSP_GAME`** | The standard root directory of a PSP game disc. |
| **`EBOOT.BIN`** | The encrypted, signed executable the PSP actually boots. |
| **`BOOT.BIN`** | The *unencrypted* executable. Usually present on retail UMDs, and the reverse-engineering target. |
| **PBP** | A container bundling an executable with icons and metadata. Magic `\0PBP`. |
| **PRX** | A PSP relocatable shared library. |
| **SFO** | System file object: the key/value metadata blob in `PARAM.SFO`. Magic `\0PSF`. |
| **PMF** | PSP Movie Format, an MPEG-4 container. |
| **AT3** | ATRAC3 audio, in a RIFF container. |
| **VFPU** | The PSP's vector FPU. Fast, and **not** IEEE-conformant for reciprocal and reciprocal square root, which is why bit-exactness is a non-goal. |
| **Game sharing** | Sharing a playable subset with nearby PSPs over ad-hoc wireless. `GSHARE/SHARE.BIN` is the payload. |

## PS2

| Term | Meaning |
| --- | --- |
| **EE** | Emotion Engine, the main MIPS CPU. |
| **IOP** | I/O Processor, a secondary CPU handling storage, controllers and audio. Runs `.IRX` modules. |
| **IRX** | An IOP relocatable executable. |
| **VU0 / VU1** | Vector Units. VU1 typically drives geometry transformation. |
| **GS** | Graphics Synthesizer. |
| **IPU** | Image Processing Unit, the MPEG decoder. `.IPF` files target it. |
| **PSS** | PlayStation Stream, an MPEG-2 program stream. |
| **SCREAM** | Sony's audio engine, shipped here as `IOP/SCREAM.IRX`. |
| **`SYSTEM.CNF`** | Boot configuration; its `BOOT2` line names the main ELF. |

## Disc images

| Term | Meaning |
| --- | --- |
| **CHD** | Compressed Hunks of Data, MAME's compressed disc image format. |
| **Hunk** | A CHD's compression unit, containing several sectors. |
| **Unit** | One sector's worth of storage inside a hunk. 2048 bytes for DVD/UMD, 2448 for CD. |
| **Cooked vs raw** | A cooked sector is just the 2048 bytes of user data. A raw sector is the full 2352-byte CD frame including sync, header and error correction. Which one a CHD holds depends on the *track type*, not the unit size. See [ADR notes in the PS2 disc layout](../ps2/pulse-disc-layout.md). |
| **LBA** | Logical block address: a sector index from the start of the disc. |
| **PVD** | Primary volume descriptor, the ISO 9660 root structure at sector 16. |

## Project

| Term | Meaning |
| --- | --- |
| **Tick** | One step of the simulation, at a fixed rate. Distinct from a rendered frame. |
| **Trace** | A per-tick recording of simulation state, from either the original or our engine, used for comparison. |
| **Probe** | The toy simulation in `oag-core` that exists only to detect cross-platform float divergence. |
| **Confidence** | A score from 0 to 100 attached to every reverse-engineering claim. See the [rubric](../reverse-engineering/confidence-rubric.md). |
| **Golden test** | A test comparing against a committed known-good output. |
| **Behavioural test** | A test comparing against the original, within documented tolerances. |
