# Sky export

An export can carry the map's sky, sun and fog into the engine, so a level looks
there as it does in the 3D view. **Sky and sun** under *What to write* switches it
(`sky` in MCP options and project profiles); it is on by default. Switch it off
when the game sets its own environment.

## Where the sky comes from

The first render camera saved in the map, or the 3D lighting preferences when the
map has none: sky, horizon and ground colours, sun colour, direction and
intensity, ambient intensity and exposure. A worldspawn with `sky` set to 1
replaces the sky and horizon colours with `sky_top_color` and
`sky_horizon_color`; one with `fog_enabled` adds exponential fog from
`fog_light_color` (the horizon when unset) and `fog_density` per metre.

`scene_export/src/writer/sky.rs` evaluates the same function as the view's
`sky_radiance` and bakes it as each engine wants it. The sun's glow is part of
the image; the sun itself is the engine's own directional light.

| Target | Sky | Light | Fog |
| --- | --- | --- | --- |
| Godot | `<level>/sky.hdr` panorama on a `PanoramaSkyMaterial`, background, ambient and reflections | `HeronSun` `DirectionalLight3D` with shadows, unless the map has a directional light; AgX | `Environment` fog, leaving the sky clear |
| three.js | `environment` in `<level>.json` and `<level>/sky.hdr` (equirectangular, for `HDRLoader`); the HTML page draws the sky from the numbers | `DirectionalLight` from `to_sun`; AgX | `FogExp2` |
| Babylon.js | `<level>/sky.hdr` for `HDRCubeTexture`, baked a quarter turn round from three.js's, as the environment and skybox; the page embeds a 256 by 128 copy | `DirectionalLight` from `to_sun`, the sun's intensity over pi; ACES, as Babylon.js has no AgX | `FOGMODE_EXP2`, the same falloff as three.js |
| Bevy | `<level>/sky.ktx2`, an RGBA16F cube (Bevy samples cubes with Z negated, so it is baked mirrored); `heron_sky.rs` makes the camera's `Skybox`, `GeneratedEnvironmentMapLight`, HDR, AgX and the exposure that shows the editor's units | `DirectionalLight` with shadows, illuminance = the sun's intensity | `DistanceFog` |
| raylib | `<level>/sky.png`, six faces in a vertical strip for `LoadTextureCubemap`, baked tone mapped because raylib draws colours as they are; the preview draws it round the camera; `environment` in `<level>.json` | Numbers in `environment` | Numbers in `environment` |
| Defold | `<level>/sky.cubemap` from six faces baked tone mapped, drawn on `heron_sky` (a cube model) by the `heron_sky` predicate; `<level>.render` draws it before the models | Numbers in `<level>_environment.lua` and the level JSON | Numbers likewise |
| Stride | HDR cube `Sky.dds`, `Skybox` asset | `LightDirectional` with shadows | -- |
| Cocos Creator | Six PNG faces, skybox with IBL | The scene's directional light, with shadows | Exponential fog |
| GDevelop | Six PNG faces, `Skybox` effect | `DirectionalLight` with shadows; AgX | `ExponentialFog` |

Light intensities follow the view's convention: diffuse light is the sun's
intensity over pi, and the view's AgX runs at its exposure times 1.8.

## Native verification, 18 October 2026

macOS 26.5, Apple M4. Each check faces the camera at the sun and away from it.

| Target | Result |
| --- | --- |
| Godot 4.5 (Forward+, Metal) | motion_lab and relay_yard: the sun's glow straight ahead at 45 degrees, shadows away from it. A worldspawn sky and fog override (motion_lab) fogs the level and keeps the sky. |
| Bevy 0.19 (Metal) | The sample with motion_lab: the glow centred looking along `to_sun` (`--at 0.5,2,0.5 --look 0.5,0.707,-0.5`); lantern_lane textured and shadowed under the sky. The demo's own level, without an environment, renders as before. |
| raylib 5.5 (OpenGL 3.3) | motion_lab preview: the glow centred when looking along `to_sun`, no seams between faces. |
| Defold 1.13 (Bob + dmengine, OpenGL) | motion_lab: the glow straight ahead at 45 degrees; looking up and away, a seamless sky round the zenith. |
| three.js 0.186 (Edge) | motion_lab and lantern_lane HTML pages: the glow where the sun is, textures under the sky's light. `sky.hdr` loaded through `HDRLoader` draws the same sky as the page. |
| Stride, Cocos, GDevelop | See their pages. |

Flax, Unity and Unreal do not carry the sky yet. Defold's own render script does not draw `heron_sky`; use `<level>.render` or add the pass to the project's.
