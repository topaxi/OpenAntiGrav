---
status: parked
titles: [all]
updated: 2026-11-22
summary: "Web keeps no replays and a phone keeps them out of its file manager; offer DOWNLOAD and SHARE on a replay row"
---

# Replays leave the web build and a phone by download and share

2026-11-17, the maintainer: "on web builds we should offer a download button for replays,
on mobile maybe a download and share button? or both for both platforms". Written down,
not planned into a lane.

## What exists

- The REPLAYS page lists files from a directory. **A phone now records into the app's own files
  directory** (`replays::WRITES`; reached over USB, hidden from the phone's file manager on
  Android 11 and later, `replays::REACHABLE` is false there), so its page lists what it
  recorded. The web build has no file system, so its page is empty
  ([replays.md](../../docs/ui/replays.md), the open list).
- A replay is one `.oagr` file ([replay-file.md](../../docs/formats/replay-file.md)), a few
  hundred KB for a five-minute race of eight craft, so handing it to the platform is cheap.

## Open

1. **Where a web build keeps its replays first.** A browser has no directory: the origin's
   private file system (OPFS) or IndexedDB would hold them so the page lists something.
   Without that, a download can only follow straight after a race.
2. **The buttons, per platform** (proposal, chosen, not measured):

   | Platform | DOWNLOAD | SHARE |
   | --- | --- | --- |
   | Web, desktop browser | a file download of the `.oagr` | the Web Share API with files where the browser has it (mostly mobile browsers), else hidden |
   | Web, phone browser | as desktop | Web Share API (the phone's share sheet) |
   | Android | save through the system file picker (Storage Access Framework) | `ACTION_SEND` share sheet |
   | iOS | save through the document picker ("Save to Files") | `UIActivityViewController` share sheet |
   | Desktop native | not needed (the directory is reachable); optional "show in folder" | not needed |

   Offer both wherever the platform has both; hide a button the platform cannot do
   rather than drawing it dead.
3. **The way back in.** A downloaded or shared replay is only useful if a player can open
   one: an IMPORT/OPEN on the same page (file picker on web and phones, a file association
   or drag-and-drop on desktop). Old files may not reproduce on a newer build; POSE VIEW
   covers that when the file carries a pose track.
4. **Where the rows live.** The REPLAYS page's row actions, and the end screens' WATCH
   REPLAY row (the online real-replay lane adds one) could offer SAVE/SHARE straight after a
   race. Strings need `string_id` and English text (CLAUDE.md, `just check-strings`).
5. **Platform glue.** Web glue sits beside the web build's existing browser calls; Android
   and iOS calls belong in `oag-mobile` ([ADR-0065](../../docs/architecture/adr/0065-one-mobile-crate-for-every-phone-os.md)).

## Next Steps

None drawn. When picked up: decide item 1 for the web build first (without storage the page
stays empty), then one lane per platform family (web, Android, iOS), each with a test or a
recorded manual check that a saved file opens again on the same build.
