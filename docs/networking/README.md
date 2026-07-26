# Networking

> **Not yet started.** Milestone M7, after single-player is complete.

## Scope

- Ad-hoc wireless multiplayer, the PSP's local mode
- Infrastructure multiplayer, if it existed
- Game sharing: transmitting a playable subset to a nearby PSP
- Downloadable content, if any

## What is known

The PSP release ships only three PRX modules: `libfont`, `libmp3` and
`pspnet_ap_dialog_dummy`. **There is no `pspnet` stack and no `libhttp`.**

Pure, by contrast, ships 26 modules including the full network stack, `libhttp`,
`libssl` and the ad-hoc download modules. Pure had HTTP-delivered downloadable
content.

Pulse's absence of those modules suggests it either has no online functionality
or reaches it another way. The `pspnet_ap_dialog_dummy` name is suggestive: a
*dummy* access-point dialog implies networking was considered and stubbed.

The PSP disc does carry `UCES00465/GSHARE/SHARE.BIN`, a 6.6 MiB PBP that is
almost certainly the game-sharing payload. See
[PSP disc layout](../psp/pulse-disc-layout.md).

Confidence that Pulse lacks an HTTP stack: **90**, from the module list.
Confidence in any conclusion about what that means: **low**. It has not been
checked against the binary.

## Approach

Deliberately last. Multiplayer built on an unverified simulation inherits every
one of its bugs and adds desync on top. Determinism, which the engine has from
M0, is the prerequisite that makes lockstep networking viable at all.
