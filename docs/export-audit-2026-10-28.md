# Local export audit, 28 October 2026

This audit used the Linux PC's installed engines and a fresh export of
`demos/feature_lab/feature_lab.map`. File checks and native checks are separate.
The editable map and its local vent_station work were left untouched.

| Target | Changes or audit result | Native evidence and remaining scope |
| --- | --- | --- |
| Godot | Neutral PBR map references and slot names corrected; existing exporter audited | Feature lab import, materials, lights, collision, animation, triggers and water passed in Godot |
| Defold | Explicit project PBR sampler bindings, shared final/live mapping, portable normal companions, validation; sample normals now follow rotation and nonuniform scale | Full feature lab passed Bob/dmengine, physics and screenshot checks. Separate production albedo/normal binding fixture compiled in Bob. Default project shader remains albedo-based |
| three.js | Native common lights; production loader preserves PBR and shared geometry; playtest keeps source maps and cleans up light resources | Chromium verified the production loader and generated page, PBR maps, common lights, collision and animation |
| Babylon.js | Existing adapter audited; corrected neutral PBR image metadata | Chromium/Havok feature lab and generated page passed |
| PlayCanvas | Existing adapter audited; corrected neutral PBR image metadata | Chromium/ammo.js feature lab and generated page passed |
| Bevy | Optional native light plugin, spotlight support, authored sun retained with a sky | Native Bevy ECS verified light types, range/cone/colour, placement and preservation of project lights; the exported feature lab also rendered after all assets loaded |
| raylib | Optional forward light shader and normal textures in preview; mip filtering; collision-only files skipped as visual models and redundant orphan collision meshes removed | Actual OpenGL compiled all shaders and drew the fixture. Baseline response is not full PBR or shadow-map rendering; underwater rendering uses the separate water adapter |
| Unity | HDR panorama, persistent sky material, native sun/ambient and map fog | Complete native edit/play feature lab passed. Sky material/resource/fog state checked without graphics; GPU captures also verified the sky glow against the native sun direction. HDRP and live environment publishing remain separate |
| Flax | Native common lights in final/live import; skybox, skylight and sun; normalized cube compatible with Linux importer | Complete native feature lab passed with Null graphics, including native environment actors and cube loading. Rendered appearance remains unverified; homogeneous fog and the native importer's missing model AO map remain open |
| Stride | Native normal/emission/MR/AO maps, alpha masks, samplers and material identity fixes | Writer regressions and actual Stride 4.4.1 YAML loading passed. No installed native Stride editor/game project for rendered verification |
| OBJ | Portable normal images, imported material factors/maps, PBR-aware identity, Blender import helper | Blender 4.0.2 loaded native normal-map nodes and authored strength through the generated helper. Raw Blender OBJ import ignores MTL `norm`; AO and glTF material extensions are not represented by MTL |
| glTF | Portable format audited; existing typed metadata and PBR retained | Chromium GLTFLoader feature lab passed, including shared models and PBR. Gameplay/physics components are outside the format |
| GLB | Portable format audited | Chromium GLTFLoader feature lab passed, including shared models and PBR. Gameplay/physics components are outside the format |
| Unreal | File export audited | No supported native Unreal editor found on this PC. Native import, collision/gameplay and sky gaps are not marked complete |
| Cocos Creator | File export audited | No supported native Creator installation found on this Linux PC; Cocos2d-x is a different product |
| GDevelop | File export audited | No installed native editor/runtime project found here; native verification from another OS is not counted as this PC's result |

## Reproduction

Build the current editor, export the fixture and check the written files:

```sh
cargo build -p heron-brush --offline
python3 examples/engine-checks/export_feature_lab.py --root /tmp/heron-export-audit
python3 examples/engine-checks/check_feature_lab.py --root /tmp/heron-export-audit
```

Each engine's commands are in `examples/engine-checks/README.md` and its target
folder. The Defold sampler fixture has its own `verify_material_samplers.py`.
OBJ exports containing normal maps write `<level>_materials.py`; run that file
inside Blender or with `blender --background --python <level>_materials.py`.
The helper imports the OBJ and copies imported materials before changing nodes.

For the optional Bevy common-light adapter, include `heron_entities.rs` and
`heron_lights.rs`, add both `HeronEntityPlugin` and `HeronLightPlugin`, and attach
`HeronEntityData` to each imported entity root. Existing light components win.
For raylib, include `<level>_lighting.h` after raylib headers, call
`HbLightingLoad()` once, then `HbLightingDraw(shader, model, camera.position)`.
The generated preview does this automatically. Giving its executable an image
filename writes a screenshot after 30 frames and exits.

The audit does not imply identical tone mapping or calibrated light units across
engines. It does not claim to close every project-specific material/runtime gap.
