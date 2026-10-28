# Stride export

Stride is available in the Export window, MCP (`engine: "stride"` or
`"stride3d"`), per-project profiles and export-on-save. Export into a folder under
the game package's `Assets` folder. The exporter writes native Stride assets and
generated C#, and adds the level's models and sky to the package's root assets.
See the README written beside each export for usage.

| Written | Stride side |
| --- | --- |
| `<level>/<entity>.glb` + `.sdm3d` | Model assets, with `Materials` in glTF material order |
| `<level>/Materials/*.sdmat`, `Textures/*` | Base colour image from the GLB (textures, face paint, gradients), tint, metallic/roughness maps and factors, emission, normal maps, AO, double-sided culling, glTF `BLEND` and `MASK`; each texture keeps its U/V sampler |
| `<level>/Sky/Sky.dds`, `.sdtex`, `Skybox.sdsky` | The editor's procedural sky as a 128 px RGBA16F cube map |
| `<level>/HeronBrushStrideLevel.cs` | Records, `Build`, `Rebuild`, `CreateEnvironment`, `HeronBrushStrideLevelLoader` |
| `<level>/HeronBrushStrideRuntime.cs` | Keyframe animation, trigger volumes and water volumes, as the other runtimes |
| `<level>/HeronBrushStridePhysics.cs` | Bullet colliders: static trimesh, kinematic convex movers, trigger sensors |
| `<level>.json` | The complete typed level with inline collision |

The content URL is taken from where the level lands below the package's asset
folder. An explicit content directory that disagrees produces a warning. Root
assets under the level's URL are replaced on re-export; other entries remain.
Without an enclosing package the files are still written, with a warning to add
the models as root assets by hand.

The sky comes from the map's first render camera (its sky, horizon, ground and
sun colours, sun direction and intensities), or the editor's lighting when the
map has no camera. It evaluates the same function as the 3D view's sky.
Direct3D's cube layout is left-handed and Stride samples with its right-handed
direction, so the faces are written mirrored in Z.

## Native verification, 18 October 2026

Stride 4.4.1 (CLI 1.0.4), .NET SDK 10.0.102, macOS 26.5 on Apple M4. Stride runs on
macOS through its Linux platform project with Homebrew MoltenVK 1.4.2 and
vulkan-loader; the asset compiler also needs Vulkan:

```sh
export DYLD_FALLBACK_LIBRARY_PATH=/opt/homebrew/lib
export VK_ICD_FILENAMES=/opt/homebrew/etc/vulkan/icd.d/MoltenVK_icd.json
dotnet build MyGame.Linux/MyGame.Linux.csproj
```

`examples/engine-checks/stride/export_map.py` exports any map into a package. A
`stride new stride-game` project with a reflection-based check scene loaded:

| Map | Result |
| --- | --- |
| motion_lab | 10 entities, 9 models, 1 static + 4 kinematic + 4 trigger colliders, 4 animations. Walking into `func_button_3` played `open` on `func_animate_3`, ending at (6.625, 2, 4.5) with a 90° Y turn, equal to the Three.js runtime's maths. A 0.25 m ball came to rest at y = 0.37 on the 0.12 m start pad. |
| lantern_lane | 64 entities, 21 models, 35 colliders; Kenney textures, imported models and paint drawn; a ball settled on the worldspawn mesh. |
| relay_yard | Gradient face paint drawn on walls and roofs; ball settled on the floor. |
| lantern_lane, sky | Generated skybox and sun replaced the template's; the sun's glow sits in the direction the sun's light comes from. |

Screenshots must be taken from the window: `Texture.Save` of the sRGB back buffer
on this setup writes washed-out colours that the window does not show.

Stride's texture converter cannot generate mipmaps or compressed blocks for
images under 4 pixels, which one-pixel solid paint is; those textures are written
uncompressed without mipmaps.

## Remaining

- Imported materials using UV sets other than UV0, mirrored-repeat samplers or texture transforms still need separate mapping.
- Game Studio was not run (Windows, or Linux through Proton/Wine). Assets written
  here were compiled by the CLI asset compiler, not opened in the editor.
- Windows and Linux native runs; Bepu physics as an alternative to Bullet.

## PBR material export, 28 October 2026

Native materials now carry normal maps, emissive images and factors, including
`KHR_materials_emissive_strength`, alpha masks and their cutoff. Missing mask
cutoffs use glTF's 0.5 default. `BLEND` and opaque materials retain their modes.
Metallic-roughness uses linear B/G channels and the source factors, with roughness
inverted to glossiness. AO uses the linear R channel and mixes with white according
to its strength. Colour and emission images stay sRGB and are not premultiplied.

Normal textures use `NormalMapTextureType` with `InvertY: true` for glTF's OpenGL
convention. The material scales signed X/Y by `normalTexture.scale`. Normal images
are uncompressed to preserve Z; the material does not reconstruct Z from scaled
XY. Each map keeps independent U/V clamp or repeat settings. Small PNGs still
omit mipmaps when either dimension is below four pixels.

Same-named materials from different meshes share an asset only when their factors,
images and sampler settings match. Conflicting names receive a content fingerprint,
independent of mesh order. Auxiliary texture names use `.normal`, `.mr`, `.ao` and
`.emissive` before their file extension, so a material called `paint_normal` cannot
overwrite the normal map of `paint`.

Regression tests cover the native assets, default cutoffs, scalar emission,
normal strength, PBR channels, asymmetric samplers and same-name materials.
Stride 4.4.1's actual YAML serializer loaded the generated material and all five
texture assets on Linux. This verifies deserialization and material state, not
asset compilation, shader rendering or Game Studio integration.

```sh
HERON_STRIDE_MATERIAL_OUTPUT=/tmp/heron-stride-materials cargo test -p scene_export --lib --offline writer::stride
dotnet run --project examples/engine-checks/stride/materials/StrideMaterialCheck.csproj -- /tmp/heron-stride-materials/arena
```
