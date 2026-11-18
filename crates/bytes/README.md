# oag-bytes

Endian-aware byte reading for the whole workspace: the one `ByteOrder`, the
`Option` word readers (`u16_at`, `u32_at`, `f32_at`, the `le::` and `be::` forms)
and `cstr_at`. Every decoder reads its input through it.

It is a leaf: no workspace dependency. Most format crates sit directly on it.
