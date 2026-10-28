# Common entities and engine exports

The editor embeds `gamedata/heron_brush/heron_brush.fgd`. Its point and brush entities are available in new maps, the entity browser and MCP without choosing a game or loading an FGD. A project FGD replaces classes with the same name. The other built-in classes are offered for placement only with the Heron Brush and Generic profiles, or with a profile whose `GameConfig.cfg` sets `"heron": { "builtinEntities": true }`; with Quake's profile the browser lists Quake's classes alone. A map that already has an entity of a built-in class still shows it and edits its keys.

The catalogue includes point, spot and sun lights; player starts; doors, buttons and platforms; one-shot and repeatable triggers; waypoints, cameras, sounds and static props. At the default 32 map units per metre, the 64-unit player-start box is 2 metres tall.

## Export support

- **Godot:** the existing HERON_BRUSH directives produce native light nodes, spawn markers, physics bodies and trigger areas. Target links and entity properties follow the existing scene builder. Door movement still needs a gameplay script, such as HERON_BRUSH's Godot example; selecting `func_door` alone does not install that script.
- **Unity:** common lights become native `Light` components. Entity properties and all target links are retained in a generated `HeronBrushEntity` component as JSON, as well as the level manifest. Player starts are markers, not player controllers. Native MeshColliders include invisible sensors. Optional animation, trigger, platform-walker and water adapters are generated separately; project-owned gameplay can use the complete metadata.
- **Unreal:** common lights become native light actors, and player starts become `PlayerStart` actors. Full entity data is embedded in the Python importer and written beside it as `<level>.json`. Short `heron_brush_entity_index:N` actor tags identify records in that data; large JSON strings are not stored in Unreal name tags. Doors and triggers still need project gameplay and collision setup.
- **Defold:** brush entities retain their properties in generated component scripts, including door speed and travel. Propertyless player starts retain their classname. The JSON manifest preserves all targets and typed values. Existing collision export uses trigger and kinematic collision objects. Rendering lights and executing gameplay require project scripts/materials.
- **Flax:** common lights become native point/spot/directional actors below the entity root in final and live imports. Ranges convert metres to centimetres and spot angles remain half angles. The importer also places models and generic entity actors; the JSON manifest carries the entity data. Current native adapters and their limits are recorded in [the export contract](engine-export-contract.md) and [water export](water-export.md).

Light ranges use metres in Unity and are converted to centimetres in Unreal. The HERON_BRUSH spotlight half-angle becomes Unity's full cone angle and Unreal's outer half-angle. Intensity values are transferred numerically, not calibrated for identical exposure across rendering pipelines.

## Per-engine settings

Each target stores its settings independently, including its last export folder and export-on-save switch.

- **Godot:** `res://` texture and material paths, trigger physics layer, UV2 lightmap generation and lightmap texel size. The Godot-only controls are shown only for Godot.
- **Defold:** asset resource directory, model material resource (default `/builtins/materials/model.material`) and colour texture sampler (default `tex0`). The chosen sampler must exist in the chosen material.
- **Unreal:** content path, default `/Game/<level>`, used by the generated import task.
- **Flax:** content path relative to `Content`, default `<level>`, used to resolve models in the generated importer.

Blank engine-specific fields retain the exporter defaults. Existing saved settings continue to load.

## Verification and follow-up

Automated tests cover bundled/custom definitions, MCP discovery, entity placement scale, generated native lights, preservation of properties and targets, Defold material options and settings persistence. Generated Unity, Unreal and Flax scripts still need execution in their respective editors; unit tests do not replace those checks.

Three.js exports include `heron_lights.js`, used by the generated playtest and the
production example loader. Lights follow their entity roots, with local -Z as the
emission direction. The loader also preserves imported PBR materials and shares
geometry between repeated model instances.

Bevy exports include the optional `heron_lights.rs` module. Add `HeronLightPlugin`
alongside `HeronEntityPlugin`; it handles point, spot and sun classes on newly
loaded metadata entities and preserves existing project-owned light components.
Its energy mapping is 1000 lumens per unit for point/spot and 10000 lux per unit
for sun lights. These are explicit sample conventions, not photometric calibration.
The viewer now handles spotlights and keeps authored suns when a sky is present.

Raylib exports include `<level>_lighting.h`, with `HbLightingLoad` and
`HbLightingDraw`. The generated preview uses it for authored point, spot and sun
lights, normal textures and basic diffuse/specular response. It supplies a sky
sun when no authored sun exists. It does not generate shadow maps, full PBR or
animated point-light transforms. Water view shading remains its separate adapter.

Native regression results for these adapters are recorded in
[the local export audit](export-audit-2026-10-28.md). Useful later options include Unity layers, Unreal light mobility, Defold collision groups/masks and Flax static flags; add them together with their actual importer support.

### Editor symbols and sound

The built-in entity browser and 2D views display class symbols. In the 3D view,
point classes without a project model or sprite use bundled billboard symbols;
point lights retain their existing colour-aware light marker. The Entity models
view filter switches these representations back to bounding boxes. Symbols are
editor helpers and are not written into exported level geometry.

`ambient_sound` is the built-in positional sound entity. Set `stream` to the audio
resource path, `volume_db` for volume, `max_distance` for range and `autoplay` for
startup playback. The Godot exporter creates an `AudioStreamPlayer3D`. Looping is
configured on the audio resource, not guaranteed by the entity's name. The editor
does not preview audio playback.


## Water volumes

`water_volume` is a built-in brush class with a water icon and inspector parameters.
Custom brush classes can opt in with `water_enabled=1`. Bevy, Three.js, Babylon.js,
PlayCanvas, Defold, Unity and Flax consume a versioned, exact-volume water payload.
Surface and player adapters are optional; `water_surface_material=0` preserves
game-owned appearance.
See [water export](water-export.md) and the bundled Water Lab example.
