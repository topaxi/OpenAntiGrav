# oag-unity

A generic Unity asset-bundle reader: UnityFS containers, SerializedFiles,
typetrees and the built-in classes. It knows no game. A game's own classes are
read by a layer on top, and never the other way round.

Its only workspace dependency is `oag-bytes` (dependency rule 4). See
[ADR-0062](../../docs/architecture/adr/0062-unity-content-is-a-generic-reader-with-a-content-layer-on-top.md).
