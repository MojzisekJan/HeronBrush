# Surface materials

Each brush face may override roughness, metallic and emission independently of its solid colour, gradient or texture. The editor stores percentages for roughness/metallic and hundredths for emission. Defaults when explicitly applying are 0.65, 0 and 0. Removing the override restores renderer preferences; export targets keep their own default material behaviour for faces without overrides.

The `.map` face comment appends `pbr ROUGHNESS METALLIC EMISSION` after the `heron_brush` marker and any face paint. Older maps need no migration. Explicit parameters survive save/load, geometry edits, undo/redo, clipboard operations and serialized collaboration state. MCP exposes `set_surface_material` with roughness/metallic 0–1, emission 0–8, optional brush/face and reset. `describe_brush` returns the values.

The lit viewport and render cameras use per-material batches for these factors. This does not add vertex attributes. The face representation grows by 8 bytes including alignment, without a heap allocation. Wireframe and flat shading retain their diagnostic appearance. Emission uses albedo in linear light and participates in exposure and bloom; it is not a light source for other geometry.

| Export | Behaviour |
| --- | --- |
| Godot | StandardMaterial3D roughness, metallic, emission texture and energy. |
| glTF / GLB | Core metallic/roughness factors; albedo-coloured emission with KHR_materials_emissive_strength. Solid opaque colours remain factors; gradients use images. |
| Unity | Generated Build Level importer creates persistent materials for Standard or URP Lit, maps smoothness to 1 − roughness and sets emission. HDRP and custom shaders need their own property mapping. |
| three.js demo | Preserves exported factors and applies emission with the albedo texture. |
| Bevy, Flax, Unreal and other GLB consumers | GLB contains the factors and emission extension. Actual appearance depends on the engine importer and its shader/extension support; native engine execution was not validated for this change. |
| Defold / raylib | GLB and level metadata preserve the factors. Their default sample shaders do not implement full PBR; supply an appropriate material/shader to use them. |
| OBJ / MTL | Writes Pr/Pm, approximate Ns/Ks and Ke/map_Ke. These extensions are importer-dependent; use GLB for portable PBR. |

Generated material identity includes surface settings, so identically coloured matte and metallic faces do not merge into one material. Neutral level JSON carries optional `surface: [roughness, metallic, emission]` on each texture/material entry.

The current controls cover portable metallic/roughness materials. Clearcoat, transmission/refraction, anisotropy and normal/height map editing are not exposed. Emission is not a substitute for a point light or global illumination.

Material terminology follows [Godot StandardMaterial3D](https://docs.godotengine.org/en/stable/tutorials/3d/standard_material_3d.html). Engine appearance can differ with lighting, tone mapping and shader choice.
