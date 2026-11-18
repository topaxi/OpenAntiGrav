# oag-bng-import

Maps a BallisticNG Workshop track, read by `oag-ballisticng`, into
`oag_trackfile::Track`. Every constant that turns a reading into a number sits in
one module, and what the content does not author is named in the track's notes
instead of being filled in.

The one crate that depends on both the content layer and the track model. The
race loader calls it. No gameplay crate may depend on it.
