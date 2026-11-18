# oag-video

Video containers: the formats the originals wrap a bitstream in (`.PMF`, IPUF,
Bink, MP4) and this project's own IVF/AV1 movie cache. A demuxer only; playing a
bitstream happens elsewhere.

Its only normal workspace dependency is `oag-bytes`. Cache design:
[ADR-0008](../../docs/architecture/adr/0008-av1-movie-cache.md).
