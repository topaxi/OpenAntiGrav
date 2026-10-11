# Track fixture

A small closed-loop track, authored by this project: **no game or Workshop
content**, so it is committed and its tests run in plain `just test` and CI
(`tests/fixture_track.rs`).

| Piece | Content |
| --- | --- |
| `fixture_loop` | a prefab bundle: `TrackFloor`, `TrackWallLeft`, `TrackWallRight` (258 vertices, 256 triangles each), one 64x64 DXT1 texture with 7 mips, a Standard material, mesh colliders, a `StartLine` transform, and one `TrackSpline` MonoBehaviour |
| `fixture_loop_scene` | the same content as a scene bundle: two SerializedFiles |
| `lzma/`, `lz4/`, `raw/` | the same bundles with LZMA, LZ4HC and uncompressed data blocks |

Built with Unity **6000.6.5f1**: UnityFS archive version **8** and SerializedFile
version **23** (the newest this reader handles; the Workshop content is 17 and 22).

`TrackSpline.cs` is the one custom script: a closed Catmull-Rom loop through
`controlPoints[8]`, `width` 12, `wallHeight` 2, `samples` 128, `startSample` 0,
`trackName` "Fixture Loop". `Builder.cs` is the editor script that generates the
meshes from it and builds the bundles.

## Regenerating

Needs Unity 6000.6.5f1. Put `Builder.cs` under `Assets/Editor/` and
`TrackSpline.cs` under `Assets/Scripts/` of an empty project, then:

    Unity -batchmode -nographics -quit -projectPath <project> -executeMethod Builder.Build

The bundles land under the project's `out/` in one directory per compression.
Copy them here. The tests assert counts and values, not bytes, so a rebuilt
fixture only has to keep the shape above.
