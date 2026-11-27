# oag-io

File I/O helpers for the crates that write the player's own files. Today that
is the atomic write (ADR-0095): `write`, `write_bytes` and `write_private`
stage the new content beside the file, flush it and rename it over the old one,
so an interrupted write leaves the previous file readable. `write_shared` is the
same for a derived file that two processes may produce at once (the unpacked
DLC cache): its temp file is per process and lives where the caller says.

It is a leaf: std only, no workspace dependency, so any crate can reach it.
`oag-game`'s `profile` module is the thin caller for the profile folder; the
mod campaign editor and the PS3 disc-key cache call it directly.
