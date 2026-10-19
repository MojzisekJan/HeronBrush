using UnityEngine;
namespace HeronBrush.Live {
    // Scene-persistent attachment. Only generated is replaced by the editor addon.
    public sealed class HeronBrushLiveRoot : MonoBehaviour {
        [HideInInspector] public string source;
        [HideInInspector] public string revision;
        [HideInInspector] public GameObject generated;
    }
}
