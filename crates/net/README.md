# oag-net

Network play: the wire messages, a transport with two channel kinds over
WebTransport, and the authoritative server and predicting client around a race.
Also the `oag-server` binary.

The server links no renderer, window, audio or UI crate (dependency rule 6), and no
gameplay crate may depend on this one. See [networking](../../docs/networking/README.md) and
[ADR-0070](../../docs/architecture/adr/0070-the-network-transport-is-webtransport.md).
