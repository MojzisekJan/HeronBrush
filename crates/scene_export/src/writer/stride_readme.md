# Stride export

Written for Stride 4.4. Export into a folder under your game package's `Assets`
folder, for example `MyGame.Game/Assets/Levels`. The exporter then finds the
package's `.sdpkg`, takes the level's content URL from where it lands
(`Levels/<level>`), and lists the level's models and sky as root assets so
they compile and load by URL. Other root assets in the package are left alone.

Each level folder holds:

- `<entity>.glb` and `<entity>.sdm3d`: the meshes and their model assets.
- `Materials/*.sdmat` and `Textures/*`: one material per glTF material, with its
  base-colour image (textures, baked face paint and gradients), metalness,
  roughness, normal maps, emission, ambient occlusion and double-sidedness. Each
  texture keeps its U/V clamp or repeat settings.
- `Sky/`: the editor's procedural sky as an HDR cube map and skybox asset.
- `HeronBrushStrideLevel.cs`: the entity records and `Build`, which creates every
  entity under one root, attaches models and project prefabs, the editor's
  keyframe animation and trigger volumes, then calls your configure callback.
- `HeronBrushStrideRuntime.cs`: the animation player, trigger volumes and water:
  `HeronBrushStrideWater` (`Sample`, `Velocity`; `level.Water(point, out depth)`)
  and `HeronBrushStrideWaterView`, which gives a camera under water the
  compositor's fog. The loader adds the view to the scene's first camera.
- `HeronBrushStridePhysics.cs`: Bullet colliders (the `Stride.Physics` package).
  Delete it when your game uses other physics.

The quickest start is the `HeronBrushStrideLevelLoader` component: add it to an
entity in a scene and the level, its colliders and its sky and sun load when the
scene starts. From code:

```csharp
var level = HeronBrushStrideLevel.Build(Content, (entity, record, all) => { /* your components */ });
HeronBrushStridePhysics.AddColliders(level);
Entity.Scene.Entities.Add(level.Root);
level.Activators.Add(player);   // whatever walks into trigger volumes
```

`HeronBrushStrideLevel.CreateEnvironment(Content)` returns the sky and sun
entities. `Rebuild` replaces a previous copy in place. Generated classes live in
a namespace unique to the level, so several levels compile in one game.

Static world geometry becomes a static triangle mesh, movers kinematic convex
bodies that follow their animation, and `trigger_*` volumes trigger colliders.
The level JSON keeps every property, target and collision shape for project code.

`stride_model` replaces an entity's model URL and `stride_prefab` adds a project
prefab. A blended glTF material becomes transparent; `MASK` becomes a native
cutoff material with the authored threshold. Normal maps retain their strength
and use OpenGL-to-Stride Y inversion. Metallic/roughness and AO textures are
sampled in linear space, with the source factors and AO strength. Emission retains
its image, colour and intensity. Same-named materials with different contents
receive separate native assets.

UV sets beyond UV0, mirrored-repeat wrapping and texture transforms need project
mapping. The PBR additions have passed native YAML loading in Stride 4.4.1;
rendering and asset compilation still need engine verification.
