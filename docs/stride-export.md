# Stride export

Stride is available in the Export window, MCP (`engine: "stride"` or
`"stride3d"`), per-project profiles and export-on-save. Export into a folder under
the game package's `Assets` folder. The exporter writes native Stride assets and
generated C#, and adds the level's models and sky to the package's root assets.
See the README written beside each export for usage.

| Written | Stride side |
| --- | --- |
| `<level>/<entity>.glb` + `.sdm3d` | Model assets, with `Materials` in glTF material order |
| `<level>/Materials/*.sdmat`, `Textures/*` | Base colour image from the GLB (textures, face paint, gradients), tint, metalness, glossiness (1 − roughness), double-sided culling, glTF `BLEND` as a blended material of its own; paint clamps |
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

- Emission, normal maps and the `MASK` alpha mode are not transferred to materials.
- Game Studio was not run (Windows, or Linux through Proton/Wine). Assets written
  here were compiled by the CLI asset compiler, not opened in the editor.
- Windows and Linux native runs; Bepu physics as an alternative to Bullet.
