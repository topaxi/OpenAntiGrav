using System.IO;
using UnityEditor;
using UnityEditor.SceneManagement;
using UnityEngine;

// Generates a small closed-loop track from a spline and exports it as asset
// bundles. Run headless:
//   Unity -batchmode -nographics -quit -projectPath <p> -executeMethod Builder.Build
public static class Builder
{
    static readonly Vector3[] Points =
    {
        new Vector3(0, 0, 0),
        new Vector3(120, 0, 10),
        new Vector3(200, 6, 80),
        new Vector3(190, 12, 190),
        new Vector3(100, 8, 240),
        new Vector3(10, 2, 200),
        new Vector3(-60, 0, 130),
        new Vector3(-50, 0, 40),
    };

    static Vector3 CatmullRom(Vector3 p0, Vector3 p1, Vector3 p2, Vector3 p3, float t)
    {
        float t2 = t * t, t3 = t2 * t;
        return 0.5f * (2f * p1 + (p2 - p0) * t + (2f * p0 - 5f * p1 + 4f * p2 - p3) * t2
            + (3f * p1 - p0 - 3f * p2 + p3) * t3);
    }

    static Vector3 Sample(float u)
    {
        int n = Points.Length;
        float f = u * n;
        int i = Mathf.FloorToInt(f) % n;
        float t = f - Mathf.Floor(f);
        return CatmullRom(Points[(i + n - 1) % n], Points[i], Points[(i + 1) % n], Points[(i + 2) % n], t);
    }

    static Mesh Ribbon(string name, int samples, System.Func<int, Vector3> left, System.Func<int, Vector3> right)
    {
        var verts = new Vector3[(samples + 1) * 2];
        var uvs = new Vector2[verts.Length];
        for (int s = 0; s <= samples; s++)
        {
            verts[s * 2] = left(s % samples);
            verts[s * 2 + 1] = right(s % samples);
            uvs[s * 2] = new Vector2(0, s * 0.25f);
            uvs[s * 2 + 1] = new Vector2(1, s * 0.25f);
        }
        var tris = new int[samples * 6];
        for (int s = 0; s < samples; s++)
        {
            int a = s * 2, b = a + 1, c = a + 2, d = a + 3;
            tris[s * 6 + 0] = a; tris[s * 6 + 1] = c; tris[s * 6 + 2] = b;
            tris[s * 6 + 3] = b; tris[s * 6 + 4] = c; tris[s * 6 + 5] = d;
        }
        var mesh = new Mesh { name = name, vertices = verts, uv = uvs, triangles = tris };
        mesh.RecalculateNormals();
        mesh.RecalculateBounds();
        return mesh;
    }

    static GameObject Part(string name, Transform parent, Mesh mesh, Material mat, bool collider)
    {
        var go = new GameObject(name);
        go.transform.SetParent(parent, false);
        go.AddComponent<MeshFilter>().sharedMesh = mesh;
        go.AddComponent<MeshRenderer>().sharedMaterial = mat;
        if (collider) go.AddComponent<MeshCollider>().sharedMesh = mesh;
        return go;
    }

    static string WriteChecker()
    {
        const int size = 64;
        var tex = new Texture2D(size, size, TextureFormat.RGB24, false);
        for (int y = 0; y < size; y++)
            for (int x = 0; x < size; x++)
            {
                bool dark = ((x / 8) + (y / 8)) % 2 == 0;
                bool edge = x < 4 || x >= size - 4;
                tex.SetPixel(x, y, edge ? new Color(0.9f, 0.6f, 0.1f) : dark ? new Color(0.2f, 0.22f, 0.25f) : new Color(0.55f, 0.58f, 0.62f));
            }
        Directory.CreateDirectory("Assets/Generated");
        const string path = "Assets/Generated/track_checker.png";
        File.WriteAllBytes(path, tex.EncodeToPNG());
        AssetDatabase.ImportAsset(path);
        var importer = (TextureImporter)AssetImporter.GetAtPath(path);
        importer.mipmapEnabled = true;
        importer.textureCompression = TextureImporterCompression.Compressed;
        importer.SaveAndReimport();
        return path;
    }

    public static void Build()
    {
        const int samples = 128;
        const float width = 12f, wallHeight = 2f;
        var centre = new Vector3[samples];
        var side = new Vector3[samples];
        for (int s = 0; s < samples; s++)
        {
            centre[s] = Sample((float)s / samples);
            var ahead = Sample((float)(s + 1) / samples) - centre[s];
            side[s] = Vector3.Cross(Vector3.up, ahead.normalized).normalized;
        }

        Directory.CreateDirectory("Assets/Generated");
        var floor = Ribbon("TrackFloor", samples, s => centre[s] - side[s] * width * 0.5f, s => centre[s] + side[s] * width * 0.5f);
        var wallL = Ribbon("TrackWallLeft", samples, s => centre[s] - side[s] * width * 0.5f + Vector3.up * wallHeight, s => centre[s] - side[s] * width * 0.5f);
        var wallR = Ribbon("TrackWallRight", samples, s => centre[s] + side[s] * width * 0.5f, s => centre[s] + side[s] * width * 0.5f + Vector3.up * wallHeight);
        AssetDatabase.CreateAsset(floor, "Assets/Generated/TrackFloor.asset");
        AssetDatabase.CreateAsset(wallL, "Assets/Generated/TrackWallLeft.asset");
        AssetDatabase.CreateAsset(wallR, "Assets/Generated/TrackWallRight.asset");

        var texPath = WriteChecker();
        var mat = new Material(Shader.Find("Standard")) { name = "TrackSurface" };
        mat.mainTexture = AssetDatabase.LoadAssetAtPath<Texture2D>(texPath);
        AssetDatabase.CreateAsset(mat, "Assets/Generated/TrackSurface.mat");

        var root = new GameObject("FixtureLoop");
        var spline = root.AddComponent<TrackSpline>();
        spline.controlPoints = Points;
        spline.width = width;
        spline.wallHeight = wallHeight;
        spline.samples = samples;
        spline.startSample = 0;
        Part("Floor", root.transform, floor, mat, true);
        Part("WallLeft", root.transform, wallL, mat, true);
        Part("WallRight", root.transform, wallR, mat, true);
        var start = new GameObject("StartLine");
        start.transform.SetParent(root.transform, false);
        start.transform.localPosition = centre[0];
        start.transform.localRotation = Quaternion.LookRotation(centre[1] - centre[0], Vector3.up);

        const string prefabPath = "Assets/Generated/FixtureLoop.prefab";
        PrefabUtility.SaveAsPrefabAsset(root, prefabPath);
        Object.DestroyImmediate(root);

        var scene = EditorSceneManager.NewScene(NewSceneSetup.EmptyScene, NewSceneMode.Single);
        PrefabUtility.InstantiatePrefab(AssetDatabase.LoadAssetAtPath<GameObject>(prefabPath));
        const string scenePath = "Assets/Generated/FixtureLoop.unity";
        EditorSceneManager.SaveScene(scene, scenePath);
        AssetDatabase.SaveAssets();

        var builds = new[]
        {
            new AssetBundleBuild { assetBundleName = "fixture_loop", assetNames = new[] { prefabPath } },
            new AssetBundleBuild { assetBundleName = "fixture_loop_scene", assetNames = new[] { scenePath } },
        };
        Export("../out/lzma", builds, BuildAssetBundleOptions.None);
        Export("../out/lz4", builds, BuildAssetBundleOptions.ChunkBasedCompression);
        Export("../out/raw", builds, BuildAssetBundleOptions.UncompressedAssetBundle);
        Debug.Log("Builder: done");
    }

    static void Export(string dir, AssetBundleBuild[] builds, BuildAssetBundleOptions options)
    {
        Directory.CreateDirectory(dir);
        BuildPipeline.BuildAssetBundles(dir, builds, options | BuildAssetBundleOptions.ForceRebuildAssetBundle, BuildTarget.StandaloneLinux64);
    }
}
