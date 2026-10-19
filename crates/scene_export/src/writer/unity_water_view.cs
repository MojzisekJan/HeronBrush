using System;
using UnityEngine;
// Attach to a game camera if using the built-in fog adapter. Custom pipelines can
// query HeronBrushWater.Sample themselves and own their post processing.
public sealed class HeronBrushWaterView:MonoBehaviour {
    public HeronBrushWater[] Waters;bool active,oldFog;Color oldColor;float oldDensity;FogMode oldMode;
    void LateUpdate(){var water=HeronBrushWater.Sample(Waters??Array.Empty<HeronBrushWater>(),transform.position,out float depth);if(water){if(!active){oldFog=RenderSettings.fog;oldColor=RenderSettings.fogColor;oldDensity=RenderSettings.fogDensity;oldMode=RenderSettings.fogMode;active=true;}RenderSettings.fog=true;RenderSettings.fogMode=FogMode.ExponentialSquared;RenderSettings.fogColor=water.Tint;RenderSettings.fogDensity=water.Settings.fog_density;}else Restore();}
    void Restore(){if(!active)return;RenderSettings.fog=oldFog;RenderSettings.fogColor=oldColor;RenderSettings.fogDensity=oldDensity;RenderSettings.fogMode=oldMode;active=false;}
    void OnDisable(){Restore();}
}
