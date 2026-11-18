# oag-disc

Reads PSP UMD, PS2 DVD and PS3 BD-ROM disc images, as CHD or raw ISO, and walks
their ISO 9660 filesystems. The container is sniffed by magic, and the sector
source and the filesystem walker are separate so each tests alone.

It knows what a disc is and nothing about what is on one. Archives start in
`oag-formats` and `oag-assets`.
