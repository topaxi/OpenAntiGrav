using UnityEngine;

// A closed Catmull-Rom loop through authored control points: the one custom
// MonoBehaviour in the fixture, so a reader sees a user script's typetree.
public class TrackSpline : MonoBehaviour
{
    public Vector3[] controlPoints;
    public float width = 12f;
    public float wallHeight = 2f;
    public int samples = 128;
    public int startSample;
    public string trackName = "Fixture Loop";
}
